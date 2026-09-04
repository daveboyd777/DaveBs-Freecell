const { execFileSync } = require('child_process');
const path = require('path');
const edge = 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
const shots = [
  ['html/title_open.html', 'images/title_open.png'],
  ['html/title_end.html', 'images/title_end.png'],
  ['html/broll_terminal.html', 'images/broll_term0.png'],
  ['html/broll_terminal.html?s=1', 'images/broll_term1.png'],
  ['html/broll_gui.html', 'images/broll_gui0.png'],
  ['html/broll_gui.html?s=1', 'images/broll_gui1.png'],
  ['html/broll_dashboard.html', 'images/broll_dash0.png'],
  ['html/broll_dashboard.html?s=1', 'images/broll_dash1.png'],
];
for (const [src, out] of shots) {
  const q = src.indexOf('?');
  const abs = path.resolve(q >= 0 ? src.slice(0, q) : src).replace(/\\/g, '/');
  const url = 'file:///' + abs + (q >= 0 ? src.slice(q) : '');
  execFileSync(edge, [
    '--headless', '--disable-gpu', '--force-device-scale-factor=2',
    '--window-size=1920,1080', '--screenshot=' + path.resolve(out), url,
  ], { stdio: 'ignore' });
  console.log('shot', out);
}
