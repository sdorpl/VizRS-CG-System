// Film ze wszystkimi paczkami grafik (tools/sets.js): każda strona TAKE → na antenie → (CONTINUE) → OUT.
//   node tools/video.js [ścieżka-do-vizrs] [--out plik.mp4] [--only R7,BTB] [--fps 25]
// Nagranie klatka po klatce w czasie wirtualnym (Playwright page.clock: performance.now, requestAnimationFrame):
// zegar strony przesuwa się dokładnie o 1/fps na klatkę, więc film jest płynny niezależnie od wydajności
// maszyny i braku GPU (renderowanie programowe w trybie headless). Klatki trafiają wprost do ffmpeg (H.264).
const { spawn } = require('child_process');
const path = require('path');
const { ROOT, SETS, BG, sleep, loadPlaywright, startSet } = require('./sets');

const args = process.argv.slice(2);
const opt = (n) => { const i = args.indexOf(n); return i >= 0 ? args[i + 1] : null; };
const OUT = path.resolve(opt('--out') || path.join(ROOT, 'media/vizrs_WSZYSTKIE_PACZKI.mp4'));
const ONLY = opt('--only') ? opt('--only').split(',') : null;
const FPS = Math.max(1, +opt('--fps') || 25);
const BIN = path.resolve(args.find((a, i) => !a.startsWith('--') && !['--out', '--only', '--fps'].includes(args[i - 1])) || path.join(__dirname, '../target/release/vizrs'));
const TITLES = { INFO24: 'INFO24 – oprawa demo', R7: 'R7 NEWS', BTB: 'Szablony BTB CG (import do VizRS)', 'BTB-PILNE': 'Belka BTB „PILNE” – wiele linii' };

const hasType = (nodes, types) => (nodes || []).some((n) => types.includes(n.type) || hasType(n.children, types));
/** Czas na antenie (s): tickery i rotacje dłużej, żeby było widać zmianę wiadomości. */
function holdFor(sc) {
  if (hasType(sc.nodes, ['ticker'])) return 9;
  if (JSON.stringify(sc.nodes).includes('"cycle":')) return 7;
  return 3.5;
}

(async () => {
  const { chromium } = loadPlaywright();
  const ff = spawn('ffmpeg', ['-v', 'error', '-y', '-f', 'image2pipe', '-framerate', String(FPS), '-c:v', 'mjpeg', '-i', '-',
    '-vf', 'scale=1920:1080,format=yuv420p', '-c:v', 'libx264', '-preset', 'slow', '-crf', '22', '-r', String(FPS), '-movflags', '+faststart', OUT], { stdio: ['pipe', 'inherit', 'inherit'] });
  const ffDone = new Promise((res, rej) => ff.on('close', (c) => (c === 0 ? res() : rej(new Error('ffmpeg: kod ' + c)))));
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  let frames = 0;
  /** Nagrywa `sec` sekund: zegar strony +1/fps, zrzut, do ffmpeg. */
  const record = async (sec, each) => {
    const n = Math.round(sec * FPS);
    for (let i = 0; i < n; i++) {
      if (each) await each(i / n);
      await page.clock.runFor(1000 / FPS);
      const jpg = await page.screenshot({ type: 'jpeg', quality: 92 });
      if (!ff.stdin.write(jpg)) await new Promise((r) => ff.stdin.once('drain', r));
      frames++;
    }
  };
  // zdarzenie z serwera (SSE) przychodzi w czasie rzeczywistym – krótko czekamy, zanim ruszy zegar wirtualny
  const command = async (url) => { await fetch(url, { method: 'POST' }); await sleep(250); };
  // overlay z opisem (jak w filmach w media/) i plansza tytułowa paczki – dodawane do każdego wyjścia
  const overlay = () => page.evaluate((bg) => {
    document.documentElement.style.background = document.body.style.background = bg;
    const lab = document.createElement('div');
    lab.id = 'vlab';
    lab.style.cssText = 'position:fixed;left:16px;top:14px;z-index:99999;padding:8px 14px;background:#0b0d10d9;border-left:4px solid #f5a800;border-radius:4px;color:#fff;font:700 22px Inter,system-ui,sans-serif;display:none';
    lab.innerHTML = '<div id="vlab1"></div><div id="vlab2" style="font-weight:500;font-size:16px;color:#b8c0ca"></div>';
    const card = document.createElement('div');
    card.id = 'vcard';
    card.style.cssText = 'position:fixed;inset:0;z-index:99998;display:none;align-items:center;justify-content:center;flex-direction:column;background:#0b0d10;color:#fff;font:800 72px Inter,system-ui,sans-serif';
    card.innerHTML = '<div id="vcard1"></div><div id="vcard2" style="font-weight:500;font-size:30px;color:#8a929c;margin-top:18px"></div>';
    document.body.append(lab, card);
  }, BG);
  const label = (a, b) => page.evaluate(([a, b]) => {
    const l = document.getElementById('vlab');
    l.style.display = a ? 'block' : 'none';
    document.getElementById('vlab1').textContent = a || '';
    document.getElementById('vlab2').textContent = b || '';
  }, [a, b]);
  const cardOpacity = (o) => page.evaluate((o) => { const c = document.getElementById('vcard'); c.style.display = o > 0 ? 'flex' : 'none'; c.style.opacity = String(o); }, o);
  const titleCard = async (a, b) => {
    await page.evaluate(([a, b]) => { document.getElementById('vcard1').textContent = a; document.getElementById('vcard2').textContent = b; }, [a, b]);
    await cardOpacity(1);
    await record(2.2);
    await record(0.5, (u) => cardOpacity(1 - u)); // przenikanie sterowane klatkami (przejścia CSS nie podlegają zegarowi wirtualnemu)
    await cardOpacity(0);
  };

  await page.clock.install(); // obowiązuje też po kolejnych page.goto
  try {
    for (const [setName, file] of SETS) {
      if (ONLY && !ONLY.includes(setName)) continue;
      const { B, show, stop } = await startSet(BIN, setName, file);
      try {
        await page.goto(B + '/output');
        await overlay();
        await sleep(800); // połączenie SSE i fonty (czas rzeczywisty)
        const pages = [...show.pages].sort((a, b) => a.number - b.number);
        const scenes = Object.fromEntries(show.scenes.map((s) => [s.id, s]));
        await titleCard(TITLES[setName] || setName, `${pages.length} ${pages.length === 1 ? 'strona' : 'stron'} · ${path.relative(ROOT, file)}`);
        for (const p of pages) {
          const sc = scenes[p.scene] || {};
          await label(`TAKE ${p.number}`, `${p.name} — ${sc.name || p.scene}`);
          await command(`${B}/api/take/${p.number}`);
          await record(holdFor(sc));
          for (let s = 1; s < (sc.stops || []).length; s++) {
            await label(`CONTINUE ${p.number}`, `${p.name} — punkt ${s + 1}`);
            await command(`${B}/api/continue/${p.number}`);
            await record(2.5);
          }
          await label(`OUT ${p.number}`, p.name);
          await command(`${B}/api/out/${p.number}`);
          await record(1.6);
          await command(`${B}/api/clear`);
          console.log(`${setName} ${p.number} ${p.name}`);
        }
        await label('', '');
        await record(0.3);
      } finally { stop(); }
    }
  } finally {
    await browser.close();
    ff.stdin.end();
  }
  await ffDone;
  console.log(`\nFilm: ${OUT} (${frames} klatek, ${(frames / FPS).toFixed(1)} s, ${FPS} kl./s)`);
})();
