import { qrSvg } from '/qr.js';
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
  textFit: { t: 'select', d: 'shrink', g: 'Tekst', opts: ['shrink', 'squeeze', 'squeeze-lines', 'none'], label: 'dopasowanie' },
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
  tickerMode: { t: 'select', d: 'crawl', g: 'Ticker', opts: ['crawl', 'roll', 'wipe'], label: 'tryb' },
  hold: { t: 'num', d: 5, g: 'Ticker', min: 1, step: 0.5, label: 'czas wiadomości (s)' },
  catW: { t: 'num', d: 170, g: 'Ticker', min: 0, label: 'szer. kategorii' },
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
  ticker: { label: 'Ticker', icon: '⇠', props: [...COMMON, 'text', 'tickerMode', 'hold', 'speed', 'sep', 'sepColor', 'catW', 'catBg', 'catFg', 'font', 'size', 'weight', 'italic', 'upper', 'spacing', 'color', 'shadow'] },
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
  const MIN = Math.min(1, Math.max(0.3, +o.minSqueeze || 0.6));
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
  return { w: Math.min(maxW, avail), h: sp.offsetHeight, sq };
}

/**
 * Dopasowanie tekstu do ramki (jak "Shrink"/"Squeeze" w Viz):
 *  shrink  – zmniejsza font, by zmieścić się w szerokości i w maxLines liniach
 *  squeeze – najpierw ściska tekst poziomo (do 62%), dopiero potem zmniejsza font
 * Zwraca rzeczywisty rozmiar treści (szer., wys.) w px sceny.
 */
function fitText(sp, o) {
  const ss = sp.style;
  ss.display = 'block'; ss.flexDirection = ''; ss.alignItems = '';
  const L = Math.max(1, Math.round(o.maxLines || 1));
  const avail = Math.max(1, o.avail);
  const lh = o.lineHeight || 1.15;
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
  };
  let sx = 1;
  if (o.mode === 'none') {
    ss.fontSize = o.size + 'px';
    ss.maxWidth = L > 1 ? avail + 'px' : 'none';
  } else if (fits(o.size, avail)) {
    // mieści się bez zmian
  } else if (o.mode === 'squeeze') {
    const MIN = 0.62;
    if (L === 1) {
      ss.fontSize = o.size + 'px';
      const sw = sp.scrollWidth;
      sx = Math.max(MIN, avail / sw);
      if (avail / sw < MIN) searchFont(avail / MIN);
    } else if (fits(o.size, avail / MIN)) {
      let lo = MIN, hi = 1;
      for (let i = 0; i < 7; i++) { const mid = (lo + hi) / 2; if (fits(o.size, avail / mid)) lo = mid; else hi = mid; }
      sx = lo; fits(o.size, avail / lo);
    } else { sx = MIN; searchFont(avail / MIN); }
    if (sx < 0.999) ss.transform = `scaleX(${sx})`;
  } else {
    searchFont(avail);
  }
  return { w: Math.min(sp.offsetWidth * sx, L > 1 ? avail : Infinity), h: sp.offsetHeight };
}

