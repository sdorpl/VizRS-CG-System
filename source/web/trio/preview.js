// Trio – podgląd strony: zoom, kadr na element, przesuwanie.

import { fitStage } from '/renderer.js';
import { $, $$, draft, pv, sceneOf, toast } from './core.js';
import { hitField } from './fields.js';

// zoom podglądu: k – krotność względem dopasowania, x/y – przesunięcie w px pudełka
// pvAuto: kadr na element (środek na elementach strony, mały element powiększony w całości) – do ręcznego zoomu/przesunięcia
export let pvZ = { k: 1, x: 0, y: 0 }, pvFit = { base: 1, ox: 0, oy: 0 }, pvAuto = true;
// przypisanie z innych modułów (import ES jest tylko do odczytu)
export function setPvZ(v) { pvZ = v; }
export function setPvAuto(v) { pvAuto = v; }
export function fitPv() {
  const s = sceneOf(draft), W = s?.width || 1920, H = s?.height || 1080, st = $('#pvStage'), r = $('#pvBox').getBoundingClientRect();
  const base = fitStage(st, $('#pvBox'), W, H);
  pvFit = { base, ox: (r.width - W * base) / 2, oy: (r.height - H * base) / 2 };
  if (pvZ.k !== 1 || pvZ.x || pvZ.y) st.style.transform = `translate(${pvFit.ox + pvZ.x}px, ${pvFit.oy + pvZ.y}px) scale(${base * pvZ.k})`;
  $('#pvZoom').textContent = Math.round(pvZ.k * 100) + '%';
}
/** Prostokąt widocznych elementów strony w px sceny (przycięty do kadru) albo null. */
function pvBounds() {
  const s = sceneOf(draft), W = s?.width || 1920, H = s?.height || 1080, B = $('#pvBox').getBoundingClientRect();
  const S = pvFit.base * pvZ.k, tx = pvFit.ox + pvZ.x, ty = pvFit.oy + pvZ.y;
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (const it of pv?.items || []) {
    // to, co widać: tekst/zegar – sam napis (pole węzła bywa dużo szersze), grupy – przez dzieci, bez węzłów przezroczystych
    if (!it.vis || it.node.type === 'group' || +getComputedStyle(it.el).opacity === 0) continue;
    const r = (it.span && it.span.textContent ? it.span : it.el).getBoundingClientRect();
    if (r.width < 1 || r.height < 1) continue;
    x0 = Math.min(x0, (r.left - B.left - tx) / S); y0 = Math.min(y0, (r.top - B.top - ty) / S);
    x1 = Math.max(x1, (r.right - B.left - tx) / S); y1 = Math.max(y1, (r.bottom - B.top - ty) / S);
  }
  x0 = Math.max(0, x0); y0 = Math.max(0, y0); x1 = Math.min(W, x1); y1 = Math.min(H, y1);
  return x1 - x0 >= 1 && y1 - y0 >= 1 ? { x0, y0, x1, y1 } : null;
}
/** Kadr na element: środek podglądu na elementach strony; mały element powiększony tak, by zmieścił się cały (~90% podglądu). */
export function pvFrame() {
  if (!pv) return;
  const b = pvBounds(), B = $('#pvBox').getBoundingClientRect();
  if (!b || B.width < 2 || B.height < 2) return;
  const bw = (b.x1 - b.x0) * pvFit.base, bh = (b.y1 - b.y0) * pvFit.base; // rozmiar przy dopasowaniu całości
  const k = Math.min(32, Math.max(1, Math.min(0.9 * B.width / bw, 0.9 * B.height / bh)));
  const S2 = pvFit.base * k, cx = (b.x0 + b.x1) / 2, cy = (b.y0 + b.y1) / 2;
  pvZ = { k, x: B.width / 2 - cx * S2 - pvFit.ox, y: B.height / 2 - cy * S2 - pvFit.oy };
  pvAuto = true;
  fitPv();
}
let pvFrameT = 0;
/** Ponowny kadr po zmianie treści (dłuższy tekst, inna wersja) – tylko w trybie automatycznym. */
export const pvReframe = () => { if (!pvAuto) return; clearTimeout(pvFrameT); pvFrameT = setTimeout(pvFrame, 120); };
/** Zoom do krotności k, z punktem (px, py) pudełka nieruchomym. */
function pvZoomAt(k, px, py) {
  k = Math.min(32, Math.max(1, k));
  // oddalenie do minimum: cała scena, ale ze środkiem na elemencie
  if (k === 1) {
    const b = pvBounds(), B = $('#pvBox').getBoundingClientRect();
    pvZ = b ? { k: 1, x: B.width / 2 - ((b.x0 + b.x1) / 2) * pvFit.base - pvFit.ox, y: B.height / 2 - ((b.y0 + b.y1) / 2) * pvFit.base - pvFit.oy } : { k: 1, x: 0, y: 0 };
    pvAuto = false;
    fitPv();
    return;
  }
  pvAuto = false;
  const f = k / pvZ.k;
  pvZ = { k, x: px - pvFit.ox - (px - pvFit.ox - pvZ.x) * f, y: py - pvFit.oy - (py - pvFit.oy - pvZ.y) * f };
  fitPv();
}
/** Powiększa tak, by prostokąt (współrzędne ekranu) zajął ~80% podglądu. */
function pvZoomRect(r) {
  const B = $('#pvBox').getBoundingClientRect();
  if (!r || r.width < 1 || r.height < 1) return;
  const k = Math.min(32, Math.max(1, pvZ.k * Math.min(0.8 * B.width / r.width, 0.8 * B.height / r.height)));
  const S = pvFit.base * pvZ.k, tx = pvFit.ox + pvZ.x, ty = pvFit.oy + pvZ.y;
  const cx = (r.left + r.width / 2 - B.left - tx) / S, cy = (r.top + r.height / 2 - B.top - ty) / S; // środek w px sceny
  const S2 = pvFit.base * k;
  pvZ = { k, x: B.width / 2 - cx * S2 - pvFit.ox, y: B.height / 2 - cy * S2 - pvFit.oy };
  pvAuto = false;
  fitPv();
}
function pvItemAt(x, y) {
  let best = null;
  for (const it of pv?.items || []) {
    if (!it.vis) continue;
    const r = it.el.getBoundingClientRect();
    if (r.width < 3 || r.height < 3 || x < r.left || x > r.right || y < r.top || y > r.bottom) continue;
    if (!best || r.width * r.height < best.width * best.height) best = r;
  }
  return best;
}
const pvBoxPt = (e) => { const B = $('#pvBox').getBoundingClientRect(); return [e.clientX - B.left, e.clientY - B.top]; };
// przeciąganie (lewy przycisk przy powiększeniu albo środkowy) przesuwa podgląd
export let pvDrag = null, pvPanned = false;
// przypisanie z innych modułów (import ES jest tylko do odczytu)
export function setPvPanned(v) { pvPanned = v; }
const pvCenter = () => { const B = $('#pvBox').getBoundingClientRect(); return [B.width / 2, B.height / 2]; };
let lastField = null;

