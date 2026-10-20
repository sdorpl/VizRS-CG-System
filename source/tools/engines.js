// Zgodność silników: Chromium (jak OBS/vMix) vs WebView okna natywnego (vizrs-shell: WebKitGTK / WebView2 / WKWebView).
//   node tools/engines.js [ścieżka-do-vizrs] [ścieżka-do-vizrs-shell] [--json wynik.json]
// Każda strona zestawów INFO24, R7 i BTB_PILNE jest brana na wyjście w obu silnikach; porównujemy dla każdego pola tekstowego
// ściśnięcie (scaleX), szerokość i wysokość w układzie 1920×1080. Różnica = Artist w oknie natywnym pokazałby co innego niż antena.
// Uwaga: vizrs-shell otwiera na kilka minut widoczne okno „VizRS Wyjście” (test lokalny, poza CI).
// Playwright: zmienna PLAYWRIGHT_DIR (katalog z node_modules/playwright) albo zwykłe `require('playwright')`.
const { spawn, spawnSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

function loadPlaywright() {
  const dirs = [process.env.PLAYWRIGHT_DIR, path.join(os.homedir(), 'Nextcloud/GitHub/BTB-CG-System-alpha')].filter(Boolean);
  for (const d of dirs) { try { return require(require.resolve('playwright', { paths: [d] })); } catch (e) { /* dalej */ } }
  return require('playwright');
}
const args = process.argv.slice(2);
const jsonAt = args.indexOf('--json');
const JSON_OUT = jsonAt >= 0 ? args.splice(jsonAt, 2)[1] : null;
const BIN = path.resolve(args[0] || path.join(__dirname, '../target/release/vizrs'));
const SHELL = path.resolve(args[1] || path.join(__dirname, '../shell/target/release/vizrs-shell'));
const ROOT = path.resolve(__dirname, '../..');
const SETS = [['INFO24', 'examples/INFO24.show.json'], ['R7', 'examples/R7_NEWS.show.json'], ['BTB_PILNE', 'examples/BTB_PILNE.show.json', 'examples/BTB_CG/assets']];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Sonda wykonywana na stronie wyjścia (w obu silnikach ten sam kod). W powłoce wynik wraca przez window.ipc.
const PROBE = `(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const st = document.createElement('style');
  st.textContent = '.vz-clock, .vz-ticker { visibility: hidden !important; }';
  document.head.append(st);
  await document.fonts.ready;
  const show = await (await fetch('/api/show')).json();
  const out = { ua: navigator.userAgent, pages: [] };
  for (const p of show.pages) {
    await fetch('/api/clear', { method: 'POST' }); await sleep(150);
    await fetch('/api/take/' + p.number, { method: 'POST' }); await sleep(2400);
    const k = Math.min(innerWidth / 1920, innerHeight / 1080);
    const nodes = [...document.querySelectorAll('.vz-node.vz-text')].filter((el) => !el.closest('.vz-ticker') && el.getBoundingClientRect().width > 0).map((el) => {
      const r = el.getBoundingClientRect();
      const sx = [el, ...el.querySelectorAll('*')].map((e) => /scaleX\\(([\\d.]+)\\)/.exec(e.style.transform || '')).filter(Boolean).map((m) => +(+m[1]).toFixed(3));
      return { t: el.textContent.trim().slice(0, 32), w: +(r.width / k).toFixed(1), h: +(r.height / k).toFixed(1), sx };
    });
    out.pages.push({ n: p.number, nodes });
  }
  await fetch('/api/clear', { method: 'POST' });
  const json = JSON.stringify(out);
  if (window.ipc) window.ipc.postMessage('probe:' + json); else window.__probe = json;
})();`;

(async () => {
  const { chromium } = loadPlaywright();
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-engines-'));
  const probeFile = path.join(tmp, 'probe.js');
  fs.writeFileSync(probeFile, PROBE);
  const browser = await chromium.launch();
  const res = {};
  let total = 0, bad = 0;
  try {
    for (const [name, file, assets] of SETS) {
      const data = path.join(tmp, name);
      fs.mkdirSync(path.join(data, 'assets'), { recursive: true });
      fs.copyFileSync(path.join(ROOT, file), path.join(data, 'show.json'));
      if (assets) for (const f of fs.readdirSync(path.join(ROOT, assets))) fs.copyFileSync(path.join(ROOT, assets, f), path.join(data, 'assets', f));
      const port = 19500 + Math.floor(Math.random() * 400);
      const srv = spawn(BIN, ['--port', String(port), '--tcp', '0', '--data', data], { stdio: 'ignore' });
      try {
        for (let i = 0; i < 50; i++) { try { await fetch(`http://127.0.0.1:${port}/health`); break; } catch (e) { await sleep(100); } }
        const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
        await page.goto(`http://127.0.0.1:${port}/output?checker=1&status=1`);
        await sleep(800);
        await page.evaluate(PROBE);
        await page.waitForFunction(() => window.__probe, null, { timeout: 600000, polling: 500 });
        const chrome = JSON.parse(await page.evaluate(() => window.__probe));
        await page.close();
        const out = path.join(data, 'shell.json');
        const r = spawnSync(SHELL, ['output', '--data', data, '--port', String(port), '--probe', probeFile, '--probe-out', out], { encoding: 'utf8', timeout: 600000 });
        if (!fs.existsSync(out)) { console.log(`FAIL ${name}: brak wyniku z vizrs-shell (${r.status}) ${r.stderr || ''}`); bad++; continue; }
        const shell = JSON.parse(fs.readFileSync(out, 'utf8'));
        res[name] = { chrome, shell };
        console.log(`${name}: ${chrome.pages.length} stron; WebView: ${shell.ua}`);
        chrome.pages.forEach((cp, i) => cp.nodes.forEach((c, j) => {
          total++;
          const w = shell.pages[i]?.nodes[j];
          const cs = c.sx[0] ?? 1, ws = w ? (w.sx[0] ?? 1) : NaN;
          const diff = !w || w.t !== c.t || Math.abs(cs - ws) > 0.01 || Math.abs(c.h - w.h) > 4 || Math.abs(c.w - w.w) / Math.max(1, c.w) > 0.01;
          if (diff) { bad++; console.log(`  RÓŻNICA ${name} ${cp.n} „${c.t}”: scaleX ${cs}→${ws}, w ${c.w}→${w?.w}, h ${c.h}→${w?.h}`); }
        }));
      } finally {
        srv.kill();
      }
    }
  } finally {
    await browser.close();
    if (JSON_OUT) fs.writeFileSync(JSON_OUT, JSON.stringify(res));
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  console.log(`\npól tekstowych: ${total}, różnych: ${bad}`);
  console.log(bad ? 'NIEZGODNE – Artist w oknie natywnym pokaże co innego niż antena' : 'Silniki zgodne');
  process.exit(bad ? 1 : 0);
})();
