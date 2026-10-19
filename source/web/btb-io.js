// VizRS ↔ BTB CG: konwersja elementów (F4.2).
//   import: BTB `.json` (eksport v3, goły element lub lista) i `.cgpack` (ZIP) → scena + strona VizRS, z raportem „co się nie przeniosło”
//   eksport: scena VizRS → element BTB (`scene.layers`, kod HTML/JS BTB kompiluje sam `SceneModel` przy imporcie)
// Moduł bez DOM (jedyna zależność: ticker-model.js) – działa w przeglądarce i w Node (testy: tests/vectors/btb.js).
// Format: docs w BTB-CG `docs/edytor-kontrakt.md` (scene = { version, detached, frame:{w,h}, layers[] }).

import { serializeTickerItems, parseTickerItems } from './ticker-model.js';

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
  throw new Error('to nie jest plik elementu BTB CG (oczekiwano eksportu v3, elementu albo .cgpack)');
}

// ---------------------------------------------------------------- animacje (presety BTB → klucze VizRS)
/** Opis ruchu: lista {prop, a, b} (wartość na początku i na końcu odcinka) albo null (nieobsługiwany). */
function motionTracks(kind, m, box, rest, warn, where) {
  const type = m.type;
  const dir = m.direction || 'bottom';
  const inn = kind === 'in';
  const tr = [];
  const slide = () => {
    const dist = { top: ['y', -(box.h || 100)], bottom: ['y', box.h || 100], left: ['x', -(box.w || 100)], right: ['x', box.w || 100] }[dir] || ['y', box.h || 100];
    const [p, off] = dist;
    tr.push(inn ? { prop: p, a: rest[p] + off, b: rest[p] } : { prop: p, a: rest[p], b: rest[p] + off });
  };
  const fade = () => tr.push(inn ? { prop: 'opacity', a: 0, b: rest.opacity } : { prop: 'opacity', a: rest.opacity, b: 0 });
  const clip = () => {
    const prop = { left: 'clipR', right: 'clipL', top: 'clipB', bottom: 'clipT' }[dir] || 'clipR';
    tr.push(inn ? { prop, a: 100, b: 0 } : { prop, a: 0, b: 100 });
  };
  switch (type) {
    case 'slide': slide(); break;
    case 'fade': fade(); break;
    case 'zoom': tr.push(inn ? { prop: 'scale', a: 0.85, b: rest.scale } : { prop: 'scale', a: rest.scale, b: 0.85 }); fade(); break;
    case 'pop': tr.push(inn ? { prop: 'scale', a: 0.6, b: rest.scale, e: 'back' } : { prop: 'scale', a: rest.scale, b: 0.6 }); fade(); break;
    case 'blur': tr.push(inn ? { prop: 'blur', a: 14, b: 0 } : { prop: 'blur', a: 0, b: 14 }); fade(); break;
    case 'wipe': case 'clip': clip(); break;
    case 'none': case undefined: case null: case '': return [];
    default: warn(`${where}: animacja „${type}” nie ma odpowiednika – użyto przenikania`); fade();
  }
  return tr;
}

function textTracks(kind, t, warn, where) {
  if (!t || !t.type || t.type === 'none') return [];
  const inn = kind === 'in';
  const mk = (prop) => [inn ? { prop, a: 100, b: 0 } : { prop, a: 0, b: 100 }];
  if (t.type === 'reveal') return mk('clipR');
  if (t.type === 'clip-up') return mk('clipT');
  if (t.type === 'clip-down') return mk('clipB');
  if (t.type === 'clip-left') return mk('clipR');
  if (t.type === 'clip-right') return mk('clipL');
  warn(`${where}: efekt tekstu „${t.type}” nie ma odpowiednika – użyto przenikania`);
  return [inn ? { prop: 'opacity', a: 0, b: 1 } : { prop: 'opacity', a: 1, b: 0 }];
}

