// Paczka „BTB alpha” dla VizRS: grafiki z gałęzi alpha BTB CG System (szablony HTML/CSS/JS) odtworzone jako sceny VizRS.
//   node tools/build-btb-alpha.mjs [katalog]   (domyślnie examples/archiwum/BTB_ALPHA)
// Źródło: <katalog>/zrodlo.json – grafiki z bazy alpha + pomiar DOM każdej grafiki po wejściu na wyjściu BTB
// (tools/measure-btb-alpha.cjs). Szablony kodowe nie mają sceny, więc scena powstaje z pomiaru: prostokąty, gradienty,
// ramki, teksty, obrazy; tickery, zegary i belki rysowane na <canvas> są składane według kodu szablonu.
// Każda grafika = strona; grafiki o identycznym wyglądzie (różny tylko tekst) dzielą jedną scenę.
// Wynik: show.json (sceny + strony 14001–14501), assets/ (obrazy z data: URI).
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { register } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const WEB = pathToFileURL(path.join(HERE, '../web/')).href;
register('data:text/javascript,' + encodeURIComponent(`export async function resolve(s, c, next) {
  return /^\\/[\\w.-]+\\.js$/.test(s) ? next(${JSON.stringify(WEB)} + s.slice(1), c) : next(s, c); }`));
const io = await import('../web/btb-io.js');
const DIR = path.resolve(process.argv[2] || path.join(HERE, '../../examples/archiwum/BTB_ALPHA'));
const SRC = JSON.parse(fs.readFileSync(path.join(DIR, 'zrodlo.json'), 'utf8'));
const BAHN = 'Bahnschrift.ttf';

fs.rmSync(path.join(DIR, 'assets'), { recursive: true, force: true });
fs.mkdirSync(path.join(DIR, 'assets'), { recursive: true });

