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

// reguły plików: wspólne wektory z VizRS CG Artist (src/deploy.rs czyta ten sam plik)
{
  const fs = await import('node:fs');
  const V = JSON.parse(fs.readFileSync(new URL('./vectors/cgpack-placer.json', import.meta.url), 'utf8'));
  for (const c of V.cases) {
    const io = store(Object.fromEntries(Object.entries(c.existing).map(([n, d]) => [n, b(d)])));
    let place = placer(io);
    for (const st of c.steps) {
      if (st.fresh) place = placer(io);
      const r = await place(st.from, b(st.data));
      ok(r.status === st.status && r.name === st.name, `wektor: ${c.name} – ${st.from}`, r);
    }
    const now = Object.fromEntries([...io.fsys].map(([n, d]) => [n, new TextDecoder().decode(d)]));
    ok(JSON.stringify(Object.entries(now).sort()) === JSON.stringify(Object.entries(c.files).sort()), `wektor: ${c.name} – stan assets`, now);
  }
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
  ok(r.scenes.length === 1 && r.scenes[0].nodes[0].props.font === 'F.ttf' && !('rev' in r.scenes[0]) && r.scenes[0].name === 'Belka (2)', 'importPack bez planu: zajęta nazwa → kopia „Belka (2)”, font wskazuje istniejącą czcionkę', r.scenes[0]);
  ok(r.pages === 1 && made[1].scene === 'n0' && !('number' in made[1]), 'importPack: strona wskazuje nową scenę, zajęty numer → nowy', made[1]);
  ok(r.report.some((l) => l.includes('nazwa zajęta – dodano jako „Belka (2)”')) && r.report.some((l) => l.includes('użyto istniejącej')) && r.report.some((l) => l.includes('numer zajęty')), 'importPack: raport', r.report);
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


// P0: buildPack nie zbiera plików ze stron innych scen
{
  const files = { 'a.png': b('A'), 'obca.png': b('O') };
  const scenes = [{ id: 's1', name: 'jeden', nodes: [{ props: { src: 'a.png' } }] }];
  const pages = [{ id: 'p1', number: 1, scene: 's1', values: {} }, { id: 'p2', number: 2, scene: 'inna', values: { img: 'obca.png' } }];
  const { assets, blob } = await P.buildPack({ scenes, pages, known: Object.keys(files), fetchAsset: async (n) => files[n] });
  const pk = await P.readPack(new Uint8Array(await blob.arrayBuffer()));
  ok(assets.map((a) => a.name).join() === 'a.png' && pk.pages.length === 1, 'buildPack: pliki tylko ze stron pakowanych scen', assets);
}

// plan importu: stany scen, decyzja użytkownika, aktualizacja z rev, numeracja stron, przesunięcie, powiązania
{
  const mkApi = (log) => async (m, path, body) => {
    log.push([m, path, structuredClone(body)]);
    if (m === 'PUT' && path.startsWith('/api/scenes/') && body.rev != null && body.rev !== 7) throw new Error('konflikt: obiekt zmieniony w międzyczasie');
    return { ...body, id: body.id || 'gen' + log.length, number: body.number ?? 5000 };
  };
  const show = {
    scenes: [
      { id: 'same', name: 'Taka sama', rev: 2, nodes: [{ props: { text: 'x' } }] },
      { id: 'chg', name: 'Zmieniona', rev: 7, nodes: [{ props: { text: 'stara' } }] },
      { id: 'inna', name: 'Zajęta', rev: 1, nodes: [] },
    ],
    pages: [{ id: 'pz', number: 1001, scene: 'chg', values: {}, rev: 3 }, { id: 'pe', number: 2000, scene: 'same', values: {}, rev: 4 }],
  };
  const pk = {
    manifest: { _type: 'vizrs_pack' },
    scenes: [
      { id: 'same', name: 'Taka sama', nodes: [{ props: { text: 'x' } }] },
      { id: 'chg', name: 'Zmieniona', nodes: [{ props: { text: 'nowa' } }] },
      { id: 'nz', name: 'Zajęta', nodes: [], dock: [{ target: 'page:1001|scene:Zajęta', dy: -90 }] },
      { id: 'nowa', name: 'Nowa', nodes: [] },
    ],
    pages: [
      { id: 'p1', number: 1001, scene: 'chg', values: { a: 1 } },
      { id: 'p2', number: 1002, scene: 'nowa', values: {} },
      { id: 'pe', number: 2000, scene: 'same', values: { b: 2 } },
      { id: 'p3', number: 1003, scene: 'brak', values: {} },
    ],
    assets: new Map(),
  };
  const io = store();
  const plan = await P.planImport(pk, { show, existing: [], fetchAsset: io.fetchAsset });
  ok(plan.scenes.map((e) => e.status).join() === 'same,changed,name-taken,new', 'planImport: stany scen same/changed/name-taken/new', plan.scenes.map((e) => e.status));
  ok(plan.scenes.map((e) => e.action).join() === 'skip,,,add' && P.undecided(plan).length === 2, 'planImport: zmieniona i zajęta nazwa czekają na decyzję', plan.scenes.map((e) => e.action));
  ok(plan.pages.map((r) => r.status).join() === 'number-taken,new,exists,no-scene' && plan.pages[0].to === 2001, 'planPages: zajęty numer → pierwszy wolny po najwyższym, istniejąca strona, strona bez sceny', plan.pages);
  let err = null; try { await P.importPack(pk, { api: mkApi([]), existing: [], fetchAsset: io.fetchAsset, upload: io.upload, show, plan }); } catch (e) { err = e.message; }
  ok(/wybierz, co zrobić/.test(err || ''), 'importPack: bez decyzji dla konfliktów nic nie zapisuje', err);

  plan.scenes[1].action = 'update'; plan.scenes[2].action = 'copy';
  P.planPages(plan, 100);
  ok(plan.pages.map((r) => r.to).join() === '1101,1102,2000,1103' && plan.pages[0].status === 'new', 'planPages: przesunięcie numerów o +100', plan.pages.map((r) => r.to));
  ok(plan.refs.length === 2 && plan.refs.some((r) => r.from === 'page:1001' && r.to === 'page:1101') && plan.refs.some((r) => r.to === 'scene:Zajęta (2)'), 'planPages: powiązania dock wskazujące przenumerowaną stronę i nazwę kopii', plan.refs);
  plan.pages[2].action = 'overwrite';
  const log = [];
  const r = await P.importPack(pk, { api: mkApi(log), existing: [], fetchAsset: io.fetchAsset, upload: io.upload, show, plan });
  const put = log.find(([m, p]) => m === 'PUT' && p === '/api/scenes/chg');
  ok(put && put[2].rev === 7 && put[2].nodes[0].props.text === 'nowa', 'importPack: aktualizacja sceny PUT z rev z planu', put);
  const cp = log.find(([m, p, bd]) => m === 'POST' && p === '/api/scenes' && bd.name === 'Zajęta (2)');
  ok(cp && cp[2].id === 'nz' && cp[2].dock[0].target === 'page:1101|scene:Zajęta (2)', 'importPack: kopia „Zajęta (2)” z id z paczki i poprawionymi powiązaniami', cp);
  ok(log.some(([m, p, bd]) => m === 'POST' && p === '/api/scenes' && bd.id === 'nowa'), 'importPack: nowa scena zachowuje id z paczki');
  const pp = log.filter(([, p]) => p.startsWith('/api/pages'));
  ok(pp.length === 3 && pp[0][2].number === 1101 && pp[0][2].scene === 'chg' && pp[0][2].id === 'p1', 'importPack: strona aktualizowanej sceny wskazuje istniejącą scenę, numer z planu', pp[0]);
  ok(pp.some(([m, p, bd]) => m === 'PUT' && p === '/api/pages/pe' && bd.rev === 4 && bd.number === 2000 && bd.values.b === 2), 'importPack: nadpisanie strony o tym samym id (rev, numer istniejący)', pp);
  ok(r.updated.length === 1 && r.scenes.length === 2 && r.pages === 3 && r.report.some((l) => l.includes('strona 1003') && l.includes('jej sceny nie ma w paczce')), 'importPack: wynik i raport pominiętej strony', r);
  ok(/1 zaktualizowanych/.test(P.importSummary(r)), 'importSummary: zaktualizowane sceny', P.importSummary(r));

  // konflikt rewizji przy aktualizacji: raport, strony dalej wskazują istniejącą scenę
  const plan2 = await P.planImport(pk, { show: { ...show, scenes: show.scenes.map((s) => (s.id === 'chg' ? { ...s, rev: 6 } : s)) }, existing: [], fetchAsset: io.fetchAsset });
  plan2.scenes[1].action = 'update'; plan2.scenes[2].action = 'skip'; P.planPages(plan2);
  const log2 = [];
  const r2 = await P.importPack(pk, { api: mkApi(log2), existing: [], fetchAsset: io.fetchAsset, upload: io.upload, show, plan: plan2 });
  ok(r2.report.some((l) => l.includes('zmieniona w międzyczasie')) && log2.some(([, p, bd]) => p === '/api/pages' && bd.scene === 'chg'), 'importPack: konflikt rev – raport, strony wskazują istniejącą scenę', r2.report);
  ok(log2.some(([, p, bd]) => p === '/api/pages' && bd.scene === 'inna') === false && !log2.some(([, p, bd]) => p === '/api/scenes' && bd.name?.startsWith('Zajęta')), 'importPack: pominięta scena o zajętej nazwie nie jest zapisywana', log2);
  ok(P.uniqueName('A', new Set(['A', 'A (2)'])) === 'A (3)' && P.uniqueName('B', new Set(['A'])) === 'B', 'uniqueName');
  ok(P.sceneKey({ id: 1, rev: 2, b: 1, a: [1, { y: 1, x: 2 }] }) === P.sceneKey({ a: [1, { x: 2, y: 1 }], b: 1 }), 'sceneKey: bez id/rev, niezależny od kolejności kluczy');
}

console.log(failures ? `\n${failures} FAIL z ${checks}` : `OK (${checks} sprawdzeń)`);
process.exit(failures ? 1 : 0);
