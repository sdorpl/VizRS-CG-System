// VizRS – ruch na wzór BTB CG (presety IN/OUT, efekty tekstu, pętle uwagi)
//
// Model węzła (opcjonalny, obok kluczy `anim`):
//   node.motion = {
//     in:      { type, dir, dur, delay, ease },          // wejście; czasy od początku sceny (TAKE)
//     out:     { type, dir, dur, delay, ease },          // wyjście; czasy od ostatniego stop pointu (OUT)
//     textIn:  { type, dur, delay, stagger, from },      // efekt tekstu przy wejściu (od początku sceny)
//     textOut: { type, dur, delay, stagger, from },      // efekt tekstu przy wyjściu (od ostatniego stopu)
//     loop:    { type, dur, amp, color },                 // pętla uwagi – gra na wizji, gdy grafika stoi (zegar ścienny)
//     after:   'id węzła'                                 // sekwencja: wejście zaczyna się po wejściu wskazanego węzła
//   }
// Semantyka jak w BTB CG (`cgmotion.js`, btb-cgeditor `src/anim.rs`): źródła ruchu składają się z kluczami –
// przesunięcie, obrót, pochylenie i rozmycie się sumują, skala i krycie mnożą.
// Moduł bez DOM – liczy stan; rysuje `renderer.js`.

// ---------------------------------------------------------------- krzywe (nazwy GSAP jak w CGMotion.normalizeEase)
const PI = Math.PI;
const backOut = (x, s) => { const t = x - 1; return t * t * ((s + 1) * t + s) + 1; };
const backIn = (x, s) => x * x * ((s + 1) * x - s);
function bounceOut(x) {
  const n1 = 7.5625, d1 = 2.75;
  if (x < 1 / d1) return n1 * x * x;
  if (x < 2 / d1) { x -= 1.5 / d1; return n1 * x * x + 0.75; }
  if (x < 2.5 / d1) { x -= 2.25 / d1; return n1 * x * x + 0.9375; }
  x -= 2.625 / d1; return n1 * x * x + 0.984375;
}
const elasticOut = (x) => (x <= 0 ? 0 : x >= 1 ? 1 : 2 ** (-10 * x) * Math.sin((x * 10 - 0.75) * (2 * PI / 3)) + 1);
const powOut = (x, p) => 1 - (1 - x) ** p;
const powIn = (x, p) => x ** p;
const powInOut = (x, p) => (x < 0.5 ? 0.5 * (2 * x) ** p : 1 - 0.5 * (2 * (1 - x)) ** p);
function cubicBezier(args, x) {
  if (args.includes('1.56') || args.includes('0.64')) return backOut(x, 1.7);
  if (args.includes('-0.55')) return backIn(x, 1.7);
  const v = args.split(',').map((s) => parseFloat(s));
  if (v.length !== 4 || v.some((n) => !isFinite(n)) || v[0] < 0 || v[0] > 1 || v[2] < 0 || v[2] > 1) return null;
  const [x1, y1, x2, y2] = v;
  const cx = 3 * x1, cy = 3 * y1, bx = 3 * (x2 - x1) - cx, by = 3 * (y2 - y1) - cy, ax = 1 - cx - bx, ay = 1 - cy - by;
  let lo = 0, hi = 1;
  for (let i = 0; i < 30; i++) { const t = (lo + hi) / 2; if (((ax * t + bx) * t + cx) * t < x) lo = t; else hi = t; }
  const t = (lo + hi) / 2;
  return ((ay * t + by) * t + cy) * t;
}
const ALIAS = {
  '': 'power2.out', 'ease-in': 'power2.in', 'ease-out': 'power2.out', 'ease-in-out': 'power2.inOut',
  spring: 'back.out(1.4)', 'spring-soft': 'back.out(0.8)', 'spring-hard': 'back.out(2.2)', overshoot: 'back.out(1.7)', broadcast: 'power3.out',
  'back-in': 'back.in', 'back-out': 'back.out', 'bounce-out': 'bounce.out', 'elastic-out': 'elastic.out',
};
/** Krzywa o nazwie GSAP (`power2.out`, `back.out(1.7)`, `bounce.out`, `cubic-bezier(…)`, aliasy BTB) → wartość dla x ∈ [0, 1]. */
export function ease(name, x) {
  x = Math.min(1, Math.max(0, +x || 0));
  name = String(name ?? '');
  if (name.startsWith('cubic-bezier(')) { const y = cubicBezier(name.slice(13).replace(/\)$/, ''), x); return y ?? powOut(x, 2); }
  if (name === 'linear' || name === 'none') return x;
  name = ALIAS[name] ?? name;
  const m = /^([a-zA-Z0-9.]+)(?:\(([-\d.]+)\))?$/.exec(name);
  const kind = m ? m[1] : name, arg = m && m[2] != null ? parseFloat(m[2]) : null;
  switch (kind) {
    case 'power1.out': case 'quad.out': return powOut(x, 2);
    case 'power1.in': case 'quad.in': return powIn(x, 2);
    case 'power1.inOut': case 'quad.inOut': return powInOut(x, 2);
    case 'sine.out': return Math.sin(x * PI / 2);
    case 'sine.in': return 1 - Math.cos(x * PI / 2);
    case 'sine.inOut': return -(Math.cos(PI * x) - 1) / 2;
    case 'power2.out': case 'cubic.out': return powOut(x, 3);
    case 'power2.in': case 'cubic.in': return powIn(x, 3);
    case 'power2.inOut': case 'cubic.inOut': return powInOut(x, 3);
    case 'power3.out': case 'quart.out': return powOut(x, 4);
    case 'power3.in': case 'quart.in': return powIn(x, 4);
    case 'power3.inOut': case 'quart.inOut': return powInOut(x, 4);
    case 'power4.out': case 'quint.out': return powOut(x, 5);
    case 'power4.in': case 'quint.in': return powIn(x, 5);
    case 'power4.inOut': case 'quint.inOut': return powInOut(x, 5);
    case 'expo.out': return x >= 1 ? 1 : 1 - 2 ** (-10 * x);
    case 'expo.in': return x <= 0 ? 0 : 2 ** (10 * x - 10);
    case 'back.out': return backOut(x, arg ?? 1.70158);
    case 'back.in': return backIn(x, arg ?? 1.70158);
    case 'back.inOut': { const s = (arg ?? 1.70158) * 1.525; return x < 0.5 ? ((2 * x) ** 2 * ((s + 1) * 2 * x - s)) / 2 : ((2 * x - 2) ** 2 * ((s + 1) * (x * 2 - 2) + s) + 2) / 2; }
    case 'bounce.out': return bounceOut(x);
    case 'bounce.in': return 1 - bounceOut(1 - x);
    case 'elastic.out': return elasticOut(x);
    case 'elastic.in': return 1 - elasticOut(1 - x);
    default: return powOut(x, 3);
  }
}

