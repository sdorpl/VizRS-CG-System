// Import elementów → VizRS (F4.2):  node tests/vectors/import.js
// Testy na kopiach elementów z oryginału (tests/fixtures/import).
const fs = require('fs');
const path = require('path');
const { pathToFileURL } = require('url');
let failed = 0, passed = 0;
const check = (name, cond, info) => { if (cond) passed++; else { failed++; console.log('FAIL ' + name + (info !== undefined ? ' → ' + JSON.stringify(info).slice(0, 300) : '')); } };
const FX = path.join(__dirname, '../fixtures/import');

(async () => {
  const io = await import(pathToFileURL(path.join(__dirname, '../../web/element-import.js')).href);

  // --- ease / kolory
  check('mapEase: nazwy CSS', io.mapEase('ease-in') === 'in' && io.mapEase('ease-out') === 'out' && io.mapEase('ease-in-out') === 'inout' && io.mapEase('linear') === 'linear');
  check('mapEase: cubic-bezier (0.2,0.8,0.2,1) → out', io.mapEase('cubic-bezier(0.2, 0.8, 0.2, 1)') === 'out');
  check('mapEase: nadmiar → back', io.mapEase('cubic-bezier(0.3, 1.5, 0.6, 1.6)') === 'back');
  check('toHex: rgb i rgba', io.toHex('rgb(255, 0, 16)') === '#ff0010' && io.toHex('rgba(0,0,0,0.5)') === '#00000080' && io.toHex('#abc') === '#abc');

  // --- import elementów
  const load = (n) => io.readElements(fs.readFileSync(path.join(FX, n), 'utf8'));
  const { elements } = await load('belka_glowna.json');
  check('readElements: eksport v3 → lista elementów', elements.length === 1 && elements[0].scene.layers.length > 0);
  const r = io.convertElement(elements[0], { knownFonts: ['Barlow Condensed'] });
  const g = r.scene.nodes[0];
  check('belka: scena 1920×1080, jedna grupa z dziećmi', r.scene.width === 1920 && g.type === 'group' && g.children.length === elements[0].scene.layers.length);
  check('belka: położenie grupy z layout.x/y', g.props.x === Math.round(elements[0].layout.x * 1000) / 1000, g.props);
  check('belka: gradient kształtu (color2 + kąt)', g.children[0].type === 'rect' && g.children[0].props.color2 && g.children[0].props.gradAngle === 160, g.children[0].props);
  check('belka: kolory rgb() zamienione na hex', /^#/.test(g.children[0].props.color));
  check('belka: animacja IN (zoom) → ruch grupy 1:1 (typ, kierunek, czas, krzywa)', g.motion?.in?.type === 'zoom' && g.motion.in.dir === 'top' && g.motion.in.dur === 0.25 && g.motion.in.ease === 'ease-in' && !g.anim, g.motion);
  check('belka: OUT (zoom) → motion.out, pusta krzywa pominięta', g.motion?.out?.type === 'zoom' && g.motion.out.dur === 0.2 && !('ease' in g.motion.out), g.motion);
  check('belka: stop point na końcu wejścia', r.scene.stops.length === 1 && Math.abs(r.scene.stops[0] - 0.25) < 1e-6, r.scene.stops);
  check('belka: OUT (zoom) po stopie → duration > stop', r.scene.duration > r.scene.stops[0]);
  check('belka: ostrzeżenie o foncie Bahnschrift', r.warnings.some((w) => /Bahnschrift/.test(w)), r.warnings);
  check('belka: tekst z squash → textFit squeeze', g.children.some((c) => c.type === 'text' && c.props.textFit === 'squeeze'));

  // warstwy tekstu bez pola (oryginał: edycja w WYSIWYG) → pola tekst1, tekst2; szerokość do krawędzi tła
  const ft = g.children.filter((c) => c.type === 'text');
  check('belka: tekst bez pola → pola tekst1/tekst2 z wartością przykładu', ft.map((c) => c.fields.text).join() === 'tekst1,tekst2' && r.page.values.tekst1 === 'TEST BELEK' && r.scene.fieldMeta.tekst1?.label === 'Tekst 1', [ft.map((c) => c.fields), r.page.values]);
  check('belka: tekst bez pola rozszerzony do krawędzi tła (1650 − 2×30)', ft.every((c) => c.props.w === 1590), ft.map((c) => c.props.w));
  check('belka: ostrzeżenie o dodanym polu', r.warnings.some((w) => /bez pola w źródle/.test(w)), r.warnings);
  const nz = io.convertElement((await load('na_zywo_lewa.json')).elements[0]);
  const t = nz.scene.nodes[0].children.find((c) => c.type === 'text');
  check('na żywo: pole TITLE → pole kontrolne title', t.fields.text === 'title' && nz.scene.fieldMeta.title && nz.page.values.title === 'na żywo', [t.fields, nz.page.values]);
  const sh = nz.scene.nodes[0].children[0];
  check('na żywo: fitText → autofollow kształtu za tekstem', sh.props.follow === t.id && sh.props.followAxis === 'w' && sh.props.followPad >= 0, sh.props);
  check('na żywo: autoW bez squash → squeeze w maxW, granica 0,5', t.props.textFit === 'squeeze' && t.props.squeezeMin === 0.5 && t.props.w > 122, t.props);
  check('na żywo: zoom IN + fade OUT jako ruch elementu', nz.scene.nodes[0].motion?.in?.type === 'zoom' && nz.scene.nodes[0].motion?.out?.type === 'fade', nz.scene.nodes[0].motion);
  check('na żywo: długość = stop + wyjście', Math.abs(nz.scene.stops[0] - 0.9) < 1e-6 && Math.abs(nz.scene.duration - 1.7) < 1e-6, [nz.scene.stops, nz.scene.duration]);

  // ruch warstw: presety spoza listy, efekty tekstu, pętle, „po warstwie”
  const rich = io.convertElement({ id: 'x1', name: 'Ruch', layout: { x: 0, y: 0 }, scene: { frame: { w: 800, h: 100 }, layers: [
    { id: 'a', type: 'shape', x: 0, y: 0, w: 800, h: 100, fill: { type: 'solid', color: '#000' }, motion: { in: { type: 'mask-diag', direction: 'right', duration: 0.5, ease: 'spring' }, out: { type: 'teleport', fallback: 'clip', duration: 0.3 } } },
    { id: 'b', type: 'text', x: 10, y: 0, w: 700, h: 100, text: 'Ala ma kota', font: { size: 40 }, motion: { in: { type: 'fade', duration: 0.2, after: 'a' }, text: { type: 'blurIn', duration: 0.4, stagger: 0.05, from: 'center' }, textOut: { type: 'words-up', duration: 0.3 } } },
  ] }, animation: { loop: { type: 'pulse', duration: 1, amplitude: 6 } } });
  const ra = rich.scene.nodes[0].children[0], rb = rich.scene.nodes[0].children[1];
  check('ruch: maska ukośna z kierunkiem i krzywą spring', ra.motion.in.type === 'mask-diag' && ra.motion.in.dir === 'right' && ra.motion.in.ease === 'spring', ra.motion);
  check('ruch: nieznany typ → fallback oryginału z ostrzeżeniem', ra.motion.out.type === 'clip' && rich.warnings.some((w) => /teleport/.test(w)), [ra.motion, rich.warnings]);
  check('ruch: efekt tekstu (alias blurIn → blur) z odstępem i kolejnością', rb.motion.textIn.type === 'blur' && rb.motion.textIn.stagger === 0.05 && rb.motion.textIn.from === 'center' && rb.motion.textOut.type === 'words-up', rb.motion);
  check('ruch: „po warstwie” → motion.after', rb.motion.after === 'a', rb.motion);
  check('ruch: pętla elementu z siłą', rich.scene.nodes[0].motion.loop.type === 'pulse' && rich.scene.nodes[0].motion.loop.amp === 6, rich.scene.nodes[0].motion);
  check('ruch: efekt tekstu bez opóźnienia – domyślne 0,2 s jak w cgmotion', rb.motion.textIn.delay === 0.2 && !rb.motion.textOut.delay, rb.motion);
  check('ruch: stop po sekwencji (0,5 + max(0,2; 0,2 + 0,4)) i długość z wyjściem tekstu', Math.abs(rich.scene.stops[0] - 1.1) < 1e-6 && rich.scene.duration > rich.scene.stops[0] + 0.3, [rich.scene.stops, rich.scene.duration]);

  const tk = io.convertElement((await load('ticker_pelny.json')).elements[0]);
  const tn = tk.scene.nodes[0].children.find((c) => c.type === 'ticker');
  check('ticker: warstwa ticker → węzeł ticker z wiadomościami z items', tn && tn.props.text.split('\n').length >= 3 && tn.props.speed === 120, tn && tn.props);

  check('ticker: pole kontrolne items z wiadomościami, tryb crawl (warstwa sceny oryginału)', tn.fields.text === 'items' && tk.page.values.items === tn.props.text && tn.props.tickerMode === 'crawl', [tn.fields, tn.props.tickerMode]);

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

  const rl = io.convertElement((await load('republika_lt.json')).elements[0]);
  check('Republika: konwersja bez wyjątku, scena ma węzły', rl.scene.nodes[0].children.length > 0);

  // --- .cgpack
  const pack = await io.readElements(fs.readFileSync(path.join(FX, 'belka_pilne.cgpack')), 'x.cgpack');
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

  // --- poprawki z przeniesienia wzorców oryginału (examples/archiwum/WZORCE, tests/e2e/import-visual.js)
  check('zegar: HH:MM (minuty w oryginale) → HH:mm', io.mapClockFormat('HH:MM') === 'HH:mm' && io.mapClockFormat('HH:MM:SS') === 'HH:mm:ss' && io.mapClockFormat() === 'HH:mm:ss');
  check('zegar: format rozszerzony DD.MM.YYYY HH:mm → dd.MM.yyyy HH:mm', io.mapClockFormat('DD.MM.YYYY HH:mm') === 'dd.MM.yyyy HH:mm');
  const zg = io.convertElement((await load('zegar.json')).elements[0]);
  const ck = zg.scene.nodes[0].children.find((c) => c.type === 'clock');
  check('zegar: węzeł clock z formatem HH:mm', ck && ck.props.format === 'HH:mm', ck && ck.props);
  const ml = io.convertElement((await load('miasto_lewa.json')).elements[0]);
  check('BADGE z domyślnymi items oryginał: bez pola ticker i bez ostrzeżenia o tickerze', !('items' in ml.page.values) && !ml.warnings.some((w) => /TICKER/.test(w)), [Object.keys(ml.page.values), ml.warnings]);
  const bd = io.convertElement((await load('badge_dlugi_tekst.json')).elements[0]);
  const bt = bd.scene.nodes[0].children.find((c) => c.type === 'text');
  check('BADGE: tekst szerszy niż tło → ściskanie do wnętrza tła (620 − 2×16), granica 0,5', bt.props.w === 588 && bt.props.textFit === 'squeeze' && bt.props.squeezeMin === 0.5, bt.props);
  const sq = g.children.find((c) => c.props.textFit === 'squeeze'), srcSq = elements[0].scene.layers.find((l) => l.id === sq.id);
  check('squash: granica ściśnięcia z oryginału (0 i brak → bez granicy 0,3)', sq.props.squeezeMin === (srcSq.minSquash > 0 ? Math.max(0.3, srcSq.minSquash) : 0.3), [sq.props.squeezeMin, srcSq.minSquash]);
  const sq8 = io.convertElement({ ...elements[0], scene: { ...elements[0].scene, layers: elements[0].scene.layers.map((l) => (l.id === sq.id ? { ...l, minSquash: 0.8 } : l)) } });
  check('squash: jawna granica 0,8 przenoszona', sq8.scene.nodes[0].children.find((c) => c.id === sq.id).props.squeezeMin === 0.8);
  const sqNo = io.convertElement({ ...elements[0], scene: { ...elements[0].scene, layers: elements[0].scene.layers.map((l) => (l.id === sq.id ? { ...l, minSquash: undefined } : l)) } });
  check('squash: brak granicy (element sprzed M7) → 0,3', sqNo.scene.nodes[0].children.find((c) => c.id === sq.id).props.squeezeMin === 0.3);
  const gt = io.convertElement((await load('grafika_theme.json')).elements[0]);
  const im = gt.scene.nodes[0].children.find((c) => c.type === 'image');
  check('obraz z adresem https zostaje pełnym adresem (+ ostrzeżenie o sieci)', /^https:\/\//.test(im.props.src) && gt.warnings.some((w) => /z sieci/.test(w)), [im.props.src, gt.warnings]);
  check('warstwa ticker oryginał: treści bez kategorii, crawl od lewej', !tn.props.text.includes('|') && tn.props.crawlFrom === 'left', tn.props);
  const wl = io.convertElement((await load('ticker_unified.json')).elements[0]).scene.nodes[0].children.find((c) => c.type === 'ticker');
  check('wiper → rozmiar i grubość etykiety z tekstu wipera', wl.props.labelSize > 0 && (!wl.props.labelWeight || wl.props.labelWeight !== wl.props.weight), wl.props);

  // --- błędne wejście
  let bad = '';
  try { await io.readElements('{"foo":1}'); } catch (e) { bad = e.message; }
  check('readElements: obcy JSON → czytelny błąd', /nie jest plik/.test(bad), bad);
  bad = '';
  try { io.convertElement({ name: 'X', html_template: '<div></div>' }); } catch (e) { bad = e.message; }
  check('convertElement: element z ręcznym kodem → komunikat „Przekształć w scenę”', /Przekształć w scenę/.test(bad), bad);

  console.log(failed ? `\n${failed} NIEUDANYCH (zaliczone: ${passed})` : `Import elementów zgodny (${passed} sprawdzeń)`);
  process.exit(failed ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(1); });
