// Import elementów BTB CG do VizRS (F4.2) – tylko w tym kierunku: BTB jest bazą pomysłów, VizRS systemem docelowym.
//   BTB `.json` (eksport v3 z BTB, goły element lub lista) i `.cgpack` (ZIP) → scena + strona VizRS, z raportem „co się nie przeniosło”
// Moduł bez DOM (jedyna zależność: ticker-model.js) – działa w przeglądarce i w Node (testy: tests/vectors/btb.js).
// Format: docs w BTB-CG `docs/edytor-kontrakt.md` (scene = { version, detached, frame:{w,h}, layers[] }).

import { serializeTickerItems, parseTickerItems } from './ticker-model.js';
import * as MO from './motion.js';

const EASES = { 'ease-in': 'in', 'ease-out': 'out', 'ease-in-out': 'inout', ease: 'inout', linear: 'linear' };
const round3 = (n) => Math.round(n * 1000) / 1000;
const num = (v, d = 0) => (typeof v === 'number' && isFinite(v) ? v : d);

/** Krzywa BTB (nazwa CSS albo cubic-bezier) → krzywa VizRS. */
export function mapEase(e) {
  const s = String(e || '').trim();
  if (EASES[s]) return EASES[s];
  const m = s.match(/cubic-bezier\(\s*([-\d.]+)\s*,\s*([-\d.]+)\s*,\s*([-\d.]+)\s*,\s*([-\d.]+)\s*\)/);
  if (m) { const y2 = +m[4], y1 = +m[2]; return y2 > 1 || y1 > 1 ? 'back' : +m[1] <= 0.35 && y2 >= 0.9 ? 'out' : 'inout'; }
  return 'out';
}

export function toHex(c) {
  const s = String(c ?? '').trim();
  const m = s.match(/^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+)\s*)?\)$/i);
  if (!m) return s;
  const h = (n) => Math.max(0, Math.min(255, +n)).toString(16).padStart(2, '0');
  const a = m[4] !== undefined && +m[4] < 1 ? h(Math.round(+m[4] * 255)) : '';
  return '#' + h(m[1]) + h(m[2]) + h(m[3]) + a;
}

const weightOf = (w) => {
  const s = String(w ?? '700').toLowerCase();
  if (s === 'bold') return '700';
  if (s === 'normal') return '400';
  const n = parseInt(s);
  return isNaN(n) ? '700' : String(Math.max(300, Math.min(900, Math.round(n / 100) * 100)));
};

/** Format zegara BTB → VizRS. BTB: `HH:MM` i `HH:MM:SS` (MM = minuty) albo format rozszerzony
 *  z tokenami `dddd MMMM YYYY DD MM D HH mm ss` (MM = miesiąc). VizRS: `yyyy MM dd HH H mm ss`. */
export function mapClockFormat(f, warn = () => {}) {
  const s = String(f || 'HH:MM:SS');
  if (s === 'HH:MM') return 'HH:mm';
  if (s === 'HH:MM:SS') return 'HH:mm:ss';
  return s.replace(/dddd|MMMM|YYYY|DD|MM|D|HH|mm|ss/g, (t) => {
    const m = { YYYY: 'yyyy', DD: 'dd', MM: 'MM', HH: 'HH', mm: 'mm', ss: 'ss' }[t];
    if (m) return m;
    warn(`token zegara „${t}” nie ma odpowiednika w VizRS – pominięty`);
    return t === 'D' ? 'dd' : '';
  });
}

