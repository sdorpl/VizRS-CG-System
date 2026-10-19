// Paczka „BTB CG” dla VizRS: każdy element z katalogu wzorców BTB CG konwertowany osobno (web/btb-io.js) → examples/BTB_CG/
//   node tools/build-btb-pack.mjs [katalog-wzorców] [katalog-wyjściowy]
//   BTB_CG_DIR = BTB CG System (pliki /uploads/… i font Bahnschrift), domyślnie ~/Nextcloud/GitHub/BTB-CG-System-alpha
// Wynik: show.json (sceny + strony, numeracja wg rodzaju elementu), assets/ (obrazy, fonty), konwersja.json (ostrzeżenia).
import fs from 'fs';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.resolve(process.argv[2] || path.join(os.homedir(), 'Nextcloud/GitHub/btb-cgeditor/CG elements converted'));
const OUT = path.resolve(process.argv[3] || path.join(HERE, '../../examples/BTB_CG'));
const BTB = path.resolve(process.env.BTB_CG_DIR || path.join(os.homedir(), 'Nextcloud/GitHub/BTB-CG-System-alpha'));
const io = await import(path.join(HERE, '../web/btb-io.js'));

const OFFLINE = ['Oswald', 'Barlow Condensed', 'Inter', 'Roboto Condensed', 'Montserrat', 'Roboto'];
// fonty BTB dostępne jako pliki: rodzina → [plik w BTB, nazwa zasobu VizRS]
const FONT_FILES = { Bahnschrift: ['font/BAHNSCHRIFT.TTF', 'Bahnschrift.ttf'] };
// rodzaj elementu BTB → [pierwszy numer strony, warstwa VizRS]; osobne warstwy pozwalają łączyć belkę, plakietkę, ticker i zegar na antenie
const KIND = { LOWER_THIRD: [8001, 'main'], BADGE: [8101, 'badge'], TICKER: [8201, 'ticker'], CLOCK: [8301, 'clock'], IMAGE: [8401, 'logo'] };

const slug = (s) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/ł/g, 'l').replace(/Ł/g, 'L').replace(/[^\w-]+/g, '_').replace(/_+/g, '_').replace(/^_|_$/g, '').toLowerCase();

fs.rmSync(OUT, { recursive: true, force: true });
fs.mkdirSync(path.join(OUT, 'assets'), { recursive: true });
const copyAsset = (from, name) => { fs.copyFileSync(from, path.join(OUT, 'assets', name)); return name; };

const show = { name: 'BTB CG – elementy przeniesione', scenes: [], pages: [], playlists: [] };
const report = [];
const next = {};
const names = new Set();
for (const file of fs.readdirSync(SRC).filter((f) => /\.(json|cgpack)$/i.test(f)).sort((a, b) => a.localeCompare(b, 'pl'))) {
  const buf = fs.readFileSync(path.join(SRC, file));
  const { elements, files } = await io.readBtb(/\.cgpack$/i.test(file) ? new Uint8Array(buf) : buf.toString('utf8'), file);
  elements.forEach((el, i) => {
    const r = io.convertElement(el, {
      knownFonts: [...OFFLINE, 'Arial', ...Object.keys(FONT_FILES)],
      assetName: (src) => {
        const k = Object.keys(files).find((n) => n === src || n === src.replace(/^\//, ''));
        if (k) { const n = k.split('/').pop(); fs.writeFileSync(path.join(OUT, 'assets', n), files[k]); return n; }
        const up = src.match(/^\/?uploads\/([\w.-]+)$/);
        if (up && fs.existsSync(path.join(BTB, 'uploads', up[1]))) return copyAsset(path.join(BTB, 'uploads', up[1]), up[1]);
        return null;
      },
    });
    // fonty z plików BTB → zasób VizRS (renderer ładuje rodzinę o nazwie pliku z /assets)
    const walk = (n) => {
      const ff = FONT_FILES[n.props.font];
      if (ff && fs.existsSync(path.join(BTB, ff[0]))) n.props.font = copyAsset(path.join(BTB, ff[0]), ff[1]);
      (n.children || []).forEach(walk);
    };
    r.scene.nodes.forEach(walk);
    const stem = file.replace(/\.(json|cgpack)$/i, '') + (elements.length > 1 ? `_${i + 1}` : '');
    const [first, layer] = KIND[el.type] || [8501, 'main'];
    const number = (next[el.type] ??= first);
    next[el.type]++;
    // ta sama nazwa w kilku plikach (np. warianty „Belka - PILNE”) → dopisek z nazwy pliku
    let name = r.scene.name;
    if (names.has(name)) name = `${name} (${file})`.slice(0, 100);
    names.add(name);
    const id = ('btb_' + slug(stem)).slice(0, 50);
    show.scenes.push({ ...r.scene, id, name, layer });
    show.pages.push({ id: 'p' + number, number, name, scene: id, values: r.page.values });
    report.push({ file, element: el.name, type: el.type || '', scene: id, page: number, layer, warnings: r.warnings });
  });
}
show.pages.sort((a, b) => a.number - b.number);
fs.writeFileSync(path.join(OUT, 'show.json'), JSON.stringify(show, null, 1) + '\n');
fs.writeFileSync(path.join(OUT, 'konwersja.json'), JSON.stringify(report, null, 1) + '\n');
console.log(`${report.length} elementów → ${path.relative(process.cwd(), OUT)}/show.json, zasoby: ${fs.readdirSync(path.join(OUT, 'assets')).join(', ') || '—'}`);
for (const r of report) console.log(`${r.page} ${r.layer.padEnd(6)} ${r.file}${r.warnings.length ? '\n   ! ' + r.warnings.join('\n   ! ') : ''}`);
