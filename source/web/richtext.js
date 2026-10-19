// VizRS – lekki znacznik tekstu sformatowanego (F3.2). Wartość pola nadal jest zwykłym tekstem:
//   [[tekst]]         wyróżnienie (kolorowe tło – jak dotąd)
//   {b}…{/b}          pogrubienie        {i}…{/i}  kursywa        {u}…{/u}  podkreślenie
//   {c=#e30613}…{/c}  kolor              {sz=120}…{/sz}  rozmiar w % (30–300)
// Znaczniki można zagnieżdżać; niedomknięte zamykają się na końcu. Rozróżnianie wielkości liter wyłączone
// (WERSALIKI pola nie psują znaczników). Konwersja do/z RichText v1 z BTB-CG: toRich / fromRich (potrzebują CGText).

const TOKEN = /\[\[|\]\]|\{(\/?)(b|i|u|c|sz)(?:=([^}]*))?\}/gi;
const COLOR_OK = /^(#[0-9a-f]{3,8}|[a-z]{3,20})$/i;

export const hasMarkup = (t) => /\[\[|\{\/?(?:b|i|u|c|sz)(?:=[^}]*)?\}/i.test(String(t ?? ''));

const clone = (s) => ({ b: s.b, i: s.i, u: s.u, hl: s.hl, cs: [...s.cs], ss: [...s.ss] });
export const emptyState = () => ({ b: 0, i: 0, u: 0, hl: 0, cs: [], ss: [] });

/** Rozbija tekst na przebiegi `{t, b, i, u, hl, c, sz}`; zwraca też stan na końcu (do przenoszenia między wierszami). */
export function parseMarkup(text, init) {
  text = String(text ?? '');
  const st = init ? clone(init) : emptyState();
  const runs = [];
  let last = 0, m;
  const push = (s) => {
    if (s) runs.push({ t: s, b: st.b > 0, i: st.i > 0, u: st.u > 0, hl: st.hl > 0, c: st.cs[st.cs.length - 1] || '', sz: st.ss[st.ss.length - 1] || 0 });
  };
  TOKEN.lastIndex = 0;
  while ((m = TOKEN.exec(text))) {
    push(text.slice(last, m.index));
    last = TOKEN.lastIndex;
    const tok = m[0];
    if (tok === '[[') st.hl++;
    else if (tok === ']]') st.hl = Math.max(0, st.hl - 1);
    else {
      const close = m[1] === '/', name = m[2].toLowerCase(), val = (m[3] || '').trim();
      if (name === 'b' || name === 'i' || name === 'u') st[name] = Math.max(0, st[name] + (close ? -1 : 1));
      else if (name === 'c') { if (close) st.cs.pop(); else st.cs.push(COLOR_OK.test(val) ? val : (st.cs[st.cs.length - 1] || '')); }
      else if (name === 'sz') {
        if (close) st.ss.pop();
        else { const n = parseFloat(val); st.ss.push(n >= 30 && n <= 300 ? n : (st.ss[st.ss.length - 1] || 0)); }
      }
    }
  }
  push(text.slice(last));
  return { runs, state: st };
}

/** Tekst bez znaczników (do pomiarów, liczników znaków, tickerów). */
export const stripMarkup = (t) => parseMarkup(t).runs.map((r) => r.t).join('');

const openTags = (st) => '{b}'.repeat(st.b) + '{i}'.repeat(st.i) + '{u}'.repeat(st.u) + st.cs.map((c) => `{c=${c || 'inherit'}}`).join('') + st.ss.map((s) => `{sz=${s || 100}}`).join('');
const closeTags = (st) => '{/b}'.repeat(st.b) + '{/i}'.repeat(st.i) + '{/u}'.repeat(st.u) + '{/c}'.repeat(st.cs.length) + '{/sz}'.repeat(st.ss.length);

/**
 * Wiersze po podziale (np. ręcznym lub automatycznym) tracą powiązanie znaczników – ta funkcja domyka znaczniki na końcu
 * każdego wiersza i otwiera je ponownie na początku następnego, więc każdy wiersz jest samodzielnie poprawny.
 * Znaczniki `[[` nie są przenoszone (wyróżnienie jest niepodzielne).
 */
export function carryLines(lines) {
  let st = emptyState();
  return lines.map((line) => {
    const prefix = openTags(st);
    const { state } = parseMarkup(line, st);
    const out = prefix + line + closeTags({ ...state, hl: 0 });
    st = { ...state, hl: 0 };
    return out;
  });
}

// ---------------------------------------------------------------- import z RichText v1 (BTB-CG, text-model.js)
const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/** HTML BTB-CG → znacznik VizRS (po sanityzacji przez `CGText.toRich`). */
export function fromBtbHtml(html, CGText = globalThis.CGText) {
  const rich = CGText.toRich(html);
  return rich.blocks.map((b) => b.runs.map((r) => {
    if (r.br) return '\n';
    let s = r.t, m = r.m || {};
    if (m.bg) s = `[[${s}]]`;
    if (m.weight >= 600) s = `{b}${s}{/b}`;
    if (m.italic) s = `{i}${s}{/i}`;
    if (m.underline) s = `{u}${s}{/u}`;
    if (m.color) s = `{c=${m.color}}${s}{/c}`;
    return s;
  }).join('')).join('\n');
}
export { esc as _esc };
