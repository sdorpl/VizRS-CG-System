// Film porównawczy: renderer Chromium (`/output`) i renderer natywny (`vizrs-render stream`) klatka w klatkę.
//   node tools/native-video.js [ścieżka-do-vizrs] [--render render/target/release/vizrs-render] [--out plik.mp4]
//        [--only INFO24,R7] [--fps 25]
// Jeden serwer VizRS na paczkę, te same komendy (TAKE → na antenie → CONTINUE → OUT) dla obu rendererów.
// Chromium nagrywany w czasie wirtualnym (jak tools/video.js), renderer natywny dostaje te same czasy klatek
// (`F <czas> <ścienny>`), więc animacje, tickery i zegary są w tych samych chwilach; requestAnimationFrame strony
// biegnie raz na klatkę, jak `beginFrame` na kanale. Kadr 2×2: Chromium | natywnie
// / różnica ×4 | opis strony (czy wszystko natywnie, czas klatki renderera natywnego). Wymaga ffmpeg.
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { ROOT, SETS, sleep, loadPlaywright, startSet } = require('./sets');

const args = process.argv.slice(2);
const opt = (n) => { const i = args.indexOf(n); return i >= 0 ? args[i + 1] : null; };
const OUT = path.resolve(opt('--out') || path.join(os.tmpdir(), 'vizrs-chromium-vs-natywny.mp4'));
const ONLY = (opt('--only') || 'INFO24').split(',');
const FPS = Math.max(1, +opt('--fps') || 25);
// --stats plik.csv: błąd każdej klatki (MSE Chromium–natywnie, filtr psnr ffmpeg) i podsumowanie stron
const STATS = opt('--stats');
const RENDER = path.resolve(opt('--render') || path.join(__dirname, '../render/target/release/vizrs-render'));
const BIN = path.resolve(args.find((a, i) => !a.startsWith('--') && !['--out', '--only', '--fps', '--render', '--stats'].includes(args[i - 1])) || path.join(__dirname, '../target/release/vizrs'));
const ASSETS = path.join(ROOT, 'examples/archiwum/assets');
const [W, H] = [1920, 1080];
const BGC = [0x2b, 0x34, 0x42]; // jednolite tło obu stron (różnica nie zależy od tła)
const BGCSS = '#2b3442';
const TITLES = { INFO24: 'INFO24 – oprawa demo', R7: 'R7 NEWS', 'BTB-PILNE': 'Belka BTB „PILNE”' };

const hasType = (nodes, types) => (nodes || []).some((n) => types.includes(n.type) || hasType(n.children, types));
const holdFor = (sc) => (hasType(sc.nodes, ['ticker']) ? 9 : JSON.stringify(sc.nodes).includes('"cycle":') ? 7 : 3.5);

/** `vizrs-render stream`: klatki na żądanie, rekordy VZR1 (DOKUMENTACJA §10.1). */
function nativeRenderer(url) {
  const p = spawn(RENDER, ['stream', '--url', url, '--size', `${W}x${H}`], { stdio: ['pipe', 'pipe', 'pipe'] });
  const issues = [];
  p.stderr.on('data', (d) => { for (const l of String(d).split('\n')) if (l.trim()) issues.push(l.trim()); });
  let buf = Buffer.alloc(0);
  const waiting = [];
  const pump = () => {
    while (waiting.length && buf.length >= 16) {
      const kind = buf[4], flags = buf[5], w = buf.readUInt32LE(8), h = buf.readUInt32LE(12);
      let n = 16, region = null;
      if (kind === 1) n += w * h * 4;
      if (kind === 4) {
        if (buf.length < 32) return;
        region = { x: buf.readUInt32LE(16), y: buf.readUInt32LE(20), w: buf.readUInt32LE(24), h: buf.readUInt32LE(28) };
        n = 32 + region.w * region.h * 4;
      }
      if (buf.length < n) return;
      const rec = { kind, flags, w, h, region, px: buf.subarray(kind === 4 ? 32 : 16, n) };
      buf = buf.subarray(n);
      waiting.shift()(rec);
    }
  };
  p.stdout.on('data', (d) => { buf = Buffer.concat([buf, d]); pump(); });
  return {
    issues,
    frame: (now, wall) => new Promise((res) => { waiting.push(res); p.stdin.write(`F ${now.toFixed(3)} ${wall.toFixed(3)}\n`); pump(); }),
    stop: () => { try { p.stdin.write('Q\n'); } catch (_) { /* zamknięty */ } p.kill(); },
  };
}