// ---------------------------------------------------------------- katalogi (nazwy jak w BTB CG; etykiety dla Artist)
export const ANIM_TYPES = ['none', 'slide', 'fade', 'zoom', 'zoom-in', 'pop', 'wipe', 'wipe-y', 'clip', 'blur', 'flip', 'flip-x', 'rotate', 'skew', 'drop', 'glitch', 'slide-overshoot', 'split-reveal', 'mask-diag', 'shutter', 'bar-grow', 'rgb-split'];
export const ANIM_LABELS = {
  none: 'brak (od razu)', slide: 'wjazd', fade: 'przenikanie', zoom: 'zoom', 'zoom-in': 'zoom z bliska', pop: 'wyskok', wipe: 'przesłona pozioma',
  'wipe-y': 'przesłona pionowa', clip: 'przycięcie', blur: 'rozmycie', flip: 'odwrócenie Y', 'flip-x': 'odwrócenie X', rotate: 'obrót', skew: 'skos',
  drop: 'spadanie', glitch: 'glitch', 'slide-overshoot': 'wjazd z przeskokiem', 'split-reveal': 'rozsunięcie', 'mask-diag': 'maska ukośna',
  shutter: 'żaluzja', 'bar-grow': 'rosnący pasek', 'rgb-split': 'rozdzielenie RGB',
};
/** Typy, dla których kierunek ma znaczenie. */
export const DIR_TYPES = new Set(['slide', 'wipe', 'wipe-y', 'clip', 'flip-x', 'rotate', 'skew', 'drop', 'slide-overshoot', 'split-reveal', 'mask-diag']);
export const DIRECTIONS = ['left', 'right', 'top', 'bottom'];
export const DIR_LABELS = { left: '← z lewej', right: '→ z prawej', top: '↑ z góry', bottom: '↓ z dołu' };
export const MOTION_EASES = ['power2.out', 'power2.in', 'power2.inOut', 'power3.out', 'power4.out', 'expo.out', 'sine.inOut', 'linear', 'spring', 'overshoot', 'broadcast', 'back.out', 'back.in', 'bounce.out', 'elastic.out'];
export const TEXT_IN = ['none', 'reveal', 'typewriter', 'typewriter-caret', 'clip-up', 'fade', 'blur', 'scale', 'slide-up', 'slide-down', 'slide-left', 'slide-right',
  'words-up', 'words-down', 'words-fade', 'words-blur', 'words-scale', 'words-clip', 'chars-up', 'chars-down', 'chars-fade', 'chars-blur', 'chars-scale', 'chars-clip',
  'lines-up', 'lines-down', 'lines-fade', 'lines-scale', 'lines-clip', 'scramble', 'counter'];
export const TEXT_OUT = ['none', 'fade', 'blur', 'scale', 'hide', 'clip-down', 'slide-up', 'slide-down', 'slide-left', 'slide-right',
  'words-up', 'words-down', 'words-fade', 'words-blur', 'words-scale', 'words-clip', 'chars-up', 'chars-down', 'chars-fade', 'chars-blur', 'chars-scale', 'chars-clip',
  'lines-up', 'lines-down', 'lines-fade', 'lines-scale', 'lines-clip'];