// ---------------------------------------------------------------- tickery (BTB ↔ VizRS)
const MODE_IN = { horizontal: 'crawl', scrolling: 'crawl', vertical: 'roll', whip: 'whip' };
const MODE_OUT = { crawl: 'horizontal', roll: 'vertical', wipe: 'whip', whip: 'whip' };
const SEP_IN = { skewed: 'skew', dot: 'dot', square: 'square', pipe: 'pipe' };
const SEP_OUT = { skew: 'skewed', dot: 'dot', square: 'square', pipe: 'pipe', text: 'dot' };
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
  if (!sc || !Array.isArray(sc.layers)) throw new Error(`element „${el.name || el.id}”: brak sceny graficznej (kod ręczny HTML/CSS/JS nie jest obsługiwany – w BTB użyj „Przekształć w scenę”)`);
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
  if (lay.dock) warn('powiązania layout.dock (M4 BTB) nie są przenoszone – użyj sekcji „Powiązania” w Artist');
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
  const isTickerEl = el.type === 'TICKER' || (Array.isArray(el.items) && el.items.length > 0);
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
      warn('ticker: kod szablonu BTB (tryby whip/poziomy/pionowy) odtworzony jako węzeł ticker VizRS – sprawdź wygląd w Artist');
    } else warn('element TICKER bez warstwy ticker i bez tekstu wiadomości – wiadomości trafiły tylko do pola strony');
  }

  // --- warstwy
  const rest = {};
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
        if (!p.catFg && tkPlan.wText.font?.color) p.catFg = toHex(tkPlan.wText.font.color);
        if (tkPlan.wText.field) { const lf = String(tkPlan.wText.field).toLowerCase().replace(/[^\w.\-]/g, '_'); node.fields.label = lf; if (!(lf in fields)) fields[lf] = p.label || ''; fieldMeta[lf] ||= { label: 'Etykieta' }; }
      }
      rest[l.id] = { x: p.x, y: p.y, opacity: p.opacity ?? 1, scale: p.scale ?? 1 };
      idMap.set(l.id, node);
      return node;
    }
    if (num(l.rotation)) p.rotation = l.rotation;
    if (num(l.skew)) p.skewX = l.skew;
    if (l.opacity !== undefined && l.opacity !== 1) p.opacity = l.opacity;
    if (l.visible === false) node.hidden = true;
    if (l.maskLayer) warn(`maska warstwy „${name}” nie jest przenoszona`);
    if (l.effect && l.effect !== 'none') warn(`efekt „${l.effect}” (warstwa „${name}”) pominięty`);
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
        p.textFit = l.squash ? 'squeeze' : 'none';
        if (l.squash && num(l.minSquash) > 0) p.squeezeMin = l.minSquash;
        if (l.autoW) { p.w = round3(num(l.maxW, num(l.w))); }
        if (l.field) {
          const fid = String(l.field).toLowerCase().replace(/[^\w.\-]/g, '_') + (l.line ? `_l${l.line}` : '');
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
        p.src = mapped || (src && !src.startsWith('data:') ? src.split('/').pop() : '');
        if (src && !mapped) warn(`plik „${src}” (warstwa „${name}”) nie został dołączony – wgraj go ręcznie w Artist`);
        p.fitMode = l.fit === 'cover' ? 'cover' : l.fit === 'fill' ? 'fill' : 'contain';
        if (num(l.radius)) p.radius = l.radius;
        if (l.bind) warn(`powiązanie obrazu z polem „${l.bind}” pominięte`);
        break;
      }
      case 'clock': {
        node.type = 'clock';
        const f = l.font || {};
        p.mode = 'time'; p.format = l.format || 'HH:mm:ss';
        p.font = f.family || 'Inter'; p.size = num(f.size, 32); p.weight = weightOf(f.weight); p.color = toHex(f.color || '#ffffff');
        p.align = f.align || 'left'; p.valign = f.valign || 'middle';
        break;
      }
      case 'ticker': {
        p.sep = `   ${l.separator || '•'}   `;
        tickerNode(node, l.font || {}, true);
        if (!el.speed) p.speed = num(l.speed, 120);
        break;
      }
      default: warn(`nieznany typ warstwy „${l.type}” (${name}) – pominięta`); return null;
    }
    rest[l.id] = { x: p.x, y: p.y, opacity: p.opacity ?? 1, scale: p.scale ?? 1 };
    idMap.set(l.id, node);
    return node;
  };
  /** Węzeł ticker: wiadomości z `items` (z kategoriami) jako pole kontrolne `items`, ustawienia z elementu. */
  function tickerNode(node, f, layerBased) {
    const p = node.props;
    node.type = 'ticker';
    Object.assign(p, tickerSettings(el, layerBased));
    p.text = btbItemsText(el.items) || 'Wiadomość 1\nWiadomość 2';
    p.font = f.family || 'Inter'; p.size = num(f.size, 32); p.weight = weightOf(f.weight); p.color = toHex(f.color || '#ffffff');
    if (f.transform === 'uppercase') p.upper = true;
    if (opts.knownFonts && f.family && !opts.knownFonts.includes(f.family)) warn(`font „${f.family}” niedostępny w VizRS – wgraj plik fontu w Artist albo zostanie użyty zastępczy`);
    if (!p.catW && parseTickerItems(p.text).some((x) => x.cat) && p.tickerMode !== 'crawl') p.catW = 170;
    node.fields.text = 'items';
    fields.items = p.text;
    fieldMeta.items ||= { label: 'Wiadomości' };
  }
  for (const l of sc.layers) { if (tkPlan?.drop.has(l.id)) continue; const n = toNode(l); if (n) nodes.push(n); }

  // --- animacje
  const ins = [], outs = []; // { node, tracks, delay, dur, ease }
  const addSpec = (list, node, tracks, m) => { if (tracks.length) list.push({ node, tracks, delay: num(m.delay), dur: Math.max(0.01, num(m.duration, 0.4)), ease: mapEase(m.ease) }); };
  const group = { id: 'grp_' + (el.id || 'el').slice(0, 8), type: 'group', name: String(el.name || 'element').slice(0, 60), props: { x: gx, y: gy, w: W, h: H }, anim: {}, fields: {}, children: nodes };
  if (scale !== 1) group.props.scale = scale;
  const groupRest = { x: gx, y: gy, opacity: 1, scale };
  const an = el.animation || {};
  if (an.in?.type && an.in.type !== 'none') addSpec(ins, group, motionTracks('in', an.in, { w: W, h: H }, groupRest, warn, 'element'), an.in);
  if (an.out?.type && an.out.type !== 'none') addSpec(outs, group, motionTracks('out', an.out, { w: W, h: H }, groupRest, warn, 'element'), an.out);
  if (an.loop?.type && an.loop.type !== 'none') warn(`pętla uwagi „${an.loop.type}” nie jest przenoszona`);
  for (const l of sc.layers) {
    const n = idMap.get(l.id); if (!n) continue;
    const m = l.motion || {};
    const r = rest[l.id], box = { w: n.props.w, h: n.props.h };
    if (m.in?.type && m.in.type !== 'none') addSpec(ins, n, motionTracks('in', m.in, box, r, warn, `warstwa „${n.name}”`), m.in);
    if (m.out?.type && m.out.type !== 'none') addSpec(outs, n, motionTracks('out', m.out, box, r, warn, `warstwa „${n.name}”`), m.out);
    if (l.type === 'text') {
      if (m.text) addSpec(ins, n, textTracks('in', m.text, warn, `tekst „${n.name}”`), { delay: m.text.delay, duration: m.text.duration, ease: m.text.ease });
      if (m.textOut) addSpec(outs, n, textTracks('out', m.textOut, warn, `tekst „${n.name}”`), { delay: m.textOut.delay, duration: m.textOut.duration, ease: m.textOut.ease });
    }
  }
  // tekstowe animacje elementu (an.text / an.textOut) – dla wszystkich warstw tekstowych
  if (an.text?.type && an.text.type !== 'none') for (const l of sc.layers) if (l.type === 'text' && idMap.get(l.id)) addSpec(ins, idMap.get(l.id), textTracks('in', an.text, warn, 'tekst elementu'), an.text);
  if (an.textOut?.type && an.textOut.type !== 'none') for (const l of sc.layers) if (l.type === 'text' && idMap.get(l.id)) addSpec(outs, idMap.get(l.id), textTracks('out', an.textOut, warn, 'tekst elementu'), an.textOut);

  const stop = ins.reduce((mx, s) => Math.max(mx, s.delay + s.dur), 0);
  let end = stop;
  const keysFor = (node) => node.anim;
  const push = (node, prop, k) => { (keysFor(node)[prop] ||= []).push(k); };
  for (const s of ins) for (const t of s.tracks) {
    push(s.node, t.prop, { t: round3(s.delay), v: round3(t.a), e: t.e || s.ease });
    push(s.node, t.prop, { t: round3(s.delay + s.dur), v: round3(t.b) });
  }
  for (const s of outs) {
    const st = stop + s.delay, en = st + s.dur;
    end = Math.max(end, en);
    for (const t of s.tracks) {
      push(s.node, t.prop, { t: round3(st), v: round3(t.a), e: t.e || s.ease });
      push(s.node, t.prop, { t: round3(en), v: round3(t.b) });
    }
  }
  const all = [group, ...nodes];
  for (const n of all) for (const [prop, tr] of Object.entries(n.anim)) {
    tr.sort((a, b) => a.t - b.t);
    // dwa klucze w tym samym czasie (koniec IN = początek OUT bez przerwy) → zostaw ostatni
    n.anim[prop] = tr.filter((k, i) => !(i < tr.length - 1 && Math.abs(tr[i + 1].t - k.t) < 0.0005));
  }
  const scene = {
    name: String(el.name || 'Element BTB').slice(0, 100), layer: 'main', width: 1920, height: 1080,
    duration: round3(Math.max(end, 0.05)), stops: stop > 0 ? [round3(stop)] : [], fieldMeta, nodes: [group],
  };
  if (!ins.length && !outs.length) { scene.duration = 0.05; }
  const values = {};
  for (const [id, v] of Object.entries(fields)) values[id] = typeof v === 'string' ? v : String(v ?? '');
  if (el.title && values.title !== undefined && !values.title) values.title = el.title;
  return { scene, page: { name: scene.name, values }, warnings };
}

