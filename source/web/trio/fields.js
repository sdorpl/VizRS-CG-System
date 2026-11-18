// Trio – pola strony: status, warstwa, reguły, formatowanie, edycja w podglądzie, Word Replace, skrót klawiszowy, zapis.

import { listFields, layerZ, PROPS, stripMarkup } from '/renderer.js';
import { ask, askText } from '/dialogs.js';
import { visLen, limitText, polish } from '/field-rules.js';
import { setHtml as writeHtml } from '/dom.js';
import { $, $$, api, draft, esc, html, loadState, lockedBy, onAirLayer, onAnt, pv, sceneOf, selId, setAssets, setDirtyFlag, setDraft, show, state, toast } from './core.js';
import { renderEditor, renderPages, selectPage, setDirtyPage, showSqueeze, tkEditors } from './pages.js';
import { pvPanned, pvReframe, setPvPanned } from './preview.js';
import { act } from './onair.js';

export function renderStatus() {
  renderLayerSel();
  if (!draft) { $('#estatus').textContent = ''; return; }
  const s = sceneOf(draft), L = onAirLayer(draft.id);
  const st = s ? html`szablon <b>${esc(s.name)}</b> · stopów: ${(s.stops || []).length}` : '';
  writeHtml($('#estatus'), html`${lockedBy ? html`<b style="color:#ff9a3d">🔒 edytuje ${lockedBy}</b> · ` : ''}${L ? html`<b class="on">● ON AIR</b> (stop ${L.stop + 1}/${Math.max(1, L.stops)}) · ` : ''}${st}`);
}
// ---- warstwa szablonu strony: zmiana także na antenie, bez ponownej animacji
const NEW_LAYER = '+nowa';
function renderLayerSel() {
  const s = sceneOf(draft), sel = $('#elayer');
  $('#elayerBox').hidden = !s;
  if (!s || document.activeElement === sel) return;
  const cur = s.layer || 'main', names = new Set(['back', 'main', 'front', cur]);
  for (const x of show.scenes) names.add(x.layer || 'main');
  for (const l of state.layers) names.add(l.layer);
  const air = (n) => state.layers.find((l) => l.layer === n);
  const label = (n) => (air(n) ? `${n} ● ${air(n).page?.number ?? ''}` : n);
  const pages = show.pages.filter((p) => p.scene === s.id).length;
  const opts = html`${[...names].sort((a, b) => layerZ(a) - layerZ(b) || a.localeCompare(b)).map((n) => html`<option value="${n}">${label(n)}</option>`)}<option value="${NEW_LAYER}">＋ nowa warstwa…</option>`;
  if (sel.dataset.html !== String(opts)) { writeHtml(sel, opts); sel.dataset.html = opts; }
  sel.value = cur;
  sel.title = `Warstwa należy do szablonu „${s.name}”: zmiana obejmie stron: ${pages}. ● = zajęta na antenie`;
}
async function setLayer(layer) {
  const s = sceneOf(draft); if (!s) return;
  const post = (force) => api('POST', `/api/layer/${draft.number}${force ? '?force=1' : ''}`, { layer });
  try {
    const r = await post().catch(async (e) => {
      if (!/^konflikt/.test(e.message)) throw e;
      return (await ask(`${e.message.replace(/^konflikt: /, '')}.\n\nZastąpienie zdejmie ją z anteny.`, { ok: 'Zastąp', cancel: 'Nie zmieniaj warstwy', danger: true })) ? post(true) : null;
    });
    if (r) toast(`Szablon „${s.name}”: warstwa ${r.layer} (stron: ${r.pages})`);
  } catch (e) { toast(`Warstwa: ${e.message}`, true); }
  loadState();
}
// ---- reguły pól (F3.1): limity, wersaliki, typografia, formatowanie, Word Replace
const fieldDef = (fid) => listFields(sceneOf(draft) || { nodes: [] }).find((x) => x.id === fid);
export const rulesOf = (fid) => fieldDef(fid)?.rules || {};
export const lastGood = new Map();
export function updateCounters() {
  for (const c of $$('[data-cnt]')) {
    const fid = c.dataset.cnt, R = rulesOf(fid), el = $(`[data-f="${CSS.escape(fid)}"]`);
    if (!R.maxChars || !el) { c.textContent = ''; continue; }
    const n = visLen(el.value), soft = R.softMax || Math.floor(R.maxChars * 0.85);
    c.textContent = `${n} / ${R.maxChars}`;
    c.className = 'cnt ' + (n >= R.maxChars ? 'bad' : n >= soft ? 'warn' : '');
  }
}
/** Egzekwuje reguły przy wpisywaniu; zwraca false, gdy zmiana została cofnięta. */
function enforceRules(el) {
  const fid = el.dataset.f, R = rulesOf(fid);
  if (R.readonly) return false;
  if (el.dataset.tk) return true; // ticker: reguły (limit wiadomości, znaków na wiadomość) pilnuje edytor listy
  const caret = el.selectionStart, fromEnd = el.value.length - (caret ?? el.value.length);
  const { v, err } = limitText(el.value, R, lastGood.get(fid));
  if (err) toast(err, true);
  if (v !== el.value) { el.value = v; if (caret != null) el.setSelectionRange(Math.max(0, v.length - fromEnd), Math.max(0, v.length - fromEnd)); }
  lastGood.set(fid, v);
  return true;
}
/** Reguły stosowane po zakończeniu wpisywania (typografia działa na całym tekście, więc nie w trakcie pisania). */
export function applyFinalRules() {
  if (!draft) return;
  for (const el of $$('#fields [data-f]')) {
    const R = rulesOf(el.dataset.f);
    if (!R.typo || !/^(INPUT|TEXTAREA)$/.test(el.tagName)) continue;
    const nv = polish(el.value);
    if (nv !== el.value) { el.value = nv; draft.values[el.dataset.f] = nv; setDirtyPage(true); tkEditors.get(el.dataset.f)?.setValue(nv); }
  }
  pv?.update(draft.values); showSqueeze(); updateCounters(); pvReframe();
}
// ---- pasek formatowania (znaczniki {b} {i} {u} {c=} {sz=} [[ ]])
function wrapSel(el, open, close) {
  const a = el.selectionStart, b = el.selectionEnd, v = el.value;
  const before = v.slice(0, a), sel = v.slice(a, b), after = v.slice(b);
  if (before.endsWith(open) && after.startsWith(close)) { // przełączenie: zdejmij
    el.value = before.slice(0, -open.length) + sel + after.slice(close.length);
    el.setSelectionRange(a - open.length, b - open.length);
  } else {
    el.value = before + open + sel + close + after;
    if (a === b) el.setSelectionRange(a + open.length, a + open.length); else el.setSelectionRange(a + open.length, b + open.length);
  }
  el.focus(); el.dispatchEvent(new Event('input', { bubbles: true }));
}
const TAGS = { b: ['{b}', '{/b}'], i: ['{i}', '{/i}'], u: ['{u}', '{/u}'], hl: ['[[', ']]'] };
// ---- edycja w podglądzie (jak w Viz Trio): klik w element grafiki przenosi do jego pola
export function hitField(x, y) {
  if (!pv) return null;
  let best = null;
  for (const it of pv.items) {
    if (!it.vis || !it.node.fields) continue;
    const props = Object.keys(it.node.fields);
    const prop = props.find((k) => PROPS[k]?.t === 'text') || props[0];
    const fid = it.node.fields[prop];
    if (!fid || !$(`[data-f="${CSS.escape(fid)}"]`)) continue;
    const r = it.el.getBoundingClientRect();
    if (r.width < 3 || r.height < 3 || x < r.left || x > r.right || y < r.top || y > r.bottom) continue;
    const area = r.width * r.height;
    if (!best || area < best.area) best = { fid, r, area };
  }
  return best;
}
// ---- Word Replace (Ctrl+Spacja): skrót przed kursorem → słowo z listy stanowiska
export let wrList = {};
// przypisanie z innych modułów (import ES jest tylko do odczytu)
export function setWrList(v) { wrList = v; }
async function loadWr() { wrList = await api('GET', '/api/wordreplace').catch(() => ({})); }
const wrMenu = document.createElement('div');
let wrCtx = null;
function wrApply(el, hit, word) {
  const a = el.selectionStart, lineStart = el.value.lastIndexOf('\n', a - 1) + 1;
  const startIdx = lineStart + hit.start; // indeks w znakach (UTF-16 ≈ znaki dla typowych skrótów)
  el.value = el.value.slice(0, startIdx) + word + el.value.slice(a);
  el.setSelectionRange(startIdx + word.length, startIdx + word.length);
  wrMenu.style.display = 'none'; wrCtx = null;
  el.focus(); el.dispatchEvent(new Event('input', { bubbles: true }));
}
function wordReplaceAt(el) {
  if (!window.CGText) return;
  const a = el.selectionStart, lineStart = el.value.lastIndexOf('\n', a - 1) + 1;
  const hit = CGText.wordReplaceFind(el.value.slice(lineStart, a), wrList);
  if (!hit) return toast('Brak skrótu przed kursorem (lista zamian: ⚙ Stan)', true);
  if (hit.words.length === 1) return wrApply(el, hit, hit.words[0]);
  const r = el.getBoundingClientRect();
  Object.assign(wrMenu.style, { display: 'block', left: r.left + 'px', top: r.bottom + 2 + 'px' });
  writeHtml(wrMenu, hit.words.map((w, i) => html`<div data-wi="${i}"><b>${i + 1}</b> ${w}</div>`));
  wrCtx = { el, hit };
}
// ---- skrót klawiszowy strony
const RESERVED = new Set(['F1', 'F2', 'F3', 'F4', 'F6', 'F8', 'Ctrl+S', 'Ctrl+Enter', 'Shift+Escape', 'Escape']);
function comboOf(e) {
  const k = e.key;
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(k)) return '';
  const name = k.length === 1 ? k.toUpperCase() : k === ' ' ? 'Space' : k;
  return [e.ctrlKey || e.metaKey ? 'Ctrl' : '', e.altKey ? 'Alt' : '', e.shiftKey ? 'Shift' : '', name].filter(Boolean).join('+');
}
const comboOk = (c) => /^(Ctrl|Alt)\+/.test(c) || /^(Shift\+)?F(7|9|1[0-2])$/.test(c) || /^F(7|9|1[0-2])$/.test(c);
export function pageHotkey(e) {
  const c = comboOf(e);
  if (!c) return false;
  const p = show.pages.find((x) => x.hotkey === c);
  if (!p) return false;
  e.preventDefault();
  const L = onAirLayer(p.id);
  act(() => (L ? api('POST', onAnt('/api/out/' + encodeURIComponent(L.layer))) : api('POST', onAnt(`/api/take/${p.number}`))), 'Skrót');
  return true;
}
export const isConflict = (e) => /^konflikt/.test(e.message || '');
export async function savePage(force) {
  if (!draft) return true;
  applyFinalRules();
  if (lockedBy && !(await ask(`Tę stronę edytuje ${lockedBy}.`, { ok: 'Zapisz mimo to' }))) return false;
  try {
    const body = force === true ? (({ rev, ...r }) => r)(draft) : draft; // bez rev = świadome nadpisanie
    const p = await api('PUT', '/api/pages/' + draft.id, body);
    const i = show.pages.findIndex((x) => x.id === p.id); show.pages[i] = p;
    setDraft(structuredClone(p)); setDirtyPage(false); renderPages(); toast(`Zapisano stronę ${p.number}`);
    return true;
  } catch (e) {
    if (isConflict(e)) {
      if (await ask('Ta strona została zmieniona przez kogoś innego (lub w innym oknie).', { ok: 'Nadpisz moją wersją', cancel: 'Nie zapisuj', danger: true })) return savePage(true);
      toast('Konflikt zapisu – zmiany nie zapisane', true);
    } else toast(e.message, true);
    return false;
  }
}

