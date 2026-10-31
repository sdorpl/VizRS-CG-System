// Edytor wiadomości tickera – wspólny dla Trio (pole strony) i Artist (wiadomości próbne).
// Widoki: lista (kategoria + treść), grupy po kategoriach (przeciąganie między grupami) i tekst surowy.
// Wynik zawsze trafia do pola jako wiersze `KAT#kolor|treść` (ticker-model.js), więc reszta systemu się nie zmienia.
import { parseTickerItems, serializeTickerItems, parseCatColors, pageCatColors, catColorOf, checkTickerItems, TICKER_LIMITS } from '/ticker-model.js';
import { openColorPicker, normHex } from '/color-picker.js';

const STYLE = `
.tkx { border: 1px solid var(--line, #333); border-radius: 4px; background: var(--panel2, #1b1d21); }
.tkx-bar { display: flex; gap: 4px; align-items: center; padding: 3px 4px; border-bottom: 1px solid var(--line, #333); font-size: 11px; }
.tkx-bar button { padding: 1px 8px; font-size: 11px; }
.tkx-bar button.on { background: var(--acc, #f0a020); color: #1a1100; border-color: var(--acc, #f0a020); font-weight: 700; }
.tkx-bar .tkx-info { margin-left: auto; color: var(--dim, #888); font: 11px var(--mono, monospace); }
.tkx-body { padding: 4px; display: flex; flex-direction: column; gap: 3px; max-height: 340px; overflow: auto; }
.tkx-row { display: grid; grid-template-columns: 22px 18px minmax(70px, 130px) 1fr auto auto auto auto; gap: 4px; align-items: center; border-radius: 3px; }
.tkx-row.g { grid-template-columns: 22px 1fr auto auto auto; }
.tkx-row:hover { background: rgba(255,255,255,.04); }
.tkx-row.over { outline: 1px dashed var(--acc, #f0a020); }
.tkx-row.drag { opacity: .4; }
.tkx-n { font: 11px var(--mono, monospace); color: var(--dim, #888); text-align: right; cursor: grab; user-select: none; }
.tkx .tkx-dot { width: 18px; height: 18px; min-width: 0; padding: 0; border-radius: 4px; border: 1px solid rgba(255,255,255,.35); }
.tkx .tkx-dot:not(:disabled) { cursor: pointer; } .tkx .tkx-dot:not(:disabled):hover { outline: 1px solid var(--acc, #f0a020); }
.tkx .tkx-dot:disabled { opacity: .45; border-style: dashed; }
.tkx .tkx-dot.bare { border-style: dashed; }
.tkx .tkx-dot.mix { outline: 1px dashed var(--acc, #f0a020); outline-offset: 1px; }
.tkx input, .tkx select { width: 100%; min-width: 0; }
.tkx-txt { font-size: 15px; padding: 4px 6px; }
.tkx-meta { font: 10px var(--mono, monospace); color: var(--dim, #888); white-space: nowrap; display: flex; gap: 4px; }
.tkx-meta .warn { color: var(--acc, #f0a020); } .tkx-meta .bad { color: #ff6a66; font-weight: 700; }
.tkx-row button, .tkx-card button { padding: 0 5px; min-width: 20px; font-size: 11px; }
.tkx-add { align-self: flex-start; font-size: 11px; padding: 2px 10px; }
.tkx-card { border: 1px solid var(--line, #333); border-radius: 4px; padding: 3px; display: flex; flex-direction: column; gap: 3px; }
.tkx-card.over { outline: 1px dashed var(--acc, #f0a020); }
.tkx-chead { display: grid; grid-template-columns: 18px 1fr auto auto; gap: 6px; align-items: center; padding: 2px 4px; }
.tkx-chead input { font-weight: 700; }
.tkx-raw { width: 100%; min-height: 140px; font: 13px var(--mono, monospace); resize: vertical; }
.tkx-err { color: #ff6a66; font-size: 11px; padding: 0 6px 4px; }
.tkx-help { color: var(--dim, #888); font-size: 10.5px; padding: 0 6px 4px; }
`;
let styled = false;
let seq = 0;
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const VIEWS = { list: 'Lista', groups: 'Grupy', raw: 'Tekst' };

