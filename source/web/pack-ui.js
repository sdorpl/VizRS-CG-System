// VizRS – okna paczki `.cgpack` wspólne dla Trio i VizRS CG Artist (kopia przez tools/sync-cg.sh):
//   planDialog(plan, { title }) → Promise<{ fixRefs } | null>   – plan importu (`cgpack.planImport`): czynności scen i stron,
//       przesunięcie numerów, powiązania dock; zmienia `plan` na miejscu, „Importuj” dopiero po decyzji dla każdej sceny
//   pickPages({ pages, scenes, current, assetsOf }) → Promise<string[] | null>  – wybór stron do eksportu (id stron)
// Wygląd: zmienne rdzenia vizrs-ui.css (--panel, --line, --acc…), własne klasy `pk-*` (bez zależności od ui.css strony).
import { planPages, undecided, FONT_RE } from './cgpack.js';
import { esc, html, orEmpty, setHtml } from './dom.js';

const CSS = `
.pk-back{position:fixed;inset:0;background:rgba(0,0,0,.6);z-index:3000;display:flex;align-items:center;justify-content:center}
.pk-box{width:min(900px,96vw);max-height:90vh;display:flex;flex-direction:column;background:var(--panel,#16181d);border:1px solid var(--line,#2a2e36);border-radius:8px;box-shadow:0 12px 40px rgba(0,0,0,.6)}
.pk-head{padding:14px 16px 6px}.pk-head b{font-size:15px}
.pk-body{padding:4px 16px;overflow:auto;flex:1}
.pk-foot{display:flex;gap:6px;align-items:center;padding:10px 16px 14px;border-top:1px solid var(--line,#2a2e36)}
.pk-foot .pk-grow,.pk-row .pk-grow{flex:1}
.pk-dim{color:var(--dim,#8a8f98);font-size:12px}
.pk-sec{margin:12px 0 4px;font-weight:600;font-size:13px;display:flex;gap:8px;align-items:center;flex-wrap:wrap}
.pk-t{width:100%;border-collapse:collapse;font-size:13px}
.pk-t td{padding:4px 6px;border-bottom:1px solid var(--line,#2a2e36);vertical-align:middle}
.pk-t td.pk-n{white-space:nowrap;font-variant-numeric:tabular-nums}
.pk-t select{max-width:100%}
.pk-warn{color:var(--warn,#e8b04b)}
.pk-need select{outline:2px solid var(--warn,#e8b04b);outline-offset:1px}
.pk-list{max-height:46vh;overflow:auto;border:1px solid var(--line,#2a2e36);border-radius:6px;padding:4px 8px}
.pk-list label{display:flex;gap:8px;align-items:center;padding:3px 0}
.pk-row{display:flex;gap:6px;align-items:center;flex-wrap:wrap;margin:6px 0}
.pk-off{width:7em}
@media (max-width:760px){.pk-box{width:100vw;max-height:100vh;height:100%;border-radius:0}.pk-t,.pk-t tbody,.pk-t tr,.pk-t td{display:block}.pk-t tr{padding:6px 0;border-bottom:1px solid var(--line,#2a2e36)}.pk-t td{border:0;padding:2px 0}.pk-t select{width:100%;min-height:40px}}
`;
function style() {
  if (document.getElementById('pk-css')) return;
  const st = document.createElement('style'); st.id = 'pk-css'; st.textContent = CSS; document.head.appendChild(st);
}

