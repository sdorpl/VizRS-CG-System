// E2E ruchu (node.motion): panel „Ruch” w Artist, dopasowanie stopu i długości, odtwarzanie na /output
// (wejście, efekt tekstu, pętla, wyjście):  node tests/e2e/motion.js [ścieżka-do-vizrs]
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

function loadPlaywright() {
  const dirs = [process.env.PLAYWRIGHT_DIR].filter(Boolean);
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

    // tryb sekwencji: grupa z trzema prostokątami
    const sq = { id: 'sq', name: 'Sekwencja', layer: 'front', width: 1920, height: 1080, duration: 2, stops: [1], fieldMeta: {}, nodes: [{ id: 'g', type: 'group', name: 'grupa', props: { x: 0, y: 0, w: 600, h: 300 }, children: [
      { id: 'r1', type: 'rect', name: 'r1', props: { x: 0, y: 0, w: 100, h: 50 } }, { id: 'r2', type: 'rect', name: 'r2', props: { x: 0, y: 60, w: 100, h: 50 } }, { id: 'r3', type: 'rect', name: 'r3', props: { x: 0, y: 120, w: 100, h: 50 } }] }] };
    await api('/api/scenes', { method: 'POST', body: JSON.stringify(sq) });
    await pg.goto(B + '/artist?scene=sq'); await sleep(900);
    if (!(await pg.locator('#seqMode.on').count())) await pg.click('#seqMode');
    await sleep(200);
    ok(await pg.locator('#tl .tl-row.seq').count() === 4, 'Sekwencja: wiersz na każdy element (grupa + 3)');
    await pg.click('#tl .tl-lbl[data-sel="g"]'); await sleep(150);
    await pg.fill('#seqStep', '0.15');
    await pg.click('[data-seq="stagger"]'); await sleep(200);
    ok(await pg.locator('#tl .mb.in').count() === 3, 'Kaskada bez ruchu nadaje przenikanie wszystkim dzieciom');
    const xs = await pg.$$eval('#tl .mb.in', (els) => els.map((e) => parseFloat(e.style.left)));
    const pps = await pg.evaluate(() => +document.querySelector('#zoom').value);
    ok(Math.abs(xs[1] - xs[0] - 0.15 * pps) < 1 && Math.abs(xs[2] - xs[0] - 0.3 * pps) < 1, 'Kaskada: wejścia co 0,15 s', xs);
    // przeciągnięcie paska r1 o 0,5 s w prawo
    const b1 = await pg.locator('#tl .mb.in[data-mb="r1|in"]').boundingBox();
    await pg.mouse.move(b1.x + 10, b1.y + 5); await pg.mouse.down(); await pg.mouse.move(b1.x + 10 + 0.5 * pps, b1.y + 5, { steps: 5 }); await pg.mouse.up(); await sleep(200);
    // krawędź paska r2: czas +0,2 s
    const h2 = await pg.locator('#tl .mbh[data-mbh="r2|in"]').boundingBox();
    await pg.mouse.move(h2.x + 2, h2.y + 4); await pg.mouse.down(); await pg.mouse.move(h2.x + 2 + 0.2 * pps, h2.y + 4, { steps: 5 }); await pg.mouse.up(); await sleep(200);
    await pg.keyboard.press('Control+s'); await sleep(500);
    let s2 = (await api('/api/show')).scenes.find((x) => x.id === 'sq');
    let [c1, c2, c3] = s2.nodes[0].children;
    ok(Math.abs(c1.motion.in.delay - 0.5) < 0.011 && Math.abs(c2.motion.in.dur - 0.6) < 0.011 && Math.abs(c3.motion.in.delay - 0.3) < 0.011, 'Przeciąganie paska: opóźnienie i czas', [c1.motion.in, c2.motion.in, c3.motion.in]);
    await pg.click('[data-seq="chain"]'); await sleep(200);
    ok(await pg.locator('#tl .tl-lbl:has-text("↳")').count() === 2, 'Po kolei: dwa powiązania „po węźle” w etykietach');
    // na klucze: r3
    await pg.click('#tl .tl-lbl[data-sel="r3"]'); await sleep(150);
    await pg.click('[data-seq="keys"]'); await sleep(300);
    ok(!(await pg.locator('#seqMode.on').count()) && await pg.locator('#tl .kf').count() >= 2, 'Na klucze: widok kluczy z kluczami krycia');
    await pg.keyboard.press('Control+s'); await sleep(500);
    s2 = (await api('/api/show')).scenes.find((x) => x.id === 'sq');
    [c1, c2, c3] = s2.nodes[0].children;
    ok(c2.motion.after === 'r1' && c3.anim?.opacity?.length === 2 && !c3.motion?.in && c3.anim.opacity[0].t > 1, 'Zapis: po kolei + klucze z przesunięciem sekwencji', [c2.motion, c3.anim, c3.motion]);
    // „po węźle” w panelu Ruch
    await pg.click('#tree .item[data-id="r2"]'); await sleep(150);
    await pg.selectOption('select[data-moafter]', ''); await sleep(150);
    await pg.keyboard.press('Control+s'); await sleep(500);
    s2 = (await api('/api/show')).scenes.find((x) => x.id === 'sq');
    ok(!s2.nodes[0].children[1].motion.after, 'Panel Ruch: „po węźle” – od początku sceny usuwa powiązanie');
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
