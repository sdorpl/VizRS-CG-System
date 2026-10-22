// Wspólne dla tools/gallery.js i tools/video.js: lista paczek grafik i uruchamianie serwera VizRS z jedną paczką.
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { pathToFileURL } = require('url');

const ROOT = path.resolve(__dirname, '../..');
// [nazwa, plik show.json albo katalog z elementami BTB CG (.json v3 / .cgpack)]
const SETS = [
  ['INFO24', path.join(ROOT, 'examples/archiwum/INFO24.show.json')],
  ['R7', path.join(ROOT, 'examples/archiwum/R7_NEWS.show.json')],
  ['BTB', path.join(ROOT, 'source/tests/fixtures/btb')],
  ['BTB-PILNE', path.join(ROOT, 'examples/archiwum/BTB_PILNE.show.json')],
];
const KNOWN_FONTS = ['Oswald', 'Barlow Condensed', 'Inter', 'Roboto Condensed', 'Montserrat', 'Roboto'];
const BG = 'radial-gradient(ellipse at 60% 35%, #56657a 0%, #2b3442 45%, #15191f 100%)';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function loadPlaywright() {
  const dirs = [process.env.PLAYWRIGHT_DIR, path.join(os.homedir(), 'Nextcloud/GitHub/BTB-CG-System-alpha')].filter(Boolean);
  for (const d of dirs) { try { return require(require.resolve('playwright', { paths: [d] })); } catch (e) { /* dalej */ } }
  return require('playwright');
}

// Import szablonów BTB CG (.json v3 / .cgpack) do pustego show – ta sama ścieżka co „Import BTB” w Artist.
async function importBtb(B, dir) {
  const io = await import(pathToFileURL(path.join(ROOT, 'source/web/btb-io.js')).href);
  const post = async (url, body, raw) => {
    const r = await fetch(B + url, { method: 'POST', body: raw ? body : JSON.stringify(body), headers: raw ? {} : { 'Content-Type': 'application/json' } });
    const j = await r.json().catch(() => ({}));
    if (!r.ok) throw new Error(`${url}: ${j.error || r.status}`);
    return j;
  };
  const warnings = {};
  for (const name of fs.readdirSync(dir).filter((n) => /\.(json|cgpack)$/i.test(n)).sort()) {
    const buf = fs.readFileSync(path.join(dir, name));
    const { elements, files } = await io.readBtb(name.endsWith('.json') ? buf.toString('utf8') : buf, name);
    for (const el of elements) {
      const used = new Set();
      const r = io.convertElement(el, {
        knownFonts: KNOWN_FONTS,
        assetName: (src) => { const k = Object.keys(files).find((n) => n === src || n === src.replace(/^\//, '')); if (!k) return null; used.add(k); return k.split('/').pop(); },
      });
      for (const k of used) await post('/api/assets?name=' + encodeURIComponent(k.split('/').pop()), files[k], true);
      const saved = await post('/api/scenes', r.scene);
      await post('/api/pages', { scene: saved.id, name: r.page.name, values: r.page.values });
      warnings[saved.id] = [`plik: ${name}`, ...r.warnings];
    }
  }
  return warnings;
}

/** Uruchamia VizRS na kopii paczki (tymczasowy katalog danych). Zwraca { B, show, notes, stop }. */
async function startSet(BIN, setName, file) {
  const port = 19000 + Math.floor(Math.random() * 900);
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-set-'));
  const isDir = fs.statSync(file).isDirectory();
  if (isDir) fs.writeFileSync(path.join(data, 'show.json'), JSON.stringify({ name: setName, scenes: [], pages: [], playlists: [] }));
  else fs.copyFileSync(file, path.join(data, 'show.json'));
  const srv = spawn(BIN, ['--port', String(port), '--tcp', '0', '--data', data], { stdio: 'ignore' });
  const stop = () => { srv.kill(); fs.rmSync(data, { recursive: true, force: true }); };
  const B = `http://127.0.0.1:${port}`;
  try {
    for (let i = 0; i < 60; i++) { try { await fetch(B + '/api/state'); break; } catch (e) { await sleep(100); } }
    const notes = isDir ? await importBtb(B, file) : {};
    const show = await fetch(B + '/api/show').then((r) => r.json());
    return { B, show, notes, stop };
  } catch (e) { stop(); throw e; }
}

module.exports = { ROOT, SETS, BG, sleep, loadPlaywright, startSet };