/** Szkielet okna: zwraca { d, body, foot, close(v) } i obietnicę wyniku. */
function shell(title, sub) {
  style();
  let done;
  const result = new Promise((r) => (done = r));
  const back = document.activeElement;
  const d = document.createElement('div');
  d.className = 'pk-back';
  setHtml(d, html`<div class="pk-box" role="dialog" aria-modal="true"><div class="pk-head"><b>${title}</b>${sub ? html`<div class="pk-dim" style="margin-top:4px">${sub}</div>` : ''}</div>
    <div class="pk-body"></div><div class="pk-foot"></div></div>`);
  const keys = (e) => { if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close(null); } else if (d.contains(e.target)) e.stopPropagation(); };
  const close = (v) => { d.remove(); removeEventListener('keydown', keys, true); back?.focus?.(); done(v); };
  addEventListener('keydown', keys, true); // modalne: skróty strony (F-klawisze Trio) nie działają pod oknem
  document.body.appendChild(d);
  return { d, body: d.querySelector('.pk-body'), foot: d.querySelector('.pk-foot'), close, result };
}

const SCENE_STATUS = {
  new: 'nowa',
  same: 'taka sama jest już w show',
  changed: 'jest już w show w innej wersji',
  'name-taken': 'w show jest inna scena o tej nazwie',
};
const sceneChoice = (e, a) => ({
  add: 'Dodaj',
  skip: e.target ? 'Pomiń (strony z paczki wskażą istniejącą)' : 'Pomiń',
  copy: 'Dodaj jako kopię („… (2)”)',
  update: e.status === 'name-taken' ? `Zastąp „${e.targetName}” (jej strony zostają)` : 'Zaktualizuj istniejącą (strony zostają)',
}[a]);
const PAGE_STATUS = { new: 'nowa', 'number-taken': 'numer zajęty', exists: 'jest już w show', 'no-scene': 'bez sceny' };
const pageChoice = { add: 'Dodaj', renumber: 'Dodaj z nowym numerem', skip: 'Pomiń', overwrite: 'Nadpisz wartościami z paczki' };

/**
 * Okno planu importu. Zmienia czynności w `plan` (sceny, strony, przesunięcie) i zwraca { fixRefs } albo null (Anuluj).
 */