const UNIT_PL = { words: 'słowa', chars: 'znaki', lines: 'linie' };
const VAR_PL = { up: 'w górę', down: 'w dół', fade: 'przenikanie', blur: 'rozmycie', scale: 'skala', clip: 'z maski' };
export function textLabel(t) {
  const fixed = {
    none: 'brak', reveal: 'odsłonięcie', typewriter: 'maszyna do pisania', 'typewriter-caret': 'maszyna z kursorem', 'clip-up': 'z maski w górę', 'clip-down': 'do maski w dół',
    fade: 'przenikanie', blur: 'rozmycie', scale: 'skala', hide: 'zasłonięcie', 'slide-up': 'w górę', 'slide-down': 'w dół', 'slide-left': 'w lewo', 'slide-right': 'w prawo',
    scramble: 'szyfrowanie', counter: 'licznik liczby',
  };
  if (fixed[t]) return fixed[t];
  const [u, v] = String(t).split('-');
  return UNIT_PL[u] ? `${UNIT_PL[u]}: ${VAR_PL[v] || v}` : t;
}
export const LOOP_TYPES = ['none', 'pulse', 'breathe', 'heartbeat', 'float', 'bounce', 'wobble', 'shake', 'flash', 'glow', 'shine', 'underline-sweep'];
export const LOOP_LABELS = {
  none: 'brak', pulse: 'pulsowanie', breathe: 'oddychanie', heartbeat: 'bicie serca', float: 'unoszenie', bounce: 'podskakiwanie', wobble: 'chwianie',
  shake: 'drżenie', flash: 'błyski', glow: 'poświata', shine: 'połysk', 'underline-sweep': 'przesuwane podkreślenie',
};
export const STAGGER_FROM = ['start', 'end', 'center', 'edges'];

// ---------------------------------------------------------------- stan IN / OUT
const SLIDE_X = 1920, SLIDE_Y = 1080;
/** Stan neutralny: nic nie zmienia. `clip` = widoczny obszar [l, t, r, b] w ułamkach pola; `diag` < 0 = brak maski ukośnej. */
export const identity = () => ({ tx: 0, ty: 0, sx: 1, sy: 1, rot: 0, skew: 0, opacity: 1, origin: null, clip: null, blur: 0, flipX: 0, flipY: 0, shutter: 1, diag: -1, diagR: false, rgb: 0 });
const clipInset = (dir, a) => ({ right: [a, 0, 1, 1], top: [0, 0, 1, 1 - a], bottom: [0, a, 1, 1] })[dir] || [0, 0, 1 - a, 1];
const clipCenter = (dir, a) => { const h = (1 - a) * 0.5; return dir === 'top' || dir === 'bottom' ? [0, 0.5 - h, 1, 0.5 + h] : [0.5 - h, 0, 0.5 + h, 1]; };
const slideVec = (dir) => ({ right: [SLIDE_X, 0], top: [0, -SLIDE_Y], bottom: [0, SLIDE_Y] })[dir] || [-SLIDE_X, 0];
const opposite = (dir) => ({ left: 'right', right: 'left', top: 'bottom' })[dir] || 'top';
const clamp01 = (v) => Math.min(1, Math.max(0, v));

/** Stan animacji IN przy postępie p ∈ [0, 1] (0 = przed wejściem, 1 = na miejscu). */
export function stateIn(type, dir, easeName, p) {
  const t = identity(), m = ease(easeName, p);
  switch (type) {
    case 'none': case '': case undefined: break;
    case 'fade': t.opacity = m; break;
    case 'zoom': t.opacity = m; t.sx = t.sy = 0.8 + 0.2 * m; break;
    case 'zoom-in': t.opacity = m; t.sx = t.sy = 1.15 - 0.15 * m; break;
    case 'wipe': t.origin = [dir === 'right' ? 1 : 0, 0.5]; t.opacity = m; t.sx = Math.max(0.0001, m); break;
    case 'wipe-y': t.origin = [0.5, dir === 'bottom' ? 1 : 0]; t.opacity = m; t.sy = Math.max(0.0001, m); break;
    case 'flip': t.origin = [0, 0.5]; t.opacity = m; t.flipY = 90 * (1 - m); break;
    case 'flip-x': t.origin = [0.5, dir === 'bottom' ? 1 : 0]; t.opacity = m; t.flipX = 90 * (1 - m); break;
    case 'clip': t.clip = clipInset(dir, 1 - m); break;
    case 'blur': t.opacity = m; t.blur = 24 * (1 - m); break;
    case 'pop': { const k = ease('back.out(2)', p); t.opacity = clamp01(k); t.sx = t.sy = 0.3 + 0.7 * k; break; }
    case 'drop': {
      const k = ease('bounce.out', p);
      const [dx, dy] = ({ bottom: [0, 160], left: [-160, 0], right: [160, 0] })[dir] || [0, -160];
      t.opacity = clamp01(k); t.tx = dx * (1 - k); t.ty = dy * (1 - k); break;
    }
    case 'rotate': { const r = dir === 'right'; t.origin = [r ? 1 : 0, 1]; t.opacity = m; t.rot = (r ? 90 : -90) * (1 - m); break; }
    case 'skew': { const r = dir === 'right'; t.opacity = m; t.tx = (r ? SLIDE_X : -SLIDE_X) * (1 - m); t.skew = (r ? 25 : -25) * (1 - m); break; }
    case 'slide-overshoot': { const k = ease('back.out(1.2)', p); const [dx, dy] = slideVec(dir); t.opacity = clamp01(k); t.tx = dx * (1 - k); t.ty = dy * (1 - k); break; }
    case 'split-reveal': t.clip = clipCenter(dir, 1 - m); break;
    case 'mask-diag': t.diag = m; t.diagR = dir === 'right'; break;
    case 'shutter': t.shutter = m; break;
    case 'bar-grow': t.origin = [0.5, 0.5]; t.sy = Math.max(0.0001, 0.04 + 0.96 * m); break;
    case 'rgb-split': t.opacity = m; t.tx = -14 * (1 - m); t.rgb = 1 - m; break;
    case 'glitch': {
      const slot = Math.floor(p * 7);
      const [o, x] = [[0, 0], [1, -12], [0.2, 9], [1, -5], [0.4, 4], [0.4, 4]][slot] || [1, 0];
      t.opacity = o; t.tx = x; t.rgb = slot < 6 ? 0.6 : 0; break;
    }
    default: { // slide
      const [dx, dy] = slideVec(dir);
      t.opacity = m; t.tx = dx * (1 - m); t.ty = dy * (1 - m);
    }
  }
  return t;
}

