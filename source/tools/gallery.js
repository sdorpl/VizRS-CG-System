// Galeria wszystkich elementów grafiki (stron i scen) zestawów INFO24 i R7 oraz szablonów BTB CG (tests/fixtures/btb):
//   node tools/gallery.js [ścieżka-do-vizrs] [--out katalog]
// Szablony BTB są importowane jak w Artist (btb-io.js → /api/scenes + /api/pages), z raportem konwersji.
// Każda strona jest brana osobno na czyste wyjście, zrzut po zakończeniu animacji wejścia.
// Wynik: <out>/<ZESTAW>-<numer>.png, arkusze <out>/<ZESTAW>-arkusz.png i przegląd <out>/index.html.
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

function loadPlaywright() {
  const dirs = [process.env.PLAYWRIGHT_DIR, path.join(os.homedir(), 'Nextcloud/GitHub/BTB-CG-System-alpha')].filter(Boolean);
  for (const d of dirs) { try { return require(require.resolve('playwright', { paths: [d] })); } catch (e) { /* dalej */ } }
  return require('playwright');
}
const args = process.argv.slice(2);
const outIdx = args.indexOf('--out');
const ROOT = path.resolve(__dirname, '../..');
const OUT = path.resolve(outIdx >= 0 ? args[outIdx + 1] : path.join(ROOT, 'media/galeria'));
const BIN = path.resolve(args.find((a, i) => !a.startsWith('--') && args[i - 1] !== '--out') || path.join(__dirname, '../target/release/vizrs'));
const SETS = [
  ['INFO24', path.join(ROOT, 'examples/INFO24.show.json')],
  ['R7', path.join(ROOT, 'examples/R7_NEWS.show.json')],
  ['BTB', path.join(ROOT, 'source/tests/fixtures/btb')],
  ['BTB-PILNE', path.join(ROOT, 'examples/BTB_PILNE.show.json')],
];
const KNOWN_FONTS = ['Oswald', 'Barlow Condensed', 'Inter', 'Roboto Condensed', 'Montserrat', 'Roboto'];
const BG = 'radial-gradient(ellipse at 60% 35%, #56657a 0%, #2b3442 45%, #15191f 100%)';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

