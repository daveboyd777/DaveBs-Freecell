// Analyze Dave's voice sample: median fundamental frequency (F0), estimated
// speaking rate, and level — used to pick and tune the Nick TTS voice.
// Usage: node analyze-voice.js [path/to/sample.wav]
// The reference recording is personal audio and was never checked into the
// repo (see docs/media/podcast/AGENT_PLAN.md §1) -- supply your own via a
// CLI arg or the VOICE_SAMPLE_PATH env var; the default below only matches
// the original build machine's layout as a documented example.
const fs = require('fs');

const path = process.argv[2] || process.env.VOICE_SAMPLE_PATH || 'C:/Users/daveboyd/Desktop/davevoicesample.wav';
if (!fs.existsSync(path)) {
  console.error(`Voice sample not found: ${path}`);
  console.error('Pass a path as the first argument, or set VOICE_SAMPLE_PATH.');
  process.exit(1);
}
const buf = fs.readFileSync(path);

// Parse WAV header (PCM 16-bit mono 44.1k per ffprobe)
const riff = buf.toString('ascii', 0, 4);
if (riff !== 'RIFF') throw new Error('not a wav');
const channels = buf.readUInt16LE(22);
const sr = buf.readUInt32LE(24);
const bits = buf.readUInt16LE(34);
console.log(`format: ${bits}bit ${channels}ch ${sr}Hz`);

// Walk chunks to find "data"
let dataStart = -1;
let off = 12;
while (off + 8 <= buf.length) {
  const id = buf.toString('ascii', off, off + 4);
  const size = buf.readUInt32LE(off + 4);
  if (id === 'data') { dataStart = off + 8; break; }
  off += 8 + size + (size % 2);
}
if (dataStart < 0) throw new Error('no data chunk');
const nSamples = Math.floor((buf.length - dataStart) / 2);

// Read as float mono
const x = new Float32Array(nSamples);
for (let i = 0; i < nSamples; i++) x[i] = buf.readInt16LE(dataStart + i * 2) / 32768;

// Frame the signal, autocorrelation pitch detection per frame
const frameLen = Math.round(0.04 * sr);   // 40 ms
const hop = Math.round(0.02 * sr);        // 20 ms hop
const minLag = Math.floor(sr / 400);      // 400 Hz ceiling
const maxLag = Math.floor(sr / 60);       // 60 Hz floor

const f0s = [];
const rmsVals = [];
for (let start = 0; start + frameLen < nSamples; start += hop) {
  // RMS gate
  let energy = 0;
  for (let i = 0; i < frameLen; i++) energy += x[start + i] * x[start + i];
  const rms = Math.sqrt(energy / frameLen);
  rmsVals.push(rms);
  if (rms < 0.02) continue; // silence gate

  // Remove mean, autocorrelate
  let mean = 0;
  for (let i = 0; i < frameLen; i++) mean += x[start + i];
  mean /= frameLen;

  let bestLag = -1, bestVal = 0;
  let e0 = 0;
  for (let i = 0; i < frameLen; i++) { const v = x[start + i] - mean; e0 += v * v; }
  if (e0 < 1e-6) continue;
  for (let lag = minLag; lag <= maxLag; lag++) {
    let sum = 0;
    for (let i = 0; i < frameLen - lag; i++) sum += (x[start + i] - mean) * (x[start + i + lag] - mean);
    const norm = sum / e0;
    if (norm > bestVal) { bestVal = norm; bestLag = lag; }
  }
  if (bestLag > 0 && bestVal > 0.45) f0s.push(sr / bestLag);
}

f0s.sort((a, b) => a - b);
const median = f0s[Math.floor(f0s.length / 2)];
const p10 = f0s[Math.floor(f0s.length * 0.1)];
const p90 = f0s[Math.floor(f0s.length * 0.9)];
const activeFrames = rmsVals.filter(v => v > 0.02).length;
const voicedPct = (100 * f0s.length / Math.max(1, activeFrames)).toFixed(1);

console.log(`voiced frames detected: ${f0s.length} (${voicedPct}% of active frames)`);
console.log(`median F0: ${median.toFixed(1)} Hz  [p10 ${p10.toFixed(1)} - p90 ${p90.toFixed(1)}]`);
console.log(`duration: ${(nSamples / sr).toFixed(1)}s, active speech: ${(activeFrames * 0.02).toFixed(1)}s`);

// Rough classification guide
if (median < 100) console.log('register: deep male');
else if (median < 125) console.log('register: male');
else if (median < 145) console.log('register: male, higher');
else if (median < 175) console.log('register: borderline male/female');
else console.log('register: female range');