/** Stan animacji OUT przy postępie p ∈ [0, 1] (0 = na miejscu, 1 = schowany). */
export function stateOut(type, dir, easeName, p) {
  const t = identity(), m = ease(easeName, p);
  switch (type) {
    case 'none': case '': case undefined: t.opacity = 0; break; // jak cgmotion: od razu znika (po opóźnieniu)
    case 'fade': t.opacity = 1 - m; break;
    case 'zoom': t.opacity = 1 - m; t.sx = t.sy = 1 - 0.2 * m; break;
    case 'zoom-in': t.opacity = 1 - m; t.sx = t.sy = 1 + 0.15 * m; break;
    case 'wipe': t.origin = [dir === 'left' ? 0 : 1, 0.5]; t.opacity = 1 - m; t.sx = Math.max(0.0001, 1 - m); break;
    case 'wipe-y': t.origin = [0.5, dir === 'top' ? 0 : 1]; t.opacity = 1 - m; t.sy = Math.max(0.0001, 1 - m); break;
    case 'flip': t.origin = [1, 0.5]; t.opacity = 1 - m; t.flipY = -90 * m; break;
    case 'flip-x': t.origin = [0.5, dir === 'top' ? 0 : 1]; t.opacity = 1 - m; t.flipX = -90 * m; break;
    case 'clip': t.clip = clipInset(opposite(dir), m); break;
    case 'blur': t.opacity = 1 - m; t.blur = 24 * m; break;
    case 'pop': { const k = ease('back.in(2)', p); t.opacity = clamp01(1 - k); t.sx = t.sy = 1 - 0.7 * k; break; }
    case 'drop': {
      const k = ease('power2.in', p);
      const [dx, dy] = ({ top: [0, -120], left: [-120, 0], right: [120, 0] })[dir] || [0, 120];
      t.opacity = 1 - k; t.tx = dx * k; t.ty = dy * k; break;
    }
    case 'rotate': { const r = dir === 'right'; t.origin = [r ? 1 : 0, 1]; t.opacity = 1 - m; t.rot = (r ? 90 : -90) * m; break; }
    case 'skew': { const r = dir === 'right'; t.opacity = 1 - m; t.tx = (r ? SLIDE_X : -SLIDE_X) * m; t.skew = (r ? -25 : 25) * m; break; }
    case 'slide-overshoot': { const k = ease('back.in(1.2)', p); const [dx, dy] = slideVec(dir); t.opacity = clamp01(1 - k); t.tx = dx * k; t.ty = dy * k; break; }
    case 'split-reveal': t.clip = clipCenter(dir, m); break;
    case 'mask-diag': t.diag = 1 - m; t.diagR = dir === 'right'; break;
    case 'shutter': t.shutter = 1 - m; break;
    case 'bar-grow': t.origin = [0.5, 0.5]; t.sy = Math.max(0.0001, 1 - 0.96 * m); break;
    case 'rgb-split': t.opacity = 1 - m; t.tx = 14 * m; t.rgb = m; break;
    case 'glitch': {
      const slot = Math.floor(p * 6);
      const [o, x] = [[0.3, 10], [1, -8], [0.2, 6], [1, -3]][slot] || [0, 0];
      t.opacity = o; t.tx = x; t.rgb = slot < 4 ? 0.6 : 0; break;
    }
    default: { const [dx, dy] = slideVec(dir); t.opacity = 1 - m; t.tx = dx * m; t.ty = dy * m; }
  }
  return t;
}

const num = (v, d) => (Number.isFinite(+v) && v !== '' && v != null ? +v : d);
export const specDur = (s) => Math.max(0, num(s?.dur, 0.5));
export const specDelay = (s) => Math.max(0, num(s?.delay, 0));
const active = (s) => !!s && !!s.type && s.type !== 'none';

