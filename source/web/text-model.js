/* Kopia text-model.js z BTB-CG-System-alpha (commit f40f334, gałąź v0.2.x): wspólny model tekstu (sanityzacja HTML, RichText v1, typografia PL,
 * reguły pola, zamiana słów, tokeny pisowni). NIE EDYTUJ RĘCZNIE – odświeżaj: tools/sync-btb.sh. Zgodność pilnują wspólne wektory tests/vectors/*.json. */
/**
 * text-model.js — wspólny model tekstu (M3.2): sanityzacja HTML, tekst/linie, RichText v1 ↔ HTML.
 *
 * Jedna implementacja logiki w dwóch językach: ten plik (Node + przeglądarka, bez DOM — własny tokenizer) i `src/text.rs`
 * w BTB CG Editor. Zgodność pilnują wspólne wektory `test-fixtures/text/*.json` (Node: `test-text-model.js`, Rust: `cargo test`).
 *
 * RichText v1:  { v: 1, blocks: [ { align?: 'left'|'center'|'right', lh?: 0.5…3 (interlinia bloku, mnożnik), runs: [ { t: 'tekst', m?: marks } | { br: true } ] } ] }
 * marks:        { weight?: 100–900, italic?, underline?, strike?, color?, bg?, family?, size? (px), tracking? (em), transform?, squash? (0–1), padding? (CSS, np. '0 8px'), radius? (CSS, np. '6px') }
 * RichText 1.1 (M7 T2, formatowanie znaków jak w Viz): marks dodatkowo { opacity? (0–1), dx?/dy? (em, −2…2), scale? (0,25–4), rotate? (stopnie, −360…360) };
 *               dokument z którymkolwiek z nich ma `v: 1.1`, bez nich `v: 1` (czytniki v1 dostają to samo co dotąd).
 *               HTML: `opacity: …` oraz `transform: translate(dx, dy) scale(s) rotate(r) scaleX(squash)` + `display: inline-block`.
 *
 * Sanityzator: lista dozwolonych znaczników i właściwości CSS; wszystko inne jest usuwane (zawartość `script`/`style` razem z nią).
 * Wynik jest kanoniczny: `b→strong`, `i→em`, `strike→s`, `p→div` (opcja `blocks`), małe litery w nazwach, `style` tylko z dozwolonych właściwości.
 */
