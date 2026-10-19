import { qrSvg } from '/qr.js';
import { parseTickerItems, parseCatColors, catColorOf, labelOf, continuation, TICKER_MODES, SEP_STYLES } from '/ticker-model.js';
export { parseTickerItem, serializeTickerItem, parseTickerItems, serializeTickerItems, parseCatColors, checkTickerItems } from '/ticker-model.js';
import { hasMarkup, parseMarkup, stripMarkup, carryLines } from '/richtext.js';
export { hasMarkup, parseMarkup, stripMarkup, carryLines } from '/richtext.js';

// VizRS renderer – odtwarza sceny (drzewo kontenerów + keyframe'y + stop pointy)
// Wspólny dla: wyjścia OBS (/output), podglądu w Trio i edytora Artist.

// ------------------------------------------------------------------ właściwości
export const PROPS = {
  x: { t: 'num', d: 0, g: 'Transform', anim: 1 },
  y: { t: 'num', d: 0, g: 'Transform', anim: 1 },
  w: { t: 'num', d: 400, g: 'Transform', anim: 1, min: 0 },
  h: { t: 'num', d: 100, g: 'Transform', anim: 1, min: 0 },
  anchorX: { t: 'select', d: 'left', g: 'Transform', opts: ['left', 'center', 'right'], label: 'punkt X' },
  anchorY: { t: 'select', d: 'top', g: 'Transform', opts: ['top', 'middle', 'bottom'], label: 'punkt Y' },
  rotation: { t: 'num', d: 0, g: 'Transform', anim: 1, label: 'rotacja°' },
  scale: { t: 'num', d: 1, g: 'Transform', anim: 1, step: 0.01, label: 'skala' },
  opacity: { t: 'num', d: 1, g: 'Transform', anim: 1, step: 0.01, min: 0, max: 1, label: 'krycie' },
  clipL: { t: 'num', d: 0, g: 'Maska / efekty', anim: 1, min: 0, max: 100, label: 'maska L%' },
  clipR: { t: 'num', d: 0, g: 'Maska / efekty', anim: 1, min: 0, max: 100, label: 'maska P%' },
  clipT: { t: 'num', d: 0, g: 'Maska / efekty', anim: 1, min: 0, max: 100, label: 'maska G%' },
  clipB: { t: 'num', d: 0, g: 'Maska / efekty', anim: 1, min: 0, max: 100, label: 'maska D%' },
  blur: { t: 'num', d: 0, g: 'Maska / efekty', anim: 1, min: 0, label: 'rozmycie' },
  skewX: { t: 'num', d: 0, g: 'Maska / efekty', anim: 1, label: 'pochylenie°' },
  blink: { t: 'num', d: 0, g: 'Maska / efekty', min: 0, step: 0.1, label: 'miganie (okres s)' },

  follow: { t: 'node', d: '', g: 'Autofollow', label: 'śledź węzeł(y)' },
  followAxis: { t: 'select', d: 'w', g: 'Autofollow', opts: ['w', 'h', 'wh'], label: 'oś' },
  followPad: { t: 'num', d: 40, g: 'Autofollow', label: 'margines' },
  followMin: { t: 'num', d: 0, g: 'Autofollow', min: 0, label: 'minimum' },

  color: { t: 'color', d: '#ffffff', g: 'Wygląd', anim: 1, label: 'kolor' },
  color2: { t: 'color', d: '', g: 'Wygląd', anim: 1, label: 'kolor 2 (gradient)' },
  gradAngle: { t: 'num', d: 90, g: 'Wygląd', anim: 1, label: 'kąt gradientu' },
  radius: { t: 'num', d: 0, g: 'Wygląd', anim: 1, min: 0, label: 'zaokrąglenie' },
  borderColor: { t: 'color', d: '#ffffff', g: 'Wygląd', anim: 1, label: 'kolor ramki' },
  borderWidth: { t: 'num', d: 0, g: 'Wygląd', anim: 1, min: 0, label: 'ramka px' },
  shadow: { t: 'num', d: 0, g: 'Wygląd', anim: 1, min: 0, label: 'cień' },

  text: { t: 'text', d: 'Tekst', g: 'Tekst', label: 'tekst' },
  font: { t: 'font', d: 'Inter', g: 'Tekst', label: 'font' },
  size: { t: 'num', d: 48, g: 'Tekst', anim: 1, min: 1, label: 'rozmiar' },
  weight: { t: 'select', d: '700', g: 'Tekst', opts: ['300', '400', '500', '600', '700', '800', '900'], label: 'grubość' },
  italic: { t: 'bool', d: false, g: 'Tekst', label: 'kursywa' },
  upper: { t: 'bool', d: false, g: 'Tekst', label: 'WERSALIKI' },
  spacing: { t: 'num', d: 0, g: 'Tekst', anim: 1, step: 0.5, label: 'odstęp liter' },
  lineHeight: { t: 'num', d: 1.15, g: 'Tekst', step: 0.05, label: 'interlinia' },
  align: { t: 'select', d: 'left', g: 'Tekst', opts: ['left', 'center', 'right'], label: 'wyrównanie' },
  valign: { t: 'select', d: 'middle', g: 'Tekst', opts: ['top', 'middle', 'bottom'], label: 'w pionie' },
  textFit: { t: 'select', d: 'shrink', g: 'Tekst', opts: ['squeeze', 'squeeze-shrink', 'squeeze-lines', 'shrink', 'none'], label: 'dopasowanie' },
  squeezeMin: { t: 'num', d: 0.6, g: 'Tekst', min: 0.3, max: 1, step: 0.05, label: 'min. ściśnięcie' },
  maxLines: { t: 'num', d: 1, g: 'Tekst', min: 1, label: 'maks. linii' },
  padX: { t: 'num', d: 0, g: 'Tekst', anim: 1, min: 0, label: 'margines X' },
  hlColor: { t: 'color', d: '#e30613', g: 'Tekst', label: 'tło [[wyróżnienia]]' },
  hlText: { t: 'color', d: '#ffffff', g: 'Tekst', label: 'tekst [[wyróżnienia]]' },
  cycle: { t: 'num', d: 0, g: 'Tekst', min: 0, step: 0.5, label: 'rotacja wierszy (s)' },
  qrBg: { t: 'color', d: '#ffffff', g: 'Kod QR', label: 'tło kodu' },

  src: { t: 'image', d: '', g: 'Obraz', label: 'plik' },
  fitMode: { t: 'select', d: 'contain', g: 'Obraz', opts: ['contain', 'cover', 'fill'], label: 'dopasowanie' },
  loop: { t: 'bool', d: true, g: 'Obraz', label: 'zapętl wideo' },

  mode: { t: 'select', d: 'time', g: 'Zegar', opts: ['time', 'up', 'down', 'target'], label: 'tryb' },
  format: { t: 'string', d: 'HH:mm:ss', g: 'Zegar', label: 'format' },
  seconds: { t: 'num', d: 300, g: 'Zegar', label: 'start (s)' },
  target: { t: 'string', d: '2027-01-01 00:00:00', g: 'Zegar', label: 'cel (data)' },

  speed: { t: 'num', d: 140, g: 'Ticker', anim: 1, label: 'prędkość px/s' },
  sep: { t: 'string', d: '   •   ', g: 'Ticker', label: 'separator' },
  sepColor: { t: 'color', d: '', g: 'Ticker', label: 'kolor separatora' },
  tickerMode: { t: 'select', d: 'crawl', g: 'Ticker', opts: TICKER_MODES, label: 'tryb' },
  sepStyle: { t: 'select', d: 'text', g: 'Ticker', opts: SEP_STYLES, label: 'styl separatora' },
  crawlFrom: { t: 'select', d: 'right', g: 'Ticker', opts: ['right', 'left'], label: 'start crawla' },
  label: { t: 'string', d: '', g: 'Ticker', label: 'etykieta domyślna' },
  catColors: { t: 'string', d: '', g: 'Ticker', label: 'kolory kategorii' },
  gleam: { t: 'num', d: 0, g: 'Ticker', min: 0, step: 0.5, label: 'błysk etykiety (co s)' },
  hold: { t: 'num', d: 5, g: 'Ticker', min: 1, step: 0.5, label: 'czas wiadomości (s)' },
  catW: { t: 'num', d: 170, g: 'Ticker', min: 0, label: 'szer. kategorii' },
  labelSize: { t: 'num', d: 0, g: 'Ticker', min: 0, label: 'rozmiar etykiety (0 = 80%)' },
  labelWeight: { t: 'select', d: '', g: 'Ticker', opts: ['', '300', '400', '500', '600', '700', '800', '900'], label: 'grubość etykiety' },
  catBg: { t: 'color', d: '#c8102e', g: 'Ticker', label: 'tło kategorii' },
  catFg: { t: 'color', d: '#ffffff', g: 'Ticker', label: 'tekst kategorii' },
};

const COMMON = ['x', 'y', 'w', 'h', 'anchorX', 'anchorY', 'rotation', 'scale', 'opacity', 'clipL', 'clipR', 'clipT', 'clipB', 'blur', 'skewX', 'blink'];
const FOLLOW = ['follow', 'followAxis', 'followPad', 'followMin'];
const TEXT_STYLE = ['font', 'size', 'weight', 'italic', 'upper', 'spacing', 'lineHeight', 'align', 'valign', 'textFit', 'maxLines', 'squeezeMin', 'padX', 'color', 'shadow'];

