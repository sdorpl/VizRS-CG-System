// Galeria wszystkich elementów grafiki (stron i scen) ze wszystkich paczek z tools/sets.js (INFO24, R7, szablony BTB CG, belka BTB wielowierszowa):
//   node tools/gallery.js [ścieżka-do-vizrs] [--out katalog]
// Szablony BTB są importowane jak w Artist (btb-io.js → /api/scenes + /api/pages), z raportem konwersji.
// Każda strona jest brana osobno na czyste wyjście, zrzut po zakończeniu animacji wejścia.
// Wynik: <out>/<ZESTAW>-<numer>.png, arkusze <out>/<ZESTAW>-arkusz.png i przegląd <out>/index.html.
const fs = require('fs');
const path = require('path');
const { ROOT, SETS, BG, sleep, loadPlaywright, startSet } = require('./sets');

const args = process.argv.slice(2);
const outIdx = args.indexOf('--out');
const OUT = path.resolve(outIdx >= 0 ? args[outIdx + 1] : path.join(ROOT, 'media/galeria'));
const BIN = path.resolve(args.find((a, i) => !a.startsWith('--') && args[i - 1] !== '--out') || path.join(__dirname, '../target/release/vizrs'));
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
  for (const [setName, file] of SETS) {
    const { B, show, notes, stop } = await startSet(BIN, setName, file);
    const cards = [];
    try {
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
    } finally { stop(); }

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
