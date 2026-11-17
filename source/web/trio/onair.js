// Trio – podgląd lokalny, monitor PGM, sterowanie anteną, warstwy na antenie.

import { ask, askText } from '/dialogs.js';
import { $, ant, antQuery, api, buzz, draft, esc, loadState, onAirLayer, onAnt, phone, pv, renderAnt, setAnt, setHtml, state, toast } from './core.js';
import { applyFinalRules } from './fields.js';
import { closeDrawer, renderBar } from './mobile.js';

// ------------------------------------------------------------ tło monitora PGM (tylko podgląd w Trio – wyjście OBS zostaje przezroczyste)
let pgmBg = 'checker', pgmBgCol = '#808080';
export function applyPgmBg() {
  const custom = pgmBg === 'custom', bg = custom ? pgmBgCol : pgmBg;
  $('#bgSel').value = pgmBg; $('#bgCol').value = pgmBgCol; $('#bgCol').style.display = custom ? '' : 'none';
  const mon = $('#pgm').parentElement; mon.classList.toggle('checker', bg === 'checker'); mon.style.background = bg === 'checker' ? '' : bg;
  const b = $('#pgm').contentDocument?.body; // ta sama domena – ustawiamy tło bezpośrednio
  if (b) { b.classList.toggle('checker', bg === 'checker'); b.style.background = bg === 'checker' ? '' : bg; }
  $('#pgmOpen').href = '/output?status=1&' + (bg === 'checker' ? 'checker=1' : 'bg=' + encodeURIComponent(bg)) + antQuery();
}
function setPgmBg(v, col) {
  pgmBg = v; if (col) pgmBgCol = col;
  try { localStorage.setItem('vizrs.pgmBg', pgmBg); localStorage.setItem('vizrs.pgmBgCol', pgmBgCol); } catch { /* ignoruj */ }
  applyPgmBg();
}
// adres wyjścia dla OBS: w oknie natywnym nie ma paska adresu, a okno samo wybiera wolny port (8088 albo kolejny)
async function copyText(t) {
  try { await navigator.clipboard.writeText(t); return true; } catch { /* np. strona spoza bezpiecznego kontekstu */ }
  const ta = Object.assign(document.createElement('textarea'), { value: t });
  ta.style.cssText = 'position:fixed;left:-9999px'; document.body.appendChild(ta); ta.select();
  const ok = document.execCommand('copy'); ta.remove(); return ok;
}
export async function showObsUrl() {
  const h = await api('GET', '/health').catch(() => ({}));
  const url = location.origin + '/output' + (ant ? '?bus=ch-' + encodeURIComponent(ant) : '') + (h.tokenRequired ? (ant ? '&' : '?') + 'token=' + (h.outputToken ? 'TOKEN_WYJŚCIA' : 'TOKEN') : '');
  const text = 'OBS Studio → Źródła → ＋ → Przeglądarka: wklej adres, szerokość 1920, wysokość 1080.' +
    (ant ? `\nAdres dotyczy kanału ${ant}.` : '') +
    (h.tokenRequired ? `\nSerwer wymaga tokenu: w miejsce ${h.outputToken ? 'TOKEN_WYJŚCIA wpisz token wyjścia (--output-token, tylko odczyt)' : 'TOKEN wpisz token serwera'}.` : '') +
    '\nOBS na innym komputerze: zamiast 127.0.0.1 / localhost adres tego komputera w sieci (serwer uruchomiony z --bind 0.0.0.0 i tokenem).';
  const v = await askText(text, { title: 'Adres wyjścia dla OBS', value: url, ok: 'Kopiuj', cancel: 'Zamknij' });
  if (v) toast(await copyText(v) ? 'Skopiowano adres dla OBS' : 'Nie udało się skopiować – zaznacz adres i Ctrl+C', false);
}
// ------------------------------------------------------------ playout
// na telefonie jedna komenda naraz i chwila przerwy po niej: podwójne dotknięcie nie może zagrać TAKE dwa razy
export let busy = false;
export async function act(fn, label) {
  if (!phone()) { try { await fn(); loadState(); } catch (e) { toast(`${label}: ${e.message}`, true); } return; }
  if (busy) return;
  busy = true; renderBar();
  try { await fn(); buzz(25); } catch (e) { buzz([60, 40, 60]); toast(`${label}: ${e.message}`, true); }
  loadState();
  setTimeout(() => { busy = false; renderBar(); }, 350);
}
export const take = () => draft && (applyFinalRules(), true) && act(() => api('POST', onAnt(`/api/take/${draft.number}`), { values: draft.values }), 'TAKE');
// strona na antenie: jej rzeczywista warstwa (szablon mógł w tym czasie dostać inną w Artist)
const airKey = () => encodeURIComponent(onAirLayer(draft.id)?.layer ?? draft.number);
export const cont = () => draft && act(() => api('POST', onAnt(`/api/continue/${airKey()}`)), 'CONTINUE');
export const out = () => draft && act(() => api('POST', onAnt(`/api/out/${airKey()}`)), 'OUT');
export const upd = () => draft && (applyFinalRules(), true) && act(async () => {
  const r = await api('POST', `/api/update/${draft.number}?save=0`, { values: draft.values });
  if (!r.onAir) toast('Strona nie jest na antenie – nic do aktualizacji');
}, 'UPDATE');
export const arm = () => draft && (applyFinalRules(), true) && act(() => api('POST', `/api/arm/${draft.number}`, { values: draft.values }), 'ARM');
export const clearAll = () => act(() => api('POST', onAnt('/api/clear')), 'CLEAR');
export function renderLayers() {
  setHtml($('#layers'), state.layers.map((l) => `<div class="item">
    <span class="lname">${esc(l.layer)}</span>
    <span><span class="num" style="font-family:var(--mono);color:var(--acc)">${l.page?.number ?? ''}</span> ${esc(l.page?.name || l.scene)} <span class="dim" style="font-size:11px">stop ${l.stop + 1}/${Math.max(1, l.stops)}</span>${l.pending ? ` <span class="pend" title="Szablon został zmieniony w Artist – wizja gra starą wersję">⏳ nowa wersja szablonu <a href="#" data-ldeploy="${esc(l.layer)}">Wdróż teraz</a></span>` : ''}</span>
    ${l.hasDock ? `<button class="icon ${l.dockOff ? '' : 'acc'}" data-ldock="${esc(l.layer)}" data-off="${l.dockOff ? 0 : 1}" title="Powiązanie (dock) tej sceny: ${l.dockOff ? 'wyłączone – kliknij, by włączyć' : 'aktywne – kliknij, by wyłączyć (nadpisanie)'}">⇅</button>` : ''}
    <button class="icon" data-lcont="${esc(l.layer)}" title="Continue">⏭</button>
    <button class="icon danger" data-lout="${esc(l.layer)}" title="Out">⏏</button></div>`).join('')
    || '<div class="empty">Antena czysta</div>');
  $('#conn').title = `połączone wyjścia/odbiorniki: ${state.outputs ?? '?'}`;
  const se = $('#saveErr');
  se.style.display = state.saveError ? '' : 'none';
  se.textContent = '⚠ zapis show nieudany';
  se.title = state.saveError || '';
  const ep = $('#errPill');
  ep.style.display = state.errors ? '' : 'none';
  ep.textContent = `⚠ błędy scen: ${state.errors || 0}`;
}