export const TYPES = {
  group: { label: 'Grupa', icon: '▣', props: [...COMMON, ...FOLLOW], children: true },
  rect: { label: 'Prostokąt', icon: '■', props: [...COMMON, ...FOLLOW, 'color', 'color2', 'gradAngle', 'radius', 'borderColor', 'borderWidth', 'shadow'], children: true },
  ellipse: { label: 'Elipsa', icon: '●', props: [...COMMON, ...FOLLOW, 'color', 'color2', 'gradAngle', 'borderColor', 'borderWidth', 'shadow'] },
  text: { label: 'Tekst', icon: 'T', props: [...COMMON, 'text', 'cycle', ...TEXT_STYLE, 'hlColor', 'hlText'] },
  qr: { label: 'Kod QR', icon: '▦', props: [...COMMON, 'text', 'color', 'qrBg', 'radius', 'shadow'] },
  image: { label: 'Obraz/wideo', icon: '🖼', props: [...COMMON, ...FOLLOW, 'src', 'fitMode', 'radius', 'shadow', 'loop'] },
  clock: { label: 'Zegar', icon: '⏱', props: [...COMMON, 'mode', 'format', 'seconds', 'target', ...TEXT_STYLE] },
  ticker: { label: 'Ticker', icon: '⇠', props: [...COMMON, 'text', 'tickerMode', 'hold', 'speed', 'crawlFrom', 'sep', 'sepStyle', 'sepColor', 'label', 'catColors', 'catW', 'catBg', 'catFg', 'labelSize', 'labelWeight', 'gleam', 'textFit', 'squeezeMin', 'font', 'size', 'weight', 'italic', 'upper', 'spacing', 'color', 'shadow'] },
};

export const FONTS = ['Barlow Condensed', 'Barlow', 'Inter', 'Oswald', 'Bebas Neue', 'Roboto Condensed', 'Montserrat', 'Anton', 'Archivo Black', 'Teko', 'Roboto Mono', 'Playfair Display', 'Arial'];
export const EASES = ['inout', 'out', 'in', 'linear', 'back', 'hold'];

export const LAYER_Z = { back: 10, main: 20, front: 30 };
export const layerZ = (n) => LAYER_Z[n] ?? 25;

// ------------------------------------------------------------------ pomocnicze
let _id = 0;
export const uid = (p = 'n') => p + Date.now().toString(36) + (++_id).toString(36) + Math.random().toString(36).slice(2, 5);

export function walk(nodes, fn, parent = null) {
  for (const n of nodes || []) {
    if (fn(n, parent) === false) return false;
    if (n.children && walk(n.children, fn, n) === false) return false;
  }
}

/** Pola wariantów (wybór z listy) zdefiniowane na poziomie sceny. */
export function variantFields(scene) {
  return Object.entries(scene.fieldMeta || {})
    .filter(([, m]) => m && m.type === 'select' && Array.isArray(m.options) && m.options.length)
    .map(([id, m]) => ({ id, prop: null, type: 'select', opts: m.options, node: '(wariant)', nodeType: 'variant', label: m.label || id, default: m.default ?? m.options[0] }));
}

/** Pola kontrolne (Control Object) sceny – warianty, potem w kolejności drzewa. */
export function listFields(scene) {
  const out = variantFields(scene);
  const seen = new Set(out.map((f) => f.id));
  walk(scene.nodes, (n) => {
    for (const [prop, fid] of Object.entries(n.fields || {})) {
      if (!fid || seen.has(fid)) continue;
      seen.add(fid);
      const def = PROPS[prop] || { t: 'string' };
      const meta = scene.fieldMeta?.[fid] || {};
      out.push({ id: fid, prop, type: def.t, opts: def.opts, node: n.name || n.type, nodeType: n.type, lines: Math.round(+n.props?.maxLines || 1), label: meta.label || fid, rules: meta, default: n.props?.[prop] ?? def.d });
    }
  });
  return out;
}

function coerce(v, t) {
  if (t === 'num') { const n = parseFloat(v); return isNaN(n) ? 0 : n; }
  if (t === 'bool') return v === true || v === 'true' || v === '1' || v === 1;
  return v == null ? '' : String(v);
}

const EASE = {
  linear: (u) => u,
  in: (u) => u * u * u,
  out: (u) => 1 - Math.pow(1 - u, 3),
  inout: (u) => (u < 0.5 ? 4 * u * u * u : 1 - Math.pow(-2 * u + 2, 3) / 2),
  back: (u) => { const c1 = 1.70158, c3 = c1 + 1; return 1 + c3 * Math.pow(u - 1, 3) + c1 * Math.pow(u - 1, 2); },
  hold: () => 0,
};

export function parseColor(c) {
  if (!c || typeof c !== 'string') return null;
  c = c.trim();
  if (c[0] === '#') {
    let h = c.slice(1);
    if (h.length === 3 || h.length === 4) h = [...h].map((x) => x + x).join('');
    const n = (i) => parseInt(h.slice(i, i + 2), 16);
    return [n(0), n(2), n(4), h.length >= 8 ? n(6) / 255 : 1];
  }
  const m = c.match(/rgba?\(([^)]+)\)/);
  if (m) { const p = m[1].split(',').map(parseFloat); return [p[0], p[1], p[2], p[3] ?? 1]; }
  return null;
}
const rgba = (c) => `rgba(${Math.round(c[0])},${Math.round(c[1])},${Math.round(c[2])},${+c[3].toFixed(3)})`;

function mix(a, b, u, t) {
  if (t === 'num') return (+a) + ((+b) - (+a)) * u;
  if (t === 'color') {
    const ca = parseColor(a), cb = parseColor(b);
    if (!ca || !cb) return u < 0.5 ? a : b;
    return rgba(ca.map((x, i) => x + (cb[i] - x) * u));
  }
  return u < 1 ? a : b;
}

export function trackValue(track, t, type) {
  if (!track || !track.length) return undefined;
  if (t <= track[0].t) return track[0].v;
  const last = track[track.length - 1];
  if (t >= last.t) return last.v;
  for (let i = 0; i < track.length - 1; i++) {
    const a = track[i], b = track[i + 1];
    if (t < b.t) {
      const span = b.t - a.t;
      const u = span <= 0 ? 1 : (EASE[a.e || 'inout'] || EASE.inout)((t - a.t) / span);
      return mix(a.v, b.v, u, type);
    }
  }
  return last.v;
}

/** Wartość właściwości w czasie t, z uwzględnieniem pól strony (values). */
export function propAt(node, prop, t, values) {
  const def = PROPS[prop] || { t: 'string', d: '' };
  const fid = node.fields?.[prop];
  if (fid && values && values[fid] !== undefined && values[fid] !== null) return coerce(values[fid], def.t);
  const tr = node.anim?.[prop];
  if (tr && tr.length) return trackValue(tr, t, def.t);
  const v = node.props?.[prop];
  if (v === undefined) {
    if (prop === 'textFit' && node.props?.fit === false) return 'none'; // zgodność ze starszymi scenami
    if (prop === 'textFit' && node.type === 'ticker') return 'squeeze-shrink'; // tickery sprzed ustawienia dopasowania
    return def.d;
  }
  return v;
}

/** Czy węzeł jest widoczny w danym wariancie (showIf: {field, value}). */
export function showIfOk(node, scene, values) {
  const c = node.showIf;
  if (!c || !c.field) return true;
  let v = values?.[c.field];
  if (v === undefined || v === null) {
    const m = scene?.fieldMeta?.[c.field];
    v = m?.default ?? m?.options?.[0] ?? '';
  }
  return String(c.value ?? '').split(',').map((s) => s.trim()).includes(String(v));
}

// ------------------------------------------------------------------ fonty
const loadedFonts = new Set();
/** Rodziny dostępne offline (wbudowane w serwer: /fonts/fonts.css). Reszta idzie przez Google Fonts. */
export const OFFLINE_FONTS = ['Oswald', 'Barlow Condensed', 'Inter', 'Roboto Condensed', 'Montserrat', 'Roboto'];
if (typeof document !== 'undefined' && !document.querySelector('link[data-vz-fonts]')) {
  const l = document.createElement('link');
  l.rel = 'stylesheet'; l.href = '/fonts/fonts.css'; l.dataset.vzFonts = '1';
  document.head.appendChild(l);
}
export function ensureFont(name, assetBase = '') {
  if (!name || loadedFonts.has(name)) return;
  loadedFonts.add(name);
  if (OFFLINE_FONTS.includes(name)) return;
  const sys = ['Arial', 'Helvetica', 'Verdana', 'Georgia', 'Times New Roman', 'Courier New', 'Tahoma', 'Impact', 'sans-serif', 'serif', 'monospace'];
  if (sys.includes(name)) return;
  if (/\.(ttf|otf|woff2?)$/i.test(name)) {
    const st = document.createElement('style');
    st.textContent = `@font-face{font-family:"${name}";src:url("${assetBase}/assets/${encodeURIComponent(name)}");font-display:block}`;
    document.head.appendChild(st);
  } else {
    const l = document.createElement('link');
    l.rel = 'stylesheet';
    l.onerror = () => window.dispatchEvent(new CustomEvent('vizrs-font-fail', { detail: { name } }));
    l.href = `https://fonts.googleapis.com/css2?family=${encodeURIComponent(name).replace(/%20/g, '+')}:ital,wght@0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,400;1,700&display=block`;
    document.head.appendChild(l);
  }
}
let fontEpoch = 0;
if (typeof document !== 'undefined' && document.fonts) {
  document.fonts.addEventListener?.('loadingdone', () => { fontEpoch++; });
}

