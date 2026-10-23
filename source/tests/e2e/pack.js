// E2E paczki belek .cgpack: eksport z Artist (sceny + strony + grafika + czcionka), import na czystym serwerze,
// czcionka z paczki ładowana na /output z serwera:  node tests/e2e/pack.js [ścieżka-do-vizrs]
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
let failures = 0;
const ok = (cond, msg, info) => { console.log((cond ? 'PASS ' : 'FAIL ') + msg + (!cond && info !== undefined ? ' → ' + JSON.stringify(info) : '')); if (!cond) failures++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function server() {
  const port = 20000 + Math.floor(Math.random() * 900);
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-pack-'));
  fs.writeFileSync(path.join(data, 'show.json'), JSON.stringify({ scenes: [], pages: [], playlists: [] }));
  const proc = spawn(BIN, ['--port', String(port), '--tcp', '0', '--data', data], { stdio: 'ignore' });
  const B = `http://127.0.0.1:${port}`;
  for (let i = 0; i < 50; i++) { try { await fetch(B + '/api/state'); break; } catch (e) { await sleep(100); } }
  return { B, proc, data, api: (p, o = {}) => fetch(B + p, o).then((r) => r.json()) };
}

(async () => {
  const { chromium } = loadPlaywright();
  const A = await server(), Z = await server();
  let browser;
  try {
    // czcionka testowa: wbudowany woff2 (Oswald) wgrany do assets pod własną nazwą; grafika: mały PNG
    const font = new Uint8Array(await (await fetch(A.B + '/fonts/oswald-latin-700-normal.woff2')).arrayBuffer());
    const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==', 'base64');
    await fetch(A.B + '/api/assets?name=MojaCzcionka.woff2', { method: 'POST', body: font });
    await fetch(A.B + '/api/assets?name=logo.png', { method: 'POST', body: png });
    await fetch(A.B + '/api/assets?name=dodatek.png', { method: 'POST', body: png });
    await fetch(A.B + '/api/assets?name=nieuzywany.png', { method: 'POST', body: png });
    const scene = {
      id: 'pk', name: 'Belka paczka', layer: 'main', width: 1920, height: 1080, duration: 1, stops: [], fieldMeta: {}, packAssets: ['dodatek.png'],
      nodes: [
        { id: 'i1', type: 'image', name: 'logo', props: { x: 10, y: 10, w: 100, h: 100, src: 'logo.png' } },
        { id: 't1', type: 'text', name: 'napis', fields: { text: 'tytul' }, props: { x: 50, y: 400, w: 800, h: 80, text: 'Tytuł', size: 40, color: '#ffffff', font: 'MojaCzcionka.woff2' } },
      ],
    };
    const s = await A.api('/api/scenes', { method: 'POST', body: JSON.stringify(scene) });
    await A.api('/api/pages', { method: 'POST', body: JSON.stringify({ scene: s.id, number: 4321, name: 'strona paczki', values: { tytul: 'Ala ma kota' } }) });

    browser = await chromium.launch();
    const ctx = await browser.newContext({ viewport: { width: 1600, height: 950 }, acceptDownloads: true });
    const pg = await ctx.newPage();
    const errs = [];
    pg.on('pageerror', (e) => errs.push(e.message));
    pg.on('dialog', (d) => d.accept());
    await pg.goto(A.B + '/artist?scene=pk');
    await sleep(1000);
    const chips = await pg.locator('#props .chip').allTextContents();
    ok(chips.some((c) => c.includes('logo.png')) && chips.some((c) => c.includes('MojaCzcionka.woff2')) && chips.some((c) => c.includes('dodatek.png')), 'Zasoby paczki: używane i dołączone pliki widoczne w ustawieniach sceny', chips);

    await pg.click('#bPack'); await sleep(300);
    ok((await pg.textContent('#pkSum')).includes('1 czcionek'), 'Okno paczki liczy czcionki', await pg.textContent('#pkSum'));
    const [dl] = await Promise.all([pg.waitForEvent('download'), pg.click('#pkGo')]);
    const file = path.join(A.data, 'test.cgpack');
    await dl.saveAs(file);
    ok(/\.cgpack$/.test(dl.suggestedFilename()), 'Pobrany plik ma rozszerzenie .cgpack', dl.suggestedFilename());
    const zipTxt = fs.readFileSync(file).toString('latin1');
    ok(['manifest.json', 'scenes.json', 'assets/logo.png', 'assets/MojaCzcionka.woff2', 'assets/dodatek.png'].every((n) => zipTxt.includes(n)) && !zipTxt.includes('nieuzywany.png'), 'Paczka zawiera sceny, grafiki i czcionkę (bez nieużywanych plików)');

    // import na czystym serwerze
    await pg.goto(Z.B + '/artist'); await sleep(800);
    await pg.click('#bImp');
    await pg.setInputFiles('#fileIn', file);
    await sleep(1500);
    const show = await Z.api('/api/show');
    const sc = show.scenes.find((x) => x.name === 'Belka paczka');
    ok(!!sc && sc.packAssets?.[0] === 'dodatek.png', 'Import: scena odtworzona', show.scenes.map((x) => x.name));
    const page = show.pages.find((p) => p.number === 4321);
    ok(!!page && page.scene === sc?.id && page.values.tytul === 'Ala ma kota', 'Import: strona z wartościami wskazuje nową scenę', page);
    const names = (await Z.api('/api/assets')).map((a) => a.name);
    ok(['logo.png', 'MojaCzcionka.woff2', 'dodatek.png'].every((n) => names.includes(n)), 'Import: pliki w assets', names);

    // ponowny import: te same pliki pomijane, scena jako kopia, numer strony zajęty → nowy
    await pg.click('#repOk').catch(() => {});
    await fetch(Z.B + '/api/assets?name=logo.png', { method: 'POST', body: Buffer.from('<svg xmlns="http://www.w3.org/2000/svg"/>') });
    await pg.click('#bImp');
    await pg.setInputFiles('#fileIn', file);
    await sleep(1500);
    const show2 = await Z.api('/api/show');
    const copies = show2.scenes.filter((x) => x.name === 'Belka paczka');
    ok(copies.length === 2 && show2.pages.length === 2, 'Ponowny import: druga scena i strona z nowym numerem', [copies.length, show2.pages.map((p) => p.number)]);
    const names2 = (await Z.api('/api/assets')).map((a) => a.name);
    ok(names2.includes('logo_2.png') && copies[1].nodes[0].props.src === 'logo_2.png', 'Konflikt nazwy pliku: nowa nazwa i przepisany odnośnik', [names2, copies[1]?.nodes[0].props.src]);

    // /output: czcionka z paczki ładowana z serwera
    const out = await ctx.newPage();
    await out.goto(Z.B + '/output'); await sleep(1500);
    const loaded = await out.evaluate(() => [...document.fonts].some((f) => f.family.replace(/"/g, '') === 'MojaCzcionka.woff2' && f.status === 'loaded'));
    ok(loaded, '/output ładuje czcionkę z assets z serwera (bez TAKE)');
    ok(errs.length === 0, 'Bez błędów JS', errs);
  } catch (e) {
    console.error(e); failures++;
  } finally {
    if (browser) await browser.close();
    A.proc.kill(); Z.proc.kill();
    fs.rmSync(A.data, { recursive: true, force: true }); fs.rmSync(Z.data, { recursive: true, force: true });
    console.log(failures ? `\n${failures} FAIL` : '\nOK');
    process.exit(failures ? 1 : 0);
  }
})();
