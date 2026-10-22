// Paczki grafik .cgpack – jedna paczka = cała oprawa (sceny + strony + grafiki + czcionki):
//   node tools/build-packs.mjs [katalog-wyjściowy]   (domyślnie examples/)
// Źródła: examples/archiwum/ (dawne pliki .show.json i katalog BTB_CG).
// → INFO24.cgpack, R7_NEWS.cgpack, BTB.cgpack (BTB zawiera Bahnschrift – nie do dystrybucji).
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

const PACKS = [
  ['INFO24.cgpack', () => read('INFO24.show.json'), [path.join(ARCH, 'assets')]],
  ['R7_NEWS.cgpack', () => read('R7_NEWS.show.json'), [path.join(ARCH, 'assets')]],
  ['BTB.cgpack', btb, [path.join(ARCH, 'BTB_CG/assets')]],
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