/** Pełny obraz RGBA (bez przemnożenia alfy) → RGB na jednolitym tle. */
function flatten(rgba, out) {
  for (let i = 0, j = 0; i < rgba.length; i += 4, j += 3) {
    const a = rgba[i + 3];
    if (a === 0) { out[j] = BGC[0]; out[j + 1] = BGC[1]; out[j + 2] = BGC[2]; continue; }
    out[j] = (rgba[i] * a + BGC[0] * (255 - a) + 127) / 255;
    out[j + 1] = (rgba[i + 1] * a + BGC[1] * (255 - a) + 127) / 255;
    out[j + 2] = (rgba[i + 2] * a + BGC[2] * (255 - a) + 127) / 255;
  }
}

// pliki pośrednie bezstratnie w RGB: JPEG Chromium (pełny zakres) i RGB renderera natywnego w tej samej przestrzeni barw
const ffEncode = (file, input) => {
  const ff = spawn('ffmpeg', ['-v', 'error', '-y', ...input, '-c:v', 'libx264rgb', '-preset', 'ultrafast', '-qp', '0', '-pix_fmt', 'rgb24', '-r', String(FPS), file], { stdio: ['pipe', 'inherit', 'inherit'] });
  const done = new Promise((res, rej) => ff.on('close', (c) => (c === 0 ? res() : rej(new Error('ffmpeg: kod ' + c)))));
  const write = async (b) => { if (!ff.stdin.write(b)) await new Promise((r) => ff.stdin.once('drain', r)); };
  return { write, end: () => { ff.stdin.end(); return done; } };
};

