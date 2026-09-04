// Synthesize all dialogue lines with msedge-tts (Microsoft neural voices).
// Nick: en-US-ChristopherNeural (deep male) — Dave's sample measures ~97 Hz median F0.
// Nora: en-US-JennyNeural (warm, conversational).
const fs = require('fs');
const path = require('path');
const { MsEdgeTTS, OUTPUT_FORMAT } = require('msedge-tts');

const dialogue = JSON.parse(fs.readFileSync('dialogue.json', 'utf8'));
const OUT = 'audio';

const VOICES = {
  nick: { voice: 'en-US-ChristopherNeural', rate: '+3%', pitch: '-6Hz' },
  nora: { voice: 'en-US-JennyNeural', rate: '-4%', pitch: '+0Hz' },
};

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const lines = dialogue.segments.filter((s) => s.type === 'nick' || s.type === 'nora');
  const nickTts = new MsEdgeTTS();
  const noraTts = new MsEdgeTTS();
  await nickTts.setMetadata(VOICES.nick.voice, OUTPUT_FORMAT.AUDIO_24KHZ_48KBITRATE_MONO_MP3);
  await noraTts.setMetadata(VOICES.nora.voice, OUTPUT_FORMAT.AUDIO_24KHZ_48KBITRATE_MONO_MP3);

  for (const seg of lines) {
    const cfg = VOICES[seg.type];
    const tts = seg.type === 'nick' ? nickTts : noraTts;
    const outFile = path.join(OUT, `seg${String(seg.id).padStart(2, '0')}_${seg.type}.mp3`);
    let done = false;
    for (let attempt = 1; attempt <= 4 && !done; attempt++) {
      try {
        const { audioFilePath } = await tts.toFile(OUT, seg.text, { rate: cfg.rate, pitch: cfg.pitch });
        fs.renameSync(audioFilePath, outFile);
        done = true;
      } catch (e) {
        console.error(`seg${seg.id} attempt ${attempt} failed: ${e.message}`);
        await new Promise((r) => setTimeout(r, 1500 * attempt));
        try { tts.close(); } catch {}
        const fresh = new MsEdgeTTS();
        await fresh.setMetadata(cfg.voice, OUTPUT_FORMAT.AUDIO_24KHZ_48KBITRATE_MONO_MP3);
        if (seg.type === 'nick') Object.assign(tts, fresh); else Object.assign(tts, fresh);
      }
    }
    if (!done) { console.error(`GIVING UP ON seg${seg.id}`); process.exit(1); }
    console.log(`ok seg${seg.id} ${seg.type}`);
  }
  console.log('ALL LINES DONE');
})().catch((e) => { console.error(e); process.exit(1); });
