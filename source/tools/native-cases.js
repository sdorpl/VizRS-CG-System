// Plansze testowe renderera natywnego: każda strona to siatka wariantów jednej grupy właściwości
// (pola, gradienty, cienie, przekształcenia, maski, rozmycie, tekst we wszystkich trybach dopasowania, fonty,
// znaczniki, obrazy, QR, autofollow). Zapisuje show i assets do katalogu:
//   node tools/native-cases.js <katalog>
//   node tools/native-compare.js <katalog>/show.json --assets <katalog>/assets [--out wynik]
const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

const out = path.resolve(process.argv[2] || 'native-cases');
fs.mkdirSync(path.join(out, 'assets'), { recursive: true });

let nid = 0;
const node = (type, props, extra = {}) => ({ id: `n${++nid}`, type, name: `${type}${nid}`, props, ...extra });
/** Siatka kolumn × wierszy: komórka i (od 0) → lewy górny róg. */
const cell = (i, cols = 4, w = 440, h = 230) => ({ x: 60 + (i % cols) * (w + 20), y: 60 + Math.floor(i / cols) * (h + 20), w, h });
const LONG = 'Rząd przedstawił projekt budżetu na 2027 rok. Deficyt niższy od oczekiwań ekonomistów';
const MID = 'Utrudnienia na autostradzie A2 w kierunku Warszawy';

const scenes = [];
const add = (name, nodes) => scenes.push({ id: `c_${scenes.length + 1}`, name, layer: 'main', width: 1920, height: 1080, duration: 0, stops: [], nodes });

