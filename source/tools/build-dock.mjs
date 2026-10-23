// Paczka DOCK dla VizRS: grafiki z gałęzi TRIO oryginału (szablony HTML/CSS/JS) odtworzone jako sceny VizRS
// razem z powiązaniami (dokowanie `layout.attachedToGraphicId*` → `scene.dock`).
//   node tools/build-dock.mjs [katalog]   (domyślnie examples/archiwum/DOCK)
// Źródło: <katalog>/zrodlo.json – grafiki z db.json gałęzi TRIO + pomiar DOM każdej grafiki na wyjściu oryginału
// (tools/measure-alpha.cjs, serwer oryginału z gałęzi TRIO). Obrazy z `/uploads/` czyta z katalogu uploads/ repozytorium oryginału (SRC_REPO; pliki są poza gitem).
// Każda grafika = strona na własnej warstwie `z<N>_…` (N = warstwa oryginału), więc – – wszystkie mogą być na wizji naraz.
// Wynik: show.json (sceny + strony 15001–15401), konwersja.json, assets/.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { register } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const WEB = pathToFileURL(path.join(HERE, '../web/')).href;
register('data:text/javascript,' + encodeURIComponent(`export async function resolve(s, c, next) {
  return /^\\/[\\w.-]+\\.js$/.test(s) ? next(${JSON.stringify(WEB)} + s.slice(1), c) : next(s, c); }`));
const io = await import('../web/element-import.js');
const DIR = path.resolve(process.argv[2] || path.join(HERE, '../../examples/archiwum/DOCK'));
const SRC_REPO = process.env.CG_SRC_DIR || '.';
const SRC = JSON.parse(fs.readFileSync(path.join(DIR, 'zrodlo.json'), 'utf8'));
const L = await import('./measure-lib.mjs');
L.setup({ io, dir: DIR });
const { r1, BAHN, groupMotion, sceneTimes, dataAsset, visible, tickerNodes, genericNodes } = L;

fs.rmSync(path.join(DIR, 'assets'), { recursive: true, force: true });
fs.mkdirSync(path.join(DIR, 'assets'), { recursive: true });

// obraz z /uploads/ serwera oryginału → data: URI
function uploadUri(src) {
  const m = /\/uploads\/([\w.-]+\.(png|webp|jpe?g|gif|svg))$/.exec(src || '');
  if (!m) return src;
  const buf = fs.readFileSync(path.join(SRC_REPO, 'uploads', m[1]));
  const mime = m[2] === 'svg' ? 'svg+xml' : m[2] === 'jpg' ? 'jpeg' : m[2];
  return `data:image/${mime};base64,${buf.toString('base64')}`;
}

// ---------------------------------------------------------------- korekty pomiaru
function prepare(g, els) {
  // „Belka glowna”: pomiar po flip-x zastał kontener przesunięty o 1080 px (translateY) – położenie wg layoutu jak pozostałe belki
  const box = els.find((e) => /rep-top-bar/.test(e.cls));
  if (box && /matrix\(1, 0, 0, 1, 0, 1080\)/.test(box.tf)) els = els.map((e) => ({ ...e, y: e.y - 1080 }));
  const img = els.find((e) => e.tag === 'IMG');
  if (img) g = { ...g, url: uploadUri(img.src) };
  return { g, els };
}

// animacje elementów szablonu (`data-cg-anim` w html_template – cgmotion oryginału) dla plakietek miast: tło wipe, tekst clip
function cgAnim(g, nodes) {
  if (!/data-cg-anim="wipe/.test(g.html || '') && !/miasto|chwili/i.test(g.name)) return;
  for (const n of nodes) {
    if (n.type === 'rect') n.motion = { ...(n.motion || {}), in: { type: 'wipe', dir: 'left', dur: 0.4, ease: 'power2.out' } };
    if (n.type === 'text') n.motion = { ...(n.motion || {}), in: { type: 'clip', dir: 'left', dur: 0.45, delay: 0.12 }, out: { type: 'clip', dir: 'right', dur: 0.3 } };
  }
}

// ---------------------------------------------------------------- budowa
const KIND = { belka: [15001, 'belka'], plakietka: [15101, 'plakietka'], ticker: [15201, 'ticker'], zegar: [15301, 'zegar'], obraz: [15401, 'obraz'] };
const kindOf = (g, els) => g.type === 'TICKER' ? 'ticker' : g.type === 'CLOCK' ? 'zegar' : g.type === 'IMAGE' ? 'obraz'
  : els.some((e) => /na-zywo/.test(e.cls)) ? 'plakietka' : 'belka';
const ORIGIN = { plakietka: [0.5, 0.5], ticker: [0.5, 0.5], zegar: [0.5, 0.5], obraz: [0.5, 0.5], belka: [0, 0.5] };
const slug = (s) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/ł/g, 'l').replace(/Ł/g, 'L').replace(/[^\w]+/g, '_').replace(/^_|_$/g, '').toLowerCase();
const nameOf = (g) => g.name.replace(/\s*\(kopia\)/g, '').replace(/osoatniej/, 'ostatniej').trim();
const strip = (n) => { const c = structuredClone(n); delete c.id; if (c.props) { delete c.props.text; if (c.type === 'text') delete c.props.w; } if (c.children) c.children = c.children.map(strip); return c; };

