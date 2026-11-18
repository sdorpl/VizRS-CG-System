// Zapis HTML do DOM – jedyne miejsce w stronach VizRS z `innerHTML` (opisany wyjątek w zasadzie B).
// Ten sam plik w VizRS CG Artist (`web/dom.js`, kopia przez tools/sync-cg.sh).
//
//   html`<b>${nazwa}</b>`   – szablon: każda wstawiona wartość jest escapowana (& < > " '), chyba że sama jest
//                             wynikiem html`` / raw(); tablice są składane bez separatora (lista wierszy: arr.map((x) => html`…`));
//                             null i undefined dają pusty tekst (false – „false”, jak w zwykłym szablonie).
//   raw(s)                  – HTML zaufany (stała w kodzie, SVG z własnego generatora qr.js) – bez escapowania.
//   esc(s)                  – sam tekst escapowany (wynik działa w html`` jak raw, a poza nim jak zwykły napis).
//   orEmpty(lista, pusta)   – lista albo pusta, gdy nie ma elementów.
//   setHtml(el, h)          – wpisuje h (html``/raw) do el; zwykły napis trafia jako tekst (escapowany).
//   appendHtml(el, h)       – dopisuje h na końcu el (jak innerHTML +=).
//   replaceHtml(el, h)      – zastępuje el treścią h (jak outerHTML).
// Działa w OBS 27 (CEF 85 = Chromium 85): bez nowszej składni i API.

class Html {
  constructor(s) { this.s = s; }
  toString() { return this.s; }
}

const ESC = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };
const escText = (s) => String(s).replace(/[&<>"']/g, (c) => ESC[c]);

function part(v) {
  if (v instanceof Html) return v.s;
  if (v === null || v === undefined) return '';
  if (Array.isArray(v)) return v.map(part).join('');
  return escText(v);
}

export function html(strings, ...vals) {
  let s = strings[0];
  for (let i = 0; i < vals.length; i++) s += part(vals[i]) + strings[i + 1];
  return new Html(s);
}

export const raw = (s) => new Html(String(s ?? ''));
export const esc = (s) => new Html(escText(s ?? ''));
export const isHtml = (v) => v instanceof Html;
/** Lista wierszy albo `pusta` (html``), gdy lista nie ma elementów – zamiast `arr.map(…).join('') || '…'`. */
export const orEmpty = (list, pusta) => (list.length ? list : pusta);

export function setHtml(el, h) {
  el.innerHTML = part(h);
}

/** Dopisuje h (html``) na końcu treści el – jak `el.innerHTML += …` (treść el jest parsowana od nowa). */
export function appendHtml(el, h) {
  el.innerHTML += part(h);
}

/** Zastępuje cały element el treścią h (html``). */
export function replaceHtml(el, h) {
  el.outerHTML = part(h);
}