// ---------------------------------------------------------------- ZIP (.cgpack)
/** Minimalny czytnik ZIP (stored + deflate przez DecompressionStream). Limity jak w BTB `pack-io.js`. */
export async function unzip(buf) {
  try { return await unzipRaw(buf); } catch (e) { if (e instanceof RangeError) throw new Error('uszkodzony plik ZIP'); throw e; }
}
async function unzipRaw(buf) {
  const u8 = buf instanceof Uint8Array ? buf : new Uint8Array(buf);
  const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
  let eocd = -1;
  for (let i = u8.length - 22; i >= Math.max(0, u8.length - 65557); i--) if (dv.getUint32(i, true) === 0x06054b50) { eocd = i; break; }
  if (eocd < 0) throw new Error('to nie jest plik ZIP (.cgpack)');
  const count = dv.getUint16(eocd + 10, true);
  let p = dv.getUint32(eocd + 16, true);
  if (count > 200) throw new Error('paczka: więcej niż 200 plików');
  const files = {};
  let total = 0;
  for (let n = 0; n < count; n++) {
    if (dv.getUint32(p, true) !== 0x02014b50) throw new Error('uszkodzony katalog ZIP');
    const method = dv.getUint16(p + 10, true), csize = dv.getUint32(p + 20, true), usize = dv.getUint32(p + 24, true);
    const nlen = dv.getUint16(p + 28, true), elen = dv.getUint16(p + 30, true), clen = dv.getUint16(p + 32, true), off = dv.getUint32(p + 42, true);
    const name = new TextDecoder().decode(u8.subarray(p + 46, p + 46 + nlen));
    p += 46 + nlen + elen + clen;
    if (name.endsWith('/')) continue;
    if (name.includes('..') || name.startsWith('/') || name.includes('\\')) throw new Error('paczka: niedozwolona ścieżka ' + name);
    if (usize > 200 * 1024 * 1024 || (total += usize) > 256 * 1024 * 1024) throw new Error('paczka: za duża');
    const lh = off, nl = dv.getUint16(lh + 26, true), el = dv.getUint16(lh + 28, true);
    const raw = u8.subarray(lh + 30 + nl + el, lh + 30 + nl + el + csize);
    if (method === 0) files[name] = raw.slice();
    else if (method === 8) {
      const ds = new DecompressionStream('deflate-raw');
      const out = new Response(new Blob([raw]).stream().pipeThrough(ds));
      files[name] = new Uint8Array(await out.arrayBuffer());
    } else throw new Error('paczka: nieobsługiwana kompresja ' + method);
  }
  return files;
}

/** Wczytuje plik BTB (`.json` jako tekst albo `.cgpack` jako bajty) → { elements, files }. */
export async function readBtb(input, name = '') {
  if (typeof input === 'string') return { elements: normalizeElements(JSON.parse(input)), files: {} };
  const u8 = input instanceof Uint8Array ? input : new Uint8Array(input);
  if (u8[0] === 0x50 && u8[1] === 0x4b) {
    const files = await unzip(u8);
    if (!files['element.json']) throw new Error('paczka bez element.json');
    const el = JSON.parse(new TextDecoder().decode(files['element.json']));
    return { elements: normalizeElements(el), files };
  }
  return { elements: normalizeElements(JSON.parse(new TextDecoder().decode(u8))), files: {} };
}

export function normalizeElements(j) {
  if (Array.isArray(j)) return j;
  if (j && Array.isArray(j.elements)) return j.elements;
  if (j && Array.isArray(j.graphics)) return j.graphics;
  if (j && typeof j === 'object' && (j.scene || j.html_template)) return [j];
  throw new Error('to nie jest plik elementu (oczekiwano eksportu v3, elementu albo .cgpack)');
}

