// Film ze wszystkimi paczkami grafik (tools/sets.js): każda strona TAKE → na antenie → (CONTINUE) → OUT.
//   node tools/video.js [ścieżka-do-vizrs] [--out plik.mp4] [--only R7,BTB]
// Klatki z Chromium (screencast CDP, 1920×1080) składa ffmpeg w H.264 25 kl./s. Wymaga Playwright i ffmpeg.
const { execFileSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { ROOT, SETS, BG, sleep, loadPlaywright, startSet } = require('./sets');

const args = process.argv.slice(2);
const opt = (n) => { const i = args.indexOf(n); return i >= 0 ? args[i + 1] : null; };
const OUT = path.resolve(opt('--out') || path.join(ROOT, 'media/vizrs_WSZYSTKIE_PACZKI.mp4'));
const ONLY = opt('--only') ? opt('--only').split(',') : null;
const BIN = path.resolve(args.find((a, i) => !a.startsWith('--') && !['--out', '--only'].includes(args[i - 1])) || path.join(__dirname, '../target/release/vizrs'));
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
  const frames = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-video-'));
  const shots = [];
  let cdp = null;
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  // overlay z opisem (jak w filmach w media/) i plansza tytułowa paczki – dodawane do każdego wyjścia
  const overlay = async () => page.evaluate((bg) => {
    document.documentElement.style.background = document.body.style.background = bg;
    const lab = document.createElement('div');
    lab.id = 'vlab';
    lab.style.cssText = 'position:fixed;left:16px;top:14px;z-index:99999;padding:8px 14px;background:#0b0d10d9;border-left:4px solid #f5a800;border-radius:4px;color:#fff;font:700 22px Inter,system-ui,sans-serif;display:none';
    lab.innerHTML = '<div id="vlab1"></div><div id="vlab2" style="font-weight:500;font-size:16px;color:#b8c0ca"></div>';
    const card = document.createElement('div');
    card.id = 'vcard';
    card.style.cssText = 'position:fixed;inset:0;z-index:99998;display:none;align-items:center;justify-content:center;flex-direction:column;background:#0b0d10;color:#fff;font:800 72px Inter,system-ui,sans-serif;transition:opacity .4s';
    card.innerHTML = '<div id="vcard1"></div><div id="vcard2" style="font-weight:500;font-size:30px;color:#8a929c;margin-top:18px"></div>';
    document.body.append(lab, card);
  }, BG);
  const label = (a, b) => page.evaluate(([a, b]) => {
    const l = document.getElementById('vlab');
    l.style.display = a ? 'block' : 'none';
    document.getElementById('vlab1').textContent = a || '';
    document.getElementById('vlab2').textContent = b || '';
  }, [a, b]);
  const titleCard = async (a, b) => {
    await page.evaluate(([a, b]) => {
      const c = document.getElementById('vcard');
      document.getElementById('vcard1').textContent = a; document.getElementById('vcard2').textContent = b;
      c.style.opacity = '1'; c.style.display = 'flex';
    }, [a, b]);
    await sleep(2200);
    await page.evaluate(() => { document.getElementById('vcard').style.opacity = '0'; });
    await sleep(500);
    await page.evaluate(() => { document.getElementById('vcard').style.display = 'none'; });
  };

  const startCapture = async () => {
    cdp = await page.context().newCDPSession(page);
    cdp.on('Page.screencastFrame', ({ data, metadata, sessionId }) => {
      const f = path.join(frames, `f${String(shots.length).padStart(6, '0')}.jpg`);
      fs.writeFileSync(f, Buffer.from(data, 'base64'));
      shots.push({ f, t: metadata.timestamp });
      cdp.send('Page.screencastFrameAck', { sessionId }).catch(() => {});
    });
    await cdp.send('Page.startScreencast', { format: 'jpeg', quality: 92, maxWidth: 1920, maxHeight: 1080, everyNthFrame: 1 });
  };
  const stopCapture = async () => { if (cdp) { await cdp.send('Page.stopScreencast').catch(() => {}); await cdp.detach().catch(() => {}); cdp = null; } };

  try {
    for (const [setName, file] of SETS) {
      if (ONLY && !ONLY.includes(setName)) continue;
      const { B, show, stop } = await startSet(BIN, setName, file);
      try {
        await stopCapture();
        await page.goto(B + '/output');
        await overlay();
        await sleep(600);
        await startCapture();
        const pages = [...show.pages].sort((a, b) => a.number - b.number);
        const scenes = Object.fromEntries(show.scenes.map((s) => [s.id, s]));
        await titleCard(TITLES[setName] || setName, `${pages.length} ${pages.length === 1 ? 'strona' : 'stron'} · ${path.relative(ROOT, file)}`);
        for (const p of pages) {
          const sc = scenes[p.scene] || {};
          await label(`TAKE ${p.number}`, `${p.name} — ${sc.name || p.scene}`);
          await fetch(`${B}/api/take/${p.number}`, { method: 'POST' });
          await sleep(holdFor(sc) * 1000);
          for (let s = 1; s < (sc.stops || []).length; s++) {
            await label(`CONTINUE ${p.number}`, `${p.name} — punkt ${s + 1}`);
            await fetch(`${B}/api/continue/${p.number}`, { method: 'POST' });
            await sleep(2500);
          }
          await label(`OUT ${p.number}`, p.name);
          await fetch(`${B}/api/out/${p.number}`, { method: 'POST' });
          await sleep(1600);
          await fetch(`${B}/api/clear`, { method: 'POST' });
          await sleep(200);
          console.log(`${setName} ${p.number} ${p.name}`);
        }
        await label('', '');
        await sleep(300);
        await stopCapture(); // przed zatrzymaniem serwera – bez klatek rozłączonego wyjścia
      } finally { stop(); }
    }
    await stopCapture();
  } finally { await browser.close(); }

  // klatki o zmiennym odstępie (screencast wysyła tylko zmiany) → stałe 25 kl./s
  const list = shots.map((s, i) => `file '${s.f}'\nduration ${Math.max(0.001, ((shots[i + 1]?.t ?? s.t + 0.5) - s.t)).toFixed(4)}`).join('\n');
  fs.writeFileSync(path.join(frames, 'list.txt'), `ffconcat version 1.0\n${list}\nfile '${shots[shots.length - 1].f}'\n`);
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  execFileSync('ffmpeg', ['-v', 'error', '-y', '-f', 'concat', '-safe', '0', '-i', path.join(frames, 'list.txt'),
    '-vf', 'fps=25,scale=1920:1080,format=yuv420p', '-c:v', 'libx264', '-preset', 'slow', '-crf', '24', '-movflags', '+faststart', OUT], { stdio: 'inherit' });
  fs.rmSync(frames, { recursive: true, force: true });
  console.log(`\nFilm: ${OUT} (${shots.length} klatek źródłowych)`);
})();
