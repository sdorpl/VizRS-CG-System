// E2E Trio mobilnego (/mobile): telefon z ekranem dotykowym steruje anteną przez to samo API co Trio.
//   node tests/e2e/mobile.js [ścieżka-do-vizrs] [katalog-na-zrzuty]
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
const SHOTS = process.argv[3];
const PORT = 19000 + Math.floor(Math.random() * 1000);
const B = `http://127.0.0.1:${PORT}`;
const TOKEN = 'e2e-token-1', OUT_TOKEN = 'e2e-out-1';
let failures = 0;
const ok = (cond, msg, info) => { console.log((cond ? 'PASS ' : 'FAIL ') + msg + (!cond && info !== undefined ? ' → ' + JSON.stringify(info) : '')); if (!cond) failures++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const { chromium } = loadPlaywright();
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-e2e-m-'));
  const start = () => spawn(BIN, ['--port', String(PORT), '--tcp', '0', '--data', data, '--token', TOKEN, '--output-token', OUT_TOKEN], { stdio: 'ignore' });
  let srv = start(), browser;
  const api = (p, o = {}) => fetch(B + p, { ...o, headers: { 'X-Token': TOKEN, ...(o.headers || {}) } }).then((r) => r.json());
  const waitUp = async () => { for (let i = 0; i < 50; i++) { try { await api('/api/state'); return; } catch (e) { await sleep(100); } } throw new Error('serwer nie wystartował'); };
  try {
    await waitUp();
    ok((await fetch(B + '/mobile')).status === 401, '/mobile bez tokenu → 401');
    ok((await (await fetch(B + '/mobile')).text()).includes('name="token"'), '/mobile bez tokenu pokazuje ekran logowania');
    ok((await fetch(B + '/mobile.webmanifest')).status === 200 && (await fetch(B + '/mobile-icon.png')).headers.get('content-type') === 'image/png', 'manifest i ikona bez tokenu');
    ok((await fetch(B + '/trio')).status === 401 && !(await (await fetch(B + '/trio')).text()).includes('name="token"'), 'Trio bez zmian: bez ekranu logowania');
    ok((await fetch(`${B}/mobile?token=${OUT_TOKEN}`)).status === 403, 'token wyjścia: /mobile → 403');

    browser = await chromium.launch();
    const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 });
    const errs = [];
    const p = await ctx.newPage();
    p.on('pageerror', (e) => errs.push(e.message));
    p.on('dialog', (d) => d.accept());
    const shot = async (n) => { if (SHOTS) await p.screenshot({ path: path.join(SHOTS, n + '.png') }); };
    const layers = async () => (await api('/api/state')).layers;
    const pages = () => p.locator('#pages .item');
    // ekran logowania: zły token, potem dobry; token nie zostaje w adresie
    await p.goto(`${B}/mobile`);
    await p.fill('input[name=token]', 'zly');
    await p.tap('form button');
    await p.waitForFunction(() => !document.querySelector('#bad').hidden);
    ok(true, 'zły token: komunikat na ekranie logowania');
    await p.fill('input[name=token]', TOKEN);
    await p.tap('form button');
    await p.waitForSelector('#pages .item');
    ok(new URL(p.url()).search === '', 'token nie zostaje w adresie', p.url());
    await p.goto(`${B}/mobile`);
    await p.waitForSelector('#pages .item');
    ok(true, 'zalogowany telefon wchodzi bez ekranu logowania');
    await p.waitForFunction(() => document.querySelector('#conn').textContent === 'OK');

    const over = await p.evaluate(() => document.documentElement.scrollWidth - innerWidth);
    ok(over <= 0, 'strona mieści się na szerokość telefonu', over);
    ok(await p.locator('#bTake').isDisabled(), 'bez wybranej strony TAKE jest wyłączony');
    await shot('1-strony');

    // dotknięcie strony niczego nie wysyła na antenę
    const num = +(await pages().first().locator('.num').textContent());
    await pages().first().tap();
    await sleep(200);
    ok((await layers()).length === 0, 'dotknięcie strony tylko ją wybiera');
    ok((await p.textContent('#barName')).includes(String(num)), 'pasek pokazuje wybraną stronę');
    ok(await p.locator('#bOut').isDisabled(), 'OUT wyłączony, gdy strony nie ma na antenie');

    // TAKE: podwójne dotknięcie gra raz
    let takes = 0;
    p.on('request', (r) => { if (r.url().includes('/api/take/')) takes++; });
    await p.evaluate(() => { const b = document.querySelector('#bTake'); b.click(); b.click(); });
    await p.waitForFunction(() => document.querySelector('#barSt').textContent.includes('ON AIR'));
    ok(takes === 1, 'podwójne dotknięcie TAKE wysyła jedną komendę', takes);
    ok((await layers()).some((l) => l.page?.number === num), 'TAKE z telefonu jest na antenie');
    await shot('2-on-air');

    // edycja pola: TAKE/UPDATE bez zapisu, potem zapis
    await p.tap('[data-tab="edit"]');
    const field = p.locator('#fields input:not([type=checkbox]):not([type=number]), #fields textarea').first();
    const fid = await field.getAttribute('data-f');
    await field.fill('Z telefonu');
    await shot('3-edycja');
    await sleep(450);
    await p.tap('#bUpd');
    await sleep(300);
    const updated = await p.evaluate(() => performance.getEntriesByType('resource').some((r) => r.name.includes('/api/update/') && r.responseStatus === 200));
    ok(updated, 'UPDATE działa na stronie na antenie');
    const live = await fetch(`${B}/output?token=${TOKEN}`).then(() => api('/api/audit?n=5'));
    ok(JSON.stringify(live).includes('UPDATE') || JSON.stringify(live).includes('update'), 'UPDATE jest w logu audytowym', live);
    const before = (await api('/api/show')).pages.find((x) => x.number === num);
    ok(before.values?.[fid] !== 'Z telefonu', 'UPDATE nie zapisuje strony');
    ok((await api('/api/state')).locks?.some((l) => l.kind === 'pages' && l.id === before.id), 'zmiana pola blokuje stronę dla innych');
    await p.tap('#eSave');
    await p.waitForFunction(() => document.querySelector('#eSave').disabled);
    ok((await api('/api/show')).pages.find((x) => x.number === num).values[fid] === 'Z telefonu', 'Zapisz zapisuje stronę');
    await sleep(100);
    ok(!(await api('/api/state')).locks?.some((l) => l.kind === 'pages'), 'zapis zwalnia blokadę');

    // reguły pól jak w Trio
    const rules = await p.evaluate(async () => { const m = await import('/field-rules.js'); return [m.limitText('abc', { upper: true }).v, m.limitText('a\nb', { oneLine: true }).v, m.limitText('abcd', { maxChars: 3 }, 'abc').v, m.visLen('{b}ab{/b}\nc')]; });
    ok(JSON.stringify(rules) === JSON.stringify(['ABC', 'a b', 'abc', 3]), 'wspólne reguły pól', rules);

    // Antena: monitor tylko na tej zakładce, CONTINUE/OUT warstwy, CLEAR
    ok(await p.locator('#pgmBox iframe').count() === 0, 'monitor PGM nie działa poza zakładką Antena');
    await p.tap('[data-tab="air"]');
    await p.waitForSelector('#layers .item');
    ok(await p.locator('#pgmBox iframe').count() === 1, 'zakładka Antena otwiera monitor PGM');
    await sleep(900);
    await shot('4-antena');
    await p.tap('#layers [data-lout]');
    await p.waitForFunction(() => document.querySelector('#layers').textContent.includes('Antena czysta'), null, { timeout: 8000 });
    ok((await layers()).length === 0, 'OUT warstwy z telefonu');
    await api(`/api/take/${num}`, { method: 'POST' });
    await p.waitForSelector('#layers [data-lout]');
    await sleep(450);
    await p.tap('#bClear');
    await p.waitForFunction(() => document.querySelector('#layers').textContent.includes('Antena czysta'), null, { timeout: 8000 });
    ok((await layers()).length === 0, 'CLEAR ALL po potwierdzeniu');

    // Kolejka: dotknięcie ustawia kursor, NEXT bierze pozycję; makro
    const show = await api('/api/show');
    const pg = show.pages.slice(0, 2);
    await api('/api/rundowns', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name: 'Program', items: pg.map((x, i) => ({ id: 'i' + i, page: x.id, note: i ? 'po przerwie' : '' })) }) });
    await api('/api/macros', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name: 'Wejście', steps: [{ delay: 0, cmd: `TAKE ${pg[0].number}` }] }) });
    await p.tap('[data-tab="queue"]');
    await p.waitForSelector('#rd .item');
    await sleep(450);
    await p.locator('#rd .item').nth(1).tap();
    await p.waitForSelector('#rd .item:nth-child(2).next');
    ok((await layers()).length === 0, 'dotknięcie pozycji rundownu niczego nie wysyła');
    await sleep(450);
    await p.tap('#bNext');
    await p.waitForFunction(() => document.querySelector('#rd .item:nth-child(2)').classList.contains('onair'));
    ok((await layers()).some((l) => l.page?.number === pg[1].number), 'NEXT bierze wskazaną pozycję');
    await shot('5-kolejka');
    await sleep(450);
    await api('/api/clear', { method: 'POST' });
    await p.tap('[data-mrun]');
    await sleep(500);
    ok((await layers()).some((l) => l.page?.number === pg[0].number), 'makro z telefonu');
    await api('/api/clear', { method: 'POST' });

    // antena kanału; usunięty kanał wraca na główną zamiast cicho sterować nieistniejącą
    await api('/api/channels/7', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name: 'Sport' }) });
    await p.waitForSelector('#antSel:not([hidden])');
    await sleep(450);
    await p.selectOption('#antSel', '7');
    await p.tap('[data-tab="pages"]');
    await sleep(450);
    await p.tap('#bTake');
    await sleep(400);
    ok((await api('/api/state?channel=7')).layers.length === 1 && (await layers()).length === 0, 'TAKE trafia na wybrany kanał');
    await api('/api/channels/7', { method: 'DELETE' });
    await p.waitForFunction(() => !localStorage.getItem('vizrs.ant') && document.querySelector('#antSel').hidden, null, { timeout: 5000 }).catch(() => {});
    ok(await p.evaluate(() => !localStorage.getItem('vizrs.ant') && document.querySelector('#antSel').hidden), 'usunięty kanał: powrót na antenę główną');

    // poziomo: przyciski w prawej kolumnie
    await p.setViewportSize({ width: 844, height: 390 });
    await sleep(200);
    const bar = await p.locator('#bar').boundingBox();
    ok(bar.x > 500 && bar.height > 300, 'poziomo przyciski są w prawej kolumnie', bar);
    await shot('6-poziomo');
    await p.setViewportSize({ width: 320, height: 568 });
    await sleep(200);
    const clip = await p.evaluate(() => [...document.querySelectorAll('#btns button')].filter((b) => b.scrollWidth > b.clientWidth).map((b) => b.id));
    ok(clip.length === 0, 'na najmniejszym telefonie napisy mieszczą się w przyciskach', clip);
    await shot('7-maly');

    // zanik serwera: widoczny stan, powrót sam
    srv.kill('SIGTERM');
    await p.waitForFunction(() => document.querySelector('#conn').textContent === 'brak połączenia', null, { timeout: 8000 });
    ok(true, 'zanik serwera widać na pasku');
    await sleep(450);
    await p.tap('#bTake');
    await p.waitForFunction(() => document.querySelector('.toast')?.textContent.startsWith('TAKE:'), null, { timeout: 8000 });
    ok(true, 'nieudany TAKE zgłasza błąd');
    await shot('8-brak-polaczenia');
    srv = start();
    await waitUp();
    await p.waitForFunction(() => document.querySelector('#conn').textContent === 'OK', null, { timeout: 15000 });
    ok(true, 'po powrocie serwera strona łączy się sama');

    // service worker: bez serwera aplikacja startuje i pokazuje brak połączenia zamiast błędu przeglądarki
    await p.waitForFunction(() => navigator.serviceWorker.ready.then(() => !!navigator.serviceWorker.controller), null, { timeout: 8000 });
    await p.reload();
    await p.waitForSelector('#pages .item');
    srv.kill('SIGTERM');
    await sleep(500);
    await p.reload();
    await p.waitForFunction(() => document.querySelector('#conn')?.textContent === 'brak połączenia', null, { timeout: 12000 });
    ok(true, 'bez serwera aplikacja startuje z pamięci telefonu');
    srv = start();
    await waitUp();
    await p.waitForFunction(() => document.querySelector('#conn').textContent === 'OK', null, { timeout: 15000 });
    await p.waitForSelector('#pages .item');
    ok(true, 'po powrocie serwera wczytuje dane');

    ok(errs.length === 0, 'brak błędów JS', errs);
  } catch (e) {
    ok(false, 'wyjątek: ' + e.message);
  } finally {
    await browser?.close();
    srv.kill();
    fs.rmSync(data, { recursive: true, force: true });
  }
  console.log(failures ? `\n${failures} błędów` : '\nOK');
  process.exit(failures ? 1 : 0);
})();