// ---------------------------------------------------------------- animacje (ruch BTB → node.motion)
/** Krzywa BTB (CSS albo GSAP) → nazwa GSAP rozumiana przez web/motion.js (jak CGMotion.normalizeEase). */
const CSS_EASE = {
  'ease-in-sine': 'sine.in', 'ease-out-sine': 'sine.out', 'ease-in-out-sine': 'sine.inOut', 'ease-in-quad': 'quad.in', 'ease-out-quad': 'quad.out',
  'ease-in-out-quad': 'quad.inOut', 'ease-in-cubic': 'cubic.in', 'ease-out-cubic': 'cubic.out', 'ease-in-out-cubic': 'cubic.inOut', 'bounce-in': 'bounce.in',
  'bounce-in-out': 'bounce.out', 'elastic-in': 'elastic.in', 'elastic-in-out': 'elastic.out', 'back-in-out': 'back.inOut', ease: 'power2.inOut',
};
export function motionEase(e) {
  const s = String(e ?? '').trim();
  if (!s) return undefined;
  const v = CSS_EASE[s] || s;
  return v.length <= 60 && /^[A-Za-z0-9.(),\- ]+$/.test(v) ? v : undefined;
}
const BTB_TEXT_ALIAS = { blurIn: 'blur', blurOut: 'blur', slideReveal: 'slide-up', slideHide: 'hide' };
const rnd = (v) => Math.round(v * 1000) / 1000;
/** Preset IN/OUT warstwy/elementu BTB → motion.in/out; nieznany typ → `fallback` z BTB albo przenikanie. */
export function motionSpec(m, warn, where) {
  if (!m || !m.type) return null;
  let type = m.type;
  if (!MO.ANIM_TYPES.includes(type)) {
    const fb = m.fallback;
    if (MO.ANIM_TYPES.includes(fb)) { warn(`${where}: animacja „${type}” zastąpiona „${fb}”`); type = fb; }
    else { warn(`${where}: animacja „${type}” nie ma odpowiednika – użyto przenikania`); type = 'fade'; }
  }
  const o = { type };
  if (MO.DIRECTIONS.includes(m.direction)) o.dir = m.direction;
  o.dur = rnd(Math.min(60, Math.max(0, num(m.duration, 0.5))));
  if (num(m.delay) > 0) o.delay = rnd(Math.min(60, num(m.delay)));
  const e = motionEase(m.ease);
  if (e) o.ease = e;
  return o;
}
/** Efekt tekstu BTB (`motion.text` / `textOut`) → motion.textIn/textOut. */
export function textSpec(t, kind, warn, where) {
  if (!t || !t.type || t.type === 'none') return null;
  const list = kind === 'in' ? MO.TEXT_IN : MO.TEXT_OUT;
  let type = BTB_TEXT_ALIAS[t.type] || t.type;
  if (!list.includes(type)) {
    const fb = BTB_TEXT_ALIAS[t.fallback] || t.fallback;
    if (list.includes(fb)) type = fb;
    else { warn(`${where}: efekt tekstu „${t.type}” nie ma odpowiednika – użyto przenikania`); type = 'fade'; }
  }
  // domyślne jak w cgmotion: tekst IN – 1 s po 0,2 s, tekst OUT – 0,5 s bez opóźnienia
  const o = { type, dur: rnd(Math.min(60, Math.max(0, num(t.duration, kind === 'in' ? 1 : 0.5)))) };
  const delay = t.delay == null || t.delay === '' ? (kind === 'in' ? 0.2 : 0) : num(t.delay);
  if (delay > 0) o.delay = rnd(Math.min(60, delay));
  if (num(t.stagger) > 0) o.stagger = rnd(Math.min(60, num(t.stagger)));
  if (MO.STAGGER_FROM.includes(t.from)) o.from = t.from;
  const e = motionEase(t.ease);
  if (e) o.ease = e;
  return o;
}
/** Pętla uwagi BTB → motion.loop. */
export function loopSpec(l) {
  if (!l || !l.type || l.type === 'none' || !MO.LOOP_TYPES.includes(l.type)) return null;
  const o = { type: l.type };
  if (num(l.duration) > 0) o.dur = rnd(Math.min(60, num(l.duration)));
  if (l.amplitude !== undefined && l.amplitude !== '' && num(l.amplitude, -1) >= 0) o.amp = rnd(Math.min(1000, num(l.amplitude)));
  if (typeof l.color === 'string' && l.color.length <= 60) o.color = l.color;
  return o;
}

// ---------------------------------------------------------------- tickery (import z BTB)
const MODE_IN = { horizontal: 'crawl', scrolling: 'crawl', vertical: 'roll', whip: 'whip' };
const SEP_IN = { skewed: 'skew', dot: 'dot', square: 'square', pipe: 'pipe' };
/** Wiadomości elementu BTB (stringi albo {text, category}) → wiersze `KAT|treść`. */
export function btbItemsText(items) {
  return serializeTickerItems((items || []).map((i) => (i && typeof i === 'object' ? { cat: String(i.category || ''), color: '', text: String(i.text || '') } : { cat: '', color: '', text: String(i ?? '') })));
}
/** Ustawienia tickera z elementu BTB (tryb, prędkość, etykieta, kolory kategorii, separator, wiper). */
function tickerSettings(el, layerBased) {
  const p = {};
  const f = el.fields || {};
  const mode = layerBased ? 'crawl' : MODE_IN[el.tickerMode] || (el.features?.wiper || el.features?.ticker_mode ? 'whip' : 'crawl');
  p.tickerMode = mode;
  if (mode === 'crawl') p.speed = num(el.speed, 120);
  else p.hold = mode === 'roll' ? Math.max(1, num(el.speed, 5)) : 6;
  const label = el.introText ?? f.INTROTEXT ?? f.introText ?? '';
  if (label && !layerBased) p.label = String(label);
  const cc = f.CATEGORY_COLORS ?? (el.ocgInputs || []).find((i) => i.id === 'CATEGORY_COLORS')?.default;
  if (cc) p.catColors = String(cc);
  if (el.separatorStyle) { if (SEP_IN[el.separatorStyle]) p.sepStyle = SEP_IN[el.separatorStyle]; else if (el.separatorStyle === 'none') p.sep = '   '; }
  const w = el.wiper || {};
  if (w.bgColor) p.catBg = toHex(w.bgColor);
  if (w.textColor) p.catFg = toHex(w.textColor);
  if (el.features?.wiper && w.gleamEnabled !== false && !layerBased) p.gleam = round3(num(w.gleamDuration, 2) + num(w.gleamFrequency, 3));
  return p;
}

