// E2E VizRS: uruchamia binarkę na wolnym porcie z pustym katalogiem danych i sprawdza UI w Chromium.
//   node tests/e2e/run.js [ścieżka-do-vizrs]
// Playwright: zmienna PLAYWRIGHT_DIR (katalog z node_modules/playwright) albo zwykłe `require('playwright')`.
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
const PORT = 18000 + Math.floor(Math.random() * 1000);
const B = `http://127.0.0.1:${PORT}`;
const TOKEN = 'e2e-token-1';
let failures = 0;
const ok = (cond, msg, info) => { console.log((cond ? 'PASS ' : 'FAIL ') + msg + (!cond && info !== undefined ? ' → ' + JSON.stringify(info) : '')); if (!cond) failures++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitUp() {
  for (let i = 0; i < 50; i++) { try { await fetch(B + '/api/state', { headers: { 'X-Token': TOKEN } }); return; } catch (e) { await sleep(100); } }
  throw new Error('serwer nie wystartował');
}

(async () => {
  const { chromium } = loadPlaywright();
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-e2e-'));
  const srv = spawn(BIN, ['--port', String(PORT), '--tcp', '0', '--data', data, '--token', TOKEN], { stdio: 'ignore' });
  let browser;
  try {
    await waitUp();
    const api = (p, o = {}) => fetch(B + p, { ...o, headers: { 'X-Token': TOKEN, ...(o.headers || {}) } }).then((r) => r.json());

    // --- zabezpieczenia ---
    ok((await fetch(B + '/api/state')).status === 401, 'API bez tokenu → 401');
    ok((await fetch(B + '/trio')).status === 401, 'UI bez tokenu → 401');
    ok((await fetch(B + '/api/state', { headers: { 'Sec-Fetch-Site': 'cross-site', 'X-Token': TOKEN } })).status === 403, 'żądanie cross-site → 403');
    ok((await fetch(B + '/api/clear', { headers: { Origin: 'http://evil.example', 'X-Token': TOKEN } })).status === 403, 'obcy Origin → 403');

    browser = await chromium.launch();
    const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
    const errs = [];
    const page = async (url) => {
      const p = await ctx.newPage();
      p.on('pageerror', (e) => errs.push(url + ': ' + e.message));
      await p.goto(B + url);
      return p;
    };

    // --- Output + Trio (cookie z tokenem ustawiane przy pierwszym wejściu) ---
    const out = await page('/output?checker=1&token=' + TOKEN);
    await sleep(800);
    await api('/api/take/1001?headline=E2E%20TEKST');
    await sleep(2200);
    ok(await out.evaluate(() => document.body.innerText.includes('E2E TEKST')), 'TAKE → tekst widoczny na Output');
    await api('/api/update/1001?headline=ZMIENIONE&save=0');
    await sleep(800);
    ok(await out.evaluate(() => document.body.innerText.includes('ZMIENIONE')), 'UPDATE na żywo → Output');
    await api('/api/out/main');
    await sleep(2500);
    ok(await out.evaluate(() => !document.body.innerText.includes('ZMIENIONE')), 'OUT → tekst znika z Output');

    // --- zepsuty węzeł nie psuje reszty sceny ani innych warstw; błąd trafia do serwera ---
    const show = await api('/api/show');
    show.scenes.push({
      id: 'bad', name: 'Bad', layer: 'front', width: 1920, height: 1080, duration: 1, stops: [],
      nodes: [
        { id: 'n1', type: 'text', name: 'zly', props: { text: { nie: 'tekst' }, font: [1], size: 'x' }, anim: {} },
        { id: 'n2', type: 'rect', name: 'dobry', props: { x: 10, y: 10, w: 200, h: 50, color: '#ff0000' } },
      ],
    });
    show.pages.push({ id: 'pbad', number: 9000, scene: 'bad', values: {} });
    const imp = await fetch(B + '/api/show?force=1', { method: 'PUT', headers: { 'X-Token': TOKEN }, body: JSON.stringify(show) });
    ok(imp.status === 200, 'import show z force=1 → 200');
    await api('/api/take/1001?headline=PRZED');
    await api('/api/take/9000');
    await sleep(1800);
    ok(await out.evaluate(() => document.body.innerText.includes('PRZED')), 'błędna scena nie psuje innych warstw');
    ok(await out.evaluate(() => { const r = document.querySelector('.vz-layer[data-layer="front"] .vz-rect'); return !!r && r.style.display !== 'none'; }), 'zdrowy węzeł w tej samej scenie nadal się rysuje');
    // wymuszony wyjątek w węźle (getter rzucający) – izolacja + zgłoszenie do serwera
    const isolated = await out.evaluate(async () => {
      const { ScenePlayer } = await import('/renderer.js');
      const div = document.createElement('div'); document.body.appendChild(div);
      const bad = { id: 'b', type: 'rect', name: 'wybuchowy' };
      Object.defineProperty(bad, 'props', { get() { throw new Error('boom-e2e'); } });
      const good = { id: 'g', type: 'rect', name: 'dobry', props: { x: 0, y: 0, w: 50, h: 50, color: '#0f0' } };
      const pl = new ScenePlayer(div, { name: 'WymuszonyBlad', width: 1920, height: 1080, duration: 1, stops: [], nodes: [bad, good] }, {});
      pl.seek(0);
      const g = div.querySelector('[data-id="g"]'), b = div.querySelector('[data-id="b"]');
      const r = { goodShown: g.style.display !== 'none', badHidden: b.style.display === 'none' };
      pl.destroy(); div.remove();
      return r;
    });
    ok(isolated.goodShown && isolated.badHidden, 'wyjątek w jednym węźle: wadliwy ukryty, reszta sceny gra');
    await sleep(600);
    const errs0 = await api('/api/errors');
    ok(errs0.some((e) => e.message.includes('boom-e2e')), 'błąd węzła zgłoszony do /api/errors');
    ok((await api('/health')).sceneErrors >= 1, '/health pokazuje błędy scen');
    await api('/api/errors', { method: 'DELETE' });
    await api('/api/clear');

    // --- fonty offline ---
    const css = await fetch(B + '/fonts/fonts.css', { headers: { 'X-Token': TOKEN } });
    ok(css.status === 200 && (await css.text()).includes('Barlow Condensed'), 'fonts.css serwowany lokalnie');
    ok(await out.evaluate(async () => { await document.fonts.load('700 20px "Barlow Condensed"'); return document.fonts.check('700 20px "Barlow Condensed"'); }), 'Barlow Condensed ładuje się offline');
    ok(await out.evaluate(() => ![...document.querySelectorAll('link')].some((l) => /fonts\.googleapis/.test(l.href && l.href) && /Barlow\+Condensed/.test(l.href))), 'brak żądania Google Fonts dla fontu offline');

    // --- snapshot bez migotania (reconnect) ---
    const t0 = await api('/api/take/1002');
    await sleep(2000);
    const flick = await out.evaluate(async () => {
      const host = window.__host; // udostępnione przez output.html
      const before = document.querySelector('.vz-layer[data-layer="main"] .vz-scene');
      const snap = await fetch('/api/state').then((r) => r.json());
      return { hasHost: !!host, same: !!before };
    });
    ok(flick.same, 'warstwa main istnieje przed snapshotem');
    const same = await out.evaluate(async () => {
      const { LayerHost } = await import('/renderer.js');
      const div = document.createElement('div'); document.body.appendChild(div);
      const h = new LayerHost(div);
      const ev = await new Promise((res) => { const es = new EventSource('/events'); es.onmessage = (m) => { es.close(); res(JSON.parse(m.data)); }; });
      h.handle(ev);
      const first = div.querySelector('.vz-scene');
      h.handle(ev); // ponowny snapshot tego samego stanu – ten sam węzeł DOM
      const second = div.querySelector('.vz-scene');
      return ev.t === 'snapshot' && !!first && first === second;
    });
    ok(same, 'ponowny snapshot nie przebudowuje tej samej strony (brak migotania)');
    await api('/api/clear');

    // --- odroczona aktualizacja szablonu ---
    await api('/api/take/1001?headline=SZABLON');
    const sc = (await api('/api/show')).scenes.find((x) => x.name === 'Belka');
    sc.name = 'Belka v2';
    const put = await fetch(B + '/api/scenes/' + sc.id, { method: 'PUT', headers: { 'X-Token': TOKEN }, body: JSON.stringify(sc) });
    ok(put.status === 200, 'zapis szablonu na antenie przyjęty');
    let stt = await api('/api/state');
    ok(stt.layers.some((l) => l.pending), 'warstwa oznaczona jako „czeka nowa wersja szablonu”');
    sc.rev = 0; // nieaktualny odczyt
    const stale = await fetch(B + '/api/scenes/' + sc.id, { method: 'PUT', headers: { 'X-Token': TOKEN }, body: JSON.stringify(sc) });
    ok(stale.status === 409, 'nieaktualny rev sceny → 409');
    await api('/api/deploy/main', { method: 'POST' });
    stt = await api('/api/state');
    ok(!stt.layers.some((l) => l.pending), 'po „Wdróż teraz” brak oczekującej aktualizacji');
    await api('/api/clear');

    // --- Trio: klawisze ---
    const trio = await page('/trio');
    await sleep(800);
    await trio.click('text=Belka PILNE – A2');
    await trio.keyboard.press('F1');
    await sleep(1200);
    let st = await api('/api/state');
    ok(st.layers.some((l) => l.page.number === 1002), 'Trio F1 → TAKE strony 1002');
    await trio.keyboard.press('F3');
    await sleep(600);
    st = await api('/api/state');
    ok(st.layers.length === 0, 'Trio F3 → OUT');

    // --- Artist ---
    const art = await page('/artist');
    await sleep(800);
    ok((await art.locator('#tl').count()) > 0, 'Artist: oś czasu załadowana');

    // --- v1.3: ARM / preview bus, key/fill ---
    await api('/api/arm/1002?headline=ARMED');
    const prv = await page('/output?bus=preview&checker=1');
    await sleep(2200);
    ok(await prv.evaluate(() => document.body.innerText.includes('ARMED')), 'ARM → strona widoczna na wyjściu Preview');
    ok(await out.evaluate(() => !document.body.innerText.includes('ARMED')), 'ARM nie wchodzi na wyjście Program');
    ok((await api('/api/state')).layers.length === 0, 'ARM nie zmienia stanu anteny');
    await api('/api/preview/clear', { method: 'POST' });
    await sleep(400);
    ok(await prv.evaluate(() => !document.body.innerText.includes('ARMED')), 'preview/clear czyści wyjście Preview');
    await prv.close();

    const sh2 = await api('/api/show');
    sh2.scenes.push({ id: 'kf', name: 'KeyFill', layer: 'front', width: 1920, height: 1080, duration: 0.2, stops: [0.2],
      nodes: [{ id: 'r1', type: 'rect', name: 'a', props: { x: 100, y: 100, w: 400, h: 200, color: '#ff0000' } },
              { id: 'r2', type: 'rect', name: 'b', props: { x: 700, y: 100, w: 400, h: 200, color: '#ff000080' } }] });
    sh2.pages.push({ id: 'pkf', number: 9100, scene: 'kf', values: {} });
    await fetch(B + '/api/show?force=1', { method: 'PUT', headers: { 'X-Token': TOKEN }, body: JSON.stringify(sh2) });
    await api('/api/take/9100');
    const px = async (mode) => {
      const pg = await ctx.newPage();
      await pg.setViewportSize({ width: 1920, height: 1080 });
      await pg.goto(B + '/output?mode=' + mode);
      await sleep(1500);
      const png = await pg.screenshot();
      await pg.close();
      const probe = await ctx.newPage();
      const r = await probe.evaluate(async (b64) => {
        const img = new Image(); img.src = 'data:image/png;base64,' + b64; await img.decode();
        const c = document.createElement('canvas'); c.width = img.width; c.height = img.height; const g = c.getContext('2d'); g.drawImage(img, 0, 0);
        const at = (x, y) => Array.from(g.getImageData(x, y, 1, 1).data).slice(0, 3);
        return { solid: at(300, 200), half: at(900, 200), none: at(1500, 800) };
      }, png.toString('base64'));
      await probe.close();
      return r;
    };
    const near = (a, b, t = 14) => a.every((v, i) => Math.abs(v - b[i]) <= t);
    const fill = await px('fill'), key = await px('key');
    ok(near(fill.solid, [255, 0, 0]) && near(fill.half, [128, 0, 0]) && near(fill.none, [0, 0, 0]), 'Fill: kolor na czerni (pełny, półprzezroczysty, tło)');
    ok(near(key.solid, [255, 255, 255]) && near(key.half, [128, 128, 128]) && near(key.none, [0, 0, 0]), 'Key: alfa jako luma (biały / szary / czarny)');
    await api('/api/clear');

    // --- v1.3: komendy zbiorcze, makro, rundown (API) ---
    await api('/api/take/1001,3001');
    ok((await api('/api/state')).layers.length === 2, 'TAKE z listą stron (1001,3001) → dwie warstwy');
    await api('/api/out/main,back');
    ok((await api('/api/state')).layers.length === 0, 'OUT z listą warstw → antena czysta');
    await api('/api/macros', { method: 'POST', body: JSON.stringify({ id: 'mk', name: 'Seg', steps: [{ cmd: 'CLEAR', delay: 0 }, { cmd: 'TAKE 1001', delay: 0 }, { cmd: 'TAKE 3001', delay: 0.4 }] }) });
    await api('/api/macro/Seg', { method: 'POST' });
    await sleep(200);
    ok((await api('/api/state')).layers.length === 1, 'makro: pierwszy krok natychmiast, drugi po opóźnieniu');
    await sleep(600);
    ok((await api('/api/state')).layers.length === 2, 'makro: po opóźnieniu dwie warstwy');
    await api('/api/clear');
    const pgs = (await api('/api/show')).pages;
    await api('/api/rundowns', { method: 'POST', body: JSON.stringify({ id: 'rdx', name: 'Dziennik', items: [
      { id: 'a', page: pgs.find((p) => p.number === 1001).id, note: 'otwarcie' }, { id: 'b', page: pgs.find((p) => p.number === 1002).id, note: '' }] }) });

    // --- v1.3: Trio – rundown, skrót strony, ARM (UI) ---
    await trio.reload();
    await sleep(800);
    await trio.click('[data-tab="rd"]');
    await sleep(300);
    ok((await trio.locator('#rd .item').count()) === 2, 'Trio: rundown pokazuje 2 pozycje');
    await trio.keyboard.press('F8');
    await sleep(900);
    let st2 = await api('/api/state');
    ok(st2.layers.some((l) => l.page.number === 1001) && st2.rundowns.find((r) => r.id === 'rdx').cursor === 'b', 'Trio: F8 (NEXT) bierze 1. pozycję i przesuwa kursor');
    await trio.click('#rdNext');
    await sleep(900);
    st2 = await api('/api/state');
    ok(st2.layers.some((l) => l.page.number === 1002), 'Trio: przycisk NEXT bierze 2. pozycję');
    ok(st2.rundowns.find((r) => r.id === 'rdx').done.includes('a'), '1. pozycja oznaczona jako odegrana');
    await api('/api/clear');
    // skrót strony: ustawiamy przez API (z rev), naciskamy w Trio
    const p3003 = (await api('/api/show')).pages.find((p) => p.number === 3003);
    await api('/api/pages/' + p3003.id, { method: 'PUT', body: JSON.stringify({ ...p3003, hotkey: 'Ctrl+Alt+K' }) });
    await trio.reload(); await sleep(800);
    await trio.keyboard.press('Control+Alt+K');
    await sleep(800);
    ok((await api('/api/state')).layers.some((l) => l.page.number === 3003), 'Trio: skrót Ctrl+Alt+K wywołuje TAKE strony');
    await trio.keyboard.press('Control+Alt+K');
    await sleep(800);
    ok((await api('/api/state')).layers.length === 0, 'Trio: ten sam skrót zdejmuje stronę z anteny');
    await trio.click('text=Belka – budżet'); await trio.keyboard.press('F6'); await sleep(700);
    ok((await api('/api/state')).preview.some((x) => x.page.number === 1001), 'Trio: F6 uzbraja stronę w preview');
    await api('/api/preview/clear', { method: 'POST' });

    // --- v1.4: tekst sformatowany (znaczniki), reguły pól, Word Replace, edycja w podglądzie ---
    const sh3 = await api('/api/show');
    sh3.scenes.push({ id: 'rules', name: 'Reguly', layer: 'front', width: 1920, height: 1080, duration: 0.2, stops: [0.2],
      fieldMeta: {
        title: { label: 'Tytuł', maxChars: 20, softMax: 10, upper: true, typo: true, tab: 2, help: 'wpisz tytuł' },
        sub: { label: 'Podtytuł', maxLines: 2, tab: 1 },
        ro: { label: 'Stały', readonly: true, tab: 3 },
        each: { label: 'Wiersze', maxLines: 2, tab: 4 },
      },
      nodes: [
        { id: 'tt', type: 'text', name: 'titleNode', fields: { text: 'title' }, props: { x: 100, y: 100, w: 1200, h: 100, size: 60, color: '#ffffff', font: 'Barlow Condensed', text: 'Tytul', textFit: 'squeeze-lines', maxLines: 2 } },
        { id: 'st', type: 'text', name: 'subNode', fields: { text: 'sub' }, props: { x: 100, y: 300, w: 1200, h: 100, size: 50, color: '#ffffff', font: 'Barlow Condensed', text: 'Sub', textFit: 'squeeze-lines', maxLines: 2 } },
        { id: 'ea', type: 'text', name: 'eachNode', fields: { text: 'each' }, props: { x: 100, y: 700, w: 600, h: 140, size: 50, color: '#ffffff', font: 'Barlow Condensed', text: 'Each', textFit: 'squeeze-each', maxLines: 2, squeezeMin: 0.6 } },
        { id: 'rt', type: 'text', name: 'roNode', fields: { text: 'ro' }, props: { x: 100, y: 500, w: 600, h: 60, size: 40, color: '#ffffff', font: 'Barlow Condensed', text: 'stały' } },
      ] });
    sh3.pages.push({ id: 'prules', number: 9200, scene: 'rules', values: { title: 'a', sub: 'b', ro: 'stały' } });
    await fetch(B + '/api/show?force=1', { method: 'PUT', headers: { 'X-Token': TOKEN }, body: JSON.stringify(sh3) });
    await fetch(B + '/api/wordreplace', { method: 'PUT', headers: { 'X-Token': TOKEN }, body: JSON.stringify({ '@p': ['Prezydent'], '@m': ['Minister', 'Ministra'] }) });

    // render znaczników na Output
    await api('/api/take/9200?sub=' + encodeURIComponent('Zwykły {b}gruby{/b} {c=#ff0000}czerwony{/c} {u}pod{/u} {sz=150}duży{/sz} [[wyróżnienie]]'));
    await sleep(1800);
    const fm = await out.evaluate(() => {
      const root = document.querySelector('[data-id="st"] span');
      const sp = [...root.querySelectorAll('span')];
      const css = (t) => { const e = sp.find((x) => x.textContent === t); return e ? getComputedStyle(e) : null; };
      return {
        bold: css('gruby')?.fontWeight, red: css('czerwony')?.color, under: css('pod')?.textDecorationLine, big: sp.find((x) => x.textContent === 'duży')?.style.fontSize,
        mark: !!root.querySelector('mark'), raw: root.textContent.includes('{b}') || root.textContent.includes('{c='),
      };
    });
    ok(Number(fm.bold) >= 800 && fm.red === 'rgb(255, 0, 0)' && fm.under === 'underline' && fm.big === '150%' && fm.mark && !fm.raw, 'znaczniki {b} {c=} {u} {sz=} [[ ]] renderują się, bez surowych znaczników', fm);
    // znacznik obejmujący dwa wiersze (squeeze-lines) – każdy wiersz poprawny
    await api('/api/take/9200?sub=' + encodeURIComponent('{b}pierwszy\ndrugi{/b}'));
    await sleep(1200);
    const carry = await out.evaluate(() => [...document.querySelector('[data-id="st"] > span').children].map((l) => (l.querySelector('span') ? getComputedStyle(l.querySelector('span')).fontWeight : 'brak')));
    ok(carry.length === 2 && carry.every((w) => Number(w) >= 800), 'znacznik {b}…{/b} przenosi się na oba wiersze', carry);
    // squeeze-each: wiersze operatora, każdy ściśnięty osobno, font (wysokość) bez zmian
    const eachLines = () => out.evaluate(() => [...document.querySelector('[data-id="ea"] > span').children].map((l) => ({ t: l.textContent, tf: l.style.transform, fs: l.style.fontSize })));
    const sx = (tf) => +(/scaleX\(([\d.]+)\)/.exec(tf)?.[1] ?? 1);
    await api('/api/take/9200?each=' + encodeURIComponent('Krótki\nBardzo długi drugi wiersz belki, który trzeba ścisnąć'));
    await sleep(1200);
    let el2 = await eachLines();
    ok(el2.length === 2 && sx(el2[0].tf) === 1 && sx(el2[1].tf) < 1 && sx(el2[1].tf) >= 0.6 && el2.every((l) => !l.fs), 'squeeze-each: każdy wiersz ma własne ściśnięcie, font bez zmian', el2);
    await api('/api/take/9200?each=' + encodeURIComponent('Wiersz o wiele za długi nawet przy granicy ściśnięcia sześćdziesięciu procent, naprawdę bardzo długi\nDrugi'));
    await sleep(1200);
    el2 = await eachLines();
    ok(el2.length === 2 && sx(el2[0].tf) === 0.6 && el2.every((l) => !l.fs), 'squeeze-each: za długi wiersz zatrzymuje się na squeezeMin (font bez zmian)', el2);
    await api('/api/clear');

    // pole w Trio z regułami
    await trio.reload(); await sleep(900);
    await trio.fill('#filter', '9200'); await sleep(200);
    await trio.click('#pages .item'); await sleep(500);
    const order = await trio.evaluate(() => [...document.querySelectorAll('#fields [data-f]')].map((e) => e.dataset.f));
    ok(order.join() === 'sub,title,ro,each', 'kolejność Tab wg reguły tab (sub, title, ro, each)', order);
    ok(await trio.evaluate(() => document.querySelector('#fields').innerText.includes('wpisz tytuł')), 'podpowiedź pola widoczna');
    ok(await trio.evaluate(() => document.querySelector('[data-f="ro"]').readOnly), 'pole readonly');
    const tf = trio.locator('[data-f="title"]');
    await tf.fill('');
    await tf.type('wersaliki');
    ok((await tf.inputValue()) === 'WERSALIKI', 'reguła upper zamienia na wersaliki');
    ok((await trio.locator('[data-cnt="title"]').innerText()).startsWith('9 / 20'), 'licznik znaków 9 / 20');
    await tf.type('abc');
    ok(await trio.evaluate(() => document.querySelector('[data-cnt="title"]').className.includes('warn')), 'licznik żółty po przekroczeniu softMax');
    await tf.fill('');
    await tf.type('x'.repeat(30));
    ok((await tf.inputValue()).length <= 20, 'twardy limit maxChars nie pozwala przekroczyć 20 znaków');
    // typografia na zakończenie edycji: „a i w” → twarde spacje, cudzysłowy
    await tf.fill('w domu i "tam" - ok');
    await tf.blur(); await sleep(300);
    const typo = await tf.inputValue();
    ok(typo.includes('W\u00a0DOMU') && typo.includes('I\u00a0\u201eTAM\u201d') && typo.includes('\u2013'), 'polska typografia po zakończeniu edycji', typo);
    // limit linii (sub: maxLines 2)
    const sf = trio.locator('[data-f="sub"]');
    await sf.fill('');
    await sf.click(); await trio.keyboard.type('a'); await trio.keyboard.press('Enter'); await trio.keyboard.type('b'); await trio.keyboard.press('Enter'); await trio.keyboard.type('c');
    ok((await sf.inputValue()).split('\n').length <= 2, 'maxLines blokuje trzecią linię');
    // pasek formatowania: pogrubienie zaznaczenia
    await sf.fill('zaznacz mnie');
    await sf.evaluate((e) => { e.focus(); e.setSelectionRange(8, 12); });
    await trio.click('[data-rtb="sub"] [data-fmt="b"]');
    ok((await sf.inputValue()) === 'zaznacz {b}mnie{/b}', 'przycisk B owija zaznaczenie w {b}…{/b}');
    await trio.click('[data-rtb="sub"] [data-fmt="b"]');
    ok((await sf.inputValue()) === 'zaznacz mnie', 'ponowne B zdejmuje pogrubienie');
    // Word Replace
    await trio.reload(); await sleep(900);
    await trio.fill('#filter', '9200'); await sleep(200);
    await trio.click('#pages .item'); await sleep(400);
    const sf2 = trio.locator('[data-f="sub"]');
    await sf2.fill('');
    await sf2.click(); await trio.keyboard.type('Spotkał się z @p');
    await trio.keyboard.press('Control+Space');
    ok((await sf2.inputValue()) === 'Spotkał się z Prezydent', 'Word Replace: @p + Ctrl+Spacja → Prezydent', await sf2.inputValue());
    await sf2.fill('@m'); await sf2.click(); await sf2.evaluate((e) => e.setSelectionRange(2, 2));
    await trio.keyboard.press('Control+Space'); await sleep(200);
    ok(await trio.evaluate(() => document.querySelector('#wrMenu').style.display === 'block'), 'Word Replace: wiele słów → menu wyboru');
    await trio.keyboard.press('2'); await sleep(200);
    ok((await sf2.inputValue()) === 'Ministra', 'Word Replace: wybór klawiszem 2', await sf2.inputValue());
    // klik w element podglądu przenosi do pola
    await sf2.fill('Podgląd'); await sleep(300);
    const box = await trio.evaluate(() => { const it = [...document.querySelectorAll('#pvStage [data-id="tt"]')][0]; const r = it.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; });
    await trio.mouse.click(box.x, box.y); await sleep(200);
    ok(await trio.evaluate(() => document.activeElement?.dataset?.f === 'title'), 'klik w element podglądu ustawia fokus w jego polu');

    // --- v2.0: powiązania (dock) ---
    const sh4 = await api('/api/show');
    const rectScene = (id, name, layer, extra = {}) => ({ id, name, layer, width: 1920, height: 1080, duration: 0.2, stops: [0.2],
      nodes: [{ id: id + 'n', type: 'rect', name: id, props: { x: 100, y: 800, w: 300, h: 100, color: '#00ff00' } }], ...extra });
    sh4.scenes.push(rectScene('dkA', 'DockA', 'main', { dock: [{ target: 'layer:back', dx: 0, dy: -100, dur: 0.3 }, { target: 'page:9302', dx: 50, dy: 0, dur: 0.3 }] }));
    sh4.scenes.push(rectScene('dkB', 'DockB', 'back'));
    sh4.scenes.push(rectScene('dkC', 'DockC', 'front'));
    sh4.pages.push({ id: 'pdkA', number: 9301, scene: 'dkA', values: {} }, { id: 'pdkB', number: 9302, scene: 'dkB', values: {} }, { id: 'pdkC', number: 9303, scene: 'dkC', values: {} });
    await fetch(B + '/api/show?force=1', { method: 'PUT', headers: { 'X-Token': TOKEN }, body: JSON.stringify(sh4) });
    const dockTf = () => out.evaluate(() => { const e = document.querySelector('.vz-layer[data-layer="main"] .vz-scene'); return e ? new DOMMatrix(getComputedStyle(e).transform).f + ',' + new DOMMatrix(getComputedStyle(e).transform).e : 'brak'; });
    await api('/api/take/9301'); await sleep(900);
    ok((await dockTf()) === '0,0', 'dock: bez grafiki z reguły scena stoi w miejscu', await dockTf());
    await api('/api/take/9303'); await sleep(700); // inna warstwa (front) – nie wyzwala reguły
    ok((await dockTf()) === '0,0', 'dock: obca warstwa nie przesuwa', await dockTf());
    await api('/api/take/9302'); await sleep(900); // back + page:9302 → dy -100, dx 50
    ok((await dockTf()) === '-100,50', 'dock: layer:back i page:9302 sumują przesunięcie (−100, +50)', await dockTf());
    await api('/api/dock/main/off', { method: 'POST' }); await sleep(800);
    ok((await dockTf()) === '0,0', 'dock: nadpisanie operatora (off) zeruje przesunięcie', await dockTf());
    ok((await api('/api/state')).layers.find((l) => l.layer === 'main').dockOff === true, 'dock: stan dockOff w /api/state');
    await api('/api/dock/main/on', { method: 'POST' }); await sleep(800);
    ok((await dockTf()) === '-100,50', 'dock: ponowne włączenie przywraca przesunięcie', await dockTf());
    // odświeżenie wyjścia (snapshot) odtwarza przesunięcie
    const out2 = await page('/output?checker=1'); await sleep(1200);
    ok(await out2.evaluate(() => { const e = document.querySelector('.vz-layer[data-layer="main"] .vz-scene'); return new DOMMatrix(getComputedStyle(e).transform).f === -100; }), 'dock: nowe wyjście (snapshot) ma przesunięcie od razu');
    await out2.close();
    await api('/api/out/back'); await sleep(900);
    ok((await dockTf()) === '0,0', 'dock: po zdjęciu grafiki scena wraca na miejsce', await dockTf());
    await api('/api/clear');

    // --- ściskanie tekstu (squeeze jak w BTB): wskaźnik w Trio, ostrzeżenie po przekroczeniu squeezeMin ---
    await trio.reload(); await sleep(900);
    await trio.fill('#filter', '1102'); await sleep(200);
    await trio.click('#pages .item'); await sleep(500);
    await trio.locator('[data-f="name"]').fill('prof. Konstanty Ildefons Gałczyński-Brzęczyszczykiewicz-Wolański, Politechnika Warszawska'); await sleep(600);
    const sqName = await trio.locator('[data-sq="name"]').innerText();
    ok(/ściśnięcie: \d+%/.test(sqName) && !sqName.includes('nie mieści'), 'Trio: squeeze – wskaźnik ściśnięcia pola', sqName);
    await trio.locator('[data-f="title"]').fill('Bardzo długi opis funkcji gościa programu '.repeat(7)); await sleep(600);
    const sqTitle = await trio.locator('[data-sq="title"]').innerText();
    ok(sqTitle.includes('nie mieści się') && sqTitle.includes('60%'), 'Trio: squeeze – ostrzeżenie poniżej granicy squeezeMin (font bez zmian)', sqTitle);
    ok(await trio.evaluate(() => { const s = document.querySelector('#pvStage [data-id="p_title"] > span'); return s && getComputedStyle(s).fontSize === '32px' && /scaleX\(0\.6\)/.test(s.style.transform); }), 'podgląd: squeeze ściska do 60% bez zmiany fontu');

    // --- v2.0: blokady edycji (dwóch operatorów) ---
    {
      await trio.goto('about:blank'); await sleep(400); // główna strona Trio zwalnia swoje blokady (pagehide)
      const ctxA = await browser.newContext({ viewport: { width: 1600, height: 950 } });
      const ctxB = await browser.newContext({ viewport: { width: 1600, height: 950 } });
      await ctxA.addInitScript(() => localStorage.setItem('vizrs.who', 'Anna'));
      await ctxB.addInitScript(() => localStorage.setItem('vizrs.who', 'Jan'));
      const pa = await ctxA.newPage(), pb = await ctxB.newPage();
      await pa.goto(B + '/trio?token=' + TOKEN); await pb.goto(B + '/trio?token=' + TOKEN);
      await sleep(900);
      for (const p of [pa, pb]) { await p.fill('#filter', '9200'); await sleep(150); }
      await pa.click('#pages .item'); await sleep(500);
      ok((await api('/api/locks')).some((l) => l.who === 'Anna' && l.kind === 'pages'), 'blokada: Anna zajęła stronę 9200');
      await pb.click('#pages .item'); await sleep(700);
      ok(await pb.evaluate(() => document.querySelector('#estatus').innerText.includes('Anna')), 'blokada: Jan widzi, że stronę edytuje Anna');
      ok(await pb.evaluate(() => document.querySelector('#pages').innerText.includes('Anna')), 'blokada: lista stron pokazuje 🔒 Anna');
      pb.once('dialog', (d) => d.dismiss());
      await pb.click('#eSave'); await sleep(300);
      ok((await api('/api/locks')).filter((l) => l.kind === 'pages').length === 1, 'blokada: Jan nie przejął blokady');
      await pa.goto(B + '/trio'); await sleep(900); // Anna wychodzi (pagehide → unlock)
      ok(!(await api('/api/locks')).some((l) => l.id === 'prules'), 'blokada: strona 9200 zwolniona po opuszczeniu jej przez Annę');
      await ctxA.close(); await ctxB.close();
      await trio.goto(B + '/trio'); await sleep(800);
    }

    // --- v1.3: OSC + webhook (osobna instancja bez tokenu) ---
    {
      const http = require('http'); const dgram = require('dgram'); const net = require('net');
      const got = [];
      const hook = http.createServer((rq, rs) => { let b = ''; rq.on('data', (c) => (b += c)); rq.on('end', () => { got.push(JSON.parse(b)); rs.end('ok'); }); });
      await new Promise((r) => hook.listen(0, '127.0.0.1', r));
      const hp = hook.address().port, p2 = PORT + 1, oscPort = PORT + 2, d2 = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-e2e2-'));
      const srv2 = spawn(BIN, ['--port', String(p2), '--tcp', '0', '--osc', String(oscPort), '--webhook', `http://127.0.0.1:${hp}/h`, '--data', d2], { stdio: 'ignore' });
      for (let i = 0; i < 50; i++) { try { await fetch(`http://127.0.0.1:${p2}/api/state`); break; } catch (e) { await sleep(100); } }
      const pad = (str) => { const b = Buffer.from(str + '\0'); return Buffer.concat([b, Buffer.alloc((4 - (b.length % 4)) % 4)]); };
      const i32 = (n) => { const b = Buffer.alloc(4); b.writeInt32BE(n); return b; };
      const sendOsc = (addr, tags, ...args) => new Promise((res) => { const u = dgram.createSocket('udp4'); u.send(Buffer.concat([pad(addr), pad(tags), ...args]), oscPort, '127.0.0.1', () => { u.close(); res(); }); });
      await sendOsc('/vizrs/take', ',i', i32(1001));
      await sleep(500);
      let s2 = await fetch(`http://127.0.0.1:${p2}/api/state`).then((r) => r.json());
      ok(s2.layers.some((l) => l.page.number === 1001), 'OSC /vizrs/take 1001 → strona na antenie');
      ok(got.some((g) => g.event === 'onAir' && g.page && g.page.number === 1001), 'webhook: zdarzenie onAir dostarczone');
      await sendOsc('/vizrs/clear', ',');
      await sleep(500);
      s2 = await fetch(`http://127.0.0.1:${p2}/api/state`).then((r) => r.json());
      ok(s2.layers.length === 0, 'OSC /vizrs/clear → antena czysta');
      ok(got.some((g) => g.event === 'offAir'), 'webhook: zdarzenie offAir dostarczone');
      srv2.kill(); hook.close(); fs.rmSync(d2, { recursive: true, force: true });
    }

    // --- ticker: edytor wiadomości w Trio, UPDATE na antenie bez restartu przewijania, /api/ticker ---
    await trio.reload(); await sleep(900);
    await trio.fill('#filter', '3001'); await sleep(200);
    await trio.click('#pages .item'); await sleep(600);
    ok((await trio.locator('.tkx-row').count()) >= 2, 'Trio: pole tickera jako lista wiadomości', await trio.locator('.tkx-row').count());
    await trio.locator('.tkx-txt').first().click();
    await trio.keyboard.press('End'); await trio.keyboard.press('Enter'); await trio.keyboard.type('E2E nowa wiadomość');
    await trio.locator('.tkx-cat').nth(1).fill('SPORT');
    const tkv = await trio.evaluate(() => document.querySelector('[data-tk]').value);
    ok(tkv.split('\n')[1] === 'SPORT|E2E nowa wiadomość', 'Trio: Enter dodaje wiadomość, kategoria zapisana jako KAT|treść', tkv);
    await trio.keyboard.press('Control+ArrowUp');
    ok((await trio.evaluate(() => document.querySelector('[data-tk]').value)).startsWith('SPORT|E2E nowa wiadomość'), 'Trio: Ctrl+↑ przesuwa wiadomość');
    await trio.keyboard.press('Control+s'); await sleep(600);
    ok(((await api('/api/show')).pages.find((p) => p.number === 3001)?.values.items || '').startsWith('SPORT|E2E nowa wiadomość'), 'Trio: zapis strony z listą wiadomości');
    await trio.click('[data-view="groups"]'); await sleep(200);
    ok((await trio.locator('.tkx-card').count()) >= 2, 'Trio: widok grup po kategoriach');
    await trio.click('[data-view="list"]');
    // ciągłość crawla: zmiana wiadomości na antenie nie zaczyna przewijania od nowa
    await api('/api/clear'); await sleep(300);
    await api('/api/take/3001'); await sleep(2500);
    const tape = () => out.evaluate(() => { const t = document.querySelector('.vz-ticker > div > div > div'); const m = /translateX\((-?[\d.]+)px\)/.exec(t?.style.transform || ''); return { x: m ? +m[1] : null, first: t?.firstElementChild?.textContent || '', t: performance.now() }; });
    const a1 = await tape();
    const add = await api('/api/ticker/3001/add?item=' + encodeURIComponent('E2E|Dodana na antenie'), { method: 'POST' });
    ok(add.onAir === true && add.items.some((x) => x.text === 'Dodana na antenie'), '/api/ticker add: strona na antenie zaktualizowana', add);
    await sleep(700);
    const a2 = await tape();
    const exp = 150 * (a2.t - a1.t) / 1000;
    ok(a1.x !== null && a2.x < a1.x && Math.abs((a1.x - a2.x) - exp) < exp * 0.5 + 40 && a2.first === a1.first, 'UPDATE tickera na antenie: przewijanie płynie dalej (bez skoku)', [a1, a2, exp]);
    ok((await api('/api/ticker/3001')).items.length >= 3, '/api/ticker: lista wiadomości strony');
    await api('/api/clear');

    // --- Trio: okno Stan systemu ---
    await trio.click('#bSys');
    await sleep(600);
    ok(await trio.evaluate(() => document.querySelector('#sysBox').innerText.includes('Stan systemu')), 'Trio: okno Stan systemu otwiera się');
    await trio.keyboard.press('Escape');

    // --- kopie zapasowe ---
    const bk = await api('/api/backups');
    ok(Array.isArray(bk) && bk.length > 0, 'kopie zapasowe show istnieją');

    ok(errs.length === 0, 'brak błędów JS w UI' + (errs.length ? ': ' + errs.join('; ') : ''));
  } catch (e) {
    console.error(e);
    failures++;
  } finally {
    if (browser) await browser.close();
    srv.kill();
    fs.rmSync(data, { recursive: true, force: true });
  }
  console.log(failures ? `\n${failures} NIEUDANYCH` : '\nWszystkie testy E2E zaliczone');
  process.exit(failures ? 1 : 0);
})();
