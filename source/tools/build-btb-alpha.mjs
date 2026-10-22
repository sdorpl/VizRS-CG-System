// Paczka „BTB alpha” dla VizRS: grafiki z gałęzi alpha BTB CG System (szablony HTML/CSS/JS) odtworzone jako sceny VizRS.
//   node tools/build-btb-alpha.mjs [katalog]   (domyślnie examples/archiwum/BTB_ALPHA)
// Źródło: <katalog>/zrodlo.json – grafiki z bazy alpha + pomiar DOM każdej grafiki po wejściu na wyjściu BTB
// (tools/measure-btb-alpha.cjs). Szablony kodowe nie mają sceny, więc scena powstaje z pomiaru: prostokąty, gradienty,
// ramki, teksty, obrazy; tickery, zegary i belki rysowane na <canvas> są składane według kodu szablonu.
// Każda grafika = strona; grafiki o identycznym wyglądzie (różny tylko tekst) dzielą jedną scenę.
// Wynik: show.json (sceny + strony 14001–14501), assets/ (obrazy z data: URI).
import fs from 'node:fs';
import path from 'node:path';
import { register } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const WEB = pathToFileURL(path.join(HERE, '../web/')).href;
register('data:text/javascript,' + encodeURIComponent(`export async function resolve(s, c, next) {
  return /^\\/[\\w.-]+\\.js$/.test(s) ? next(${JSON.stringify(WEB)} + s.slice(1), c) : next(s, c); }`));
const io = await import('../web/btb-io.js');
const DIR = path.resolve(process.argv[2] || path.join(HERE, '../../examples/archiwum/BTB_ALPHA'));
const SRC = JSON.parse(fs.readFileSync(path.join(DIR, 'zrodlo.json'), 'utf8'));

fs.rmSync(path.join(DIR, 'assets'), { recursive: true, force: true });
fs.mkdirSync(path.join(DIR, 'assets'), { recursive: true });

const L = await import('./btb-measure-lib.mjs');
L.setup({ io, dir: DIR });
const { r1, color, gradient, px, BAHN, groupMotion, sceneTimes, dataAsset, visible, rectsOf, textOf, tickerNodes, genericNodes } = L;
let uid = 0;
const nid = (p) => `${p}${++uid}`;