/** Tekst dla drawtext (cudzysłowy, dwukropki, procenty). */
const esc = (s) => String(s).replace(/\\/g, '\\\\\\\\').replace(/'/g, '\u2019').replace(/:/g, '\\:').replace(/%/g, '\\%').replace(/,/g, '\\,');

(async () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-nvideo-'));
  const fileA = path.join(tmp, 'chromium.mkv'), fileB = path.join(tmp, 'natywny.mkv');
  // przy --stats zrzuty PNG: bez artefaktów JPEG w błędzie klatek (wolniej)
  const encA = ffEncode(fileA, ['-f', 'image2pipe', '-framerate', String(FPS), '-c:v', STATS ? 'png' : 'mjpeg', '-i', '-']);
  const encB = ffEncode(fileB, ['-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', `${W}x${H}`, '-framerate', String(FPS), '-i', '-']);
  const segments = []; // { from, to (s), lines: [..] } – opis w czwartej ćwiartce
  const { chromium } = loadPlaywright();
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: W, height: H } });
  await page.clock.install();
  const base = Date.now();
  await page.clock.pauseAt(base);
  let vt = 0; // czas wirtualny (ms) wspólny dla obu rendererów
  let frames = 0;
  const rgba = Buffer.alloc(W * H * 4);
  const rgb = Buffer.alloc(W * H * 3);
  try {
    for (const [setName, file] of SETS) {
      if (!ONLY.includes(setName)) continue;
      const { B, show, stop } = await startSet(BIN, setName, file);
      // obrazy z paczek przykładowych (np. tło hashtagu) – do assets serwera, dla obu rendererów
      if (fs.existsSync(ASSETS)) {
        for (const n of fs.readdirSync(ASSETS)) await fetch(`${B}/api/assets?name=${encodeURIComponent(n)}`, { method: 'POST', body: fs.readFileSync(path.join(ASSETS, n)) });
      }
      const nat = nativeRenderer(`${B}/output`);
      rgba.fill(0);
      try {
        await page.goto(B + '/output');
        await page.evaluate((bg) => { document.documentElement.style.background = document.body.style.background = bg; }, BGCSS);
        // jedna klatka animacji na klatkę nagrania, w jej chwili – jak `beginFrame` na kanale (VizRS CG Playout);
        // zegar wirtualny Playwright odpalałby requestAnimationFrame co 16 ms, niezależnie od klatek
        await page.evaluate(() => {
          const queue = [];
          window.requestAnimationFrame = (cb) => queue.push(cb);
          window.__vzFrame = () => { const t = performance.now(); for (const cb of queue.splice(0)) cb(t); };
        });
        await sleep(1200); // SSE, fonty (czas rzeczywisty), renderer natywny wczytuje fonty serwera
        let stat = { ms: 0, n: 0, complete: true };
        const record = async (sec) => {
          for (let i = 0, n = Math.round(sec * FPS); i < n; i++) {
            vt += 1000 / FPS;
            await page.clock.runFor(1000 / FPS);
            await page.evaluate(() => window.__vzFrame());
            await encA.write(await page.screenshot(STATS ? { type: 'png' } : { type: 'jpeg', quality: 95 }));
            const t0 = process.hrtime.bigint();
            const r = await nat.frame(1000 + vt, base + vt);
            stat.ms += Number(process.hrtime.bigint() - t0) / 1e6;
            stat.n++;
            if (!(r.flags & 1)) stat.complete = false;
            if (r.kind === 1) r.px.copy(rgba);
            else if (r.kind === 2) rgba.fill(0);
            else if (r.kind === 4) {
              rgba.fill(0);
              const { x, y, w } = r.region;
              for (let row = 0; row < r.region.h; row++) r.px.copy(rgba, ((y + row) * W + x) * 4, row * w * 4, (row + 1) * w * 4);
            }
            flatten(rgba, rgb);
            await encB.write(rgb);
            frames++;
          }
        };
        const command = async (url) => { await fetch(url, { method: 'POST' }); await sleep(250); };
        const pages = [...show.pages].sort((a, b) => a.number - b.number);
        const scenes = Object.fromEntries(show.scenes.map((s) => [s.id, s]));
        for (const p of pages) {
          const sc = scenes[p.scene] || {};
          const from = frames / FPS;
          stat = { ms: 0, n: 0, complete: true };
          nat.issues.length = 0;
          await command(`${B}/api/take/${p.number}`);
          await record(holdFor(sc));
          for (let s = 1; s < (sc.stops || []).length; s++) { await command(`${B}/api/continue/${p.number}`); await record(2.5); }
          await command(`${B}/api/out/${p.number}`);
          await record(1.6);
          await command(`${B}/api/clear`);
          await record(0.2);
          const gaps = nat.issues.filter((l) => l.includes('poza rendererem')).map((l) => l.replace(/^.*natywnym: /, ''));
          segments.push({
            from, to: frames / FPS,
            lines: [`${TITLES[setName] || setName}`, `strona ${p.number} – ${p.name}`, `scena ${sc.name || p.scene}`,
              stat.complete ? 'natywnie: cała strona' : 'natywnie: NIEPEŁNA (auto → Chromium)',
              ...(gaps.length ? [gaps[0].replace(/^[^:]*: /, 'brak: ').slice(0, 60)] : []),
              `natywny: ${(stat.ms / Math.max(1, stat.n)).toFixed(1)} ms/klatkę`],
          });
          console.log(`${setName} ${p.number} ${p.name}${stat.complete ? '' : ' (niepełna natywnie)'}`);
        }
      } finally { nat.stop(); stop(); }
    }
  } finally {
    await browser.close();
    await encA.end();
    await encB.end();
  }
  if (STATS) {
    const log = path.join(tmp, 'psnr.log');
    await new Promise((res, rej) => {
      const f = spawn('ffmpeg', ['-v', 'error', '-i', fileA, '-i', fileB, '-lavfi', `[0:v][1:v]psnr=stats_file=${log}`, '-f', 'null', '-'], { stdio: 'inherit' });
      f.on('close', (c) => (c === 0 ? res() : rej(new Error('ffmpeg psnr: kod ' + c))));
    });
    const mse = fs.readFileSync(log, 'utf8').trim().split('\n').map((l) => +(/mse_avg:([\d.]+)/.exec(l) || [0, 0])[1]);
    const rows = ['klatka,czas_s,strona,mse'];
    console.log('\nstrona                                  MSE: średnio  najwięcej (s od TAKE)');
    for (const sg of segments) {
      const a = Math.round(sg.from * FPS), b = Math.round(sg.to * FPS);
      let max = 0, at = a, sum = 0;
      for (let k = a; k < b && k < mse.length; k++) {
        rows.push(`${k},${(k / FPS).toFixed(2)},${sg.lines[1].replace(/,/g, ' ')},${mse[k].toFixed(3)}`);
        sum += mse[k];
        if (mse[k] > max) { max = mse[k]; at = k; }
      }
      console.log(`${sg.lines[1].slice(0, 38).padEnd(40)} ${(sum / Math.max(1, b - a)).toFixed(2).padStart(8)} ${max.toFixed(2).padStart(9)} (${((at - a) / FPS).toFixed(2)})`);
    }
    fs.writeFileSync(path.resolve(STATS), rows.join('\n') + '\n');
  }
  // kadr 2×2: Chromium | natywnie / różnica ×4 | opis
  const font = ['/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', '/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf'].find((f) => fs.existsSync(f));
  const ff = font ? `fontfile='${font}':` : '';
  const head = (label) => `drawbox=x=0:y=0:w=iw:h=44:color=black@0.6:t=fill,drawtext=${ff}text='${esc(label)}':x=14:y=10:fontsize=26:fontcolor=white`;
  let info = `color=c=0x15191f:s=960x540:r=${FPS}:d=${(frames / FPS).toFixed(3)},drawtext=${ff}text='${esc('Chromium vs renderer natywny')}':x=30:y=30:fontsize=34:fontcolor=0xf5a800`;
  for (const s of segments) {
    s.lines.forEach((l, i) => {
      info += `,drawtext=${ff}text='${esc(l)}':x=30:y=${100 + i * 52}:fontsize=${i === 0 ? 30 : 26}:fontcolor=${i === 3 && l.includes('NIEPEŁNA') ? '0xff6b6b' : 'white'}:enable='between(t,${s.from.toFixed(3)},${(s.to - 0.001).toFixed(3)})'`;
    });
  }
  info += `,drawtext=${ff}text='${esc('różnica: |Chromium − natywny| × 4')}':x=30:y=480:fontsize=22:fontcolor=0x8a929c`;
  const graph = [
    `[0:v]scale=960:540,split[a1][a2]`,
    `[1:v]scale=960:540,split[b1][b2]`,
    `[a2]format=gbrp[ag]`, `[b2]format=gbrp[bg]`,
    `[ag][bg]blend=all_mode=difference,lutrgb=r='min(val*4,255)':g='min(val*4,255)':b='min(val*4,255)',${head('różnica ×4')}[d]`,
    `[a1]${head('Chromium (/output, obecny)')}[ca]`,
    `[b1]${head('renderer natywny (vizrs-render)')}[cb]`,
    `${info}[inf]`,
    `[ca][cb][d][inf]xstack=inputs=4:layout=0_0|w0_0|0_h0|w0_h0,format=yuv420p[v]`,
  ].join(';');
  const graphFile = path.join(tmp, 'graph.txt');
  fs.writeFileSync(graphFile, graph);
  await new Promise((res, rej) => {
    const f = spawn('ffmpeg', ['-v', 'error', '-y', '-i', fileA, '-i', fileB, '-filter_complex_script', graphFile, '-map', '[v]', '-c:v', 'libx264', '-preset', 'medium', '-crf', '20', '-r', String(FPS), '-movflags', '+faststart', OUT], { stdio: 'inherit' });
    f.on('close', (c) => (c === 0 ? res() : rej(new Error('ffmpeg (kadr 2×2): kod ' + c))));
  });
  fs.rmSync(tmp, { recursive: true, force: true });
  console.log(`\nFilm: ${OUT} (${frames} klatek, ${(frames / FPS).toFixed(1)} s, ${FPS} kl./s)`);
})();
