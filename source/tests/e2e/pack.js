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

/** Port wolny według systemu (losowy port bywał zajęty przez inny serwer testu – test rozmawiał wtedy z cudzymi danymi). */
const freePort = () => new Promise((res, rej) => { const s = require('net').createServer(); s.once('error', rej); s.listen(0, '127.0.0.1', () => { const { port } = s.address(); s.close(() => res(port)); }); });

async function server() {
  const port = await freePort();
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-pack-'));
  fs.writeFileSync(path.join(data, 'show.json'), JSON.stringify({ scenes: [], pages: [], playlists: [] }));
  const proc = spawn(BIN, ['--port', String(port), '--tcp', '0', '--data', data], { stdio: 'ignore' });
  const B = `http://127.0.0.1:${port}`;
  for (let i = 0; i < 50; i++) { try { await fetch(B + '/api/state'); break; } catch (e) { await sleep(100); } }
  // odpowiada nasz proces, nie inny serwer na tym porcie
  if (proc.exitCode !== null) throw new Error(`serwer testu nie wystartował na porcie ${port}`);
  return { B, proc, data, api: (p, o = {}) => fetch(B + p, o).then((r) => r.json()) };
}
/** Zatrzymanie serwera i usunięcie danych: po SIGTERM serwer jeszcze dopisuje log i zwalnia blokadę katalogu. */
async function stop(s) {
  const exited = s.proc.exitCode !== null ? Promise.resolve() : new Promise((r) => s.proc.once('exit', r));
  s.proc.kill();
  await Promise.race([exited, sleep(5000)]);
  fs.rmSync(s.data, { recursive: true, force: true, maxRetries: 5 });
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

    // import na serwerze, który ma już tę czcionkę pod inną nazwą – bez kopii, scena wskazuje istniejący plik
    await fetch(Z.B + '/api/assets?name=Juz_wgrana.woff2', { method: 'POST', body: font });
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
    ok(['logo.png', 'dodatek.png'].every((n) => names.includes(n)) && !names.includes('MojaCzcionka.woff2'), 'Import: pliki w assets, czcionka bez kopii', names);
    ok(sc?.nodes[1].props.font === 'Juz_wgrana.woff2', 'Import: font sceny wskazuje czcionkę już wgraną', sc?.nodes[1].props.font);
    ok((await pg.textContent('#rep')).includes('użyto istniejącej'), 'Raport importu mówi o użytej istniejącej czcionce', await pg.textContent('#rep'));

    // ponowny import: te same pliki pomijane, scena jako kopia, numer strony zajęty → nowy
    await pg.click('#repOk').catch(() => {});
    await fetch(Z.B + '/api/assets?name=logo.png', { method: 'POST', body: Buffer.from('<svg xmlns="http://www.w3.org/2000/svg"/>') });
    await pg.click('#bImp');
    await pg.setInputFiles('#fileIn', file);
    await sleep(1500);
    const show2 = await Z.api('/api/show');
    const copies = show2.scenes.filter((x) => x.name.startsWith('Belka paczka'));
    ok(copies.length === 2 && copies[1].name === 'Belka paczka (2)' && show2.pages.length === 1, 'Ponowny import bez okna planu (/artist): scena w innej wersji jako kopia „(2)”, strona o tym samym id pominięta', [copies.map((c) => c.name), show2.pages.map((p) => p.number)]);
    ok(sc?.id === 'pk', 'Import zachowuje id sceny z paczki', sc?.id);
    const names2 = (await Z.api('/api/assets')).map((a) => a.name);
    ok(names2.includes('logo_2.png') && copies[1].nodes[0].props.src === 'logo_2.png', 'Konflikt nazwy pliku: nowa nazwa i przepisany odnośnik', [names2, copies[1]?.nodes[0].props.src]);

    // ⇩ Eksport sceny = paczka .cgpack z plikami (nie .vizrs.json)
    await pg.goto(A.B + '/artist?scene=pk'); await sleep(800);
    const [dl2] = await Promise.all([pg.waitForEvent('download'), pg.click('#bExp')]);
    const file2 = path.join(A.data, 'eksport.cgpack');
    await dl2.saveAs(file2);
    const z2 = fs.readFileSync(file2).toString('latin1');
    ok(/\.cgpack$/.test(dl2.suggestedFilename()) && z2.includes('assets/MojaCzcionka.woff2') && z2.includes('scenes.json'), 'Eksport sceny zapisuje paczkę .cgpack z czcionką', dl2.suggestedFilename());

    // Trio: import paczki – okno planu; scena w innej wersji czeka na decyzję operatora, „Zaktualizuj” zachowuje strony
    const tr = await ctx.newPage();
    tr.on('pageerror', (e) => errs.push('trio: ' + e.message));
    await tr.goto(Z.B + '/trio'); await sleep(1000);
    await tr.click('#bImpPack');
    await tr.setInputFiles('#fileIn', file2);
    await tr.waitForSelector('.pk-back', { timeout: 5000 });
    ok(await tr.locator('.pk-back [data-x="go"]').isDisabled(), 'Trio: plan importu – „Importuj” zablokowany do decyzji dla sceny w innej wersji');
    ok((await tr.textContent('.pk-back')).includes('jest już w show w innej wersji') && (await tr.textContent('.pk-back')).includes('jest już w show'), 'Trio: plan pokazuje stan sceny i strony', await tr.textContent('.pk-back'));
    await tr.selectOption('.pk-back select[data-sc="0"]', 'update');
    await tr.click('.pk-back [data-x="go"]');
    await sleep(1500);
    await tr.keyboard.press('Escape').catch(() => {});
    const show3 = await Z.api('/api/show');
    const names3 = (await Z.api('/api/assets')).map((a) => a.name);
    const pk3 = show3.scenes.filter((x) => x.id === 'pk');
    ok(pk3.length === 1 && pk3[0].nodes[0].props.src === 'logo_2.png' && pk3[0].nodes[1].props.font === 'Juz_wgrana.woff2' && !names3.includes('MojaCzcionka.woff2'), 'Trio: „Zaktualizuj” podmienia scenę w miejscu (pliki wg reguł, czcionka bez kopii)', [pk3, names3]);
    ok(show3.pages.length === show2.pages.length && show3.pages.find((p) => p.number === 4321)?.scene === 'pk', 'Trio: aktualizacja sceny nie usuwa jej stron', show3.pages);

    // Trio: eksport wybranych stron do paczki
    await tr.click('#bExpPack');
    await tr.waitForSelector('.pk-back [data-q]');
    await tr.fill('.pk-back [data-q]', '4321');
    await tr.click('.pk-back [data-p="all"]');
    ok((await tr.textContent('.pk-back [data-sum]')).startsWith('1 stron · 1 scen'), 'Trio: okno eksportu liczy strony i sceny', await tr.textContent('.pk-back [data-sum]'));
    const [dl3] = await Promise.all([tr.waitForEvent('download'), tr.click('.pk-back [data-x="go"]')]);
    const file3 = path.join(A.data, 'strony.cgpack');
    await dl3.saveAs(file3);
    const P = await import(require('url').pathToFileURL(path.join(__dirname, '../../web/cgpack.js')).href);
    const pk = await P.readPack(new Uint8Array(fs.readFileSync(file3)));
    ok(/^4321_/.test(dl3.suggestedFilename()) && pk.pages.length === 1 && pk.pages[0].number === 4321 && pk.scenes.length === 1 && pk.assets.has('Juz_wgrana.woff2') && pk.assets.has('logo_2.png'), 'Trio: eksport strony – paczka ze stroną, sceną, grafiką i czcionką', [dl3.suggestedFilename(), pk.pages, [...pk.assets.keys()]]);
    await tr.close();

    // import stron z Trio na trzecim serwerze z przesunięciem numerów
    const C = await server();
    try {
      const tc = await ctx.newPage();
      tc.on('pageerror', (e) => errs.push('trio C: ' + e.message));
      await tc.goto(C.B + '/trio'); await sleep(1000);
      await tc.click('#bImpPack');
      await tc.setInputFiles('#fileIn', file3);
      await tc.waitForSelector('.pk-back [data-off]');
      await tc.fill('.pk-back [data-off]', '1000');
      await tc.locator('.pk-back [data-off]').dispatchEvent('change');
      ok((await tc.textContent('.pk-back')).includes('5321'), 'Trio: przesunięcie numerów widoczne w planie', await tc.textContent('.pk-back'));
      await tc.click('.pk-back [data-x="go"]');
      await sleep(1500);
      const sc4 = await C.api('/api/show');
      ok(sc4.scenes.length === 1 && sc4.scenes[0].id === 'pk' && sc4.pages[0]?.number === 5321 && sc4.pages[0]?.scene === 'pk', 'Trio: import stron – scena z id z paczki, strona z numerem przesuniętym o 1000', [sc4.scenes.map((x) => x.id), sc4.pages]);
      await tc.close();
    } finally { await stop(C); }

    // dawny eksport sceny .vizrs.json: tylko odczyt, przez ten sam import (scena zapisana, raport o formacie)
    const legacy = path.join(A.data, 'stara.vizrs.json');
    fs.writeFileSync(legacy, JSON.stringify({ name: 'Stara scena', layer: 'main', width: 1920, height: 1080, duration: 1, stops: [], nodes: [{ id: 'x', type: 'image', name: 'img', props: { x: 0, y: 0, w: 10, h: 10, src: 'nie_ma.png' } }] }));
    await pg.goto(Z.B + '/artist'); await sleep(800);
    await pg.click('#bImp');
    await pg.setInputFiles('#fileIn', legacy);
    await sleep(1200);
    const rep = await pg.textContent('#rep').catch(() => '');
    ok((await Z.api('/api/show')).scenes.some((x) => x.name === 'Stara scena') && rep.includes('starszy format') && rep.includes('nie_ma.png'), 'Import .vizrs.json: scena zapisana, raport o starszym formacie i braku pliku', rep);

    // /output: czcionka z paczki ładowana z serwera
    const out = await ctx.newPage();
    await out.goto(Z.B + '/output'); await sleep(1500);
    const loaded = await out.evaluate(() => [...document.fonts].some((f) => f.family.replace(/"/g, '') === 'Juz_wgrana.woff2' && f.status === 'loaded'));
    ok(loaded, '/output ładuje czcionkę z assets z serwera (bez TAKE)');
    ok(errs.length === 0, 'Bez błędów JS', errs);
  } catch (e) {
    console.error(e); failures++;
  } finally {
    if (browser) await browser.close();
    await stop(A); await stop(Z);
    console.log(failures ? `\n${failures} FAIL` : '\nOK');
    process.exit(failures ? 1 : 0);
  }
})();
