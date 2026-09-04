// Master build: renders every visual segment, concatenates, and mixes audio.
// Usage: node build.js [--only N]  (renders a single segment for testing)
const { execFileSync, execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const W = 1920, H = 1080, FPS = 24;
const dialogue = JSON.parse(fs.readFileSync('dialogue.json', 'utf8'));

const probe = (f) => {
  const out = execSync(`ffprobe -v error -show_entries format=duration -of csv=p=0 "${f}"`).toString().trim();
  const d = parseFloat(out);
  if (isNaN(d)) throw new Error(`probe failed for ${f}: "${out}"`);
  return d;
};

// ---------- film look (applied to every segment) ----------
const FILM = `hue=s=0,eq=contrast=1.05:brightness=-0.005,noise=alls=6:allf=t+u,vignette=angle=PI/4.6,format=yuv420p`;

// ---------- helpers ----------
const run = (args, label) => {
  try {
    execFileSync('ffmpeg', ['-y', '-v', 'error', ...args], { stdio: ['ignore', 'ignore', 'inherit'] });
  } catch (e) {
    console.error(`FFMPEG FAILED: ${label}\n${e.message}`);
    process.exit(1);
  }
};

// Ken Burns from a still image, returning a filter that outputs WxH (zoom into a crop region)
// srcw/srch: the crop window (aspect of output), zoompan handles the rest
const kenburns = (dur, outW, outH, rate = 0.00028) =>
  `zoompan=z='min(1+${rate}*on,1.13)':x='(iw-iw/zoom)/2':y='(ih-ih/zoom)/2':d=1:fps=${FPS}:s=${outW}x${outH}`;

// Compose a foreground (any size, already scaled) centered over a blurred, darkened fill of itself.
// Expects [fg] and [bgsrc] labeled inputs; outputs to [out].
const composePad = (out) =>
  `[bgsrc]scale=${W}:${H}:force_original_aspect_ratio=increase,crop=${W}:${H},boxblur=22:2,eq=brightness=-0.16:brightness_end=-0.16[out];
   [fg][out]overlay=(W-w)/2:(H-h)/2,setsar=1,settb=AVTB,fps=${FPS}[${out}]`;

// ---------- build timeline ----------
const audioFor = (seg) => (seg.type === 'nick')
  ? `audio/shifted/seg${String(seg.id).padStart(2, '0')}_nick.wav`
  : `audio/seg${String(seg.id).padStart(2, '0')}_nora.mp3`;

const timeline = [];
let nickCount = 0, noraCount = 0;

const push = (o) => timeline.push(o);

// opening cards
push({ kind: 'title', img: 'images/title_open.png', dur: 6.5, voice: null });
push({ kind: 'clip', clip: 'video/establish.mp4', dur: 8.0, voice: null, full: true });

for (const seg of dialogue.segments) {
  if (seg.type === 'nick' || seg.type === 'nora') {
    const af = audioFor(seg);
    const adur = probe(af);
    const dur = 0.15 + adur + 0.5;
    const variant = (seg.type === 'nick' ? nickCount++ : noraCount++) % 2 === 0 ? 'a' : 'b';
    push({ kind: 'face', who: seg.type, variant, dur, voice: { file: af, offset: 0.15, dur: adur }, seg });
  } else if (seg.type === 'asta') {
    const map = { 5: ['video/asta1.mp4', 1.0], 19: ['video/asta2.mp4', 0.8], 50: ['video/asta3.mp4', 0.4] };
    const [clip, from] = map[seg.id] || ['video/asta1.mp4', 0.8];
    push({ kind: 'clip', clip, dur: 3.5, from, voice: null });
  } else if (seg.type === 'broll') {
    const map = {
      terminal: [['images/broll_term0.png', 4.2], ['images/broll_term1.png', 3.3]],
      gui: [['images/broll_gui0.png', 7.0]],
      dashboard: [['images/broll_dash0.png', 5.0], ['images/broll_dash1.png', 3.0]],
    };
    push({ kind: 'broll', imgs: map[seg.cue], dur: map[seg.cue].reduce((a, [, d]) => a + d, 0), voice: null });
  }
}
push({ kind: 'title', img: 'images/title_end.png', dur: 8.0, voice: null });

// compute absolute start times
let t = 0;
for (const s of timeline) { s.start = t; t += s.dur; }
const TOTAL = t;
console.log(`total runtime: ${TOTAL.toFixed(1)}s (${(TOTAL / 60).toFixed(1)} min), ${timeline.length} segments`);

// ---------- segment renderers ----------
function renderTitle(i, s) {
  const out = `work/seg${String(i).padStart(3, '0')}.mp4`;
  run([
    '-loop', '1', '-framerate', FPS, '-t', s.dur + 1, '-i', s.img,
    '-filter_complex',
    `[0:v]scale=3840:2160,${kenburns(s.dur, W, H, 0.00018)},trim=duration=${s.dur},${FILM}`,
    '-an', '-c:v', 'libx264', '-crf', '20', '-preset', 'medium', '-r', FPS, out,
  ], `title ${i}`);
}

function renderClip(i, s) {
  const out = `work/seg${String(i).padStart(3, '0')}.mp4`;
  const args = ['-i', s.clip];
  let fc;
  if (s.full) {
    fc = `[0:v]scale=${W}:${H}:force_original_aspect_ratio=decrease:flags=lanczos,pad=${W}:${H}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps=${FPS}`;
    if (s.dur !== probe(s.clip)) fc += `,trim=duration=${s.dur}`;
    fc += `,${FILM}`;
  } else {
    fc = `[0:v]scale=${W}:${H}:force_original_aspect_ratio=decrease:flags=lanczos,pad=${W}:${H}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps=${FPS},` +
      `trim=start=${s.from || 0}:duration=${s.dur},setpts=PTS-STARTPTS,${FILM}`;
  }
  run([...args, '-filter_complex', fc, '-an', '-c:v', 'libx264', '-crf', '20', '-preset', 'medium', '-r', FPS, out], `clip ${i}`);
}

function renderBroll(i, s) {
  const out = `work/seg${String(i).padStart(3, '0')}.mp4`;
  const parts = s.imgs;
  const inputs = [];
  parts.forEach(([img]) => inputs.push('-loop', '1', '-framerate', FPS, '-t', '300', '-i', img));
  const chains = parts.map(([img, d], idx) => {
    const zoomDir = idx % 2 === 0 ? '1+0.00022*on' : 'max(1.10-0.00022*on,1.0)';
    return `[${idx}:v]scale=3840:2160,zoompan=z='min(${zoomDir},1.15)':x='(iw-iw/zoom)/2':y='(ih-ih/zoom)/2':d=1:fps=${FPS}:s=${W}x${H},trim=duration=${d},setpts=PTS-STARTPTS,setsar=1[v${idx}]`;
  });
  let fc = chains.join(';');
  if (parts.length === 1) fc += `;[v0]${FILM.replace(/^/, '')}`;
  else fc += `;[v0][v1]concat=n=2:v=1:a=0[vc];[vc]${FILM}`;
  if (parts.length === 1) fc = chains.join(';') + `;[v0]${FILM}`;
  const totalDur = parts.reduce((a, [, d]) => a + d, 0);
  run([...inputs, '-filter_complex', fc, '-t', totalDur, '-an', '-c:v', 'libx264', '-crf', '20', '-preset', 'medium', '-r', FPS, out], `broll ${i}`);
}

function renderFace(i, s) {
  const out = `work/seg${String(i).padStart(3, '0')}.mp4`;
  const isNick = s.who === 'nick';
  const clipFile = isNick ? `video/nick_${s.variant}.mp4` : `video/nora_${s.variant}.mp4`;
  const photo = isNick ? 'work/dave_1280.jpg' : 'work/deb_1280.jpg';
  const clipDur = probe(clipFile);
  const fgH = H; // full height
  const fgWnick = 1080; // square
  const fgWnora = 608;  // portrait

  const scaleFg = isNick
    ? `scale=${fgWnick}:${fgH}:force_original_aspect_ratio=increase,crop=${fgWnick}:${fgH}`
    : `scale=-2:${fgH}`; // portrait: width auto (~590)

  const common = `setsar=1,format=yuv420p`;

  if (s.dur <= clipDur + 0.8) {
    // single-clip path: trim or freeze-extend to exact duration
    const pad = Math.max(0, s.dur - clipDur);
    const fc =
      `[0:v]fps=${FPS},${scaleFg},settb=AVTB[fg];` +
      `[0:v]scale=${W}:${H}:force_original_aspect_ratio=increase,crop=${W}:${H},boxblur=22:2,eq=brightness=-0.16[g0];` +
      `[g0][fg]overlay=(W-w)/2:(H-h)/2,fps=${FPS}` +
      (pad > 0.05 ? `,tpad=stop_mode=clone:stop_duration=${pad.toFixed(3)}` : '') +
      `,trim=duration=${s.dur},${FILM}`;
    run(['-i', clipFile, '-filter_complex', fc, '-an', '-c:v', 'libx264', '-crf', '20', '-preset', 'medium', '-r', FPS, out], `face ${i}`);
  } else {
    // clip then crossfade into Ken Burns still
    const xfdur = 0.6;
    const kbDur = s.dur - clipDur + xfdur;
    let srcChain;
    if (isNick) {
      srcChain = `[1:v]scale=2400:2400:force_original_aspect_ratio=increase,crop=2400:2400`;
    } else {
      // portrait crop centered on Deb (face upper-center): full-height 9:16-ish window
      srcChain = `[1:v]crop=1272:2252:(iw-1272)/2:0,scale=1216:2152`; // then zoompan to fg
    }
    const kbOut = isNick ? `${fgWnick}x${fgH}` : `${fgWnora}x${fgH}`;
    const fc =
      `[0:v]fps=${FPS},${scaleFg},settb=AVTB,trim=duration=${clipDur},setpts=PTS-STARTPTS[cfg];` +
      `[0:v]scale=${W}:${H}:force_original_aspect_ratio=increase,crop=${W}:${H},boxblur=22:2,eq=brightness=-0.16[cbg];` +
      `[cbg][cfg]overlay=(W-w)/2:(H-h)/2,fps=${FPS},settb=AVTB[ccomp];` +
      `${srcChain},${kenburns(kbDur, ...(kbOut.split('x').map(Number)))},trim=duration=${kbDur.toFixed(3)},setpts=PTS-STARTPTS,settb=AVTB[kfg];` +
      `[1:v]scale=${W}:${H}:force_original_aspect_ratio=increase,crop=${W}:${H},boxblur=22:2,eq=brightness=-0.16[kbg];` +
      `[kbg][kfg]overlay=(W-w)/2:(H-h)/2,fps=${FPS},settb=AVTB[kcomp];` +
      `[ccomp][kcomp]xfade=transition=fade:duration=${xfdur}:offset=${(clipDur - xfdur).toFixed(3)},${FILM}`;
    run(['-i', clipFile, '-loop', '1', '-framerate', FPS, '-t', (kbDur + 2).toFixed(3), '-i', photo,
      '-filter_complex', fc, '-t', s.dur.toFixed(3), '-an', '-c:v', 'libx264', '-crf', '20', '-preset', 'medium', '-r', FPS, out], `face+kb ${i}`);
  }
}

// ---------- run ----------
const only = process.argv.includes('--only') ? parseInt(process.argv[process.argv.indexOf('--only') + 1]) : null;

if (only !== null) {
  const idx = timeline.findIndex((s) => s.seg && s.seg.id === only);
  console.log('rendering segment idx', idx);
  renderFace(idx, timeline[idx]);
  process.exit(0);
}

timeline.forEach((s, i) => {
  if (s.kind === 'title') renderTitle(i, s);
  else if (s.kind === 'clip') renderClip(i, s);
  else if (s.kind === 'broll') renderBroll(i, s);
  else renderFace(i, s);
  process.stdout.write(`rendered ${i + 1}/${timeline.length}\r`);
});
console.log('\nall segments rendered');

// concat
fs.writeFileSync('work/concat.txt', timeline.map((_, i) => `file 'seg${String(i).padStart(3, '0')}.mp4'`).join('\n'));
run(['-f', 'concat', '-safe', '0', '-i', 'work/concat.txt', '-c', 'copy', 'work/film_video.mp4'], 'concat');
console.log('video concatenated:', probe('work/film_video.mp4').toFixed(1), 's');

// save timeline for the audio mix step
fs.writeFileSync('work/timeline.json', JSON.stringify({ total: TOTAL, timeline: timeline.map((s) => ({ ...s, seg: s.seg ? { id: s.seg.id, type: s.seg.type } : null })) }, null, 1));
console.log('timeline saved. Now run: node mix.js');
