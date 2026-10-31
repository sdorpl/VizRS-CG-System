// Szerokości tekstu: Chromium (`getBoundingClientRect` w /output) i renderer natywny (`render/examples/measure`)
// dla fontów wbudowanych, grubości, rozmiarów (też ułamkowych z wyszukiwania `shrink`), odstępów i kursywy.
//   (cd render && cargo build --release --example measure) && node tools/native-widths.js [vizrs]
// Wynik: największe różnice i średnia |różnica| w px. Kod wyjścia 1, gdy któraś różnica przekracza 0,1%.
const { spawn, execFileSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { loadPlaywright, sleep } = require('./sets');

const SRC = path.resolve(__dirname, '..');
const BIN = path.resolve(process.argv[2] || path.join(SRC, 'target/release/vizrs'));
const MEASURE = path.join(SRC, 'render/target/release/examples/measure');
const fonts = ['Inter', 'Oswald', 'Barlow Condensed', 'Barlow', 'Roboto', 'Roboto Condensed', 'Montserrat', 'Bebas Neue', 'Roboto Mono'];
const texts = ['RZĄD PRZEDSTAWIŁ PROJEKT BUDŻETU NA 2027', 'Zażółć gęślą jaźń „cytat” – 0123', 'AVATAR WAVE Type To', 'ekonomistów Utrudnienia na autostradzie A2'];
const cases = [];
for (const f of fonts) for (const w of [400, 700]) for (const sz of [18, 23.9336, 39.9297, 46, 64.5]) for (const t of texts) cases.push([f, w, false, sz, 0, t]);
for (const sp of [-2, 0.5, 4]) cases.push(['Oswald', 400, false, 40, sp, texts[0]]);
cases.push(['Inter', 400, true, 40, 0, texts[1]], ['Oswald', 400, true, 40, 0, texts[1]], ['Bebas Neue', 700, false, 44, 0, texts[0]]);

(async () => {
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-widths-'));
  const port = 19000 + Math.floor(Math.random() * 900);
  const srv = spawn(BIN, ['--port', String(port), '--tcp', '0', '--data', data], { stdio: 'ignore' });
  const { chromium } = loadPlaywright();
  const browser = await chromium.launch();
  try {
    for (let i = 0; i < 60; i++) { try { await fetch(`http://127.0.0.1:${port}/api/state`); break; } catch (e) { await sleep(100); } }
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${port}/output`);
    await sleep(500);
    const chrome = await page.evaluate(async (cases) => {
      const out = [];
      for (const [f, w, it, sz, sp, t] of cases) {
        await document.fonts.load(`${it ? 'italic' : 'normal'} ${w} ${sz}px "${f}"`, t); // z tekstem: także podzbiór latin-ext
        const s = document.createElement('span');
        Object.assign(s.style, { position: 'absolute', whiteSpace: 'pre', fontFamily: `"${f}", "Barlow Condensed", Inter, Arial, sans-serif`, fontWeight: String(w), fontStyle: it ? 'italic' : 'normal', fontSize: sz + 'px', letterSpacing: sp + 'px' });
        s.textContent = t;
        document.body.appendChild(s);
        out.push(s.getBoundingClientRect().width);
        s.remove();
      }
      return out;
    }, cases);
    const nat = JSON.parse(execFileSync(MEASURE, { input: JSON.stringify(cases), cwd: path.join(SRC, 'render') }));
    const rows = cases.map((c, i) => ({ c, chrome: chrome[i], nat: nat[i], d: nat[i] - chrome[i] })).sort((a, b) => Math.abs(b.d) - Math.abs(a.d));
    for (const r of rows.slice(0, 10)) console.log(`${r.d.toFixed(3).padStart(8)} px  Chromium ${r.chrome.toFixed(3)}  natywnie ${r.nat.toFixed(3)}  ${JSON.stringify(r.c)}`);
    const worst = Math.max(...rows.map((r) => Math.abs(r.d) / r.chrome));
    console.log(`\n${rows.length} pomiarów, średnio |różnica| ${(rows.reduce((s, r) => s + Math.abs(r.d), 0) / rows.length).toFixed(3)} px, najwięcej ${(worst * 100).toFixed(3)}%`);
    process.exitCode = worst > 0.001 ? 1 : 0;
  } finally {
    await browser.close();
    srv.kill();
    await sleep(200);
    fs.rmSync(data, { recursive: true, force: true });
  }
})();
