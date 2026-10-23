// Zrzuty wzorcowe Outputu (F0.3) dla zestawów INFO24 i R7:  node tests/e2e/golden.js [ścieżka-do-vizrs] [--update]
// Porównanie pikselowe w przeglądarce (bez zależności): próg = odsetek pikseli różniących się o > 24/255.
// Zegary i tickery są ukrywane (zmienne w czasie). Wzorce zależą od maszyny/fontów – to test lokalny, poza CI.
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

function loadPlaywright() {
  const dirs = [process.env.PLAYWRIGHT_DIR].filter(Boolean);
  for (const d of dirs) { try { return require(require.resolve('playwright', { paths: [d] })); } catch (e) { /* dalej */ } }
  return require('playwright');
}
const args = process.argv.slice(2);
const UPDATE = args.includes('--update');
const BIN = path.resolve(args.find((a) => !a.startsWith('--')) || path.join(__dirname, '../../target/release/vizrs'));
const ROOT = path.resolve(__dirname, '../../..');
const GOLD = path.join(__dirname, '../golden');
const SETS = [['INFO24', path.join(ROOT, 'examples/archiwum/INFO24.show.json')], ['R7', path.join(ROOT, 'examples/archiwum/R7_NEWS.show.json')]];
const THRESH = 0.004;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let failures = 0;

(async () => {
  const { chromium } = loadPlaywright();
  fs.mkdirSync(GOLD, { recursive: true });
  const browser = await chromium.launch();
  for (const [setName, file] of SETS) {
    const port = 19000 + Math.floor(Math.random() * 900);
    const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-gold-'));
    fs.copyFileSync(file, path.join(data, 'show.json'));
    const srv = spawn(BIN, ['--port', String(port), '--tcp', '0', '--data', data], { stdio: 'ignore' });
    const B = `http://127.0.0.1:${port}`;
    try {
      for (let i = 0; i < 60; i++) { try { await fetch(B + '/api/state'); break; } catch (e) { await sleep(100); } }
      const show = await fetch(B + '/api/show').then((r) => r.json());
      const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
      await page.goto(B + '/output?bg=%23303036');
      await page.addStyleTag({ content: '.vz-clock, .vz-ticker { visibility: hidden !important; }' });
      await sleep(500);
      for (const p of show.pages) {
        await fetch(B + '/api/clear', { method: 'POST' });
        await sleep(150);
        await fetch(`${B}/api/take/${p.number}`, { method: 'POST' });
        await sleep(2400);
        const png = await page.screenshot();
        const name = path.join(GOLD, `${setName}-${p.number}.png`);
        if (UPDATE || !fs.existsSync(name)) { fs.writeFileSync(name, png); console.log('ZAPISANO ' + path.basename(name)); continue; }
        const ratio = await page.evaluate(async ([a, b]) => {
          const load = (u) => new Promise((res, rej) => { const i = new Image(); i.onload = () => res(i); i.onerror = rej; i.src = u; });
          const [ia, ib] = await Promise.all([load('data:image/png;base64,' + a), load('data:image/png;base64,' + b)]);
          const c = (img) => { const k = document.createElement('canvas'); k.width = img.width; k.height = img.height; const x = k.getContext('2d'); x.drawImage(img, 0, 0); return x.getImageData(0, 0, k.width, k.height); };
          const A = c(ia), Bm = c(ib);
          if (A.width !== Bm.width || A.height !== Bm.height) return 1;
          let diff = 0;
          for (let k = 0; k < A.data.length; k += 4) {
            if (Math.abs(A.data[k] - Bm.data[k]) + Math.abs(A.data[k + 1] - Bm.data[k + 1]) + Math.abs(A.data[k + 2] - Bm.data[k + 2]) > 24) diff++;
          }
          return diff / (A.width * A.height);
        }, [fs.readFileSync(name).toString('base64'), png.toString('base64')]);
        const good = ratio <= THRESH;
        if (!good) { failures++; fs.writeFileSync(name.replace('.png', '.actual.png'), png); }
        console.log((good ? 'PASS ' : 'FAIL ') + `${setName} ${p.number} ${p.name} (różnica ${(ratio * 100).toFixed(2)}%)`);
      }
      await page.close();
    } finally { srv.kill(); fs.rmSync(data, { recursive: true, force: true }); }
  }
  await browser.close();
  console.log(failures ? `\n${failures} różnic powyżej progu (pliki *.actual.png)` : '\nZrzuty wzorcowe zgodne');
  process.exit(failures ? 1 : 0);
})();