// ---------------------------------------------------------------- szablony kodowe (canvas, widżet) – wg js_template
function ocgCanvas(tpl, title, sub) {
  const ease = 'power2.inOut';
  if (tpl === 'tpl-ocg-1772988663431') { // IMIĘ NAZWISKO: belka 1575×110 + podtytuł 1575×60, x 30, y 795
    const y = 1080 - 110 - 60 - 115;
    return { nodes: [
      { type: 'rect', name: 'belka podtytułu', props: { x: 30, y: y + 110, w: 1575, h: 60, color: '#d1d1d1e6', color2: '#f0f0f0e6', gradAngle: 90 }, motion: { in: { type: 'wipe', dir: 'left', dur: 0.7, delay: 0.25, ease } } },
      { type: 'text', name: 'subtitle', props: { x: 60, y: y + 110, w: 1500, h: 60, text: sub, font: BAHN, size: 32, weight: '400', color: '#000000', valign: 'middle', textFit: 'squeeze', squeezeMin: 0.5 }, fields: { text: 'subtitle' }, motion: { textIn: { type: 'fade', dur: 0.3, delay: 0.55 } } },
      { type: 'rect', name: 'belka główna', props: { x: 30, y, w: 787.5, h: 110, color: '#ffffff', color2: '#e0e0e0', gradAngle: 90 }, motion: { in: { type: 'wipe', dir: 'left', dur: 0.67, ease } } },
      { type: 'rect', name: 'belka główna (2)', props: { x: 816.5, y, w: 788.5, h: 110, color: '#e0e0e0', color2: '#ffffff', gradAngle: 90 }, motion: { in: { type: 'wipe', dir: 'left', dur: 0.67, ease } } },
      { type: 'rect', name: 'cień dolny', props: { x: 30, y: y + 106, w: 1575, h: 4, color: '#0000001a' }, motion: { in: { type: 'wipe', dir: 'left', dur: 0.67, ease } } },
      { type: 'text', name: 'title', props: { x: 60, y, w: 1520, h: 110, text: title, font: BAHN, size: 80, weight: '700', color: '#000000', valign: 'middle', textFit: 'squeeze', squeezeMin: 0.5 }, fields: { text: 'title' }, motion: { textIn: { type: 'fade', dur: 0.33, delay: 0.27 } } },
    ], fieldMeta: { title: { label: 'Imię i nazwisko' }, subtitle: { label: 'Funkcja' } }, values: { title, subtitle: sub } };
  }
  if (tpl === 'tpl-ocg-1772988841814') { // BELKA WS: belka 1575×170 (#4a0404→#8b0000→#ff0000) + tag WYDANIE SPECJALNE
    const y = 1080 - 170 - 115;
    return { nodes: [
      { type: 'rect', name: 'tag', props: { x: 30, y: y - 40, w: 380, h: 40, color: '#e91e63', color2: '#f44336', gradAngle: 90 }, motion: { in: { type: 'fade', dur: 0.3, delay: 0.35 } } },
      { type: 'text', name: 'tag_text', props: { x: 30, y: y - 40, w: 380, h: 40, text: 'WYDANIE SPECJALNE', font: BAHN, size: 24, weight: '700', color: '#ffffff', align: 'center', valign: 'middle', textFit: 'squeeze', squeezeMin: 0.5 }, fields: { text: 'tag' }, motion: { in: { type: 'fade', dur: 0.3, delay: 0.35 } } },
      { type: 'rect', name: 'belka', props: { x: 30, y, w: 787.5, h: 170, color: '#4a0404', color2: '#8b0000', gradAngle: 90 }, motion: { in: { type: 'wipe', dir: 'left', dur: 0.67, ease } } },
      { type: 'rect', name: 'belka (2)', props: { x: 816.5, y, w: 788.5, h: 170, color: '#8b0000', color2: '#ff0000', gradAngle: 90 }, motion: { in: { type: 'wipe', dir: 'left', dur: 0.67, ease } } },
      { type: 'text', name: 'title', props: { x: 50, y: y + 5, w: 1540, h: 170, text: title, font: BAHN, size: 80, weight: '700', color: '#ffffff', valign: 'middle', textFit: 'squeeze', squeezeMin: 0.5 }, fields: { text: 'title' }, motion: { textIn: { type: 'fade', dur: 0.15, delay: 0.55 } } },
    ], fieldMeta: { tag: { label: 'Tag', oneLine: true }, title: { label: 'Tytuł' } }, values: { tag: 'WYDANIE SPECJALNE', title } };
  }
  if (tpl === 'tpl-ocg-canvas_1') { // BELKA NORM: 1575×170 #A0A0A0→#E6E6E6→#FFFFFF
    const y = 1080 - 170 - 115;
    return { nodes: [
      { type: 'rect', name: 'belka', props: { x: 30, y, w: 787.5, h: 170, color: '#a0a0a0', color2: '#e6e6e6', gradAngle: 90 }, motion: { in: { type: 'wipe', dir: 'left', dur: 0.67, ease } } },
      { type: 'rect', name: 'belka (2)', props: { x: 816.5, y, w: 788.5, h: 170, color: '#e6e6e6', color2: '#ffffff', gradAngle: 90 }, motion: { in: { type: 'wipe', dir: 'left', dur: 0.67, ease } } },
      { type: 'text', name: 'title', props: { x: 50, y: y + 5, w: 1515, h: 170, text: title, font: BAHN, size: 80, weight: '700', color: '#000000', valign: 'middle', textFit: 'squeeze', squeezeMin: 0.5 }, fields: { text: 'title' }, motion: { textIn: { type: 'fade', dur: 0.15, delay: 0.55 } } },
    ], fieldMeta: { title: { label: 'Tytuł' } }, values: { title } };
  }
  if (tpl === 'tpl-ocg-wiper_news') { // widżet 190×170: zegar na jasnym tle + BTB INFO na bordo
    const x = 100, y = 800;
    return { nodes: [
      { type: 'rect', name: 'widget-container', props: { x, y, w: 190, h: 170, color: '#000000', shadow: 10 } },
      { type: 'rect', name: 'clock-top', props: { x, y, w: 190, h: 82, color: '#f0f0f0', color2: '#c0c0c0', gradAngle: 135 }, motion: { bg: 'shimmer' } },
      { type: 'clock', name: 'zegar', props: { x, y, w: 190, h: 82, mode: 'time', format: 'HH:mm', font: BAHN, size: 65, weight: '400', color: '#000000', align: 'center', valign: 'middle' } },
      { type: 'rect', name: 'info-bottom', props: { x, y: y + 88, w: 190, h: 82, color: '#8b0000' } },
      { type: 'text', name: 'btb', props: { x, y: y + 88, w: 118, h: 82, text: 'BTB', font: 'Arial', size: 38, weight: '700', color: '#ffffff', align: 'right', valign: 'middle', textFit: 'none' } },
      { type: 'text', name: 'info', props: { x: x + 120, y: y + 88 + 6, w: 70, h: 82, text: 'INFO', font: 'Arial', size: 18, weight: '700', color: '#ffffff', align: 'left', valign: 'middle', textFit: 'none' } },
    ], fieldMeta: {}, values: {}, motion: { in: { type: 'pop', dur: 0.6, ease: 'back.out' }, out: { type: 'fade', dur: 0.4 }, origin: [0.5, 0.5] } };
  }
  if (tpl === 'tpl-ocg-tag_specjalne_v7') { // TAG: WYDANIE SPECJALNE 380×40 (w alpha nie rysuje się: brak klasy .active na rodzicu)
    return { nodes: [
      { type: 'rect', name: 'live-tag', props: { x: 100, y: 800, w: 380, h: 40, color: '#e91e63', color2: '#f44336', gradAngle: 135, borderWidth: 2, borderColor: '#ffffff', shadow: 10 }, motion: { bg: 'shimmer' } },
      { type: 'text', name: 'title', props: { x: 100, y: 800, w: 380, h: 40, text: title, font: BAHN, size: 24, weight: '700', color: '#ffffff', align: 'center', valign: 'middle', upper: true, spacing: 1, textFit: 'squeeze', squeezeMin: 0.5 }, fields: { text: 'title' } },
    ], fieldMeta: { title: { label: 'Tytuł', oneLine: true } }, values: { title },
    motion: { in: { type: 'slide', dir: 'left', dur: 0.6, ease: 'cubic-bezier(0.2, 0.8, 0.2, 1)' }, out: { type: 'fade', dur: 0.4 }, origin: [0.5, 0.5] } };
  }
  return null;
}
// Republika Ticker (Cycling): w alpha CSS ukrywa pasek (display: none) – odtworzony wg reguł szablonu: pasek 1728×60 na 96/980, wiper, wiadomości po kolei
function cycling(g) {
  const items = g.items || ['Pierwsza wiadomość', 'Druga wiadomość', 'Trzecia wiadomość'];
  const text = io.btbItemsText(items);
  return { nodes: [
    { type: 'rect', name: 'rep-bottom-bar', props: { x: 96, y: 980, w: 1728, h: 60, color: '#002d5e' } },
    { type: 'ticker', name: 'ticker', props: { x: 96, y: 980, w: 1728, h: 60, tickerMode: 'roll', hold: 5, text, font: BAHN, size: 30, weight: '800', color: '#ffffff', upper: true,
      label: 'PILNE', catW: 170, catBg: '#ff0000', catFg: '#ffffff', labelSize: 35, labelWeight: '900', gleam: 5, textFit: 'squeeze-shrink', squeezeMin: 0.6 }, fields: { text: 'items', label: 'wiper_text' } },
  ], fieldMeta: { items: { label: 'Wiadomości' }, wiper_text: { label: 'Etykieta (wiper)', oneLine: true } }, values: { items: text, wiper_text: 'PILNE' }, origin: [0, 0.5] };
}

