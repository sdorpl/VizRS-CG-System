// VizRS – paczka belek `.cgpack` (sceny + strony + grafiki + czcionki w jednym pliku ZIP)
//
//   manifest.json  { _type: 'vizrs_pack', _packVersion: 1, generator, created, scenes:[{id,name}], pages, assets:[{path,name,kind,bytes,crc32}] }
//   scenes.json    { scenes:[…], pages:[…] }       – sceny jak w show.json, strony wskazują `scene` = id sceny z paczki
//   assets/<nazwa> – pliki z katalogu assets (obrazy, wideo, fonty) użyte przez sceny lub dołączone (`scene.packAssets`)
//
// Format własny VizRS (inny `_type` niż zewnętrzna paczka elementu `cg_element_pack`). Paczki elementów czyta `element-import.js`.
import { unzip } from '/element-import.js';

export const PACK_TYPE = 'vizrs_pack';
export const PACK_VERSION = 1;
export const FONT_RE = /\.(ttf|otf|woff2?)$/i;
const ASSET_EXT = /\.(png|jpe?g|gif|webp|svg|mp4|webm|ttf|otf|woff2?)$/i;
const SAFE = /^[^/\\\0]{1,200}$/;

export const assetKind = (name) => (FONT_RE.test(name) ? 'font' : /\.(mp4|webm)$/i.test(name) ? 'video' : 'image');

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
 * `known` – lista nazw w assets, `fetchAsset(name)` → Uint8Array. Zwraca { blob, missing[], assets[] }.
 */
export async function buildPack({ scenes, pages = [], known, fetchAsset, generator = 'VizRS' }) {
  const enc = new TextEncoder();
  const names = new Set();
  for (const s of scenes) sceneAssets(s, known).forEach((n) => names.add(n));
  for (const p of pages) sceneAssets(p.values || {}, known).forEach((n) => names.add(n));
  const files = {}, assets = [], missing = [];
  for (const name of [...names].sort()) {
    try {
      const data = await fetchAsset(name);
      if (!data || !data.length) throw new Error('pusty');
      files['assets/' + name] = data;
      assets.push({ path: 'assets/' + name, name, kind: assetKind(name), bytes: data.length, crc32: crc32(data) });
    } catch { missing.push(name); }
  }
  const ids = new Set(scenes.map((s) => s.id));
  const strip = ({ rev, ...o }) => o; // eslint-disable-line no-unused-vars
  const pk = { scenes: scenes.map(strip), pages: pages.filter((p) => ids.has(p.scene)).map(strip) };
  const manifest = {
    _type: PACK_TYPE, _packVersion: PACK_VERSION, generator, created: new Date().toISOString(),
    scenes: scenes.map((s) => ({ id: s.id, name: s.name || '' })), pages: pk.pages.length, assets,
  };
  files['manifest.json'] = enc.encode(JSON.stringify(manifest, null, 2));
  files['scenes.json'] = enc.encode(JSON.stringify(pk, null, 2));
  return { blob: await zip(files), missing, assets };
}

/** Czy bajty to paczka VizRS (true), paczka elementu (`'ext'`) czy coś innego (false). */
export async function packKind(u8) {
  if (!(u8[0] === 0x50 && u8[1] === 0x4b)) return false;
  const files = await unzip(u8);
  if (files['manifest.json']) {
    try {
      const m = JSON.parse(new TextDecoder().decode(files['manifest.json']));
      if (m._type === PACK_TYPE) return true;
      if (m._type === 'cg_element_pack') return 'ext';
    } catch { /* niżej */ }
  }
  return files['element.json'] ? 'ext' : false;
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

const sameBytes = (a, b) => a.length === b.length && a.every((x, i) => x === b[i]);

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
 * Instaluje pliki z paczki w assets. Plik o tej samej nazwie i treści jest pomijany; o innej treści
 * dostaje nową nazwę (`nazwa_2.ext`), a odnośniki w scenach są przepisywane.
 * `existing` – lista nazw w assets, `fetchAsset(name)` → Uint8Array|null, `upload(name, data)` → Promise.
 * Zwraca { map: Map<stara, nowa>, uploaded[], skipped[] }.
 */
export async function installAssets(assets, { existing, fetchAsset, upload }) {
  const taken = new Set(existing), map = new Map(), uploaded = [], skipped = [];
  for (const [name, data] of assets) {
    let target = name;
    if (taken.has(name)) {
      const cur = await fetchAsset(name).catch(() => null);
      if (cur && sameBytes(cur, data)) { skipped.push(name); continue; }
      const dot = name.lastIndexOf('.');
      for (let i = 2; taken.has(target); i++) target = `${name.slice(0, dot)}_${i}${name.slice(dot)}`;
      map.set(name, target);
    }
    await upload(target, data);
    taken.add(target); uploaded.push(target);
  }
  return { map, uploaded, skipped };
}
