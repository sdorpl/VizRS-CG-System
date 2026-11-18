// Trio – lista stron i szablonów, wybór strony, edytor strony (pola, czas na kanale, ticker).

import { ScenePlayer, listFields, parseColor, PROPS, walk, squeezeFloor } from '/renderer.js';
import { mountTickerEditor } from '/ticker-items.js';
import { orEmpty, setHtml as writeHtml } from '/dom.js';
import { ask } from '/dialogs.js';
import { $, $$, api, assets, dirtyPage, draft, esc, html, loadShow, lockPage, onAirLayer, otherLocks, pageById, phone, pv, sceneOf, selId, setDirtyFlag, setDraft, setHtml, setPv, setSelId, show, sortedPages, toast } from './core.js';
import { fitPv, pvAuto, pvFrame, pvReframe, setPvAuto, setPvZ } from './preview.js';
import { lastGood, renderStatus, rulesOf, updateCounters } from './fields.js';
import { out, take } from './onair.js';
import { renderBar, view } from './mobile.js';

// ------------------------------------------------------------ lista stron
export function renderPages() {
  const f = $('#filter').value.trim().toLowerCase();
  const list = sortedPages().filter((p) => !f || `${p.number} ${p.name} ${sceneOf(p)?.name}`.toLowerCase().includes(f));
  setHtml($('#pages'), orEmpty(list.map((p) => {
    const L = onAirLayer(p.id), s = sceneOf(p);
    return html`<div class="item ${p.id === selId ? 'sel' : ''}" data-id="${p.id}" title="warstwa: ${s?.layer || 'main'}">
      <span class="dot ${L ? 'on' : ''}"></span><span class="num">${p.number}</span>
      <span><div>${p.name}${otherLocks().find((l) => l.id === p.id) ? html` <span class="pend" title="Edytuje: ${otherLocks().find((l) => l.id === p.id).who}">🔒 ${otherLocks().find((l) => l.id === p.id).who}</span>` : ''}</div><div class="tpl">${s?.name || '⚠ brak szablonu'}</div></span>
      <span class="pill">${s?.layer || 'main'}</span></div>`;
  }), html`<div class="empty">Brak stron. Kliknij ＋ przy szablonie poniżej.</div>`));
  $('#pcount').textContent = `${show.pages.length} stron`;
}
export function renderTemplates() {
  writeHtml($('#tpls'), orEmpty(show.scenes.map((s) => {
    const n = listFields(s).length;
    return html`<div class="item" data-tpl="${s.id}"><span><b>${s.name}</b> <span class="dim" style="font-size:11px">· ${s.layer || 'main'} · ${n} pól</span></span><button class="icon" data-newpage="${s.id}" title="Nowa strona z tego szablonu">＋</button></div>`;
  }), html`<div class="empty">Brak scen – utwórz je w Artist.</div>`));
}
// true = strona wybrana; bez niezapisanych zmian wybór dzieje się od razu (przed pierwszym await)
export async function trySelect(id) {
  if (id === selId) return true;
  if (dirtyPage && !(await ask('Strona ma niezapisane zmiany.', { ok: 'Porzuć zmiany', cancel: 'Zostań', danger: true }))) return false;
  if (id !== selId) selectPage(id);
  return true;
}
// ------------------------------------------------------------ edytor strony
export function selectPage(id, keepScroll) {
  const p = pageById(id); if (!p) return;
  if (selId !== id) { lockPage(id); setPvZ({ k: 1, x: 0, y: 0 }); }
  setSelId(id); setDraft(structuredClone(p)); draft.values ||= {}; setDirtyFlag(false);
  renderEditor(); renderPages(); renderStatus(); renderBar();
  if (!keepScroll) $(`#pages .item[data-id="${id}"]`)?.scrollIntoView({ block: 'nearest' });
}
// ------------------------------------------------------------ czas strony na kanale (VizRS CG Playout)
/** Opis `page.playout` dla nagłówka: `at` < 0 = przed końcem, `out` = zejście przed końcem. */
function playoutLabel(P) {
  if (!P || (P.at == null && P.out == null && !P.ref)) return '⏱ kanał: domyślnie';
  const at = P.at == null ? 'wejście domyślne' : P.at < 0 ? `${-P.at} s przed końcem` : `${P.at} s po starcie`;
  return `⏱ kanał: ${at}${P.out != null ? `, zejście ${P.out} s przed końcem` : ''}${P.ref ? ` (${P.ref === 'item' ? 'pozycji' : 'audycji'})` : ''}`;
}
function renderPlayout() {
  const P = draft?.playout || {};
  $('#eplay').hidden = !draft;
  $('#eplAt').value = P.at == null ? '' : P.at < 0 ? 'end' : 'start';
  $('#eplSec').value = P.at == null ? '' : Math.abs(P.at);
  $('#eplSec').disabled = P.at == null;
  $('#eplOut').value = P.out ?? '';
  $('#eplRef').value = P.ref || '';
  $('#eplaySum').textContent = playoutLabel(draft?.playout);
}
function readPlayout() {
  const num = (el) => { const v = parseFloat(el.value); return Number.isFinite(v) ? Math.min(86400, Math.max(0, v)) : null; };
  const mode = $('#eplAt').value, sec = num($('#eplSec')), out = num($('#eplOut')), ref = $('#eplRef').value;
  const P = {};
  // „przed końcem” bez liczby = 60 s – typowa belka „Za chwilę”
  if (mode) P.at = mode === 'end' ? -(sec || 60) : (sec ?? 0);
  if (out != null) P.out = out;
  if (ref) P.ref = ref;
  if (Object.keys(P).length) draft.playout = P; else delete draft.playout;
  renderPlayout(); setDirtyPage(true);
}
export function setDirtyPage(d) { const was = dirtyPage; setDirtyFlag(d); $('#eSave').classList.toggle('dirty', d); $('#eSave').textContent = d ? 'Zapisz ●' : 'Zapisz'; if (was !== d) renderBar(); }
function fieldInput(f, v) {
  const id = html`data-f="${f.id}"`;
  switch (f.nodeType === 'variant' ? 'variant' : f.type) {
    case 'num': return html`<input type="number" step="any" ${id} value="${v}">`;
    case 'bool': return html`<input type="checkbox" ${id} ${v === true || v === 'true' ? 'checked' : ''}>`;
    case 'color': return html`<div class="row"><input type="color" data-fc="${f.id}" value="${hex6(v)}"><input class="grow mono" ${id} value="${v}"></div>`;
    case 'select': return html`<select ${id}>${f.opts.map((o) => html`<option ${o == v ? 'selected' : ''}>${o}</option>`)}</select>`;
    case 'variant': return html`<div class="segm">${f.opts.map((o) => html`<label class="${o == v ? 'on' : ''}"><input type="radio" name="v_${f.id}" ${id} value="${o}" ${o == v ? 'checked' : ''}>${o}</label>`)}</div>`;
    case 'font': return html`<input list="fontlist" ${id} value="${v}">`;
    case 'image': {
      const imgs = assets.filter((a) => !/\.(ttf|otf|woff2?)$/i.test(a.name)).map((a) => a.name);
      if (v && !imgs.includes(v)) imgs.unshift(v);
      return html`<div class="row"><select class="grow" ${id}><option value="">— brak —</option>${imgs.map((n) => html`<option ${n === v ? 'selected' : ''}>${n}</option>`)}</select><button class="icon" data-up="${f.id}" title="Wgraj">⤒</button></div>`;
    }
    default: {
      const R = f.rules || {};
      // wiadomości tickera: edytor listy (ticker-items.js) zapisujący do ukrytego pola
      if (f.nodeType === 'ticker' && f.prop === 'text') return html`<div data-tkhost="${f.id}"></div><textarea hidden data-tk="1" ${id}>${v}</textarea>`;
      const maxLines = R.maxLines || f.lines || 1;
      const multi = !R.oneLine && (f.nodeType === 'ticker' || maxLines > 1 || String(v ?? '').includes('\n'));
      const rows = f.nodeType === 'ticker' ? 6 : Math.max(2, maxLines);
      const attrs = html`${id} ${R.readonly ? 'readonly' : ''} ${R.spell === false ? html`spellcheck="false"` : html`spellcheck="true" lang="pl"`}`;
      const rich = f.nodeType === 'text' && R.rich !== false && !R.readonly;
      const tb = rich ? html`<div class="rtb" data-rtb="${f.id}"><button data-fmt="b" title="Pogrubienie {b}"><b>B</b></button><button data-fmt="i" title="Kursywa {i}"><i>I</i></button><button data-fmt="u" title="Podkreślenie {u}"><u>U</u></button><input type="color" data-fmtc="${f.id}" value="#e30613" title="Kolor zaznaczenia {c=…}"><select data-fmtsz="${f.id}" title="Rozmiar zaznaczenia {sz=…}"><option value="">rozm.</option><option>70</option><option>85</option><option>120</option><option>150</option><option>200</option></select><button data-fmt="hl" title="Wyróżnienie [[…]]">[[ ]]</button><button data-fmt="clr" title="Usuń formatowanie z zaznaczenia (lub całego pola)">✕</button></div>` : '';
      const ctl = multi ? html`<textarea class="big" ${attrs} rows="${rows}">${v}</textarea>` : html`<input class="big" ${attrs} value="${v}">`;
      return html`${tb}${ctl}<div class="cnt" data-cnt="${f.id}"></div><div class="sq" data-sq="${f.id}"></div>`;
    }
  }
}
const hex6 = (c) => { const p = parseColor(c); if (!p) return '#000000'; return '#' + p.slice(0, 3).map((x) => Math.round(x).toString(16).padStart(2, '0')).join(''); };
export function renderEditor() {
  pv?.destroy(); setPv(null); lastGood.clear();
  if (!draft) { writeHtml($('#fields'), html`<div class="empty">Wybierz stronę z listy lub wpisz jej numer.</div>`); $('#enum').value = ''; $('#ename').value = ''; $('#eplay').hidden = true; return; }
  $('#enum').value = draft.number; $('#ename').value = draft.name; $('#ehotkey').value = draft.hotkey || '';
  renderPlayout();
  const s = sceneOf(draft);
  if (!s) { writeHtml($('#fields'), html`<div class="empty">⚠ Szablon tej strony nie istnieje.</div>`); return; }
  // kolejność Tab: pola z regułą `tab` rosnąco, reszta w kolejności sceny
  const fields = listFields(s).map((f, i) => ({ f, i })).sort((a, b) => ((a.f.rules?.tab ?? 1e6) - (b.f.rules?.tab ?? 1e6)) || a.i - b.i).map((x) => x.f);
  writeHtml($('#fields'), fields.length ? fields.map((f) => {
    const v = draft.values[f.id] ?? f.default;
    const R = f.rules || {}, ml = R.maxLines || f.lines;
    const tk = f.nodeType === 'ticker' && f.prop === 'text';
    return html`<div class="fl"><label>${f.label}<small>${f.id}${tk ? ' · Enter = nowa wiadomość · Ctrl+↑↓ = przesuń · najedź = podgląd' : f.type === 'text' ? ' · [[…]] = wyróżnienie' : ''}${ml > 1 && !tk ? ` · Enter = nowy wiersz (maks. ${ml})` : ''}</small>${R.help ? html`<div class="hlp">${R.help}</div>` : ''}</label><div>${fieldInput(f, v)}</div></div>`;
  }) : html`<div class="empty">Ten szablon nie ma pól kontrolnych (dodaj je w Artist ⛓).</div>`);
  mountTickers(s);
  updateCounters();
  makePv();
  setDirtyPage(dirtyPage);
}
/** Na telefonie podgląd działa tylko na widocznym widoku Edycja – w tle kosztowałby baterię. */
const pvWanted = () => !phone() || (view === 'edit' && !document.hidden);
function makePv() {
  pv?.destroy(); setPv(null);
  const s = sceneOf(draft);
  if (!s || !pvWanted()) return;
  setPv(new ScenePlayer($('#pvStage'), s, draft.values));
  pv.seekStop(0);
  setPvAuto(true); setPvZ({ k: 1, x: 0, y: 0 });
  fitPv();
  pvFrame();
  requestAnimationFrame(() => { fitPv(); if (pvAuto) pvFrame(); }); pvReframe(); // układ panelu i fonty ustalają się po otwarciu strony
  showSqueeze();
}
export function syncPv() { if (pvWanted() !== !!pv) makePv(); }
/** Edytory wiadomości dla pól tickera; zmiany idą przez ukryte pole, więc draft, UPDATE i zapis działają jak dla tekstu. */
export const tkEditors = new Map();
function mountTickers(s) {
  tkEditors.clear();
  for (const host of $$('[data-tkhost]')) {
    const fid = host.dataset.tkhost, ta = $(`[data-f="${CSS.escape(fid)}"]`);
    let node = null;
    walk(s.nodes, (n) => { if (!node && n.type === 'ticker' && n.fields?.text === fid) node = n; });
    const pr = node?.props || {};
    const ed = mountTickerEditor(host, {
      value: ta.value,
      catColors: pr.catColors || '', catBg: pr.catBg || PROPS.catBg.d, catFg: pr.catFg || PROPS.catFg.d,
      label: () => (node?.fields?.label ? $(`[data-f="${CSS.escape(node.fields.label)}"]`)?.value : '') || pr.label || '',
      rules: rulesOf(fid),
      onChange: (v) => { ta.value = v; ta.dispatchEvent(new Event('input', { bubbles: true })); },
      onHover: (i) => pv?.tickerPin(fid, i),
    });
    tkEditors.set(fid, ed);
  }
}
/** Pokazuje operatorowi stopień ściśnięcia każdego wiersza (jak wskaźnik squeeze w Viz). */
export function showSqueeze() {
  if (!pv) return;
  for (const [fid, ed] of tkEditors) ed.setSqueeze(pv.tickerSqueeze(fid));
  for (const el of $$('[data-sq]')) {
    const its = pv.items.filter((i) => i.vis && i.node.fields && Object.values(i.node.fields).includes(el.dataset.sq) && i.sq);
    if (!its.length) { el.textContent = ''; continue; }
    // ten sam tekst w kilku węzłach (np. warianty): pokazujemy najmocniej ściśnięty
    const it = its.reduce((a, b) => (Math.min(...b.sq) < Math.min(...a.sq) || b.over ? b : a));
    const one = it.sq.length === 1;
    writeHtml(el, html`${it.sq.map((s, i) => html`<span class="${s < 0.7 || it.over ? 'bad' : s < 0.999 ? 'warn' : ''}">${one ? 'ściśnięcie' : 'w' + (i + 1)}: ${Math.round(s * 100)}%</span>`)}${it.over ? html`<span class="bad" title="Tekst wymaga ściśnięcia poniżej granicy ${Math.round(squeezeFloor(it.p('squeezeMin')) * 100)}% (squeezeMin) – wystaje poza ramkę">⚠ nie mieści się – skróć tekst</span>` : ''}`);
  }
}