// ---------------------------------------------------------------- budowa
const KIND = { belka: [14001, 'main'], plakietka: [14101, 'badge'], ticker: [14201, 'ticker'], zegar: [14301, 'clock'], obraz: [14401, 'logo'], widzet: [14501, 'clock'] };
const kindOf = (g) => g.type === 'TICKER' ? 'ticker' : g.type === 'CLOCK' ? 'zegar' : g.type === 'IMAGE' ? 'obraz'
  : g.templateId === 'tpl-ocg-wiper_news' ? 'widzet'
  : /na-zywo-badge|city-bar|d6fcc3ac|tag_/.test(g.templateId) ? 'plakietka' : 'belka';
const ORIGIN = (tpl) => (/na-zywo|city-bar|d6fcc3ac|tag_/.test(tpl) ? [0.5, 0.5] : /utk|ticker|a1b2|41c87|6d848|23f25|57361|c53f3|urgent|crawl|cycling/.test(tpl) ? [0.5, 0.5] : [0, 0.5]);
const nameOf = (g) => g.name.replace(/^SYN /, '').replace(/\s*\((Eksport|Szablon)\)/g, '').replace(/\s*\(\d\d:\d\d:\d\d\)/g, '').replace(/\s*\(kopia\)/g, '').trim();
const strip = (n) => { const c = structuredClone(n); delete c.id; if (c.props) { delete c.props.text; if (c.type === 'text') delete c.props.w; } if (c.children) c.children = c.children.map(strip); return c; };