// ------------------------------------------------------------------ ticker: kategorie
/** "SPORT#1f8a4c|Treść" -> {cat:'SPORT', color:'#1f8a4c', text:'Treść'}; bez '|' – sama treść. */
export function parseTickerItem(line) {
  const i = line.indexOf('|');
  if (i < 0) return { cat: '', color: '', text: line.trim() };
  let cat = line.slice(0, i).trim(), color = '';
  const m = cat.match(/^(.*?)#([0-9a-fA-F]{3,8})$/);
  if (m) { cat = m[1].trim(); color = '#' + m[2]; }
  return { cat, color, text: line.slice(i + 1).trim() };
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

  /** Ticker w trybie "roll" (przewijanie góra-dół) lub "wipe" (odsłanianie maską z wycieraczką). */
  tickerSlots(it, p, now, fe) {
    const mode = p('tickerMode');
    const items = stripMarkup(String(p('text') ?? '')).split('\n').map((s) => s.trim()).filter(Boolean).map(parseTickerItem);
    const n = items.length;
    const up = p('upper'), font = p('font'), w = it.w, h = it.h;
    const hasCat = items.some((x) => x.cat);
    const catW = hasCat ? Math.max(0, p('catW')) : 0;
    ensureFont(font, this.opts.assetBase);
    const key = [mode, JSON.stringify(items), up, font, p('size'), p('weight'), p('italic'), p('spacing'), w, h, catW, p('catBg'), p('catFg'), p('color'), p('shadow'), fe].join('|');
    const S = it.strip.style;
    if (it.cache.key !== key) {
      it.cache.key = key;
      it.cache.idx = -1;
      it.strip.innerHTML = '';
      const sh = p('shadow');
      Object.assign(S, { position: 'absolute', inset: '0', width: '100%', height: '100%', transform: '', display: 'block', color: p('color'),
        fontFamily: `"${font}", "Barlow Condensed", Inter, Arial, sans-serif`, fontWeight: p('weight'), fontSize: p('size') + 'px',
        fontStyle: p('italic') ? 'italic' : 'normal', letterSpacing: p('spacing') + 'px', whiteSpace: 'pre',
        textShadow: sh > 0 ? `0 ${Math.max(1, sh / 4)}px ${sh}px rgba(0,0,0,.65)` : '' });
      const mkSlot = () => {
        const cat = document.createElement('div');
        Object.assign(cat.style, { position: 'absolute', left: '0', top: '0', width: catW + 'px', height: '100%', display: catW ? 'flex' : 'none',
          alignItems: 'center', justifyContent: 'center', color: p('catFg'), fontSize: '.8em', textShadow: 'none', overflow: 'hidden' });
        const catT = document.createElement('span');
        cat.appendChild(catT);
        const box = document.createElement('div');
        Object.assign(box.style, { position: 'absolute', left: catW + 'px', top: '0', right: '0', height: '100%', display: 'flex', alignItems: 'center', paddingLeft: '18px', overflow: 'hidden' });
        const tx = document.createElement('span');
        Object.assign(tx.style, { display: 'block', flex: 'none', transformOrigin: '0 50%' });
        box.appendChild(tx);
        it.strip.appendChild(cat); it.strip.appendChild(box);
        return { cat, catT, box, tx, idx: -1 };
      };
      it.slots = [mkSlot(), mkSlot()];
      const wiper = document.createElement('div');
      Object.assign(wiper.style, { position: 'absolute', top: '0', height: '100%', width: '6px', background: p('catBg'), display: 'none' });
      it.strip.appendChild(wiper);
      it.wiper = wiper;
    }
    if (!n) { for (const sl of it.slots) { sl.cat.style.visibility = sl.box.style.visibility = 'hidden'; } return; }
    const fill = (sl, i) => {
      if (sl.idx === i) return;
      sl.idx = i;
      const itm = items[((i % n) + n) % n];
      sl.catT.textContent = up ? itm.cat.toUpperCase() : itm.cat;
      sl.cat.style.background = itm.cat ? (itm.color || p('catBg')) : 'transparent';
      sl.tx.textContent = up ? itm.text.toUpperCase() : itm.text;
      sl.tx.style.transform = ''; sl.tx.style.fontSize = '';
      const avail = Math.max(1, w - catW - 36), sw = sl.tx.scrollWidth;
      if (sw > avail) {
        let s = avail / sw;
        if (s < 0.6) { sl.tx.style.fontSize = (p('size') * s / 0.6) + 'px'; s = Math.min(1, avail / sl.tx.scrollWidth); }
        sl.tx.style.transform = `scaleX(${Math.max(0.6, s)})`;
      }
    };
    const hold = Math.max(1, p('hold'));
    const el_s = Math.max(0, (now - this.wallStart) / 1000);
    const idx = Math.floor(el_s / hold), ph = el_s - idx * hold;
    const A = it.slots[0], B = it.slots[1];
    fill(A, idx); fill(B, idx + 1);
    const catOf = (i) => items[((i % n) + n) % n].cat + '#' + items[((i % n) + n) % n].color;
    const vis = (sl, on) => { sl.cat.style.visibility = sl.box.style.visibility = on ? 'visible' : 'hidden'; };
    const reset = (sl) => { for (const e of [sl.cat, sl.box]) { e.style.transform = ''; e.style.clipPath = ''; } };
    reset(A); reset(B); vis(A, true); vis(B, false); it.wiper.style.display = 'none';
    if (n < 2) return;
    if (mode === 'roll') {
      const T = Math.min(0.55, hold * 0.3);
      if (ph > hold - T) {
        const u = easeIO((ph - (hold - T)) / T);
        vis(B, true);
        A.box.style.transform = `translateY(${-u * h}px)`;
        B.box.style.transform = `translateY(${(1 - u) * h}px)`;
        if (catOf(idx) !== catOf(idx + 1)) {
          A.cat.style.transform = `translateY(${-u * h}px)`;
          B.cat.style.transform = `translateY(${(1 - u) * h}px)`;
        } else B.cat.style.visibility = 'hidden';
      }
    } else {
      // wipe: odsłonięcie w lewo->prawo, na końcu zmazanie; krawędź maski prowadzi "wycieraczka"
      const T = Math.min(0.45, hold * 0.25);
      const bw = w - catW;
      const wipeEdge = (u) => { it.wiper.style.display = 'block'; it.wiper.style.left = (catW + u * bw - 3) + 'px'; };
      if (ph < T) {
        const u = easeIO(ph / T);
        A.box.style.clipPath = `inset(0 ${(1 - u) * 100}% 0 0)`;
        if (catOf(idx) !== catOf(idx - 1)) A.cat.style.clipPath = `inset(${(1 - u) * 100}% 0 0 0)`;
        if (u < 0.999) wipeEdge(u);
      } else if (ph > hold - T) {
        const u = easeIO((ph - (hold - T)) / T);
        A.box.style.clipPath = `inset(0 0 0 ${u * 100}%)`;
        if (catOf(idx) !== catOf(idx + 1)) A.cat.style.clipPath = `inset(0 0 ${u * 100}% 0)`;
        if (u > 0.001) wipeEdge(u);
      }
    }
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
            it.mw = m.w; it.mh = m.h; it.sq = m.sq || null;
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
          if (p('tickerMode') !== 'crawl') { this.tickerSlots(it, p, now, fe); break; }
          const items = stripMarkup(String(p('text') ?? '')).split('\n').map((s) => s.trim()).filter(Boolean);
          const up = p('upper');
          const sep = p('sep'), sepColor = p('sepColor');
          const font = p('font');
          ensureFont(font, this.opts.assetBase);
          const w = it.w;
          const key = [items.join('\n'), up, sep, sepColor, p('catBg'), p('catFg'), font, p('size'), p('weight'), p('italic'), p('spacing'), w, fe].join('|');
          const S = it.strip.style;
          S.color = p('color');
          const sh = p('shadow');
          S.textShadow = sh > 0 ? `0 ${Math.max(1, sh / 4)}px ${sh}px rgba(0,0,0,.65)` : '';
          if (it.cache.key !== key) {
            it.cache.key = key;
            Object.assign(S, { position: 'absolute', inset: '', left: '0', top: '0', width: 'auto', height: '100%', display: 'flex', alignItems: 'center', whiteSpace: 'pre', fontFamily: `"${font}", "Barlow Condensed", Inter, Arial, sans-serif`, fontWeight: p('weight'), fontSize: p('size') + 'px', fontStyle: p('italic') ? 'italic' : 'normal', letterSpacing: p('spacing') + 'px' });
            it.strip.innerHTML = '';
            const one = document.createElement('span');
            for (const line of items) {
              const itm = parseTickerItem(line);
              if (itm.cat) {
                const b = document.createElement('span');
                b.textContent = up ? itm.cat.toUpperCase() : itm.cat;
                Object.assign(b.style, { background: itm.color || p('catBg'), color: p('catFg'), padding: '.05em .45em', marginRight: '.5em', borderRadius: '3px', fontSize: '.78em', verticalAlign: '.08em', textShadow: 'none' });
                one.appendChild(b);
              }
              one.appendChild(document.createTextNode(up ? itm.text.toUpperCase() : itm.text));
              const sp = document.createElement('span');
              sp.textContent = sep;
              if (sepColor) sp.style.color = sepColor;
              one.appendChild(sp);
            }
            it.strip.appendChild(one);
            const cw = Math.max(1, one.offsetWidth);
            const copies = Math.ceil(w / cw) + 1;
            for (let i = 0; i < copies; i++) it.strip.appendChild(one.cloneNode(true));
            it.cache.cw = cw;
          }
          const d = ((now - this.wallStart) / 1000) * p('speed');
          const cw = it.cache.cw || 1;
          const x = d < w ? w - d : -((d - w) % cw);
          S.transform = `translateX(${x}px)`;
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