/** Zdarzenia i stan początkowy; woła main.js, moduły w kolejności dawnego skryptu. */
export function init() {
  $('#pages').addEventListener('click', (e) => { const it = e.target.closest('.item[data-id]'); if (it) trySelect(it.dataset.id); });
  // na telefonie dotknięcie tylko wybiera stronę – podwójne dotknięcie nie może wysłać grafiki na antenę
  $('#pages').addEventListener('dblclick', (e) => { if (phone()) return; const it = e.target.closest('.item[data-id]'); if (it) trySelect(it.dataset.id).then((ok) => ok && take()); });
  $('#filter').addEventListener('input', renderPages);
  $('#tpls').addEventListener('click', async (e) => {
    const b = e.target.closest('[data-newpage]'); if (!b) return;
    const s = show.scenes.find((x) => x.id === b.dataset.newpage);
    const values = {}; for (const f of listFields(s)) values[f.id] = f.default;
    try {
      const p = await api('POST', '/api/pages', { scene: s.id, name: s.name, values });
      show.pages.push(p); setDirtyFlag(false); selectPage(p.id); renderPages(); toast(`Strona ${p.number} utworzona`);
    } catch (err) { toast(err.message, true); }
  });
  $('#pDup').onclick = async () => {
    if (!draft) return;
    try {
      const p = await api('POST', '/api/pages', { scene: draft.scene, name: draft.name + ' kopia', values: draft.values });
      show.pages.push(p); setDirtyFlag(false); selectPage(p.id); renderPages();
    } catch (err) { toast(err.message, true); }
  };
  $('#pDel').onclick = async () => {
    if (!selId || !(await ask(`Usunąć stronę ${draft.number} „${draft.name}”?`, { ok: 'Usuń', danger: true }))) return;
    await api('DELETE', '/api/pages/' + selId);
    show.pages = show.pages.filter((p) => p.id !== selId); setSelId(null); setDraft(null); setDirtyFlag(false);
    await loadShow();
  };
  for (const id of ['#eplAt', '#eplSec', '#eplOut', '#eplRef']) $(id).addEventListener('change', () => { if (draft) readPlayout(); });
  document.fonts?.addEventListener?.('loadingdone', () => setTimeout(() => { showSqueeze(); pvReframe(); }, 50));
}
