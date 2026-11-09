// Okna pytań w stylu aplikacji zamiast confirm() / prompt() przeglądarki: tamte w oknie natywnym mają w tytule adres
// serwera („127.0.0.1:8088 says”) i przyciski OK / Anuluj. Tu przyciski nazywają czynność, Enter zatwierdza, Esc anuluje.
//   ask(tekst, { title, ok, cancel, danger }) → Promise<boolean>
//   askText(tekst, { title, value, ok, placeholder }) → Promise<string | null>   (pusty tekst = null)
//   choose(tekst, [{ label, value, danger }], { title }) → Promise<value | null>
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

function open({ title, text, input, buttons }) {
  return new Promise((resolve) => {
    const back = document.activeElement;
    const d = document.createElement('div');
    d.className = 'dlg-back';
    d.innerHTML = `<div class="dlg" role="dialog" aria-modal="true">
      ${title ? `<div class="dlg-title">${esc(title)}</div>` : ''}
      <div class="dlg-text"></div>
      ${input ? `<input class="dlg-input" spellcheck="false" placeholder="${esc(input.placeholder || '')}">` : ''}
      <div class="dlg-btns">${buttons.map((b, i) => `<button data-dlg="${i}" class="${b.cls || ''}">${esc(b.label)}</button>`).join('')}</div>
    </div>`;
    d.querySelector('.dlg-text').textContent = text;
    const field = d.querySelector('.dlg-input');
    if (field) field.value = input.value ?? '';
    const cancel = buttons.find((b) => b.cancel);
    const main = buttons.find((b) => b.main) || buttons[buttons.length - 1];
    const finish = (b) => {
      d.remove();
      removeEventListener('keydown', keys, true);
      back?.focus?.();
      resolve(b ? b.value(field?.value) : cancel ? cancel.value(field?.value) : null);
    };
    // pierwszeństwo przed skrótami strony (F-klawisze, Ctrl+Enter = TAKE) – okno jest modalne
    const keys = (e) => {
      if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); return finish(cancel); }
      if (e.key === 'Enter' && !e.isComposing) {
        const f = document.activeElement?.dataset?.dlg;
        e.preventDefault(); e.stopPropagation();
        return finish(f !== undefined ? buttons[+f] : main);
      }
      if (e.key === 'Tab') {
        const list = [...d.querySelectorAll('input, button')];
        const i = list.indexOf(document.activeElement);
        e.preventDefault();
        list[(i + (e.shiftKey ? -1 : 1) + list.length) % list.length].focus();
        return;
      }
      // pisanie w polu działa (domyślna czynność zostaje), ale żaden skrót strony nie dostaje klawisza
      e.stopPropagation();
      if (!d.contains(e.target)) e.preventDefault();
    };
    addEventListener('keydown', keys, true);
    d.addEventListener('click', (e) => {
      const i = e.target.closest('[data-dlg]')?.dataset.dlg;
      if (i !== undefined) finish(buttons[+i]);
    });
    d.addEventListener('mousedown', (e) => { if (e.target === d) { e.preventDefault(); finish(cancel); } });
    document.body.appendChild(d);
    if (field) { field.focus(); field.select(); } else d.querySelector(`[data-dlg="${buttons.indexOf(main)}"]`).focus();
  });
}

export function ask(text, { title = '', ok = 'OK', cancel = 'Anuluj', danger = false } = {}) {
  return open({
    title, text,
    buttons: [
      { label: cancel, cancel: true, value: () => false },
      { label: ok, main: true, cls: danger ? 'acc dlg-danger' : 'acc', value: () => true },
    ],
  });
}

export function askText(text, { title = '', value = '', ok = 'OK', placeholder = '' } = {}) {
  return open({
    title, text, input: { value, placeholder },
    buttons: [
      { label: 'Anuluj', cancel: true, value: () => null },
      { label: ok, main: true, cls: 'acc', value: (v) => (v ?? '').trim() || null },
    ],
  });
}

export function choose(text, options, { title = '' } = {}) {
  const main = options.some((o) => o.main);
  return open({
    title, text,
    buttons: [
      { label: 'Anuluj', cancel: true, main: !main, value: () => null },
      ...options.map((o) => ({ label: o.label, main: !!o.main, cls: o.danger ? 'danger' : o.main ? 'acc' : '', value: () => o.value })),
    ],
  });
}