/** Stan IN w chwili t (s od początku sceny); przed opóźnieniem węzeł jest w stanie początkowym (zwykle ukryty). */
export function evalIn(spec, t, shift = 0) {
  if (!spec || !spec.type) return identity();
  const d = specDelay(spec) + shift;
  if (spec.type === 'none') { const s = identity(); if (t < d) s.opacity = 0; return s; }
  if (t < d) return stateIn(spec.type, spec.dir, spec.ease, 0);
  const dur = specDur(spec);
  return stateIn(spec.type, spec.dir, spec.ease, dur <= 0 ? 1 : clamp01((t - d) / dur));
}
/** Stan OUT w chwili to (s od początku wyjścia). */
export function evalOut(spec, to) {
  if (!spec || !spec.type || to < specDelay(spec)) return identity();
  const dur = specDur(spec);
  return stateOut(spec.type, spec.dir, spec.ease, dur <= 0 ? 1 : clamp01((to - specDelay(spec)) / dur));
}
/** Liczba jednostek efektu tekstu w tekście (znaki bez spacji, słowa, wiersze). */
export function textUnits(text, split) {
  const s = String(text ?? '').replace(/\{[^}]*\}|\[\[|\]\]/g, '');
  if (split === 'chars') return Math.max(1, s.replace(/\s+/g, '').length);
  if (split === 'words') return Math.max(1, s.split(/\s+/).filter(Boolean).length);
  if (split === 'lines') return Math.max(1, s.split('\n').filter((l) => l.trim()).length);
  return 1;
}
/** Czas rozłożenia jednostek (odstęp × (n−1)); bez tekstu – górna granica 1,2 s jak w cgmotion. */
export function textSpread(spec, text) {
  const split = splitOf(spec.type);
  if (split === 'whole') return 0;
  if (text == null) return 1.2;
  const n = textUnits(text, split);
  const per = num(spec.stagger, 0) > 0 ? +spec.stagger : split === 'chars' ? 0.04 : 0.09;
  return Math.min(per, 1.2 / n) * (n - 1);
}
/** Koniec wejścia węzła (s od początku sceny): IN i efekt tekstu IN. `text` – treść węzła (dla odstępów efektu). */
export function inEnd(m, shift = 0, text) {
  if (!m) return 0;
  let e = 0;
  if (m.in?.type) e = Math.max(e, shift + specDelay(m.in) + (m.in.type === 'none' ? 0 : specDur(m.in)));
  if (active(m.textIn)) e = Math.max(e, shift + specDelay(m.textIn) + specDur(m.textIn) + textSpread(m.textIn, text));
  return e;
}
/** Długość wyjścia węzła (s od ostatniego stopu). */
export function outLen(m, text) {
  if (!m) return 0;
  let e = 0;
  if (m.out?.type) e = Math.max(e, specDelay(m.out) + (m.out.type === 'none' ? 0 : specDur(m.out)));
  if (active(m.textOut)) e = Math.max(e, specDelay(m.textOut) + specDur(m.textOut) + textSpread(m.textOut, text));
  return e;
}

/**
 * Przesunięcia sekwencji („po węźle” – `motion.after`): wejście węzła zaczyna się, gdy skończy się wejście wskazanego węzła.
 * Zwraca Map<id, przesunięcie s>. Cykle i brakujące węzły → 0.
 */
export function sequenceShifts(nodes) {
  const byId = new Map();
  const visit = (list) => { for (const n of list || []) { byId.set(n.id, n); visit(n.children); } };
  visit(nodes);
  const out = new Map(), busy = new Set();
  const shift = (id) => {
    if (out.has(id)) return out.get(id);
    const n = byId.get(id);
    const a = n?.motion?.after;
    if (!a || !byId.has(a) || busy.has(id)) { out.set(id, 0); return 0; }
    busy.add(id);
    const prev = byId.get(a);
    const s = inEnd(prev.motion, shift(a), prev.props?.text);
    busy.delete(id);
    out.set(id, s);
    return s;
  };
  for (const id of byId.keys()) shift(id);
  return out;
}

// ---------------------------------------------------------------- efekty tekstu (jednostki: całość / słowa / znaki / linie)
export const splitOf = (type) => (/^words-/.test(type) ? 'words' : /^chars-/.test(type) || type === 'scramble' || type === 'typewriter-caret' ? 'chars' : /^lines-/.test(type) ? 'lines' : 'whole');
function staggerOrder(from, i, n) {
  const last = Math.max(0, n - 1);
  if (from === 'end') return last - i;
  if (from === 'center') return Math.abs(i - last / 2);
  if (from === 'edges') return Math.max(0, last / 2 - Math.abs(i - last / 2));
  return i;
}
/** Neutralny stan jednostki tekstu: revealX/revealY = widoczna część (1 = całość), clipY = przesunięcie w masce (0..1 wysokości). */
export const unitIdentity = () => ({ opacity: 1, dx: 0, dy: 0, scale: 1, blur: 0, revealX: 1, revealY: 1, clipY: 0, hidden: false });