export function planDialog(plan, { title = 'Import paczki .cgpack' } = {}) {
  const pages = () => plan.pages;
  const nFiles = plan.files.length, nNew = plan.files.filter((f) => f.status === 'new' || f.status === 'renamed').length;
  const sub = `${plan.scenes.length} scen${plan.withPages ? `, ${plan._pk.pages.length} stron` : ''}, ${nFiles} plików – nic nie zostanie zapisane przed „Importuj”. Bieżący show i antena zostają.`;
  const ui = shell(title, sub);
  let fixRefs = true;
  const render = () => {
    const sc = plan.scenes, conflicts = sc.filter((e) => e.choices.length > 2 || !e.action);
    let h = html`<div class="pk-sec">Sceny${conflicts.length ? html`<span class="pk-grow"></span><span class="pk-dim">wszystkie konflikty:</span>
      <button data-all="update">Zaktualizuj / zastąp</button><button data-all="copy">Kopia</button><button data-all="skip">Pomiń</button>` : ''}</div>
      <table class="pk-t">${sc.map((e, i) => html`<tr class="${e.action ? '' : 'pk-need'}"><td>${esc(e.name)}</td>
        <td class="${e.status === 'changed' || e.status === 'name-taken' ? 'pk-warn' : 'pk-dim'}">${esc(SCENE_STATUS[e.status])}${e.status === 'changed' && e.targetName !== e.name ? ` (w show: „${e.targetName}”)` : ''}</td>
        <td><select data-sc="${i}">${e.action ? '' : html`<option value="">— wybierz —</option>`}${e.choices.map((a) => html`<option value="${a}" ${a === e.action ? 'selected' : ''}>${esc(sceneChoice(e, a))}</option>`)}</select></td></tr>`)}</table>`;
    if (plan.withPages && plan._pk.pages.length) {
      h = html`${h}<div class="pk-sec">Strony<span class="pk-grow"></span><label class="pk-dim">przesuń numery o <input class="pk-off" type="number" step="1" value="${plan.offset}" data-off></label></div>
        <table class="pk-t">${pages().map((r, i) => html`<tr><td class="pk-n">${r.number ?? '?'}${(r.action === 'add' || r.action === 'renumber' || r.action === 'overwrite') && r.to !== r.number ? html` → <b>${r.to}</b>` : ''}</td>
          <td>${esc(r.name)}</td><td class="${r.status === 'new' ? 'pk-dim' : 'pk-warn'}">${esc(PAGE_STATUS[r.status])}${r.reason ? ` – ${r.reason}` : ''}</td>
          <td>${r.choices.length > 1 ? html`<select data-pg="${i}">${r.choices.map((a) => html`<option value="${a}" ${a === r.action ? 'selected' : ''}>${esc(pageChoice[a])}</option>`)}</select>` : html`<span class="pk-dim">${esc(pageChoice[r.action])}</span>`}</td></tr>`)}</table>
        <div class="pk-dim" style="margin-top:4px">Playlisty, rundowny i makra nie jadą w paczce.</div>`;
    } else if (!plan.withPages && plan._pk.pages.length) {
      h = html`${h}<div class="pk-sec">Strony</div><div class="pk-dim">${plan._pk.pages.length} stron Trio zostanie pominiętych – praca lokalna tworzy tylko sceny; strony zaimportuje Trio z tej samej paczki.</div>`;
    }
    if (plan.refs.length) {
      h = html`${h}<div class="pk-sec pk-warn">Powiązania (dock), które po imporcie wskazywałyby co innego</div>
        <ul style="margin:2px 0;padding-left:18px">${plan.refs.map((r) => html`<li>„${esc(r.scene)}”: ${esc(r.from)} → ${esc(r.to)}</li>`)}</ul>
        <label><input type="checkbox" data-fix ${fixRefs ? 'checked' : ''}> popraw je w scenach z paczki</label>`;
    }
    const renamed = plan.files.filter((f) => f.name !== f.from), had = plan.files.filter((f) => f.status === 'same' || f.status === 'reused');
    h = html`${h}<div class="pk-sec">Pliki</div><div class="pk-dim">${nNew} nowych${had.length ? `, ${had.length} już jest w zasobach` : ''}${plan.files.some((f) => FONT_RE.test(f.name)) ? ` (czcionki: ${plan.files.filter((f) => FONT_RE.test(f.name)).map((f) => f.name).join(', ')})` : ''}.
      ${renamed.length ? `Pod inną nazwą: ${renamed.map((f) => `${f.from} → ${f.name}`).join(', ')}.` : ''} Istniejące pliki nie są nadpisywane.</div>`;
    if (plan.legacy) h = html`${h}<div class="pk-warn" style="margin-top:8px">Starszy format .vizrs.json – bez grafik i czcionek.</div>`;
    setHtml(ui.body, h);
    const left = undecided(plan).length;
    setHtml(ui.foot, html`<span class="pk-dim pk-grow">${left ? html`<span class="pk-warn">Wybierz czynność dla ${left} scen(y) oznaczonych ramką.</span>` : 'Gotowe do importu.'}</span>
      <button data-x="no">Anuluj</button><button data-x="go" class="acc" ${left ? 'disabled' : ''}>⇧ Importuj</button>`);
  };
  render();
  ui.d.addEventListener('change', (e) => {
    const t = e.target;
    if (t.dataset.sc !== undefined) { plan.scenes[+t.dataset.sc].action = t.value || null; planPages(plan); }
    else if (t.dataset.pg !== undefined) { pages()[+t.dataset.pg].action = t.value; planPages(plan); }
    else if (t.dataset.off !== undefined) planPages(plan, t.value);
    else if (t.dataset.fix !== undefined) fixRefs = t.checked;
    render();
  });
  ui.d.addEventListener('click', (e) => {
    const a = e.target.closest('[data-all]')?.dataset.all;
    if (a) {
      for (const s of plan.scenes) if (s.choices.length > 2 || !s.action) if (s.choices.includes(a)) s.action = a;
      planPages(plan); render(); return;
    }
    const x = e.target.closest('[data-x]')?.dataset.x;
    if (x === 'no') ui.close(null);
    if (x === 'go' && !undecided(plan).length) ui.close({ fixRefs: fixRefs && plan.refs.length > 0 });
  });
  return ui.result;
}

