// VizRS – paczka belek `.cgpack` (sceny + strony + grafiki + czcionki w jednym pliku ZIP)
//
//   manifest.json  { _type: 'vizrs_pack', _packVersion: 1, generator, created, scenes:[{id,name}], pages, assets:[{path,name,kind,bytes,crc32}] }
//   scenes.json    { scenes:[…], pages:[…] }       – sceny jak w show.json, strony wskazują `scene` = id sceny z paczki
//   assets/<nazwa> – pliki z katalogu assets (obrazy, wideo, fonty) użyte przez sceny lub dołączone (`scene.packAssets`)
//

// Zasady przenoszenia (jedne dla VizRS CG Artist, /artist i Trio – DOKUMENTACJA §8.7):
//   – elementy grafiki (belki, tickery, plansze) i strony przenosi się tylko paczką `.cgpack`: eksport sceny, paczka belek
//     (Artist) i eksport stron (Trio) zapisują `.cgpack` (sceny + strony + pliki); `.vizrs.json` (dawny eksport sceny) jest tylko czytany;
//   – tożsamość sceny i strony to `id` – ten sam w paczce, w imporcie i w wysyłce z VizRS CG Artist (`src/deploy.rs`);
//   – import: najpierw plan (`planImport`/`planPages`, nic nie zapisuje), potem `importPack` wg czynności z planu;
//     scena w innej wersji albo o zajętej nazwie – czynność wybiera użytkownik (zaktualizuj / kopia „… (2)” / pomiń);
//     pliki wg `assetPlacer`, strony z numerem, nowym numerem (zajęty) albo przesunięte o stałą;
//   – plik `.show.json` w Trio to kopia całego show (playlisty, rundowny, makra), nie sposób przenoszenia belek.
export const PACK_TYPE = 'vizrs_pack';
export const PACK_VERSION = 1;
export const FONT_RE = /\.(ttf|otf|woff2?)$/i;
const ASSET_EXT = /\.(png|jpe?g|gif|webp|svg|mp4|webm|ttf|otf|woff2?)$/i;
const SAFE = /^[^/\\\0]{1,200}$/;

export const assetKind = (name) => (FONT_RE.test(name) ? 'font' : /\.(mp4|webm)$/i.test(name) ? 'video' : 'image');

/**
 * Nazwa pliku tak, jak zapisze ją serwer (`safe_name` w `source/src/main.rs` i w magazynie VizRS CG Artist):
 * znaki spoza `[A-Za-z0-9._-]` → `_`, bez kropek na początku, najwyżej 120 znaków. Pusta → null.
 */
export function safeAssetName(name) {
  const n = [...String(name ?? '')].map((c) => (/^[A-Za-z0-9._-]$/.test(c) ? c : '_')).join('').replace(/^\.+/, '');
  return n && n.length <= 120 ? n : null;
}

// ---------------------------------------------------------------- odczyt ZIP
/** Minimalny czytnik ZIP (stored + deflate przez DecompressionStream): najwyżej 200 plików, 200 MB na plik, 256 MB razem. */
export async function unzip(buf) {
  try { return await unzipRaw(buf); } catch (e) { if (e instanceof RangeError) throw new Error('uszkodzony plik ZIP'); throw e; }
}
async function unzipRaw(buf) {
  const u8 = buf instanceof Uint8Array ? buf : new Uint8Array(buf);
  const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
  let eocd = -1;
  for (let i = u8.length - 22; i >= Math.max(0, u8.length - 65557); i--) if (dv.getUint32(i, true) === 0x06054b50) { eocd = i; break; }
  if (eocd < 0) throw new Error('to nie jest plik ZIP (.cgpack)');
  const count = dv.getUint16(eocd + 10, true);
  let p = dv.getUint32(eocd + 16, true);
  if (count > 200) throw new Error('paczka: więcej niż 200 plików');
  const files = {};
  let total = 0;
  for (let n = 0; n < count; n++) {
    if (dv.getUint32(p, true) !== 0x02014b50) throw new Error('uszkodzony katalog ZIP');
    const method = dv.getUint16(p + 10, true), csize = dv.getUint32(p + 20, true), usize = dv.getUint32(p + 24, true);
    const nlen = dv.getUint16(p + 28, true), elen = dv.getUint16(p + 30, true), clen = dv.getUint16(p + 32, true), off = dv.getUint32(p + 42, true);
    const name = new TextDecoder().decode(u8.subarray(p + 46, p + 46 + nlen));
    p += 46 + nlen + elen + clen;
    if (name.endsWith('/')) continue;
    if (name.includes('..') || name.startsWith('/') || name.includes('\\')) throw new Error('paczka: niedozwolona ścieżka ' + name);
    if (usize > 200 * 1024 * 1024 || (total += usize) > 256 * 1024 * 1024) throw new Error('paczka: za duża');
    const lh = off, nl = dv.getUint16(lh + 26, true), el = dv.getUint16(lh + 28, true);
    const raw = u8.subarray(lh + 30 + nl + el, lh + 30 + nl + el + csize);
    if (method === 0) files[name] = raw.slice();
    else if (method === 8) {
      const ds = new DecompressionStream('deflate-raw');
      const out = new Response(new Blob([raw]).stream().pipeThrough(ds));
      files[name] = new Uint8Array(await out.arrayBuffer());
    } else throw new Error('paczka: nieobsługiwana kompresja ' + method);
  }
  return files;
}