/** Zdarzenia i stan początkowy; woła main.js, moduły w kolejności dawnego skryptu. */
export function init() {
  $('#pvBox').addEventListener('wheel', (e) => { e.preventDefault(); pvZoomAt(pvZ.k * Math.exp(-e.deltaY * 0.0015), ...pvBoxPt(e)); }, { passive: false });
  $('#pvBox').addEventListener('dblclick', (e) => { const h = hitField(e.clientX, e.clientY); const r = h?.r || pvItemAt(e.clientX, e.clientY); if (r) pvZoomRect(r); else pvFrame(); });
  $('#pvBox').addEventListener('mousedown', (e) => {
    if (e.button === 1 || (e.button === 0 && pvZ.k > 1)) { pvDrag = { x: e.clientX, y: e.clientY, zx: pvZ.x, zy: pvZ.y, on: e.button === 1 }; pvPanned = false; if (e.button === 1) e.preventDefault(); }
  });
  addEventListener('mousemove', (e) => {
    if (!pvDrag) return;
    const dx = e.clientX - pvDrag.x, dy = e.clientY - pvDrag.y;
    if (!pvDrag.on && Math.hypot(dx, dy) < 4) return;
    pvDrag.on = pvPanned = true; $('#pvBox').classList.add('pan');
    pvZ.x = pvDrag.zx + dx; pvZ.y = pvDrag.zy + dy; pvAuto = false; fitPv();
  });
  addEventListener('mouseup', () => { pvDrag = null; $('#pvBox').classList.remove('pan'); });
  $('#pvZIn').onclick = () => pvZoomAt(pvZ.k * 1.5, ...pvCenter());
  $('#pvZOut').onclick = () => pvZoomAt(pvZ.k / 1.5, ...pvCenter());
  $('#pvZFit').onclick = () => { pvZ = { k: 1, x: 0, y: 0 }; pvAuto = false; fitPv(); };
   // cały kadr 1920×1080
  $('#pvZEl').onclick = () => {
    // element powiązany z ostatnio edytowanym polem (albo pierwszym polem strony)
    const fid = (document.activeElement?.closest?.('[data-f]') || lastField || $('#fields [data-f]'))?.dataset.f;
    const of = (f) => pv?.items.find((i) => i.vis && i.node.fields && Object.values(i.node.fields).includes(f));
    // pole bez własnego elementu (np. przełącznik stylu) → pierwsze pole, które ma widoczny element
    const it = of(fid) || [...$$('#fields [data-f]')].map((e) => of(e.dataset.f)).find(Boolean);
    if (it) pvZoomRect(it.el.getBoundingClientRect()); else toast('Brak widocznego elementu dla tego pola');
  };
  $('#fields').addEventListener('focusin', (e) => { const f = e.target.closest('[data-f]'); if (f) lastField = f; });
  new ResizeObserver(() => { fitPv(); if (pvAuto) pvFrame(); }).observe($('#pvBox'));
}
