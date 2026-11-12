// E2E Trio na telefonie: ta sama strona /trio w układzie telefonu (≤ 760 px albo dotykowy ekran poziomo) – widoki
// przesuwane palcem, pasek TAKE na dole, szuflada narzędzi spod ☰; dawne /mobile przekierowuje na /trio.
//   node tests/e2e/mobile.js [ścieżka-do-vizrs] [katalog-na-zrzuty]
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
  const post = (p, body) => api(p, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
  const waitUp = async () => { for (let i = 0; i < 50; i++) { try { await api('/api/state'); return; } catch (e) { await sleep(100); } } throw new Error('serwer nie wystartował'); };
  try {
    await waitUp();
    ok((await fetch(B + '/trio')).status === 401, '/trio bez tokenu → 401');
    ok((await (await fetch(B + '/trio')).text()).includes('name="token"'), '/trio bez tokenu pokazuje ekran logowania');
    ok((await fetch(B + '/trio.webmanifest')).status === 200 && (await fetch(B + '/trio-icon.png')).headers.get('content-type') === 'image/png', 'manifest i ikona bez tokenu');
    ok((await fetch(`${B}/trio?token=${OUT_TOKEN}`)).status === 403, 'token wyjścia: /trio → 403');
    const old = await fetch(`${B}/mobile?channel=7`, { redirect: 'manual' });
    ok(old.status === 302 && old.headers.get('location') === '/trio?channel=7', 'dawne /mobile przekierowuje na /trio z parametrami', [old.status, old.headers.get('location')]);
    ok((await fetch(B + '/mobile.webmanifest')).status !== 200, 'dawny manifest /mobile nie jest podawany');

    browser = await chromium.launch();
    const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 });
    const errs = [];
    const p = await ctx.newPage();
    p.on('pageerror', (e) => errs.push(e.message));
    p.on('dialog', (d) => { errs.push('okienko przeglądarki: ' + d.message()); d.accept(); });
    const shot = async (n) => { if (SHOTS) await p.screenshot({ path: path.join(SHOTS, n + '.png') }); };
    const layers = async () => (await api('/api/state')).layers;
    const pages = () => p.locator('#pages .item');
    // widok przewija się płynnie: czekamy, aż stanie (monitor i podgląd startują dopiero wtedy)
    const PANES = ['pages', 'edit', 'air', 'queue'];
    const tapPane = async (v) => {
      await p.tap(`[data-pane-tab="${v}"]`);
      await p.waitForFunction((i) => { const m = document.querySelector('#main'); return Math.abs(m.scrollLeft - i * m.clientWidth) < 2; }, PANES.indexOf(v));
      await sleep(350);
    };
    const clean = async () => { await p.waitForFunction(() => !document.querySelector('#eSave').classList.contains('dirty')); };
    const confirmSheet = async () => { await p.waitForSelector('.dlg-back'); await p.tap('.dlg-btns button.acc'); };

    // ekran logowania: zły token, potem dobry; token nie zostaje w adresie
    const await0 = await api('/api/show');
    await p.goto(`${B}/mobile`);
    ok(new URL(p.url()).pathname === '/trio', 'dawny adres /mobile otwiera /trio', p.url());
    await p.fill('input[name=token]', 'zly');
    await p.tap('form button');
    await p.waitForFunction(() => !document.querySelector('#bad').hidden);
    ok(true, 'zły token: komunikat na ekranie logowania');
    await p.fill('input[name=token]', TOKEN);
    await p.tap('form button');
    await p.waitForSelector('#pages .item');
    ok(new URL(p.url()).search === '', 'token nie zostaje w adresie', p.url());
    await p.goto(`${B}/trio`);
    await p.waitForSelector('#pages .item');
    ok(true, 'zalogowany telefon wchodzi bez ekranu logowania');
    await p.waitForFunction(() => document.querySelector('#conn').textContent === 'połączono');

    const over = await p.evaluate(() => document.documentElement.scrollWidth - innerWidth);
    ok(over <= 0, 'strona mieści się na szerokość telefonu', over);
    ok(await p.evaluate(() => document.querySelector('#ctl').parentElement.id === 'bar'), 'przyciski komend w pasku na dole');
    ok(await p.locator('#nav').isVisible() && !(await p.locator('.keys').isVisible()), 'zakładki widoków zamiast podpowiedzi klawiszy');
    ok(await p.evaluate(() => document.querySelector('#pgm').dataset.src === 'about:blank'), 'monitor PGM nie działa poza widokiem Antena');
    await shot('1-strony');

    // dotknięcie strony niczego nie wysyła na antenę (pierwsza strona jest wybrana od startu)
    const num = +(await pages().first().locator('.num').textContent());
    await pages().nth(1).tap();
    await pages().first().tap();
    await sleep(200);
    ok((await layers()).length === 0, 'dotknięcie strony tylko ją wybiera');
    ok((await p.textContent('#barName')).includes(String(num)), 'pasek pokazuje wybraną stronę');
    ok(await p.locator('#bOut').isDisabled(), 'OUT wyłączony, gdy strony nie ma na antenie');
    await pages().first().dblclick();
    await sleep(300);
    ok((await layers()).length === 0, 'podwójne dotknięcie strony nie wysyła jej na antenę');

    // TAKE: podwójne dotknięcie gra raz
    let takes = 0;
    p.on('request', (r) => { if (r.url().includes('/api/take/')) takes++; });
    await p.evaluate(() => { const b = document.querySelector('#bTake'); b.click(); b.click(); });
    await p.waitForFunction(() => document.querySelector('#barSt').textContent.includes('ON AIR'));
    ok(takes === 1, 'podwójne dotknięcie TAKE wysyła jedną komendę', takes);
    ok((await layers()).some((l) => l.page?.number === num), 'TAKE z telefonu jest na antenie');
    await p.waitForFunction(() => document.querySelector('#top').classList.contains('air'));
    ok(await p.evaluate(() => document.querySelector('meta[name=theme-color]').content === '#2a0f0e'), 'antena zajęta: czerwony nagłówek i pasek systemu');
    await shot('2-on-air');

    // edycja pola: podgląd nad polami, UPDATE bez zapisu, potem zapis
    await tapPane('edit');
    const field = p.locator('#fields .fl input.big, #fields .fl textarea.big').first();
    const fid = await field.getAttribute('data-f');
    await p.waitForFunction(() => document.querySelector('#pvStage').childElementCount > 0);
    ok(true, 'Edycja pokazuje podgląd strony');
    const [pvY, fY] = await p.evaluate(() => [document.querySelector('#pvBox').getBoundingClientRect().top, document.querySelector('#fields').getBoundingClientRect().top]);
    ok(pvY < fY, 'podgląd nad polami', [pvY, fY]);
    await field.fill('Z telefonu');
    await sleep(100);
    ok(/z telefonu/i.test(await p.textContent('#pvStage')), 'podgląd śledzi wpisywany tekst');
    ok(await p.evaluate(() => document.body.classList.contains('kb')), 'klawiatura ekranowa chowa zakładki');
    await shot('3-edycja');
    await sleep(450);
    // dotknięcie przy otwartej klawiaturze: przycisk nie może uciec spod palca, gdy wraca pasek zakładek
    await p.tap('#bUpd');
    await sleep(400);
    const updated = await p.evaluate(() => performance.getEntriesByType('resource').some((r) => r.name.includes('/api/update/') && r.responseStatus === 200));
    ok(updated, 'UPDATE działa na stronie na antenie');
    const before = (await api('/api/show')).pages.find((x) => x.number === num);
    ok(before.values?.[fid] !== 'Z telefonu', 'UPDATE nie zapisuje strony');
    ok((await api('/api/state')).locks?.some((l) => l.kind === 'pages' && l.id === before.id), 'wybrana strona jest zablokowana dla innych');
    await p.tap('#eSave');
    await clean();
    await sleep(200);
    ok((await api('/api/show')).pages.find((x) => x.number === num).values[fid] === 'Z telefonu', 'Zapisz zapisuje stronę');

    // czas strony na kanale (⏱ kanał): przed końcem bez liczby = 60 s, zapis w page.playout
    ok((await p.textContent('#eplaySum')).includes('domyślnie'), '⏱ kanał: domyślnie bez page.playout');
    await p.tap('#eplaySum');
    await p.selectOption('#eplAt', 'end');
    ok(await p.inputValue('#eplSec') === '60' && (await p.textContent('#eplaySum')).includes('60 s przed końcem'), '„przed końcem” bez liczby = 60 s');
    await p.fill('#eplOut', '5'); await p.dispatchEvent('#eplOut', 'change');
    await p.selectOption('#eplRef', 'item');
    await p.tap('#eSave');
    await clean();
    await sleep(200);
    const pl = (await api('/api/show')).pages.find((x) => x.number === num).playout;
    ok(JSON.stringify(pl) === JSON.stringify({ at: -60, out: 5, ref: 'item' }), 'Zapisz zapisuje page.playout', pl);
    await p.selectOption('#eplAt', ''); await p.fill('#eplOut', ''); await p.dispatchEvent('#eplOut', 'change'); await p.selectOption('#eplRef', '');
    await p.tap('#eSave');
    await clean();
    await sleep(200);
    ok((await api('/api/show')).pages.find((x) => x.number === num).playout === undefined, 'domyślne ustawienia usuwają page.playout');
    await p.evaluate(() => document.activeElement?.blur());
    await sleep(500);

    // przesunięcie palcem zmienia widok; podgląd działa tylko na Edycji
    await p.evaluate(() => { document.querySelector('#main').scrollLeft = 0; });
    await p.waitForFunction(() => document.querySelector('#nav .on').dataset.paneTab === 'pages');
    await sleep(300);
    ok(await p.evaluate(() => document.querySelector('#pvStage').childElementCount === 0), 'poza Edycją podgląd jest wyłączony');

    // przytrzymanie strony = menu strony; ticker: lista wiadomości, zapis KAT|treść
    const tick = await0.pages.find((x) => { const sc = (await0.scenes || []).find((c) => c.id === x.scene); return sc && JSON.stringify(sc.nodes).includes('"ticker"'); });
    const row = p.locator(`#pages .item[data-id="${tick.id}"]`);
    await row.scrollIntoViewIfNeeded();
    await p.evaluate((id) => { const el = document.querySelector(`#pages .item[data-id="${id}"]`); el.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerType: 'touch' })); }, tick.id);
    await p.waitForSelector('.dlg-back');
    ok((await p.textContent('.dlg-text')).includes(String(tick.number)), 'przytrzymanie strony otwiera menu strony');
    const sheetY = await p.evaluate(() => { const r = document.querySelector('.dlg').getBoundingClientRect(); return innerHeight - r.bottom; });
    ok(Math.abs(sheetY) < 2, 'menu jako arkusz od dołu ekranu', sheetY);
    await shot('3b-menu');
    await p.tap('.dlg-btns button.acc');
    await p.waitForFunction(() => document.querySelector('#nav .on').dataset.paneTab === 'edit');
    await p.waitForSelector('.tkx-row');
    await sleep(600);
    await shot('3c-ticker');
    await p.tap('.tkx [data-a="add"]');
    await p.locator('.tkx-cat').last().fill('SPORT');
    await p.locator('.tkx-txt').last().fill('Gol');
    ok((await p.inputValue('[data-tk]')).endsWith('SPORT|Gol'), 'nowa wiadomość tickera w formacie KAT|treść', await p.inputValue('[data-tk]'));
    await p.tap('#eRevert');
    await p.evaluate(() => document.activeElement?.blur());
    await tapPane('pages');
    await pages().first().tap();
    await sleep(150);

    // Antena: monitor tylko na tym widoku, OUT warstwy, CLEAR po potwierdzeniu
    await tapPane('air');
    await p.waitForSelector('#layers .item [data-lout]');
    ok(await p.evaluate(() => document.querySelector('#pgm').dataset.src.includes('/output') && document.querySelector('#pgm').dataset.src.includes('fps=15')), 'widok Antena włącza monitor PGM (15 kl./s)');
    await sleep(900);
    await shot('4-antena');
    await p.tap('#layers [data-lout]');
    await p.waitForFunction(() => document.querySelector('#layers').textContent.includes('Antena czysta'), null, { timeout: 8000 });
    ok((await layers()).length === 0, 'OUT warstwy z telefonu');
    await api(`/api/take/${num}`, { method: 'POST' });
    await p.waitForSelector('#layers [data-lout]');
    await sleep(450);
    await p.tap('#bClear2');
    await p.waitForSelector('.dlg-back');
    ok((await layers()).length > 0, 'CLEAR ALL czeka na potwierdzenie');
    await shot('4b-arkusz');
    await confirmSheet();
    await p.waitForFunction(() => document.querySelector('#layers').textContent.includes('Antena czysta'), null, { timeout: 8000 });
    ok((await layers()).length === 0, 'CLEAR ALL po potwierdzeniu');
    await tapPane('pages');
    ok(await p.evaluate(() => document.querySelector('#pgm').dataset.src === 'about:blank'), 'po zejściu z widoku Antena monitor się wyłącza');

    // Kolejka: ⏵ ustawia kursor rundownu, NEXT bierze pozycję; makro; karuzela
    const pg = await0.pages.slice(0, 2);
    await post('/api/rundowns', { name: 'Program', items: pg.map((x, i) => ({ id: 'i' + i, page: x.id, note: i ? 'po przerwie' : '' })) });
    await post('/api/macros', { name: 'Wejście', steps: [{ delay: 0, cmd: `TAKE ${pg[0].number}` }] });
    await tapPane('queue');
    await p.tap('[data-tab="rd"]');
    await p.waitForSelector('#rd .item');
    await sleep(450);
    await p.locator('#rd .item').nth(1).locator('[data-rdcur]').tap();
    await p.waitForSelector('#rd .item:nth-child(2).next');
    ok((await layers()).length === 0, 'ustawienie kursora rundownu niczego nie wysyła');
    await sleep(450);
    await p.tap('#rdNext');
    await p.waitForFunction(() => document.querySelector('#rd .item:nth-child(2)').classList.contains('onair'));
    ok((await layers()).some((l) => l.page?.number === pg[1].number), 'NEXT bierze wskazaną pozycję');
    await shot('5-kolejka');
    await sleep(450);
    await api('/api/clear', { method: 'POST' });
    await p.tap('[data-tab="mc"]');
    await sleep(450);
    await p.tap('#mcRun');
    await p.waitForFunction(async () => true);
    await sleep(800);
    ok((await layers()).some((l) => l.page?.number === pg[0].number), 'makro z telefonu');
    await api('/api/clear', { method: 'POST' });
    await post('/api/playlists', { name: 'Pętla', loop: true, items: [{ page: pg[0].id, duration: 30 }] });
    await p.tap('[data-tab="pl"]');
    await p.waitForFunction(() => [...document.querySelectorAll('#plSel option')].some((o) => o.textContent === 'Pętla'));
    await p.selectOption('#plSel', { label: 'Pętla' });
    await sleep(450);
    await p.tap('#plStart');
    await sleep(500);
    ok((await api('/api/state')).carousel?.playlist, 'karuzela startuje z telefonu');
    await sleep(450);
    await p.tap('#plStop');
    await sleep(400);
    await api('/api/clear', { method: 'POST' });

    // szuflada spod ☰: antena kanału; usunięty kanał wraca na główną zamiast cicho sterować nieistniejącą
    await post('/api/channels/7', { name: 'Sport' });
    await tapPane('pages');
    await p.waitForFunction(() => [...document.querySelectorAll('#antSel option')].some((o) => o.value === '7'));
    const drawerX = () => p.evaluate(() => document.querySelector('#tools').getBoundingClientRect().right);
    ok(await drawerX() <= 0, 'wybór anteny schowany w szufladzie', await drawerX());
    await p.tap('#bMenu');
    await p.waitForFunction(() => document.querySelector('#tools').getBoundingClientRect().left >= -1);
    ok(await p.locator('#antSel').isVisible() && await p.locator('#bSys').isVisible(), 'szuflada pokazuje narzędzia stanowiska');
    await shot('6-szuflada');
    await p.selectOption('#antSel', '7');
    await p.waitForFunction(() => !document.body.classList.contains('nav-open'));
    ok(true, 'wybór w szufladzie ją zamyka');
    await sleep(450);
    await p.tap('#bTake');
    await sleep(500);
    ok((await api('/api/state?channel=7')).layers.length === 1 && (await layers()).length === 0, 'TAKE trafia na wybrany kanał');
    await api('/api/channels/7', { method: 'DELETE' });
    await p.waitForFunction(() => !localStorage.getItem('vizrs.ant') && document.querySelector('#antSel').value === '', null, { timeout: 5000 }).catch(() => {});
    ok(await p.evaluate(() => !localStorage.getItem('vizrs.ant') && document.querySelector('#antSel').value === ''), 'usunięty kanał: powrót na antenę główną');

    // poziomo: przyciski w prawej kolumnie; najmniejszy telefon
    await p.setViewportSize({ width: 844, height: 390 });
    await sleep(300);
    const bar = await p.locator('#bar').boundingBox();
    ok(bar.x > 500 && bar.height > 300, 'poziomo przyciski są w prawej kolumnie', bar);
    await shot('7-poziomo');
    await p.setViewportSize({ width: 320, height: 568 });
    await sleep(300);
    const clip = await p.evaluate(() => [...document.querySelectorAll('#ctl button')].filter((b) => b.offsetParent && b.scrollWidth > b.clientWidth).map((b) => b.id));
    ok(clip.length === 0, 'na najmniejszym telefonie napisy mieszczą się w przyciskach', clip);
    ok(await p.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'najmniejszy telefon: bez przewijania w bok');
    await shot('8-maly');
    await p.setViewportSize({ width: 390, height: 844 });
    await sleep(300);

    // zanik serwera: widoczny stan, powrót sam
    srv.kill('SIGTERM');
    await p.waitForFunction(() => document.querySelector('#conn').textContent === 'brak połączenia', null, { timeout: 8000 });
    ok(true, 'zanik serwera widać na pasku');
    await sleep(450);
    await p.tap('#bTake');
    await p.waitForFunction(() => document.querySelector('.toast')?.textContent.startsWith('TAKE:'), null, { timeout: 8000 });
    ok(true, 'nieudany TAKE zgłasza błąd');
    srv = start();
    await waitUp();
    await p.waitForFunction(() => document.querySelector('#conn').textContent === 'połączono', null, { timeout: 15000 });
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
    await p.waitForFunction(() => document.querySelector('#conn').textContent === 'połączono', null, { timeout: 15000 });
    await p.waitForSelector('#pages .item');
    ok(true, 'po powrocie serwera wczytuje dane');
    ok(errs.length === 0, 'telefon: brak błędów JS', errs);

    // stanowisko: wąskie okno przechodzi w układ telefonu i wraca (jak panel VizRS Live)
    const d = await browser.newContext({ viewport: { width: 1600, height: 900 } });
    const w = await d.newPage();
    const werrs = [];
    w.on('pageerror', (e) => werrs.push(e.message));
    await w.goto(`${B}/trio?token=${TOKEN}`);
    await w.waitForSelector('#pages .item');
    ok(await w.evaluate(() => document.querySelector('#ctl').parentElement.id === 'colEdit' && !document.querySelector('#nav').offsetParent), 'stanowisko: trzy kolumny, przyciski pod edytorem');
    ok(await w.evaluate(() => document.querySelector('#pgm').dataset.src.startsWith('/output') && !document.querySelector('#pgm').dataset.src.includes('fps')), 'stanowisko: monitor PGM zawsze w pełnym klatkażu');
    await w.setViewportSize({ width: 700, height: 900 });
    await sleep(300);
    ok(await w.evaluate(() => document.querySelector('#ctl').parentElement.id === 'bar' && !!document.querySelector('#nav').offsetParent), 'wąskie okno: układ telefonu');
    await w.setViewportSize({ width: 1600, height: 900 });
    await sleep(300);
    ok(await w.evaluate(() => document.querySelector('#ctl').parentElement.id === 'colEdit' && document.querySelector('#bTake').disabled === false), 'szerokie okno: z powrotem układ stanowiska');
    ok(werrs.length === 0, 'stanowisko: brak błędów JS', werrs);
    await d.close();
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