// ---------------------------------------------------------------- eksport: scena VizRS → element BTB
/** Spłaszcza drzewo (grupy → pozycje bezwzględne) i buduje `scene.layers` BTB. */
export function exportScene(scene, values = {}) {
  const warnings = [];
  const warn = (m) => { if (!warnings.includes(m)) warnings.push(m); };
  const layers = [];
  const ocg = [];
  const used = new Set();
  const W = scene.width || 1920, H = scene.height || 1080;
  const strip = (s) => String(s ?? '').replace(/\{\/?(?:b|i|u|c|sz)(?:=[^}]*)?\}/gi, '').replace(/\[\[|\]\]/g, '');
  let minX = Infinity, minY = Infinity, maxX = 0, maxY = 0;
  let idc = 0;
  let ticker = null;
  const walkNodes = (nodes, ox, oy, depth) => {
    for (const n of nodes || []) {
      const p = n.props || {};
      const x = (+p.x || 0) + ox, y = (+p.y || 0) + oy, w = +p.w || 0, h = +p.h || 0;
      if (n.type === 'group') {
        if (p.rotation || (p.scale && p.scale !== 1)) warn(`grupa „${n.name}”: obrót/skala grupy nie są przenoszone`);
        walkNodes(n.children, x, y, depth + 1); continue;
      }
      if (n.hidden) { warn(`ukryty węzeł „${n.name}” pominięty`); continue; }
      const id = (n.id || '').replace(/[^\w-]/g, '') || 'l' + (++idc);
      const base = { id: used.has(id) ? id + '_' + (++idc) : id, name: n.name || id, visible: true, locked: false, x: round3(x), y: round3(y), w: round3(w), h: round3(h), rotation: +p.rotation || 0, skew: +p.skewX || 0, opacity: p.opacity ?? 1, motion: { in: null, out: null, text: null, textOut: null } };
      used.add(base.id);
      minX = Math.min(minX, x); minY = Math.min(minY, y); maxX = Math.max(maxX, x + w); maxY = Math.max(maxY, y + h);
      const fontOf = () => ({ family: p.font || 'Inter', size: +p.size || 40, weight: String(p.weight || '700'), color: p.color || '#ffffff', transform: p.upper ? 'uppercase' : 'none', align: p.align || 'left', valign: p.valign || 'middle', letterSpacing: +p.spacing || 0, lineHeight: +p.lineHeight || 1.15, italic: !!p.italic, shadow: p.shadow ? '0 2px 8px rgba(0,0,0,.6)' : 'none' });
      if (n.type === 'rect' || n.type === 'ellipse') {
        layers.push({ ...base, type: 'shape', fill: p.color2 ? { type: 'gradient', color: p.color || '#ffffff', color2: p.color2, angle: +p.gradAngle || 90 } : { type: 'solid', color: p.color || '#ffffff', color2: '', angle: 90 }, stroke: { color: p.borderColor || '#ffffff', width: +p.borderWidth || 0 }, radius: n.type === 'ellipse' ? Math.round(Math.min(w, h) / 2) : +p.radius || 0, shadow: (+p.shadow || 0) > 0, effect: 'none' });
        if (p.follow) warn(`autofollow „${n.name}” → pole fitText BTB wymaga ręcznego ustawienia`);
      } else if (n.type === 'text') {
        const f = n.fields?.text;
        const fid = f ? f.toUpperCase() : '';
        layers.push({ ...base, type: 'text', field: fid, text: strip(f ? (values[f] ?? p.text) : p.text), defaultText: '', font: fontOf(), squash: /squeeze/.test(p.textFit || ''), minSquash: +p.squeezeMin || 0 });
        if (f && !ocg.some((o) => o.id === fid)) {
          const m = scene.fieldMeta?.[f] || {};
          const o = { id: fid, label: m.label || f, default: strip(values[f] ?? p.text), type: 'text' };
          const text = {};
          if (m.maxChars) text.maxChars = m.maxChars;
          if (m.softMax) text.softMax = m.softMax;
          if (m.maxLines) text.maxLines = m.maxLines;
          if (m.oneLine) text.singleLine = true;
          if (m.upper) text.upperCase = true;
          if (m.typo) text.typography = { pl: true };
          if (Object.keys(text).length) o.text = text;
          if (m.tab) o.tab = m.tab;
          if (m.help) o.help = m.help;
          if (m.readonly) o.readonly = true;
          ocg.push(o);
        }
        if (/\{\/?(?:b|i|u|c|sz)|\[\[/i.test(String(p.text || ''))) warn(`formatowanie tekstu „${n.name}” ({b}, [[ ]]…) pominięte – w BTB ustaw je w Edytorze Tekstu`);
      } else if (n.type === 'image') {
        layers.push({ ...base, type: /\.(mp4|webm|mov)$/i.test(p.src || '') ? 'video' : 'image', src: p.src ? '/assets/' + p.src : '', fit: p.fitMode || 'contain', radius: +p.radius || 0, bind: '' });
        warn(`obraz „${p.src}”: skopiuj plik do BTB (ścieżka /assets/… z VizRS)`);
      } else if (n.type === 'clock') {
        layers.push({ ...base, type: 'clock', font: fontOf(), format: p.format || 'HH:mm:ss' });
      } else if (n.type === 'ticker') {
        layers.push({ ...base, type: 'ticker', font: fontOf(), speed: +p.speed || 120, separator: p.sepStyle && p.sepStyle !== 'text' ? '•' : String(p.sep || '•').trim() || '•', gap: 48 });
        if (!ticker) {
          const f = n.fields?.text;
          const mode = p.tickerMode || 'crawl';
          const src = parseTickerItems(String(f ? values[f] ?? p.text : p.text ?? ''));
          ticker = { items: src.map((x) => (x.cat ? { text: x.text, category: x.cat } : { text: x.text })), tickerMode: MODE_OUT[mode] || 'horizontal', speed: mode === 'crawl' ? +p.speed || 120 : +p.hold || 5 };
          if (p.label) ticker.introText = String(p.label);
          if (p.sepStyle && SEP_OUT[p.sepStyle] && p.sepStyle !== 'text') ticker.separatorStyle = SEP_OUT[p.sepStyle];
          if (p.catBg || p.catFg) ticker.wiper = { show: !!p.label, ...(p.catBg ? { bgColor: p.catBg } : {}), ...(p.catFg ? { textColor: p.catFg } : {}) };
          if (p.catColors) ticker.fields = { CATEGORY_COLORS: String(p.catColors) };
          if (mode !== 'crawl') warn(`ticker „${n.name}”: scena BTB przewija tylko poziomo – tryb „${mode}” zapisany w tickerMode (działa w szablonach kodowych BTB)`);
          if (src.some((x) => x.color)) warn(`ticker „${n.name}”: kolory w wiadomościach (KAT#kolor) pominięte – użyj kolorów kategorii`);
        } else warn(`ticker „${n.name}”: BTB ma jedną listę wiadomości na element – przeniesiono wiadomości pierwszego tickera`);
      } else warn(`węzeł „${n.name}” typu ${n.type} pominięty`);
    }
  };
  walkNodes(scene.nodes, 0, 0, 0);
  if (!layers.length) throw new Error('scena nie ma węzłów do wyeksportowania');
  // ramka elementu = obwiednia warstw; współrzędne warstw względem jej lewego górnego rogu
  const fx = Math.max(0, minX), fy = Math.max(0, minY);
  for (const l of layers) { l.x = round3(l.x - fx); l.y = round3(l.y - fy); }
  const fw = Math.max(1, Math.round(maxX - fx)), fh = Math.max(1, Math.round(maxY - fy));
  if ((scene.stops || []).length > 1) warn('wiele stop pointów: BTB nie ma Local Stop na wizji – przeniesiono tylko wejście/wyjście');
  if (scene.nodes.some((n) => hasKeys(n))) warn('klucze animacji nie są przenoszone – ustawiono domyślne wejście/wyjście (przenikanie 0,3 s)');
  const el = {
    id: scene.id ? 'vizrs-' + scene.id : 'vizrs-' + Date.now().toString(36), name: scene.name || 'Scena VizRS', type: 'LOWER_THIRD', title: '', subtitle: '', visible: false,
    layout: { side: 'custom', x: round3(fx), y: round3(fy), width: fw, height: fh, marginX: 0, marginY: 0, scale: 1, layer: 1 },
    animation: { in: { type: 'fade', direction: 'bottom', duration: 0.3, delay: 0, ease: 'ease-out' }, out: { type: 'fade', direction: 'bottom', duration: 0.3, delay: 0, ease: 'ease-in' } },
    ocgInputs: ocg, fields: {}, scene: { version: 1, detached: false, frame: { w: fw, h: fh }, layers },
  };
  if (ticker) { el.type = 'TICKER'; Object.assign(el, { ...ticker, fields: { ...el.fields, ...(ticker.fields || {}) } }); }
  return { file: { _exportVersion: 3, elements: [el] }, warnings };
}
function hasKeys(n) {
  if (Object.values(n.anim || {}).some((t) => t && t.length)) return true;
  return (n.children || []).some(hasKeys);
}