/**
 * Wybór stron do eksportu paczki. `pages` – strony show, `scenes` – sceny show, `current` – id zaznaczonej strony,
 * `assetsOf(sceny, strony)` → { images, fonts: [nazwy] } do podsumowania. Zwraca id wybranych stron albo null.
 */
export function pickPages({ pages, scenes, current = null, assetsOf = null, title = 'Eksport stron do paczki .cgpack' }) {
  const sub = 'Strony razem z ich scenami, grafikami i czcionkami – do importu w Trio na innym serwerze albo w innym show.';
  const ui = shell(title, sub);
  const sceneName = new Map(scenes.map((s) => [s.id, s.name]));
  const list = [...pages].sort((a, b) => a.number - b.number);
  const chosen = new Set(current ? [current] : []);
  setHtml(ui.body, html`<div class="pk-row"><input type="search" placeholder="szukaj: numer, nazwa, scena" data-q class="pk-grow">
      <button data-p="all">zaznacz widoczne</button><button data-p="none">żadna</button></div>
    <div class="pk-list"></div><div class="pk-dim" data-sum style="margin:8px 0"></div>`);
  const box = ui.body.querySelector('.pk-list');
  const visible = () => {
    const q = ui.body.querySelector('[data-q]').value.trim().toLowerCase();
    return list.filter((p) => !q || `${p.number} ${p.name || ''} ${sceneName.get(p.scene) || ''}`.toLowerCase().includes(q));
  };
  const draw = () => {
    setHtml(box, orEmpty(visible().map((p) => html`<label><input type="checkbox" value="${esc(p.id)}" ${chosen.has(p.id) ? 'checked' : ''}>
      <span class="pk-n" style="min-width:4.5em">${p.number}</span><span class="pk-grow">${esc(p.name || '')}</span><span class="pk-dim">${esc(sceneName.get(p.scene) || '?')}</span></label>`), html`<div class="pk-dim">Brak stron.</div>`));
    sum();
  };
  const sum = () => {
    const ps = list.filter((p) => chosen.has(p.id)), sc = new Set(ps.map((p) => p.scene));
    const a = assetsOf ? assetsOf(scenes.filter((s) => sc.has(s.id)), ps) : null;
    ui.body.querySelector('[data-sum]').textContent = `${ps.length} stron · ${sc.size} scen${a ? ` · ${a.images.length} grafik · ${a.fonts.length} czcionek${a.fonts.length ? ': ' + a.fonts.join(', ') : ''}` : ''}`;
    ui.foot.querySelector('[data-x="go"]').disabled = !ps.length;
  };
  setHtml(ui.foot, html`<span class="pk-grow"></span><button data-x="no">Anuluj</button><button data-x="go" class="acc">⇩ Zapisz .cgpack</button>`);
  draw();
  ui.body.querySelector('[data-q]').addEventListener('input', draw);
  ui.d.addEventListener('change', (e) => { if (e.target.type === 'checkbox') { e.target.checked ? chosen.add(e.target.value) : chosen.delete(e.target.value); sum(); } });
  ui.d.addEventListener('click', (e) => {
    const p = e.target.closest('[data-p]')?.dataset.p;
    if (p) { for (const x of visible()) p === 'all' ? chosen.add(x.id) : chosen.delete(x.id); if (p === 'none') chosen.clear(); draw(); return; }
    const x = e.target.closest('[data-x]')?.dataset.x;
    if (x === 'no') ui.close(null);
    if (x === 'go' && chosen.size) ui.close(list.filter((q) => chosen.has(q.id)).map((q) => q.id));
  });
  return ui.result;
}
