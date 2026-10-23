// Model wiadomości tickera – wspólny dla renderera, Trio, Artist, importu elementów i testów (bez DOM, działa w Node).
// Zapis w show: jedna wiadomość w wierszu, `KAT#kolor|treść` (kategoria i kolor opcjonalne).
// `\|` w treści = dosłowny znak `|` (potrzebne tylko w wiadomości bez kategorii). Te same reguły: src/ticker.rs.

export const TICKER_MODES = ['crawl', 'roll', 'wipe', 'whip'];
export const SEP_STYLES = ['text', 'dot', 'square', 'pipe', 'skew'];
export const TICKER_LIMITS = { items: 200, chars: 500 };

/** Indeks pierwszego `|`, przed którym nie stoi `\`. */
function splitAt(line) {
  for (let i = 0; i < line.length; i++) if (line[i] === '|' && line[i - 1] !== '\\') return i;
  return -1;
}
const unesc = (s) => s.replace(/\\\|/g, '|');

/** "SPORT#1f8a4c|Treść" → {cat:'SPORT', color:'#1f8a4c', text:'Treść'}; bez `|` – sama treść. */
export function parseTickerItem(line) {
  line = String(line ?? '');
  const i = splitAt(line);
  if (i < 0) return { cat: '', color: '', text: unesc(line.trim()) };
  let cat = line.slice(0, i).trim(), color = '';
  const m = cat.match(/^(.*?)#([0-9a-fA-F]{3,8})$/);
  if (m) { cat = m[1].trim(); color = '#' + m[2]; }
  return { cat: unesc(cat), color, text: unesc(line.slice(i + 1).trim()) };
}

/** Odwrotność parseTickerItem. Kategoria nie może zawierać `|` ani końca wiersza (są usuwane). */
export function serializeTickerItem(it) {
  const text = String(it?.text ?? '').replace(/\s*\n\s*/g, ' ').trim();
  const cat = String(it?.cat ?? '').replace(/[|\n]/g, ' ').trim();
  const color = /^#[0-9a-fA-F]{3,8}$/.test(it?.color || '') ? it.color : '';
  if (!cat && !color) return text.replace(/\|/g, '\\|');
  return `${cat}${color}|${text}`;
}

/** Tekst pola tickera → lista wiadomości (puste wiersze pomijane). */
export function parseTickerItems(text) {
  return String(text ?? '').split('\n').map((s) => s.trim()).filter(Boolean).map(parseTickerItem);
}

/** Lista wiadomości → tekst pola (wiadomości bez treści pomijane). */
export function serializeTickerItems(list) {
  return (list || []).filter((it) => String(it?.text ?? '').trim()).map(serializeTickerItem).join('\n');
}

/** "PILNE=#e00000; INFO=#0050c0" → { PILNE: '#e00000', INFO: '#0050c0' } (klucze wielkimi literami). */
export function parseCatColors(s) {
  const out = {};
  for (const part of String(s ?? '').split(/[;\n]/)) {
    const i = part.indexOf('=');
    if (i < 1) continue;
    const k = part.slice(0, i).trim().toUpperCase(), v = part.slice(i + 1).trim();
    if (k && v) out[k] = v;
  }
  return out;
}
export function formatCatColors(map) {
  return Object.entries(map || {}).map(([k, v]) => `${k}=${v}`).join('; ');
}

/** Kolor plakietki: kolor z wiadomości > mapa kategorii > kolor domyślny. */
export function catColorOf(item, map, fallback) {
  if (item?.color) return item.color;
  const k = String(item?.cat ?? '').trim().toUpperCase();
  return (k && map?.[k]) || fallback;
}

/** Etykieta wiadomości: kategoria, a gdy jej brak – etykieta domyślna węzła. */
export const labelOf = (item, label) => String(item?.cat || label || '');

const same = (a, b) => a && b && a.text === b.text && a.cat === b.cat;

/**
 * Ciągłość po zmianie listy (UPDATE na antenie): gdzie jest teraz wyświetlana wiadomość i która ma być następna.
 * @returns { cur: indeks bieżącej wiadomości w nowej liście albo -1 (usunięta), next: indeks następnej }
 */
export function continuation(oldItems, oldIdx, newItems) {
  const n = newItems.length;
  if (!n) return { cur: -1, next: 0 };
  const cur = oldItems[oldIdx];
  const at = newItems.findIndex((x) => same(x, cur));
  if (at >= 0) return { cur: at, next: (at + 1) % n };
  // bieżąca usunięta: następna to pierwsza z kolejnych starych, która przetrwała
  for (let k = 1; k < oldItems.length; k++) {
    const j = newItems.findIndex((x) => same(x, oldItems[(oldIdx + k) % oldItems.length]));
    if (j >= 0) return { cur: -1, next: j };
  }
  return { cur: -1, next: 0 };
}

/**
 * Sprawdzenie listy względem reguł pola (fieldMeta) i limitów systemu.
 * @returns lista problemów [{ i, msg }] (i = -1 dla całej listy)
 */
export function checkTickerItems(list, rules = {}) {
  const out = [];
  const maxItems = Math.min(TICKER_LIMITS.items, rules.maxItems || Infinity);
  if (list.length > maxItems) out.push({ i: -1, msg: `maks. ${maxItems} wiadomości` });
  const maxChars = Math.min(TICKER_LIMITS.chars, rules.maxChars || Infinity);
  const cats = Array.isArray(rules.cats) && rules.cats.length ? rules.cats.map((c) => String(c).toUpperCase()) : null;
  list.forEach((it, i) => {
    if (Array.from(it.text).length > maxChars) out.push({ i, msg: `wiadomość ${i + 1}: maks. ${maxChars} znaków` });
    if (cats && it.cat && !cats.includes(it.cat.toUpperCase())) out.push({ i, msg: `wiadomość ${i + 1}: kategoria „${it.cat}” spoza listy` });
  });
  return out;
}
