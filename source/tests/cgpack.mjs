// Testy reguł zasobów paczki (web/cgpack.js – nazwy, czcionki bez duplikatów, brak nadpisywania):  node tests/cgpack.mjs
const P = await import(new URL('../web/cgpack.js', import.meta.url).href);

let failures = 0, checks = 0;
const ok = (cond, msg, info) => { checks++; if (!cond) { failures++; console.log('FAIL ' + msg + (info !== undefined ? ' → ' + JSON.stringify(info) : '')); } };
const b = (s) => new TextEncoder().encode(s);

/** Udawany katalog assets z regułą nazw serwera; liczy wgrania. */
function store(files = {}) {
  const fsys = new Map(Object.entries(files));
  const io = { uploads: [], fetches: 0 };
  io.existing = () => [...fsys.keys()];
  io.fetchAsset = async (n) => { io.fetches++; if (!fsys.has(n)) throw new Error('404'); return fsys.get(n); };
  io.upload = async (n, d) => { const s = P.safeAssetName(n); fsys.set(s, d); io.uploads.push(s); return { name: s }; };
  io.fsys = fsys;
  return io;
}
const placer = (io) => P.assetPlacer({ existing: io.existing(), fetchAsset: io.fetchAsset, upload: io.upload });

// nazwy jak na serwerze
ok(P.safeAssetName('Moja Czcionka Ł.ttf') === 'Moja_Czcionka__.ttf', 'safeAssetName: spacje i znaki spoza ASCII → _', P.safeAssetName('Moja Czcionka Ł.ttf'));
ok(P.safeAssetName('..ukryty.otf') === 'ukryty.otf', 'safeAssetName: bez kropek na początku');
ok(P.safeAssetName('') === null && P.safeAssetName('x'.repeat(121)) === null, 'safeAssetName: pusta i za długa → null');

{ // czcionka o tej samej treści pod inną nazwą → istniejąca, bez wgrywania
  const io = store({ 'Oswald-Bold.ttf': b('FONT-A') });
  const r = await placer(io)('oswald bold (kopia).ttf', b('FONT-A'));
  ok(r.status === 'reused' && r.name === 'Oswald-Bold.ttf' && io.uploads.length === 0, 'Czcionka już wgrana pod inną nazwą: użyta istniejąca', r);
}
{ // ta sama nazwa i treść → pominięta
  const io = store({ 'Bahn.otf': b('F1') });
  const r = await placer(io)('Bahn.otf', b('F1'));
  ok(r.status === 'same' && r.name === 'Bahn.otf' && !io.uploads.length, 'Ta sama czcionka pod tą samą nazwą: pominięta', r);
}
{ // ta sama nazwa, inna treść → nowa nazwa, stara zostaje
  const io = store({ 'Bahn.otf': b('F1') });
  const r = await placer(io)('Bahn.otf', b('F2'));
  ok(r.status === 'renamed' && r.name === 'Bahn_2.otf' && new TextDecoder().decode(io.fsys.get('Bahn.otf')) === 'F1', 'Inna czcionka pod zajętą nazwą: Bahn_2.otf, nic nie nadpisane', r);
}
{ // wielkość liter: Windows/macOS nie rozróżniają
  const io = store({ 'Logo.PNG': b('IMG1') });
  const r = await placer(io)('logo.png', b('IMG2'));
  ok(r.name === 'logo_2.png', 'Nazwa różna tylko wielkością liter traktowana jako zajęta', r);
  const r2 = await placer(io)('logo.png', b('IMG1'));
  ok(r2.status === 'same' && r2.name === 'Logo.PNG', 'Ta sama grafika pod nazwą różną wielkością liter: pominięta', r2);
}
{ // ponowny import tej samej (innej) grafiki: wcześniejsza kopia logo_2.png, nie logo_3.png
  const io = store({ 'logo.png': b('A'), 'logo_2.png': b('B') });
  const r = await placer(io)('logo.png', b('B'));
  ok(r.status === 'reused' && r.name === 'logo_2.png' && !io.uploads.length, 'Kolizja nazwy: wcześniejsza kopia o tej treści użyta ponownie', r);
}
{ // nazwa poprawiana jak na serwerze – odnośnik wskazuje zapisany plik
  const io = store();
  const r = await placer(io)('Moja Czcionka.woff2', b('W'));
  ok(r.status === 'renamed' && r.name === 'Moja_Czcionka.woff2' && io.fsys.has('Moja_Czcionka.woff2'), 'Nazwa ze spacją: zapisana i zwrócona jako Moja_Czcionka.woff2', r);
}
{ // różne rozszerzenia nie są łączone, nawet przy tej samej treści
  const io = store({ 'A.ttf': b('X') });
  const r = await placer(io)('A.otf', b('X'));
  ok(r.status === 'new' && r.name === 'A.otf', 'Czcionka o innym rozszerzeniu nie jest podmieniana', r);
}
{ // grafiki o tej samej treści pod inną nazwą – zostają osobno (porównujemy tylko czcionki)
  const io = store({ 'a.png': b('P') });
  const r = await placer(io)('b.png', b('P'));
  ok(r.status === 'new' && r.name === 'b.png', 'Grafika pod inną nazwą: wgrana (deduplikacja dotyczy czcionek)', r);
}
{ // dwie czcionki o tej samej treści w jednej paczce → jedna w assets
  const io = store({ 'Inne.ttf': b('Z') });
  const res = await P.installAssets(new Map([['F.ttf', b('Q')], ['F kopia.ttf', b('Q')], ['Inne.ttf', b('Y')]]), { existing: io.existing(), fetchAsset: io.fetchAsset, upload: io.upload });
  ok(res.uploaded.join() === 'F.ttf,Inne_2.ttf' && res.map.get('F kopia.ttf') === 'F.ttf' && res.reused.length === 1 && res.renamed[0]?.[1] === 'Inne_2.ttf', 'installAssets: duplikat w paczce scalony, kolizja przemianowana', res);
  const sc = P.renameAssets({ nodes: [{ props: { font: 'F kopia.ttf' } }, { props: { text: 'url(/assets/Inne.ttf)' } }], packAssets: ['F kopia.ttf'] }, res.map);
  ok(sc.nodes[0].props.font === 'F.ttf' && sc.nodes[1].props.text === 'url(/assets/Inne_2.ttf)' && sc.packAssets[0] === 'F.ttf', 'renameAssets: font, odnośnik /assets/ i packAssets przepisane', sc);
}
{ // niedozwolony typ
  const io = store();
  let err = null; try { await placer(io)('skrypt.js', b('x')); } catch (e) { err = e.message; }
  ok(err && !io.uploads.length, 'Niedozwolone rozszerzenie: błąd, nic nie wgrane', err);
}