const show = { name: 'BTB CG alpha – grafiki przeniesione', scenes: [], pages: [], playlists: [] };
const bySig = new Map();
const next = {};
const report = [];
const origin = {};
const sceneNames = new Set();
SRC.grafiki.forEach((g, i) => {
  const els = SRC.pomiar[i] || [];
  const tpl = g.templateId;
  const kind = kindOf(g);
  let built = ocgCanvas(tpl, g.title || '', g.subtitle || 'Ekspert ds. Geopolityki');
  if (!built && tpl === 'tpl-republika-ticker-cycling') built = cycling(g);
  if (!built && g.type === 'TICKER') built = tickerNodes(g, els);
  if (!built) {
    if (!els.some(visible)) { report.push(`pominięto „${g.name}”: szablon ${tpl} nie istnieje w bazie alpha albo grafika nic nie rysuje`); return; }
    built = genericNodes(g, els);
  }
  if (!built.nodes.length) { report.push(`pominięto „${g.name}”: brak elementów`); return; }
  // grupa w położeniu z BTB; dzieci względem grupy
  const xs = built.nodes.map((n) => n.props.x), ys = built.nodes.map((n) => n.props.y);
  const gx = Math.min(...xs), gy = Math.min(...ys);
  const gw = Math.max(...built.nodes.map((n) => n.props.x + n.props.w)) - gx, gh = Math.max(...built.nodes.map((n) => n.props.y + n.props.h)) - gy;
  uid = 0;
  const children = built.nodes.map((n) => ({ id: nid(n.type === 'text' ? 'tx' : n.type === 'ticker' ? 'tk' : n.type === 'clock' ? 'ck' : n.type === 'image' ? 'im' : 'sh'), ...n,
    props: { ...n.props, x: r1(n.props.x - gx), y: r1(n.props.y - gy) } }));
  const motion = built.motion || groupMotion(g, built.origin || ORIGIN(tpl));
  const group = { id: 'grp', type: 'group', name: nameOf(g), props: { x: r1(gx), y: r1(gy), w: r1(gw), h: r1(gh) }, children, motion };
  const sig = JSON.stringify([kind, strip(group), built.fieldMeta]);
  let scene = bySig.get(sig);
  if (!scene) {
    let base = nameOf(g), name = base, k = 2;
    while (sceneNames.has(name)) name = `${base} ${k++}`;
    sceneNames.add(name);
    const id = 'alpha_' + name.normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/ł/g, 'l').replace(/Ł/g, 'L').replace(/[^\w]+/g, '_').replace(/^_|_$/g, '').toLowerCase();
    scene = { id, name, layer: KIND[kind][1], width: 1920, height: 1080, ...sceneTimes(motion), fieldMeta: built.fieldMeta, nodes: [group] };
    bySig.set(sig, scene);
    show.scenes.push(scene);
  }
  next[kind] = next[kind] ?? KIND[kind][0];
  const number = next[kind]++;
  origin[number] = `${i}: ${g.name}`;
  show.pages.push({ id: 'alpha_p' + number, number, name: nameOf(g) + (g.title && kind !== 'ticker' && built.values?.title && !/^SYN/.test(g.name) ? ` – ${String(built.values.title).slice(0, 40)}` : ''),
    scene: scene.id, values: built.values || {} });
});
fs.writeFileSync(path.join(DIR, 'show.json'), JSON.stringify(show, null, 1) + '\n');
fs.writeFileSync(path.join(DIR, 'konwersja.json'), JSON.stringify({ zrodlo: SRC.zrodlo, uwagi: report, strony: origin }, null, 1) + '\n');
console.log(`${show.scenes.length} scen, ${show.pages.length} stron, pliki: ${fs.readdirSync(path.join(DIR, 'assets')).join(', ')}`);
for (const r of report) console.log('  ' + r);