(function (root, factory) {
    if (typeof module === 'object' && module.exports) module.exports = factory();
    else root.CGText = factory();
})(typeof self !== 'undefined' ? self : this, function () {
    'use strict';

    const VERSION = 1;
    const VERSION_FORMATTED = 1.1;
    const CHAR_MARKS = ['opacity', 'dx', 'dy', 'scale', 'rotate'];
    const MAX_HTML = 200000, MAX_DEPTH = 24;
    const TAG_MAP = { strong: 'strong', b: 'strong', em: 'em', i: 'em', u: 'u', s: 's', strike: 's', span: 'span', div: 'div', p: 'p', br: 'br' };
    const VOID = new Set(['br']);
    const DROP_WITH_CONTENT = new Set(['script', 'style', 'iframe', 'object', 'embed', 'template', 'noscript', 'head', 'title']);
    const BLOCK = new Set(['div', 'p']);
    const COLOR = /^(#[0-9a-f]{3,8}|rgba?\(\s*\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3}\s*(,\s*(0|1|0?\.\d+|1\.0+)\s*)?\)|hsla?\([\d.,%\s]+\)|[a-z]{3,20}|transparent|currentcolor)$/i;
    const LEN = /^-?\d+(\.\d+)?(px|em|rem|%|pt)$/i;
    const FONT_SIZE_KW = /^(xx-small|x-small|small|medium|large|x-large|xx-large|xxx-large|smaller|larger)$/i;
    const STYLE_RULES = {
        'color': (v) => COLOR.test(v),
        'background-color': (v) => COLOR.test(v),
        'background': (v) => COLOR.test(v),
        'font-family': (v) => /^[\p{L}\p{N} ,'"_-]{1,120}$/u.test(v),
        'font-size': (v) => LEN.test(v) || FONT_SIZE_KW.test(v),
        'font-weight': (v) => /^(normal|bold|bolder|lighter|[1-9]00)$/i.test(v),
        'font-style': (v) => /^(normal|italic|oblique)$/i.test(v),
        'text-decoration': (v) => /^(none|underline|line-through|overline)( (underline|line-through|overline))*$/i.test(v),
        'text-decoration-line': (v) => /^(none|underline|line-through|overline)( (underline|line-through|overline))*$/i.test(v),
        'letter-spacing': (v) => LEN.test(v) || /^normal$/i.test(v),
        'text-transform': (v) => /^(none|uppercase|lowercase|capitalize)$/i.test(v),
        'text-align': (v) => /^(left|center|right|justify)$/i.test(v),
        'line-height': (v) => /^(normal|\d+(\.\d+)?|\d+(\.\d+)?(px|em|%))$/i.test(v),
        'display': (v) => /^(inline|inline-block)$/i.test(v),
        'transform': (v) => TRANSFORM_LIST.test(v),
        'opacity': (v) => /^\d*\.?\d+$/.test(v) && parseFloat(v) <= 1,
        'transform-origin': (v) => /^(left|center|right)( (center|top|bottom))?$/i.test(v),
        'padding': (v) => /^((\d+(\.\d+)?(px|em)|0) ?){1,4}$/i.test(v.trim()),
        'border-radius': (v) => /^((\d+(\.\d+)?(px|em|%)|0) ?){1,4}$/i.test(v.trim()),
    };
    // lista funkcji transform: translate(Xem, Yem) scale(S) rotate(Rdeg) scaleX(Q) — w dowolnej kolejności, każda najwyżej raz (sprawdzane przy parsowaniu)
    const TF_FN = '(?:translate\\(\\s*-?\\d*\\.?\\d+em\\s*,\\s*-?\\d*\\.?\\d+em\\s*\\)|scalex\\(\\s*\\d*\\.?\\d+\\s*\\)|scale\\(\\s*\\d*\\.?\\d+\\s*\\)|rotate\\(\\s*-?\\d*\\.?\\d+deg\\s*\\))';
    const TRANSFORM_LIST = new RegExp('^' + TF_FN + '(?: ' + TF_FN + '){0,3}$', 'i');
    const ENTITY_OK = /^&(#\d{1,7}|#x[0-9a-f]{1,6}|[a-z][a-z0-9]{1,10});/i;

    function escText(s) { return s.replace(/&(?!(#\d{1,7}|#x[0-9a-f]{1,6}|[a-z][a-z0-9]{1,10});)/gi, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;'); }

    /** Style: "prop: value; …" → tylko dozwolone, kanonicznie ("prop: value; prop: value"). */
    function cleanStyle(style) {
        const out = [];
        String(style || '').split(';').forEach((decl) => {
            const i = decl.indexOf(':');
            if (i < 1) return;
            const prop = decl.slice(0, i).trim().toLowerCase();
            let val = decl.slice(i + 1).trim().replace(/\s+/g, ' ').replace(/!important$/i, '').trim();
            const rule = STYLE_RULES[prop];
            if (!rule || !val || /url\s*\(|expression|javascript:|[<>{}\\]|\/\*/i.test(val)) return;
            if (!rule(val)) return;
            if (prop === 'color' || prop === 'background-color' || prop === 'background') val = val.toLowerCase().replace(/\s*,\s*/g, ', ').replace(/\(\s+/g, '(').replace(/\s+\)/g, ')');
            if (prop === 'font-family') val = val.replace(/"/g, "'");
            if (prop === 'background') prop2(out, 'background-color', val); else prop2(out, prop, val);
        });
        return out.join('; ');
    }
    function prop2(out, prop, val) {
        const k = out.findIndex((d) => d.startsWith(prop + ':'));
        if (k >= 0) out.splice(k, 1);
        out.push(`${prop}: ${val}`);
    }

    /** Tokenizer: [{ type:'text', v } | { type:'open', name, attrs, selfClose } | { type:'close', name }] */
    function tokenize(html) {
        const tokens = [];
        let i = 0;
        const n = html.length;
        while (i < n) {
            const lt = html.indexOf('<', i);
            if (lt < 0) { tokens.push({ type: 'text', v: html.slice(i) }); break; }
            if (lt > i) tokens.push({ type: 'text', v: html.slice(i, lt) });
            if (html.startsWith('<!--', lt)) { const e = html.indexOf('-->', lt + 4); i = e < 0 ? n : e + 3; continue; }
            const m = /^<(\/?)([a-zA-Z][a-zA-Z0-9]*)((?:[^>"']|"[^"]*"|'[^']*')*)>/.exec(html.slice(lt, lt + 4000));
            if (!m) { tokens.push({ type: 'text', v: '<' }); i = lt + 1; continue; }
            const name = m[2].toLowerCase();
            if (m[1]) tokens.push({ type: 'close', name });
            else {
                const attrs = {};
                const re = /([a-zA-Z_:][-a-zA-Z0-9_:.]*)(?:\s*=\s*("([^"]*)"|'([^']*)'|([^\s"'>]+)))?/g;
                let a;
                const raw = m[3].replace(/\/\s*$/, '');
                while ((a = re.exec(raw))) { const v = a[3] !== undefined ? a[3] : a[4] !== undefined ? a[4] : a[5] || ''; attrs[a[1].toLowerCase()] = v; }
                tokens.push({ type: 'open', name, attrs, selfClose: /\/\s*$/.test(m[3]) || VOID.has(name) });
            }
            i = lt + m[0].length;
        }
        return tokens;
    }

    /**
     * Sanityzuje HTML. opts: { blocks: 'div'|'keep' (domyślnie 'div': p→div), allowClass: bool (domyślnie true dla `cg-text-squash`) }.
     * Zwraca kanoniczny HTML (string).
     */
    function sanitize(html, opts) {
        const o = Object.assign({ blocks: 'div' }, opts);
        let src = String(html == null ? '' : html);
        if (src.length > MAX_HTML) src = src.slice(0, MAX_HTML);
        const tokens = tokenize(src);
        const out = [];
        const stack = [];
        let skip = 0, skipName = null;
        for (const t of tokens) {
            if (skip) {
                if (t.type === 'open' && t.name === skipName && !t.selfClose) skip++;
                else if (t.type === 'close' && t.name === skipName) skip--;
                continue;
            }
            if (t.type === 'text') { out.push(escText(t.v)); continue; }
            if (t.type === 'open') {
                if (DROP_WITH_CONTENT.has(t.name)) { if (!t.selfClose) { skip = 1; skipName = t.name; } continue; }
                let tag = TAG_MAP[t.name];
                if (!tag) continue;                                   // nieznany znacznik: zawartość zostaje, znacznik znika
                if (tag === 'p' && o.blocks === 'div') tag = 'div';
                if (tag === 'br') { out.push('<br>'); continue; }
                if (stack.length >= MAX_DEPTH) continue;
                let attr = '';
                const st = cleanStyle(decode(t.attrs.style || ''));   // atrybut może mieć encje (&quot;)
                if (st && (tag === 'span' || tag === 'div' || tag === 'p' || tag === 'strong' || tag === 'em' || tag === 'u' || tag === 's')) attr += ` style="${st}"`;
                if (tag === 'span' && /\bcg-text-squash\b/.test(t.attrs.class || '')) attr = ' class="cg-text-squash"' + attr;
                out.push(`<${tag}${attr}>`);
                stack.push(tag);
                continue;
            }
            // close
            let tag = TAG_MAP[t.name];
            if (!tag || tag === 'br') continue;
            if (tag === 'p' && o.blocks === 'div') tag = 'div';
            const k = stack.lastIndexOf(tag);
            if (k < 0) continue;
            while (stack.length > k) out.push(`</${stack.pop()}>`);
        }
        while (stack.length) out.push(`</${stack.pop()}>`);
        return out.join('');
    }

    const NAMED = { nbsp: ' ', lt: '<', gt: '>', quot: '"', apos: "'", amp: '&', copy: '©', reg: '®', deg: '°', euro: '€', ndash: '–', mdash: '—', hellip: '…',
        laquo: '«', raquo: '»', bdquo: '„', ldquo: '“', rdquo: '”', lsquo: '‘', rsquo: '’', middot: '·', bull: '•' };
    // jedno przejście (bez podwójnego dekodowania: &amp;lt; → &lt;)
    const decode = (s) => s.replace(/&(#\d{1,7}|#x[0-9a-fA-F]{1,6}|[a-zA-Z][a-zA-Z0-9]{1,10});/g, (m, e) => {
        if (e[0] === '#') { const c = e[1] === 'x' || e[1] === 'X' ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10); return c > 0 && c < 0x110000 && !(c >= 0xd800 && c < 0xe000) ? String.fromCodePoint(c) : ''; }
        return Object.prototype.hasOwnProperty.call(NAMED, e) ? NAMED[e] : m;
    });

    // ── RichText v1 ──────────────────────────────────────────────────────────────────────────────

    const px = (v) => { const m = /^(-?\d+(\.\d+)?)px$/i.exec(v); return m ? Math.round(parseFloat(m[1]) * 100) / 100 : null; };
    const emv = (v) => { const m = /^(-?\d+(\.\d+)?)em$/i.exec(v); return m ? Math.round(parseFloat(m[1]) * 1000) / 1000 : null; };
    const wnum = (v) => /^bold$/i.test(v) ? 700 : /^normal$/i.test(v) ? 400 : /^[1-9]00$/.test(v) ? +v : null;

    function marksFromTag(tag, style, cls) {
        const m = {};
        if (tag === 'strong') m.weight = 700;
        if (tag === 'em') m.italic = true;
        if (tag === 'u') m.underline = true;
        if (tag === 's') m.strike = true;
        if (cls && /cg-text-squash/.test(cls)) m.squash = null;
        String(style || '').split(';').forEach((d) => {
            const i = d.indexOf(':'); if (i < 1) return;
            const p = d.slice(0, i).trim(), v = d.slice(i + 1).trim();
            if (p === 'color') m.color = v;
            else if (p === 'background-color') m.bg = v;
            else if (p === 'font-family') m.family = v.split(',')[0].replace(/['"]/g, '').trim();
            else if (p === 'font-size') { const n = px(v); if (n != null) m.size = n; }
            else if (p === 'font-weight') { const n = wnum(v); if (n != null) m.weight = n; }
            else if (p === 'font-style') { if (/italic|oblique/i.test(v)) m.italic = true; else if (/normal/i.test(v)) m.italic = false; }
            else if (p === 'text-decoration' || p === 'text-decoration-line') { if (/underline/.test(v)) m.underline = true; if (/line-through/.test(v)) m.strike = true; }
            else if (p === 'letter-spacing') { const n = emv(v); if (n != null) m.tracking = n; }
            else if (p === 'text-transform') m.transform = v.toLowerCase();
            else if (p === 'padding') { if (STYLE_RULES.padding(v)) m.padding = v.trim().replace(/ +/g, ' '); }
            else if (p === 'border-radius') { if (STYLE_RULES['border-radius'](v)) m.radius = v.trim().replace(/ +/g, ' '); }
            else if (p === 'transform') {
                const s = /scalex\(\s*(\d*\.?\d+)\s*\)/i.exec(v); if (s) m.squash = Math.round(parseFloat(s[1]) * 1000) / 1000;
                const tr = /translate\(\s*(-?\d*\.?\d+)em\s*,\s*(-?\d*\.?\d+)em\s*\)/i.exec(v);
                if (tr) { const dx = clampN(parseFloat(tr[1]), -2, 2, 1000), dy = clampN(parseFloat(tr[2]), -2, 2, 1000); if (dx) m.dx = dx; if (dy) m.dy = dy; }
                const sc = /(?:^|\s)scale\(\s*(\d*\.?\d+)\s*\)/i.exec(v); if (sc) { const k = clampN(parseFloat(sc[1]), 0.25, 4, 100); if (k !== 1) m.scale = k; }
                const ro = /rotate\(\s*(-?\d*\.?\d+)deg\s*\)/i.exec(v); if (ro) { const r = clampN(parseFloat(ro[1]), -360, 360, 10); if (r) m.rotate = r; }
            }
            else if (p === 'opacity') { const o = clampN(parseFloat(v), 0, 1, 100); if (o < 1) m.opacity = o; }
        });
        if (m.squash === null) delete m.squash;
        return m;
    }
    /** Liczba przycięta do [lo, hi] i zaokrąglona do 1/scale (NaN → lo). Lustro: `clamp_n` w text.rs. */
    function clampN(v, lo, hi, scale) { if (!isFinite(v)) v = lo; return Math.round(Math.min(hi, Math.max(lo, v)) * scale) / scale; }
    // znaczniki w kolejności alfabetycznej kluczy — porównanie i zapis nie zależą od kolejności (parytet z text.rs)
    const canon = (m) => Object.fromEntries(Object.keys(m).sort().map((k) => [k, m[k]]));
    const sameMarks = (a, b) => JSON.stringify(canon(a || {})) === JSON.stringify(canon(b || {}));

    /** Interlinia bloku z `style` (tylko liczba bez jednostki, 0,5–3; zaokrąglenie do 0,01) albo null. Lustro: btb-cgeditor `text::block_lh`. */
    function blockLh(style) {
        const m = /(?:^|;)\s*line-height:\s*(\d+(?:\.\d+)?)\s*(?=;|$)/i.exec(style || '');
        if (!m) return null;
        const n = Math.round(parseFloat(m[1]) * 100) / 100;
        return n >= 0.5 && n <= 3 ? n : null;
    }

    /** HTML (dowolny) → RichText v1. */
    function toRich(html) {
        const tokens = tokenize(sanitize(html));
        const blocks = [];
        let cur = null;
        const stack = [];            // znaczniki inline: { tag, marks }
        let blockAlign = null, blockLhNow = null, depthBlock = 0;
        const marksNow = () => Object.assign({}, ...stack.map((s) => s.marks));
        const ensure = () => { if (!cur) { cur = { runs: [] }; if (blockAlign) cur.align = blockAlign; if (blockLhNow) cur.lh = blockLhNow; blocks.push(cur); } return cur; };
        const push = (t) => {
            // odstępy na początku bloku/linii (między blokami, po <br>) nie tworzą treści
            if (t === '' || (/^\s*$/.test(t) && (!cur || !cur.runs.length || cur.runs[cur.runs.length - 1].br))) return;
            const b = ensure(); const m = marksNow();
            const last = b.runs[b.runs.length - 1];
            if (last && last.t !== undefined && sameMarks(last.m, m)) last.t += t;
            else { const r = { t }; if (Object.keys(m).length) r.m = canon(m); b.runs.push(r); }
        };
        for (const t of tokens) {
            if (t.type === 'text') { push(decode(t.v)); continue; }
            if (t.type === 'open') {
                if (t.name === 'br') { ensure().runs.push({ br: true }); continue; }
                if (BLOCK.has(t.name)) {
                    cur = null; depthBlock++;
                    const al = /text-align:\s*(left|center|right)/.exec(t.attrs.style || '');
                    blockAlign = al ? al[1] : null;
                    blockLhNow = blockLh(t.attrs.style);
                    stack.push({ tag: t.name, marks: marksFromTag(t.name, (t.attrs.style || '').replace(/text-align:[^;]*;?/g, '').replace(/line-height:[^;]*;?/g, ''), '') , block: true });
                    continue;
                }
                stack.push({ tag: t.name, marks: marksFromTag(t.name, t.attrs.style, t.attrs.class) });
                continue;
            }
            if (t.type === 'close') {
                const k = stack.map((s) => s.tag).lastIndexOf(t.name === 'b' ? 'strong' : t.name);
                if (k < 0) continue;
                const closed = stack.splice(k);
                if (closed.some((c) => c.block)) { cur = null; blockAlign = null; blockLhNow = null; depthBlock = Math.max(0, depthBlock - 1); }
            }
        }
        // puste bloki na końcu odrzucamy; pusty dokument = jeden pusty blok nie jest potrzebny
        while (blocks.length && blocks[blocks.length - 1].runs.length === 0) blocks.pop();
        const formatted = blocks.some((b) => b.runs.some((r) => r.m && CHAR_MARKS.some((k) => r.m[k] != null)));
        return { v: formatted ? VERSION_FORMATTED : VERSION, blocks };
    }

    function marksToStyle(m) {
        const st = [];
        if (m.color) st.push(`color: ${m.color}`);
        if (m.bg) st.push(`background-color: ${m.bg}`);
        if (m.family) st.push(`font-family: '${m.family}'`);
        if (m.size != null) st.push(`font-size: ${m.size}px`);
        if (m.weight != null && m.weight !== 700) st.push(`font-weight: ${m.weight}`);
        if (m.tracking != null) st.push(`letter-spacing: ${m.tracking}em`);
        if (m.transform && m.transform !== 'none') st.push(`text-transform: ${m.transform}`);
        if (m.padding && STYLE_RULES.padding(String(m.padding))) st.push(`padding: ${m.padding}`);
        if (m.radius && STYLE_RULES['border-radius'](String(m.radius))) st.push(`border-radius: ${m.radius}`);
        if (typeof m.opacity === 'number' && m.opacity < 1) st.push(`opacity: ${m.opacity}`);
        const tf = [];
        if (m.dx || m.dy) tf.push(`translate(${m.dx || 0}em, ${m.dy || 0}em)`);
        if (typeof m.scale === 'number' && m.scale !== 1) tf.push(`scale(${m.scale})`);
        if (m.rotate) tf.push(`rotate(${m.rotate}deg)`);
        if (m.squash != null) tf.push(`scaleX(${m.squash})`);
        if (tf.length) st.push(`transform: ${tf.join(' ')}`, 'display: inline-block', `transform-origin: ${m.squash != null ? 'left center' : 'center'}`);
        else if (st.some((x) => x.startsWith('padding:') || x.startsWith('border-radius:'))) st.push('display: inline-block');
        return st.join('; ');
    }

    /** RichText v1 → kanoniczny HTML (każdy blok to <div>, przebiegi ze znacznikami to <span>/<strong>/<em>/<u>/<s>). */
    function toHtml(rich) {
        if (!rich || !Array.isArray(rich.blocks)) return '';
        return rich.blocks.slice(0, 500).map((b) => {
            const inner = (b.runs || []).slice(0, 2000).map((r) => {
                if (r.br) return '<br>';
                let s = escText(String(r.t == null ? '' : r.t));
                const m = r.m || {};
                if (m.weight === 700) s = `<strong>${s}</strong>`;
                if (m.italic) s = `<em>${s}</em>`;
                if (m.underline) s = `<u>${s}</u>`;
                if (m.strike) s = `<s>${s}</s>`;
                const st = marksToStyle(m);
                if (st) s = `<span${m.squash != null ? ' class="cg-text-squash"' : ''} style="${st}">${s}</span>`;
                return s;
            }).join('');
            const bst = [];
            if (b.align && b.align !== 'left') bst.push(`text-align: ${b.align}`);
            const lh = typeof b.lh === 'number' && b.lh >= 0.5 && b.lh <= 3 ? Math.round(b.lh * 100) / 100 : null;
            if (lh) bst.push(`line-height: ${lh}`);
            const al = bst.length ? ` style="${bst.join('; ')}"` : '';
            return `<div${al}>${inner}</div>`;
        }).join('');
    }

    /**
     * Linie tekstu RichText: blok = jedna lub więcej linii (podział na <br>); końcowy <br> w bloku nie daje pustej linii,
     * blok złożony tylko z <br> to jedna pusta linia, blok bez przebiegów nie daje linii (jak w przeglądarce).
     */
    function richLines(rich) {
        const out = [];
        for (const b of (rich && rich.blocks) || []) {
            const segs = [''];
            let any = false;
            for (const r of b.runs || []) { if (r.br) { segs.push(''); any = true; } else if (r.t) { segs[segs.length - 1] += r.t; any = true; } }
            if (!any) continue;
            if (segs.length > 1 && segs[segs.length - 1] === '') segs.pop();
            out.push(...segs);
        }
        return out;
    }
    /** Tekst RichText (linie rozdzielone \n, bez pustych linii na końcu). */
    function richPlain(rich) { return richLines(rich).join('\n').replace(/\n+$/, ''); }
    /** Tekst bez znaczników; bloki i <br> rozdzielone znakiem nowej linii. */
    function plain(html) { return richPlain(toRich(html)); }
    /** Linie tekstu (jak `plain`, jako tablica). */
    function lines(html) { const p = plain(html); return p === '' ? [] : p.split('\n'); }

    // ── Reguły pola (Viz Control Text: Upper Case, zakresy „expose … min/max”) ─────────────────────────

    /** Zakres `{ min, max }` pola (liczby, min ≤ max) albo null. */
    function rangeOf(r) {
        if (!r || typeof r !== 'object') return null;
        const min = typeof r.min === 'number' && isFinite(r.min) ? r.min : null;
        const max = typeof r.max === 'number' && isFinite(r.max) ? r.max : null;
        if (min == null && max == null) return null;
        if (min != null && max != null && min > max) return null;
        return { min, max };
    }
    function clampTo(v, rg) { if (v == null || !rg) return v; if (rg.min != null && v < rg.min) return rg.min; if (rg.max != null && v > rg.max) return rg.max; return v; }

    /**
     * Reguły pola przy zapisie (`ocgInputs[].text`): `upperCase` (wersaliki w treści), zakresy `size` (px), `letterSpacing` (em), `lineHeight`
     * (mnożnik bloku) — wartości spoza zakresu są przycinane. Bez reguł HTML wraca bez zmian; z regułami — kanoniczny (toRich → toHtml).
     * Lustro: `text::field_rules` w btb-cgeditor, wektory `test-fixtures/text/field-rules.json`.
     */
    function fieldRules(html, rules) {
        const r = rules || {};
        const size = rangeOf(r.size), ls = rangeOf(r.letterSpacing), lh = rangeOf(r.lineHeight);
        if (!(r.upperCase === true || size || ls || lh)) return html;
        const rich = toRich(html);
        for (const b of rich.blocks) {
            if (lh && typeof b.lh === 'number') b.lh = clampTo(b.lh, lh);
            for (const run of b.runs) {
                if (r.upperCase === true && typeof run.t === 'string') run.t = run.t.toUpperCase();
                if (run.m) {
                    if (size && typeof run.m.size === 'number') run.m.size = clampTo(run.m.size, size);
                    if (ls && typeof run.m.tracking === 'number') run.m.tracking = clampTo(run.m.tracking, ls);
                }
            }
        }
        return toHtml(rich);
    }

    // ── Zamiana słów (Viz „Word Replace”, M7 T4) ──────────────────────────────────────────────────

    /**
     * Lista zamian: { "klucz": ["słowo", …], … } (klucz 1–8 znaków, bez spacji; rozróżnia wielkość liter jak w Viz). Zwraca listę znormalizowaną
     * [{ key, words }] posortowaną malejąco po długości klucza (dłuższy klucz wygrywa), bez pustych wpisów, do 50 słów na klucz.
     */
    function wordReplaceList(list) {
        const out = [];
        if (!list || typeof list !== 'object' || Array.isArray(list)) return out;
        for (const key of Object.keys(list)) {
            if (!/^[^\s]{1,8}$/u.test(key)) continue;
            const words = (Array.isArray(list[key]) ? list[key] : []).filter((w) => typeof w === 'string' && w.trim() !== '').map((w) => w.trim().slice(0, 200)).slice(0, 50);
            if (words.length) out.push({ key, words });
        }
        return out.sort((a, b) => b.key.length - a.key.length || (a.key < b.key ? -1 : a.key > b.key ? 1 : 0));
    }
    /**
     * Klucz tuż przed kursorem: `before` = tekst bieżącej linii do kursora. Klucz musi stać na początku linii albo po białym znaku lub znaku interpunkcji
     * (nie w środku słowa). Zwraca { key, words, start } (start = indeks początku klucza w `before`) albo null.
     */
    function wordReplaceFind(before, list) {
        const src = Array.from(String(before == null ? '' : before));
        for (const e of wordReplaceList(list)) {
            const k = Array.from(e.key);
            if (src.length < k.length) continue;
            const start = src.length - k.length;
            if (src.slice(start).join('') !== e.key) continue;
            const prev = start > 0 ? src[start - 1] : '';
            if (prev && /[\p{L}\p{N}]/u.test(prev)) continue;
            return { key: e.key, words: e.words, start };
        }
        return null;
    }

    /** Tekst ustawień → lista: wiersze `klucz: słowo1 | słowo2` (puste i zaczynające się od `#` pomijane; powtórzony klucz dokłada słowa). */
    function wordReplaceParse(text) {
        const out = {};
        String(text == null ? '' : text).split(/\r?\n/).forEach((line) => {
            const t = line.trim();
            if (!t || t[0] === '#') return;
            const m = /^(\S+?)\s*:\s*(.*)$/u.exec(t);
            if (!m) return;
            const words = m[2].split('|').map((w) => w.trim()).filter(Boolean);
            if (!words.length) return;
            out[m[1]] = (out[m[1]] || []).concat(words);
        });
        return wordReplaceList(out).reduce((o, e) => { o[e.key] = e.words; return o; }, {});
    }
    /** Lista → tekst ustawień (klucze w kolejności alfabetycznej, jeden wiersz na klucz). */
    function wordReplaceFormat(list) {
        return wordReplaceList(list).sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0)).map((e) => `${e.key}: ${e.words.join(' | ')}`).join('\n');
    }

    // ── Pisownia: podział tekstu na słowa (M7 T5) ─────────────────────────────────────────────────

    const isWordCh = (c) => /\p{Alphabetic}/u.test(c) || (c >= '\u0300' && c <= '\u036f');
    const isJoiner = (c) => c === "'" || c === '\u2019' || c === '-' || c === '\u2010';
    /**
     * Słowa tekstu do sprawdzania pisowni: [{ word, form, start, end }] (indeksy w znakach Unicode, `end` wyłącznie). Słowo = litery (z łącznikiem lub apostrofem
     * między literami); pomijane: jednoliterowe, przyległe do cyfry, skrót wersalikami do 4 liter (TVP, PAP). `form` = co sprawdzać: wersaliki dłuższe niż
     * 4 litery (nagłówki) sprawdzamy małymi literami. Lustro: `text::spell_tokens` w btb-cgeditor, wektory `test-fixtures/text/spell-tokens.json`.
     */
    function spellTokens(text) {
        const cs = Array.from(String(text == null ? '' : text));
        const out = [];
        let i = 0;
        while (i < cs.length) {
            if (!isWordCh(cs[i])) { i++; continue; }
            let j = i + 1;
            while (j < cs.length && (isWordCh(cs[j]) || (isJoiner(cs[j]) && j + 1 < cs.length && isWordCh(cs[j + 1])))) j++;
            const word = cs.slice(i, j).join('');
            const digit = (c) => c !== undefined && /\p{N}/u.test(c);
            if (!(digit(cs[i - 1]) || digit(cs[j])) && j - i >= 2) {
                const up = word.toUpperCase(), low = word.toLowerCase();
                let form = word;
                if (word === up && word !== low) form = (j - i) <= 4 ? null : (Array.from(low)[0].toUpperCase() + Array.from(low).slice(1).join(''));
                if (form !== null) out.push({ word, form, start: i, end: j });
            }
            i = j;
        }
        return out;
    }

    // ── Polska typografia (opcja pola) ────────────────────────────────────────────────────────────

    const NBSP = ' ';
    /**
     * Poprawia typografię polskiego tekstu w HTML (tylko w węzłach tekstowych; znaczniki bez zmian):
     *  - twarda spacja po jednoliterowych spójnikach i przyimkach (a, i, o, u, w, z) — bez „sierot” na końcu wiersza,
     *  - cudzysłowy proste " → „ ” (naprzemiennie, licznik zerowany na końcu bloku),
     *  - „...” → „…”, „ - ” (spacja, myślnik, spacja) → twarda spacja + półpauza „–” + spacja.
     * Wynik jest sanityzowany; operacja jest idempotentna.
     */
    function typo(html, opts) {
        const o = Object.assign({ pl: true }, opts);
        const src = sanitize(html);
        if (!o.pl) return src;
        let open = true;
        let prev = '';        // ostatni znak poprzedniego węzła tekstowego (kontekst dla spójników i cudzysłowów)
        const out = [];
        for (const t of tokenize(src)) {
            if (t.type === 'text') {
                let x = decode(t.v);
                x = x.replace(/\.\.\./g, '…').replace(/ - /g, NBSP + '– ');
                let res = '';
                const chars = Array.from(x);
                for (let i = 0; i < chars.length; i++) {
                    const c = chars[i], before = i ? chars[i - 1] : prev;
                    if (c === '"') { const isOpen = open; open = !open; res += isOpen ? '„' : '”'; continue; }
                    if (c === ' ' && i && /^[aiouwzAIOUWZ]$/.test(chars[i - 1]) && (i === 1 ? /^(|[\s(„"'–— ])$/.test(prev) : /[\s(„"'–— ]/.test(chars[i - 2]))) { res += NBSP; continue; }
                    res += c;
                }
                prev = res.length ? Array.from(res).pop() : prev;
                out.push(escText(res));
            } else if (t.type === 'open') {
                out.push(t.name === 'br' ? '<br>' : rebuildOpen(t));
                if (t.name === 'br') prev = '';
            } else {
                out.push(`</${t.name}>`);
                if (t.name === 'div') { open = true; prev = ''; }
            }
        }
        return out.join('');
    }
    // znaczniki po sanityzacji są już kanoniczne — odtwarzamy je z atrybutów (style/class)
    function rebuildOpen(t) {
        let attr = '';
        if (t.attrs.class) attr += ` class="${t.attrs.class}"`;
        if (t.attrs.style) attr += ` style="${t.attrs.style}"`;
        return `<${t.name}${attr}>`;
    }

    /** Czy HTML jest już kanoniczny (sanityzacja nic nie zmienia). */
    const isClean = (html) => sanitize(html) === String(html);

    return { VERSION, VERSION_FORMATTED, CHAR_MARKS, sanitize, plain, lines, toRich, toHtml, richPlain, richLines, cleanStyle, isClean, tokenize, typo, fieldRules, rangeOf, wordReplaceList, wordReplaceFind, wordReplaceParse, wordReplaceFormat, spellTokens };
});
