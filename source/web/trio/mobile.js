// Trio – telefon i wąskie okno: widoki, pasek, szuflada, instalacja.

import { choose } from '/dialogs.js';
import { setHtml as writeHtml } from '/dom.js';
import { $, $$, PHONE, antQuery, api, buzz, dirtyPage, draft, html, isNative, loadShow, loadState, onAirLayer, phone, show, state, toast } from './core.js';
import { syncPv, trySelect } from './pages.js';
import { fitPv } from './preview.js';
import { act, arm, busy } from './onair.js';

// ------------------------------------------------------------ telefon i wąskie okno (dawne /mobile, układ jak w VizRS Live)
/** Widok na telefonie: pages | edit | air | queue (na stanowisku widać wszystkie naraz). */
export let view = 'pages';
/** Monitor PGM; na telefonie tylko na widocznym widoku Antena i w 15 kl./s – wyjście w telefonie kosztuje baterię i transfer. */
export function syncPgm(force) {
  const live = !phone() || (view === 'air' && !document.hidden);
  const src = live ? '/output?checker=1' + (phone() ? '&fps=15' : '') + antQuery() : 'about:blank';
  const f = $('#pgm');
  if (!force && f.dataset.src === src) return;
  f.dataset.src = src; f.src = src;
}
// widoki: dotknięcie w pasku zakładek albo przesunięcie palcem (scroll-snap – gest ma fizykę systemu)
const VIEWS = ['pages', 'edit', 'air', 'queue'];
const pager = $('#main');
let going = null, settleT = 0;
function markView(v) { $$('#nav [data-pane-tab]').forEach((b) => b.classList.toggle('on', b.dataset.paneTab === v)); }
function activate(v) { view = v; going = null; markView(v); syncPgm(); syncPv(); }
function setView(v) {
  if (!phone()) return;
  const left = VIEWS.indexOf(v) * pager.clientWidth;
  if (Math.abs(left - pager.scrollLeft) < 2) return activate(v);
  going = v; markView(v);
  pager.scrollTo({ left, behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' });
}
/** Przełączenie stanowisko ↔ telefon: przyciski komend pod edytorem albo w pasku na dole. */
export function applyLayout() {
  const ctl = $('#ctl'), home = phone() ? $('#bar') : $('#colEdit');
  if (ctl.parentElement !== home) home.appendChild(ctl);
  closeDrawer(); document.body.classList.remove('kb');
  pager.scrollLeft = phone() ? VIEWS.indexOf(view) * pager.clientWidth : 0;
  markView(view); syncPgm(); syncPv(); renderBar(); renderTop();
  if (!phone()) requestAnimationFrame(fitPv);
}
export function renderBar() {
  const L = draft && onAirLayer(draft.id), ph = phone();
  writeHtml($('#barName'), draft ? html`<span class="num">${draft.number}</span> ${draft.name}` : html`<span class="dim">Wybierz stronę</span>`);
  $('#barSt').textContent = L ? `● ON AIR ${L.stop + 1}/${Math.max(1, L.stops)}` : dirtyPage ? '✎ zmiany' : '';
  $('#barSt').className = L ? 'on' : '';
  // na stanowisku przyciski zostają czynne jak dotąd (skróty F1–F4), na telefonie pokazują, co ma sens
  $('#bTake').disabled = $('#bUpd').disabled = ph && (busy || !draft);
  $('#bCont').disabled = $('#bOut').disabled = ph && (busy || !L);
}
export function renderTop() {
  const n = state.layers.length;
  $('#top').classList.toggle('air', n > 0);
  $('meta[name=theme-color]').content = n && phone() ? '#2a0f0e' : '#0a0b0f';
  $('#airN').textContent = n || '';
  $('#showName').textContent = show.name || '';
}
// szuflada narzędzi spod ☰ (jak menu VizRS Live na telefonie): zamyka ją dotknięcie obok, Esc i wykonana czynność
export function closeDrawer() {
  if (!document.body.classList.contains('nav-open')) return;
  document.body.classList.remove('nav-open'); $('#bMenu').setAttribute('aria-expanded', 'false');
}
// przytrzymanie strony na liście = menu strony (prawy przycisk telefonu)
let pressT = 0;
async function pageMenu() {
  if (!draft) return;
  const a = await choose(`${draft.number} ${draft.name}`, [
    { label: 'Usuń stronę…', value: 'del', danger: true },
    { label: 'Duplikuj', value: 'dup' },
    { label: 'Wyczyść Preview', value: 'pclear' },
    { label: 'ARM – na Preview', value: 'arm' },
    { label: 'Edytuj pola', value: 'edit', main: true },
  ], { title: 'Strona' });
  if (a === 'edit') setView('edit');
  if (a === 'arm') arm();
  if (a === 'pclear') act(() => api('POST', '/api/preview/clear'), 'Preview');
  if (a === 'dup') $('#pDup').onclick();
  if (a === 'del') $('#pDel').onclick();
}
// klawiatura ekranowa: pasek zakładek ustępuje miejsca polu
let kbT = 0;
// wygaszony ekran telefonu w trakcie audycji: prosimy o czuwanie (tylko HTTPS)
let wakeLock = null;
export async function keepAwake() {
  if (!phone() || wakeLock || document.hidden) return;
  try { wakeLock = await navigator.wakeLock?.request('screen'); wakeLock?.addEventListener('release', () => { wakeLock = null; }); } catch { /* tylko HTTPS */ }
}
// instalacja aplikacji (ikona na ekranie telefonu)
let installEv = null;
const installed = () => matchMedia('(display-mode: standalone)').matches || navigator.standalone === true;

/** Zdarzenia i stan początkowy; woła main.js, moduły w kolejności dawnego skryptu. */
export function init() {
  pager.addEventListener('scroll', () => {
    if (!phone()) return;
    const v = VIEWS[Math.round(pager.scrollLeft / pager.clientWidth)];
    if (!going) markView(v);
    // monitor i podgląd startują dopiero, gdy widok się zatrzyma
    clearTimeout(settleT); settleT = setTimeout(() => activate(v), 120);
  }, { passive: true });
  new ResizeObserver(() => { if (phone()) pager.scrollLeft = VIEWS.indexOf(view) * pager.clientWidth; }).observe(pager);
  $('#nav').onclick = (e) => { const b = e.target.closest('[data-pane-tab]'); if (b) setView(b.dataset.paneTab); };
  $('#barPage').onclick = () => setView('edit');
  PHONE.addEventListener('change', applyLayout);
  $('#bMenu').addEventListener('click', (e) => {
    e.stopPropagation();
    $('#bMenu').setAttribute('aria-expanded', String(document.body.classList.toggle('nav-open')));
  });
  document.addEventListener('click', (e) => { if (!e.target.closest('#tools') || e.target.closest('#tools button')) closeDrawer(); });
  $('#tools').addEventListener('change', closeDrawer);
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape') closeDrawer(); });
  $('#pages').addEventListener('pointerdown', (e) => {
    clearTimeout(pressT);
    const it = e.target.closest('.item[data-id]');
    if (!it || !phone() || e.pointerType === 'mouse') return;
    pressT = setTimeout(async () => { buzz(20); if (await trySelect(it.dataset.id)) pageMenu(); }, 500);
  });
  for (const ev of ['pointerup', 'pointercancel', 'pointerleave']) $('#pages').addEventListener(ev, () => clearTimeout(pressT));
  addEventListener('contextmenu', (e) => { if (phone() && !isNative && !e.target.closest('input, textarea')) e.preventDefault(); });
  addEventListener('focusin', (e) => {
    if (!phone() || !/^(INPUT|TEXTAREA)$/.test(e.target.tagName) || /^(checkbox|color|file|radio)$/.test(e.target.type)) return;
    clearTimeout(kbT); document.body.classList.add('kb');
  });
  // z opóźnieniem: pasek wracający od razu przesunąłby TAKE spod palca między wciśnięciem a puszczeniem i dotknięcie by przepadło
  addEventListener('focusout', () => { kbT = setTimeout(() => document.body.classList.remove('kb'), 300); });
  addEventListener('beforeinstallprompt', (e) => { e.preventDefault(); installEv = e; });
  addEventListener('appinstalled', () => { installEv = null; toast('Zainstalowano'); });
  if (installed() || isNative) $('#bInstall').remove();
  else $('#bInstall').onclick = () => {
    if (installEv) { installEv.prompt(); installEv = null; return; }
    toast(/iPhone|iPad/.test(navigator.userAgent) ? 'Safari → Udostępnij → Do ekranu początkowego'
      : isSecureContext ? 'Menu przeglądarki → Zainstaluj aplikację' : 'Instalacja wymaga HTTPS, teraz możesz dodać tylko skrót na ekranie');
  };
  $('#bReload').onclick = () => loadShow().then(loadState).then(() => toast('Dane odświeżone')).catch((e) => toast(e.message, true));
  $('#bOutput').onclick = () => open('/output?checker=1&status=1' + antQuery(), '_blank');
}