function unitProgress(spec, i, n, t) {
  const split = splitOf(spec.type);
  const per = num(spec.stagger, 0) > 0 ? +spec.stagger : split === 'chars' ? 0.04 : 0.09;
  const stagger = split === 'whole' ? 0 : Math.min(per, 1.2 / Math.max(1, n));
  const local = t - stagger * staggerOrder(spec.from, i, n);
  const dur = specDur(spec);
  return dur <= 0 ? (local >= 0 ? 1 : 0) : clamp01(local / dur);
}
/** Stan jednostki i z n przy efekcie IN; t = s od startu efektu (po opóźnieniu może być ujemne). */
export function textInFx(spec, i, n, t) {
  const fx = unitIdentity(), type = spec.type;
  const p = unitProgress(spec, i, n, t), m = ease(spec.ease || 'power2.out', p);
  const variant = type.split('-')[1] || '';
  switch (type) {
    case 'reveal': case 'typewriter': fx.revealX = m; break;
    case 'typewriter-caret': fx.hidden = p <= 0; break;
    case 'clip-up': fx.clipY = 1 - m; break;
    case 'fade': fx.opacity = m; break;
    case 'blur': fx.opacity = m; fx.blur = 10 * (1 - m); break;
    case 'scale': fx.opacity = m; fx.scale = 0.5 + 0.5 * m; break;
    case 'slide-up': fx.opacity = m; fx.dy = 50 * (1 - m); break;
    case 'slide-down': fx.opacity = m; fx.dy = -50 * (1 - m); break;
    case 'slide-left': fx.opacity = m; fx.dx = 50 * (1 - m); break;
    case 'slide-right': fx.opacity = m; fx.dx = -50 * (1 - m); break;
    case 'scramble': fx.scramble = p < 1; break;
    case 'counter': fx.count = m; break;
    default:
      if (splitOf(type) === 'whole') break;
      if (variant === 'up') { fx.opacity = m; fx.dy = 40 * (1 - m); }
      else if (variant === 'down') { fx.opacity = m; fx.dy = -40 * (1 - m); }
      else if (variant === 'scale') { fx.opacity = m; fx.scale = 0.3 + 0.7 * m; }
      else if (variant === 'clip') fx.clipY = 1 - m;
      else if (variant === 'blur') { fx.opacity = m; fx.blur = 8 * (1 - m); }
      else fx.opacity = m;
  }
  return fx;
}
/** Stan jednostki przy efekcie OUT; t = s od startu efektu. */
export function textOutFx(spec, i, n, t) {
  const fx = unitIdentity(), type = spec.type;
  const p = unitProgress(spec, i, n, t), m = ease(spec.ease || 'power2.in', p);
  const variant = type.split('-')[1] || '';
  switch (type) {
    case 'fade': fx.opacity = 1 - m; break;
    case 'blur': fx.opacity = 1 - m; fx.blur = 10 * m; break;
    case 'scale': fx.opacity = 1 - m; fx.scale = 1 - 0.5 * m; break;
    case 'hide': fx.revealX = 1 - m; break;
    case 'slide-up': fx.opacity = 1 - m; fx.dy = -50 * m; break;
    case 'slide-down': fx.opacity = 1 - m; fx.dy = 50 * m; break;
    case 'slide-left': fx.opacity = 1 - m; fx.dx = -50 * m; break;
    case 'slide-right': fx.opacity = 1 - m; fx.dx = 50 * m; break;
    case 'clip-down': fx.clipY = m; break;
    default:
      if (splitOf(type) === 'whole') break;
      if (variant === 'up') { fx.opacity = 1 - m; fx.dy = -40 * m; }
      else if (variant === 'down') { fx.opacity = 1 - m; fx.dy = 40 * m; }
      else if (variant === 'scale') { fx.opacity = 1 - m; fx.scale = 1 - 0.7 * m; }
      else if (variant === 'clip') fx.clipY = -m;
      else if (variant === 'blur') { fx.opacity = 1 - m; fx.blur = 8 * m; }
      else fx.opacity = 1 - m;
  }
  return fx;
}

/** Licznik: pierwsza liczba w tekście liczy od 0 do wartości z zachowaniem formatu (jak `_counterIn` w cgmotion). */
export function counterText(full, k) {
  const m = /-?\d[\d\s.,]*\d|-?\d/.exec(full);
  if (!m) return full;
  const raw = m[0];
  const sep = /[.,](\d+)$/.exec(raw);
  const isDec = !!sep && sep[1].length !== 3;
  const decimals = isDec ? sep[1].length : 0;
  const decSep = isDec ? raw[raw.length - decimals - 1] : '.';
  const intPart = isDec ? raw.slice(0, raw.length - decimals - 1) : raw;
  const gm = /\d([\s.,])\d{3}(?!\d)/.exec(intPart);
  const grp = gm ? gm[1] : '';
  const target = parseFloat(intPart.replace(/[\s.,]/g, '') + (isDec ? '.' + sep[1] : ''));
  if (!isFinite(target)) return full;
  let [i, d] = (target * k).toFixed(decimals).split('.');
  if (grp) i = i.replace(/\B(?=(\d{3})+(?!\d))/g, grp);
  return full.slice(0, m.index) + (d ? i + decSep + d : i) + full.slice(m.index + raw.length);
}
const SCRAMBLE = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#%&*@$';
/** Znak zastępczy szyfrowania – deterministyczny w czasie (ten sam kadr = ten sam znak). */
export const scrambleChar = (i, t) => SCRAMBLE[Math.abs(Math.floor(Math.sin((i + 1) * 12.9898 + Math.floor(t * 20) * 78.233) * 43758.5453)) % SCRAMBLE.length];

// ---------------------------------------------------------------- pętle uwagi (CGMotion.ATTENTION_PRESETS)
const yoyo = (t, d) => { const ph = (t / d) % 2; const lin = ph <= 1 ? ph : 2 - ph; return 0.5 - 0.5 * Math.cos(PI * lin); };
/**
 * Stan pętli w chwili t (s zegara ściennego od wejścia na wizję). `amp` – siła jak „amplituda” w inspektorze BTB
 * (pulse/heartbeat: % skali, wobble: stopnie, breathe: % krycia); puste = domyślna.
 * Zwraca { tx, ty, sx, sy, rot, opacity, glow (0..1), shine (pozycja 0..1 lub -1), underline [od, do] lub null }.
 */