/**
 * @param host element, w którym powstaje edytor
 * @param o { value, onChange(text), catColors, catBg, catFg, label (tekst albo funkcja), rules: {maxItems, maxChars, cats, upper, readonly}, onHover(i|null) }
 * @returns { setValue(text), setSqueeze(arr|null), setOptions(o), focus(), el }
 */
export function mountTickerEditor(host, o = {}) {
  if (!styled && typeof document !== 'undefined') { const st = document.createElement('style'); st.textContent = STYLE; document.head.appendChild(st); styled = true; }
  const id = 'tkx' + (++seq);
  let items = parseTickerItems(o.value);
  if (!items.length) items = [{ cat: '', color: '', text: '' }];
  let last = serializeTickerItems(items);
  let sq = null, sqMin = 0;
  let view = 'list';
  try { view = localStorage.getItem('vizrs.tkView') || 'list'; } catch { /* brak localStorage */ }
  if (!VIEWS[view]) view = 'list';
  const root = document.createElement('div');
  root.className = 'tkx';
  host.appendChild(root);
  const R = () => o.rules || {};
  const colors = () => parseCatColors(o.catColors);
  const maxChars = () => Math.min(TICKER_LIMITS.chars, R().maxChars || Infinity);
  const maxItems = () => Math.min(TICKER_LIMITS.items, R().maxItems || Infinity);
  const allCats = () => [...new Set([...(R().cats || []), ...Object.keys(colors()), ...items.map((x) => x.cat).filter(Boolean)])];

  const emit = () => {
    const v = serializeTickerItems(items);
    if (v === last) return;
    last = v;
    o.onChange?.(v);
    status();
  };
  const up = (s) => String(s ?? '').trim().toUpperCase();
  const label = () => String((typeof o.label === 'function' ? o.label() : o.label) || '');
  const sceneColor = (cat) => catColorOf({ cat }, colors(), o.catBg || '#c8102e', label());
  const pageColors = () => pageCatColors(items, label());
  const dot = (x) => (x.cat || x.color || label() ? catColorOf(x, { ...colors(), ...pageColors() }, o.catBg || '#c8102e', label()) : 'transparent');
  /** Pole koloru. Bez kategorii i etykiety nie ma plakietki: kliknięcie prowadzi do wpisania kategorii. */
  const swatch = (x, attr) => {
    const off = R().readonly, bare = !(x.cat || x.color || label());
    const mix = x.mix ? ' mix' : '';
    const tip = off ? 'Kolor kategorii' : bare ? 'Brak plakietki – wpisz najpierw kategorię' : x.mix ? 'Wiadomości tej kategorii mają różne kolory – wybór ujednolici je' : x.cat ? `Kolor kategorii ${x.cat}` : `Kolor etykiety ${label()}`.trim();
    return `<button type="button" class="tkx-dot${mix}${bare ? ' bare' : ''}" data-a="color" ${attr} style="background:${esc(dot(x))}" title="${esc(tip)}" ${off ? 'disabled' : ''}></button>`;
  };
  const paintDots = () => root.querySelectorAll('.tkx-dot').forEach((d) => {
    const x = d.dataset.i != null ? items[+d.dataset.i] : groups().find((g) => g.cat === d.dataset.cat);
    if (x) d.style.background = dot(x);
  });
  /** Kolor należy do kategorii: obowiązuje wszystkie jej wiadomości; kolor równy scenowemu nie jest nadpisaniem. */
  function setCatColor(cat, c) {
    const k = up(cat), base = sceneColor(cat);
    const color = !c || (normHex(c) && normHex(c) === normHex(base)) ? '' : c;
    for (const x of items) if (up(x.cat) === k) x.color = color;
    paintDots(); emit();
  }
  function pickColor(btn, cat) {
    if (!cat && !label() && !items.some((x) => !x.cat && x.color)) {
      const row = btn.closest('[data-row]'), card = btn.closest('.tkx-card');
      (row?.querySelector('.tkx-cat') || card?.querySelector('.tkx-cname'))?.focus();
      return;
    }
    const color = pageColors()[up(cat || label())] || items.find((x) => up(x.cat) === up(cat))?.color || '';
    openColorPicker(btn, {
      value: color, base: sceneColor(cat), label: cat || label() || 'kategoria', fg: o.catFg,
      palette: [...new Set(Object.values({ ...colors(), ...pageColors() }))],
      onInput: (c) => { if (root.isConnected) setCatColor(cat, c); },
    });
  }
  /** Po zmianie nazwy kategorii: kolor wpisany > kolor tej kategorii na liście > mapa sceny > dotychczasowy. */
  function settleCat(list, typed, was) {
    const k = up(list[0]?.cat);
    if (typed) { for (const x of items) if (up(x.cat) === k) x.color = typed; return; }
    if (k === up(was)) return;
    const rest = items.filter((x) => up(x.cat) === k && !list.includes(x));
    const color = !k ? '' : rest.length ? rest.find((x) => x.color)?.color || '' : colors()[k] ? '' : list[0].color;
    for (const x of list) x.color = color;
  }
  const catCtl = (x, i) => {
    const ro = R().readonly ? 'disabled' : '';
    if (R().cats?.length) return `<select class="tkx-cat" data-i="${i}" data-was="${esc(x.cat)}" ${ro}><option value=""></option>${R().cats.map((c) => `<option ${c.toUpperCase() === x.cat.toUpperCase() ? 'selected' : ''}>${esc(c)}</option>`).join('')}</select>`;
    return `<input class="tkx-cat" data-i="${i}" data-was="${esc(x.cat)}" list="${id}-cats" value="${esc(x.cat)}" placeholder="kategoria" ${ro}>`;
  };
  const meta = (x, i) => {
    const n = Array.from(x.text).length, mc = maxChars();
    const s = sq?.[i];
    const over = s != null && s < sqMin - 1e-3;
    return `<span class="tkx-meta">${s != null && s < 0.999 ? `<span class="${over ? 'bad' : 'warn'}" title="${over ? `nie mieści się: potrzebne ściśnięcie poniżej granicy ${Math.round(sqMin * 100)}% – wiadomość zostanie ucięta` : 'ściśnięcie w grafice'}">${over ? '⚠ ' : ''}${Math.round(s * 100)}%</span>` : ''}${isFinite(mc) && R().maxChars ? `<span class="${n >= mc ? 'bad' : n >= mc * 0.85 ? 'warn' : ''}">${n}/${mc}</span>` : ''}</span>`;
  };
  const txt = (x, i) => `<input class="tkx-txt" data-i="${i}" value="${esc(x.text)}" maxlength="${isFinite(maxChars()) ? maxChars() : ''}" placeholder="treść wiadomości" spellcheck="true" lang="pl" ${R().readonly ? 'readonly' : ''}>`;
  const btns = (i, g) => R().readonly ? '<span></span><span></span>' : `${g ? '' : `<button data-a="up" data-i="${i}" title="W górę (Ctrl+↑)">↑</button><button data-a="down" data-i="${i}" title="W dół (Ctrl+↓)">↓</button>`}<button data-a="del" data-i="${i}" title="Usuń wiadomość">✕</button>`;
  const row = (x, i) => `<div class="tkx-row" data-row="${i}" draggable="${!R().readonly}"><span class="tkx-n" title="przeciągnij, aby przenieść">${i + 1}</span>${swatch(x, `data-i="${i}"`)}${catCtl(x, i)}${txt(x, i)}${meta(x, i)}${btns(i)}</div>`;
  const grow = (x, i) => `<div class="tkx-row g" data-row="${i}" draggable="${!R().readonly}"><span class="tkx-n" title="przeciągnij do innej grupy">${i + 1}</span>${txt(x, i)}${meta(x, i)}<span></span>${btns(i, true)}</div>`;

  function groups() {
    const map = new Map();
    items.forEach((x, i) => {
      const k = x.cat.toUpperCase();
      if (!map.has(k)) map.set(k, { cat: x.cat, color: '', mix: false, idx: [] });
      const g = map.get(k);
      g.idx.push(i);
      if (x.color && !g.color) g.color = x.color;
      else if (x.color && x.color.toLowerCase() !== g.color.toLowerCase()) g.mix = true;
    });
    return [...map.values()];
  }
  /** Ułożenie listy w kolejności grup (widok grup zmienia kolejność wiadomości na antenie). */
  function regroup() { items = groups().flatMap((g) => g.idx.map((i) => items[i])); }

  function render(focus) {
    const ro = R().readonly;
    const full = items.length >= maxItems();
    let body = '';
    if (view === 'list') {
      body = items.map(row).join('') + (ro ? '' : `<button class="tkx-add" data-a="add" ${full ? 'disabled' : ''}>＋ wiadomość</button>`);
    } else if (view === 'groups') {
      body = groups().map((g) => `<div class="tkx-card" data-cat="${esc(g.cat)}"><div class="tkx-chead">${swatch(g, `data-cat="${esc(g.cat)}"`)}${R().cats?.length
        ? `<select class="tkx-cname" data-cat="${esc(g.cat)}" ${ro ? 'disabled' : ''}><option value=""></option>${R().cats.map((c) => `<option ${c.toUpperCase() === g.cat.toUpperCase() ? 'selected' : ''}>${esc(c)}</option>`).join('')}</select>`
        : `<input class="tkx-cname" data-cat="${esc(g.cat)}" list="${id}-cats" value="${esc(g.cat)}" placeholder="(bez kategorii)" ${ro ? 'disabled' : ''}>`}<span class="tkx-meta">${g.idx.length}</span>${ro ? '' : `<button data-a="gdel" data-cat="${esc(g.cat)}" title="Usuń grupę z wiadomościami">✕</button>`}</div>
        ${g.idx.map((i) => grow(items[i], i)).join('')}${ro ? '' : `<button class="tkx-add" data-a="gadd" data-cat="${esc(g.cat)}" data-color="${esc(g.color)}" ${full ? 'disabled' : ''}>＋ wiadomość</button>`}</div>`).join('') +
        (ro ? '' : `<button class="tkx-add" data-a="newgroup" ${full ? 'disabled' : ''}>＋ grupa</button>`);
    } else {
      body = `<textarea class="tkx-raw" spellcheck="true" lang="pl" ${ro ? 'readonly' : ''}>${esc(serializeTickerItems(items))}</textarea><div class="tkx-help">Jedna wiadomość w wierszu. Kategoria: <code>KAT|treść</code>, z kolorem: <code>KAT#c00|treść</code> (kolor obowiązuje całą kategorię).</div>`;
    }
    root.innerHTML = `<div class="tkx-bar">${Object.entries(VIEWS).map(([k, l]) => `<button data-view="${k}" class="${k === view ? 'on' : ''}">${l}</button>`).join('')}<span class="tkx-info"></span></div>
      <div class="tkx-body">${body}</div><div class="tkx-err"></div><datalist id="${id}-cats">${allCats().map((c) => `<option value="${esc(c)}">`).join('')}</datalist>`;
    status();
    if (focus != null) {
      const el = root.querySelector(`.tkx-txt[data-i="${focus.i}"]`);
      if (el) { el.focus(); const p = focus.end ? el.value.length : (focus.pos ?? el.value.length); el.setSelectionRange(p, p); }
    }
  }
  function status() {
    const n = items.filter((x) => x.text.trim()).length;
    const info = root.querySelector('.tkx-info'); if (info) info.textContent = `${n} ${n === 1 ? 'wiadomość' : 'wiadomości'}`;
    const err = root.querySelector('.tkx-err');
    if (err) err.textContent = checkTickerItems(items.filter((x) => x.text.trim()), R()).map((e) => e.msg).join(' · ');
  }
  const newItem = (cat = '', color = '') => ({ cat, color, text: '' });
  const insert = (at, x) => { if (items.length >= maxItems()) return false; items.splice(at, 0, x); return true; };
  const move = (i, j) => { if (j < 0 || j >= items.length || i === j) return; const [x] = items.splice(i, 1); items.splice(j, 0, x); };
  const catParse = (v) => { const it = parseTickerItems(`${String(v).trim().replace(/[|\n]/g, ' ')}|x`)[0]; return { cat: it?.cat || '', color: it?.color || '' }; };

  root.addEventListener('click', (e) => {
    const vb = e.target.closest('[data-view]');
    if (vb) { view = vb.dataset.view; try { localStorage.setItem('vizrs.tkView', view); } catch { /* ignoruj */ } render(); return; }
    const b = e.target.closest('[data-a]'); if (!b) return;
    const i = +b.dataset.i, a = b.dataset.a;
    if (a === 'color') { pickColor(b, b.dataset.i != null ? items[i].cat : b.dataset.cat); return; }
    if (a === 'add') { if (insert(items.length, newItem())) render({ i: items.length - 1 }); return; }
    if (a === 'up') { move(i, i - 1); emit(); render(); return; }
    if (a === 'down') { move(i, i + 1); emit(); render(); return; }
    if (a === 'del') { items.splice(i, 1); if (!items.length) items.push(newItem()); emit(); render(); return; }
    if (a === 'gadd') {
      const g = groups().find((x) => x.cat === b.dataset.cat);
      const at = g ? g.idx[g.idx.length - 1] + 1 : items.length;
      if (insert(at, newItem(b.dataset.cat, b.dataset.color))) { regroup(); render({ i: at }); }
      return;
    }
    if (a === 'gdel') {
      const g = groups().find((x) => x.cat === b.dataset.cat);
      if (g && (g.idx.every((k) => !items[k].text.trim()) || confirm(`Usunąć grupę „${g.cat || 'bez kategorii'}” (${g.idx.length} wiad.)?`))) {
        items = items.filter((x) => x.cat.toUpperCase() !== b.dataset.cat.toUpperCase());
        if (!items.length) items.push(newItem());
        emit(); render();
      }
      return;
    }
    if (a === 'newgroup') {
      const name = prompt('Nazwa kategorii (np. PILNE, SPORT; kolor: SPORT#1f8a4c)', ''); if (name === null) return;
      const c = catParse(name);
      if (insert(items.length, newItem(c.cat, c.color))) { regroup(); render({ i: items.length - 1 }); }
    }
  });
  root.addEventListener('input', (e) => {
    const el = e.target;
    if (el.classList.contains('tkx-raw')) {
      items = parseTickerItems(el.value);
      if (!items.length) items = [newItem()];
      const v = serializeTickerItems(items);
      if (v !== last) { last = v; o.onChange?.(v); }
      status(); return;
    }
    const i = +el.dataset.i;
    if (el.classList.contains('tkx-txt')) {
      if (R().upper && el.value !== el.value.toUpperCase()) { const p = el.selectionStart; el.value = el.value.toUpperCase(); el.setSelectionRange(p, p); }
      items[i].text = el.value.replace(/\n/g, ' ');
      const m = el.parentElement.querySelector('.tkx-meta'); if (m) m.outerHTML = meta(items[i], i);
      emit(); return;
    }
    if (el.classList.contains('tkx-cat')) {
      const c = catParse(el.value);
      items[i].cat = c.cat; if (c.color) items[i].color = c.color;
      paintDots(); emit(); return;
    }
  });
  root.addEventListener('change', (e) => {
    const el = e.target;
    if (el.classList.contains('tkx-cname')) {
      // zmiana nazwy grupy = zmiana kategorii wszystkich jej wiadomości
      const from = el.dataset.cat.toUpperCase(), c = catParse(el.value), list = items.filter((x) => x.cat.toUpperCase() === from);
      for (const x of list) x.cat = c.cat;
      settleCat(list, c.color, el.dataset.cat);
      regroup(); emit(); render(); return;
    }
    if (el.classList.contains('tkx-cat') && view === 'list') {
      settleCat([items[+el.dataset.i]], catParse(el.value).color, el.dataset.was);
      emit(); render();
    }
  });
  root.addEventListener('keydown', (e) => {
    const el = e.target;
    const isTxt = el.classList.contains('tkx-txt');
    if (!isTxt && !el.classList.contains('tkx-cat')) return;
    const i = +el.dataset.i;
    const grouped = view === 'groups';
    if ((e.ctrlKey || e.metaKey) && (e.key === 'ArrowUp' || e.key === 'ArrowDown') && !grouped) {
      e.preventDefault(); e.stopPropagation();
      const j = i + (e.key === 'ArrowUp' ? -1 : 1);
      if (j < 0 || j >= items.length) return;
      move(i, j); emit(); render({ i: j, pos: isTxt ? el.selectionStart : undefined }); return;
    }
    if (!isTxt) return;
    if (e.key === 'Enter' && !e.ctrlKey && !e.metaKey) {
      e.preventDefault(); e.stopPropagation();
      if (R().readonly) return;
      // Enter dzieli wiadomość w miejscu kursora; nowa dostaje kategorię bieżącej
      const p = el.selectionStart, rest = items[i].text.slice(p);
      if (!insert(i + 1, newItem(items[i].cat, items[i].color))) return;
      items[i].text = items[i].text.slice(0, p).trimEnd(); items[i + 1].text = rest.trimStart();
      emit(); render({ i: i + 1, pos: 0 }); return;
    }
    if (!e.ctrlKey && !e.altKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
      const all = [...root.querySelectorAll('.tkx-txt')], k = all.indexOf(el) + (e.key === 'ArrowUp' ? -1 : 1);
      if (all[k]) { e.preventDefault(); e.stopPropagation(); all[k].focus(); }
      return;
    }
    if (e.key === 'Backspace' && !el.value && !items[i].cat && items.length > 1 && !R().readonly) {
      e.preventDefault(); e.stopPropagation();
      items.splice(i, 1); emit(); render({ i: Math.max(0, i - 1), end: true });
    }
  });
  root.addEventListener('paste', (e) => {
    const el = e.target;
    if (!el.classList.contains('tkx-txt') || R().readonly) return;
    const t = e.clipboardData?.getData('text') || '';
    if (!t.includes('\n')) return;
    e.preventDefault();
    // wklejenie wielu wierszy = wiele wiadomości (składnia KAT|treść działa)
    const i = +el.dataset.i, add = parseTickerItems(t);
    if (!add.length) return;
    const base = items[i];
    if (!base.text.trim()) { items.splice(i, 1, ...add.slice(0, Math.max(1, maxItems() - items.length + 1))); }
    else items.splice(i + 1, 0, ...add.slice(0, Math.max(0, maxItems() - items.length)));
    emit(); render({ i: i + add.length - (base.text.trim() ? 0 : 1), end: true });
  });
  // przeciąganie wiadomości (lista: kolejność; grupy: także zmiana kategorii)
  let dragI = null;
  root.addEventListener('dragstart', (e) => {
    const r = e.target.closest?.('[data-row]'); if (!r || e.target.closest('input,select,button')) return;
    dragI = +r.dataset.row; r.classList.add('drag'); e.dataTransfer.effectAllowed = 'move'; e.dataTransfer.setData('text/plain', String(dragI));
  });
  root.addEventListener('dragend', () => { dragI = null; root.querySelectorAll('.drag,.over').forEach((x) => x.classList.remove('drag', 'over')); });
  root.addEventListener('dragover', (e) => {
    if (dragI == null) return;
    const t = e.target.closest('[data-row]') || e.target.closest('.tkx-card'); if (!t) return;
    e.preventDefault();
    root.querySelectorAll('.over').forEach((x) => x.classList.remove('over'));
    t.classList.add('over');
  });
  root.addEventListener('drop', (e) => {
    if (dragI == null) return;
    e.preventDefault();
    const r = e.target.closest('[data-row]'), card = e.target.closest('.tkx-card');
    const x = items[dragI];
    if (card) { const c = items.find((y) => y.cat === card.dataset.cat) || { cat: card.dataset.cat, color: '' }; x.cat = c.cat; x.color = c.color; }
    let j = r ? +r.dataset.row : items.length - 1;
    if (!r && card) { const g = groups().find((g2) => g2.cat === card.dataset.cat); j = g ? g.idx[g.idx.length - 1] : j; }
    move(dragI, j);
    if (view === 'groups') regroup();
    dragI = null; emit(); render();
  });
  // podgląd wiadomości w grafice po najechaniu na wiersz
  // indeks w grafice liczy tylko wiadomości z treścią
  root.addEventListener('mouseover', (e) => {
    const r = e.target.closest('[data-row]'); if (!r) return;
    const i = +r.dataset.row;
    o.onHover?.(items[i]?.text.trim() ? items.slice(0, i).filter((x) => x.text.trim()).length : null);
  });
  root.addEventListener('mouseleave', () => o.onHover?.(null));

  render();
  return {
    el: root,
    setValue(v) {
      v = String(v ?? '');
      if (v === last) return;
      items = parseTickerItems(v); if (!items.length) items = [newItem()];
      last = serializeTickerItems(items);
      if (!root.contains(document.activeElement)) render(); else status();
    },
    setSqueeze(arr) {
      // indeksy pomiaru odnoszą się do wiadomości niepustych
      const map = [];
      let k = 0;
      items.forEach((x, i) => { if (x.text.trim()) map[i] = arr?.[k++]; });
      sq = arr ? map : null;
      sqMin = arr?.min || 0;
      root.querySelectorAll('[data-row]').forEach((r) => { const i = +r.dataset.row; const m = r.querySelector('.tkx-meta'); if (m) m.outerHTML = meta(items[i], i); });
    },
    setOptions(n) { Object.assign(o, n); render(); },
    focus() { root.querySelector('.tkx-txt')?.focus(); },
  };
}
