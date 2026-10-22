// Pomiar grafik BTB CG (gałąź alpha, szablony HTML/CSS/JS) → examples/archiwum/BTB_ALPHA/zrodlo.json dla tools/build-btb-alpha.mjs.
//   1. serwer BTB alpha na kopii bazy:  PORT=3999 DATABASE_URL=kopia.sqlite node server.js   (w katalogu BTB-CG-System, gałąź alpha)
//   2. node tools/measure-btb-alpha.cjs [adres=http://localhost:3999] [plik-wyjściowy]
// Każda grafika (i każdy szablon bez grafiki – z ustawieniami domyślnymi szablonu) wchodzi osobno na output.html;
// po 3 s zapisywane są prostokąty, tła, ramki, teksty i obrazy z DOM. Playwright i socket.io-client: PLAYWRIGHT_DIR (node_modules BTB).
const fs = require('fs');
const os = require('os');
const path = require('path');

const NM = path.join(process.env.PLAYWRIGHT_DIR || path.join(os.homedir(), 'Nextcloud/GitHub/BTB-CG-System-alpha'), 'node_modules');
const { chromium } = require(path.join(NM, 'playwright'));
const io = require(path.join(NM, 'socket.io-client'));
const URL = process.argv[2] || 'http://localhost:3999';
const OUT = path.resolve(process.argv[3] || path.join(__dirname, '../../examples/archiwum/BTB_ALPHA/zrodlo.json'));
const KEEP = ['id', 'name', 'type', 'templateId', 'title', 'subtitle', 'introText', 'items', 'animation', 'style', 'layout', 'wiper', 'tickerMode',
  'separatorStyle', 'speed', 'accentColor', 'url', 'sideImage', 'titleHtml', 'groupId'];

function measure() {
  const out = [];
  for (const el of document.body.querySelectorAll('*')) {
    if (['STYLE', 'SCRIPT', 'HEAD', 'META', 'LINK', 'TITLE'].includes(el.tagName)) continue;
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    if (r.width < 1 || r.height < 1 || cs.display === 'none' || cs.visibility === 'hidden') continue;
    let op = 1;
    for (let e = el; e && e.nodeType === 1; e = e.parentElement) op *= +getComputedStyle(e).opacity;
    const own = [...el.childNodes].filter((n) => n.nodeType === 3 && n.textContent.trim()).map((n) => n.textContent).join('');
    const bg = cs.backgroundImage !== 'none' ? cs.backgroundImage : '';
    const bc = cs.backgroundColor !== 'rgba(0, 0, 0, 0)' ? cs.backgroundColor : '';
    const pse = [getComputedStyle(el, '::before'), getComputedStyle(el, '::after')]
      .map((q) => (q.content !== 'none' && q.display !== 'none' ? `${q.backgroundImage} ${q.backgroundColor} w${q.width} l${q.left}` : '')).filter(Boolean);
    const bordered = cs.borderTopWidth !== '0px' || cs.borderLeftWidth !== '0px' || cs.borderBottomWidth !== '0px';
    if (!own && !bg && !bc && el.tagName !== 'IMG' && el.tagName !== 'CANVAS' && !bordered && !pse.length) continue;
    out.push({ tag: el.tagName, cls: String(el.className).slice(0, 60), x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height),
      op: +op.toFixed(2), bg, bc, rad: cs.borderRadius,
      bord: bordered ? [cs.borderTopWidth, cs.borderRightWidth, cs.borderBottomWidth, cs.borderLeftWidth, cs.borderTopColor, cs.borderLeftColor, cs.borderBottomColor].join(' ') : '',
      sh: cs.boxShadow !== 'none' ? cs.boxShadow : '', tf: cs.transform !== 'none' ? cs.transform : '', pse,
      src: el.tagName === 'IMG' ? el.currentSrc.slice(0, 120) : undefined,
      text: own ? own.trim().slice(0, 200) : undefined,
      font: own ? `${cs.fontFamily}|${cs.fontSize}|${cs.fontWeight}|${cs.color}|ls${cs.letterSpacing}|${cs.textTransform}|lh${cs.lineHeight}|${cs.fontStyle}|ts${cs.textShadow}` : undefined,
      anim: cs.animationName !== 'none' ? `${cs.animationName} ${cs.animationDuration}` : undefined });
  }
  return out;
}

(async () => {
  const s = io(URL);
  const st = await new Promise((res) => s.on('initialState', res));
  // szablony bez grafik: grafika z domyślnymi ustawieniami szablonu (szerokość z `var(--v-width, Npx)`)
  const used = new Set(st.graphics.map((g) => g.templateId));
  const extra = st.templates.filter((t) => !used.has(t.id)).map((t) => {
    const df = { ...(t.defaultFields || {}) };
    delete df.fields;
    const w = /var\(--v-width, (\d+)px\)/.exec(t.css_template || '');
    const layout = { x: 100, y: 800, scale: 1, ...(t.defaultLayout || {}) };
    if (w && !layout.width) layout.width = +w[1];
    return { ...df, id: 'syn-' + t.id, templateId: t.id, type: t.type, name: 'SYN ' + t.name, layout, style: t.defaultStyle || {},
      animation: t.defaultAnimation || {}, title: df.title || 'PRZYKŁADOWY TYTUŁ', visible: false };
  });
  const all = [...st.graphics, ...extra];
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1920, height: 1080 } });
  await p.goto(URL + '/output.html');
  await p.waitForTimeout(1500);
  const pomiar = [];
  for (let i = 0; i < all.length; i++) {
    s.emit('updateState', { ...st, graphics: all.map((g) => ({ ...g, visible: false })) });
    await p.waitForTimeout(1200);
    s.emit('updateState', { ...st, graphics: all.map((g, j) => ({ ...g, visible: j === i })) });
    await p.waitForTimeout(3000);
    pomiar.push(await p.evaluate(measure));
    console.error(i, all[i].name, pomiar[i].length);
  }
  s.emit('updateState', st);
  const zrodlo = { zrodlo: 'github.com/sdorpl/BTB-CG-System gałąź alpha, database.sqlite + templates/',
    szablony: Object.fromEntries(st.templates.map((t) => [t.id, t.name])),
    grafiki: all.map((g) => Object.fromEntries(KEEP.filter((k) => k in g).map((k) => [k, g[k]]))), pomiar };
  fs.writeFileSync(OUT, JSON.stringify(zrodlo, null, 0) + '\n');
  await b.close();
  process.exit(0);
})();