// 1. pola: zaokrąglenia, ramki, elipsy
add('Pola i ramki', [
  ...[0, 8, 40, 200].map((r, i) => node('rect', { ...cell(i), color: '#c8102e', radius: r })),
  ...[1, 4, 12, 30].map((b, i) => node('rect', { ...cell(4 + i), color: '#20262e', radius: i * 12, borderWidth: b, borderColor: '#f5a800' })),
  ...[0, 3, 10, 0].map((b, i) => node('ellipse', { ...cell(8 + i), color: i === 3 ? '#ffffff80' : '#1f8a4c', borderWidth: b, borderColor: '#ffffff' })),
]);
// 2. gradienty pod różnymi kątami, z przezroczystością
add('Gradienty', [0, 45, 90, 135, 180, 225, 270, 315, 30, 90, 0, 160].map((a, i) => node('rect', {
  ...cell(i), color: i >= 9 ? '#ffffff00' : '#c8102e', color2: i === 10 ? '#00000000' : '#1c3f94', gradAngle: a, radius: i % 3 === 0 ? 20 : 0,
})));
// 3. cienie pól i krycie grup z ramką i cieniem
add('Cienie i krycie', [
  ...[4, 12, 30, 60].map((s, i) => node('rect', { ...cell(i), color: '#f3f3f3', radius: 10, shadow: s })),
  ...[1, 0.75, 0.5, 0.25].map((o, i) => node('rect', { ...cell(4 + i), color: '#c8102e', radius: 16, borderWidth: 10, borderColor: '#ffffff', shadow: 20, opacity: o })),
  ...[0.5, 0.8].map((o, i) => node('group', { ...cell(8 + i), opacity: o }, { children: [
    node('rect', { x: 0, y: 0, w: 300, h: 150, color: '#1c3f94' }), node('rect', { x: 120, y: 60, w: 300, h: 150, color: '#f5a800' })] })),
  node('ellipse', { ...cell(10), color: '#ffffff', shadow: 24 }),
  node('rect', { ...cell(11), color: '#ffffff40', radius: 30, shadow: 30 }),
]);
// 4. przekształcenia: obrót, skala, pochylenie, punkty zaczepienia, zagnieżdżone grupy
add('Przekształcenia', [
  ...[15, 45, -30, 90].map((r, i) => node('rect', { ...cell(i), w: 300, h: 120, color: '#c8102e', rotation: r, radius: 8 }, { children: [node('text', { x: 10, y: 10, w: 280, h: 100, text: 'Obrót ' + r, size: 44, font: 'Oswald', align: 'center' })] })),
  ...[0.5, 1.4, 0.8, 1].map((s, i) => node('rect', { ...cell(4 + i), w: 300, h: 120, color: '#1c3f94', scale: s, skewX: i === 3 ? 20 : 0 }, { children: [node('text', { x: 10, y: 10, w: 280, h: 100, text: i === 3 ? 'Pochylenie' : 'Skala ' + s, size: 44, font: 'Barlow Condensed', align: 'center' })] })),
  ...['left|top', 'center|middle', 'right|bottom', 'center|top'].map((a, i) => {
    const c = cell(8 + i), [ax, ay] = a.split('|');
    return node('group', { x: c.x, y: c.y, w: c.w, h: c.h, rotation: i * 5 }, { children: [
      node('rect', { x: 200, y: 110, w: 160, h: 70, anchorX: ax, anchorY: ay, color: '#1f8a4c', rotation: 10 }),
      node('ellipse', { x: 196, y: 106, w: 8, h: 8, color: '#ffffff' })] });
  }),
]);
// 5. maski i rozmycie
add('Maski i rozmycie', [
  ...[[20, 0, 0, 0], [0, 30, 10, 0], [10, 10, 10, 10], [45, 45, 0, 0]].map((c, i) => node('rect', { ...cell(i), color: '#c8102e', radius: 20, clipL: c[0], clipR: c[1], clipT: c[2], clipB: c[3] }, { children: [node('text', { x: 20, y: 20, w: 400, h: 190, text: 'Maska', size: 90, font: 'Oswald' })] })),
  ...[1, 3, 8, 20].map((b, i) => node('group', { ...cell(4 + i), blur: b }, { children: [node('rect', { x: 40, y: 40, w: 360, h: 150, color: '#f5a800', radius: 12 }), node('text', { x: 40, y: 40, w: 360, h: 150, text: 'Blur ' + b, size: 60, font: 'Inter', align: 'center', color: '#000000' })] })),
  ...[15, -20].map((r, i) => node('rect', { ...cell(8 + i), color: '#1c3f94', rotation: r, clipL: 25, clipR: 10 })),
  node('rect', { ...cell(10), color: '#1f8a4c', opacity: 0.6, blur: 4, clipT: 20 }, { children: [node('text', { x: 0, y: 0, w: 440, h: 230, text: 'Razem', size: 70, align: 'center' })] }),
]);
// 6–8. tekst: dopasowanie w każdym trybie (jeden i wiele wierszy)
const fits = ['none', 'shrink', 'squeeze', 'squeeze-shrink', 'squeeze-lines', 'squeeze-each'];
add('Dopasowanie – 1 wiersz', fits.flatMap((f, i) => [
  node('rect', { ...cell(i * 2, 4, 440, 110), color: '#20262e' }), node('text', { ...cell(i * 2, 4, 440, 110), text: MID, textFit: f, size: 46, font: 'Barlow Condensed', weight: '600', padX: 12 }),
  node('rect', { ...cell(i * 2 + 1, 4, 440, 110), color: '#20262e' }), node('text', { ...cell(i * 2 + 1, 4, 440, 110), text: LONG, textFit: f, size: 40, font: 'Inter', weight: '500', align: 'center', squeezeMin: 0.5 }),
]));
add('Dopasowanie – wiele wierszy', fits.flatMap((f, i) => [
  node('rect', { ...cell(i * 2, 4, 440, 200), color: '#20262e' }), node('text', { ...cell(i * 2, 4, 440, 200), text: LONG, textFit: f, maxLines: 2, size: 46, font: 'Barlow Condensed', weight: '700', upper: true, lineHeight: 1.02 }),
  node('rect', { ...cell(i * 2 + 1, 4, 440, 200), color: '#20262e' }), node('text', { ...cell(i * 2 + 1, 4, 440, 200), text: LONG + ' ' + MID, textFit: f, maxLines: 3, size: 40, font: 'Roboto', align: 'right', lineBreak: i % 2 ? 'fill' : 'balance', lineHeight: 1.3 }),
]));
add('Wyrównanie i interlinia', [
  ...['left|top', 'center|middle', 'right|bottom', 'center|top'].flatMap((a, i) => {
    const [al, va] = a.split('|');
    return [node('rect', { ...cell(i), color: '#20262e' }), node('text', { ...cell(i), text: 'Linia pierwsza\nDruga, dłuższa linia', align: al, valign: va, maxLines: 2, textFit: 'none', size: 40, font: 'Montserrat', weight: '600', lineHeight: 0.9 + i * 0.25, padX: 20 })];
  }),
  ...[-2, 0, 4, 12].flatMap((s, i) => [node('rect', { ...cell(4 + i), color: '#20262e' }), node('text', { ...cell(4 + i), text: 'Odstęp ' + s, spacing: s, size: 56, font: 'Oswald', align: 'center', textFit: 'none' })]),
  ...[4, 12, 24, 0].flatMap((s, i) => [node('rect', { ...cell(8 + i), color: '#c8102e' }), node('text', { ...cell(8 + i), text: i === 3 ? '1234567890 / 11:11' : 'Cień ' + s, shadow: s, size: 64, font: 'Barlow', weight: '700', align: 'center', textFit: 'none' })]),
]);
// 9. fonty: rodziny i grubości, kursywa (prawdziwa i syntetyczna), pogrubienie syntetyczne, polskie znaki
const fams = ['Inter', 'Oswald', 'Barlow Condensed', 'Barlow', 'Roboto', 'Roboto Condensed', 'Montserrat', 'Bebas Neue', 'Roboto Mono'];
add('Fonty i grubości', [
  ...fams.map((f, i) => node('text', { x: 60, y: 40 + i * 64, w: 900, h: 60, text: `${f}: Zażółć gęślą jaźń „cytat” – 0123`, font: f, size: 40, weight: '400', textFit: 'none' })),
  ...['300', '500', '700', '900'].map((w, i) => node('text', { x: 1000, y: 40 + i * 64, w: 860, h: 60, text: `Inter ${w}: Zażółć gęślą jaźń`, font: 'Inter', size: 40, weight: w, textFit: 'none' })),
  node('text', { x: 1000, y: 296, w: 860, h: 60, text: 'Inter kursywa 400', font: 'Inter', size: 40, weight: '400', italic: true, textFit: 'none' }),
  node('text', { x: 1000, y: 360, w: 860, h: 60, text: 'Oswald kursywa syntetyczna', font: 'Oswald', size: 40, weight: '400', italic: true, textFit: 'none' }),
  node('text', { x: 1000, y: 424, w: 860, h: 60, text: 'Bebas Neue 700 (syntetyczne)', font: 'Bebas Neue', size: 44, weight: '700', textFit: 'none' }),
  node('text', { x: 1000, y: 488, w: 860, h: 60, text: 'Arial (systemowy zastępczy)', font: 'Arial', size: 40, textFit: 'none' }),
  ...[12, 18, 24, 120].map((s, i) => node('text', { x: 1000, y: 560 + [0, 30, 66, 110][i], w: 900, h: s * 1.4, text: `Rozmiar ${s} Zażółć`, font: 'Inter', size: s, weight: '600', textFit: 'none', valign: 'top' })),
]);
// 10. tekst sformatowany
add('Znaczniki', [
  ['Infolinia: [[800 123 456]] całą dobę', 'squeeze', 1], ['{b}Pogrubione{/b} i {i}kursywa{/i} oraz {u}podkreślenie{/u}', 'none', 1],
  ['Kolor {c=#f5a800}żółty{/c} i {c=#4caf50}zielony{/c}', 'none', 1], ['Mały {sz=60}tekst{/sz} i {sz=140}duży{/sz}', 'none', 1],
  ['[[PILNE]] ' + LONG, 'squeeze-lines', 2], ['{b}' + MID + '{/b} [[A2]]', 'squeeze-shrink', 2], ['[[Wyróżnienie {b}pogrubione{/b}]] dalej', 'shrink', 1], ['{u}' + LONG + '{/u}', 'squeeze-each', 3],
].flatMap(([t, f, l], i) => [node('rect', { ...cell(i, 4, 440, 230), color: '#20262e' }), node('text', { ...cell(i, 4, 440, 230), text: t, textFit: f, maxLines: l, size: 38, font: i % 2 ? 'Inter' : 'Barlow Condensed', weight: '600', hlColor: '#c8102e', hlText: '#ffffff', padX: 10 })]));
// 11. obrazy (PNG z alfą, JPEG), dopasowanie, zaokrąglenie, cień; kody QR
add('Obrazy i QR', [
  ...['contain', 'cover', 'fill'].map((f, i) => node('image', { ...cell(i), src: 'test_alpha.png', fitMode: f, radius: i * 20, shadow: i === 2 ? 20 : 0 })),
  node('image', { ...cell(3), src: 'tlo_hashtag.jpg', fitMode: 'cover', radius: 30 }),
  node('image', { ...cell(4), w: 200, h: 200, src: 'test_alpha.png', fitMode: 'contain', rotation: 20, opacity: 0.7 }),
  ...['https://info24.example', 'Zażółć gęślą jaźń 2026', 'x'.repeat(120)].map((t, i) => node('qr', { ...cell(5 + i), text: t, color: i === 1 ? '#1c3f94' : '#000000', qrBg: '#ffffff', radius: i * 10, shadow: i === 2 ? 12 : 0 })),
  node('qr', { ...cell(8), w: 120, h: 120, text: 'QR mały' }),
  node('qr', { ...cell(9), w: 333, h: 333, text: 'QR o nierównym module', rotation: 7 }),
]);
// 12. autofollow
add('Autofollow', ['Krótko', 'Średnia długość tekstu', 'Bardzo długi podpis, który rośnie'].flatMap((t, i) => [
  node('rect', { x: 100, y: 100 + i * 150, w: 100, h: 90, color: '#c8102e', radius: 8, follow: `ft${i}`, followAxis: 'w', followPad: 60, followMin: 150 }),
  node('text', { x: 130, y: 100 + i * 150, w: 1400, h: 90, text: t, size: 54, font: 'Barlow Condensed', weight: '700', textFit: 'none' }, { name: `ft${i}` }),
  node('rect', { x: 1500, y: 100 + i * 150, w: 300, h: 40, anchorX: 'center', color: '#1c3f94', follow: `fh${i}`, followAxis: 'h', followPad: 20 }),
  node('text', { x: 1360, y: 110 + i * 150, w: 280, h: 300, text: t.split(' ').join('\n'), maxLines: 4, textFit: 'none', size: 22, font: 'Inter', valign: 'top' }, { name: `fh${i}` }),
]));

