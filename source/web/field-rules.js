// Reguły pól strony (limity, wersaliki, typografia) – wspólne dla Trio i Trio mobilnego, żeby tekst na antenie nie zależał od stanowiska.
import { stripMarkup } from '/renderer.js';

/** Długość tekstu, jaką widzi widz: bez znaczników i podziałów wierszy. */
export const visLen = (t) => Array.from(stripMarkup(t).replace(/\n/g, '')).length;

/** Wersaliki, jeden wiersz, limity wierszy i znaków; `prev` – ostatnia poprawna wartość, wraca po przekroczeniu limitu. */
export function limitText(v, R, prev) {
  let err = '';
  if (R.upper) v = v.toUpperCase();
  const ml = R.oneLine ? 1 : (R.maxLines || 0);
  if (R.oneLine && /\n/.test(v)) v = v.replace(/\n+/g, ' ');
  if (ml > 1 && v.split('\n').length > ml) { err = `Maksymalnie ${ml} linie`; v = prev ?? v.split('\n').slice(0, ml).join('\n'); }
  if (R.maxChars && visLen(v) > R.maxChars) { err = `Limit ${R.maxChars} znaków`; v = prev ?? v; }
  return { v, err };
}

const escHtml = (t) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
/** Polska typografia (CGText.typo z text-model.js): twarde spacje po spójnikach, cudzysłowy „”, wielokropek, półpauza. */
export function polish(text) {
  if (!window.CGText) return text;
  return text.split('\n').map((l) => (l ? CGText.plain(CGText.typo('<div>' + escHtml(l) + '</div>')) : l)).join('\n');
}
