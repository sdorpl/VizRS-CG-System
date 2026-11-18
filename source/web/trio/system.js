// Trio – stan systemu, biblioteka show, import i eksport, klawiatura, okno natywne.

import { DEFAULT_TZ } from '/renderer.js';
import { nativeBridge, contextMenu } from '/native.js';
import { ask, askText, choose } from '/dialogs.js';
import * as PACK from '/cgpack.js';
import { planDialog, pickPages } from '/pack-ui.js';
import { orEmpty, setHtml as writeHtml } from '/dom.js';
import { $, api, assets, dirtyPage, draft, esc, html, loadShow, onAirLayer, onAnt, selId, setDirtyFlag, setDraft, setSelId, setWho, show, sortedPages, state, toast, who } from './core.js';
import { trySelect } from './pages.js';
import { pageHotkey, savePage, setWrList, wrList } from './fields.js';
import { act, arm, clearAll, cont, out, showObsUrl, take, upd } from './onair.js';
import { rdNext } from './rundown.js';

// ------------------------------------------------------------ stan systemu
const fmtT = (ms) => new Date(ms).toLocaleString('pl-PL', { timeZone: DEFAULT_TZ });
async function openSys() {
  const [h, errs, bks, aud, fonts] = await Promise.all([
    api('GET', '/health').catch(() => ({})), api('GET', '/api/errors').catch(() => []),
    api('GET', '/api/backups').catch(() => []), api('GET', '/api/audit?n=40').catch(() => []), api('GET', '/api/fonts').catch(() => []),
  ]);
  const aRows = aud.slice().reverse().map((l) => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean);
  writeHtml($('#sysBox'), html`
    <div class="row"><b style="font-size:15px">Stan systemu</b><span class="grow"></span><button id="sysClose">Zamknij (Esc)</button></div>
    <table><tr><th>Status</th><td>${esc(h.status)} · VizRS ${esc(h.version)} (${esc(h.build)}) · działa ${Math.round((h.uptimeS || 0) / 60)} min · rev ${h.rev}</td></tr>
    <tr><th>Wyjścia</th><td>${h.outputs} połączone · na antenie warstw: ${h.layersOnAir} · ostatnie zdarzenie: ${h.lastEventAgeS == null ? '—' : Math.round(h.lastEventAgeS) + ' s temu'}</td></tr>
    <tr><th>Zapis</th><td>${h.saveError ? html`<b style="color:var(--red-t)">${esc(h.saveError)}</b>` : 'OK'} · kopii: ${h.backups}</td></tr>
    <tr><th>Ty (blokady edycji)</th><td><input id="whoIn" value="${esc(who)}" style="width:200px"> <span class="dim">nazwa widoczna dla innych operatorów przy edycji strony</span></td></tr>
    <tr><th>Fonty offline</th><td>${esc(fonts.join(', '))}</td></tr></table>
    <h4>Błędy scen na wyjściu (${errs.length}) <button id="errClear" ${errs.length ? '' : 'disabled'}>Wyczyść</button></h4>
    ${errs.length ? html`<table>${errs.map((e) => html`<tr><td>${esc(e.scene)}</td><td class="mono">${esc(e.layer)}</td><td>${esc(e.message)}</td><td>×${e.count}</td><td class="mono">${fmtT(e.last)}</td></tr>`)}</table>` : html`<div class="dim">Brak błędów.</div>`}
    <h4>Word Replace – lista zamian słów stanowiska <span class="dim" style="text-transform:none;letter-spacing:0">(w polu tekstowym: skrót + Ctrl+Spacja)</span></h4>
    <textarea id="wrText" rows="5" style="width:100%;font:12px var(--mono)" placeholder="@p: Prezydent | Premier&#10;@m: Minister&#10;# wiersz = skrót: słowo1 | słowo2 (skrót 1–8 znaków, bez spacji)">${esc(window.CGText ? CGText.wordReplaceFormat(wrList) : '')}</textarea>
    <div class="row" style="margin-top:4px"><button id="wrSave">Zapisz listę zamian</button></div>
    <h4>Kopie zapasowe show</h4>
    <div class="row"><select id="bkSel" style="flex:1">${bks.map((b) => html`<option>${b}</option>`)}</select><button id="bkRestore" class="danger">Przywróć wybraną</button></div>
    <h4>Ostatnie komendy (log audytowy)</h4>
    <table>${orEmpty(aRows.map((r) => html`<tr><td class="mono">${fmtT(r.t)}</td><td class="mono">${esc(r.src)}</td><td class="mono">${esc(r.cmd)}</td><td>${r.status ?? ''}</td></tr>`), html`<tr><td class="dim">Pusty.</td></tr>`)}</table>`);
  $('#sysModal').classList.add('open');
  $('#whoIn').onchange = (e) => { setWho(e.target.value.trim() || who); try { localStorage.setItem('vizrs.who', who); } catch { /* ignoruj */ } toast('Zapisano nazwę operatora'); };
  $('#sysClose').onclick = () => $('#sysModal').classList.remove('open');
  $('#wrSave').onclick = async () => {
    try { setWrList(await api('PUT', '/api/wordreplace', CGText.wordReplaceParse($('#wrText').value))); toast('Zapisano listę zamian'); $('#wrText').value = CGText.wordReplaceFormat(wrList); }
    catch (e) { toast(e.message, true); }
  };
  $('#errClear').onclick = async () => { await api('DELETE', '/api/errors'); openSys(); };
  $('#bkRestore').onclick = async () => {
    const n = $('#bkSel').value;
    if (!n || !(await ask(`Przywrócić show z kopii ${n}?\nBieżący stan zostanie zachowany jako najnowsza kopia.`, { ok: 'Przywróć' }))) return;
    try { await api('POST', '/api/backups/restore/' + encodeURIComponent(n)); setSelId(null); setDirtyFlag(false); await loadShow(); toast('Przywrócono show'); $('#sysModal').classList.remove('open'); }
    catch (e) { toast(e.message, true); }
  };
}
// ------------------------------------------------------------ biblioteka show (paczka grafiki na audycję)
let shows = { active: '', shows: [] };
export async function loadShows() {
  shows = await api('GET', '/api/shows').catch(() => shows);
  writeHtml($('#showSel'), shows.shows.map((s) => html`<option value="${esc(s.id)}"${s.id === shows.active ? ' selected' : ''}>${esc(s.id === shows.active ? show.name || s.name : s.name)}</option>`));
}
// paczka .cgpack – te same zasady co w VizRS CG Artist (cgpack.planImport + importPack): najpierw plan (sceny w innej wersji
// albo o zajętej nazwie – decyduje operator), pliki bez duplikatów i bez nadpisywania, strony z numerem, nowym albo przesuniętym;
// bieżący show i antena zostają
const assetBytes = async (n) => { const x = await fetch('/assets/' + encodeURIComponent(n)); if (!x.ok) throw new Error(String(x.status)); return new Uint8Array(await x.arrayBuffer()); };
const uploadAsset = async (n, d) => { const x = await fetch('/api/assets?name=' + encodeURIComponent(n), { method: 'POST', body: d }); const j = await x.json().catch(() => ({})); if (!x.ok) throw new Error(`${n}: ${j.error || x.status}`); return j.name || n; };
// okno natywne: tytuł ze znacznikiem zmian i pytanie przy zamykaniu
// tytuł widać w pasku zadań / Docku: stan anteny i połączenia bez przełączania się do okna
let connected = true;
// przypisanie z innych modułów (import ES jest tylko do odczytu)
export function setConnected(v) { connected = v; }
const winTitle = () => {
  const n = state.layers.length;
  return `${dirtyPage ? '● ' : ''}VizRS Trio${connected ? '' : ' – ⚠ BRAK POŁĄCZENIA'}${n ? ` – NA ANTENIE (${n})` : ''}`;
};

