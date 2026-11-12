// Paczki grafik .cgpack – jedna paczka = cała oprawa (sceny + strony + grafiki + czcionki):
//   node tools/build-packs.mjs [katalog-wyjściowy]   (domyślnie examples/)
// Źródła: examples/archiwum/ (pliki .show.json i assets/) → INFO24.cgpack, R7_NEWS.cgpack.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const { buildPack } = await import('../web/cgpack.js');
const ARCH = path.resolve(HERE, '../../examples/archiwum');
const OUT = path.resolve(process.argv[2] || path.join(HERE, '../../examples'));
const read = (f) => JSON.parse(fs.readFileSync(path.join(ARCH, f), 'utf8'));

const PACKS = [
  ['INFO24.cgpack', () => read('INFO24.show.json'), [path.join(ARCH, 'assets')]],
  ['R7_NEWS.cgpack', () => read('R7_NEWS.show.json'), [path.join(ARCH, 'assets')]],
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
