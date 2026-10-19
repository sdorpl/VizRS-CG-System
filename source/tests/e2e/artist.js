// E2E edytora Artist (oś czasu: zoom, markery, zaznaczanie ramką, kopiuj/wklej klucze; drzewo: przeciąganie; presety animacji;
// reguły pól i dock w ustawieniach sceny):  node tests/e2e/artist.js [ścieżka-do-vizrs]
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
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-artist-'));
  const srv = spawn(BIN, ['--port', String(PORT), '--tcp', '0', '--data', data], { stdio: 'ignore' });
  let browser;
  try {
    for (let i = 0; i < 50; i++) { try { await fetch(B + '/api/state'); break; } catch (e) { await sleep(100); } }
    const scene = {
      id: 'at', name: 'ArtistTest', layer: 'main', width: 1920, height: 1080, duration: 1.2, stops: [], markers: [],
      fieldMeta: {}, nodes: [
        { id: 'g1', type: 'group', name: 'grupa', props: { x: 300, y: 200, w: 600, h: 300 }, children: [] },
        { id: 'r1', type: 'rect', name: 'prostokat', props: { x: 100, y: 100, w: 300, h: 80, color: '#ff0000' },
          anim: { x: [{ t: 0, v: 0, e: 'out' }, { t: 0.4, v: 100 }, { t: 0.8, v: 200 }], opacity: [{ t: 0, v: 0 }, { t: 0.4, v: 1 }] } },
        { id: 't1', type: 'text', name: 'napis', fields: { text: 'tytul' }, props: { x: 50, y: 400, w: 800, h: 80, text: 'Tytuł', size: 40, color: '#ffffff' } },
      ],
    };
    await api('/api/scenes', { method: 'POST', body: JSON.stringify(scene) });
    browser = await chromium.launch();
    const ctx = await browser.newContext({ viewport: { width: 1700, height: 1000 } });
    const pg = await ctx.newPage();
    const errs = [];
    pg.on('pageerror', (e) => errs.push(e.message));
    let promptAnswer = 'x';
    pg.on('dialog', (d) => d.accept(promptAnswer));
    await pg.goto(B + '/artist?scene=at');
    await sleep(1200);
    const nKf = () => pg.locator('#tl .kf').count();
    const saved = () => api('/api/scenes/at').catch(() => null).then(async () => (await api('/api/show')).scenes.find((x) => x.id === 'at'));

    // zoom: Ctrl+kółko i „dopasuj”
    const z0 = +(await pg.inputValue('#zoom'));
    await pg.mouse.move(600, 880);
    await pg.keyboard.down('Control'); await pg.mouse.wheel(0, -300); await pg.keyboard.up('Control');
    await sleep(200);
    const z1 = +(await pg.inputValue('#zoom'));
    ok(z1 > z0, 'Ctrl+kółko przybliża oś czasu', [z0, z1]);
    await pg.click('#zoomFit'); await sleep(200);
    const zf = +(await pg.inputValue('#zoom'));
    ok(zf > 40 && zf < 1200 && zf !== z1, '„Dopasuj” ustawia zoom do szerokości osi', zf);

    // markery: klawisz M, dwuklik na linijce, przekształcenie w stop
    await pg.click('#tl .tl-lbl.ruler, #tl .tl-row.ruler .tl-lbl'); // fokus obszaru osi
    promptAnswer = 'wejscie';
    await pg.keyboard.press('m'); await sleep(300);
    ok((await pg.locator('#tl .mkm').count()) === 1, 'klawisz M dodaje marker na osi');
    promptAnswer = 'drugi';
    const ruler = await pg.locator('#tl .tl-area[data-area="ruler"]').boundingBox();
    await pg.mouse.dblclick(ruler.x + 0.7 * zf, ruler.y + 8); await sleep(300);
    ok((await pg.locator('#tl .mkm').count()) === 2, 'dwuklik na linijce dodaje marker');
    await pg.click('#tl .stopm, #tl .tl-area[data-area="ruler"]', { position: { x: 5, y: 5 }, force: true }).catch(() => {});

    // zaznaczanie kluczy: kliknięcie + Shift, kopiuj / wklej
    await pg.click('#tl .tl-lbl[data-sel="r1"]'); await sleep(200);
    const kfs = pg.locator('#tl .tl-area[data-row="r1|x"] .kf');
    const before = await nKf();
    await kfs.nth(0).click(); await kfs.nth(1).click({ modifiers: ['Shift'] }); await sleep(200);
    ok((await pg.locator('#tl .kf.sel').count()) === 2, 'Shift+klik zaznacza dwa klucze', await pg.locator('#tl .kf.sel').count());
    await pg.keyboard.press('Control+c'); await sleep(150);
    // przesuń głowicę (klik w linijkę) i wklej na zaznaczony węzeł
    await pg.mouse.click(ruler.x + 1.0 * zf, ruler.y + 12); await sleep(200);
    await pg.click('#tl .tl-lbl[data-sel="r1"]'); await sleep(150);
    await pg.keyboard.press('Control+v'); await sleep(300);
    ok((await nKf()) > before, 'Ctrl+V wkleja skopiowane klucze w miejscu głowicy', [before, await nKf()]);
    // zaznaczanie ramką (Shift + przeciąganie po pustym polu ścieżki)
    const row = await pg.locator('#tl .tl-area[data-row="r1|opacity"]').boundingBox();
    await pg.mouse.move(row.x + 0.9 * zf, row.y + 2);
    await pg.keyboard.down('Shift');
    await pg.mouse.down(); await pg.mouse.move(row.x - 4, row.y + 30, { steps: 6 }); await pg.mouse.up();
    await pg.keyboard.up('Shift'); await sleep(250);
    ok((await pg.locator('#tl .kf.sel').count()) >= 2, 'Shift + przeciągnięcie ramką zaznacza klucze', await pg.locator('#tl .kf.sel').count());

    // przeciąganie w drzewie: prostokąt do grupy (zmiana rodzica, położenie na scenie zachowane)
    await pg.locator('#tree .item[data-id="r1"]').dragTo(pg.locator('#tree .item[data-id="g1"]'));
    await sleep(300);
    await pg.click('#bSave'); await sleep(600);
    let sc = await saved();
    const grp = sc.nodes.find((n) => n.id === 'g1');
    const moved = grp && (grp.children || []).find((n) => n.id === 'r1');
    ok(!!moved, 'przeciągnięcie w drzewie przenosi węzeł do grupy (zmiana rodzica)');
    ok(moved && moved.props.x === -200 && moved.props.y === -100, 'położenie na scenie zachowane (x 100−300, y 100−200)', moved && moved.props);
    ok((sc.markers || []).length === 2 && sc.markers.some((m) => m.name === 'wejscie'), 'markery zapisane w scenie', sc.markers);

    // presety animacji
    await pg.click('#tree .item[data-id="r1"]'); await sleep(200);
    promptAnswer = 'Wjazd testowy';
    await pg.click('[data-ap="save"]'); await sleep(500);
    const presets = await api('/api/animpresets');
    ok(presets.length === 1 && presets[0].name === 'Wjazd testowy' && presets[0].tracks.x && presets[0].tracks.x[0].t === 0, 'preset animacji zapisany w bibliotece (czasy względne)', presets);
    await pg.selectOption('#apSel', { index: 1 }); await sleep(100);
    const kBefore = await nKf();
    await pg.click('#tl .tl-row.ruler .tl-area', { position: { x: 0.9 * zf, y: 8 } }); await sleep(150);
    await pg.click('[data-ap="apply"]'); await sleep(400);
    ok((await nKf()) !== kBefore, 'zastosowanie presetu wstawia klucze od głowicy', [kBefore, await nKf()]);

    // ustawienia sceny: reguły pola + dock
    await pg.mouse.click(1400, 400); // puste miejsce na podglądzie? użyj wiersza sceny w drzewie
    await pg.click('#tree .item[data-id=""]'); await sleep(300);
    await pg.click('[data-frtoggle="tytul"]'); await sleep(200);
    await pg.fill('[data-fr="tytul::maxChars"]', '15');
    await pg.check('[data-fr="tytul::upper"]');
    await pg.fill('[data-fr="tytul::help"]', 'wpisz krótko');
    await pg.click('#dkAdd'); await sleep(200);
    await pg.fill('[data-dk="0::target"]', 'page:3001');
    await pg.click('#bSave'); await sleep(600);
    sc = await saved();
    ok(sc.fieldMeta.tytul && sc.fieldMeta.tytul.maxChars === 15 && sc.fieldMeta.tytul.upper === true && sc.fieldMeta.tytul.help === 'wpisz krótko', 'reguły pola zapisane w fieldMeta', sc.fieldMeta);
    ok(Array.isArray(sc.dock) && sc.dock[0].target === 'page:3001', 'powiązanie dock zapisane w scenie', sc.dock);
    // import / eksport BTB CG
    const FXD = path.join(__dirname, '../fixtures/btb');
    const importBtb = async (file) => {
      await pg.keyboard.press('Escape');
      const [fc] = await Promise.all([pg.waitForEvent('filechooser'), pg.click('#bImpBtb')]);
      await fc.setFiles(path.join(FXD, file)); await sleep(1500);
    };
    const nBefore = (await api('/api/show')).scenes.length;
    await importBtb('na_zywo_lewa.json');
    ok(await pg.locator('#rep').count() === 1, 'import BTB: okno raportu konwersji');
    await pg.click('#repOk');
    let sh = await api('/api/show');
    const imported = sh.scenes.find((x) => x.name === 'NA ŻYWO lewa' || /ŻYWO/i.test(x.name));
    ok(sh.scenes.length === nBefore + 1 && !!imported, 'import BTB (.json): scena zapisana na serwerze (walidacja przeszła)', sh.scenes.map((x) => x.name));
    ok(imported && imported.nodes[0].type === 'group' && sh.pages.some((p) => p.scene === imported.id && p.values.title === 'na żywo'), 'import BTB: strona z wartością pola title');
    await importBtb('belka_pilne.cgpack');
    await pg.click('#repOk').catch(() => {});
    sh = await api('/api/show');
    ok(sh.scenes.length === nBefore + 2, 'import BTB (.cgpack): ZIP rozpakowany, scena dodana', sh.scenes.length);
    const [dl] = await Promise.all([pg.waitForEvent('download'), pg.click('#bExpBtb')]);
    const dlPath = await dl.path();
    const exported = JSON.parse(fs.readFileSync(dlPath, 'utf8'));
    ok(exported._exportVersion === 3 && exported.elements[0].scene.layers.length > 0, 'eksport BTB: plik v3 z warstwami', Object.keys(exported));
    await pg.click('#repOk').catch(() => {});

    // presety tickerów i panel węzła ticker (tylko właściwości bieżącego trybu, edytor wiadomości próbnych)
    await pg.goto(B + '/artist?scene=at'); await sleep(1000);
    await pg.selectOption('#addSel', 'preset:whip'); await sleep(400);
    ok((await pg.locator('#props .tkx').count()) === 1, 'preset tickera: węzeł zaznaczony, edytor wiadomości w panelu');
    ok((await pg.locator('#props [data-prop="speed"]').count()) === 0 && (await pg.locator('#props [data-prop="hold"]').count()) === 1, 'ticker whip: panel bez prędkości crawla, z czasem wiadomości');
    await pg.selectOption('[data-p="tickerMode"]', 'crawl'); await sleep(300);
    ok((await pg.locator('#props [data-prop="speed"]').count()) === 1 && (await pg.locator('#props [data-prop="hold"]').count()) === 0, 'ticker crawl: panel z prędkością, bez czasu wiadomości');
    await pg.locator('#props .tkx-txt').first().fill('Próbna z Artist');
    await pg.keyboard.press('Control+s'); await sleep(600);
    const sc2 = await saved();
    const tkn = sc2.nodes.find((x) => x.type === 'group' && x.children?.some((c) => c.type === 'ticker'))?.children.find((c) => c.type === 'ticker');
    ok(tkn && tkn.fields.text === 'news' && tkn.props.text.split('\n')[0] === 'Próbna z Artist' && sc2.fieldMeta.news?.label === 'Wiadomości', 'preset tickera zapisany: pole news, wiadomości próbne z edytora', tkn && [tkn.fields, tkn.props.text]);

    ok(errs.length === 0, 'brak błędów JS w Artist' + (errs.length ? ': ' + errs.join('; ') : ''));
  } catch (e) { console.error(e); failures++; } finally {
    if (browser) await browser.close();
    srv.kill(); fs.rmSync(data, { recursive: true, force: true });
  }
  console.log(failures ? `\n${failures} NIEUDANYCH` : '\nWszystkie testy Artist zaliczone');
  process.exit(failures ? 1 : 0);
})();
