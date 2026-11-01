// Most stron Artist/Trio z oknem natywnym vizrs-shell (w przeglądarce nic nie robi).
// Strona zgłasza stan (tytuł, zmiany, cofanie, kopia robocza), powłoka wysyła komendy z menu i pliki.
const shell = window.__vizrsShell;
const post = (o) => { try { window.ipc.postMessage('app:' + JSON.stringify(o)); } catch { /* okno zamykane */ } };

// pytanie przy zamykaniu okna: strona zna stan, więc i zapis należy do niej
function askClose(what, save, discard) {
  if (document.querySelector('#nativeClose')) return;
  const d = document.createElement('div');
  d.id = 'nativeClose';
  d.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,.6);z-index:3000;display:flex;align-items:center;justify-content:center';
  d.innerHTML = `<div style="width:min(460px,94vw);background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:16px">
    <b style="font-size:15px">Niezapisane zmiany</b><div id="ncWhat" style="margin:10px 0 14px"></div>
    <div class="row" style="gap:6px;justify-content:flex-end"><button data-nc="drop" class="danger">Nie zapisuj</button><button data-nc="stay">Anuluj</button><button data-nc="save" class="acc">Zapisz</button></div></div>`;
  d.querySelector('#ncWhat').textContent = what;
  const done = (msg) => { d.remove(); removeEventListener('keydown', esc, true); post({ t: msg }); };
  const esc = (e) => { if (e.key === 'Escape') { e.preventDefault(); done('stay'); } };
  addEventListener('keydown', esc, true);
  d.addEventListener('click', async (e) => {
    const b = e.target.dataset.nc; if (!b) return;
    if (b === 'stay') return done('stay');
    if (b === 'drop') { discard(); return done('close'); }
    for (const x of d.querySelectorAll('button')) x.disabled = true;
    if (await save()) done('close'); else done('stay');
  });
  document.body.appendChild(d);
  d.querySelector('.acc').focus();
}

// state() → { title, dirty, undo, redo, scene }; draft() → { id, text } kopii roboczej albo null
export function nativeBridge({ state, save, whatChanged, draft, commands = {} }) {
  if (!shell) return { sync() {}, clearDraft() {} };
  let last = '', draftId = null, draftText = null, parts = null;
  const clearDraft = () => { if (draftId) post({ t: 'draft', id: draftId, data: null }); draftId = null; draftText = null; };
  const sync = () => {
    const s = state(); if (!s) return;
    const j = JSON.stringify(s);
    if (j !== last) { last = j; post({ t: 'state', ...s }); }
  };
  window.__vizrsCmd = (name, arg) => {
    if (name === 'close') askClose(whatChanged(), save, clearDraft);
    else commands[name]?.(arg);
  };
  window.__vizrsFile = {
    begin(name) { parts = { name, list: [] }; },
    chunk(b64) { parts.list.push(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))); },
    end() { const f = new File(parts.list, parts.name); parts = null; commands.file?.(f); },
  };
  setInterval(sync, 300);
  if (draft) {
    setInterval(() => {
      const d = draft();
      if (draftId && d?.id !== draftId) clearDraft();
      if (d && d.text !== draftText) { draftId = d.id; draftText = d.text; post({ t: 'draft', id: d.id, data: d.text }); }
    }, 5000);
  }
  return { sync, clearDraft };
}