/** Zdarzenia i stan początkowy; woła main.js, moduły w kolejności dawnego skryptu. */
export function init() {
  $('#elayer').onchange = async (e) => {
    const sel = e.target, s = sceneOf(draft), cur = s?.layer || 'main';
    let layer = sel.value;
    if (layer === NEW_LAYER) layer = await askText('Litery, cyfry, - i _. Numer z<N>_ ustala kolejność, np. z5_ticker', { title: 'Nowa warstwa', ok: 'Utwórz', placeholder: 'z5_ticker' });
    sel.blur(); // skróty klawiszowe znów działają od razu
    if (layer && layer !== cur) setLayer(layer); else sel.value = cur;
  };
  $('#fields').addEventListener('change', (e) => { if (e.target.dataset.f && rulesOf(e.target.dataset.f).typo) applyFinalRules(); });
  $('#fields').addEventListener('mousedown', (e) => { if (e.target.closest('.rtb button')) e.preventDefault(); });
  $('#fields').addEventListener('click', (e) => {
    const bt = e.target.closest('.rtb button'); if (!bt) return;
    const fid = bt.closest('.rtb').dataset.rtb, el = $(`[data-f="${CSS.escape(fid)}"]`); if (!el) return;
    if (bt.dataset.fmt === 'clr') {
      const a = el.selectionStart, b = el.selectionEnd;
      if (a === b) el.value = stripMarkup(el.value); else el.value = el.value.slice(0, a) + stripMarkup(el.value.slice(a, b)) + el.value.slice(b);
      el.focus(); el.dispatchEvent(new Event('input', { bubbles: true })); return;
    }
    const t = TAGS[bt.dataset.fmt]; if (t) wrapSel(el, t[0], t[1]);
  });
  $('#fields').addEventListener('change', (e) => {
    const c = e.target.dataset.fmtc, z = e.target.dataset.fmtsz;
    if (c) { const el = $(`[data-f="${CSS.escape(c)}"]`); if (el) wrapSel(el, `{c=${e.target.value}}`, '{/c}'); }
    if (z && e.target.value) { const el = $(`[data-f="${CSS.escape(z)}"]`); if (el) wrapSel(el, `{sz=${e.target.value}}`, '{/sz}'); e.target.value = ''; }
  });
  $('#pvBox').addEventListener('mousemove', (e) => {
    const h = hitField(e.clientX, e.clientY), box = $('#pvBox').getBoundingClientRect(), hi = $('#pvHit');
    $('#pvBox').classList.toggle('edit', !!h);
    if (!h) { hi.style.display = 'none'; return; }
    Object.assign(hi.style, { display: 'block', left: h.r.left - box.left + 'px', top: h.r.top - box.top + 'px', width: h.r.width + 'px', height: h.r.height + 'px' });
  });
  $('#pvBox').addEventListener('mouseleave', () => { $('#pvHit').style.display = 'none'; });
  $('#pvBox').addEventListener('click', (e) => {
    if (pvPanned) { setPvPanned(false); return; }
    const h = hitField(e.clientX, e.clientY); if (!h) return;
    const ed = tkEditors.get(h.fid);
    if (ed) { ed.el.scrollIntoView({ block: 'nearest' }); ed.focus(); return; }
    const el = $(`[data-f="${CSS.escape(h.fid)}"]`);
    el.scrollIntoView({ block: 'nearest' }); el.focus(); if (el.select) el.select();
  });
  loadWr();
  wrMenu.id = 'wrMenu'; document.body.appendChild(wrMenu);
  wrMenu.addEventListener('mousedown', (e) => { e.preventDefault(); const d = e.target.closest('[data-wi]'); if (d && wrCtx) wrApply(wrCtx.el, wrCtx.hit, wrCtx.hit.words[+d.dataset.wi]); });
  $('#fields').addEventListener('keydown', (e) => {
    const el = e.target;
    if (wrCtx && wrMenu.style.display === 'block') {
      if (/^[1-9]$/.test(e.key) && wrCtx.hit.words[+e.key - 1] !== undefined) { e.preventDefault(); wrApply(wrCtx.el, wrCtx.hit, wrCtx.hit.words[+e.key - 1]); return; }
      if (e.key === 'Escape') { e.preventDefault(); wrMenu.style.display = 'none'; wrCtx = null; return; }
    }
    if (e.ctrlKey && e.code === 'Space' && /^(INPUT|TEXTAREA)$/.test(el.tagName) && el.dataset.f) { e.preventDefault(); wordReplaceAt(el); return; }
    // Enter w polu jednoliniowym / przy limicie linii nie wstawia nowego wiersza
    if (e.key === 'Enter' && el.tagName === 'TEXTAREA' && el.dataset.f) {
      const R = rulesOf(el.dataset.f), ml = R.oneLine ? 1 : R.maxLines || 0;
      if (ml && el.value.split('\n').length >= ml && !e.ctrlKey) { e.preventDefault(); if (ml > 1) toast(`Maksymalnie ${ml} linie`, true); }
    }
  });
  addEventListener('mousedown', (e) => { if (!e.target.closest('#wrMenu')) { wrMenu.style.display = 'none'; wrCtx = null; } });
  $('#fields').addEventListener('input', (e) => {
    const el = e.target;
    if (el.dataset.f && /^(INPUT|TEXTAREA)$/.test(el.tagName) && !enforceRules(el)) return;
    updateCounters();
    if (el.dataset.fc) {
      const tx = $(`[data-f="${CSS.escape(el.dataset.fc)}"]`); tx.value = el.value;
      draft.values[el.dataset.fc] = el.value;
    } else if (el.dataset.f) {
      draft.values[el.dataset.f] = el.type === 'checkbox' ? el.checked : el.value;
      if (el.type === 'radio') $$(`input[name="${CSS.escape(el.name)}"]`).forEach((r) => r.parentElement.classList.toggle('on', r.checked));
    } else return;
    setDirtyPage(true);
    pv?.update(draft.values);
    showSqueeze();
    pvReframe();
  });
  $('#fields').addEventListener('click', (e) => {
    const b = e.target.closest('[data-up]'); if (!b) return;
    const fi = $('#fileIn'); fi.value = ''; fi.accept = 'image/*,video/mp4,video/webm';
    fi.onchange = async () => {
      const f = fi.files[0]; if (!f) return;
      const r = await fetch('/api/assets?name=' + encodeURIComponent(f.name), { method: 'POST', body: f });
      const j = await r.json(); if (!r.ok) return toast(j.error, true);
      setAssets(await api('GET', '/api/assets'));
      draft.values[b.dataset.up] = j.name; setDirtyPage(true); renderEditor();
    };
    fi.click();
  });
  $('#ehotkey').addEventListener('keydown', (e) => {
    if (!draft) return;
    e.preventDefault(); e.stopPropagation();
    if (e.key === 'Backspace' || e.key === 'Delete') { draft.hotkey = ''; $('#ehotkey').value = ''; setDirtyPage(true); return; }
    const c = comboOf(e);
    if (!c) return;
    if (RESERVED.has(c) || !comboOk(c)) return toast(`Skrót ${c} niedozwolony – użyj Ctrl/Alt+klawisz albo F7, F9–F12`, true);
    const other = show.pages.find((p) => p.id !== draft.id && p.hotkey === c);
    if (other) return toast(`Skrót ${c} jest już przypisany do strony ${other.number} „${other.name}”`, true);
    draft.hotkey = c; $('#ehotkey').value = c; setDirtyPage(true);
  });
  $('#ename').addEventListener('input', () => { draft.name = $('#ename').value; setDirtyPage(true); });
  $('#enum').addEventListener('input', () => { draft.number = parseInt($('#enum').value) || draft.number; setDirtyPage(true); });
  $('#eSave').onclick = () => savePage();
  $('#eRevert').onclick = () => { if (selId) { setDirtyFlag(false); selectPage(selId); } };
}
