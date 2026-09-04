// Synthesize all dialogue lines with msedge-tts (Microsoft neural voices).
// Nick: en-US-ChristopherNeural (deep male) — Dave's sample measures ~97 Hz median F0.
// Nora: en-US-JennyNeural (warm, conversational).
const fs = require('fs');
const path = require('path');
const { MsEdgeTTS, OUTPUT_FORMAT } = require('msedge-tts');

const dialogue = JSON.parse(fs.readFileSync('dialogue.json', 'utf8'));
// Matches the checked-in docs/media/podcast/production/audio-dialogue/
// layout -- not 'audio' (that was this script's original build-machine
// output path before the files were renamed on check-in).
const OUT = 'audio-dialogue';

const VOICES = {
  // -12Hz, not -6: matches production/README.md's build-inputs table and
  // mix.js's own "Christopher at -12Hz+rubberband" comment -- both agree
  // independently, so -12Hz is what actually produced the checked-in lines.
  nick: { voice: 'en-US-ChristopherNeural', rate: '+3%', pitch: '-12Hz' },
  nora: { voice: 'en-US-JennyNeural', rate: '-4%', pitch: '+0Hz' },
};

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const lines = dialogue.segments.filter((s) => s.type === 'nick' || s.type === 'nora');

  // A per-voice-type slot holding the live MsEdgeTTS instance, rather than
  // one const captured per segment: on a failed attempt, the previous fix
  // tried to patch the broken instance in place via Object.assign(tts,
  // fresh), which doesn't reliably transplant a class instance's internal
  // state (private fields aren't own-enumerable and won't copy), and even
  // when it happened to work, the *next* retry in the same segment's loop
  // still called the same object reference rather than genuinely starting
  // over. Storing the current instance here and re-fetching it from
  // `getTts` every attempt means a retry after `instances[type] = null`
  // always gets a truly fresh connection.
  const instances = { nick: null, nora: null };
  async function getTts(type) {
    if (!instances[type]) {
      const t = new MsEdgeTTS();
      await t.setMetadata(VOICES[type].voice, OUTPUT_FORMAT.AUDIO_24KHZ_48KBITRATE_MONO_MP3);
      instances[type] = t;
    }
    return instances[type];
  }

  for (const seg of lines) {
    const cfg = VOICES[seg.type];
    const outFile = path.join(OUT, `seg${String(seg.id).padStart(2, '0')}_${seg.type}.mp3`);
    let done = false;
    for (let attempt = 1; attempt <= 4 && !done; attempt++) {
      try {
        const tts = await getTts(seg.type);
        const { audioFilePath } = await tts.toFile(OUT, seg.text, { rate: cfg.rate, pitch: cfg.pitch });
        fs.renameSync(audioFilePath, outFile);
        done = true;
      } catch (e) {
        console.error(`seg${seg.id} attempt ${attempt} failed: ${e.message}`);
        await new Promise((r) => setTimeout(r, 1500 * attempt));
        try { instances[seg.type]?.close(); } catch {}
        instances[seg.type] = null; // force getTts to build a genuinely fresh instance next attempt
      }
    }
    if (!done) { console.error(`GIVING UP ON seg${seg.id}`); process.exit(1); }
    console.log(`ok seg${seg.id} ${seg.type}`);
  }
  console.log('ALL LINES DONE');
})().catch((e) => { console.error(e); process.exit(1); });
