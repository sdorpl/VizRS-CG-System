// Zgodność renderera natywnego (source/render, vizrs-render) z Chromium – strona po stronie, ten sam show.
//   node tools/native-compare.js [show.json] [--vizrs target/release/vizrs] [--render render/target/release/vizrs-render]
//        [--out katalog] [--pages 1001,3004] [--limit 0.02]
// Chromium: /output po TAKE i zakończonym wejściu (stop point 0); natywnie: `vizrs-render frame --page N --stop 0`.
// Zegary i tickery są ukryte po obu stronach (zmienne w czasie). Sceny, których renderer natywny nie obsługuje
// (`vizrs-render check`), są pomijane; strony z brakiem wykrytym dopiero w danych (np. `[[ ]]`) – opisane, bez oceny. Wynik: odsetek pikseli różniących się o > 24/255 (na tle #303036)
// i trójki obrazów Chromium | natywnie | różnica w katalogu --out. Kod wyjścia 1, gdy któraś strona przekracza --limit.
const { spawn, spawnSync, execFileSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

function loadPlaywright() {
  const dirs = [process.env.PLAYWRIGHT_DIR, path.join(os.homedir(), 'Nextcloud/GitHub/BTB-CG-System-alpha')].filter(Boolean);
  for (const d of dirs) { try { return require(require.resolve('playwright', { paths: [d] })); } catch (e) { /* dalej */ } }
  return require('playwright');
}
const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf('--' + k); return i >= 0 ? args[i + 1] : d; };
const SRC = path.resolve(__dirname, '..');
const ROOT = path.resolve(SRC, '..');
const SHOW = path.resolve(args.find((a, i) => !a.startsWith('--') && !(i > 0 && args[i - 1].startsWith('--'))) || path.join(SRC, 'web/demo_show.json'));
const VIZRS = path.resolve(opt('vizrs', path.join(SRC, 'target/release/vizrs')));
const RENDER = path.resolve(opt('render', path.join(SRC, 'render/target/release/vizrs-render')));
const OUT = opt('out', '');
const LIMIT = +opt('limit', '0.02');
const ONLY = opt('pages', '') ? opt('pages', '').split(',') : null;
// --tickers: tickery widoczne (Chromium 2,6 s po TAKE, natywnie te same 2,6 s od TAKE w rytmie 25 kl./s)
const TICKERS = args.includes('--tickers');
const ASSETS = path.resolve(opt('assets', path.join(ROOT, 'examples/archiwum/assets')));
const FONTS = path.join(SRC, 'web/fonts');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const show = JSON.parse(fs.readFileSync(SHOW, 'utf8'));
  // sceny obsługiwane natywnie (raport `check`)
  const report = execFileSync(RENDER, ['check', '--show', SHOW, '--fonts', FONTS, '--assets', ASSETS], { encoding: 'utf8' });
  const native = new Set(report.split('\n').filter((l) => l.startsWith('OK ')).map((l) => l.slice(3).trim()));
  const sceneOf = (p) => show.scenes.find((s) => s.id === p.scene || s.name === p.scene);
  const pages = show.pages.filter((p) => (!ONLY || ONLY.includes(String(p.number))) && native.has(sceneOf(p)?.name || sceneOf(p)?.id));
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-native-'));
  fs.copyFileSync(SHOW, path.join(data, 'show.json'));
  if (fs.existsSync(ASSETS)) fs.cpSync(ASSETS, path.join(data, 'assets'), { recursive: true });
  const port = 19000 + Math.floor(Math.random() * 900);
  const srv = spawn(VIZRS, ['--port', String(port), '--tcp', '0', '--data', data], { stdio: 'ignore' });
  const B = `http://127.0.0.1:${port}`;
  if (OUT) fs.mkdirSync(OUT, { recursive: true });
  const { chromium } = loadPlaywright();
  const browser = await chromium.launch();
  let worst = 0, fails = 0;
  try {
    for (let i = 0; i < 80; i++) { try { await fetch(B + '/api/state'); break; } catch (e) { await sleep(100); } }
    const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
    await page.goto(B + '/output');
    // ukrycie z potomkami: przegródki roll/wipe same ustawiają sobie visibility: visible
    await page.addStyleTag({ content: TICKERS ? '.vz-clock, .vz-clock * { visibility: hidden !important; }' : '.vz-clock, .vz-clock *, .vz-ticker, .vz-ticker * { visibility: hidden !important; }' });
    await sleep(800);
    for (const p of pages) {
      await fetch(B + '/api/clear', { method: 'POST' });
      await sleep(200);
      await fetch(`${B}/api/take/${p.number}`, { method: 'POST' });
      await sleep(2600);
      const chrome = await page.screenshot({ omitBackground: true });
      const file = path.join(data, `n${p.number}.png`);
      const run = spawnSync(RENDER, ['frame', '--show', SHOW, '--page', String(p.number), ...(TICKERS ? ['--time', '2.6', '--hide', 'clock'] : ['--stop', '0', '--hide', 'clock,ticker']),
        '--fonts', FONTS, '--assets', ASSETS, '-o', file], { encoding: 'utf8' });
      if (run.status !== 0) throw new Error(`vizrs-render frame ${p.number}: ${run.stderr}`);
      // braki zgłoszone dopiero przy danych strony (np. `[[wyróżnienie]]` w polu) – na antenie strona poszłaby do Chromium
      const gaps = run.stderr.split('\n').filter((l) => l.includes('poza rendererem natywnym')).map((l) => l.replace(/^.*natywnym: /, ''));
      const res = await page.evaluate(async ([a, b]) => {
        const load = (u) => new Promise((ok, no) => { const i = new Image(); i.onload = () => ok(i); i.onerror = no; i.src = u; });
        const [ia, ib] = await Promise.all([load('data:image/png;base64,' + a), load('data:image/png;base64,' + b)]);
        const W = ia.width, H = ia.height;
        const flat = (img) => { const k = document.createElement('canvas'); k.width = W; k.height = H; const x = k.getContext('2d'); x.fillStyle = '#303036'; x.fillRect(0, 0, W, H); x.drawImage(img, 0, 0); return x.getImageData(0, 0, W, H); };
        const A = flat(ia), Bm = flat(ib);
        const k = document.createElement('canvas'); k.width = W * 3; k.height = H; const x = k.getContext('2d');
        x.putImageData(A, 0, 0); x.putImageData(Bm, W, 0);
        const D = x.createImageData(W, H);
        let diff = 0, sum = 0;
        for (let i = 0; i < A.data.length; i += 4) {
          const d = Math.abs(A.data[i] - Bm.data[i]) + Math.abs(A.data[i + 1] - Bm.data[i + 1]) + Math.abs(A.data[i + 2] - Bm.data[i + 2]);
          sum += d;
          if (d > 24) diff++;
          D.data[i] = Math.min(255, d * 2); D.data[i + 1] = d > 24 ? 40 : 0; D.data[i + 2] = 0; D.data[i + 3] = 255;
        }
        x.putImageData(D, W * 2, 0);
        return { ratio: diff / (W * H), mean: sum / (W * H * 3), png: k.toDataURL('image/png').split(',')[1] };
      }, [chrome.toString('base64'), fs.readFileSync(file).toString('base64')]);
      if (OUT) fs.writeFileSync(path.join(OUT, `${p.number}.png`), Buffer.from(res.png, 'base64'));
      if (gaps.length) {
        console.log(`CHROMIUM ${p.number} ${sceneOf(p)?.name}: ${(res.ratio * 100).toFixed(2)}% pikseli (${gaps.join('; ')})`);
        continue;
      }
      worst = Math.max(worst, res.ratio);
      const ok = res.ratio <= LIMIT;
      if (!ok) fails++;
      console.log(`${ok ? 'ZGODNE  ' : 'RÓŻNE   '} ${p.number} ${sceneOf(p)?.name}: ${(res.ratio * 100).toFixed(2)}% pikseli, średnio ${res.mean.toFixed(3)}/255`);
    }
  } finally {
    await browser.close();
    const gone = new Promise((r) => srv.once('exit', r));
    srv.kill();
    await gone;
    fs.rmSync(data, { recursive: true, force: true, maxRetries: 5 });
  }
  console.log(`\n${pages.length} stron natywnie, największa różnica ${(worst * 100).toFixed(2)}%${fails ? `, ${fails} powyżej ${(LIMIT * 100).toFixed(1)}%` : ''}`);
  process.exit(fails ? 1 : 0);
})();
