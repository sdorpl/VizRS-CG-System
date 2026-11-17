// Trio – playlisty, zakładki, rundown, makra.

import { DEFAULT_TZ } from '/renderer.js';
import { ask, askText } from '/dialogs.js';
import { $, $$, api, draft, esc, loadShow, onAirLayer, onAnt, pageById, phone, setHtml, show, state, toast } from './core.js';
import { trySelect } from './pages.js';
import { isConflict } from './fields.js';
import { act } from './onair.js';

// ------------------------------------------------------------ playlisty
export let plId = null;
// przypisanie z innych modułów (import ES jest tylko do odczytu)
export function setPlId(v) { plId = v; }
const curPl = () => show.playlists.find((p) => p.id === plId);
export function renderPlaylist() {
  $('#plSel').innerHTML = show.playlists.map((p) => `<option value="${p.id}" ${p.id === plId ? 'selected' : ''}>${esc(p.name)}</option>`).join('') || '<option value="">(brak)</option>';
  const pl = curPl();
  const running = state.carousel && state.carousel.playlist === plId;
  $('#plLoop').checked = pl ? pl.loop !== false : false;
  $('#plStart').disabled = !pl || !pl.items?.length;
  $('#plStart').textContent = running ? '▶ gra…' : '▶ Start';
  if (!pl) { setHtml($('#pl'), '<div class="empty">Utwórz playlistę ＋</div>'); return; }
  setHtml($('#pl'), (pl.items || []).map((it, i) => {
    const p = pageById(it.page);
    return `<div class="item ${running && state.carousel.index === i ? 'cur' : ''}" data-i="${i}">
      <span class="dim">${i + 1}</span>
      <span><span style="font-family:var(--mono);color:var(--acc)">${p?.number ?? '?'}</span> ${esc(p?.name ?? '(usunięta)')}</span>
      <input type="number" min="0.5" step="0.5" value="${it.duration ?? 5}" data-dur="${i}" title="czas trwania (s)">
      <span class="row" style="gap:2px"><button class="icon" data-mv="${i}" title="w górę">↑</button><button class="icon danger" data-rm="${i}" title="usuń">×</button></span></div>`;
  }).join('') || '<div class="empty">Pusta. Zaznacz stronę i kliknij „＋ strona”.</div>');
}
async function savePl(pl) {
  try { const r = await api('PUT', '/api/playlists/' + pl.id, pl); Object.assign(pl, r); renderPlaylist(); }
  catch (e) {
    toast(e.message, true);
    if (isConflict(e)) loadShow(); // playlista zmieniona gdzie indziej – wczytaj aktualną
  }
}
// ------------------------------------------------------------ zakładki (playlista / rundown / makra)
let tab = 'pl';
function setTab(t) {
  tab = t;
  try { localStorage.setItem('vizrs.tab', t); } catch { /* ignoruj */ }
  $$('[data-tab]').forEach((b) => b.classList.toggle('on', b.dataset.tab === t));
  $$('[data-tabbody]').forEach((d) => (d.style.display = d.dataset.tabbody === t ? 'flex' : 'none'));
  $$('[data-tabtools]').forEach((d) => (d.style.display = d.dataset.tabtools === t ? '' : 'none'));
}
// ------------------------------------------------------------ rundown
let rdId = null;
const curRd = () => show.rundowns?.find((r) => r.id === rdId);
const rdRun = () => (state.rundowns || []).find((r) => r.id === rdId) || {};
export function renderRundown() {
  show.rundowns ||= [];
  if (!rdId || !curRd()) rdId = show.rundowns[0]?.id || null;
  $('#rdSel').innerHTML = show.rundowns.map((r) => `<option value="${r.id}" ${r.id === rdId ? 'selected' : ''}>${esc(r.name)}</option>`).join('') || '<option value="">(brak)</option>';
  const rd = curRd(), run = rdRun();
  $('#rdNext').disabled = !rd || !run.cursor;
  if (!rd) { setHtml($('#rd'), '<div class="empty">Utwórz rundown ＋ i dodaj strony.</div>'); return; }
  const done = new Set(run.done || []);
  setHtml($('#rd'), (rd.items || []).map((it, i) => {
    const p = pageById(it.page), air = !!onAirLayer(it.page);
    const cls = air ? 'onair' : it.id === run.cursor ? 'next' : done.has(it.id) ? 'done' : '';
    const mark = air ? '●' : it.id === run.cursor ? '▶' : done.has(it.id) ? '✓' : '';
    return `<div class="item ${cls}" data-iid="${esc(it.id)}" draggable="true">
      <span class="grip" title="Przeciągnij, by zmienić kolejność">☰</span><span class="st" style="color:${air ? 'var(--red)' : 'var(--green)'}">${mark}</span>
      <span style="font-family:var(--mono);color:var(--acc)">${p?.number ?? '?'}</span>
      <span>${esc(p?.name ?? '(usunięta)')}</span>
      <input class="note" data-note="${esc(it.id)}" value="${esc(it.note || '')}" placeholder="notatka">
      <span class="row" style="gap:2px"><button class="icon" data-rdcur="${esc(it.id)}" title="Ustaw jako następną">⏵</button><button class="icon danger" data-rdrm="${esc(it.id)}" title="Usuń z rundownu">×</button></span></div>`;
  }).join('') || '<div class="empty">Pusty. Zaznacz stronę i kliknij „＋ strona”.</div>');
}
async function saveRd(rd) {
  try { const r = await api('PUT', '/api/rundowns/' + rd.id, rd); Object.assign(rd, r); renderRundown(); }
  catch (e) { toast(e.message, true); if (isConflict(e)) loadShow(); }
}
const newItemId = () => 'i' + Date.now().toString(36) + Math.random().toString(36).slice(2, 5);
export function rdNext() { const rd = curRd(); if (rd) act(() => api('POST', onAnt(`/api/rundown/${rd.id}/next`)), 'NEXT'); }
// przeciąganie pozycji (zmiana kolejności)
let dragId = null;
// CSV (numer;notatka) – separator ; lub , wykrywany z nagłówka; cudzysłowy jak w RFC 4180
function parseCsv(text) {
  const first = text.split(/\r?\n/)[0] || ''; const sep = first.includes(';') || !first.includes(',') ? ';' : ',';
  const rows = []; let row = [], cur = '', q = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (q) { if (c === '"') { if (text[i + 1] === '"') { cur += '"'; i++; } else q = false; } else cur += c; }
    else if (c === '"') q = true;
    else if (c === sep) { row.push(cur); cur = ''; }
    else if (c === '\n') { row.push(cur.replace(/\r$/, '')); rows.push(row); row = []; cur = ''; }
    else cur += c;
  }
  if (cur || row.length) { row.push(cur.replace(/\r$/, '')); rows.push(row); }
  return rows.filter((r) => r.some((x) => x.trim() !== ''));
}
const csvCell = (v) => (/[;"\n]/.test(v) ? '"' + v.replace(/"/g, '""') + '"' : v);
// ------------------------------------------------------------ makra / presety segmentów
let mcId = null;
const curMc = () => show.macros?.find((m) => m.id === mcId);
const stepsToText = (m) => (m.steps || []).map((s) => (s.delay > 0 ? `+${s.delay} ` : '') + s.cmd).join('\n');
const textToSteps = (t) => t.split('\n').map((l) => l.trim()).filter(Boolean).map((l) => {
  const m = l.match(/^\+(\d+(?:[.,]\d+)?)\s+(.*)$/);
  return m ? { delay: parseFloat(m[1].replace(',', '.')), cmd: m[2] } : { delay: 0, cmd: l };
});
export function renderMacros() {
  show.macros ||= [];
  if (!mcId || !curMc()) mcId = show.macros[0]?.id || null;
  $('#mcSel').innerHTML = show.macros.map((m) => `<option value="${m.id}" ${m.id === mcId ? 'selected' : ''}>${esc(m.name)}</option>`).join('') || '<option value="">(brak)</option>';
  const m = curMc();
  if (document.activeElement !== $('#mcText')) $('#mcText').value = m ? stepsToText(m) : '';
  $('#mcText').disabled = !m; $('#mcRun').disabled = !m; $('#mcSave').disabled = !m;
}
async function newMacro(name, steps) {
  try { const m = await api('POST', '/api/macros', { name, steps }); (show.macros ||= []).push(m); mcId = m.id; renderMacros(); setTab('mc'); return m; }
  catch (e) { toast(e.message, true); }
}

/** Zdarzenia i stan początkowy; woła main.js, moduły w kolejności dawnego skryptu. */
export function init() {
  $('#plSel').addEventListener('change', (e) => { plId = e.target.value; renderPlaylist(); });
  $('#plNew').onclick = async () => {
    const n = await askText('Nazwa playlisty', { title: 'Nowa playlista', value: 'Playlista ' + (show.playlists.length + 1), ok: 'Utwórz' }); if (!n) return;
    const r = await api('POST', '/api/playlists', { name: n, loop: true, items: [] });
    show.playlists.push(r); plId = r.id; renderPlaylist();
  };
  $('#plDel').onclick = async () => {
    const pl = curPl(); if (!pl || !(await ask(`Usunąć playlistę „${pl.name}”?`, { ok: 'Usuń', danger: true }))) return;
    await api('DELETE', '/api/playlists/' + pl.id);
    show.playlists = show.playlists.filter((p) => p.id !== pl.id); plId = show.playlists[0]?.id || null; renderPlaylist();
  };
  $('#plAdd').onclick = () => { const pl = curPl(); if (!pl || !draft) return; (pl.items ||= []).push({ page: draft.id, duration: 6 }); savePl(pl); };
  $('#plLoop').onchange = (e) => { const pl = curPl(); if (pl) { pl.loop = e.target.checked; savePl(pl); } };
  $('#pl').addEventListener('change', (e) => { const i = e.target.dataset.dur; if (i === undefined) return; const pl = curPl(); pl.items[+i].duration = parseFloat(e.target.value) || 5; savePl(pl); });
  $('#pl').addEventListener('click', (e) => {
    const pl = curPl(); if (!pl) return;
    const rm = e.target.closest('[data-rm]'), mv = e.target.closest('[data-mv]');
    if (rm) { pl.items.splice(+rm.dataset.rm, 1); savePl(pl); return; }
    if (mv) { const i = +mv.dataset.mv; if (i > 0) { [pl.items[i - 1], pl.items[i]] = [pl.items[i], pl.items[i - 1]]; savePl(pl); } return; }
    const it = e.target.closest('.item[data-i]'); if (it && e.target.tagName !== 'INPUT') { const p = pl.items[+it.dataset.i]; if (p) trySelect(p.page); }
  });
  $('#plStart').onclick = () => { const pl = curPl(); if (pl) act(() => api('POST', onAnt('/api/carousel/start/' + pl.id)), 'Karuzela'); };
  $('#plStop').onclick = () => act(() => api('POST', onAnt('/api/carousel/stop')), 'Karuzela');
  try { tab = localStorage.getItem('vizrs.tab') || 'pl'; } catch { /* brak localStorage */ }
  $$('[data-tab]').forEach((b) => (b.onclick = () => setTab(b.dataset.tab)));
  $('#rdSel').onchange = (e) => { rdId = e.target.value; renderRundown(); };
  $('#rdNew').onclick = async () => {
    const n = await askText('Nazwa rundownu', { title: 'Nowy rundown', value: 'Rundown ' + ((show.rundowns?.length || 0) + 1), ok: 'Utwórz' }); if (!n) return;
    try { const r = await api('POST', '/api/rundowns', { name: n, items: [] }); (show.rundowns ||= []).push(r); rdId = r.id; renderRundown(); } catch (e) { toast(e.message, true); }
  };
  $('#rdDel').onclick = async () => {
    const rd = curRd(); if (!rd || !(await ask(`Usunąć rundown „${rd.name}”?`, { ok: 'Usuń', danger: true }))) return;
    await api('DELETE', '/api/rundowns/' + rd.id); show.rundowns = show.rundowns.filter((r) => r.id !== rd.id); rdId = null; renderRundown();
  };
  $('#rdAdd').onclick = () => { const rd = curRd(); if (!rd || !draft) return; (rd.items ||= []).push({ id: newItemId(), page: draft.id, note: '' }); saveRd(rd); };
  $('#rdReset').onclick = () => { const rd = curRd(); if (rd) act(() => api('POST', onAnt(`/api/rundown/${rd.id}/reset`)), 'Rundown'); };
  $('#rdNext').onclick = rdNext;
  $('#rd').addEventListener('click', (e) => {
    const rd = curRd(); if (!rd) return;
    const rm = e.target.closest('[data-rdrm]'), cur = e.target.closest('[data-rdcur]');
    if (rm) { rd.items = rd.items.filter((i) => i.id !== rm.dataset.rdrm); saveRd(rd); return; }
    if (cur) { act(() => api('POST', onAnt(`/api/rundown/${rd.id}/cursor/${encodeURIComponent(cur.dataset.rdcur)}`)), 'Rundown'); return; }
    const it = e.target.closest('.item[data-iid]');
    if (it && e.target.tagName !== 'INPUT') { const x = rd.items.find((i) => i.id === it.dataset.iid); if (x) trySelect(x.page); }
  });
  $('#rd').addEventListener('dblclick', (e) => {
    const rd = curRd(), it = e.target.closest('.item[data-iid]');
    if (rd && it && e.target.tagName !== 'INPUT' && !phone()) act(() => api('POST', onAnt(`/api/rundown/${rd.id}/take/${encodeURIComponent(it.dataset.iid)}`)), 'Rundown');
  });
  $('#rd').addEventListener('change', (e) => {
    const id = e.target.dataset.note; if (!id) return;
    const rd = curRd(); const it = rd?.items.find((i) => i.id === id); if (it) { it.note = e.target.value; saveRd(rd); }
  });
  $('#rd').addEventListener('dragstart', (e) => { const it = e.target.closest('.item[data-iid]'); if (!it) return; dragId = it.dataset.iid; e.dataTransfer.effectAllowed = 'move'; try { e.dataTransfer.setData('text/plain', dragId); } catch { /* ignoruj */ } });
  $('#rd').addEventListener('dragover', (e) => { if (dragId && e.target.closest('.item[data-iid]')) { e.preventDefault(); $$('#rd .dragover').forEach((x) => x.classList.remove('dragover')); e.target.closest('.item').classList.add('dragover'); } });
  $('#rd').addEventListener('dragend', () => { dragId = null; $$('#rd .dragover').forEach((x) => x.classList.remove('dragover')); });
  $('#rd').addEventListener('drop', (e) => {
    const over = e.target.closest('.item[data-iid]'), rd = curRd();
    if (!over || !rd || !dragId || over.dataset.iid === dragId) return;
    e.preventDefault();
    const from = rd.items.findIndex((i) => i.id === dragId), to = rd.items.findIndex((i) => i.id === over.dataset.iid);
    const [m] = rd.items.splice(from, 1); rd.items.splice(to, 0, m);
    dragId = null; saveRd(rd);
  });
  $('#rdExp').onclick = () => {
    const rd = curRd(); if (!rd) return;
    const lines = ['numer;notatka', ...rd.items.map((it) => `${pageById(it.page)?.number ?? ''};${csvCell(it.note || '')}`)];
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob(['\ufeff' + lines.join('\r\n')], { type: 'text/csv' }));
    a.download = (rd.name || 'rundown').replace(/[^\w.\-]+/g, '_') + '.csv'; a.click();
  };
  $('#rdImp').onclick = () => {
    const rd = curRd(); if (!rd) return toast('Najpierw utwórz rundown', true);
    const fi = $('#fileIn'); fi.value = ''; fi.accept = '.csv,text/csv,text/plain';
    fi.onchange = async () => {
      const f = fi.files[0]; if (!f) return;
      const rows = parseCsv((await f.text()).replace(/^\ufeff/, ''));
      const start = rows[0] && isNaN(parseInt(rows[0][0])) ? 1 : 0; // pierwszy wiersz może być nagłówkiem
      const items = [], missing = [];
      for (const r of rows.slice(start)) {
        const n = parseInt(r[0]); const p = show.pages.find((x) => x.number === n);
        if (p) items.push({ id: newItemId(), page: p.id, note: (r[1] || '').trim() }); else missing.push(r[0]);
      }
      if (!items.length) return toast('Brak pasujących numerów stron w pliku', true);
      if (rd.items?.length && !(await ask(`Zastąpić ${rd.items.length} pozycji rundownu ${items.length} pozycjami z pliku?`, { ok: 'Zastąp' }))) return;
      rd.items = items; await saveRd(rd);
      toast(missing.length ? `Wczytano ${items.length}, pominięto nieznane numery: ${missing.join(', ')}` : `Wczytano ${items.length} pozycji`, !!missing.length);
    };
    fi.click();
  };
  $('#mcSel').onchange = (e) => { mcId = e.target.value; renderMacros(); };
  $('#mcNew').onclick = async () => { const n = await askText('Nazwa makra', { title: 'Nowe makro', value: 'Makro ' + ((show.macros?.length || 0) + 1), ok: 'Utwórz' }); if (n) newMacro(n, []); };
  $('#mcDel').onclick = async () => {
    const m = curMc(); if (!m || !(await ask(`Usunąć makro „${m.name}”?`, { ok: 'Usuń', danger: true }))) return;
    await api('DELETE', '/api/macros/' + m.id); show.macros = show.macros.filter((x) => x.id !== m.id); mcId = null; renderMacros();
  };
  $('#mcSave').onclick = async () => {
    const m = curMc(); if (!m) return;
    try { const r = await api('PUT', '/api/macros/' + m.id, { ...m, steps: textToSteps($('#mcText').value) }); Object.assign(m, r); toast('Zapisano makro'); renderMacros(); }
    catch (e) { toast(e.message, true); if (isConflict(e)) loadShow(); }
  };
  $('#mcRun').onclick = async () => { await $('#mcSave').onclick(); const m = curMc(); if (m) act(() => api('POST', onAnt('/api/macro/' + encodeURIComponent(m.id))), 'Makro'); };
  $('#mcStop').onclick = () => act(() => api('POST', onAnt('/api/macro/stop')), 'Makro');
  $('#mcFromAir').onclick = async () => {
    if (!state.layers.length) return toast('Antena jest czysta – nie ma czego zapisać', true);
    const n = await askText('Nazwa presetu (makra) z bieżącego stanu anteny', { title: 'Preset z anteny', value: 'Preset ' + new Date().toLocaleTimeString('pl-PL', { timeZone: DEFAULT_TZ }), ok: 'Zapisz' }); if (!n) return;
    newMacro(n, [{ delay: 0, cmd: 'CLEAR' }, ...state.layers.map((l) => ({ delay: 0, cmd: 'TAKE ' + l.page.number }))]);
  };
  setTab(tab);
}