// buildPack: ta sama czcionka pod dwiema nazwami trafia do paczki raz
{
  const files = { 'A.ttf': b('SAME'), 'A_2.ttf': b('SAME'), 'B.woff2': b('SAME') };
  const scenes = [
    { id: 's1', name: 'jeden', nodes: [{ props: { font: 'A.ttf' } }] },
    { id: 's2', name: 'dwa', nodes: [{ props: { font: 'A_2.ttf' } }, { props: { font: 'B.woff2' } }], packAssets: ['A.ttf', 'A_2.ttf'] },
  ];
  const { assets, merged, blob } = await P.buildPack({ scenes, known: Object.keys(files), fetchAsset: async (n) => files[n] });
  ok(assets.map((a) => a.name).join() === 'A.ttf,B.woff2' && merged.length === 1 && merged[0][0] === 'A_2.ttf', 'buildPack: identyczne czcionki raz (inne rozszerzenie osobno)', { assets, merged });
  const pk = await P.readPack(new Uint8Array(await blob.arrayBuffer()));
  ok(pk.scenes[1].nodes[0].props.font === 'A.ttf' && pk.scenes[1].packAssets.join() === 'A.ttf', 'buildPack: odnośniki i packAssets wskazują jedną nazwę', pk.scenes[1]);
}

// importPack: wspólny import Artist i Trio
{
  const io = store({ 'F.ttf': b('FONT') });
  const made = [];
  const api = async (m, path, body) => {
    if (path === '/api/scenes') { const sv = { ...body, id: 'n' + made.length }; made.push(sv); return sv; }
    if (path === '/api/pages') { if (body.number === 100) throw new Error('numer strony 100 zajęty'); made.push(body); return body; }
    throw new Error('?');
  };
  const pk = {
    manifest: { _type: 'vizrs_pack' },
    scenes: [{ id: 'a', name: 'Belka', rev: 3, nodes: [{ props: { font: 'kopia.ttf' } }] }],
    pages: [{ id: 'p', number: 100, name: 'str', scene: 'a', values: {} }],
    assets: new Map([['kopia.ttf', b('FONT')]]),
  };
  const r = await P.importPack(pk, { api, existing: io.existing(), fetchAsset: io.fetchAsset, upload: io.upload, sceneNames: ['Belka'] });
  ok(r.scenes.length === 1 && r.scenes[0].nodes[0].props.font === 'F.ttf' && !('rev' in r.scenes[0]), 'importPack: scena jako nowa, font wskazuje istniejącą czcionkę', r.scenes[0]);
  ok(r.pages === 1 && made[1].scene === 'n0' && !('number' in made[1]), 'importPack: strona wskazuje nową scenę, zajęty numer → nowy', made[1]);
  ok(r.report.some((l) => l.includes('już istnieje – dodano jako kopię')) && r.report.some((l) => l.includes('użyto istniejącej')) && r.report.some((l) => l.includes('numer zajęty')), 'importPack: raport', r.report);
  const r2 = await P.importPack(pk, { api, existing: io.existing(), fetchAsset: io.fetchAsset, upload: io.upload, withPages: false });
  ok(r2.pages === 0 && r2.report.some((l) => l.includes('stron Trio pominięto')), 'importPack bez stron (praca lokalna)', r2.report);
  ok(/1 scen, 1 stron, 0 nowych plików, 1 plików już było/.test(P.importSummary(r)), 'importSummary', P.importSummary(r));
}
{ // dawny .vizrs.json: tylko odczyt, raport o braku plików
  const pk = P.legacyScenePack(JSON.stringify({ name: 'Stara', nodes: [{ props: { src: 'tlo.png', font: 'Jest.ttf' } }] }));
  const r = await P.importPack(pk, { api: async (m, p, body) => ({ ...body, id: 'x' }), existing: ['Jest.ttf'], fetchAsset: async () => null, upload: async () => 'x' });
  ok(r.scenes.length === 1 && r.report.some((l) => l.includes('starszy format')) && r.report.some((l) => l.includes('brak pliku tlo.png')) && !r.report.some((l) => l.includes('Jest.ttf')), 'legacyScenePack: scena wczytana, brakujący plik w raporcie', r.report);
  let err = null; try { P.legacyScenePack('{"a":1}'); } catch (e) { err = e.message; }
  ok(err === 'to nie jest scena VizRS', 'legacyScenePack: odrzuca inny JSON', err);
}

console.log(failures ? `\n${failures} FAIL z ${checks}` : `OK (${checks} sprawdzeń)`);
process.exit(failures ? 1 : 0);