/** Zdarzenia i stan początkowy; woła main.js, moduły w kolejności dawnego skryptu. */
export function init() {
  // ------------------------------------------------------------ podgląd lokalny
  $('#pvIn').onclick = () => pv?.take();
  $('#pvCont').onclick = () => pv?.cont();
  $('#pvOut').onclick = () => { if (!pv) return; const s = pv.stops; if (s.length && pv.duration > s[s.length - 1]) pv.playTo(pv.duration, Math.max(pv.t, s[s.length - 1])); };
  $('#pvHold').onclick = () => pv?.seekStop(0);
  try { pgmBg = localStorage.getItem('vizrs.pgmBg') || 'checker'; pgmBgCol = localStorage.getItem('vizrs.pgmBgCol') || pgmBgCol; } catch { /* brak localStorage */ }
  $('#bgSel').onchange = (e) => setPgmBg(e.target.value);
  $('#bgCol').oninput = (e) => setPgmBg('custom', e.target.value);
  $('#pgm').addEventListener('load', applyPgmBg);
  $('#pgmObs').onclick = showObsUrl;
  $('#antSel').onchange = async (e) => {
    const v = e.target.value;
    if (v === '-') {
      if (!(await ask(`Usunąć kanał ${ant}? Jego grafika zejdzie z anteny.`, { ok: 'Usuń kanał', danger: true }))) return renderAnt();
      return act(async () => { await api('DELETE', '/api/channels/' + encodeURIComponent(ant)); setAnt(''); }, 'Kanał');
    }
    if (v !== '+') return setAnt(v);
    const name = (await askText('Numer kanału (jak w ErsatzTV):', { title: 'Nowy kanał', ok: 'Dodaj' })) || '';
    if (!/^[\w.-]{1,32}$/.test(name)) return renderAnt();
    act(async () => { await api('POST', '/api/channels/' + encodeURIComponent(name)); setAnt(name); }, 'Kanał');
  };
  if (ant) setAnt(ant);
  applyPgmBg();
  $('#bArm').onclick = arm; $('#bTake').onclick = take; $('#bCont').onclick = cont; $('#bOut').onclick = out; $('#bUpd').onclick = upd;
  $('#bClear').onclick = $('#bClear2').onclick = async () => { closeDrawer(); if (await ask('Zdjąć wszystkie grafiki z anteny?', { ok: 'Zdejmij wszystko', danger: true })) clearAll(); };
  $('#layers').addEventListener('click', (e) => {
    const dk = e.target.closest('[data-ldock]');
    if (dk) { act(() => api('POST', onAnt(`/api/dock/${encodeURIComponent(dk.dataset.ldock)}/${dk.dataset.off === '1' ? 'off' : 'on'}`)), 'Dock'); return; }
    const d = e.target.closest('[data-ldeploy]');
    if (d) { e.preventDefault(); act(() => api('POST', onAnt('/api/deploy/' + encodeURIComponent(d.dataset.ldeploy))), 'Wdrożenie'); return; }
    const c = e.target.closest('[data-lcont]'), o = e.target.closest('[data-lout]');
    if (c) act(() => api('POST', onAnt('/api/continue/' + encodeURIComponent(c.dataset.lcont))), 'CONTINUE');
    if (o) act(() => api('POST', onAnt('/api/out/' + encodeURIComponent(o.dataset.lout))), 'OUT');
  });
}
