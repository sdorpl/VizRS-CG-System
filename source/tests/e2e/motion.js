// E2E ruchu na wzór BTB (node.motion): panel „Ruch” w Artist, dopasowanie stopu i długości, odtwarzanie na /output
// (wejście, efekt tekstu, pętla, wyjście):  node tests/e2e/motion.js [ścieżka-do-vizrs]
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

function loadPlaywright() {
  const dirs = [process.env.PLAYWRIGHT_DIR, path.join(os.homedir(), 'Nextcloud/GitHub/BTB-CG-System-alpha')].filter(Boolean);
  for (const d of dirs) { try { return require(require.resolve('playwright', { paths: [d] })); } catch (e) { /* dalej */ } }
  return require('playwright');
}
const BIN = path.resolve(process.argv[2] || path.join(__dirname, '../../target/release/vizrs'));
const PORT = 20000 + Math.floor(Math.random() * 900);
const B = `http://127.0.0.1:${PORT}`;
let failures = 0;
const ok = (cond, msg, info) => { console.log((cond ? 'PASS ' : 'FAIL ') + msg + (!cond && info !== undefined ? ' → ' + JSON.stringify(info) : '')); if (!cond) failures++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const api = (p, o = {}) => fetch(B + p, o).then((r) => r.json());

(async () => {
  const { chromium } = loadPlaywright();
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-motion-'));
  fs.writeFileSync(path.join(data, 'show.json'), JSON.stringify({ scenes: [], pages: [], playlists: [] }));
  const srv = spawn(BIN, ['--port', String(PORT), '--tcp', '0', '--data', data], { stdio: 'ignore' });
  let browser;
  try {
    for (let i = 0; i < 50; i++) { try { await fetch(B + '/api/state'); break; } catch (e) { await sleep(100); } }
    const scene = {
      id: 'mo', name: 'Ruch', layer: 'main', width: 1920, height: 1080, duration: 1, stops: [], fieldMeta: {},
      nodes: [
        { id: 'bg', type: 'rect', name: 'tlo', props: { x: 100, y: 800, w: 900, h: 100, color: '#123' } },
        { id: 'tx', type: 'text', name: 'tytul', fields: { text: 'tytul' }, props: { x: 120, y: 800, w: 860, h: 100, text: 'Ala ma kota', size: 50, color: '#fff' } },
      ],
    };
    await api('/api/scenes', { method: 'POST', body: JSON.stringify(scene) });
    browser = await chromium.launch();
    const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
    const pg = await ctx.newPage();
    const errs = [];
    pg.on('pageerror', (e) => errs.push(e.message));
    pg.on('dialog', (d) => d.accept());
    await pg.goto(B + '/artist?scene=mo');
    await sleep(1000);

    // tło: wjazd z lewej, wyjście – przenikanie, pętla – połysk
    await pg.click('#tree .item[data-id="bg"]');
    await pg.selectOption('select[data-mo="in.type"]', 'slide'); await sleep(150);
    ok(await pg.locator('select[data-mo="in.dir"]').count() === 1, 'Preset z kierunkiem pokazuje wybór kierunku');
    await pg.selectOption('select[data-mo="in.dir"]', 'left');
    await pg.selectOption('select[data-mo="out.type"]', 'fade'); await sleep(100);
    await pg.selectOption('select[data-mo="loop.type"]', 'shine'); await sleep(100);
    const tf0 = await pg.evaluate(() => document.querySelector('#stage .vz-node[data-id="bg"]').style.transform);
    ok(/translate\(-1920px/.test(tf0), 'Na początku sceny tło jest poza kadrem z lewej', tf0);
    // tytuł: po tle, efekt tekstu – słowa w górę
    await pg.click('#tree .item[data-id="tx"]');
    await pg.selectOption('select[data-mo="textIn.type"]', 'words-up'); await sleep(100);
    ok(await pg.locator('[data-mofit]').count() === 1, 'Podpowiedź „Dopasuj stop i długość” przy braku stopu');
    await pg.click('[data-mofit]'); await sleep(200);
    await pg.keyboard.press('Control+s'); await sleep(600);
    const saved = (await api('/api/show')).scenes.find((s) => s.id === 'mo');
    const bg = saved.nodes[0], tx = saved.nodes[1];
    ok(bg.motion?.in?.type === 'slide' && bg.motion.in.dir === 'left' && bg.motion.out?.type === 'fade' && bg.motion.loop?.type === 'shine', 'Zapisany ruch tła', bg.motion);
    ok(tx.motion?.textIn?.type === 'words-up' && tx.motion.textIn.dur === 0.6, 'Zapisany efekt tekstu', tx.motion);
    ok(saved.stops.length === 1 && Math.abs(saved.stops[0] - 0.78) < 0.011 && Math.abs(saved.duration - (saved.stops[0] + 0.5)) < 0.011, 'Stop po wejściu i długość = stop + wyjście', [saved.stops, saved.duration]);

    // wyjście OBS
    const page = await api('/api/pages', { method: 'POST', body: JSON.stringify({ scene: 'mo', number: 1500, values: { tytul: 'Ala ma kota' } }) });
    const out = await ctx.newPage();
    out.on('pageerror', (e) => errs.push('output: ' + e.message));
    await out.goto(B + '/output'); await sleep(800);
    await api('/api/take/' + page.number);
    await sleep(250);
    const mid = await out.evaluate(() => ({ units: document.querySelectorAll('.vz-node[data-id="tx"] .vz-u').length, tf: document.querySelector('.vz-node[data-id="bg"]').style.transform }));
    ok(mid.units === 3, 'Efekt tekstu dzieli tekst na słowa w trakcie wejścia', mid);
    await sleep(1600);
    const hold = await out.evaluate(() => ({ units: document.querySelectorAll('.vz-node[data-id="tx"] .vz-u').length, tf: document.querySelector('.vz-node[data-id="bg"]').style.transform, shine: !!document.querySelector('.vz-node[data-id="bg"] .vz-loop') }));
    ok(hold.units === 0 && !/translate/.test(hold.tf), 'Po wejściu: tło na miejscu, tekst w zwykłym układzie', hold);
    ok(hold.shine, 'Pętla „połysk” gra, gdy grafika stoi');
    await api('/api/out/main');
    await sleep(200);
    const op = await out.evaluate(() => { const e = document.querySelector('.vz-node[data-id="bg"]'); return e ? +e.style.opacity : -1; });
    ok(op >= 0 && op < 0.95, 'OUT: tło przenika (krycie < 1)', op);
    await sleep(700);
    ok(await out.locator('.vz-node').count() === 0, 'Po wyjściu scena zdjęta');
    ok(errs.length === 0, 'Bez błędów JS', errs);
  } catch (e) {
    console.error(e); failures++;
  } finally {
    if (browser) await browser.close();
    srv.kill();
    fs.rmSync(data, { recursive: true, force: true });
    console.log(failures ? `\n${failures} FAIL` : '\nOK');
    process.exit(failures ? 1 : 0);
  }
})();
