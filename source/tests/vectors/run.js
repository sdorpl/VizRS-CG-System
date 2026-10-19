// Wspólne wektory testowe modelu tekstu (F4.1): ten sam zestaw JSON co w BTB-CG (Node) i btb-cgeditor (Rust).
//   node tests/vectors/run.js        – sprawdza web/text-model.js (kopia z BTB-CG) względem wektorów
// Wektory i kopię odświeża tools/sync-btb.sh; rozjazd z BTB wykrywa `tools/sync-btb.sh --check`.
const fs = require('fs');
const path = require('path');
const T = require('../../web/text-model.js');
const load = (n) => JSON.parse(fs.readFileSync(path.join(__dirname, n + '.json'), 'utf8'));
let failed = 0, passed = 0;
const eq = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const check = (name, ok, info) => { if (ok) passed++; else { failed++; console.log('FAIL ' + name + (info !== undefined ? ' → ' + JSON.stringify(info) : '')); } };

const m = load('model');
m.sanitize.forEach((c, i) => check('sanitize #' + i, T.sanitize(c.in) === c.out, T.sanitize(c.in)));
m.sanitizeKeepP.forEach((c, i) => check('sanitizeKeepP #' + i, T.sanitize(c.in, { blocks: 'keep' }) === c.out));
m.plain.forEach((c, i) => check('plain #' + i, T.plain(c.in) === c.out));
m.lines.forEach((c, i) => check('lines #' + i, eq(T.lines(c.in), c.out)));
m.rich.forEach((c, i) => check('toRich #' + i, eq(T.toRich(c.in), c.out)));
m.html.forEach((c, i) => check('toHtml #' + i, T.toHtml(c.rich) === c.out));
m.typo.forEach((c, i) => { check('typo #' + i, T.typo(c.in) === c.out); check('typo idempotentne #' + i, T.typo(c.out) === c.out); });

load('field-rules').cases.forEach((c) => check('fieldRules: ' + c.name, T.fieldRules(c.html, c.rules) === c.expect, T.fieldRules(c.html, c.rules)));

const wr = load('word-replace');
wr.cases.forEach((c) => check('wordReplaceFind: ' + c.name, eq(T.wordReplaceFind(c.before, c.list), c.expect)));
wr.parse.forEach((c) => {
  const list = T.wordReplaceParse(c.text);
  check('wordReplaceParse: ' + c.name, eq(list, c.expectList) && T.wordReplaceFormat(list) === c.expectText);
});

load('spell-tokens').cases.forEach((c) => check('spellTokens: ' + JSON.stringify(c.text).slice(0, 30), eq(T.spellTokens(c.text), c.expect)));

console.log(failed ? `\n${failed} NIEUDANYCH (zaliczone: ${passed})` : `Wektory zgodne (${passed} sprawdzeń)`);
process.exit(failed ? 1 : 0);