// Import szablonów BTB CG (.json v3 / .cgpack) do pustego show – ta sama ścieżka co „Import BTB” w Artist.
async function importBtb(B, dir) {
  const io = await import(path.join(ROOT, 'source/web/btb-io.js'));
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

function nodeTypes(nodes) {
  const n = {};
  const walk = (list) => (list || []).forEach((x) => { n[x.type] = (n[x.type] || 0) + 1; walk(x.children); });
  walk(nodes);
  return Object.entries(n).map(([t, c]) => (c > 1 ? `${t}×${c}` : t)).join(' · ');
}

(async () => {
  const { chromium } = loadPlaywright();
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch();
  const sections = [];
  for (const [setName, file] of SETS) {
    const port = 19000 + Math.floor(Math.random() * 900);
    const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-gal-'));
    const isDir = fs.statSync(file).isDirectory();
    if (isDir) fs.writeFileSync(path.join(data, 'show.json'), JSON.stringify({ name: setName, scenes: [], pages: [], playlists: [] }));
    else fs.copyFileSync(file, path.join(data, 'show.json'));
    const srv = spawn(BIN, ['--port', String(port), '--tcp', '0', '--data', data], { stdio: 'ignore' });
    const B = `http://127.0.0.1:${port}`;
    const cards = [];
    try {
      for (let i = 0; i < 60; i++) { try { await fetch(B + '/api/state'); break; } catch (e) { await sleep(100); } }
      const notes = isDir ? await importBtb(B, file) : {};
      const show = await fetch(B + '/api/show').then((r) => r.json());
      const scenes = Object.fromEntries(show.scenes.map((s) => [s.id, s]));
      const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
      await page.goto(B + '/output');
      await page.addStyleTag({ content: `html, body { background: ${BG} !important; }` });
      await sleep(500);
      const pages = [...show.pages].sort((a, b) => a.number - b.number);
      for (const p of pages) {
        await fetch(B + '/api/clear', { method: 'POST' });
        await sleep(300);
        await fetch(`${B}/api/take/${p.number}`, { method: 'POST' });
        await sleep(2600);
        const name = `${setName}-${p.number}.png`;
        fs.writeFileSync(path.join(OUT, name), await page.screenshot());
        const sc = scenes[p.scene] || {};
        cards.push({ file: name, number: p.number, name: p.name, scene: sc.name || p.scene, layer: sc.layer || 'main', types: nodeTypes(sc.nodes), values: p.values || {}, notes: notes[p.scene] || [] });
        console.log(`ZAPISANO ${name}  ${p.name}`);
      }
      await page.close();
    } finally { srv.kill(); fs.rmSync(data, { recursive: true, force: true }); }

    // Arkusz zbiorczy zestawu (siatka 3 kolumny, miniatury 640×360)
    const cols = 3, tw = 640, th = 360, gap = 12, lab = 30;
    const rows = Math.ceil(cards.length / cols);
    const sheet = await browser.newPage({ viewport: { width: cols * tw + (cols + 1) * gap, height: rows * (th + lab) + (rows + 1) * gap } });
    await sheet.setContent(`<html><body style="margin:0;background:#0b0d10;font:600 15px system-ui,sans-serif;color:#e8eaed">
      <div style="display:grid;grid-template-columns:repeat(${cols},${tw}px);gap:${gap}px;padding:${gap}px">
      ${cards.map((c) => `<div><img style="display:block;width:${tw}px;height:${th}px" src="data:image/png;base64,${fs.readFileSync(path.join(OUT, c.file)).toString('base64')}">
        <div style="height:${lab}px;line-height:${lab}px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis"><span style="color:#f5a800">${c.number}</span> ${esc(c.name)} <span style="color:#8a929c;font-weight:400">— ${esc(c.scene)}</span></div></div>`).join('')}
      </div></body></html>`);
    await sheet.screenshot({ path: path.join(OUT, `${setName}-arkusz.png`) });
    await sheet.close();
    console.log(`ZAPISANO ${setName}-arkusz.png`);
    sections.push({ setName, file: path.relative(ROOT, file), cards });
  }
  await browser.close();

  const html = `<!doctype html><html lang="pl"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>VizRS – galeria grafik</title>
<style>
body{margin:0;background:#0b0d10;color:#e8eaed;font:14px/1.45 system-ui,sans-serif}
header{padding:24px 24px 8px}h1{margin:0 0 4px;font-size:22px}h2{margin:28px 24px 12px;font-size:18px}
.muted{color:#8a929c}.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(420px,1fr));gap:16px;padding:0 24px 24px}
.card{background:#15191f;border:1px solid #232a33;border-radius:8px;overflow:hidden}
.card img{display:block;width:100%;aspect-ratio:16/9;cursor:zoom-in}.meta{padding:10px 12px}
.num{color:#f5a800;font-weight:700;margin-right:6px}.tags{font-size:12px;color:#8a929c;margin-top:4px}
details{margin-top:6px;font-size:12px}pre{white-space:pre-wrap;margin:4px 0 0;color:#b8c0ca}
#lb{position:fixed;inset:0;background:#000d;display:none;align-items:center;justify-content:center}#lb img{max-width:96vw;max-height:94vh}
</style></head><body>
<header><h1>VizRS – wszystkie elementy grafiki</h1>
<div class="muted">Zestawy INFO24 i R7, szablony BTB CG zaimportowane do VizRS (tests/fixtures/btb) oraz belka BTB w wersji wielowierszowej (examples/BTB_PILNE.show.json).</div>
<div class="muted">Każda strona zestawu pokazana osobno po animacji wejścia (wyjście 1920×1080, zrzut 960×540). Wygenerowano: <code>node source/tools/gallery.js</code>.</div></header>
${sections.map((s) => `<h2>${s.setName} <span class="muted">(${s.cards.length} stron · ${esc(s.file)} · <a style="color:#7fb3ff" href="${s.setName}-arkusz.png">arkusz</a>)</span></h2>
<div class="grid">${s.cards.map((c) => `<div class="card"><img loading="lazy" src="${c.file}" alt="${esc(c.name)}">
<div class="meta"><span class="num">${c.number}</span>${esc(c.name)}
<div class="tags">scena: ${esc(c.scene)} · warstwa: ${esc(c.layer)}<br>węzły: ${esc(c.types)}</div>
${c.notes.length ? `<details><summary>raport konwersji BTB (${c.notes.length - 1})</summary><pre>${esc(c.notes.join('\n'))}</pre></details>` : ''}
${Object.keys(c.values).length ? `<details><summary>wartości pól</summary><pre>${esc(Object.entries(c.values).map(([k, v]) => `${k}: ${typeof v === 'string' ? v : JSON.stringify(v)}`).join('\n'))}</pre></details>` : ''}
</div></div>`).join('')}</div>`).join('')}
<div id="lb" onclick="this.style.display='none'"><img></div>
<script>document.querySelectorAll('.card img').forEach(i=>i.onclick=()=>{const l=document.getElementById('lb');l.firstChild.src=i.src;l.style.display='flex'})</script>
</body></html>`;
  fs.writeFileSync(path.join(OUT, 'index.html'), html);
  console.log(`\nGaleria: ${path.join(OUT, 'index.html')}`);
})();