// ---------------------------------------------------------------- CSS → wartości VizRS
const r1 = (v) => Math.round(v * 10) / 10;
const hex2 = (n) => Math.round(Math.max(0, Math.min(255, n))).toString(16).padStart(2, '0');
function color(c) {
  const m = /rgba?\(([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)/.exec(c || '');
  if (!m) return c && c.startsWith('#') ? c.toLowerCase() : null;
  const a = m[4] === undefined ? 1 : +m[4];
  return '#' + hex2(+m[1]) + hex2(+m[2]) + hex2(+m[3]) + (a < 1 ? hex2(a * 255) : '');
}
// linear-gradient(160deg, c1, c2 [, c3]) → { angle, stops: [[kolor, pozycja %]] }
function gradient(s) {
  const m = /^linear-gradient\((.*)\)$/.exec((s || '').trim());
  if (!m) return null;
  const parts = m[1].split(/,(?![^(]*\))/).map((x) => x.trim());
  let angle = 180;
  if (/deg$/.test(parts[0])) angle = parseFloat(parts.shift());
  else if (/^to /.test(parts[0])) angle = { 'to right': 90, 'to left': 270, 'to bottom': 180, 'to top': 0 }[parts.shift()] ?? 180;
  const stops = parts.map((p, i) => {
    const c = /^(rgba?\([^)]*\)|#[0-9a-f]+)\s*([\d.]+%)?/i.exec(p);
    return [color(c[1]), c[2] ? parseFloat(c[2]) : (i / (parts.length - 1)) * 100];
  });
  return { angle: ((angle % 360) + 360) % 360, stops };
}
const px = (s) => parseFloat(s) || 0;
function shadowBlur(s) {
  const m = /\)\s+(-?[\d.]+)px\s+(-?[\d.]+)px\s+([\d.]+)px/.exec(s || '');
  return m ? +m[3] : 0;
}
// font BTB → VizRS: Bahnschrift z pliku (paczka), reszta po nazwie
function fontOf(fam) {
  const first = String(fam || '').split(',')[0].replace(/["']/g, '').trim();
  if (/bahnschrift/i.test(first)) return BAHN;
  if (/system-ui|ui-monospace/i.test(first)) return 'Arial';
  return first || 'Arial';
}
function parseFont(f) {
  const [fam, size, weight, col, ls, tt, lh, style] = f.split('|');
  return { font: fontOf(fam), size: px(size), weight: String(weight), color: color(col), spacing: ls === 'lsnormal' ? 0 : px(ls.slice(2)),
    upper: tt === 'uppercase', italic: style === 'italic', lineHeight: lh === 'lhnormal' ? 1.2 : r1(px(lh.slice(2)) / px(size) * 100) / 100 };
}

// ---------------------------------------------------------------- ruch (animacja BTB grafiki → motion grupy)
function groupMotion(g, origin) {
  const warn = () => {};
  const a = g.animation || {};
  const mo = {};
  const i = io.motionSpec(a.in || { type: 'slide', direction: 'left', duration: 0.5 }, warn, g.name);
  const o = io.motionSpec(a.out || { type: 'fade', duration: 0.5 }, warn, g.name);
  if (i) mo.in = i;
  if (o) mo.out = o;
  mo.origin = origin;
  return mo;
}
const sceneTimes = (mo) => {
  const inEnd = (mo.in?.dur ?? 0) + (mo.in?.delay ?? 0);
  const outLen = (mo.out?.dur ?? 0) + (mo.out?.delay ?? 0);
  const stop = Math.round(Math.max(0.5, inEnd + 0.1) * 100) / 100;
  return { stops: [stop], duration: Math.round((stop + Math.max(0.3, outLen)) * 100) / 100 };
};

// ---------------------------------------------------------------- obrazy z data: URI → assets
function dataAsset(uri, base) {
  const m = /^data:image\/(png|webp|jpeg|gif|svg\+xml);base64,(.+)$/.exec(uri || '');
  if (!m) return null;
  const buf = Buffer.from(m[2], 'base64');
  const name = `${base}_${crypto.createHash('sha1').update(buf).digest('hex').slice(0, 8)}.${m[1] === 'svg+xml' ? 'svg' : m[1] === 'jpeg' ? 'jpg' : m[1]}`;
  fs.writeFileSync(path.join(DIR, 'assets', name), buf);
  return name;
}

// ---------------------------------------------------------------- pomiar DOM → węzły
let uid = 0;
const nid = (p) => `${p}${++uid}`;
const inside = (a, b) => a.x >= b.x - 2 && a.y >= b.y - 2 && a.x + a.w <= b.x + b.w + 2 && a.y + a.h <= b.y + b.h + 2;
const visible = (e) => e.op > 0.05 && e.x < 1920 && e.x + e.w > 0 && e.y < 1080 && e.y + e.h > 0;

/** Prostokąt(y) z tła elementu: kolor, gradient 2-kolorowy, gradient symetryczny c1-c2-c1 (dwie połówki). */
function rectsOf(e, name) {
  const out = [];
  const base = { x: e.x, y: e.y, w: e.w, h: e.h };
  const rad = /%$/.test(e.rad) ? Math.min(e.w, e.h) / 2 : px(e.rad);
  const common = {};
  if (rad) common.radius = r1(rad * (e.scale || 1));
  const sh = shadowBlur(e.sh);
  if (sh) common.shadow = sh;
  const gr = gradient(e.bg);
  if (gr && gr.stops.length >= 2) {
    const s = gr.stops;
    if (s.length === 3 && s[0][0] === s[2][0] && Math.abs(s[1][1] - 50) < 1 && gr.angle === 90) {
      // CSS 90deg c1 0% → c2 50% → c1 100%: dwie połówki (VizRS ma gradient dwukolorowy)
      const hw = e.w / 2;
      out.push({ type: 'rect', name, props: { ...base, w: r1(hw), color: s[0][0], color2: s[1][0], gradAngle: 90, ...common } });
      out.push({ type: 'rect', name: name + ' (2)', props: { ...base, x: r1(e.x + hw - 1), w: r1(hw + 1), color: s[1][0], color2: s[2][0], gradAngle: 90, ...common } });
    } else {
      out.push({ type: 'rect', name, props: { ...base, color: s[0][0], color2: s[s.length - 1][0], gradAngle: gr.angle, ...common } });
    }
  } else if (e.bc) {
    out.push({ type: e.rad === '50%' && Math.abs(e.w - e.h) < 2 ? 'ellipse' : 'rect', name, props: { ...base, color: color(e.bc), ...common } });
  }
  // ramki: równa → borderWidth; jednostronna → pasek
  if (e.bord) {
    const [t, rr, b, l, ct, cl, cb] = e.bord.replace(/rgba?\([^)]*\)/g, (m) => m.replace(/\s/g, '')).split(' ');
    const W = [px(t), px(rr), px(b), px(l)];
    if (W.every((v) => v === W[0] && v > 0) && out[0]) Object.assign(out[0].props, { borderWidth: W[0], borderColor: color(ct) });
    else {
      if (W[3] > 0) out.push({ type: 'rect', name: name + ' – krawędź L', props: { x: e.x, y: e.y, w: W[3], h: e.h, color: color(cl) } });
      if (W[2] > 0) out.push({ type: 'rect', name: name + ' – krawędź D', props: { x: e.x, y: e.y + e.h - W[2], w: e.w, h: W[2], color: color(cb) } });
      if (W[0] > 0) out.push({ type: 'rect', name: name + ' – krawędź G', props: { x: e.x, y: e.y, w: e.w, h: W[0], color: color(ct) } });
    }
  }
  return out;
}

/** Tekst: pole do szerokości tła (tekst BTB jest w jednym wierszu, nowrap), wyśrodkowany gdy BTB go centruje. */
function textOf(e, bgs, field, scale = 1) {
  const f = parseFont(e.font);
  f.size = r1(f.size * scale); f.spacing = r1(f.spacing * scale);
  const host = bgs.filter((b) => inside(e, b)).sort((a, b) => a.w * a.h - b.w * b.h)[0];
  let x = e.x, w = e.w + 8, align = 'left';
  if (host) {
    const cx = e.x + e.w / 2, hx = host.x + host.w / 2;
    // wyśrodkowany przez BTB: środki zgodne, a pole tekstu węższe od tła (zegar zawsze na środku swojego pola)
    const centred = Math.abs(cx - hx) < 4 && (field === null || (host.w > e.w + 12 && e.w < host.w * 0.8));
    if (centred) { x = host.x; w = host.w; align = 'center'; }
    else w = Math.max(e.w + 8, host.x + host.w - e.x - 20);
  }
  const props = { x, y: e.y, w: r1(w), h: e.h, text: e.text, font: f.font, size: f.size, weight: f.weight, color: f.color, align, valign: 'middle',
    lineHeight: f.lineHeight, textFit: 'squeeze', squeezeMin: 0.5 };
  const lines = Math.round(e.h / (f.size * f.lineHeight));
  if (lines > 1) Object.assign(props, { w: e.w + 4, maxLines: lines, textFit: 'squeeze-lines', lineBreak: 'fill' });
  if (f.spacing) props.spacing = f.spacing;
  if (f.upper) props.upper = true;
  if (f.italic) props.italic = true;
  const n = { type: 'text', name: field || 'tekst', props };
  if (field) n.fields = { text: field };
  return n;
}

// ---------------------------------------------------------------- rodziny szablonów
const TICKER_BOX = /utk-container|news-container-v8|rep-bottom-bar|ticker-vert-container|sub-ticker-v8/;
const MODE = { horizontal: 'crawl', scrolling: 'crawl', vertical: 'roll', whip: 'whip' };
const SEP = { skewed: 'skew', dot: 'dot', square: 'square', pipe: 'pipe' };

function tickerNodes(g, els) {
  const box = els.find((e) => TICKER_BOX.test(e.cls));
  if (!box) return null;
  const nodes = rectsOf(box, 'tło paska');
  const wiper = els.find((e) => /utk-wiper$|wiper-v8$|ticker-vert-label/.test(e.cls.trim()));
  const wtext = els.find((e) => /wiper-text|ticker-vert-label/.test(e.cls) && e.text);
  const item = els.find((e) => /utk-item|utk-ticker-item|news-text-v8|ticker-item|ticker-vert-item|utk-msg-box|utk-single-text/.test(e.cls) && e.text);
  const sep = els.find((e) => /sep/.test(e.cls));
  const f = item ? parseFont(item.font) : { font: BAHN, size: 30, weight: '600', color: '#ffffff', upper: true };
  const tpl = g.templateId;
  let mode = MODE[g.tickerMode] || null;
  if (!mode) mode = /news-container-v8/.test(box.cls) ? 'whip' : /ticker-vert/.test(box.cls) ? 'roll' : 'crawl';
  const p = { x: box.x, y: box.y, w: box.w, h: box.h, tickerMode: mode, text: io.btbItemsText(g.items || []), font: f.font, size: f.size, weight: f.weight,
    color: f.color, textFit: 'squeeze-shrink', squeezeMin: 0.6 };
  if (f.upper) p.upper = true;
  if (f.spacing) p.spacing = f.spacing;
  if (mode === 'crawl') p.speed = Math.max(40, Math.min(400, +g.speed || 120));
  else p.hold = mode === 'roll' ? Math.max(3, +g.speed > 20 ? 5 : +g.speed || 5) : 6;
  if (sep && g.separatorStyle !== 'none') { p.sepStyle = SEP[g.separatorStyle || 'skewed'] || 'skew'; p.sepColor = color(sep.bc) || undefined; }
  if (wiper && wtext) {
    const wf = parseFont(wtext.font);
    const wg = gradient(wiper.bg);
    p.label = wtext.text;
    p.catW = wiper.w;
    p.catBg = wg ? wg.stops[0][0] : color(wiper.bc);
    p.catFg = wf.color;
    p.labelSize = wf.size;
    p.labelWeight = wf.weight;
    if (els.some((e) => /gleam/.test(e.cls)) || (wiper.pse || []).length) p.gleam = (g.wiper?.gleamDuration || 2) + (g.wiper?.gleamFrequency || 3);
  } else if (wiper) {
    // wiper bez tekstu (Ticker zwykły): sam kolorowy klocek z połyskiem, treść zaczyna się za nim
    const wr = rectsOf(wiper, 'wiper')[0];
    if (wr) { if (els.some((e) => /gleam/.test(e.cls))) wr.motion = { bg: 'shimmer' }; nodes.push(wr); }
    p.x = wiper.x + wiper.w; p.w = box.x + box.w - p.x;
  }
  if (mode === 'crawl' && p.label !== undefined) p.text = io.btbItemsText((g.items || []).map((x) => (x && typeof x === 'object' ? x.text : x)));
  if (/sub-ticker-v8/.test(box.cls)) Object.assign(p, { color: '#000000', size: 30, weight: '400', font: BAHN, upper: true, sepStyle: 'square', sepColor: '#7f0000', speed: 80 });
  const t = { type: 'ticker', name: 'ticker', props: p, fields: { text: 'items' } };
  if (p.label !== undefined && g.templateId !== 'vertical-ticker') t.fields.label = 'wiper_text';
  else if (p.label !== undefined) t.fields.label = 'wiper_text';
  nodes.push(t);
  return { nodes, fieldMeta: { items: { label: 'Wiadomości' }, ...(t.fields.label ? { wiper_text: { label: 'Etykieta (wiper)', oneLine: true } } : {}) },
    values: { items: p.text, ...(t.fields.label ? { wiper_text: p.label } : {}) }, origin: [0, 0], tpl };
}

function genericNodes(g, els) {
  const scale = +g.layout?.scale || 1;
  const nodes = [];
  const bgs = [];
  const values = {};
  const fieldMeta = {};
  let seen = [];
  const texts = [];
  for (const e of els) {
    if (!visible(e)) continue;
    if (/gleam/.test(e.cls)) continue; // połysk → motion.bg na tle
    if (e.tag === 'CANVAS') continue;
    if (e.tag === 'IMG') {
      const src = dataAsset(g.sideImage && !/logo/.test(e.cls) ? g.sideImage : g.url, /logo/.test(e.cls) ? 'logo' : 'grafika');
      if (src) nodes.push({ type: 'image', name: /logo/.test(e.cls) ? 'logo' : 'grafika', props: { x: e.x, y: e.y, w: e.w, h: e.h, src, fitMode: 'contain' } });
      continue;
    }
    if (/rep-side-graphic/.test(e.cls)) continue; // pusty url(''), gradient nad pustym tłem – niewidoczny
    // wrapper + container o tym samym prostokącie i tle: jeden kształt
    const key = [e.x, e.y, e.w, e.h, e.bg, e.bc].join('|');
    if ((e.bg || e.bc || e.bord) && !seen.includes(key)) {
      seen.push(key);
      const rs = rectsOf({ ...e, scale }, e.cls.replace(/\s*(active|visible|entering)\b/g, '').trim() || e.tag.toLowerCase());
      if (/na-zywo-dot/.test(e.cls)) for (const r of rs) {
        // kropka: pulse-dot 1 s alternate (krycie 1 → 0,3, skala 1 → 0,9); pomiar trafia w różne fazy – rysujemy pełną
        r.type = 'ellipse'; delete r.props.radius;
        r.props.w = r.props.h = 14; r.props.x = r1(e.x + e.w / 2 - 7); r.props.y = r1(e.y + e.h / 2 - 7);
        r.motion = { loop: { type: 'breathe', dur: 2, amp: 70 } };
      }
      if (/modern-plate/.test(e.cls)) for (const r of rs) r.motion = { bg: 'shimmer' };
      nodes.push(...rs);
      if (rs.length) bgs.push(e);
    }
    if (e.text) texts.push(e);
  }
  // pola: pierwszy tekst = TITLE, drugi = SUBTITLE (jak ocgInputs szablonów BTB)
  texts.forEach((e, i) => {
    const clock = /rep-clock|current-time/.test(e.cls);
    if (clock) {
      const n = textOf(e, bgs, null, scale);
      n.type = 'clock';
      n.name = 'zegar';
      Object.assign(n.props, { mode: 'time', format: 'HH:mm' });
      for (const k of ['text', 'textFit', 'squeezeMin', 'lineHeight', 'upper']) delete n.props[k];
      nodes.push(n);
      return;
    }
    const field = i === 0 ? 'title' : i === 1 ? 'subtitle' : `tekst_${i + 1}`;
    const n = textOf(e, bgs, field, scale);
    const tx = io.textSpec(g.animation?.text, 'in', () => {}, g.name);
    if (tx && field === 'title') n.motion = { textIn: tx };
    nodes.push(n);
    values[field] = e.text;
    fieldMeta[field] = { label: field === 'title' ? 'Tytuł' : field === 'subtitle' ? 'Podtytuł' : `Tekst ${i + 1}` };
  });
  return { nodes, values, fieldMeta };
}

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