const show = { name: 'Plansze renderera natywnego', scenes, pages: scenes.map((s, i) => ({ id: `p${i + 1}`, number: 9001 + i, name: s.name, scene: s.id, values: {} })), playlists: [] };
fs.writeFileSync(path.join(out, 'show.json'), JSON.stringify(show, null, 1));

// obraz PNG z kanałem alfa (gradient koła) 200×120 – bez zależności
function png(w, h, px) {
  const crc = (b) => { let c, t = []; for (let n = 0; n < 256; n++) { c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } let x = 0xffffffff; for (const v of b) x = t[(x ^ v) & 0xff] ^ (x >>> 8); return (x ^ 0xffffffff) >>> 0; };
  const chunk = (type, data) => { const len = Buffer.alloc(4); len.writeUInt32BE(data.length); const td = Buffer.concat([Buffer.from(type), data]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([len, td, c]); };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 6;
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) px(x, y).forEach((v, k) => { raw[y * (w * 4 + 1) + 1 + x * 4 + k] = v; });
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}
fs.writeFileSync(path.join(out, 'assets/test_alpha.png'), png(200, 120, (x, y) => {
  const d = Math.hypot(x - 100, y - 60) / 60;
  return [Math.round(255 * x / 200), 80, Math.round(255 * y / 120), d < 1 ? 255 : Math.max(0, Math.round(255 * (1.4 - d) / 0.4))];
}));
const jpg = path.join(__dirname, '../../examples/archiwum/assets/tlo_hashtag.jpg');
if (fs.existsSync(jpg)) fs.copyFileSync(jpg, path.join(out, 'assets/tlo_hashtag.jpg'));
console.log(`${scenes.length} plansz: ${out}/show.json`);
