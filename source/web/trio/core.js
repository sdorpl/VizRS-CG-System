// Trio – wspólne: skróty DOM, antena, API, komunikaty, show i stan anteny, blokady edycji.

import { FONTS } from '/renderer.js';
import { html, esc, setHtml as writeHtml } from '/dom.js';
import { renderPages, renderTemplates, selectPage } from './pages.js';
import { renderStatus } from './fields.js';
import { applyPgmBg, renderLayers } from './onair.js';
import { plId, renderMacros, renderPlaylist, renderRundown, setPlId } from './rundown.js';
import { loadShows } from './system.js';
import { renderBar, renderTop, syncPgm } from './mobile.js';

// deklaracje funkcji (nie stałe): moduły z cyklu importów mogą ich użyć, zanim core.js się wykona
export function $(s, r = document) { return r.querySelector(s); }
export function $$(s, r = document) { return [...r.querySelectorAll(s)]; }
export { html, esc };
export const isNative = document.documentElement.classList.contains('native');
/** Układ telefonu (jak w VizRS Live: wąski ekran ≤ 760 px albo dotykowy ekran położony poziomo). Ten sam warunek co w CSS. */
export const PHONE = matchMedia('(max-width: 760px), (pointer: coarse) and (max-height: 560px)');
export const phone = () => PHONE.matches;
export const buzz = (p) => phone() && navigator.vibrate?.(p);
// Odświeżenie listy w chwili dotknięcia podmieniłoby element pod palcem i tap by przepadł, więc listy zmieniamy tylko przy zmianie treści.
const shown = new WeakMap();
export function setHtml(el, h) { const s = String(h); if (shown.get(el) !== s) { shown.set(el, s); writeHtml(el, h); } }
// ------------------------------------------------------------ antena: główna ('') albo kanał playoutu
export let ant = new URLSearchParams(location.search).get('channel');
/** Adres komendy na wybranej antenie. */
export const onAnt = (url) => ant ? `${url}${url.includes('?') ? '&' : '?'}channel=${encodeURIComponent(ant)}` : url;
export const antQuery = () => ant ? '&bus=ch-' + encodeURIComponent(ant) : '';
const antName = (n) => state.channelNames?.[n] ? `${n} ${state.channelNames[n]}` : 'Kanał ' + n;
export function renderAnt() {
  const names = state.channels || [];
  const opts = html`<option value="">Antena główna</option>${names.map((n) => html`<option value="${n}">${antName(n)}</option>`)}<option value="+">+ nowy kanał…</option>${ant ? html`<option value="-">− usuń kanał ${ant}</option>` : ''}`;
  if ($('#antSel').dataset.opts !== String(opts)) { writeHtml($('#antSel'), opts); $('#antSel').dataset.opts = opts; }
  $('#antSel').value = ant;
  $('#pgmTag').textContent = ant ? antName(ant).toUpperCase() : 'PGM';
}
export function setAnt(name) {
  ant = name;
  try { localStorage.setItem('vizrs.ant', ant); } catch { /* ignoruj */ }
  syncPgm(true);
  renderAnt(); applyPgmBg(); loadState();
}
export async function api(method, url, body) {
  // Wi-Fi na hali potrafi zawiesić żądanie: operator musi szybko wiedzieć, że komenda nie doszła
  const signal = AbortSignal.timeout(method === 'GET' ? 20000 : 6000);
  let r;
  try { r = await fetch(url, { method, signal, headers: body !== undefined ? { 'Content-Type': 'application/json' } : {}, body: body !== undefined ? JSON.stringify(body) : undefined }); }
  catch (e) { throw new Error(e.name === 'TimeoutError' ? 'brak odpowiedzi serwera – sprawdź antenę' : 'brak połączenia'); }
  const j = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(j.error || r.statusText);
  return j;
}
let toastT;
export function toast(msg, err) {
  let el = $('.toast'); if (!el) { el = document.createElement('div'); el.className = 'toast'; document.body.appendChild(el); }
  el.textContent = msg; el.classList.toggle('err', !!err); el.style.opacity = 1;
  clearTimeout(toastT); toastT = setTimeout(() => (el.style.opacity = 0), 2400);
}
export let show = { scenes: [], pages: [], playlists: [] }, state = { layers: [] }, assets = [];
// przypisanie z innych modułów (import ES jest tylko do odczytu)
export function setAssets(v) { assets = v; }
export let selId = null, draft = null, dirtyPage = false, pv = null;
// przypisanie z innych modułów (import ES jest tylko do odczytu)
export function setSelId(v) { selId = v; }
export function setDraft(v) { draft = v; }
export function setDirtyFlag(v) { dirtyPage = v; }
export function setPv(v) { pv = v; }
export const sceneOf = (p) => show.scenes.find((s) => s.id === p?.scene);
export const pageById = (id) => show.pages.find((p) => p.id === id);
export const sortedPages = () => [...show.pages].sort((a, b) => a.number - b.number);
export const onAirLayer = (pid) => state.layers.find((l) => l.page?.id === pid);
export async function loadShow() {
  show = await api('GET', '/api/show');
  assets = await api('GET', '/api/assets').catch(() => []);
  loadShows();
  if (!plId || !show.playlists.some((p) => p.id === plId)) setPlId(show.playlists[0]?.id || null);
  if (selId && !pageById(selId)) { selId = null; draft = null; }
  if (!selId && show.pages.length) selectPage(sortedPages()[0].id, true);
  else if (selId && !dirtyPage) selectPage(selId, true);
  renderPages(); renderTemplates(); renderPlaylist(); renderRundown(); renderMacros(); renderStatus(); renderTop(); renderBar();
}
let stTimer;
export function loadState() {
  clearTimeout(stTimer);
  stTimer = setTimeout(async () => {
    state = await api('GET', onAnt('/api/state')).catch(() => state);
    // TAKE na usuniętym kanale założyłby go od nowa i grafika poszłaby w próżnię
    if (ant && state.channels && !state.channels.includes(ant)) { toast(`Kanał ${ant} nie istnieje – antena główna`, true); setAnt(''); return; }
    renderAnt(); renderLayers(); renderPages(); renderStatus(); renderPlaylist(); renderRundown(); renderTop(); renderBar();
  }, 40);
}
// ------------------------------------------------------------ blokady edycji (wielu operatorów)
export let who = '';
// przypisanie z innych modułów (import ES jest tylko do odczytu)
export function setWho(v) { who = v; }
export let lockedBy = null, lockedId = null;
export const otherLocks = () => (state.locks || []).filter((l) => l.kind === 'pages' && l.who !== who);
export async function lockPage(id) {
  if (lockedId && lockedId !== id) fetch(`/api/unlock/pages/${encodeURIComponent(lockedId)}?who=${encodeURIComponent(who)}`, { method: 'POST' }).catch(() => {});
  lockedId = id; lockedBy = null;
  try { await api('POST', `/api/lock/pages/${encodeURIComponent(id)}?who=${encodeURIComponent(who)}`); }
  catch (e) { if (/^konflikt/.test(e.message)) { lockedBy = e.message.replace(/^konflikt: edytuje /, '').replace(/ \(.*$/, ''); toast(`Strona jest edytowana przez ${lockedBy}`, true); } }
  renderStatus();
}

/** Zdarzenia i stan początkowy; woła main.js, moduły w kolejności dawnego skryptu. */
export function init() {
  // token z adresu trafił już do ciasteczka: nie zostawiamy go w historii, skrótach ani na zrzutach ekranu
  if (new URLSearchParams(location.search).has('token')) {
    const q = new URLSearchParams(location.search); q.delete('token');
    history.replaceState(null, '', '/trio' + (q.size ? '?' + q : ''));
  }
  // aplikacja z ikony telefonu startuje bez sieci; bez HTTPS przeglądarka nie daje service workera i Trio działa tak samo, tylko nie startuje offline
  if (!isNative) navigator.serviceWorker?.register('/trio-sw.js', { scope: '/trio' }).catch(() => {});
  try { ant ??= localStorage.getItem('vizrs.ant') || ''; } catch { ant ??= ''; }
  writeHtml($('#fontlist'), FONTS.map((f) => html`<option>${f}</option>`));
  try { who = localStorage.getItem('vizrs.who') || ''; } catch { /* brak localStorage */ }
  if (!who) { who = 'Operator ' + Math.floor(10 + Math.random() * 90); try { localStorage.setItem('vizrs.who', who); } catch { /* ignoruj */ } }
  setInterval(() => { if (lockedId && !lockedBy) api('POST', `/api/lock/pages/${encodeURIComponent(lockedId)}?who=${encodeURIComponent(who)}`).catch(() => {}); }, 15000);
  addEventListener('pagehide', () => { if (lockedId && !lockedBy) navigator.sendBeacon(`/api/unlock/pages/${encodeURIComponent(lockedId)}?who=${encodeURIComponent(who)}`); });
}
