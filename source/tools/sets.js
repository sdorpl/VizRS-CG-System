// Wspólne dla tools/gallery.js i tools/video.js: lista paczek grafik i uruchamianie serwera VizRS z jedną paczką.
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

const ROOT = path.resolve(__dirname, '../..');
// [nazwa, plik show.json]
const SETS = [
  ['INFO24', path.join(ROOT, 'examples/archiwum/INFO24.show.json')],
  ['R7', path.join(ROOT, 'examples/archiwum/R7_NEWS.show.json')],
];
const BG = 'radial-gradient(ellipse at 60% 35%, #56657a 0%, #2b3442 45%, #15191f 100%)';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function loadPlaywright() {
  const dirs = [process.env.PLAYWRIGHT_DIR].filter(Boolean);
  for (const d of dirs) { try { return require(require.resolve('playwright', { paths: [d] })); } catch (e) { /* dalej */ } }
  return require('playwright');
}

/** Uruchamia VizRS na kopii paczki (tymczasowy katalog danych). Zwraca { B, show, notes, stop }. */
async function startSet(BIN, setName, file) {
  const port = 19000 + Math.floor(Math.random() * 900);
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-set-'));
  fs.copyFileSync(file, path.join(data, 'show.json'));
  const srv = spawn(BIN, ['--port', String(port), '--tcp', '0', '--data', data], { stdio: 'ignore' });
  const stop = () => { srv.kill(); fs.rmSync(data, { recursive: true, force: true }); };
  const B = `http://127.0.0.1:${port}`;
  try {
    for (let i = 0; i < 60; i++) { try { await fetch(B + '/api/state'); break; } catch (e) { await sleep(100); } }
    const notes = {};
    const show = await fetch(B + '/api/show').then((r) => r.json());
    return { B, show, notes, stop };
  } catch (e) { stop(); throw e; }
}

module.exports = { ROOT, SETS, BG, sleep, loadPlaywright, startSet };