/** Zdarzenia i stan początkowy; woła main.js, moduły w kolejności dawnego skryptu. */
export function init() {
  $('#bSys').onclick = openSys;
  // wylogowanie przeglądarki: ciasteczka tokenu i sesji konta panelu (okno natywne loguje się adresem z tokenem – bez przycisku)
  if (window.__vizrsShell) $('#bLogout').hidden = true;
  $('#bLogout').onclick = async () => {
    if (!(await ask('Wylogować tę przeglądarkę z Trio? Grafika na antenie zostaje.', { ok: 'Wyloguj' }))) return;
    try { await fetch('/logout', { method: 'POST' }); } catch { /* i tak przeładuj */ }
    location.replace(location.pathname);
  };
  $('#errPill').onclick = openSys;
  $('#sysModal').addEventListener('mousedown', (e) => { if (e.target.id === 'sysModal') $('#sysModal').classList.remove('open'); });
  addEventListener('vizrs-font-fail', (e) => toast(`Font „${e.detail.name}” niedostępny offline – użyto zastępczego`, true));
  $('#showSel').onchange = async () => {
    const id = $('#showSel').value, s = shows.shows.find((x) => x.id === id);
    const onAir = state.layers.length ? ' Wszystko zejdzie z anteny.' : '';
    if (!(await ask(`Przełączyć na show „${s?.name || id}”?${onAir}`, { ok: 'Przełącz', danger: !!onAir }))) { $('#showSel').value = shows.active; return; }
    try { await api('POST', '/api/shows/switch/' + encodeURIComponent(id)); setSelId(null); setDraft(null); setDirtyFlag(false); await loadShow(); toast('Show: ' + (s?.name || id)); }
    catch (e) { toast(e.message, true); $('#showSel').value = shows.active; }
  };
  $('#bShowMenu').onclick = async () => {
    const a = await choose('Kopia bieżącego ma te same szablony.', [
      { label: 'Usuń show z listy…', value: '3', danger: true },
      { label: 'Nowy pusty', value: '1' },
      { label: 'Kopia bieżącego', value: '2', main: true },
    ], { title: 'Show / paczka grafiki' });
    if (!a) return;
    try {
      if (a === '3') {
        const other = shows.shows.filter((s) => s.id !== shows.active);
        if (!other.length) return toast('Na liście jest tylko aktywny show', true);
        const s = await choose('Który show usunąć?', other.map((x) => ({ label: x.name, value: x })), { title: 'Usuń show' });
        if (!s) return;
        if (!(await ask(`Usunąć show „${s.name}” z dysku?`, { ok: 'Usuń', danger: true }))) return;
        await api('DELETE', '/api/shows/' + encodeURIComponent(s.id)); await loadShows(); toast('Usunięto: ' + s.name);
        return;
      }
      const name = await askText('Np. nazwa audycji', { title: a === '2' ? 'Kopia bieżącego show' : 'Nowy pusty show', value: a === '2' ? (show.name || 'Show') + ' – kopia' : '', ok: 'Utwórz' });
      if (!name) return;
      await api('POST', '/api/shows', { name, copy: a === '2' });
      setSelId(null); setDraft(null); setDirtyFlag(false); await loadShow(); toast('Utworzono show: ' + name);
    } catch (e) { toast(e.message, true); }
  };
  // ------------------------------------------------------------ show import/eksport
  $('#bExpShow').onclick = async () => {
    const s = await api('GET', '/api/show');
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob([JSON.stringify(s, null, 2)], { type: 'application/json' }));
    a.download = (s.name || 'show').replace(/[^\w.\-]+/g, '_') + '.show.json'; a.click();
  };
  $('#bImpShow').onclick = () => {
    const fi = $('#fileIn'); fi.value = ''; fi.accept = '.json,application/json';
    fi.onchange = async () => {
      const f = fi.files[0]; if (!f) return;
      try {
        const s = JSON.parse(await f.text());
        if (!Array.isArray(s.scenes)) throw new Error('to nie jest plik show VizRS');
        if (!(await ask(`Zastąpić bieżący show plikiem „${f.name}”? (${s.scenes.length} scen, ${(s.pages || []).length} stron)`, { ok: 'Zastąp', danger: true }))) return;
        await api('PUT', '/api/show?force=1', s); setSelId(null); setDirtyFlag(false); await loadShow(); toast('Wczytano show');
      } catch (e) { toast('Import: ' + e.message, true); }
    };
    fi.click();
  };
  $('#bImpPack').onclick = () => {
    const fi = $('#fileIn'); fi.value = ''; fi.accept = '.cgpack,application/zip';
    fi.onchange = async () => {
      const f = fi.files[0]; if (!f) return;
      try {
        const pk = await PACK.readPack(new Uint8Array(await f.arrayBuffer()));
        const [list, cur] = await Promise.all([api('GET', '/api/assets'), api('GET', '/api/show')]);
        const existing = list.map((a) => a.name);
        const plan = await PACK.planImport(pk, { show: cur, existing, fetchAsset: assetBytes, withPages: true });
        const ok = await planDialog(plan, { title: `Import paczki „${f.name}”` });
        if (!ok) return;
        const r = await PACK.importPack(pk, { api, existing, fetchAsset: assetBytes, upload: uploadAsset, show: cur, plan, fixRefs: ok.fixRefs });
        await loadShow();
        toast(PACK.importSummary(r));
        if (r.report.length) await ask(r.report.map((l) => '• ' + l).join('\n'), { title: PACK.importSummary(r), ok: 'OK', cancel: 'Zamknij' });
      } catch (e) { toast('Import paczki: ' + e.message, true); }
    };
    fi.click();
  };
  // eksport wybranych stron (z ich scenami, grafikami i czcionkami) – przenoszenie stron między serwerami i między show
  $('#bExpPack').onclick = async () => {
    try {
      const [cur, list] = await Promise.all([api('GET', '/api/show'), api('GET', '/api/assets')]);
      if (!cur.pages.length) return toast('Show nie ma stron', true);
      const known = list.map((a) => a.name);
      const assetsOf = (sc, ps) => {
        const n = new Set(); sc.forEach((s) => PACK.sceneAssets(s, known).forEach((x) => n.add(x))); ps.forEach((p) => PACK.sceneAssets(p.values || {}, known).forEach((x) => n.add(x)));
        const all = [...n]; return { fonts: all.filter((x) => PACK.FONT_RE.test(x)), images: all.filter((x) => !PACK.FONT_RE.test(x)) };
      };
      const ids = await pickPages({ pages: cur.pages, scenes: cur.scenes, current: selId, assetsOf });
      if (!ids) return;
      const pages = cur.pages.filter((p) => ids.includes(p.id)), sids = new Set(pages.map((p) => p.scene));
      const scenes = cur.scenes.filter((s) => sids.has(s.id));
      const { blob, missing, assets, merged } = await PACK.buildPack({ scenes, pages, known, fetchAsset: assetBytes, generator: 'VizRS Trio' });
      const a = document.createElement('a');
      a.href = URL.createObjectURL(blob);
      const base = pages.length === 1 ? `${pages[0].number}_${pages[0].name || 'strona'}` : `${cur.name || 'show'}_strony`;
      a.download = base.replace(/[^\w.\-]+/g, '_') + '.cgpack'; a.click();
      toast(`Paczka: ${pages.length} stron, ${scenes.length} scen, ${assets.length} plików (${(blob.size / 1024).toFixed(0)} KB)`);
      const notes = [...missing.map((n) => `nie udało się odczytać ${n}`), ...merged.map(([n, t]) => `czcionka ${n} to ten sam plik co ${t} – w paczce raz, jako ${t}`)];
      if (notes.length) await ask(notes.map((l) => '• ' + l).join('\n'), { title: 'Paczka zapisana – uwagi', ok: 'OK', cancel: 'Zamknij' });
    } catch (e) { toast('Eksport paczki: ' + e.message, true); }
  };
  // ------------------------------------------------------------ klawiatura / linia komend
  $('#cmd').addEventListener('keydown', (e) => {
    if (e.key !== 'Enter') return;
    const n = parseInt($('#cmd').value);
    const p = show.pages.find((p) => p.number === n);
    if (p) { trySelect(p.id); $('#cmd').value = ''; $('#cmd').blur(); }
    else toast(`Brak strony ${$('#cmd').value}`, true);
  });
  addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && $('#sysModal').classList.contains('open')) { $('#sysModal').classList.remove('open'); return; }
    const F = { F1: take, F2: cont, F3: out, F4: upd, F6: arm, F8: rdNext };
    if (F[e.key]) { e.preventDefault(); F[e.key](); return; }
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); take(); return; }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') { e.preventDefault(); savePage(); return; }
    if (e.shiftKey && e.key === 'Escape') { e.preventDefault(); clearAll(); return; }
    const typing = /INPUT|TEXTAREA|SELECT/.test(document.activeElement?.tagName);
    if ((!typing || e.ctrlKey || e.altKey || /^F\d/.test(e.key)) && pageHotkey(e)) return;
    if (typing) return;
    if (/^[0-9]$/.test(e.key)) { $('#cmd').focus(); return; }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const list = sortedPages(); const i = list.findIndex((p) => p.id === selId);
      const n = list[Math.max(0, Math.min(list.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)))];
      if (n) trySelect(n.id);
    }
  });
  addEventListener('beforeunload', (e) => { if (dirtyPage) { e.preventDefault(); e.returnValue = ''; } });
  nativeBridge({
    commands: { obs: showObsUrl },
    state: () => ({ title: winTitle(), dirty: dirtyPage, undo: false, redo: false, alert: !connected, scene: null }),
    save: () => savePage(),
    whatChanged: () => `Strona ${draft?.number ?? ''} ma niezapisane zmiany.`,
  });
  // okno natywne: bez przeglądarkowego menu pod prawym przyciskiem; w polach tekstowych zostaje (kopiuj, wklej, pisownia)
  if (document.documentElement.classList.contains('native')) {
    addEventListener('contextmenu', (e) => { if (!e.target.closest('input, textarea, [contenteditable]')) e.preventDefault(); });
  }
  // systemowe menu pod prawym przyciskiem: strona na liście (wybiera ją) i warstwa na antenie
  $('#pages').addEventListener('contextmenu', async (e) => {
    const it = e.target.closest('.item[data-id]'); if (!it || !window.__vizrsShell?.menu) return;
    e.preventDefault();
    if (!(await trySelect(it.dataset.id)) || !draft) return;
    const pid = draft.id, air = !!onAirLayer(pid);
    const acts = { take, arm, cont, upd, out, dup: () => $('#pDup').onclick(), del: () => $('#pDel').onclick() };
    contextMenu([
      { id: 'take', label: `TAKE ${draft.number} (F1)` },
      { id: 'arm', label: 'ARM – podgląd (F6)' },
      '-',
      { id: 'cont', label: 'CONTINUE (F2)', enabled: air },
      { id: 'upd', label: 'UPDATE (F4)', enabled: air },
      { id: 'out', label: 'OUT (F3)', enabled: air },
      '-',
      { id: 'dup', label: 'Duplikuj stronę' },
      { id: 'del', label: 'Usuń stronę…' },
    ], (id) => { if (draft?.id === pid) acts[id]?.(); }); // TAKE tylko strony, dla której otwarto menu
  });
  $('#layers').addEventListener('contextmenu', (e) => {
    const row = e.target.closest('.item'), lay = row?.querySelector('[data-lout]')?.dataset.lout;
    if (lay === undefined || !window.__vizrsShell?.menu) return;
    e.preventDefault();
    // lista warstw odświeża się przy każdej zmianie anteny – komendy idą po nazwie warstwy, nie po elemencie wiersza
    const dock = row.querySelector('[data-ldock]')?.dataset.off, pending = !!row.querySelector('[data-ldeploy]'), L = encodeURIComponent(lay);
    const acts = {
      cont: () => act(() => api('POST', onAnt('/api/continue/' + L)), 'CONTINUE'),
      out: () => act(() => api('POST', onAnt('/api/out/' + L)), 'OUT'),
      dock: () => act(() => api('POST', onAnt(`/api/dock/${L}/${dock === '1' ? 'off' : 'on'}`)), 'Dock'),
      deploy: () => act(() => api('POST', onAnt('/api/deploy/' + L)), 'Wdrożenie'),
    };
    contextMenu([
      { id: 'cont', label: `CONTINUE ${lay} – następny stop` },
      { id: 'out', label: `OUT ${lay} – zdejmij z anteny` },
      ...(dock !== undefined ? ['-', { id: 'dock', label: dock === '1' ? 'Wyłącz powiązanie (dock)' : 'Włącz powiązanie (dock)' }] : []),
      ...(pending ? ['-', { id: 'deploy', label: 'Wdróż nową wersję szablonu' }] : []),
    ], (id) => acts[id]?.());
  });
}