// ---------------------------------------------------------------- import: element BTB → scena VizRS
/**
 * @param el   element BTB (eksport v3)
 * @param opts { knownFonts?: string[], assetName?: (src)=>string|null }
 * @returns { scene, page: { name, values }, warnings: string[] }
 */
export function convertElement(el, opts = {}) {
  const warnings = [];
  const seen = new Set();
  const warn = (m) => { if (!seen.has(m)) { seen.add(m); warnings.push(m); } };
  const sc = el.scene;
  if (!sc || !Array.isArray(sc.layers)) throw new Error(`element „${el.name || el.id}”: brak sceny graficznej (kod ręczny HTML/CSS/JS nie jest obsługiwany – w edytorze źródłowym użyj „Przekształć w scenę”)`);
  const W = num(sc.frame?.w, el.layout?.width || 1920), H = num(sc.frame?.h, el.layout?.height || 110);
  const lay = el.layout || {};
  const scale = num(lay.scale, 1) || 1;
  // położenie ramki elementu na polu 1920×1080
  let L, T;
  if (lay.side === 'custom' || (lay.x !== undefined && lay.y !== undefined && !lay.side)) { L = num(lay.x); T = num(lay.y); }
  else {
    const mx = num(lay.marginX), my = num(lay.marginY);
    L = lay.side === 'right' ? 1920 - mx - W * scale : lay.side === 'center' ? (1920 - W * scale) / 2 : mx;
    T = 1080 - my - H * scale;
    if (lay.side && !['left', 'right', 'center', 'custom'].includes(lay.side)) warn(`położenie „${lay.side}” przybliżone (lewy dolny róg z marginesami)`);
  }
  if (lay.dock) warn('powiązania layout.dock nie są przenoszone – użyj sekcji „Powiązania” w Artist');
  if (scale !== 1) warn(`skala elementu ${scale} przeliczona na skalę grupy`);
  // skala w VizRS działa wokół środka; zachowujemy lewy górny róg
  const gx = round3(L + (W * scale) / 2 - W / 2), gy = round3(T + (H * scale) / 2 - H / 2);
  const idMap = new Map();
  const nodes = [];
  const fields = {};
  const fieldMeta = {};
  const fieldsList = el.ocgInputs || [];
  for (const f of fieldsList) {
    const id = String(f.id || '').toLowerCase().replace(/[^\w.\-]/g, '_');
    if (!id) continue;
    const meta = { label: f.label || f.id };
    const tx = f.text || {};
    if (tx.maxChars) meta.maxChars = tx.maxChars;
    if (tx.softMax) meta.softMax = tx.softMax;
    if (tx.maxLines) meta.maxLines = tx.maxLines;
    if (tx.singleLine) meta.oneLine = true;
    if (tx.upperCase) meta.upper = true;
    if (tx.typography?.pl) meta.typo = true;
    if (f.tab) meta.tab = f.tab;
    if (f.help) meta.help = f.help;
    if (f.readonly) meta.readonly = true;
    fieldMeta[id] = meta;
    fields[id] = f.default ?? '';
  }

  // --- ticker z szablonu kodowego przekształconego w scenę: warstwa ticker zginęła, został statyczny tekst pierwszej wiadomości.
  // Odtwarzamy węzeł ticker z `items` i ustawień elementu; wiper (kształt + tekst etykiety) przejmuje pas kategorii tickera.
  // tylko typ TICKER: BTB zapisuje domyślne `items` („Wiadomość 1/2”) także w belkach, plakietkach i obrazach
  const isTickerEl = el.type === 'TICKER';
  let tkPlan = null;
  if (isTickerEl && !sc.layers.some((l) => l.type === 'ticker')) {
    const first = String((el.items || [])[0]?.text ?? (el.items || [])[0] ?? '').trim();
    const msg = sc.layers.find((l) => l.type === 'text' && !l.field && first && String(l.text || l.defaultText || '').trim() === first)
      || sc.layers.find((l) => l.type === 'text' && !l.field && /msg|news|wiadomo|tre[sś][cć]/i.test(l.name || ''));
    if (msg) {
      const wipers = sc.layers.filter((l) => /wiper/i.test(l.name || ''));
      const wShape = wipers.find((l) => l.type === 'shape' && !/::/.test(l.name || ''));
      const wText = sc.layers.find((l) => l.type === 'text' && (/wiper/i.test(l.name || '') || /^intro_?text$/i.test(l.field || '')));
      const drop = new Set([...wipers, wText].filter(Boolean).map((l) => l.id));
      const cont = sc.layers.filter((l) => l.type === 'shape' && !drop.has(l.id)).sort((a, b) => num(b.w) * num(b.h) - num(a.w) * num(a.h))[0];
      const left = wShape ? num(wShape.x) : num(msg.x), right = cont ? num(cont.x) + num(cont.w) : W;
      tkPlan = { msg, drop, wShape, wText, x: left, y: cont ? num(cont.y) : num(msg.y), w: Math.max(10, right - left), h: cont ? num(cont.h) : num(msg.h) };
      warn('ticker: kod szablonu (tryby whip/poziomy/pionowy) odtworzony jako węzeł ticker VizRS – sprawdź wygląd w Artist');
    } else warn('element TICKER bez warstwy ticker i bez tekstu wiadomości – wiadomości trafiły tylko do pola strony');
  }

  // --- warstwy
  let freeText = 0; // warstwy tekstu bez pola BTB → pola tekst1, tekst2…
  const layerById = new Map(sc.layers.map((l) => [l.id, l]));
  const toNode = (l) => {
    const name = String(l.name || l.id).replace(/^(Kształt|Tekst|Obraz|Ticker|Zegar)\s*—\s*/i, '') || l.id;
    const node = { id: l.id, type: 'rect', name: `${name}`.slice(0, 60), props: { x: round3(num(l.x)), y: round3(num(l.y)), w: round3(num(l.w)), h: round3(num(l.h)) }, anim: {}, fields: {} };
    const p = node.props;
    if (tkPlan?.msg === l) {
      // tekst wiadomości → ticker na całej szerokości paska (od wipera do prawej krawędzi tła)
      Object.assign(p, { x: round3(tkPlan.x), y: round3(tkPlan.y), w: round3(tkPlan.w), h: round3(tkPlan.h) });
      node.name = 'ticker';
      tickerNode(node, l.font || {}, false);
      if (tkPlan.wShape) {
        p.catW = round3(num(tkPlan.wShape.w));
        if (!p.catBg) p.catBg = toHex(tkPlan.wShape.fill?.color || '#c8102e');
      }
      if (tkPlan.wText) {
        const wf = tkPlan.wText.font || {};
        if (!p.catFg && wf.color) p.catFg = toHex(wf.color);
        if (num(wf.size)) p.labelSize = wf.size;
        if (wf.weight && weightOf(wf.weight) !== p.weight) p.labelWeight = weightOf(wf.weight);
        if (tkPlan.wText.field) { const lf = String(tkPlan.wText.field).toLowerCase().replace(/[^\w.\-]/g, '_'); node.fields.label = lf; if (!(lf in fields)) fields[lf] = p.label || ''; fieldMeta[lf] ||= { label: 'Etykieta' }; }
      }
      idMap.set(l.id, node);
      return node;
    }
    if (num(l.rotation)) p.rotation = l.rotation;
    if (num(l.skew)) p.skewX = l.skew;
    if (l.opacity !== undefined && l.opacity !== 1) p.opacity = l.opacity;
    if (l.visible === false) node.hidden = true;
    if (l.maskLayer) warn(`maska warstwy „${name}” nie jest przenoszona`);
    if (l.effect && l.effect !== 'none' && !(l.type === 'shape' && MO.BG_TYPES.includes(l.effect))) warn(`efekt „${l.effect}” (warstwa „${name}”) pominięty`);
    switch (l.type) {
      case 'shape': {
        node.type = 'rect';
        const f = l.fill || {};
        p.color = toHex(f.color || '#ffffff');
        if (f.type === 'gradient' && f.color2) { p.color2 = toHex(f.color2); p.gradAngle = num(f.angle, 90); }
        if (num(l.radius)) p.radius = l.radius;
        if (l.stroke && num(l.stroke.width) > 0) { p.borderWidth = l.stroke.width; p.borderColor = toHex(l.stroke.color || '#ffffff'); }
        if (l.shadow) p.shadow = 20;
        if (l.fitText && layerById.has(l.fitText.to)) {
          const tgt = layerById.get(l.fitText.to);
          p.follow = tgt.id; p.followAxis = 'w'; p.followPad = Math.max(0, round3(num(l.w) - num(tgt.w))); p.followMin = 0;
          if (l.fitText.mode === 'follow') warn(`dopasowanie „follow” (warstwa „${name}”) zastąpiono rozciąganiem`);
        }
        break;
      }
      case 'text': {
        node.type = 'text';
        const f = l.font || {};
        p.text = String(l.defaultText || l.text || '');
        p.font = f.family || 'Inter'; p.size = num(f.size, 40); p.weight = weightOf(f.weight);
        p.color = toHex(f.color || '#ffffff');
        if (f.italic) p.italic = true;
        if (f.transform === 'uppercase') p.upper = true;
        p.align = f.align || 'left'; p.valign = f.valign || 'middle';
        if (num(f.letterSpacing)) p.spacing = f.letterSpacing;
        p.lineHeight = num(f.lineHeight, 1.15);
        if (f.shadow && f.shadow !== 'none') p.shadow = 8;
        // ściskanie jak w BTB (renderer: tryb `squeeze` – tylko scaleX do granicy, font bez zmian)
        p.textFit = l.squash ? 'squeeze' : 'none';
        // granica ściśnięcia jak w BTB: 0 = bez granicy (w VizRS najmniejsze dozwolone 0,3); brak granicy w zapisie to element
        // sprzed M7 – BTB migruje go (upgrade-minsquash.js) do minSquash 0, więc też bez granicy (nowe warstwy BTB zapisują 0,8 jawnie)
        if (l.squash) p.squeezeMin = num(l.minSquash) > 0 ? Math.max(0.3, Math.min(1, num(l.minSquash))) : 0.3;
        // wnętrze tła pod tekstem: od lewej krawędzi tekstu do prawej krawędzi tła minus ten sam margines co z lewej
        const plateInner = () => {
          const plate = sc.layers.filter((s) => s.type === 'shape' && !s.fitText && num(s.x) <= num(l.x) && num(s.x) + num(s.w) > num(l.x))
            .sort((a, b) => num(b.w) - num(a.w))[0];
          return plate ? num(plate.x) + num(plate.w) - num(l.x) - (num(l.x) - num(plate.x)) : 0;
        };
        if (l.autoW) {
          // BTB: warstwa rośnie z tekstem do maxW (tło za nią); dłuższy tekst ściskamy w maxW, zamiast wypychać plakietkę poza ekran
          p.w = round3(num(l.maxW, num(l.w)));
          if (!l.squash) { p.textFit = 'squeeze'; p.squeezeMin = 0.5; }
        }
        if (l.squash && el.type === 'BADGE' && !l.autoW) {
          // BADGE w BTB ściska tekst wg długości (> 25 znaków → 25/len, min. 0,5), a nie do szerokości warstwy;
          // w VizRS: ściskanie do wnętrza tła z tym samym marginesem, z tą samą granicą 0,5
          p.squeezeMin = 0.5;
          const inner = plateInner();
          if (inner > 0 && inner < num(l.w)) p.w = round3(inner);
          warn(`ściskanie tekstu „${name}” wg długości (BADGE) zastąpiono ściskaniem do szerokości ${inner > 0 && inner < num(l.w) ? `tła (${p.w} px)` : 'warstwy'}, granica 50%`);
        }
        let fid = l.field ? String(l.field).toLowerCase().replace(/[^\w.\-]/g, '_') + (l.line ? `_l${l.line}` : '') : '';
        if (!fid) {
          // BTB edytuje takie warstwy w edytorze WYSIWYG; w VizRS operator zmienia tekst w polu strony
          freeText++;
          fid = 'tekst' + freeText;
          while (fid in fields) fid += '_';
          fieldMeta[fid] = { label: `Tekst ${freeText}` };
          // szerokość = szerokość tekstu przykładowego z BTB; przy wyrównaniu do lewej rozszerzamy do krawędzi tła,
          // żeby dłuższy tekst nie był ściskany do szerokości przykładu (przy środku/prawej zmieniłoby to położenie)
          const inner = p.align === 'left' && !l.autoW ? plateInner() : 0;
          if (inner > num(l.w) + 1) p.w = round3(inner);
          warn(`warstwa tekstu „${name}” bez pola w źródle – dodano pole „${fid}”${inner > num(l.w) + 1 ? `, szerokość do krawędzi tła (${p.w} px)` : ''}`);
        }
        if (fid) {
          node.fields.text = fid;
          if (!(fid in fields)) fields[fid] = p.text;
          if (!fieldMeta[fid]) fieldMeta[fid] = { label: l.field };
        }
        if (opts.knownFonts && f.family && !opts.knownFonts.includes(f.family)) warn(`font „${f.family}” niedostępny w VizRS – wgraj plik fontu w Artist albo zostanie użyty zastępczy`);
        break;
      }
      case 'image': case 'video': {
        node.type = 'image';
        const src = String(l.src || '');
        const mapped = opts.assetName ? opts.assetName(src) : null;
        const remote = /^https?:\/\//i.test(src);
        p.src = mapped || (remote ? src : src && !src.startsWith('data:') ? src.split('/').pop() : '');
        if (remote && !mapped) warn(`obraz „${name}” ładowany z sieci (${src}) – bez internetu nie pojawi się na antenie; wgraj plik w Artist`);
        else if (src && !mapped) warn(`plik „${src}” (warstwa „${name}”) nie został dołączony – wgraj go ręcznie w Artist`);
        p.fitMode = l.fit === 'cover' ? 'cover' : l.fit === 'fill' ? 'fill' : 'contain';
        if (num(l.radius)) p.radius = l.radius;
        if (l.bind) warn(`powiązanie obrazu z polem „${l.bind}” pominięte`);
        break;
      }
      case 'clock': {
        node.type = 'clock';
        const f = l.font || {};
        p.mode = 'time'; p.format = mapClockFormat(l.format, warn);
        p.font = f.family || 'Inter'; p.size = num(f.size, 32); p.weight = weightOf(f.weight); p.color = toHex(f.color || '#ffffff');
        p.align = f.align || 'left'; p.valign = f.valign || 'middle';
        break;
      }
      case 'ticker': {
        // BTB: „treść␣[odstęp gap]sep[odstęp gap]” – odstęp w px przybliżony spacjami (~0,26 em)
        const sep = String(l.separator ?? '•'), gapN = Math.max(1, Math.round(num(l.gap, 48) / (num(l.font?.size, 32) * 0.18)));
        p.sep = sep ? ' '.repeat(gapN + 1) + sep + ' '.repeat(gapN) : ' '.repeat(gapN);
        tickerNode(node, l.font || {}, true);
        if (!el.speed) p.speed = num(l.speed, 120);
        break;
      }
      default: warn(`nieznany typ warstwy „${l.type}” (${name}) – pominięta`); return null;
    }
    idMap.set(l.id, node);
    return node;
  };
  /** Węzeł ticker: wiadomości z `items` (z kategoriami) jako pole kontrolne `items`, ustawienia z elementu. */
  function tickerNode(node, f, layerBased) {
    const p = node.props;
    node.type = 'ticker';
    Object.assign(p, tickerSettings(el, layerBased));
    // warstwa ticker sceny BTB pokazuje same treści (bez kategorii) i startuje z taśmą od lewej krawędzi
    const its = layerBased ? (el.items || []).map((i) => (i && typeof i === 'object' ? String(i.text || '') : String(i ?? ''))) : el.items;
    p.text = btbItemsText(its) || 'Wiadomość 1\nWiadomość 2';
    if (layerBased) p.crawlFrom = 'left';
    p.font = f.family || 'Inter'; p.size = num(f.size, 32); p.weight = weightOf(f.weight); p.color = toHex(f.color || '#ffffff');
    if (f.transform === 'uppercase') p.upper = true;
    if (opts.knownFonts && f.family && !opts.knownFonts.includes(f.family)) warn(`font „${f.family}” niedostępny w VizRS – wgraj plik fontu w Artist albo zostanie użyty zastępczy`);
    if (!p.catW && parseTickerItems(p.text).some((x) => x.cat) && p.tickerMode !== 'crawl') p.catW = 170;
    node.fields.text = 'items';
    fields.items = p.text;
    fieldMeta.items ||= { label: 'Wiadomości' };
  }
  for (const l of sc.layers) { if (tkPlan?.drop.has(l.id)) continue; const n = toNode(l); if (n) nodes.push(n); }

  // --- animacje: ruch BTB (presety IN/OUT, efekty tekstu, pętle, „po warstwie”) → node.motion 1:1 (web/motion.js)
  const group = { id: 'grp_' + (el.id || 'el').slice(0, 8), type: 'group', name: String(el.name || 'element').slice(0, 60), props: { x: gx, y: gy, w: W, h: H }, anim: {}, fields: {}, children: nodes };
  if (scale !== 1) group.props.scale = scale;
  const an = el.animation || {};
  const setMotion = (node, k, v) => { if (v) (node.motion ||= {})[k] = v; };
  setMotion(group, 'in', motionSpec(an.in, warn, 'element'));
  setMotion(group, 'out', motionSpec(an.out, warn, 'element'));
  setMotion(group, 'loop', loopSpec(an.loop));
  const textLayers = sc.layers.filter((l) => l.type === 'text' && idMap.get(l.id));
  const afterIds = new Map(); // węzeł → id warstwy BTB, po której startuje (dla wejścia)
  for (const l of sc.layers) {
    const n = idMap.get(l.id); if (!n) continue;
    const m = l.motion || {};
    const where = `warstwa „${n.name}”`;
    setMotion(n, 'in', motionSpec(m.in, warn, where));
    setMotion(n, 'out', motionSpec(m.out, warn, where));
    setMotion(n, 'loop', loopSpec(m.loop));
    if (l.type === 'text') {
      setMotion(n, 'textIn', textSpec(m.text, 'in', warn, `tekst „${n.name}”`));
      setMotion(n, 'textOut', textSpec(m.textOut, 'out', warn, `tekst „${n.name}”`));
    }
    const af = m.in?.after || m.text?.after;
    if (af) afterIds.set(n, af);
    if (m.out?.after || m.textOut?.after) warn(`${where}: „po warstwie” przy wyjściu zamienione na równoczesny start`);
  }
  // efekt tekstu elementu (an.text / an.textOut) – dla warstw tekstu bez własnego
  for (const l of textLayers) {
    const n = idMap.get(l.id);
    if (!n.motion?.textIn) setMotion(n, 'textIn', textSpec(an.text, 'in', warn, 'tekst elementu'));
    if (!n.motion?.textOut) setMotion(n, 'textOut', textSpec(an.textOut, 'out', warn, 'tekst elementu'));
  }
  for (const [n, id] of afterIds) { const tg = idMap.get(id); if (tg && tg !== n) n.motion.after = tg.id; }
  // stałe efekty tła kształtów (shimmer, pulse, glow, scanline)
  for (const l of sc.layers) { const n = idMap.get(l.id); if (n && l.type === 'shape' && MO.BG_TYPES.includes(l.effect) && l.effect !== 'none') setMotion(n, 'bg', l.effect); }
  // ruch całego elementu w BTB działa na `.cg-scene` (punkt odniesienia: lewy górny róg) wewnątrz kontenera z położeniem i skalą layoutu,
  // więc przesunięcia wjazdu też są skalowane; przy skali ≠ 1 ruch idzie do wewnętrznej grupy (zewnętrzna = layout)
  let root = group;
  if (group.motion) {
    group.motion.origin = [0, 0];
    if (scale !== 1) {
      const inner = { id: group.id + '_ruch', type: 'group', name: 'ruch elementu', props: { x: 0, y: 0, w: W, h: H }, anim: {}, fields: {}, children: nodes, motion: group.motion };
      delete group.motion;
      group.children = [inner];
      root = group;
    }
  }
  // wejście elementu: warstwy startują po nim (w BTB faza elementu i warstw biegnie razem – zostawiamy równolegle)
  const all = [group, ...(group.children[0]?.id === group.id + '_ruch' ? [group.children[0]] : []), ...nodes];
  const shifts = MO.sequenceShifts([root]);
  let stop = 0, outL = 0;
  for (const n of all) {
    if (!n.motion) continue;
    const tx = n.type === 'text' ? fields[n.fields?.text] ?? n.props.text : undefined;
    stop = Math.max(stop, MO.inEnd(n.motion, shifts.get(n.id) || 0, tx));
    outL = Math.max(outL, MO.outLen(n.motion, tx));
  }
  const hasMotion = all.some((n) => n.motion);
  if (hasMotion && stop <= 0) stop = 0.04; // stop jest potrzebny, by zagrało wyjście i pętle
  const end = stop + outL;
  for (const n of all) if (!Object.keys(n.anim).length) delete n.anim;
  const scene = {
    name: String(el.name || 'Element').slice(0, 100), layer: 'main', width: 1920, height: 1080,
    duration: round3(Math.max(end, stop + 0.01, 0.05)), stops: stop > 0 ? [round3(stop)] : [], fieldMeta, nodes: [group],
  };
  const values = {};
  for (const [id, v] of Object.entries(fields)) values[id] = typeof v === 'string' ? v : String(v ?? '');
  if (el.title && values.title !== undefined && !values.title) values.title = el.title;
  return { scene, page: { name: scene.name, values }, warnings };
}
