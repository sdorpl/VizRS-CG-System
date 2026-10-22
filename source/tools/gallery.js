// Galeria wszystkich grafik zestawów przykładowych (INFO24, R7, paczka BTB_CG, belka BTB wielowierszowa):
//   node tools/gallery.js [ścieżka-do-vizrs] [--out katalog]
// Każda strona jest brana osobno na czyste wyjście, zrzut po zakończeniu animacji wejścia.
// Wynik (poza repo, domyślnie <tmp>/vizrs-galeria): <ZESTAW>-<numer>.png, arkusze <ZESTAW>-arkusz.png i przegląd index.html.
// Playwright: zmienna PLAYWRIGHT_DIR (katalog z node_modules/playwright) albo zwykłe `require('playwright')`.
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
const OUT = path.resolve(outIdx >= 0 ? args[outIdx + 1] : path.join(os.tmpdir(), 'vizrs-galeria'));
const BIN = path.resolve(args.find((a, i) => !a.startsWith('--') && args[i - 1] !== '--out') || path.join(__dirname, '../target/release/vizrs'));
// [nazwa, show.json, katalogi assets (opcjonalnie)]
const SETS = [
  ['INFO24', 'examples/archiwum/INFO24.show.json'],
  ['R7', 'examples/archiwum/R7_NEWS.show.json'],
  ['BTB_CG', 'examples/archiwum/BTB_CG/show.json', 'examples/archiwum/BTB_CG/assets'],
  ['BTB_PILNE', 'examples/archiwum/BTB_PILNE.show.json', 'examples/archiwum/BTB_CG/assets'],
  ['BTB_ALPHA', 'examples/archiwum/BTB_ALPHA/show.json', 'examples/archiwum/BTB_CG/assets', 'examples/archiwum/BTB_ALPHA/assets'],
  ['BTB_TRIO', 'examples/archiwum/BTB_TRIO/show.json', 'examples/archiwum/BTB_CG/assets', 'examples/archiwum/BTB_TRIO/assets'],
];
const BG = 'radial-gradient(ellipse at 60% 35%, #56657a 0%, #2b3442 45%, #15191f 100%)';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

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
  for (const [setName, file, ...assets] of SETS) {
    const port = 19000 + Math.floor(Math.random() * 900);
    const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-gal-'));
    fs.copyFileSync(path.join(ROOT, file), path.join(data, 'show.json'));
    for (const a of assets) fs.cpSync(path.join(ROOT, a), path.join(data, 'assets'), { recursive: true });
    const srv = spawn(BIN, ['--port', String(port), '--tcp', '0', '--data', data], { stdio: 'ignore' });
    const B = `http://127.0.0.1:${port}`;
    const cards = [];
    try {
      for (let i = 0; i < 60; i++) { try { await fetch(B + '/api/state'); break; } catch (e) { await sleep(100); } }
      const show = await fetch(B + '/api/show').then((r) => r.json());
      const scenes = Object.fromEntries(show.scenes.map((s) => [s.id, s]));
      const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
      await page.goto(B + '/output');
      await page.addStyleTag({ content: `html, body { background: ${BG} !important; }` });
      await sleep(800);
      for (const p of [...show.pages].sort((a, b) => a.number - b.number)) {
        await fetch(B + '/api/clear', { method: 'POST' });
        await sleep(300);
        await fetch(`${B}/api/take/${p.number}`, { method: 'POST' });
        await sleep(2600);
        const name = `${setName}-${p.number}.png`;
        fs.writeFileSync(path.join(OUT, name), await page.screenshot());
        const sc = scenes[p.scene] || {};
        cards.push({ file: name, number: p.number, name: p.name, scene: sc.name || p.scene, layer: sc.layer || 'main', types: nodeTypes(sc.nodes), values: p.values || {} });
        console.log(`ZAPISANO ${name}  ${p.name}`);
      }
      await page.close();
    } finally { srv.kill(); fs.rmSync(data, { recursive: true, force: true }); }

    // arkusz zbiorczy zestawu (siatka 3 kolumny, miniatury 640×360)
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
    sections.push({ setName, file, cards });
  }
  await browser.close();

  const html = `<!doctype html><html lang="pl"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>VizRS – galeria grafik</title>
<style>
body{margin:0;background:#0b0d10;color:#e8eaed;font:14px/1.45 system-ui,sans-serif}
header{padding:24px 24px 8px}h1{margin:0 0 4px;font-size:22px}h2{margin:28px 24px 12px;font-size:18px}a{color:#7fb3ff}
.muted{color:#8a929c}.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(420px,1fr));gap:16px;padding:0 24px 24px}
.card{background:#15191f;border:1px solid #232a33;border-radius:8px;overflow:hidden}
.card img{display:block;width:100%;aspect-ratio:16/9;cursor:zoom-in}.meta{padding:10px 12px}
.num{color:#f5a800;font-weight:700;margin-right:6px}.tags{font-size:12px;color:#8a929c;margin-top:4px}
details{margin-top:6px;font-size:12px}pre{white-space:pre-wrap;margin:4px 0 0;color:#b8c0ca}
#lb{position:fixed;inset:0;background:#000d;display:none;align-items:center;justify-content:center}#lb img{max-width:96vw;max-height:94vh}
</style></head><body>
<header><h1>VizRS – wszystkie elementy grafiki</h1>
<div class="muted">Każda strona pokazana osobno po animacji wejścia (wyjście 1920×1080, zrzut 960×540). Wygenerowano: <code>node source/tools/gallery.js</code>.</div></header>
${sections.map((s) => `<h2>${s.setName} <span class="muted">(${s.cards.length} stron · ${esc(s.file)} · <a href="${s.setName}-arkusz.png">arkusz</a>)</span></h2>
<div class="grid">${s.cards.map((c) => `<div class="card"><img loading="lazy" src="${c.file}" alt="${esc(c.name)}">
<div class="meta"><span class="num">${c.number}</span>${esc(c.name)}
<div class="tags">scena: ${esc(c.scene)} · warstwa: ${esc(c.layer)}<br>węzły: ${esc(c.types)}</div>
${Object.keys(c.values).length ? `<details><summary>wartości pól</summary><pre>${esc(Object.entries(c.values).map(([k, v]) => `${k}: ${typeof v === 'string' ? v : JSON.stringify(v)}`).join('\n'))}</pre></details>` : ''}
</div></div>`).join('')}</div>`).join('')}
<div id="lb" onclick="this.style.display='none'"><img></div>
<script>document.querySelectorAll('.card img').forEach(i=>i.onclick=()=>{const l=document.getElementById('lb');l.firstChild.src=i.src;l.style.display='flex'})</script>
</body></html>`;
  fs.writeFileSync(path.join(OUT, 'index.html'), html);
  console.log(`\nGaleria: ${path.join(OUT, 'index.html')}`);
})();