export function loopState(spec, t) {
  const o = { tx: 0, ty: 0, sx: 1, sy: 1, rot: 0, opacity: 1, glow: 0, shine: -1, underline: null };
  if (!spec || !spec.type || spec.type === 'none') return o;
  const D = (d) => (num(spec.dur, 0) > 0 ? +spec.dur : d);
  const amp = spec.amp === '' || spec.amp == null ? null : num(spec.amp, null);
  switch (spec.type) {
    case 'pulse': o.sx = o.sy = 1 + (amp != null ? amp / 100 : 0.04) * yoyo(t, D(0.8)); break;
    case 'float': o.ty = -(amp ?? 6) * yoyo(t, D(2.2)); break;
    case 'wobble': { const a = amp ?? 1.2; o.rot = -a + 2 * a * yoyo(t, D(0.9)); break; }
    case 'breathe': o.opacity = 1 - (amp != null ? clamp01(amp / 100) : 0.4) * yoyo(t, D(3)); break;
    case 'glow': o.glow = yoyo(t, D(1.5)); break;
    case 'flash': {
      const d = D(0.5), cyc = d * 3 + 2, c = t % cyc;
      if (c < d * 3) { const k = (c % d) / d; o.opacity = k < 0.5 ? 1 - k * 2 : (k - 0.5) * 2; }
      break;
    }
    case 'shake': {
      const st = [-1, 1, -1, 1, -0.5, 0.5, 0], d = D(0.08), cyc = d * 7 + 3, c = t % cyc;
      if (c < d * 7) o.tx = st[Math.min(6, Math.floor(c / d))] * (amp ?? 5);
      break;
    }
    case 'heartbeat': {
      const c = t % (0.64 + 1.2), s = 1 + (amp != null ? amp / 100 : 0.06);
      const k = c < 0.12 ? 1 + (s - 1) * (c / 0.12) : c < 0.24 ? s - (s - 1) * ((c - 0.12) / 0.12)
        : c < 0.34 ? 1 + (s * 0.99 - 1) * ((c - 0.24) / 0.1) : c < 0.64 ? s * 0.99 - (s * 0.99 - 1) * ((c - 0.34) / 0.3) : 1;
      o.sx = o.sy = k; break;
    }
    case 'bounce': {
      const c = t % (0.7 + 2), a = amp ?? 10;
      if (c < 0.25) o.ty = -a * ease('power2.out', c / 0.25);
      else if (c < 0.7) o.ty = -a * (1 - ease('bounce.out', (c - 0.25) / 0.45));
      break;
    }
    case 'shine': { const d = D(1.2), c = t % (d + 2.5); o.shine = c < d ? ease('power1.inOut', c / d) : -1; break; }
    case 'underline-sweep': {
      const d = D(1), c = t % (d + 2);
      if (c < d * 0.6) o.underline = [0, ease('power2.out', c / (d * 0.6))];
      else if (c < d) o.underline = [ease('power2.in', (c - d * 0.6) / (d * 0.4)), 1];
      break;
    }
  }
  return o;
}

// ---------------------------------------------------------------- kompozycja
/** Składa stany (IN · OUT): przesunięcia, obrót, pochylenie, rozmycie, RGB sumują się; skala i krycie mnożą; maski – część wspólna. */
export function compose(a, b) {
  const r = { ...a };
  r.tx += b.tx; r.ty += b.ty; r.rot += b.rot; r.skew += b.skew; r.blur += b.blur; r.rgb += b.rgb; r.flipX += b.flipX; r.flipY += b.flipY;
  r.sx *= b.sx; r.sy *= b.sy; r.opacity *= b.opacity; r.shutter *= b.shutter;
  r.origin = b.origin || a.origin;
  if (b.clip) r.clip = a.clip ? [Math.max(a.clip[0], b.clip[0]), Math.max(a.clip[1], b.clip[1]), Math.min(a.clip[2], b.clip[2]), Math.min(a.clip[3], b.clip[3])] : b.clip;
  if (b.diag >= 0) { r.diag = a.diag >= 0 ? Math.min(a.diag, b.diag) : b.diag; r.diagR = b.diagR; }
  return r;
}

/**
 * Stan ruchu węzła w chwili t sceny. `outStart` – początek wyjścia (ostatni stop) albo Infinity, `shift` – przesunięcie sekwencji.
 * Zwraca stan (identity, gdy węzeł nie ma ruchu).
 */
export function motionAt(m, t, outStart, shift = 0) {
  if (!m) return identity();
  let s = m.in?.type ? evalIn(m.in, Math.min(t, outStart), shift) : identity();
  if (m.out?.type && t > outStart) s = compose(s, evalOut(m.out, t - outStart));
  return s;
}

/** Czy węzeł stoi (koniec wejścia minął, wyjście nie zaczęte) – wtedy grają pętle uwagi. */
export const holding = (m, t, outStart, shift = 0, text) => t >= inEnd(m, shift, text) - 1e-6 && t < outStart;