const show = { name: 'DOCK – grafiki przeniesione', scenes: [], pages: [], playlists: [] };
const report = [];
const origin = {};
const pageOf = {}; // id grafiki oryginału → numer strony
const built = [];
const next = {};
SRC.grafiki.forEach((g0, i) => {
  const { g, els } = prepare(g0, SRC.pomiar[i] || []);
  const kind = kindOf(g, els);
  let b = g.type === 'TICKER' ? tickerNodes(g, els) : null;
  if (!b) {
    if (!els.some(visible)) { report.push(`pominięto „${g.name}”: grafika nic nie rysuje`); return; }
    b = genericNodes(g, els);
  }
  if (!b.nodes.length) { report.push(`pominięto „${g.name}”: brak elementów`); return; }
  const tk = b.nodes.find((n) => n.type === 'ticker');
  if (tk && els.some((e) => /[a-z]+-fw-sep/.test(e.cls))) Object.assign(tk.props, { sepStyle: 'dot', sepColor: '#ffffff80' }); // kropka 6 px, krycie 0,5
  if (tk && tk.props.tickerMode === 'crawl') tk.props.crawlFrom = 'left'; // oryginał: taśma od razu pełna, pierwsza wiadomość od lewej
  cgAnim(g, b.nodes);
  next[kind] = next[kind] ?? KIND[kind][0];
  const number = next[kind]++;
  pageOf[g.id] = number;
  origin[number] = `${i}: ${g.name}`;
  built.push({ g, b, kind, number });
});

// powiązania z oryginału: `attachedToGraphicId` (oś Y) / `attachedToGraphicIdX` (oś X), tryb „stały odstęp” (manual) –
// gdy którykolwiek rodzic jest na wizji, dziecko przesuwa się o attachOffset w 0,6 s (renderer oryginał: transition 0.6s)
function dockOf(g) {
  const out = [];
  const lay = g.layout || {};
  for (const [ids, edge, off, axis] of [[lay.attachedToGraphicId, lay.attachToEdgeY, lay.attachOffsetY, 'dy'], [lay.attachedToGraphicIdX, lay.attachToEdgeX, lay.attachOffsetX, 'dx']]) {
    const list = [].concat(ids || []);
    const pages = list.filter((id) => id in pageOf && id !== g.id).map((id) => `page:${pageOf[id]}`);
    const lost = list.filter((id) => !(id in pageOf));
    if (lost.length) report.push(`„${g.name}”: pominięto powiązanie z grafikami spoza bazy (${lost.join(', ')})`);
    if (!pages.length || !+off) continue;
    if (edge && edge !== 'manual') report.push(`„${g.name}”: tryb krawędzi ${edge} zastąpiony stałym odstępem`);
    out.push({ target: pages.join('|'), dx: axis === 'dx' ? +off : 0, dy: axis === 'dy' ? +off : 0, dur: 0.6 });
  }
  return out;
}

const bySig = new Map();
const sceneNames = new Set();
for (const { g, b, kind, number } of built) {
  const xs = b.nodes.map((n) => n.props.x), ys = b.nodes.map((n) => n.props.y);
  const gx = Math.min(...xs), gy = Math.min(...ys);
  const gw = Math.max(...b.nodes.map((n) => n.props.x + n.props.w)) - gx, gh = Math.max(...b.nodes.map((n) => n.props.y + n.props.h)) - gy;
  let uid = 0;
  const children = b.nodes.map((n) => ({ id: `${n.type === 'text' ? 'tx' : n.type === 'ticker' ? 'tk' : n.type === 'clock' ? 'ck' : n.type === 'image' ? 'im' : 'sh'}${++uid}`, ...n,
    props: { ...n.props, x: r1(n.props.x - gx), y: r1(n.props.y - gy) } }));
  const motion = groupMotion(g, ORIGIN[kind]);
  const group = { id: 'grp', type: 'group', name: nameOf(g), props: { x: r1(gx), y: r1(gy), w: r1(gw), h: r1(gh) }, children, motion };
  const dock = dockOf(g);
  const z = Math.max(0, Math.min(99, Math.round(+g.layout?.layer || 1)));
  const sig = JSON.stringify([kind, z, strip(group), b.fieldMeta, dock]);
  let scene = bySig.get(sig);
  if (!scene) {
    let base = nameOf(g), name = base, k = 2;
    while (sceneNames.has(name)) name = `${base} ${k++}`;
    sceneNames.add(name);
    const id = 'trio_' + slug(name);
    scene = { id, name, layer: `z${z}_${slug(name)}`.slice(0, 50), width: 1920, height: 1080, ...sceneTimes(motion), fieldMeta: b.fieldMeta, nodes: [group] };
    if (dock.length) scene.dock = dock;
    bySig.set(sig, scene);
    show.scenes.push(scene);
  }
  show.pages.push({ id: 'trio_p' + number, number, name: nameOf(g) + (kind !== 'ticker' && b.values?.title ? ` – ${String(b.values.title).slice(0, 40)}` : ''),
    scene: scene.id, values: b.values || {} });
}
show.pages.sort((a, b) => a.number - b.number);
fs.writeFileSync(path.join(DIR, 'show.json'), JSON.stringify(show, null, 1) + '\n');
fs.writeFileSync(path.join(DIR, 'konwersja.json'), JSON.stringify({ zrodlo: SRC.zrodlo, uwagi: report, strony: origin,
  powiazania: show.scenes.filter((s) => s.dock).map((s) => ({ scena: s.name, dock: s.dock })) }, null, 1) + '\n');
console.log(`${show.scenes.length} scen, ${show.pages.length} stron, powiązań: ${show.scenes.reduce((n, s) => n + (s.dock?.length || 0), 0)}, pliki: ${fs.readdirSync(path.join(DIR, 'assets')).join(', ')}`);
for (const r of report) console.log('  ' + r);
