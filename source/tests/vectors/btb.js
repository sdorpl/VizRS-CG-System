// Konwersja BTB ↔ VizRS (F4.2):  node tests/vectors/btb.js
// Testy na kopiach elementów z BTB CG Editor (tests/fixtures/btb). Jeśli obok leży repo BTB-CG-System-alpha (BTB_CG_DIR),
// dodatkowo sprawdza, że eksport VizRS → BTB przechodzi przez `EditorIO.prepare/validate` systemu BTB.
const fs = require('fs');
const path = require('path');
const os = require('os');
let failed = 0, passed = 0;
const check = (name, cond, info) => { if (cond) passed++; else { failed++; console.log('FAIL ' + name + (info !== undefined ? ' → ' + JSON.stringify(info).slice(0, 300) : '')); } };
const FX = path.join(__dirname, '../fixtures/btb');

(async () => {
  const io = await import(path.join(__dirname, '../../web/btb-io.js'));

  // --- ease / kolory
  check('mapEase: nazwy CSS', io.mapEase('ease-in') === 'in' && io.mapEase('ease-out') === 'out' && io.mapEase('ease-in-out') === 'inout' && io.mapEase('linear') === 'linear');
  check('mapEase: cubic-bezier (0.2,0.8,0.2,1) → out', io.mapEase('cubic-bezier(0.2, 0.8, 0.2, 1)') === 'out');
  check('mapEase: nadmiar → back', io.mapEase('cubic-bezier(0.3, 1.5, 0.6, 1.6)') === 'back');
  check('toHex: rgb i rgba', io.toHex('rgb(255, 0, 16)') === '#ff0010' && io.toHex('rgba(0,0,0,0.5)') === '#00000080' && io.toHex('#abc') === '#abc');

  // --- import elementów
  const load = (n) => io.readBtb(fs.readFileSync(path.join(FX, n), 'utf8'));
  const { elements } = await load('belka_glowna.json');
  check('readBtb: eksport v3 → lista elementów', elements.length === 1 && elements[0].scene.layers.length > 0);
  const r = io.convertElement(elements[0], { knownFonts: ['Barlow Condensed'] });
  const g = r.scene.nodes[0];
  check('belka: scena 1920×1080, jedna grupa z dziećmi', r.scene.width === 1920 && g.type === 'group' && g.children.length === elements[0].scene.layers.length);
  check('belka: położenie grupy z layout.x/y', g.props.x === Math.round(elements[0].layout.x * 1000) / 1000, g.props);
  check('belka: gradient kształtu (color2 + kąt)', g.children[0].type === 'rect' && g.children[0].props.color2 && g.children[0].props.gradAngle === 160, g.children[0].props);
  check('belka: kolory rgb() zamienione na hex', /^#/.test(g.children[0].props.color));
  check('belka: animacja IN (zoom) → klucze scale i opacity na grupie', g.anim.scale && g.anim.opacity && g.anim.scale[0].v === 0.85, g.anim);
  check('belka: stop point na końcu wejścia', r.scene.stops.length === 1 && Math.abs(r.scene.stops[0] - 0.25) < 1e-6, r.scene.stops);
  check('belka: OUT (zoom) po stopie → duration > stop', r.scene.duration > r.scene.stops[0]);
  check('belka: ostrzeżenie o foncie Bahnschrift', r.warnings.some((w) => /Bahnschrift/.test(w)), r.warnings);
  check('belka: tekst z squash → textFit squeeze', g.children.some((c) => c.type === 'text' && c.props.textFit === 'squeeze'));

  const nz = io.convertElement((await load('na_zywo_lewa.json')).elements[0]);
  const t = nz.scene.nodes[0].children.find((c) => c.type === 'text');
  check('na żywo: pole TITLE → pole kontrolne title', t.fields.text === 'title' && nz.scene.fieldMeta.title && nz.page.values.title === 'na żywo', [t.fields, nz.page.values]);
  const sh = nz.scene.nodes[0].children[0];
  check('na żywo: fitText → autofollow kształtu za tekstem', sh.props.follow === t.id && sh.props.followAxis === 'w' && sh.props.followPad >= 0, sh.props);
  check('na żywo: slide/zoom IN + fade OUT zamienione na klucze', Object.keys(nz.scene.nodes[0].anim).length >= 2);

  const tk = io.convertElement((await load('ticker_pelny.json')).elements[0]);
  const tn = tk.scene.nodes[0].children.find((c) => c.type === 'ticker');
  check('ticker: warstwa ticker → węzeł ticker z wiadomościami z items', tn && tn.props.text.split('\n').length >= 3 && tn.props.speed === 120, tn && tn.props);

  check('ticker: pole kontrolne items z wiadomościami, tryb crawl (warstwa sceny BTB)', tn.fields.text === 'items' && tk.page.values.items === tn.props.text && tn.props.tickerMode === 'crawl', [tn.fields, tn.props.tickerMode]);

  // szablon kodowy przekształcony w scenę (ticker-unified): warstwa ticker zginęła – odtwarzamy z items
  const tu = io.convertElement((await load('ticker_unified.json')).elements[0]);
  const tun = tu.scene.nodes[0].children;
  const tuk = tun.find((c) => c.type === 'ticker');
  check('ticker-unified: węzeł ticker zamiast statycznego tekstu wiadomości', !!tuk && !tun.some((c) => c.type === 'text' && /PRZYKŁADOWA/.test(c.props.text || '')), tun.map((c) => [c.type, c.name]));
  check('ticker-unified: kategorie przeniesione (KAT|treść)', tuk && tuk.props.text === 'PILNE|PRZYKŁADOWA WIADOMOŚĆ NR 1\nINFO|PRZYKŁADOWA WIADOMOŚĆ NR 2\nPRZYKŁADOWA WIADOMOŚĆ NR 3', tuk && tuk.props.text);
  check('ticker-unified: tryb vertical → roll, speed (s) → hold', tuk && tuk.props.tickerMode === 'roll' && tuk.props.hold === 5, tuk && tuk.props);
  check('ticker-unified: wiper → pas kategorii (szerokość, kolor), etykieta z introText', tuk && tuk.props.catW === 165 && tuk.props.catBg === '#901414' && tuk.props.label === 'PILNE', tuk && tuk.props);
  check('ticker-unified: kształt i tekst wipera usunięte, etykieta jako pole kontrolne', !tun.some((c) => /wiper/i.test(c.name)) && tuk.fields.label === 'introtext', [tun.map((c) => c.name), tuk.fields]);
  check('ticker-unified: ticker od wipera do prawej krawędzi tła', tuk && tuk.props.x === 0.2 && Math.abs(tuk.props.x + tuk.props.w - 1650) < 0.01, tuk && [tuk.props.x, tuk.props.w]);
  check('ticker-unified: ostrzeżenie o odtworzeniu z kodu', tu.warnings.some((w) => /odtworzony/.test(w)), tu.warnings);

  const tw = io.convertElement((await load('ticker_wiper_pilny.json')).elements[0]);
  const twk = tw.scene.nodes[0].children.find((c) => c.type === 'ticker');
  check('pilny wiper: wiadomości-stringi, tryb whip, błysk etykiety, wiper bez ::before', twk && twk.props.tickerMode === 'whip' && twk.props.text.split('\n').length === 3 && twk.props.gleam > 0 && !tw.scene.nodes[0].children.some((c) => /::before/.test(c.name)), twk && twk.props);
  check('pilny wiper: kolor pasa z gradientu wipera', twk && twk.props.catBg === '#ff0055', twk && twk.props.catBg);

  // eksport tickera: lista z kategoriami, tryb, etykieta, kolory kategorii
  const tex = io.exportScene({ id: 't1', name: 'T', width: 1920, height: 1080, stops: [], fieldMeta: {}, nodes: [
    { id: 'tk', type: 'ticker', name: 'tk', fields: { text: 'news' }, props: { x: 0, y: 1000, w: 1920, h: 60, text: 'x', tickerMode: 'roll', hold: 4, label: 'INFO', catBg: '#c00000', catColors: 'PILNE=#e00000', sepStyle: 'skew' } }] }, { news: 'PILNE|Pierwsza\nDruga\nSPORT#1f8a4c|Trzecia' });
  const te = tex.file.elements[0];
  check('eksport tickera: typ TICKER, items z category', te.type === 'TICKER' && JSON.stringify(te.items) === JSON.stringify([{ text: 'Pierwsza', category: 'PILNE' }, { text: 'Druga' }, { text: 'Trzecia', category: 'SPORT' }]), te.items);
  check('eksport tickera: roll → vertical (speed = hold), etykieta, separator, kolory kategorii', te.tickerMode === 'vertical' && te.speed === 4 && te.introText === 'INFO' && te.separatorStyle === 'skewed' && te.fields.CATEGORY_COLORS === 'PILNE=#e00000' && te.wiper.bgColor === '#c00000', te);
  check('eksport tickera: ostrzeżenia o trybie i kolorach w wiadomościach', tex.warnings.some((w) => /przewija tylko poziomo/.test(w)) && tex.warnings.some((w) => /KAT#kolor/.test(w)), tex.warnings);
  const back = io.convertElement(te);
  const bk = back.scene.nodes[0].children.find((c) => c.type === 'ticker');
  check('round-trip tickera VizRS → BTB → VizRS: wiadomości z kategoriami', bk && bk.props.text === 'PILNE|Pierwsza\nDruga\nSPORT|Trzecia', bk && bk.props.text);

  const rl = io.convertElement((await load('republika_lt.json')).elements[0]);
  check('Republika: konwersja bez wyjątku, scena ma węzły', rl.scene.nodes[0].children.length > 0);

  // --- .cgpack
  const pack = await io.readBtb(fs.readFileSync(path.join(FX, 'belka_pilne.cgpack')), 'x.cgpack');
  check('cgpack: ZIP rozpakowany, element wczytany', pack.elements.length === 1 && !!pack.files['manifest.json'] && !!pack.files['element.json']);
  const pr = io.convertElement(pack.elements[0]);
  check('cgpack: element z paczki konwertuje się', pr.scene.nodes[0].children.length > 0);
  let err = '';
  try { await io.unzip(new Uint8Array([1, 2, 3, 4])); } catch (e) { err = e.message; }
  check('unzip: odrzuca plik, który nie jest ZIP-em', /ZIP/.test(err), err);
  // zip slip: zmodyfikuj nazwę pliku w katalogu centralnym na ../x.json
  const zb = Buffer.from(fs.readFileSync(path.join(FX, 'belka_pilne.cgpack')));
  const cd = zb.indexOf(Buffer.from([0x50, 0x4b, 0x01, 0x02]));
  Buffer.from('..').copy(zb, cd + 46); // nazwa pierwszego wpisu zaczyna się od „..”
  let slip = '';
  try { await io.unzip(new Uint8Array(zb)); } catch (e) { slip = e.message; }
  check('unzip: zip-slip („..” w nazwie) odrzucony', /niedozwolona|uszkodzony/.test(slip), slip);

  // --- błędne wejście
  let bad = '';
  try { await io.readBtb('{"foo":1}'); } catch (e) { bad = e.message; }
  check('readBtb: obcy JSON → czytelny błąd', /nie jest plik/.test(bad), bad);
  bad = '';
  try { io.convertElement({ name: 'X', html_template: '<div></div>' }); } catch (e) { bad = e.message; }
  check('convertElement: element z ręcznym kodem → komunikat „Przekształć w scenę”', /Przekształć w scenę/.test(bad), bad);

  // --- eksport VizRS → BTB
  const out = io.exportScene({ id: 's1', name: 'Test', width: 1920, height: 1080, stops: [0.5], fieldMeta: { title: { label: 'Tytuł', maxChars: 40, upper: true, typo: true, tab: 1 } }, nodes: [
    { id: 'g', type: 'group', name: 'g', props: { x: 100, y: 800, w: 600, h: 100 }, children: [
      { id: 'bg', type: 'rect', name: 'tlo', props: { x: 0, y: 0, w: 600, h: 100, color: '#112233', color2: '#334455', gradAngle: 90, radius: 8 } },
      { id: 'tt', type: 'text', name: 'napis', fields: { text: 'title' }, props: { x: 20, y: 10, w: 560, h: 80, text: 'Tytuł {b}x{/b}', size: 50, font: 'Oswald', weight: '700', color: '#ffffff', textFit: 'squeeze' } } ] } ] }, { title: 'Ala {b}ma{/b} kota' });
  const e0 = out.file.elements[0];
  check('eksport: format v3 z jedną warstwą kształtu i tekstu', out.file._exportVersion === 3 && e0.scene.layers.length === 2 && e0.scene.layers[0].type === 'shape' && e0.scene.layers[1].type === 'text');
  check('eksport: współrzędne warstw względem ramki, ramka w layout', e0.layout.x === 100 && e0.layout.y === 800 && e0.scene.layers[0].x === 0 && e0.scene.layers[1].x === 20, [e0.layout, e0.scene.layers.map((l) => [l.x, l.y])]);
  check('eksport: pole title → ocgInputs TITLE z regułami (maxChars, wersaliki, typografia, Tab)', e0.ocgInputs[0].id === 'TITLE' && e0.ocgInputs[0].text.maxChars === 40 && e0.ocgInputs[0].text.upperCase && e0.ocgInputs[0].text.typography.pl && e0.ocgInputs[0].tab === 1, e0.ocgInputs);
  check('eksport: wartość pola bez znaczników formatowania', e0.ocgInputs[0].default === 'Ala ma kota', e0.ocgInputs[0].default);
  check('eksport: ostrzeżenie o formatowaniu tekstu', out.warnings.some((w) => /formatowanie/.test(w)), out.warnings);

  // --- BTB: prepare/validate (opcjonalnie)
  const BTB = process.env.BTB_CG_DIR || path.join(os.homedir(), 'Nextcloud/GitHub/BTB-CG-System-alpha');
  if (fs.existsSync(path.join(BTB, 'editor-io.js'))) {
    try {
      global.SceneModel = require(path.join(BTB, 'scene-model.js'));
      const EditorIO = require(path.join(BTB, 'editor-io.js'));
      const prepared = EditorIO.prepare(e0);
      const v = EditorIO.validate(prepared);
      const errs = (Array.isArray(v) ? v : v.errors || []).filter((x) => (x.level || x.severity) === 'error');
      check('BTB EditorIO.prepare skompilował kod z warstw (html/css niepuste)', !!prepared.html_template && prepared.html_template.length > 50 && !!prepared.css_template, Object.keys(prepared));
      check('BTB EditorIO.validate: brak błędów dla wyeksportowanego elementu', errs.length === 0, v);
      // i import odwrotny przez nasz konwerter: element BTB po prepare → scena
      const back = io.convertElement(prepared);
      const tp = EditorIO.prepare(te);
      const tv = EditorIO.validate(tp);
      const terrs = (Array.isArray(tv) ? tv : tv.errors || []).filter((x) => (x.level || x.severity) === 'error');
      check('BTB EditorIO: wyeksportowany ticker kompiluje się i przechodzi walidację', !!tp.html_template && /ITEMS_B64/.test(tp.html_template) && terrs.length === 0, terrs);
      check('round-trip VizRS → BTB → VizRS zachowuje pole i tekst', back.scene.nodes[0].children.some((c) => c.fields.text === 'title'), back.scene.nodes[0].children.map((c) => c.fields));
    } catch (e) { check('BTB EditorIO dostępny', false, String(e.stack || e).slice(0, 400)); }
  } else console.log('(pominięto test z kodem BTB – brak ' + BTB + ')');

  console.log(failed ? `\n${failed} NIEUDANYCH (zaliczone: ${passed})` : `Konwersja BTB zgodna (${passed} sprawdzeń)`);
  process.exit(failed ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(1); });
