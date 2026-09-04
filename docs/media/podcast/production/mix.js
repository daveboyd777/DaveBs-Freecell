// Audio mix: place each voice line at its timeline offset, pan Nick slightly
// left / Nora slightly right, add a quiet film-hiss bed, loudnorm to -16 LUFS.
const { execFileSync } = require('child_process');
const fs = require('fs');

const { total, timeline } = JSON.parse(fs.readFileSync('work/timeline.json', 'utf8'));

// Per-speaker gain trim (dB): Christopher at -12Hz+rubberband runs a touch quiet
const GAIN = { nick: 2.5, nora: 0.0 };
const PAN = { nick: [0.78, 0.34], nora: [0.34, 0.78] }; // L/R weights

const voices = timeline.filter((s) => s.voice);
console.log(`${voices.length} voice lines, total ${(total / 60).toFixed(1)} min`);

const inputs = [];
const chains = [];
voices.forEach((s, i) => {
  const who = s.seg.type;
  const [l, r] = PAN[who];
  const delayMs = Math.round((s.start + s.voice.offset) * 1000);
  inputs.push('-i', s.voice.file);
  chains.push(
    `[${i}:a]aresample=48000,aformat=sample_fmts=fltp:channel_layouts=mono,` +
    `volume=${GAIN[who]}dB,pan=stereo|c0=${l}*c0|c1=${r}*c0,` +
    `adelay=${delayMs}|${delayMs}[v${i}]`
  );
});

// film hiss bed: pink noise, lowpassed, very quiet
const bedIdx = voices.length;
inputs.push('-f', 'lavfi', '-t', total.toFixed(3), '-i', 'anoisesrc=color=pink:amplitude=0.028:seed=1934');
chains.push(`[${bedIdx}:a]lowpass=f=6500,highpass=f=120,pan=stereo|c0=0.8*c0|c1=0.8*c0,volume=0.5[bed]`);

const mixLabels = voices.map((_, i) => `[v${i}]`).join('') + '[bed]';
const fc = chains.join(';') + `;${mixLabels}amix=inputs=${voices.length + 1}:normalize=0:duration=longest,` +
  `loudnorm=I=-16:TP=-1.5:LRA=11,aresample=48000,aformat=sample_fmts=fltp:channel_layouts=stereo[out]`;

execFileSync('ffmpeg', ['-y', '-v', 'error', ...inputs,
  '-filter_complex', fc, '-map', '[out]', '-t', total.toFixed(3),
  '-c:a', 'aac', '-b:a', '192k', 'work/mix.m4a'], { stdio: ['ignore', 'ignore', 'inherit'] });
console.log('mix done:', (parseFloat(execFileSync('ffprobe', ['-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', 'work/mix.m4a']).toString()) ).toFixed(1), 's');