// ---------------------------------------------------------------- CRC-32 i zapis ZIP
const CRC = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; }
  return t;
})();
export function crc32(u8) {
  let c = 0xffffffff;
  for (let i = 0; i < u8.length; i++) c = CRC[(c ^ u8[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
async function deflate(u8) {
  if (typeof CompressionStream === 'undefined') return null;
  try {
    const out = new Response(new Blob([u8]).stream().pipeThrough(new CompressionStream('deflate-raw')));
    return new Uint8Array(await out.arrayBuffer());
  } catch { return null; }
}
/** Buduje archiwum ZIP z mapy { ścieżka: Uint8Array }. Pliki już skompresowane (obrazy, woff2) zapisuje bez kompresji. */
export async function zip(files) {
  const enc = new TextEncoder();
  const parts = [], central = [];
  let off = 0;
  for (const [name, data] of Object.entries(files)) {
    const nm = enc.encode(name);
    const crc = crc32(data);
    const packed = /\.(png|jpe?g|gif|webp|mp4|webm|woff2?)$/i.test(name) ? null : await deflate(data);
    const useDeflate = packed && packed.length < data.length;
    const body = useDeflate ? packed : data;
    const method = useDeflate ? 8 : 0;
    const lh = new DataView(new ArrayBuffer(30));
    lh.setUint32(0, 0x04034b50, true); lh.setUint16(4, 20, true); lh.setUint16(6, 0x0800, true); lh.setUint16(8, method, true);
    lh.setUint16(10, 0, true); lh.setUint16(12, 0x21, true);
    lh.setUint32(14, crc, true); lh.setUint32(18, body.length, true); lh.setUint32(22, data.length, true);
    lh.setUint16(26, nm.length, true); lh.setUint16(28, 0, true);
    parts.push(new Uint8Array(lh.buffer), nm, body);
    const ch = new DataView(new ArrayBuffer(46));
    ch.setUint32(0, 0x02014b50, true); ch.setUint16(4, 20, true); ch.setUint16(6, 20, true); ch.setUint16(8, 0x0800, true); ch.setUint16(10, method, true);
    ch.setUint16(12, 0, true); ch.setUint16(14, 0x21, true);
    ch.setUint32(16, crc, true); ch.setUint32(20, body.length, true); ch.setUint32(24, data.length, true);
    ch.setUint16(28, nm.length, true); ch.setUint32(42, off, true);
    central.push(new Uint8Array(ch.buffer), nm);
    off += 30 + nm.length + body.length;
  }
  const csize = central.reduce((n, p) => n + p.length, 0);
  const end = new DataView(new ArrayBuffer(22));
  const count = Object.keys(files).length;
  end.setUint32(0, 0x06054b50, true); end.setUint16(8, count, true); end.setUint16(10, count, true);
  end.setUint32(12, csize, true); end.setUint32(16, off, true);
  return new Blob([...parts, ...central, new Uint8Array(end.buffer)], { type: 'application/zip' });
}

const sameBytes = (a, b) => !!a && !!b && a.length === b.length && a.every((x, i) => x === b[i]);
const contentKey = (u8) => u8.length + ':' + crc32(u8);
const extOf = (n) => n.slice(n.lastIndexOf('.') + 1).toLowerCase();

// ---------------------------------------------------------------- zasoby sceny
/**
 * Nazwy plików z assets, których używa scena: każdy tekst w scenie (src obrazu, font, klucze animacji, wartości pól)
 * równy nazwie istniejącego pliku, a także `url(…)`/`/assets/…` w tekstach oraz pliki dołączone ręcznie (`packAssets`).
 */
export function sceneAssets(scene, known) {
  const set = new Set(known), out = new Set();
  const visit = (v) => {
    if (typeof v === 'string') {
      if (set.has(v)) out.add(v);
      else if (v.includes('/assets/')) for (const m of v.matchAll(/\/assets\/([^"')\s?#]+)/g)) { const n = decodeURIComponent(m[1]); if (set.has(n)) out.add(n); }
    } else if (Array.isArray(v)) v.forEach(visit);
    else if (v && typeof v === 'object') Object.values(v).forEach(visit);
  };
  visit(scene);
  return [...out].sort();
}

/**
 * Tworzy paczkę. `scenes` – sceny do spakowania, `pages` – strony tych scen (opcjonalnie),
 * `known` – lista nazw w assets, `fetchAsset(name)` → Uint8Array. Zwraca { blob, missing[], assets[], merged[[nazwa, ta sama treść jak]] }.
 * Czcionki o identycznej treści (i rozszerzeniu) trafiają do paczki raz, odnośniki w scenach wskazują jedną nazwę.
 */
export async function buildPack({ scenes, pages = [], known, fetchAsset, generator = 'VizRS' }) {
  const enc = new TextEncoder();
  const ids = new Set(scenes.map((s) => s.id));
  pages = pages.filter((p) => ids.has(p.scene)); // tylko strony pakowanych scen – także przy zbieraniu plików
  const names = new Set();
  for (const s of scenes) sceneAssets(s, known).forEach((n) => names.add(n));
  for (const p of pages) sceneAssets(p.values || {}, known).forEach((n) => names.add(n));
  const files = {}, assets = [], missing = [], same = new Map(), byKey = new Map();
  for (const name of [...names].sort()) {
    try {
      const data = await fetchAsset(name);
      if (!data || !data.length) throw new Error('pusty');
      // ta sama czcionka pod dwiema nazwami (wgrana dwa razy) trafia do paczki raz – odnośniki wskazują pierwszą nazwę
      if (FONT_RE.test(name)) {
        const key = contentKey(data), twin = (byKey.get(key) || []).find((n) => extOf(n) === extOf(name) && sameBytes(files['assets/' + n], data));
        if (twin) { same.set(name, twin); continue; }
        byKey.set(key, [...(byKey.get(key) || []), name]);
      }
      files['assets/' + name] = data;
      assets.push({ path: 'assets/' + name, name, kind: assetKind(name), bytes: data.length, crc32: crc32(data) });
    } catch { missing.push(name); }
  }
  const strip = ({ rev, ...o }) => o; // eslint-disable-line no-unused-vars
  const pk = renameAssets({ scenes: scenes.map(strip), pages: pages.map(strip) }, same);
  for (const s of pk.scenes) if (Array.isArray(s.packAssets)) s.packAssets = [...new Set(s.packAssets)];
  const manifest = {
    _type: PACK_TYPE, _packVersion: PACK_VERSION, generator, created: new Date().toISOString(),
    scenes: scenes.map((s) => ({ id: s.id, name: s.name || '' })), pages: pk.pages.length, assets,
  };
  files['manifest.json'] = enc.encode(JSON.stringify(manifest, null, 2));
  files['scenes.json'] = enc.encode(JSON.stringify(pk, null, 2));
  return { blob: await zip(files), missing, assets, merged: [...same] };
}

/** Czy bajty to paczka VizRS. */
export async function packKind(u8) {
  if (!(u8[0] === 0x50 && u8[1] === 0x4b)) return false;
  const files = await unzip(u8).catch(() => ({}));
  if (!files['manifest.json']) return false;
  try { return JSON.parse(new TextDecoder().decode(files['manifest.json']))._type === PACK_TYPE; } catch { return false; }
}

/** Czyta i sprawdza paczkę VizRS → { manifest, scenes, pages, assets: Map<nazwa, Uint8Array> }. Rzuca Error z opisem. */
export async function readPack(u8) {
  const files = await unzip(u8);
  for (const name of Object.keys(files)) {
    if (name !== 'manifest.json' && name !== 'scenes.json' && !/^assets\/[^/]+$/.test(name)) throw new Error('nieoczekiwany plik w paczce: ' + name);
  }
  if (!files['manifest.json'] || !files['scenes.json']) throw new Error('paczka bez manifest.json lub scenes.json');
  let manifest, body;
  try {
    manifest = JSON.parse(new TextDecoder().decode(files['manifest.json']));
    body = JSON.parse(new TextDecoder().decode(files['scenes.json']));
  } catch { throw new Error('manifest.json lub scenes.json nie jest poprawnym JSON'); }
  if (manifest._type !== PACK_TYPE) throw new Error('to nie jest paczka VizRS (_type)');
  if (!(manifest._packVersion <= PACK_VERSION)) throw new Error(`paczka w nowszej wersji (${manifest._packVersion}) – zaktualizuj VizRS`);
  if (!Array.isArray(body.scenes) || !body.scenes.length) throw new Error('paczka bez scen');
  if (body.scenes.length > 500) throw new Error('paczka: za dużo scen');
  for (const s of body.scenes) if (!s || !Array.isArray(s.nodes)) throw new Error('paczka: uszkodzona scena');
  const assets = new Map();
  for (const a of manifest.assets || []) {
    const name = String(a.name || '');
    if (!SAFE.test(name) || !ASSET_EXT.test(name) || a.path !== 'assets/' + name) throw new Error('paczka: niedozwolona nazwa pliku ' + name);
    const data = files[a.path];
    if (!data) throw new Error('paczka: brak pliku ' + a.path);
    if (a.crc32 != null && crc32(data) !== a.crc32) throw new Error('paczka: suma kontrolna niezgodna ' + a.path);
    assets.set(name, data);
  }
  for (const name of Object.keys(files)) if (name.startsWith('assets/') && !assets.has(name.slice(7))) throw new Error('plik spoza manifestu: ' + name);
  return { manifest, scenes: body.scenes, pages: Array.isArray(body.pages) ? body.pages : [], assets };
}


/**
 * Podmienia nazwy plików w obiekcie (sceny/strony) wg mapy stara → nowa. Dotyczy tekstów równych nazwie
 * oraz odnośników `/assets/<nazwa>` w tekstach.
 */
export function renameAssets(obj, map) {
  if (!map.size) return obj;
  const fix = (v) => {
    if (typeof v === 'string') {
      if (map.has(v)) return map.get(v);
      return v.includes('/assets/') ? v.replace(/\/assets\/([^"')\s?#]+)/g, (m, n) => { const d = decodeURIComponent(n); return map.has(d) ? '/assets/' + encodeURIComponent(map.get(d)) : m; }) : v;
    }
    if (Array.isArray(v)) return v.map(fix);
    if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, fix(x)]));
    return v;
  };
  return fix(obj);
}

/**
 * Reguły umieszczania pliku w assets – wspólne dla importu paczki i plików wgrywanych ręcznie
 * (⤒ przy polu font, „Wgraj i dołącz”, przeciągnięcie pliku):
 *  1. nazwa jak na serwerze (`safeAssetName`) – odnośnik w scenie zawsze wskazuje plik, który naprawdę powstał;
 *  2. czcionka o treści identycznej z czcionką już wgraną (pod dowolną nazwą) nie jest wgrywana drugi raz –
 *     używana jest istniejąca (renderer zna font po nazwie pliku, więc kopia byłaby drugą rodziną z tym samym krojem);
 *  3. plik o tej samej nazwie i treści jest pomijany;
 *  4. nic nie jest nadpisywane: ta sama nazwa (bez względu na wielkość liter – Windows i macOS ich nie rozróżniają)
 *     z inną treścią → `nazwa_2.ext`, `nazwa_3.ext`…; gdy któraś z tych kopii ma już tę treść, używana jest ona.
 * Te same reguły (w tej kolejności) stosuje wysyłka scen z VizRS CG Artist na serwer (`src/deploy.rs`, `Placer`).
 * `existing` – nazwy w assets, `fetchAsset(name)` → Uint8Array|null, `upload(name, data)` → Promise (może zwrócić
 * nazwę zapisaną przez serwer: tekst albo `{ name }`). Zwraca funkcję `place(name, data)` →
 * { name, from, status: 'new'|'renamed'|'same'|'reused' } (`reused` – ta treść już była pod nazwą `name`).
 */
export function assetPlacer({ existing, fetchAsset, upload }) {
  const taken = new Map(); // nazwa małymi literami → nazwa
  for (const n of existing) taken.set(n.toLowerCase(), n);
  let fonts = null; // treść → nazwy czcionek (indeks budowany przy pierwszej czcionce)
  const indexFont = (name, data) => { const k = contentKey(data); fonts.set(k, [...(fonts.get(k) || []), name]); };
  const fontIndex = async () => {
    if (fonts) return fonts;
    fonts = new Map();
    for (const n of taken.values()) {
      if (!FONT_RE.test(n)) continue;
      const d = await Promise.resolve().then(() => fetchAsset(n)).catch(() => null);
      if (d?.length) indexFont(n, d);
    }
    return fonts;
  };
  const read = (n) => Promise.resolve().then(() => fetchAsset(n)).catch(() => null);
  return async function place(from, data) {
    const want = safeAssetName(from);
    if (!want || !ASSET_EXT.test(want)) throw new Error('niedozwolona nazwa pliku ' + from);
    if (!data?.length) throw new Error(from + ': pusty plik');
    if (FONT_RE.test(want)) {
      const cands = (await fontIndex()).get(contentKey(data)) || [];
      // najpierw ta sama nazwa i to samo rozszerzenie (WOFF2 i TTF tego kroju to różne pliki – nie łączymy ich)
      const order = [...cands.filter((n) => n.toLowerCase() === want.toLowerCase()), ...cands.filter((n) => n.toLowerCase() !== want.toLowerCase())];
      for (const n of order) {
        if (extOf(n) !== extOf(want)) continue;
        if (sameBytes(await read(n), data)) return { name: n, from, status: n.toLowerCase() === want.toLowerCase() ? 'same' : 'reused' };
      }
    }
    const cur = taken.get(want.toLowerCase());
    if (cur && !FONT_RE.test(want) && sameBytes(await read(cur), data)) return { name: cur, from, status: 'same' };
    let target = want;
    if (cur) {
      const dot = want.lastIndexOf('.'), stem = want.slice(0, dot), ext = want.slice(dot);
      for (let i = 2; ; i++) {
        const cand = `${stem.slice(0, 120 - ext.length - String(i).length - 1)}_${i}${ext}`, had = taken.get(cand.toLowerCase());
        if (!had) { target = cand; break; }
        // wcześniejsza kopia `nazwa_N.ext` o tej samej treści (ponowny import, ponowna wysyłka) – bez kolejnej
        if (!FONT_RE.test(want) && sameBytes(await read(had), data)) return { name: had, from, status: 'reused' };
      }
    }
    const r = await upload(target, data);
    const got = (typeof r === 'string' ? r : r?.name) || target;
    taken.set(got.toLowerCase(), got);
    if (fonts && FONT_RE.test(got)) indexFont(got, data);
    return { name: got, from, status: got === from ? 'new' : 'renamed' };
  };
}

/**
 * Instaluje pliki z paczki w assets wg reguł `assetPlacer`; odnośniki w scenach przepisuje `renameAssets(…, map)`.
 * Zwraca { map: Map<stara, nowa>, placed: [{ from, name, status }], uploaded[], skipped[], reused[[stara, istniejąca]],
 * renamed[[stara, nowa]] }. `upload`, który niczego nie zapisuje (zwraca nazwę), daje przebieg „na sucho” dla planu importu.
 */
export async function installAssets(assets, opts) {
  const place = assetPlacer(opts);
  const map = new Map(), placed = [], uploaded = [], skipped = [], reused = [], renamed = [];
  for (const [name, data] of assets) {
    const r = await place(name, data);
    placed.push(r);
    if (r.name !== name) map.set(name, r.name);
    if (r.status === 'same') skipped.push(r.name);
    else if (r.status === 'reused') reused.push([name, r.name]);
    else { uploaded.push(r.name); if (r.status === 'renamed') renamed.push([name, r.name]); }
  }
  return { map, placed, uploaded, skipped, reused, renamed };
}

// ---------------------------------------------------------------- import (wspólny dla Artist i Trio)
/** Nazwy plików wskazane w scenach (teksty wyglądające jak plik z assets), których nie ma w `existing`. */
export function missingFiles(objs, existing) {
  const have = new Set(existing.map((n) => n.toLowerCase())), out = new Set();
  const visit = (v) => {
    if (typeof v === 'string') { if (SAFE.test(v) && ASSET_EXT.test(v) && !v.includes(' ') && !have.has(v.toLowerCase())) out.add(v); }
    else if (Array.isArray(v)) v.forEach(visit);
    else if (v && typeof v === 'object') Object.values(v).forEach(visit);
  };
  visit(objs);
  return [...out].sort();
}

/**
 * Plik `.vizrs.json` (dawny eksport jednej sceny, bez plików) → paczka do `importPack`. Rzuca Error, gdy to nie scena.
 */
export function legacyScenePack(text) {
  let s;
  try { s = JSON.parse(text); } catch { throw new Error('plik nie jest poprawnym JSON'); }
  if (!s || !Array.isArray(s.nodes)) throw new Error('to nie jest scena VizRS');
  return { manifest: { _type: PACK_TYPE, legacy: true }, scenes: [s], pages: [], assets: new Map() };
}

/** Kanoniczna treść obiektu (klucze posortowane, bez `id` i `rev`) – „ta sama scena” niezależnie od kolejności pól. */
export function sceneKey(s) {
  const canon = (v) => {
    if (Array.isArray(v)) return '[' + v.map(canon).join(',') + ']';
    if (v && typeof v === 'object') return '{' + Object.keys(v).filter((k) => v[k] !== undefined).sort().map((k) => JSON.stringify(k) + ':' + canon(v[k])).join(',') + '}';
    return JSON.stringify(v ?? null);
  };
  const { id, rev, ...o } = s || {}; // eslint-disable-line no-unused-vars
  return canon(o);
}

/** Wolna nazwa sceny: `nazwa`, a gdy zajęta – `nazwa (2)`, `nazwa (3)`… (`taken` – Set nazw). */
export function uniqueName(name, taken) {
  const base = String(name ?? '').slice(0, 190);
  if (!taken.has(base)) return base;
  for (let i = 2; ; i++) { const n = `${base} (${i})`; if (!taken.has(n)) return n; }
}

const MAX_PAGE = 99999; // jak `validate.rs` (numer strony 1…99 999)

/**
 * Czynności możliwe dla stanu sceny w planie (pierwsza – domyślna, gdy `action` nie jest wymagane od użytkownika):
 *  new        – brak sceny o tym `id`:            add (z tym `id`), skip
 *  same       – scena o tym `id` z tą samą treścią: skip (strony paczki wskazują istniejącą), copy
 *  changed    – scena o tym `id` w innej wersji:   update (strony zostają) | copy | skip – wybiera użytkownik
 *  name-taken – inna scena o tej samej nazwie:     copy (nazwa „… (2)”) | update (zastępuje tamtą) | skip – wybiera użytkownik
 */
export const SCENE_CHOICES = { new: ['add', 'skip'], same: ['skip', 'copy'], changed: ['update', 'copy', 'skip'], 'name-taken': ['copy', 'update', 'skip'] };
/** Stany, w których czynność wybiera zawsze użytkownik (bez domyślnej). */
export const MUST_DECIDE = new Set(['changed', 'name-taken']);

/**
 * Plan importu paczki – nic nie zapisuje. `show` – bieżący show celu ({ scenes, pages }), `existing` – nazwy w assets,
 * `fetchAsset` – jak w `assetPlacer`. Pliki są umieszczane „na sucho” (te same reguły co przy zapisie), sceny porównywane
 * po treści (`sceneKey`, po przepisaniu nazw plików). Zwraca { scenes: [...], pages: [...], files: [...], refs: [...], offset }:
 *  – sceny: { id, name, status, target, targetName, targetRev, action, choices } – `action` = null tam, gdzie decyduje użytkownik;
 *  – strony i odniesienia liczy `planPages` (wywoływana tu i po każdej zmianie czynności sceny albo przesunięcia numerów).
 */
export async function planImport(pk, { show, existing, fetchAsset, withPages = true, offset = 0 }) {
  const dry = await installAssets(pk.assets, { existing, fetchAsset, upload: async (n) => n });
  const scenesIn = renameAssets(pk.scenes, dry.map);
  const cur = show?.scenes || [];
  const byId = new Map(cur.map((s) => [s.id, s])), byName = new Map(cur.map((s) => [s.name, s]));
  const scenes = scenesIn.map((s) => {
    const base = { id: s.id ?? null, name: s.name || '', target: null, targetName: null, targetRev: null };
    const there = s.id != null ? byId.get(s.id) : null;
    const other = !there ? byName.get(s.name) : null;
    const hit = there || other;
    if (hit) Object.assign(base, { target: hit.id, targetName: hit.name, targetRev: hit.rev ?? null });
    const status = there ? (sceneKey(s) === sceneKey(there) ? 'same' : 'changed') : other ? 'name-taken' : 'new';
    return { ...base, status, choices: SCENE_CHOICES[status], action: MUST_DECIDE.has(status) ? null : SCENE_CHOICES[status][0] };
  });
  const files = dry.placed.map((r) => ({ from: r.from, name: r.name, status: r.status }));
  const plan = { scenes, files, pages: [], refs: [], offset, withPages, legacy: !!pk.manifest?.legacy, _pk: pk, _show: show || { scenes: [], pages: [] } };
  return planPages(plan, offset);
}

/**
 * Strony i odniesienia planu wg bieżących czynności scen i przesunięcia numerów `offset` (zmienia i zwraca `plan`).
 * Strona: { id, number, name, scene, status, to, action, choices }:
 *  new          – numer wolny:                   add
 *  number-taken – numer zajęty (także przez inną stronę paczki): renumber (pierwszy wolny po najwyższym, jak serwer) | skip
 *  exists       – strona o tym `id` już jest:    skip | overwrite (wartości z paczki, numer istniejący)
 *  no-scene     – scena strony pominięta / spoza paczki: skip (z powodem w raporcie)
 * Wybór użytkownika (`action` z listy `choices`) przeżywa przeliczenie, gdy stan się nie zmienił.
 * Odniesienia (`refs`): powiązania `dock` scen z paczki wskazujące numer strony, który się zmieni (`page:N`),
 * albo nazwę sceny dodawanej jako kopia (`scene:<nazwa>`) – `importPack` może je poprawić (`fixRefs`).
 */
export function planPages(plan, offset = plan.offset || 0) {
  const pk = plan._pk, show = plan._show;
  offset = Math.trunc(Number(offset) || 0);
  plan.offset = offset;
  const prev = new Map(plan.pages.map((p) => [p.id + '|' + p.number, p]));
  const sceneOf = new Map(plan.scenes.map((e) => [e.id, e]));
  const curPages = show.pages || [];
  const byId = new Map(curPages.map((p) => [p.id, p]));
  const taken = new Set(curPages.map((p) => p.number));
  let top = Math.max(1000, ...curPages.map((p) => p.number || 0));
  const pages = [];
  for (const p of plan.withPages ? pk.pages : []) {
    const e = sceneOf.get(p.scene);
    const want = Number.isInteger(p.number) ? p.number + offset : null;
    const row = { id: p.id ?? null, number: p.number ?? null, name: p.name || '', scene: p.scene, to: want, status: 'new', choices: ['add'], action: 'add', reason: '' };
    if (!e) Object.assign(row, { status: 'no-scene', choices: ['skip'], action: 'skip', reason: 'jej sceny nie ma w paczce' });
    else if (e.action === 'skip' && !e.target) Object.assign(row, { status: 'no-scene', choices: ['skip'], action: 'skip', reason: `scena „${e.name}” pominięta` });
    else if (p.id != null && byId.has(p.id)) {
      const there = byId.get(p.id);
      Object.assign(row, { status: 'exists', to: there.number, choices: ['skip', 'overwrite'], action: 'skip', targetRev: there.rev ?? null });
    } else if (want == null || want < 1 || want > MAX_PAGE || taken.has(want)) {
      Object.assign(row, { status: 'number-taken', choices: ['renumber', 'skip'], action: 'renumber' });
    }
    const old = prev.get(row.id + '|' + row.number);
    if (old && old.status === row.status && row.choices.includes(old.action)) row.action = old.action;
    if (row.action === 'renumber') { do top++; while (taken.has(top)); row.to = top; }
    if (row.action === 'add' || row.action === 'renumber') { taken.add(row.to); top = Math.max(top, row.to); }
    pages.push(row);
  }
  plan.pages = pages;
  // odniesienia: dock scen z paczki → numery stron i nazwy kopii
  const moved = new Map(pages.filter((r) => (r.action === 'add' || r.action === 'renumber') && r.to !== r.number).map((r) => [r.number, r.to]));
  const names = copyNames(plan);
  const refs = [];
  for (const s of pk.scenes) {
    const e = sceneOf.get(s.id ?? null);
    if (!e || e.action === 'skip') continue;
    for (const d of Array.isArray(s.dock) ? s.dock : []) {
      for (const t of String(d?.target || '').split('|')) {
        const m = /^\s*page:(\d+)\s*$/.exec(t), n = /^\s*scene:(.+?)\s*$/.exec(t);
        if (m && moved.has(+m[1])) refs.push({ scene: e.name, from: t.trim(), to: 'page:' + moved.get(+m[1]) });
        else if (n && names.has(n[1])) refs.push({ scene: e.name, from: t.trim(), to: 'scene:' + names.get(n[1]) });
      }
    }
  }
  plan.refs = refs;
  return plan;
}

/** Nazwy scen dodawanych jako kopia: nazwa w paczce → nazwa po imporcie (`… (2)`). */
function copyNames(plan) {
  const taken = new Set((plan._show.scenes || []).map((s) => s.name)), out = new Map();
  for (const e of plan.scenes) if (e.action === 'add') taken.add(e.name);
  for (const e of plan.scenes) {
    if (e.action !== 'copy') continue;
    const n = uniqueName(e.name, taken);
    taken.add(n);
    if (n !== e.name) out.set(e.name, n);
  }
  return out;
}

/** Sceny bez wybranej czynności (import czeka na decyzję użytkownika). */
export const undecided = (plan) => plan.scenes.filter((e) => !e.action);

/** Przepisuje cele `dock` (`page:N`, `scene:<nazwa>`) wg listy `refs` planu. */
function fixDock(scene, refs) {
  if (!refs.length || !Array.isArray(scene.dock)) return scene;
  const map = new Map(refs.filter((r) => r.scene === scene.name).map((r) => [r.from, r.to]));
  if (!map.size) return scene;
  return { ...scene, dock: scene.dock.map((d) => ({ ...d, target: String(d.target || '').split('|').map((t) => map.get(t.trim()) || t).join('|') })) };
}

/**
 * Import paczki (wynik `readPack` albo `legacyScenePack`) na serwer albo do magazynu Artist wg planu (`planImport`):
 *  – pliki wg reguł `assetPlacer`, odnośniki w scenach i stronach przepisane (`renameAssets`);
 *  – sceny wg czynności planu: add (z `id` z paczki), update (PUT z `rev` z planu – strony celu zostają; konflikt → raport),
 *    copy (nazwa „… (2)”, `id` z paczki, gdy wolne), skip (strony paczki wskazują istniejącą scenę, gdy jest);
 *  – strony (`withPages`): add / renumber (numer z planu; numer zajęty w międzyczasie → nadany przez serwer) / overwrite / skip;
 *  – `fixRefs` (domyślnie tak) poprawia powiązania dock scen z paczki wskazujące przenumerowane strony i nazwy kopii.
 * Bez `plan` liczy plan sam i tam, gdzie decyduje użytkownik, przyjmuje „dodaj jako kopię” (zachowanie bez okna planu).
 * `api(method, path, body)` → Promise (JSON, błąd z komunikatem serwera), `existing` – nazwy w assets,
 * `fetchAsset`, `upload` – jak w `assetPlacer`, `show` – bieżący show celu (albo `sceneNames` – same nazwy scen).
 * Zwraca { scenes: [dodane], updated: [zaktualizowane], pages, report: [], uploaded, skipped, reused, renamed }.
 */
export async function importPack(pk, { api, existing, fetchAsset, upload, show = null, sceneNames = [], withPages = true, plan = null, fixRefs = true }) {
  // bez `show` (dawne wywołanie z samymi nazwami): pełny show celu – bez znanych `id` kopia z `id` z paczki nadpisałaby scenę
  show ||= await Promise.resolve().then(() => api('GET', '/api/show')).catch(() => null);
  if (!show?.scenes) show = { scenes: sceneNames.map((name) => ({ id: null, name })), pages: [] };
  const knownIds = show.scenes.every((s) => s.id != null);
  if (!plan) {
    plan = await planImport(pk, { show, existing, fetchAsset, withPages });
    for (const e of plan.scenes) if (!e.action) e.action = 'copy';
    planPages(plan);
  }
  const left = undecided(plan);
  if (left.length) throw new Error('wybierz, co zrobić ze sceną: ' + left.map((e) => `„${e.name}”`).join(', '));
  const report = [];
  const inst = await installAssets(pk.assets, { existing, fetchAsset, upload });
  for (const [a, b] of inst.renamed) report.push(`plik ${a} zapisano jako ${b} (nazwa zajęta przez inny plik albo niedozwolone znaki)`);
  for (const [a, b] of inst.reused) report.push(`${FONT_RE.test(a) ? 'czcionka' : 'plik'} ${a} jest już w zasobach jako ${b} – użyto istniejącej (bez kopii)`);
  const refs = fixRefs ? plan.refs : [];
  const scenesIn = renameAssets(pk.scenes, inst.map).map((s) => fixDock(s, refs)), pagesIn = renameAssets(pk.pages, inst.map);
  if (pk.manifest?.legacy) {
    report.push('starszy format .vizrs.json – bez grafik i czcionek; elementy przenoś paczką .cgpack (⇩ Eksport)');
    for (const n of missingFiles(scenesIn, [...existing, ...inst.uploaded])) report.push(`brak pliku ${n} w zasobach – wgraj go albo zaimportuj paczkę .cgpack z tą sceną`);
  }
  for (const r of refs) report.push(`scena „${r.scene}”: powiązanie ${r.from} → ${r.to}`);
  if (!fixRefs && plan.refs.length) for (const r of plan.refs) report.push(`scena „${r.scene}”: powiązanie ${r.from} wskazuje teraz co innego (po imporcie: ${r.to}) – popraw je w Artist`);
  const names = copyNames(plan), curIds = new Set((show.scenes || []).map((s) => s.id).filter(Boolean));
  const keepId = (id) => knownIds && id != null && !curIds.has(id); // `id` z paczki tylko, gdy na pewno wolne
  const ids = new Map(), saved = [], updated = [];
  scenesIn.forEach((s, i) => {
    const e = plan.scenes[i];
    if (e.target && (e.action === 'skip' || e.action === 'update')) ids.set(s.id ?? i, e.target); // strony → istniejąca scena
  });
  for (let i = 0; i < scenesIn.length; i++) {
    const s = scenesIn[i], e = plan.scenes[i], key = s.id ?? i;
    const { id, rev, ...body } = s; // eslint-disable-line no-unused-vars
    try {
      if (e.action === 'add') {
        const sv = await api('POST', '/api/scenes', keepId(id) ? { ...body, id } : body);
        saved.push(sv); ids.set(key, sv.id);
      } else if (e.action === 'copy') {
        body.name = names.get(e.name) || e.name;
        const sv = await api('POST', '/api/scenes', keepId(id) ? { ...body, id } : body);
        saved.push(sv); ids.set(key, sv.id);
        report.push(e.status === 'same' ? `scena „${e.name}”: dodano kopię jako „${sv.name}”` : `scena „${e.name}”: ${e.status === 'name-taken' ? 'nazwa zajęta' : 'jest już w innej wersji'} – dodano jako „${sv.name}”`);
      } else if (e.action === 'update') {
        const sv = await api('PUT', '/api/scenes/' + encodeURIComponent(e.target), { ...body, id: e.target, ...(e.targetRev != null ? { rev: e.targetRev } : {}) });
        updated.push(sv);
        report.push(e.status === 'name-taken' ? `scena „${e.name}”: zastąpiono istniejącą scenę o tej nazwie (jej strony zostały)` : `scena „${e.name}”: zaktualizowano (strony zostały)`);
      } else if (e.status !== 'same') report.push(`scena „${e.name}”: pominięto${e.target ? ' – strony z paczki wskazują istniejącą' : ''}`);
    } catch (err) {
      ids.delete(key);
      report.push(/^konflikt/.test(err.message) ? `scena „${e.name}”: zmieniona w międzyczasie – nie zaktualizowano (zaimportuj paczkę ponownie)` : `scena „${e.name}”: ${err.message}`);
      if (e.target && e.action === 'update') ids.set(key, e.target);
    }
  }
  let pages = 0;
  if (!withPages && pagesIn.length) report.push(`${pagesIn.length} stron Trio pominięto – praca lokalna tworzy tylko sceny (strony zaimportuje Trio z tej samej paczki)`);
  const sceneKeyOf = (p) => { const i = pk.scenes.findIndex((s) => (s.id ?? null) === (p.scene ?? null)); return i < 0 ? undefined : pk.scenes[i].id ?? i; };
  for (let i = 0; i < (withPages ? pagesIn.length : 0); i++) {
    const p = pagesIn[i], row = plan.pages[i] || { action: 'add', to: p.number, status: 'new' };
    const label = `strona ${p.number ?? '?'}${p.name ? ` „${p.name}”` : ''}`;
    if (row.action === 'skip') {
      if (row.status === 'no-scene') report.push(`${label}: pominięto – ${row.reason}`);
      else if (row.status === 'exists') report.push(`${label}: jest już w show – pominięto`);
      else report.push(`${label}: pominięto`);
      continue;
    }
    const sid = ids.get(sceneKeyOf(p));
    if (!sid) { report.push(`${label}: pominięto – jej scena nie została zapisana`); continue; }
    const { id, rev, ...body } = p; // eslint-disable-line no-unused-vars
    body.scene = sid;
    try {
      if (row.action === 'overwrite') {
        await api('PUT', '/api/pages/' + encodeURIComponent(id), { ...body, id, number: row.to, ...(row.targetRev != null ? { rev: row.targetRev } : {}) });
        pages++; report.push(`${label}: nadpisano wartościami z paczki`);
        continue;
      }
      body.number = row.to;
      if (id != null && knownIds && !(show.pages || []).some((q) => q.id === id)) body.id = id;
      try { await api('POST', '/api/pages', body); }
      catch (e) {
        if (!/zajęty/.test(e.message)) throw e;
        delete body.number; // numer zajęty w międzyczasie – nadaje serwer
        const sv = await api('POST', '/api/pages', body);
        row.status = 'number-taken'; row.to = sv?.number ?? null;
      }
      pages++;
      if (row.to !== p.number) report.push(`${label}: numer ${row.status === 'number-taken' ? 'zajęty' : 'przesunięty'} – nadano ${row.to ?? 'nowy'}`);
    } catch (e) { report.push(`${label}: ${e.message}`); }
  }
  if (withPages && pagesIn.length) report.push('playlisty, rundowny i makra nie jadą w paczce – dodaj strony do nich w Trio');
  return { scenes: saved, updated, pages, report, uploaded: inst.uploaded, skipped: inst.skipped, reused: inst.reused, renamed: inst.renamed };
}

/** Jedno zdanie podsumowania importu (raport Artist i Trio). */
export function importSummary(r, withPages = true) {
  const fonts = r.uploaded.filter((n) => FONT_RE.test(n)), had = r.skipped.length + r.reused.length;
  const upd = r.updated?.length ? `, ${r.updated.length} zaktualizowanych` : '';
  return `Zaimportowano paczkę: ${r.scenes.length} scen${upd}${withPages ? `, ${r.pages} stron` : ''}, ${r.uploaded.length} nowych plików${fonts.length ? ` (czcionki: ${fonts.join(', ')})` : ''}${had ? `, ${had} plików już było` : ''}`;
}