/** Czy w danej chwili efekt tekstu zmienia wygląd (poza nim tekst ma zwykły, dokładny układ). */
export function textFxActive(m, t, outStart, shift = 0) {
  if (!m) return false;
  if (active(m.textIn) && t < outStart && t < shift + specDelay(m.textIn) + specDur(m.textIn) + 1.2) return true;
  if (active(m.textOut) && t > outStart + specDelay(m.textOut) - 1e-6) return true;
  return false;
}

// ---------------------------------------------------------------- presety → klucze (jak „Presety na klucze” w BTB CG Editor)
/** Krzywa GSAP → krzywa kluczy VizRS (EASES w renderer.js), gdy przebieg jest identyczny; inaczej null (próbkowanie). */
const KEY_EASE = { 'power2.out': 'out', 'cubic.out': 'out', 'ease-out': 'out', '': 'out', 'power2.in': 'in', 'cubic.in': 'in', 'ease-in': 'in', 'power2.inOut': 'inout', 'cubic.inOut': 'inout', 'ease-in-out': 'inout', linear: 'linear', none: 'linear', 'back.out': 'back', 'back-out': 'back' };
/** Typy, w których każda wartość zależy liniowo od krzywej głównej – wystarczą dwa klucze z tą samą krzywą. */
const LINEAR_TYPES = new Set(['fade', 'zoom', 'zoom-in', 'clip', 'blur', 'slide', 'skew', 'split-reveal']);
/** Typy, których nie da się zapisać kluczami VizRS (osobna skala X/Y z punktem odniesienia, 3D, maski specjalne, RGB). */
const UNKEYABLE = new Set(['wipe', 'wipe-y', 'flip', 'flip-x', 'rotate', 'mask-diag', 'shutter', 'bar-grow', 'rgb-split', 'glitch']);

/**
 * Zamienia presety wejścia/wyjścia węzła na klucze. `rest` – wartości spoczynkowe właściwości (x, y, opacity, scale, rotation, skewX,
 * blur, clipL…), `outStart` – ostatni stop, `shift` – przesunięcie sekwencji. Zwraca { anim: {prop: [klucze]}, done: ['in','out'], skipped: [opisy] }.
 * Pętle i efekty tekstu zostają presetami (jak w BTB).
 */
export function motionToKeys(m, { rest, outStart, shift = 0 }) {
  const anim = {}, done = [], skipped = [];
  const phases = [];
  if (m?.in?.type && m.in.type !== 'none') phases.push(['in', m.in, shift + specDelay(m.in)]);
  if (m?.out?.type && m.out.type !== 'none') {
    if (Number.isFinite(outStart)) phases.push(['out', m.out, outStart + specDelay(m.out)]);
    else skipped.push('wyjście: brak stop pointu');
  }
  const r3 = (v) => Math.round(v * 1000) / 1000;
  const map = (s) => {
    if (Math.abs(s.sx - s.sy) > 1e-6) return null;
    const o = { x: rest.x + s.tx, y: rest.y + s.ty, opacity: rest.opacity * s.opacity, scale: rest.scale * s.sx, skewX: rest.skewX + s.skew, blur: rest.blur + s.blur };
    const c = s.clip || [0, 0, 1, 1];
    o.clipL = Math.max(rest.clipL, c[0] * 100); o.clipT = Math.max(rest.clipT, c[1] * 100); o.clipR = Math.max(rest.clipR, 100 - c[2] * 100); o.clipB = Math.max(rest.clipB, 100 - c[3] * 100);
    return o;
  };
  for (const [kind, spec, t0] of phases) {
    if (UNKEYABLE.has(spec.type)) { skipped.push(`${kind === 'in' ? 'wejście' : 'wyjście'} „${ANIM_LABELS[spec.type] || spec.type}” nie ma odpowiednika w kluczach`); continue; }
    const dur = specDur(spec);
    const st = (p) => (kind === 'in' ? stateIn : stateOut)(spec.type, spec.dir, spec.ease, p);
    const ke = KEY_EASE[spec.ease ?? ''] ?? null;
    const nS = Math.max(2, Math.ceil(dur * 30) + 1);
    const ps = LINEAR_TYPES.has(spec.type) && ke ? [0, 1] : Array.from({ length: nS }, (_, i) => i / (nS - 1));
    const e = ps.length === 2 ? ke : 'linear';
    const samples = ps.map((p) => [r3(t0 + p * dur), map(st(p))]);
    if (samples.some(([, v]) => !v)) { skipped.push(`${kind}: różna skala X/Y`); continue; }
    for (const prop of Object.keys(samples[0][1])) {
      const vals = samples.map(([, v]) => r3(v[prop]));
      if (vals.every((v) => Math.abs(v - rest[prop]) < 1e-6)) continue; // właściwość bez zmian w tej fazie
      const tr = (anim[prop] ||= []);
      samples.forEach(([t], i) => tr.push({ t, v: vals[i], e }));
    }
    done.push(kind);
  }
  // spoczynek między fazami: klucze IN kończą się na wartości spoczynkowej, OUT zaczyna od niej – sortowanie i usunięcie dubli czasu
  for (const tr of Object.values(anim)) {
    tr.sort((a, b) => a.t - b.t);
    for (let i = tr.length - 2; i >= 0; i--) if (Math.abs(tr[i + 1].t - tr[i].t) < 0.0005) tr.splice(i, 1);
  }
  return { anim, done, skipped };
}