// ------------------------------------------------------------------ format zegara
// Tryb "time": yyyy MM dd HH H mm ss.  Tryby licznikowe: Dn (np. "3 dni"), D, HH, H, mm, ss, S.
// Tekst w apostrofach jest dosłowny, np.  D 'dni' HH:mm:ss
const pad = (n, l = 2) => String(Math.floor(n)).padStart(l, '0');
const plDays = (d) => `${d} ${d === 1 ? 'dzień' : 'dni'}`;
export function fmtClock(fmt, mode, secs) {
  const parts = String(fmt || '').split(/('[^']*')/);
  if (mode === 'time') {
    const d = new Date();
    const map = { yyyy: d.getFullYear(), dd: pad(d.getDate()), MM: pad(d.getMonth() + 1), HH: pad(d.getHours()), H: d.getHours(), mm: pad(d.getMinutes()), ss: pad(d.getSeconds()) };
    return parts.map((s, i) => (i % 2 ? s.slice(1, -1) : s.replace(/yyyy|dd|MM|HH|H|mm|ss/g, (k) => map[k]))).join('');
  }
  secs = Math.max(0, secs);
  const plain = parts.filter((_, i) => i % 2 === 0).join('');
  const hasD = /D/.test(plain), hasH = /H/.test(plain);
  const days = Math.floor(secs / 86400);
  const h = hasD ? Math.floor((secs % 86400) / 3600) : Math.floor(secs / 3600);
  const m = hasH || hasD ? Math.floor((secs % 3600) / 60) : Math.floor(secs / 60);
  const map = { Dn: plDays(days), D: days, HH: pad(h), H: h, mm: pad(m), ss: pad(secs % 60), S: Math.floor((secs % 1) * 10) };
  return parts.map((s, i) => (i % 2 ? s.slice(1, -1) : s.replace(/Dn|D|HH|H|mm|ss|S/g, (k) => map[k]))).join('');
}

// ------------------------------------------------------------------ tekst sformatowany ([[wyróżnienie]], {b}, {i}, {u}, {c=…}, {sz=…})
function setRich(span, text, hlBg, hlFg) {
  span.textContent = '';
  if (!hasMarkup(text)) { span.textContent = text; return; }
  let mk = null;
  for (const r of parseMarkup(text).runs) {
    let node = document.createTextNode(r.t);
    if (r.b || r.i || r.u || r.c || r.sz) {
      const sp = document.createElement('span');
      sp.textContent = r.t;
      if (r.b) sp.style.fontWeight = 'bolder';
      if (r.i) sp.style.fontStyle = 'italic';
      if (r.u) sp.style.textDecoration = 'underline';
      if (r.c) sp.style.color = r.c;
      if (r.sz) sp.style.fontSize = r.sz + '%';
      node = sp;
    }
    if (r.hl) {
      if (!mk) {
        mk = document.createElement('mark');
        Object.assign(mk.style, { background: hlBg, color: hlFg, padding: '0 .16em', margin: '0 .04em', borderRadius: '.06em', whiteSpace: 'nowrap', boxDecorationBreak: 'clone', webkitBoxDecorationBreak: 'clone' });
        span.appendChild(mk);
      }
      mk.appendChild(node);
    } else { mk = null; span.appendChild(node); }
  }
}

/** Dzieli wiersz na k wierszy o możliwie równej szerokości (słowa i [[wyróżnienia]] są niepodzielne). */
function balanceLines(line, k, W) {
  const toks = line.match(/\[\[[\s\S]*?\]\]|\S+/g) || [line];
  k = Math.min(k, toks.length);
  if (k <= 1) return [line];
  const sw = W(' '), tw = toks.map(W), n = toks.length;
  const pre = [0];
  for (let i = 0; i < n; i++) pre.push(pre[i] + tw[i]);
  const width = (i, j) => pre[j] - pre[i] + (j - i - 1) * sw;
  const best = [], cut = [];
  for (let m = 1; m <= k; m++) {
    best[m] = []; cut[m] = [];
    for (let j = m; j <= n; j++) {
      if (m === 1) { best[1][j] = width(0, j); cut[1][j] = 0; continue; }
      let b = Infinity, c = m - 1;
      for (let i = m - 1; i < j; i++) {
        const v = Math.max(best[m - 1][i], width(i, j));
        if (v < b - 0.01) { b = v; c = i; }
      }
      best[m][j] = b; cut[m][j] = c;
    }
  }
  const out = [];
  let j = n;
  for (let m = k; m >= 1; m--) { const i = cut[m][j]; out.unshift(toks.slice(i, j).join(' ')); j = i; }
  return out;
}

/**
 * Tryb "squeeze-lines" – każdy wiersz jest ściskany poziomo NIEZALEŻNIE.
 * Wiersze: ręczne (Enter) albo automatyczny, zrównoważony podział, gdy tekst nie mieści się w jednym.
 * Gdy wiersz wymagałby ściśnięcia poniżej minimum, zmniejszany jest font tylko tego wiersza.
 */
function fitLines(sp, text, o) {
  const ss = sp.style;
  const L = Math.max(1, Math.round(o.maxLines || 1));
  const avail = Math.max(1, o.avail);
  const MIN = squeezeFloor(o.minSqueeze);
  const origin = o.align === 'right' ? '100% 50%' : o.align === 'center' ? '50% 50%' : '0 50%';
  Object.assign(ss, { transform: '', whiteSpace: 'pre', maxWidth: 'none', display: 'flex', flexDirection: 'column', fontSize: o.size + 'px',
    alignItems: o.align === 'right' ? 'flex-end' : o.align === 'center' ? 'center' : 'flex-start' });
  sp.textContent = '';
  const meas = document.createElement('span');
  meas.style.whiteSpace = 'pre';
  sp.appendChild(meas);
  const W = (s) => { setRich(meas, s, '', ''); return meas.offsetWidth; };
  let lines = text.split('\n').map((s) => s.trim()).filter(Boolean);
  if (!lines.length) lines = [''];
  if (lines.length > L) lines = [...lines.slice(0, L - 1), lines.slice(L - 1).join(' ')];
  if (lines.length < L) {
    // rozbij najdłuższe wiersze, dopóki są za szerokie i jest miejsce na kolejne
    let changed = true;
    while (changed && lines.length < L) {
      changed = false;
      const ws = lines.map(W);
      const i = ws.indexOf(Math.max(...ws));
      if (ws[i] > avail) {
        const parts = balanceLines(lines[i], Math.min(L - lines.length + 1, Math.ceil(ws[i] / avail)), W);
        if (parts.length > 1) { lines.splice(i, 1, ...parts); changed = true; }
      }
    }
  }
  sp.textContent = '';
  lines = carryLines(lines); // każdy wiersz samodzielnie poprawny (znaczniki przeniesione przez podział)
  let maxW = 0;
  const sq = [];
  for (const line of lines) {
    const d = document.createElement('span');
    Object.assign(d.style, { display: 'block', whiteSpace: 'pre', transformOrigin: origin });
    setRich(d, line, o.hlBg, o.hlFg);
    sp.appendChild(d);
    let w = d.offsetWidth;
    let s = w > avail ? avail / w : 1;
    let f = 1;
    if (s < MIN) {
      // ten wiersz: ściśnij do minimum, resztę nadrób mniejszym fontem (tylko w tym wierszu)
      f = s / MIN;
      d.style.fontSize = o.size * f + 'px';
      w = d.offsetWidth;
      s = w > avail ? Math.max(MIN, avail / w) : 1;
    }
    if (s < 0.999) d.style.transform = `scaleX(${s})`;
    sq.push(Math.round(s * f * 100) / 100);
    maxW = Math.max(maxW, w * s);
  }
  return { w: Math.min(maxW, avail), h: sp.offsetHeight, sq, over: false };
}

/** Granica ściśnięcia (squeezeMin) w dozwolonym zakresie 0,3…1; brak = 0,6. */
export const squeezeFloor = (v) => Math.min(1, Math.max(0.3, +v || 0.6));

/**
 * Dopasowanie tekstu do ramki (jak "Shrink"/"Squeeze" w Viz):
 *  squeeze        – jak w BTB CG: ściska tekst poziomo, najwyżej do `squeezeMin`; font się nie zmienia.
 *                   Gdy to za mało, tekst wystaje poza ramkę, a wynik ma `over` (ostrzeżenie dla operatora).
 *  squeeze-shrink – ściska do `squeezeMin`, a dopiero potem zmniejsza font
 *  shrink         – zmniejsza font, by zmieścić się w szerokości i w maxLines liniach
 * Zwraca rzeczywisty rozmiar treści (szer., wys.) w px sceny, ściśnięcie `sq` (lista z jednym elementem) i `over`.
 */
function fitText(sp, o) {
  const ss = sp.style;
  ss.display = 'block'; ss.flexDirection = ''; ss.alignItems = '';
  const L = Math.max(1, Math.round(o.maxLines || 1));
  const avail = Math.max(1, o.avail);
  const lh = o.lineHeight || 1.15;
  const MIN = squeezeFloor(o.minSqueeze);
  ss.transform = '';
  ss.transformOrigin = o.align === 'right' ? '100% 50%' : o.align === 'center' ? '50% 50%' : '0 50%';
  ss.whiteSpace = L > 1 ? 'pre-wrap' : 'pre';
  const fits = (fs, width) => {
    ss.fontSize = fs + 'px';
    if (L > 1) { ss.maxWidth = width + 'px'; return sp.offsetHeight / (fs * lh) <= L + 0.35; }
    ss.maxWidth = 'none';
    return sp.scrollWidth <= width + 0.5;
  };
  const searchFont = (width) => {
    let lo = 4, hi = o.size;
    for (let i = 0; i < 9; i++) { const mid = (lo + hi) / 2; if (fits(mid, width)) lo = mid; else hi = mid; }
    fits(lo, width);
    return lo;
  };
  let sx = 1, fs = o.size, over = false;
  const squeeze = o.mode === 'squeeze' || o.mode === 'squeeze-shrink';
  if (o.mode === 'none') {
    ss.fontSize = o.size + 'px';
    ss.maxWidth = L > 1 ? avail + 'px' : 'none';
  } else if (fits(o.size, avail)) {
    // mieści się bez zmian
  } else if (squeeze) {
    const shrink = o.mode === 'squeeze-shrink';
    if (L === 1) {
      ss.fontSize = o.size + 'px';
      const need = avail / sp.scrollWidth;
      sx = Math.max(MIN, need);
      if (need < MIN) { if (shrink) fs = searchFont(avail / MIN); else over = true; }
    } else if (fits(o.size, avail / MIN)) {
      let lo = MIN, hi = 1;
      for (let i = 0; i < 7; i++) { const mid = (lo + hi) / 2; if (fits(o.size, avail / mid)) lo = mid; else hi = mid; }
      sx = lo; fits(o.size, avail / lo);
    } else {
      sx = MIN;
      if (shrink) fs = searchFont(avail / MIN);
      else { fits(o.size, avail / MIN); over = true; }
    }
    if (sx < 0.999) ss.transform = `scaleX(${sx})`;
  } else {
    fs = searchFont(avail);
  }
  const sq = squeeze ? [Math.round(sx * fs / o.size * 100) / 100] : null;
  return { w: Math.min(sp.offsetWidth * sx, L > 1 && !over ? avail : Infinity), h: sp.offsetHeight, sq, over };
}

// ------------------------------------------------------------------ ticker: elementy wspólne dla trybów
/** Pas błysku (gleam) w etykiecie; pozycja liczona z czasu, więc działa także w render_video.py. */
function gleamEl(parent) {
  const g = document.createElement('div');
  Object.assign(g.style, { position: 'absolute', top: '0', bottom: '0', width: '45%', left: '-60%', display: 'none', pointerEvents: 'none',
    background: 'linear-gradient(90deg, rgba(255,255,255,0), rgba(255,255,255,.4), rgba(255,255,255,0))', transform: 'skewX(-20deg)' });
  parent.appendChild(g);
  return g;
}
function gleamAt(g, period, now) {
  if (!(period > 0)) { g.style.display = 'none'; return; }
  const dur = Math.min(1.6, period * 0.6), u = ((now / 1000) % period) / dur;
  if (u >= 1) { g.style.display = 'none'; return; }
  g.style.display = 'block';
  g.style.left = (-60 + u * 170) + '%';
}
/** Rozmiar tekstu etykiety/kategorii tickera: `labelSize` w px albo 80% rozmiaru treści. */
const labFont = (p) => (p('labelSize') > 0 ? p('labelSize') + 'px' : '.8em');
/** Jeden odcinek taśmy crawla: [plakietka kategorii] treść [separator]. */
function tickerSegment(x, o) {
  const seg = document.createElement('span');
  Object.assign(seg.style, { position: 'absolute', top: '0', height: '100%', display: 'flex', alignItems: 'center', whiteSpace: 'pre' });
  if (x.cat) {
    const b = document.createElement('span');
    b.textContent = o.up ? x.cat.toUpperCase() : x.cat;
    Object.assign(b.style, { background: catColorOf(x, o.colors, o.catBg), color: o.catFg, padding: '.05em .45em', marginRight: '.5em', borderRadius: '3px', fontSize: '.78em', textShadow: 'none' });
    seg.appendChild(b);
  }
  seg.appendChild(document.createTextNode(o.up ? x.text.toUpperCase() : x.text));
  const sp = document.createElement('span');
  if (o.sepStyle && o.sepStyle !== 'text') {
    const dim = { dot: ['.32em', '.32em', '50%', ''], square: ['.3em', '.3em', '0', ''], pipe: ['.12em', '.95em', '0', ''], skew: ['.16em', '1.05em', '0', 'skewX(-20deg)'] }[o.sepStyle] || ['.32em', '.32em', '50%', ''];
    Object.assign(sp.style, { display: 'inline-block', width: dim[0], height: dim[1], borderRadius: dim[2], transform: dim[3], margin: '0 .9em', flex: 'none',
      background: o.sepColor || 'currentColor', opacity: o.sepColor ? '1' : '.55' });
  } else {
    sp.textContent = o.sep;
    if (o.sepColor) sp.style.color = o.sepColor;
  }
  seg.appendChild(sp);
  return seg;
}
const easeIO = (u) => (u < 0.5 ? 4 * u * u * u : 1 - Math.pow(-2 * u + 2, 3) / 2);

// ------------------------------------------------------------------ globalna pętla animacji
const players = new Set();
let last = 0;
function loop(now) {
  const dt = last ? Math.min(0.1, (now - last) / 1000) : 0;
  last = now;
  for (const p of [...players]) p.tick(dt, now);
  requestAnimationFrame(loop);
}
if (typeof requestAnimationFrame !== 'undefined') requestAnimationFrame(loop);

// ------------------------------------------------------------------ odtwarzacz sceny
export class ScenePlayer {
  /**
   * @param parent element DOM
   * @param scene  obiekt sceny
   * @param values wartości pól kontrolnych strony
   * @param opts   { editor: bool, assetBase: '', onDone: fn }
   */
  constructor(parent, scene, values = {}, opts = {}) {
    this.scene = scene;
    this.values = { ...values };
    this.opts = opts;
    this.t = 0;
    this.target = 0;
    this.playing = false;
    this.mode = 'idle';
    this.speed = 1;
    this.wallStart = performance.now();
    this.dead = false;
    this.el = document.createElement('div');
    this.el.className = 'vz-scene';
    Object.assign(this.el.style, {
      position: 'absolute', left: '0', top: '0', overflow: 'hidden',
      width: (scene.width || 1920) + 'px', height: (scene.height || 1080) + 'px',
      pointerEvents: opts.editor ? 'auto' : 'none',
    });
    parent.appendChild(this.el);
    this.build();
    this.apply(0);
    players.add(this);
  }

  get stops() { return [...(this.scene.stops || [])].sort((a, b) => a - b); }
  get duration() { return +this.scene.duration || 0; }

  build() {
    this.el.innerHTML = '';
    this.items = [];
    this.byName = new Map();
    this._lastApply = 0;
    const mk = (nodes, parentEl) => {
      for (const n of nodes || []) {
        const el = document.createElement('div');
        el.className = 'vz-node vz-' + n.type;
        el.dataset.id = n.id;
        Object.assign(el.style, { position: 'absolute', boxSizing: 'border-box', transformOrigin: '50% 50%' });
        const it = { node: n, el, cache: {} };
        if (n.type === 'text' || n.type === 'clock') {
          Object.assign(el.style, { display: 'flex', overflow: 'visible' });
          it.span = document.createElement('span');
          Object.assign(it.span.style, { display: 'block', flex: 'none' });
          el.appendChild(it.span);
        } else if (n.type === 'ticker') {
          el.style.overflow = 'hidden';
          it.strip = document.createElement('div');
          Object.assign(it.strip.style, { position: 'absolute', left: '0', top: '0', height: '100%', display: 'flex', alignItems: 'center', whiteSpace: 'pre', willChange: 'transform' });
          el.appendChild(it.strip);
        } else if (n.type === 'image' || n.type === 'qr') {
          el.style.overflow = 'hidden';
        }
        parentEl.appendChild(el);
        this.items.push(it);
        if (n.name && !this.byName.has(n.name)) this.byName.set(n.name, it);
        this.byName.set(n.id, it);
        if (n.children?.length) mk(n.children, el);
      }
    };
    mk(this.scene.nodes, this.el);
  }

  // ---- sterowanie (logika Viz: stop pointy w dyrektorze) ----
  take() {
    this.t = 0;
    this.wallStart = performance.now();
    for (const it of this.items) it.tk = null;
    const s = this.stops;
    this.target = s.length ? s[0] : this.duration;
    this.mode = 'in';
    this.playing = true;
    return this;
  }
  cont() {
    const next = this.stops.find((s) => s > this.t + 1e-4);
    if (next !== undefined) { this.target = next; this.playing = true; this.mode = 'cont'; }
    return this;
  }
  out() {
    const s = this.stops;
    const lastStop = s.length ? s[s.length - 1] : null;
    if (lastStop !== null && this.duration > lastStop + 1e-3) {
      if (this.t < lastStop) this.t = lastStop;
      this.target = this.duration;
      this.playing = true;
      this.mode = 'out';
    } else {
      this.destroy();
    }
  }
  /** Ustaw scenę statycznie w danym stop poincie (np. po odświeżeniu OBS). */
  seekStop(i) {
    const s = this.stops;
    this.t = s.length ? s[Math.min(i, s.length - 1)] : this.duration;
    this.playing = false;
    this.mode = 'hold';
    this.apply(this.t);
    return this;
  }
  seek(t) { this.t = t; this.playing = false; this.apply(t); }
  playTo(target, from) {
    if (from !== undefined) this.t = from;
    this.target = target; this.playing = true; this.mode = 'play';
  }
  update(values) {
    this.values = { ...values };
    for (const it of this.items) it.cache = {};
    this.apply(this.t);
  }
  /** Powiązanie (dock): płynne przesunięcie całej sceny o (dx, dy) px w czasie `dur` s. */
  setDock(dx, dy, dur) {
    const st = this.el.style;
    st.transition = dur > 0 ? `transform ${dur}s cubic-bezier(.4,0,.2,1)` : 'none';
    st.transform = dx || dy ? `translate(${dx}px, ${dy}px)` : '';
  }
  /** Powiązanie (dock): płynne przesunięcie całej sceny o (dx, dy) px w czasie `dur` s. */
  setDock(dx, dy, dur) {
    const st = this.el.style;
    st.transition = dur > 0 ? `transform ${dur}s cubic-bezier(.4,0,.2,1)` : 'none';
    st.transform = dx || dy ? `translate(${dx}px, ${dy}px)` : '';
  }
  setScene(scene) { this.scene = scene; this.el.style.width = (scene.width || 1920) + 'px'; this.el.style.height = (scene.height || 1080) + 'px'; this.build(); this.apply(this.t); }
  destroy() {
    if (this.dead) return;
    this.dead = true;
    players.delete(this);
    this.el.remove();
    this.opts.onDone?.(this);
  }

  tick(dt, now) {
    if (this.playing) {
      this.t = Math.min(this.target, this.t + dt * this.speed);
      if (this.t >= this.target - 1e-6) {
        this.t = this.target;
        this.playing = false;
        this.opts.onStop?.(this);
        if (this.mode === 'out') { this.apply(this.t, now); this.destroy(); return; }
      }
    }
    this.apply(this.t, now);
  }

  // ---- ticker ----
  // Stan tickera (it.tk) jest osobno od it.cache: UPDATE czyści cache, ale nie przerywa przewijania ani rotacji.

  /** Wiadomości węzła (parsowane tylko po zmianie tekstu). */
  tickerItems(it, p) {
    const raw = String(p('text') ?? '');
    if (it.tkRaw !== raw) { it.tkRaw = raw; it.tkItems = parseTickerItems(stripMarkup(raw)); }
    return it.tkItems;
  }

  /** Podgląd jednej wiadomości (najechanie na wiersz listy); fid = pole albo id węzła, idx = null wyłącza. */
  tickerPin(fid, idx) {
    for (const it of this.items) if (it.node.type === 'ticker' && (it.node.fields?.text === fid || it.node.id === fid)) it.pin = idx;
    this.apply(this.t);
  }

  /**
   * Ściśnięcie potrzebne każdej wiadomości (1 = mieści się) dla węzła tickera z polem `fid`; crawl → null (tekst przewija się).
   * Mierzone od razu dla całej listy – operator nie musi czekać na rotację. Lista ma `min` – granicę ściśnięcia
   * (poniżej niej wiadomość w trybie `squeeze` wystaje i jest ucinana; w `squeeze-shrink` dostaje mniejszy font).
   */
  tickerSqueeze(fid) {
    const it = this.items.find((i) => i.node.type === 'ticker' && (i.node.fields?.text === fid || i.node.id === fid) && i.vis && i.p);
    if (!it || it.p('tickerMode') === 'crawl') return null;
    const items = this.tickerItems(it, it.p), up = it.p('upper');
    const avail = Math.max(1, it.w - (it.tkCatW || 0) - 36);
    const m = document.createElement('span');
    Object.assign(m.style, { position: 'absolute', visibility: 'hidden', whiteSpace: 'pre', left: '0', top: '0' });
    it.strip.appendChild(m);
    const out = items.map((x) => { m.textContent = up ? x.text.toUpperCase() : x.text; return Math.min(1, avail / Math.max(1, m.scrollWidth)); });
    m.remove();
    const fit = it.p('textFit');
    out.min = fit === 'none' ? 1 : fit === 'squeeze' || fit === 'squeeze-lines' ? squeezeFloor(it.p('squeezeMin')) : 0;
    return out;
  }

  /** Ticker w trybach roll (góra–dół), wipe (maska z wycieraczką) i whip (etykieta przejeżdża przez pasek). */
  tickerSlots(it, p, now, fe, items) {
    const mode = p('tickerMode');
    const n = items.length;
    const up = p('upper'), font = p('font'), w = it.w, h = it.h;
    const label = String(p('label') || ''), colors = parseCatColors(p('catColors')), catBg = p('catBg');
    const catW = label || items.some((x) => x.cat) ? Math.max(0, p('catW')) : 0;
    it.tkCatW = catW;
    ensureFont(font, this.opts.assetBase);
    const key = ['slots', up, font, p('size'), p('weight'), p('italic'), p('spacing'), w, h, catW, catBg, p('catFg'), p('color'), p('shadow'), p('catColors'), label, p('labelSize'), p('labelWeight'), p('textFit'), p('squeezeMin'), fe].join('|');
    const S = it.strip.style;
    if (it.cache.key !== key || !it.slots) {
      it.cache.key = key;
      if (it.tk?.mode === 'crawl') it.tk = null;
      it.strip.innerHTML = '';
      const sh = p('shadow');
      Object.assign(S, { position: 'absolute', inset: '0', left: '0', top: '0', width: '100%', height: '100%', transform: '', display: 'block', color: p('color'),
        fontFamily: `"${font}", "Barlow Condensed", Inter, Arial, sans-serif`, fontWeight: p('weight'), fontSize: p('size') + 'px',
        fontStyle: p('italic') ? 'italic' : 'normal', letterSpacing: p('spacing') + 'px', whiteSpace: 'pre',
        textShadow: sh > 0 ? `0 ${Math.max(1, sh / 4)}px ${sh}px rgba(0,0,0,.65)` : '' });
      const mkSlot = () => {
        const cat = document.createElement('div');
        Object.assign(cat.style, { position: 'absolute', left: '0', top: '0', width: catW + 'px', height: '100%', display: catW ? 'flex' : 'none', zIndex: '2',
          alignItems: 'center', justifyContent: 'center', color: p('catFg'), fontSize: labFont(p), fontWeight: p('labelWeight') || '', textShadow: 'none', overflow: 'hidden' });
        const catT = document.createElement('span');
        catT.style.position = 'relative';
        cat.appendChild(catT);
        const gl = gleamEl(cat);
        const box = document.createElement('div');
        Object.assign(box.style, { position: 'absolute', left: catW + 'px', top: '0', right: '0', height: '100%', display: 'flex', alignItems: 'center', paddingLeft: '18px', overflow: 'hidden' });
        const tx = document.createElement('span');
        Object.assign(tx.style, { display: 'block', flex: 'none', transformOrigin: '0 50%' });
        box.appendChild(tx);
        it.strip.appendChild(box); it.strip.appendChild(cat);
        return { cat, catT, gl, box, tx, k: null, lk: null };
      };
      it.slots = [mkSlot(), mkSlot()];
      const wiper = document.createElement('div');
      Object.assign(wiper.style, { position: 'absolute', top: '0', height: '100%', width: '6px', background: catBg, display: 'none', zIndex: '3' });
      it.strip.appendChild(wiper);
      it.wiper = wiper;
    }
    const [A, B] = it.slots;
    const vis = (sl, on) => { sl.cat.style.visibility = sl.box.style.visibility = on ? 'visible' : 'hidden'; };
    for (const sl of it.slots) {
      for (const e of [sl.cat, sl.box]) { e.style.transform = ''; e.style.clipPath = ''; e.style.opacity = ''; }
      vis(sl, false);
    }
    it.wiper.style.display = 'none';
    const lab = (x) => (x ? labelOf(x, label) : '');
    const labKey = (x) => lab(x) + '#' + (x ? catColorOf(x, colors, catBg) : '');
    const fillLabel = (sl, x) => {
      const k = labKey(x);
      if (sl.lk === k) return;
      sl.lk = k;
      const l = lab(x);
      sl.catT.textContent = up ? l.toUpperCase() : l;
      sl.cat.style.background = l ? catColorOf(x, colors, catBg) : 'transparent';
    };
    const fill = (sl, x) => {
      fillLabel(sl, x);
      const k = x ? x.text : '';
      if (sl.k === k) return;
      sl.k = k;
      sl.tx.textContent = up ? k.toUpperCase() : k;
      sl.tx.style.transform = ''; sl.tx.style.fontSize = '';
      const avail = Math.max(1, w - catW - 36), sw = sl.tx.scrollWidth;
      if (sw > avail) {
        const fit = p('textFit'), MIN = squeezeFloor(p('squeezeMin'));
        let s = avail / sw;
        if (fit === 'none') return; // wiadomość ucięta na krawędzi paska
        if (fit === 'shrink') { sl.tx.style.fontSize = (p('size') * s) + 'px'; s = Math.min(1, avail / sl.tx.scrollWidth); if (s < 0.999) sl.tx.style.transform = `scaleX(${s})`; return; }
        if (s < MIN && fit === 'squeeze-shrink') { sl.tx.style.fontSize = (p('size') * s / MIN) + 'px'; s = Math.min(1, avail / sl.tx.scrollWidth); }
        sl.tx.style.transform = `scaleX(${Math.max(MIN, s)})`;
      }
    };
    const gp = p('gleam');
    const gleams = () => { for (const sl of it.slots) gleamAt(sl.gl, sl.cat.style.visibility === 'visible' ? gp : 0, now); };

    // podgląd jednej wiadomości (Trio)
    if (it.pin != null && n) {
      fill(A, items[Math.max(0, Math.min(n - 1, it.pin))]); vis(A, true); gleams();
      return;
    }
    // stan rotacji: bieżąca wiadomość, następna, początek bieżącej
    const hold = Math.max(1, p('hold'));
    let tk = it.tk;
    if (!tk || tk.mode !== 'slots') tk = it.tk = { mode: 'slots', items, cur: items[0] || null, curIdx: n ? 0 : -1, prev: null, nextIdx: n > 1 ? 1 : 0, t0: now };
    if (tk.items !== items) {
      if (!tk.cur && n) Object.assign(tk, { cur: items[0], curIdx: 0, nextIdx: n > 1 ? 1 : 0, t0: now });
      else if (tk.curIdx >= 0) { const c = continuation(tk.items, tk.curIdx, items); tk.curIdx = c.cur; tk.nextIdx = c.next; }
      else { const c = continuation(tk.items, tk.nextIdx, items); tk.nextIdx = c.cur >= 0 ? c.cur : c.next; }
      tk.items = items;
    }
    if (!n || !tk.cur) { gleams(); return; }
    const single = n === 1 && tk.curIdx === 0;
    if (single) tk.t0 = now;
    if (now - tk.t0 > hold * 50000) tk.t0 = now; // po długiej przerwie (karta w tle) – bez nadrabiania
    while ((now - tk.t0) / 1000 >= hold) {
      tk.prev = tk.cur;
      tk.curIdx = tk.nextIdx % n; tk.cur = items[tk.curIdx]; tk.nextIdx = (tk.curIdx + 1) % n;
      tk.t0 += hold * 1000;
    }
    const cur = tk.cur, next = items[tk.nextIdx % n];
    const ph = (now - tk.t0) / 1000;
    fill(A, cur); fill(B, next);
    vis(A, true);
    if (single) { gleams(); return; }
    if (mode === 'roll') {
      const T = Math.min(0.55, hold * 0.3);
      if (ph > hold - T) {
        const u = easeIO((ph - (hold - T)) / T);
        vis(B, true);
        A.box.style.transform = `translateY(${-u * h}px)`;
        B.box.style.transform = `translateY(${(1 - u) * h}px)`;
        if (labKey(cur) !== labKey(next)) {
          A.cat.style.transform = `translateY(${-u * h}px)`;
          B.cat.style.transform = `translateY(${(1 - u) * h}px)`;
        } else B.cat.style.visibility = 'hidden';
      }
    } else if (mode === 'whip') {
      // wiadomość wchodzi i wychodzi przenikaniem; potem etykieta przejeżdża w prawo przez pasek,
      // w połowie drogi zmienia się na kategorię następnej wiadomości i wraca na miejsce
      const Tin = Math.min(0.25, hold * 0.1), Tout = Math.min(0.35, hold * 0.1);
      const Tw = catW ? Math.min(1.5, hold * 0.35) : 0;
      const outAt = hold - Tw - Tout;
      A.box.style.opacity = ph < Tin ? ph / Tin : ph < outAt ? 1 : Math.max(0, 1 - (ph - outAt) / Tout);
      if (Tw && ph >= hold - Tw) {
        const u = (ph - (hold - Tw)) / Tw, travel = Math.max(0, w - catW);
        const x = u < 0.65 ? easeIO(u / 0.65) * travel : (1 - easeIO((u - 0.65) / 0.35)) * travel;
        const sl = u < 0.325 ? A : B;
        if (sl === B) { A.cat.style.visibility = 'hidden'; B.cat.style.visibility = 'visible'; }
        sl.cat.style.transform = `translateX(${x}px)`;
      }
    } else {
      // wipe: odsłonięcie w lewo->prawo, na końcu zmazanie; krawędź maski prowadzi "wycieraczka"
      const T = Math.min(0.45, hold * 0.25);
      const bw = w - catW;
      it.wiper.style.background = catColorOf(cur, colors, catBg);
      const wipeEdge = (u) => { it.wiper.style.display = 'block'; it.wiper.style.left = (catW + u * bw - 3) + 'px'; };
      if (ph < T && tk.prev) {
        const u = easeIO(ph / T);
        A.box.style.clipPath = `inset(0 ${(1 - u) * 100}% 0 0)`;
        if (labKey(cur) !== labKey(tk.prev)) A.cat.style.clipPath = `inset(${(1 - u) * 100}% 0 0 0)`;
        if (u < 0.999) wipeEdge(u);
      } else if (ph > hold - T) {
        const u = easeIO((ph - (hold - T)) / T);
        A.box.style.clipPath = `inset(0 0 0 ${u * 100}%)`;
        if (labKey(cur) !== labKey(next)) A.cat.style.clipPath = `inset(0 0 ${u * 100}% 0)`;
        if (u > 0.001) wipeEdge(u);
      }
    }
    gleams();
  }

  /**
   * Crawl: taśma z wiadomościami doklejanymi na końcu i usuwanymi za lewą krawędzią.
   * Po zmianie listy to, co jest na ekranie, przewija się dalej, a nowe wiadomości dochodzą za nim (bez skoku).
   */
  tickerCrawl(it, p, now, fe, items) {
    const n = items.length;
    const up = p('upper'), font = p('font'), w = it.w;
    const label = String(p('label') || ''), colors = parseCatColors(p('catColors')), catBg = p('catBg');
    const labW = label ? Math.max(0, p('catW')) : 0;
    ensureFont(font, this.opts.assetBase);
    const S = it.strip.style;
    S.color = p('color');
    const sh = p('shadow');
    S.textShadow = sh > 0 ? `0 ${Math.max(1, sh / 4)}px ${sh}px rgba(0,0,0,.65)` : '';
    let tk = it.tk;
    if (!tk || tk.mode !== 'crawl') {
      it.strip.innerHTML = '';
      it.slots = null; it.cache.key = null;
      Object.assign(S, { position: 'absolute', inset: '0', left: '0', top: '0', width: '100%', height: '100%', display: 'block', transform: '', whiteSpace: 'pre' });
      const clip = document.createElement('div');
      Object.assign(clip.style, { position: 'absolute', top: '0', bottom: '0', right: '0', overflow: 'hidden' });
      const tape = document.createElement('div');
      Object.assign(tape.style, { position: 'absolute', left: '0', top: '0', height: '100%', willChange: 'transform' });
      const pinEl = document.createElement('div');
      Object.assign(pinEl.style, { position: 'absolute', inset: '0', display: 'none' });
      clip.appendChild(tape); clip.appendChild(pinEl);
      const lab = document.createElement('div');
      Object.assign(lab.style, { position: 'absolute', left: '0', top: '0', height: '100%', display: 'none', alignItems: 'center', justifyContent: 'center', fontSize: '.8em', textShadow: 'none', overflow: 'hidden', zIndex: '2' });
      const labT = document.createElement('span');
      labT.style.position = 'relative';
      lab.appendChild(labT);
      const gl = gleamEl(lab);
      it.strip.appendChild(clip); it.strip.appendChild(lab);
      tk = it.tk = { mode: 'crawl', clip, tape, pinEl, lab, labT, gl, segs: [], d: 0, cursor: 0, nextIdx: 0, items, styleKey: null, last: null, pinK: null, fresh: true };
    }
    const styleKey = [up, p('sep'), p('sepStyle'), p('sepColor'), catBg, p('catFg'), p('catColors'), font, p('size'), p('weight'), p('italic'), p('spacing'), fe].join('|');
    const segOf = (x) => tickerSegment(x, { up, sep: p('sep'), sepStyle: p('sepStyle'), sepColor: p('sepColor'), colors, catBg, catFg: p('catFg') });
    if (tk.styleKey !== styleKey) {
      tk.styleKey = styleKey;
      Object.assign(S, { fontFamily: `"${font}", "Barlow Condensed", Inter, Arial, sans-serif`, fontWeight: p('weight'), fontSize: p('size') + 'px', fontStyle: p('italic') ? 'italic' : 'normal', letterSpacing: p('spacing') + 'px' });
      // przerysuj to, co jest na taśmie, w nowym stylu i ułóż od nowa od pierwszej pozycji
      let pos = tk.segs[0]?.pos ?? tk.cursor;
      for (const sg of tk.segs) {
        const el = segOf(sg.item);
        sg.el.replaceWith(el); sg.el = el;
        el.style.left = pos + 'px'; sg.pos = pos; sg.w = Math.max(1, el.offsetWidth); pos += sg.w;
      }
      tk.cursor = pos;
      tk.pinK = null;
    }
    // etykieta stała (jak wiper w trybie poziomym BTB)
    tk.clip.style.left = labW + 'px';
    tk.lab.style.display = labW ? 'flex' : 'none';
    if (labW) {
      Object.assign(tk.lab.style, { width: labW + 'px', background: catColorOf({ cat: label }, colors, catBg), color: p('catFg'), fontSize: labFont(p), fontWeight: p('labelWeight') || '' });
      tk.labT.textContent = up ? label.toUpperCase() : label;
      gleamAt(tk.gl, p('gleam'), now);
    }
    const vw = Math.max(1, w - labW);
    // podgląd jednej wiadomości (Trio)
    if (it.pin != null && n) {
      const x = items[Math.max(0, Math.min(n - 1, it.pin))];
      const k = styleKey + '|' + JSON.stringify(x);
      if (tk.pinK !== k) { tk.pinK = k; tk.pinEl.innerHTML = ''; const el = segOf(x); el.style.left = '18px'; tk.pinEl.appendChild(el); }
      tk.pinEl.style.display = 'block'; tk.tape.style.visibility = 'hidden';
      return;
    }
    tk.pinEl.style.display = 'none'; tk.tape.style.visibility = 'visible';
    // zmiana listy: dalej od wiadomości po ostatnio doklejonej
    if (tk.items !== items) {
      const lastSeg = tk.segs[tk.segs.length - 1];
      if (lastSeg && tk.items.length) { const c = continuation(tk.items, lastSeg.idx, items); tk.nextIdx = c.cur >= 0 ? c.next : c.next; } else tk.nextIdx = 0;
      tk.items = items;
    }
    const dt = tk.last == null ? 0 : Math.max(0, Math.min(0.25, (now - tk.last) / 1000));
    tk.last = now;
    tk.d += p('speed') * dt;
    if (!tk.segs.length) tk.cursor = Math.max(tk.cursor, tk.d); // pusta taśma: start od prawej krawędzi
    // crawlFrom=left: pierwsza wiadomość od lewej krawędzi, taśma od razu pełna (jak warstwa ticker w BTB CG)
    if (tk.fresh) { tk.fresh = false; if (p('crawlFrom') === 'left') tk.cursor = tk.d - vw; }
    for (let guard = 0; n && tk.cursor - tk.d < vw + 40 && guard < 200; guard++) {
      const idx = tk.nextIdx % n, x = items[idx];
      const el = segOf(x);
      el.style.left = tk.cursor + 'px';
      tk.tape.appendChild(el);
      const sw = Math.max(1, el.offsetWidth);
      tk.segs.push({ el, item: x, idx, pos: tk.cursor, w: sw });
      tk.cursor += sw;
      tk.nextIdx = (idx + 1) % n;
    }
    while (tk.segs.length && tk.segs[0].pos + tk.segs[0].w < tk.d - vw) tk.segs.shift().el.remove();
    // przesunięcie układu, żeby współrzędne nie rosły bez końca (precyzja kompozytora)
    if (tk.d > 50000) {
      const off = tk.segs.length ? tk.segs[0].pos : tk.d;
      for (const sg of tk.segs) { sg.pos -= off; sg.el.style.left = sg.pos + 'px'; }
      tk.d -= off; tk.cursor -= off;
    }
    tk.tape.style.transform = `translateX(${vw - tk.d}px)`;
  }

  // ---- renderowanie ----
  /** Odrysowanie z izolacją błędów: wadliwy węzeł jest wyłączany (`broken`), reszta sceny gra dalej. */
  apply(t, now = performance.now()) {
    for (let attempt = 0; attempt < 6; attempt++) {
      try { this._apply(t, now); return; } catch (e) {
        const it = this._cur;
        if (it && !it.broken) {
          it.broken = true;
          try { it.el.style.display = 'none'; } catch (_) { /* ignoruj */ }
          ScenePlayer.onError?.({ scene: this.scene?.name || this.scene?.id || '?', node: it.node?.name || it.node?.id || '?', message: String(e?.message || e) });
        } else {
          ScenePlayer.onError?.({ scene: this.scene?.name || '?', node: '?', message: String(e?.message || e) });
          return;
        }
      }
    }
  }

  _apply(t, now = performance.now()) {
    const V = this.values;
    const fe = fontEpoch;
    const dt = this._lastApply ? Math.min(0.25, Math.max(0, (now - this._lastApply) / 1000)) : 10;
    this._lastApply = now;

    // przebieg 1: widoczność i treść (tu mierzymy teksty dla autofollow)
    for (const it of this.items) {
      this._cur = it;
      if (it.broken) continue;
      const n = it.node, el = it.el, st = el.style;
      const p = (k) => propAt(n, k, t, V);
      it.p = p;
      it.vis = !n.hidden && showIfOk(n, this.scene, V);
      if (!it.vis) { st.display = 'none'; continue; }
      st.display = (n.type === 'text' || n.type === 'clock') ? 'flex' : 'block';
      it.w = Math.max(0, p('w'));
      it.h = Math.max(0, p('h'));
      it.fol = TYPES[n.type]?.props.includes('follow') ? String(p('follow') || '').trim() : '';
      if (!it.fol) { st.width = it.w + 'px'; st.height = it.h + 'px'; }

      switch (n.type) {
        case 'rect':
        case 'ellipse': {
          const c = p('color'), c2 = p('color2');
          st.background = c2 ? `linear-gradient(${p('gradAngle')}deg, ${c}, ${c2})` : c;
          st.borderRadius = n.type === 'ellipse' ? '50%' : p('radius') + 'px';
          const bw = p('borderWidth');
          st.border = bw > 0 ? `${bw}px solid ${p('borderColor')}` : 'none';
          const sh = p('shadow');
          st.boxShadow = sh > 0 ? `0 ${sh / 3}px ${sh}px rgba(0,0,0,.5)` : '';
          break;
        }
        case 'image': {
          const src = p('src');
          if (it.cache.src !== src) {
            it.cache.src = src;
            el.innerHTML = '';
            if (src) {
              const url = /^(https?:|data:|\/)/.test(src) ? src : (this.opts.assetBase || '') + '/assets/' + encodeURIComponent(src);
              const isVid = /\.(mp4|webm|mov)$/i.test(src);
              const m = document.createElement(isVid ? 'video' : 'img');
              if (isVid) { m.muted = true; m.autoplay = true; m.playsInline = true; m.loop = p('loop'); }
              m.src = url;
              m.draggable = false;
              Object.assign(m.style, { width: '100%', height: '100%', display: 'block', pointerEvents: 'none' });
              el.appendChild(m);
              it.media = m;
            } else it.media = null;
          }
          if (it.media) it.media.style.objectFit = p('fitMode');
          st.borderRadius = p('radius') + 'px';
          const sh = p('shadow');
          st.boxShadow = sh > 0 ? `0 ${sh / 3}px ${sh}px rgba(0,0,0,.5)` : '';
          break;
        }
        case 'text':
        case 'clock': {
          let text;
          if (n.type === 'clock') {
            const mode = p('mode');
            const el_s = (now - this.wallStart) / 1000;
            let secs = 0;
            if (mode === 'up') secs = el_s;
            else if (mode === 'down') secs = p('seconds') - el_s;
            else if (mode === 'target') { const tg = Date.parse(String(p('target')).trim().replace(' ', 'T')); secs = isNaN(tg) ? 0 : (tg - Date.now()) / 1000; }
            text = fmtClock(p('format') || 'HH:mm:ss', mode, secs);
          } else text = String(p('text') ?? '');
          it.cyOp = 1;
          const cyc = n.type === 'text' ? p('cycle') : 0;
          if (cyc > 0 && text.includes('\n')) {
            // rotacja: co `cyc` sekund następny wiersz (zsynchronizowane w obrębie sceny), z przenikaniem
            const lines = text.split('\n').filter((l) => l.trim());
            const el_s = (now - this.wallStart) / 1000;
            text = lines[Math.floor(el_s / cyc) % lines.length] || '';
            const ph = el_s % cyc, F = Math.min(0.35, cyc / 4);
            it.cyOp = Math.max(0, Math.min(1, ph / F, (cyc - ph) / F));
          }
          if (p('upper')) text = text.toUpperCase();
          const font = p('font');
          ensureFont(font, this.opts.assetBase);
          const size = p('size'), padX = p('padX'), w = it.w;
          const align = p('align'), valign = p('valign');
          const hlBg = p('hlColor'), hlFg = p('hlText');
          const fitMode = p('textFit'), maxLines = p('maxLines'), lh = p('lineHeight');
          const sqMin = p('squeezeMin');
          const key = [text, font, size, p('weight'), p('italic'), p('spacing'), fitMode, maxLines, sqMin, w, padX, lh, align, hlBg, hlFg, fe].join('|');
          const sp = it.span, ss = sp.style;
          st.justifyContent = align === 'center' ? 'center' : align === 'right' ? 'flex-end' : 'flex-start';
          st.alignItems = valign === 'top' ? 'flex-start' : valign === 'bottom' ? 'flex-end' : 'center';
          st.textAlign = align;
          st.paddingLeft = st.paddingRight = padX + 'px';
          ss.color = p('color');
          const sh = p('shadow');
          ss.textShadow = sh > 0 ? `0 ${Math.max(1, sh / 4)}px ${sh}px rgba(0,0,0,.65)` : '';
          if (it.cache.key !== key) {
            it.cache.key = key;
            if (fitMode !== 'squeeze-lines') setRich(sp, text, hlBg, hlFg);
            ss.fontFamily = `"${font}", "Barlow Condensed", Inter, Arial, sans-serif`;
            ss.fontWeight = p('weight');
            ss.fontStyle = p('italic') ? 'italic' : 'normal';
            ss.letterSpacing = p('spacing') + 'px';
            ss.lineHeight = lh;
            ss.fontVariantNumeric = 'tabular-nums';
            const fo = { avail: w - 2 * padX, size, maxLines, mode: fitMode, align, lineHeight: lh, minSqueeze: sqMin, hlBg, hlFg };
            const m = fitMode === 'squeeze-lines' ? fitLines(sp, text, fo) : fitText(sp, fo);
            it.mw = m.w; it.mh = m.h; it.sq = m.sq || null; it.over = !!m.over;
          }
          break;
        }
        case 'qr': {
          const key = [p('text'), p('color'), p('qrBg')].join('|');
          if (it.cache.key !== key) { it.cache.key = key; el.innerHTML = qrSvg(String(p('text') ?? ''), p('color'), p('qrBg')); }
          st.borderRadius = p('radius') + 'px';
          const sh = p('shadow');
          st.boxShadow = sh > 0 ? `0 ${sh / 3}px ${sh}px rgba(0,0,0,.5)` : '';
          break;
        }
        case 'ticker': {
          const items = this.tickerItems(it, p);
          if (p('tickerMode') === 'crawl') this.tickerCrawl(it, p, now, fe, items);
          else this.tickerSlots(it, p, now, fe, items);
          break;
        }
      }
    }

    // przebieg 2: geometria (autofollow, punkty zaczepienia, transformacje)
    const k = 1 - Math.exp(-dt * 12);
    for (const it of this.items) {
      this._cur = it;
      if (it.broken || !it.vis) continue;
      const p = it.p, st = it.el.style;
      let w = it.w, h = it.h;
      if (it.fol) {
        let mw = 0, mh = 0, found = false;
        for (const name of it.fol.split(',')) {
          const tg = this.byName.get(name.trim());
          if (tg && tg.vis && tg.mw != null) { found = true; mw = Math.max(mw, tg.mw); mh = Math.max(mh, tg.mh); }
        }
        if (found) {
          const ax = p('followAxis'), pad = p('followPad'), mn = p('followMin');
          if (ax.includes('w')) w = Math.max(mn, mw + pad);
          if (ax.includes('h')) h = Math.max(mn, mh + pad);
        }
        // płynna zmiana rozmiaru (np. po UPDATE na antenie)
        if (it.fw == null || k > 0.99) { it.fw = w; it.fh = h; }
        else {
          it.fw += (w - it.fw) * k; it.fh += (h - it.fh) * k;
          if (Math.abs(w - it.fw) < 0.4) it.fw = w;
          if (Math.abs(h - it.fh) < 0.4) it.fh = h;
        }
        w = it.fw; h = it.fh;
        st.width = w + 'px'; st.height = h + 'px';
      }
      const ax = p('anchorX'), ay = p('anchorY');
      st.left = p('x') - (ax === 'right' ? w : ax === 'center' ? w / 2 : 0) + 'px';
      st.top = p('y') - (ay === 'bottom' ? h : ay === 'middle' ? h / 2 : 0) + 'px';
      let op = p('opacity') * (it.cyOp ?? 1);
      const bl = p('blink');
      if (bl > 0) op *= 0.15 + 0.85 * (0.5 + 0.5 * Math.cos((now / 1000) * 2 * Math.PI / bl));
      st.opacity = op;
      st.visibility = op <= 0.001 ? 'hidden' : 'visible';
      const rot = p('rotation'), sc = p('scale'), sk = p('skewX');
      st.transform = (rot || sc !== 1 || sk) ? `rotate(${rot}deg) scale(${sc}) skewX(${sk}deg)` : '';
      const cl = p('clipL'), cr = p('clipR'), ct = p('clipT'), cb = p('clipB');
      st.clipPath = (cl || cr || ct || cb) ? `inset(${ct}% ${cr}% ${cb}% ${cl}%)` : '';
      const blr = p('blur');
      st.filter = blr > 0 ? `blur(${blr}px)` : '';
    }
  }
}

// ------------------------------------------------------------------ host warstw (wyjście)
export class LayerHost {
  constructor(el, opts = {}) {
    this.el = el;
    this.opts = opts;
    this.layers = new Map();
    this.filter = opts.layers || null; // np. ['main','front']
    this.dockOff = new Set(); // warstwy z wyłączonym powiązaniem (nadpisanie operatora)
    this.bus = opts.bus || 'program'; // 'program' | 'preview' (zdarzenia z `bus` innym niż ten są pomijane)
  }
  accepts(name) { return !this.filter || this.filter.includes(name); }
  layer(name) {
    let L = this.layers.get(name);
    if (!L) {
      const div = document.createElement('div');
      div.className = 'vz-layer';
      div.dataset.layer = name;
      Object.assign(div.style, { position: 'absolute', inset: '0', zIndex: layerZ(name) });
      this.el.appendChild(div);
      L = { div, player: null };
      this.layers.set(name, L);
    }
    return L;
  }
  take(name, scene, values, since, page) {
    if (!this.accepts(name)) return;
    const L = this.layer(name);
    if (L.player && !L.player.dead) L.player.out();
    try {
      L.player = new ScenePlayer(L.div, scene, values, { assetBase: this.opts.assetBase });
      L.player.take();
      L.since = since;
      L.page = page;
    } catch (e) {
      L.player = null;
      ScenePlayer.onError?.({ scene: scene?.name || '?', node: '(scena)', message: String(e?.message || e) });
    }
  }
  /**
   * Powiązania (dock): scena z regułami `dock` przesuwa się, gdy wskazana grafika jest na antenie
   * (`layer:<nazwa>`, `page:<numer>`, `scene:<nazwa>`). Własna warstwa nie wyzwala własnej reguły.
   */
  recomputeDock() {
    const air = [];
    for (const [name, L] of this.layers) if (L.player && !L.player.dead) air.push({ layer: name, page: L.page?.number, scene: L.player.scene?.name });
    for (const [name, L] of this.layers) {
      const p = L.player; if (!p || p.dead) continue;
      let dx = 0, dy = 0, dur = 0.6;
      if (!this.dockOff.has(name)) {
        for (const r of p.scene?.dock || []) {
          const [kind, ...rest] = String(r.target || '').split(':'); const val = rest.join(':');
          const hit = air.some((a) => a.layer !== name && (kind === 'layer' ? a.layer === val : kind === 'page' ? String(a.page) === val : kind === 'scene' ? a.scene === val : false));
          if (hit) { dx += +r.dx || 0; dy += +r.dy || 0; dur = r.dur ?? dur; }
        }
      }
      p.setDock(dx, dy, dur);
    }
  }
  /** Wdrożenie nowej wersji szablonu w miejscu, bez ponownej animacji (stan czasu zostaje). */
  rescene(name, scene, stop) {
    const p = this.layers.get(name)?.player;
    if (!p || p.dead) return;
    try { p.setScene(scene); if (!p.playing) p.seekStop(stop || 0); } catch (e) {
      ScenePlayer.onError?.({ scene: scene?.name || '?', node: '(scena)', message: String(e?.message || e) });
    }
  }
  cont(name) { this.layers.get(name)?.player?.cont(); }
  out(name) {
    const L = this.layers.get(name);
    if (L?.player && !L.player.dead) L.player.out();
    if (L) L.player = null;
  }
  update(name, values) { const p = this.layers.get(name)?.player; if (p && !p.dead) p.update(values); }
  clear() {
    for (const L of this.layers.values()) {
      if (L.player) L.player.destroy();
      L.player = null;
      L.div.innerHTML = '';
    }
  }
  /**
   * Odtwarza stan anteny po (ponownym) połączeniu. Warstwy, które już grają tę samą stronę
   * (ten sam `since`), zostają nietknięte – bez migotania; reszta jest wymieniana lub zdejmowana.
   */
  snapshot(list) {
    const keep = new Set();
    for (const l of list || []) {
      if (!this.accepts(l.layer)) continue;
      const L = this.layer(l.layer);
      keep.add(l.layer);
      if (L.player && !L.player.dead && l.since && L.since === l.since) {
        L.player.update(l.values || {});
        continue;
      }
      if (L.player) L.player.destroy();
      L.div.innerHTML = '';
      try {
        L.player = new ScenePlayer(L.div, l.scene, l.values, { assetBase: this.opts.assetBase }).seekStop(l.stop || 0);
        L.since = l.since;
        L.page = l.page;
      } catch (e) {
        L.player = null;
        ScenePlayer.onError?.({ scene: l.scene?.name || '?', node: '(scena)', message: String(e?.message || e) });
      }
    }
    for (const [name, L] of this.layers) {
      if (keep.has(name)) continue;
      if (L.player) L.player.destroy();
      L.player = null;
      L.div.innerHTML = '';
    }
  }
  handle(ev) {
    if ((ev.bus || 'program') !== this.bus) return;
    switch (ev.t) {
      case 'snapshot': this.dockOff = new Set(ev.dockOff || []); this.snapshot(ev.layers); this.recomputeDock(); break;
      case 'take': this.take(ev.layer, ev.scene, ev.values, ev.since, ev.page); this.recomputeDock(); break;
      case 'dock': if (ev.off) this.dockOff.add(ev.layer); else this.dockOff.delete(ev.layer); this.recomputeDock(); break;
      case 'rescene': this.rescene(ev.layer, ev.scene, ev.stop); break;
      case 'continue': this.cont(ev.layer); break;
      case 'out': this.out(ev.layer); this.recomputeDock(); break;
      case 'update': this.update(ev.layer, ev.values); break;
      case 'clear': this.clear(); this.recomputeDock(); break;
    }
  }
}

/** Połączenie SSE z automatycznym wznowieniem. */
export function connectEvents(onEvent, onStatus) {
  let es;
  const open = () => {
    es = new EventSource('/events');
    es.onopen = () => onStatus?.(true);
    es.onmessage = (m) => { try { onEvent(JSON.parse(m.data)); } catch (e) { console.error(e); } };
    es.onerror = () => { onStatus?.(false); };
  };
  open();
  return () => es.close();
}

/** Skalowanie sceny 1920x1080 do rozmiaru kontenera. */
export function fitStage(stage, box, w = 1920, h = 1080, mode = 'contain') {
  const r = box.getBoundingClientRect();
  const s = mode === 'width' ? r.width / w : Math.min(r.width / w, r.height / h);
  stage.style.width = w + 'px';
  stage.style.height = h + 'px';
  stage.style.transformOrigin = '0 0';
  const ox = mode === 'width' ? 0 : (r.width - w * s) / 2;
  const oy = mode === 'width' ? 0 : (r.height - h * s) / 2;
  stage.style.transform = `translate(${ox}px, ${oy}px) scale(${s})`;
  return s;
}
