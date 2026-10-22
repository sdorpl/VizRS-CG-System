// Porównanie wizualne paczki BTB CG (examples/archiwum/BTB_CG) z oryginałami:  node tests/e2e/btb-visual.js [ścieżka-do-vizrs] [katalog-zrzutów]
// Oryginał: element z katalogu wzorców renderowany rendererem wyjścia BTB CG (renderer.js + GSAP + fonty BTB) w Chromium.
// Wynik: strona VizRS po TAKE na /output. Oba zrzuty w stanie po wejściu (stop point), to samo tło i ta sama godzina.
// Różnica = piksele różniące się o > 60/255 (suma RGB) względem pikseli z treścią; tickery w ruchu oceniane łagodniej.
// Test lokalny (jak golden.js): wymaga BTB CG System (BTB_CG_DIR) z node_modules (playwright, gsap, handlebars).
// --frames: dodatkowo klatki w trakcie wejścia (25%, 50%, 75% czasu do stop pointu) – zegar obu stron zatrzymany i przesuwany razem,
//           więc GSAP w BTB i renderer VizRS pokazują tę samą chwilę; próg łagodniejszy (ruch = krawędzie w innym miejscu o ułamek px).
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { pathToFileURL } = require('url');

const BTB = path.resolve(process.env.BTB_CG_DIR || path.join(os.homedir(), 'Nextcloud/GitHub/BTB-CG-System-alpha'));
const SRC = path.resolve(process.env.BTB_ELEMENTS || path.join(os.homedir(), 'Nextcloud/GitHub/btb-cgeditor/CG elements converted'));
const PACK = path.resolve(__dirname, '../../../examples/archiwum/BTB_CG');
const FRAMES = process.argv.includes('--frames');
const args = process.argv.slice(2).filter((a) => a !== '--frames');
const BIN = path.resolve(args[0] || path.join(__dirname, '../../target/release/vizrs'));
const SHOTS = args[1] ? path.resolve(args[1]) : null;
const BG = '#303036';
const NOW = new Date('2026-10-19T12:34:56');
const LIMIT = 0.12, LIMIT_MOVING = 0.6, LIMIT_FRAME = 0.25;
// znane różnice sprawdzone na zestawieniach – wyższy próg z uzasadnieniem (plik → [próg, powód])
const KNOWN = {
  'MIASTO_prawa_—_Warszawa_(odzyskana).json': [0.2, 'BADGE: BTB ściska tekst wg długości (0,50), VizRS do szerokości tła (0,51) – ten sam napis, litery przesunięte o ułamki px'],
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const { chromium } = require(require.resolve('playwright', { paths: [BTB] }));

const MIME = { js: 'text/javascript', css: 'text/css', ttf: 'font/ttf', woff2: 'font/woff2', woff: 'font/woff', webp: 'image/webp', png: 'image/png', svg: 'image/svg+xml', jpg: 'image/jpeg' };

(async () => {
  const report = JSON.parse(fs.readFileSync(path.join(PACK, 'konwersja.json'), 'utf8'));
  const show = JSON.parse(fs.readFileSync(path.join(PACK, 'show.json'), 'utf8'));
  const io = await import(pathToFileURL(path.join(__dirname, '../../web/btb-io.js')).href);
  if (SHOTS) fs.mkdirSync(SHOTS, { recursive: true });

  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-btb-'));
  // podgląd BTB zatrzymuje taśmę tickera w chwili 0 (start od lewej) – w kopii testowej crawl od lewej stoi w miejscu, żeby porównać krój i odstępy
  const still = (n) => { if (n.type === 'ticker' && n.props.crawlFrom === 'left') n.props.speed = 0; (n.children || []).forEach(still); };
  const showTest = JSON.parse(JSON.stringify(show)); showTest.scenes.forEach((s) => s.nodes.forEach(still));
  fs.writeFileSync(path.join(data, 'show.json'), JSON.stringify(showTest));
  fs.cpSync(path.join(PACK, 'assets'), path.join(data, 'assets'), { recursive: true });
  const port = 19000 + Math.floor(Math.random() * 900);
  const srv = spawn(BIN, ['--port', String(port), '--tcp', '0', '--data', data, '--web', path.join(__dirname, '../../web')], { stdio: 'ignore' });
  const V = `http://127.0.0.1:${port}`;
  const browser = await chromium.launch();
  let failures = 0;
  const rows = [];
  try {
    for (let i = 0; i < 60; i++) { try { await fetch(V + '/api/state'); break; } catch (e) { await sleep(100); } }

    // --- BTB: strona z rendererem wyjścia, pliki serwowane z katalogu BTB CG
    const btb = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
    if (FRAMES) await btb.clock.install({ time: NOW }); else await btb.clock.setFixedTime(NOW);
    btb.on('pageerror', (e) => console.log('  [BTB pageerror]', String(e.message).slice(0, 160)));
    await btb.route('http://btb.test/**', (route) => {
      const rel = decodeURIComponent(new URL(route.request().url()).pathname).replace(/^\/+/, '');
      const map = { 'gsap.min.js': 'node_modules/gsap/dist/gsap.min.js', 'handlebars.min.js': 'node_modules/handlebars/dist/handlebars.min.js' };
      const file = path.join(BTB, map[rel] || rel);
      if (!file.startsWith(BTB) || !fs.existsSync(file)) return route.fulfill({ status: 404, body: '' });
      route.fulfill({ status: 200, contentType: MIME[file.split('.').pop().toLowerCase()] || 'application/octet-stream', body: fs.readFileSync(file) });
    });
    await btb.goto('http://btb.test/index-test.html').catch(() => {});
    await btb.setContent(`<!doctype html><html><head><meta charset="utf-8"><base href="http://btb.test/">
      <link rel="stylesheet" href="/fonts/fonts.css"></head><body style="margin:0;background:${BG};overflow:hidden">
      <div id="stage" style="position:fixed;left:0;top:0;width:1920px;height:1080px;overflow:hidden"></div>
      <script src="gsap.min.js"></script><script src="handlebars.min.js"></script><script src="cgmotion.js"></script>
      <script src="dock-model.js"></script><script src="renderer.js"></script></body></html>`, { waitUntil: 'load' });

    // --- VizRS: wyjście
    const viz = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
    if (FRAMES) await viz.clock.install({ time: NOW }); else await viz.clock.setFixedTime(NOW);
    viz.on('pageerror', (e) => console.log('  [VizRS pageerror]', String(e.message).slice(0, 160)));
    await viz.goto(`${V}/output?bg=${encodeURIComponent(BG)}`);
    await sleep(600);

    const compareFn = async ([a, b, bg, save, label]) => {
        const load = (s) => new Promise((ok) => { const i = new Image(); i.onload = () => ok(i); i.src = 'data:image/png;base64,' + s; });
        const [ia, ib] = await Promise.all([load(a), load(b)]);
        const W = 1920, H = 1080;
        const px = (img) => { const c = new OffscreenCanvas(W, H); const x = c.getContext('2d'); x.drawImage(img, 0, 0); return x.getImageData(0, 0, W, H).data; };
        const da = px(ia), db = px(ib);
        const br = parseInt(bg.slice(1, 3), 16), bgg = parseInt(bg.slice(3, 5), 16), bb = parseInt(bg.slice(5, 7), 16);
        const isBg = (d, p) => Math.abs(d[p] - br) + Math.abs(d[p + 1] - bgg) + Math.abs(d[p + 2] - bb) < 12;
        let n = 0, content = 0, x0 = W, y0 = H, x1 = -1, y1 = -1;
        const mask = new Uint8Array(W * H);
        for (let p = 0, k = 0; p < da.length; p += 4, k++) {
          const ca = !isBg(da, p), cb = !isBg(db, p);
          if (ca || cb) { content++; const x = k % W, y = (k / W) | 0; if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; }
          if (Math.abs(da[p] - db[p]) + Math.abs(da[p + 1] - db[p + 1]) + Math.abs(da[p + 2] - db[p + 2]) > 60) { n++; mask[k] = 1; }
        }
        let sheet = null;
        if (save && x1 >= 0) {
          // zestawienie: oryginał BTB / VizRS / różnice (czerwone), wycięte do obszaru treści z marginesem
          const m = 24; x0 = Math.max(0, x0 - m); y0 = Math.max(0, y0 - m); x1 = Math.min(W - 1, x1 + m); y1 = Math.min(H - 1, y1 + m);
          const w = x1 - x0 + 1, h = y1 - y0 + 1, lab = 26;
          const c = new OffscreenCanvas(w, (h + lab) * 3); const x = c.getContext('2d');
          x.fillStyle = '#111'; x.fillRect(0, 0, c.width, c.height);
          x.font = 'bold 18px sans-serif'; x.fillStyle = '#fff';
          x.fillText('BTB CG (oryginał) – ' + label, 6, 19); x.drawImage(ia, x0, y0, w, h, 0, lab, w, h);
          x.fillText('VizRS (konwersja)', 6, h + lab + 19); x.drawImage(ib, x0, y0, w, h, 0, h + 2 * lab, w, h);
          x.fillText('różnice', 6, 2 * h + 2 * lab + 19);
          x.globalAlpha = 0.35; x.drawImage(ia, x0, y0, w, h, 0, 2 * h + 3 * lab, w, h); x.globalAlpha = 1;
          const id = x.getImageData(0, 2 * h + 3 * lab, w, h);
          for (let yy = 0; yy < h; yy++) for (let xx = 0; xx < w; xx++) if (mask[(yy + y0) * W + xx + x0]) { const q = (yy * w + xx) * 4; id.data[q] = 255; id.data[q + 1] = 40; id.data[q + 2] = 40; id.data[q + 3] = 255; }
          x.putImageData(id, 0, 2 * h + 3 * lab);
          const blob = await c.convertToBlob({ type: 'image/png' });
          const u8 = new Uint8Array(await blob.arrayBuffer()); let s = ''; for (let i = 0; i < u8.length; i += 0x8000) s += String.fromCharCode(...u8.subarray(i, i + 0x8000));
          sheet = btoa(s);
        }
        return { ratio: content ? n / content : n ? 1 : 0, content, sheet };
};
    const compare = (a, b, save, label) => viz.evaluate(compareFn, [a.toString('base64'), b.toString('base64'), BG, save, label]);
    let clockT = 0; // przesunięcie zegara (ms) w trybie --frames
    for (const r of report) {
      const scene = show.scenes.find((s) => s.id === r.scene);
      const wait = Math.round(((scene.stops[0] ?? scene.duration) + 1.2) * 1000);
      const buf = fs.readFileSync(path.join(SRC, r.file));
      const { elements } = await io.readBtb(/\.cgpack$/i.test(r.file) ? new Uint8Array(buf) : buf.toString('utf8'), r.file);
      const el = { ...elements[0], visible: true };

      if (FRAMES) {
        // zegar stoi: wyczyść, wystartuj obie strony w tej samej chwili, potem przesuwaj zegar razem
        await fetch(V + '/api/clear', { method: 'POST' });
        await sleep(150);
        clockT += 60000;
        await Promise.all([btb.clock.pauseAt(new Date(NOW.getTime() + clockT)), viz.clock.pauseAt(new Date(NOW.getTime() + clockT))]);
        await fetch(`${V}/api/take/${r.page}`, { method: 'POST' });
        for (let i = 0; i < 60 && !(await viz.evaluate(() => !!document.querySelector('.vz-scene'))); i++) await sleep(50);
        await btb.evaluate((el) => { const st = document.getElementById('stage'); window.__cgRenderer.renderPreview(st, [], {}, {}); st.innerHTML = ''; window.__cgRenderer.renderPreview(st, [el], {}, {}); }, el);
        const stopMs = Math.round((scene.stops[0] ?? scene.duration) * 1000);
        let at = 0;
        for (const f of [0.25, 0.5, 0.75]) {
          const target = Math.round(stopMs * f);
          await Promise.all([btb.clock.runFor(target - at), viz.clock.runFor(target - at)]);
          at = target;
          const [fa, fb] = await Promise.all([btb.screenshot(), viz.screenshot()]);
          const fr = await compare(fa, fb, !!SHOTS, `${Math.round(f * 100)}% wejścia (${(target / 1000).toFixed(2)} s)`);
          const ok = r.type === 'TICKER' || fr.ratio <= LIMIT_FRAME;
          if (!ok) failures++;
          if (SHOTS && fr.sheet) fs.writeFileSync(path.join(SHOTS, `${r.page}_${r.scene}_f${Math.round(f * 100)}.png`), Buffer.from(fr.sheet, 'base64'));
          console.log(`  ${ok ? 'pass' : 'FAIL'} ${r.page} klatka ${Math.round(f * 100)}%: różnica ${(fr.ratio * 100).toFixed(1)}%`);
        }
        await Promise.all([btb.clock.runFor(wait - at), viz.clock.runFor(wait - at)]);
        clockT += wait;
      } else {
        await btb.evaluate((el) => { const st = document.getElementById('stage'); window.__cgRenderer.renderPreview(st, [], {}, {}); st.innerHTML = ''; window.__cgRenderer.renderPreview(st, [el], {}, {}); }, el);
        await fetch(V + '/api/clear', { method: 'POST' });
        await sleep(200);
        await fetch(`${V}/api/take/${r.page}`, { method: 'POST' });
        await sleep(wait);
      }
      await btb.evaluate(() => document.fonts.ready);
      const [a, b] = await Promise.all([btb.screenshot(), viz.screenshot()]);
      const moving = r.type === 'TICKER';
      const res = await compare(a, b, !!SHOTS, 'po wejściu');
      const known = KNOWN[r.file];
      const good = res.content > 0 && res.ratio <= (known ? known[0] : moving ? LIMIT_MOVING : LIMIT);
      if (!good) failures++;
      if (SHOTS && res.sheet) fs.writeFileSync(path.join(SHOTS, `${r.page}_${r.scene}.png`), Buffer.from(res.sheet, 'base64'));
      rows.push({ page: r.page, file: r.file, ratio: res.ratio, moving, ok: good });
      console.log(`${good ? 'PASS' : 'FAIL'} ${r.page} ${r.file}: różnica ${(res.ratio * 100).toFixed(1)}%${moving ? ' (ticker w ruchu)' : ''}${known ? ` (znana różnica: ${known[1]})` : ''}${res.content ? '' : ' – pusty zrzut'}`);
    }
  } finally {
    await browser.close();
    srv.kill();
    fs.rmSync(data, { recursive: true, force: true });
  }
  if (SHOTS) fs.writeFileSync(path.join(SHOTS, 'wyniki.json'), JSON.stringify(rows, null, 1));
  console.log(failures ? `\n${failures} elementów z różnicą powyżej progu` : `\nWszystkie ${rows.length} elementów zgodne z oryginałami BTB CG`);
  process.exit(failures ? 1 : 0);
})();
