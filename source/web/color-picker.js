// Wybór koloru w UI (Trio, Artist): okienko z polem barwy, suwakiem odcienia, zapisem #hex i próbkami.
// Czyste funkcje (normHex, hexToHsv, hsvToHex, contrast) działają też w Node; okienko wymaga DOM.

/** #abc / #abcd / #aabbcc / #aabbccdd → małe litery, pełny zapis; inna wartość → ''. */
export function normHex(s) {
  const v = String(s ?? '').trim().toLowerCase();
  if (/^#[0-9a-f]{3,4}$/.test(v)) return '#' + [...v.slice(1)].map((c) => c + c).join('');
  return /^#(?:[0-9a-f]{6}|[0-9a-f]{8})$/.test(v) ? v : '';
}

const rgbOf = (hex) => { const n = normHex(hex) || '#000000'; return [1, 3, 5].map((i) => parseInt(n.slice(i, i + 2), 16)); };

/** '#rrggbb' → { h: 0–360, s: 0–1, v: 0–1 } (alfa pomijana). */
export function hexToHsv(hex) {
  const [r, g, b] = rgbOf(hex).map((c) => c / 255);
  const max = Math.max(r, g, b), d = max - Math.min(r, g, b);
  let h = 0;
  if (d) h = max === r ? ((g - b) / d + 6) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return { h: h * 60, s: max ? d / max : 0, v: max };
}

export function hsvToHex({ h, s, v }) {
  const f = (n) => { const k = (n + h / 60) % 6; return v - v * s * Math.max(0, Math.min(k, 4 - k, 1)); };
  return '#' + [f(5), f(3), f(1)].map((c) => Math.round(c * 255).toString(16).padStart(2, '0')).join('');
}

/** Kontrast dwóch kolorów wg WCAG (1–21). */
export function contrast(a, b) {
  const lum = (x) => { const [r, g, bl] = rgbOf(x).map((c) => { c /= 255; return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; }); return 0.2126 * r + 0.7152 * g + 0.0722 * bl; };
  const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/** Dowolny kolor CSS (np. `red` z mapy kategorii) → #hex; nieznany → ''. */
function cssHex(v) {
  const n = normHex(v);
  if (n || !v) return n;
  const c = document.createElement('canvas').getContext('2d');
  c.fillStyle = '#000000'; c.fillStyle = v;
  return normHex(c.fillStyle);
}

const STYLE = `
.cpk { position: fixed; z-index: 1000; width: 232px; padding: 8px; display: flex; flex-direction: column; gap: 7px; font-size: 11px; color: var(--fg, #ddd);
  background: var(--panel2, #1b1d21); border: 1px solid var(--line, #333); border-radius: 6px; box-shadow: 0 8px 24px rgba(0,0,0,.55); }
.cpk-pv { display: flex; align-items: center; gap: 6px; min-height: 22px; }
.cpk-tag { padding: 2px 8px; border-radius: 3px; font-weight: 700; font-size: 12px; max-width: 150px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.cpk-warn { color: var(--acc, #f0a020); }
.cpk-sv { position: relative; height: 120px; border-radius: 4px; cursor: crosshair; touch-action: none; }
.cpk-sv i { position: absolute; width: 12px; height: 12px; margin: -6px; border-radius: 50%; border: 2px solid #fff; box-shadow: 0 0 0 1px #000; pointer-events: none; }
.cpk-hue { width: 100%; height: 12px; margin: 0; appearance: none; -webkit-appearance: none; border-radius: 6px; background: linear-gradient(90deg, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00); }
.cpk-hue::-webkit-slider-thumb { -webkit-appearance: none; width: 12px; height: 16px; border-radius: 3px; background: #fff; border: 1px solid #000; }
.cpk-hue::-moz-range-thumb { width: 12px; height: 16px; border-radius: 3px; background: #fff; border: 1px solid #000; }
.cpk-row { display: flex; gap: 4px; align-items: center; }
.cpk-hex { flex: 1; min-width: 0; font: 13px var(--mono, monospace); }
.cpk-hex.bad { outline: 1px solid #ff6a66; }
.cpk-row button { padding: 2px 6px; font-size: 11px; white-space: nowrap; }
.cpk-row button:disabled { opacity: .45; }
.cpk-sw { display: grid; grid-template-columns: repeat(10, 1fr); gap: 3px; }
.cpk-sw button { aspect-ratio: 1; min-width: 0; padding: 0; border-radius: 3px; border: 1px solid rgba(255,255,255,.3); cursor: pointer; }
.cpk-sw button:hover { outline: 1px solid var(--acc, #f0a020); }
.cpk-h { color: var(--dim, #888); font-size: 10px; margin-bottom: -4px; }
`;
const PALETTE = ['#c8102e', '#e00000', '#e8590c', '#f0a020', '#f2c400', '#1f8a4c', '#0a9396', '#0050c0', '#5f3dc4', '#111111'];
const RECENT = 'vizrs.recentColors';
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const loadRecent = () => { try { return JSON.parse(localStorage.getItem(RECENT) || '[]').map(normHex).filter(Boolean).slice(0, 10); } catch { return []; } };
const saveRecent = (c) => { try { localStorage.setItem(RECENT, JSON.stringify([c, ...loadRecent().filter((x) => x !== c)].slice(0, 10))); } catch { /* brak localStorage */ } };

let styled = false;
let closeOpen = null;

/**
 * Otwiera okienko przy elemencie `anchor`; drugie wywołanie zamyka poprzednie. Klawisze Trio (F1–F8) działają dalej.
 * @param o { value ('' = kolor bazowy), base, label, fg, palette: [kolory], onInput(hex|'') }
 */
export function openColorPicker(anchor, o) {
  closeOpen?.();
  if (!styled) { const st = document.createElement('style'); st.textContent = STYLE; document.head.appendChild(st); styled = true; }
  const base = cssHex(o.base) || '#c8102e';
  let cur = cssHex(o.value);                 // '' = bez własnego koloru
  const start = cur;
  const shown = () => cur || base;
  let hsv = hexToHsv(shown());
  const sw = (list) => list.map((c) => `<button type="button" data-c="${esc(c)}" style="background:${esc(c)}" title="${esc(c)}"></button>`).join('');
  const own = [...new Set((o.palette || []).map(cssHex).filter(Boolean))].slice(0, 10), recent = loadRecent();
  const pop = document.createElement('div');
  pop.className = 'cpk';
  pop.innerHTML = `<div class="cpk-pv"><span class="cpk-tag"></span><span class="cpk-warn"></span></div>
    <div class="cpk-sv"><i></i></div>
    <input type="range" class="cpk-hue" min="0" max="360" step="1" aria-label="Odcień">
    <div class="cpk-row"><input class="cpk-hex" spellcheck="false" maxlength="9" aria-label="Kolor szesnastkowo"><button type="button" class="cpk-reset" title="Usuń własny kolor – obowiązuje kolor sceny"><span style="display:inline-block;width:9px;height:9px;margin-right:4px;background:${esc(base)}"></span>ze sceny</button></div>
    ${own.length ? `<div class="cpk-h">W użyciu</div><div class="cpk-sw">${sw(own)}</div>` : ''}
    <div class="cpk-h">Paleta</div><div class="cpk-sw">${sw(PALETTE)}</div>
    ${recent.length ? `<div class="cpk-h">Ostatnie</div><div class="cpk-sw">${sw(recent)}</div>` : ''}`;
  document.body.appendChild(pop);
  const q = (s) => pop.querySelector(s);
  const sv = q('.cpk-sv'), hue = q('.cpk-hue'), hex = q('.cpk-hex'), tag = q('.cpk-tag');
  tag.textContent = o.label || '';

  function paint(skipHex) {
    const c = shown();
    sv.style.background = `linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, transparent), hsl(${hsv.h} 100% 50%)`;
    q('.cpk-sv i').style.cssText = `left:${hsv.s * 100}%;top:${(1 - hsv.v) * 100}%;background:${c.slice(0, 7)}`;
    hue.value = hsv.h;
    if (!skipHex) { hex.value = c; hex.classList.remove('bad'); }
    tag.style.background = c;
    tag.style.color = o.fg || '#fff';
    const low = o.fg && contrast(c, o.fg) < 3;
    q('.cpk-warn').textContent = low ? '⚠ słabo czytelny tekst' : '';
    q('.cpk-reset').disabled = !cur;
  }
  /** Zachowuje alfę (#rrggbbaa z ręcznego wpisu), bo pole barwy jej nie pokazuje. */
  const fromHsv = () => hsvToHex(hsv) + shown().slice(7);
  function set(c, skipHex) { cur = c; o.onInput?.(c); paint(skipHex); }

  const pickSv = (e) => {
    const r = sv.getBoundingClientRect();
    hsv.s = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
    hsv.v = Math.min(1, Math.max(0, 1 - (e.clientY - r.top) / r.height));
    set(fromHsv());
  };
  sv.addEventListener('pointerdown', (e) => { sv.setPointerCapture(e.pointerId); pickSv(e); });
  sv.addEventListener('pointermove', (e) => { if (sv.hasPointerCapture(e.pointerId)) pickSv(e); });
  hue.addEventListener('input', () => { hsv.h = +hue.value; set(fromHsv()); });
  hex.addEventListener('input', () => {
    const v = hex.value.trim(), n = normHex(v.startsWith('#') ? v : '#' + v);
    hex.classList.toggle('bad', !n);
    if (!n) return;
    hsv = hexToHsv(n);
    set(n, true);
  });
  pop.addEventListener('mousedown', (e) => { if (!e.target.closest('input')) e.preventDefault(); }); // fokus zostaje w polu #hex
  pop.addEventListener('click', (e) => {
    const b = e.target.closest('[data-c]');
    if (b) { hsv = hexToHsv(b.dataset.c); set(normHex(b.dataset.c)); return; }
    if (e.target.closest('.cpk-reset')) { hsv = hexToHsv(base); set(''); }
  });

  const close = () => {
    closeOpen = null;
    pop.remove();
    removeEventListener('pointerdown', outside, true);
    removeEventListener('keydown', keys, true);
    if (cur && cur !== start) saveRecent(cur);
  };
  const outside = (e) => { if (!pop.contains(e.target)) close(); };
  const keys = (e) => {
    if ((e.key === 'Escape' && !e.shiftKey) || (e.key === 'Enter' && !e.ctrlKey && !e.metaKey)) { e.preventDefault(); e.stopPropagation(); close(); anchor.isConnected && anchor.focus(); }
  };
  addEventListener('pointerdown', outside, true);
  addEventListener('keydown', keys, true);
  closeOpen = close;

  paint();
  const a = anchor.getBoundingClientRect(), p = pop.getBoundingClientRect();
  const fit = (v, size, max) => Math.max(6, Math.min(v, max - size - 6));
  pop.style.left = fit(a.left, p.width, innerWidth) + 'px';
  pop.style.top = (a.bottom + 4 + p.height > innerHeight - 6 ? fit(a.top - p.height - 4, p.height, innerHeight) : a.bottom + 4) + 'px';
  hex.focus(); hex.select();
}
