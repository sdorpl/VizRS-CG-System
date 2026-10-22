// Paczki grafik .cgpack – jedna paczka = cała oprawa (sceny + strony + grafiki + czcionki):
//   node tools/build-packs.mjs [katalog-wyjściowy]   (domyślnie examples/)
// Źródła: examples/archiwum/ (dawne pliki .show.json i katalog BTB_CG).
// → INFO24.cgpack, R7_NEWS.cgpack, BTB.cgpack, BTB_INFO24.cgpack, BTB_ALPHA.cgpack, BTB_TRIO.cgpack (BTB* zawierają Bahnschrift – nie do dystrybucji).
import fs from 'node:fs';
import path from 'node:path';
import { register } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
// moduły web/ importują się ścieżkami serwera (`/btb-io.js`) – w Node kierujemy je do web/
const WEB = pathToFileURL(path.join(HERE, '../web/')).href;
register('data:text/javascript,' + encodeURIComponent(`export async function resolve(s, c, next) {
  return /^\\/[\\w.-]+\\.js$/.test(s) ? next(${JSON.stringify(WEB)} + s.slice(1), c) : next(s, c); }`));
const { buildPack } = await import('../web/cgpack.js');
const ARCH = path.resolve(HERE, '../../examples/archiwum');
const OUT = path.resolve(process.argv[2] || path.join(HERE, '../../examples'));
const read = (f) => JSON.parse(fs.readFileSync(path.join(ARCH, f), 'utf8'));

// BTB: decyzje o duplikatach (2026-10-22) – usunięty „zegarek (kopia)”, czytelne nazwy tickerów z trybami
function btb() {
  const cg = read('BTB_CG/show.json'), pilne = read('BTB_PILNE.show.json');
  const drop = new Set(cg.scenes.filter((s) => s.name === 'zegarek (kopia)').map((s) => s.id));
  const rename = {
    'Ticker (Tryb: Whip / Poziomy / Pionowy)': 'Ticker tryby (Whip / Poziomy / Pionowy)',
    'Ticker (Whip / Poziomy / Pionowy)': 'Ticker tryby – kategorie',
  };
  const scenes = [...cg.scenes, ...pilne.scenes].filter((s) => !drop.has(s.id))
    .map((s) => (rename[s.name] ? { ...s, name: rename[s.name] } : s));
  const pages = [...cg.pages, ...pilne.pages].filter((p) => !drop.has(p.scene));
  return { scenes, pages };
}

// BTB_INFO24: zestaw INFO24 w stylistyce BTB (czerwień, żółty akcent, Bahnschrift) – jak R7_NEWS, ale generowany
// z INFO24.show.json: nowe id (btb_*), strony +8000 (9001–13001), kolory i fonty wg tabel poniżej.
const BTB_COLORS = {
  '#141418': '#362626', '#0f0f13': '#2a1c1c', '#101014': '#2a1c1c', '#1d1d22': '#3a2a2a', '#f3f3f3': '#ffffff',
  '#d0102b': '#b91c1c', '#c8102e': '#b91c1c', '#e30613': '#c32222', '#a80c22': '#7e0e02', '#8c0f1f': '#7e0e02',
  '#ff2a3d': '#ffcc00', '#ff9aa7': '#ffe680', '#e8471f': '#d3a012',
  '#1c3f94': '#0047ab', '#2c4fb0': '#0134ad', '#173264': '#001a80', '#152a6b': '#001a80', '#1b2f6b': '#001a80',
  '#0b1730': '#000057', '#cfd8ea': '#e0e0e0',
};
function btbInfo24() {
  const src = read('INFO24.show.json');
  const style = (v, k) => {
    if (Array.isArray(v)) return v.map((x) => style(x));
    if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([kk, x]) => [kk, style(x, kk)]));
    if (k === 'font' && v !== 'Roboto Mono') return 'Bahnschrift.ttf';
    const m = typeof v === 'string' && /^(#[0-9a-f]{6})([0-9a-f]{2})?$/i.exec(v);
    return m && BTB_COLORS[m[1].toLowerCase()] ? BTB_COLORS[m[1].toLowerCase()] + (m[2] || '') : v;
  };
  const scenes = src.scenes.map((s) => ({ ...style(s), id: 'btb_' + s.id, name: 'BTB ' + s.name.replace(/^INFO24 /, '') }));
  const pages = src.pages.map((p) => ({ ...p, id: 'btb_' + p.id, number: p.number + 8000, scene: 'btb_' + p.scene }));
  return { scenes, pages };
}

const PACKS = [
  ['INFO24.cgpack', () => read('INFO24.show.json'), [path.join(ARCH, 'assets')]],
  ['R7_NEWS.cgpack', () => read('R7_NEWS.show.json'), [path.join(ARCH, 'assets')]],
  ['BTB.cgpack', btb, [path.join(ARCH, 'BTB_CG/assets')]],
  ['BTB_INFO24.cgpack', btbInfo24, [path.join(ARCH, 'assets'), path.join(ARCH, 'BTB_CG/assets')]],
  // grafiki z gałęzi alpha BTB CG System (szablony kodowe) – wynik tools/build-btb-alpha.mjs
  ['BTB_ALPHA.cgpack', () => read('BTB_ALPHA/show.json'), [path.join(ARCH, 'BTB_ALPHA/assets'), path.join(ARCH, 'BTB_CG/assets')]],
  // grafiki z gałęzi TRIO BTB CG System z powiązaniami (dock) – wynik tools/build-btb-trio.mjs
  ['BTB_TRIO.cgpack', () => read('BTB_TRIO/show.json'), [path.join(ARCH, 'BTB_TRIO/assets'), path.join(ARCH, 'BTB_CG/assets')]],
];

let bad = 0;
for (const [file, load, dirs] of PACKS) {
  const { scenes, pages } = load();
  const where = new Map();
  for (const d of dirs) for (const n of fs.readdirSync(d)) where.set(n, path.join(d, n));
  const { blob, missing, assets } = await buildPack({
    scenes, pages, known: [...where.keys()], generator: 'VizRS tools/build-packs.mjs',
    fetchAsset: async (n) => new Uint8Array(fs.readFileSync(where.get(n))),
  });
  fs.writeFileSync(path.join(OUT, file), new Uint8Array(await blob.arrayBuffer()));
  console.log(`${file}: ${scenes.length} scen, ${pages.length} stron, pliki: ${assets.map((a) => a.name).join(', ') || '–'}`);
  if (missing.length) { bad++; console.error(`  brak plików: ${missing.join(', ')}`); }
}
process.exit(bad ? 1 : 0);
