// E2E okna natywnego vizrs-shell (WebKitGTK / WebView2 / WKWebView):
//   node tests/e2e/shell.js [ścieżka-do-vizrs] [ścieżka-do-vizrs-shell]
// Sprawdza: Artist i Trio ładują się w WebView i łączą z serwerem; powłoka uruchamia `vizrs serve --idle-exit`,
// który po zamknięciu okna kończy pracę sam (gdy nie ma wyjść), a dopóki OBS jest podłączony – działa;
// drugie okno dołącza do działającego serwera. Otwiera na kilka sekund widoczne okna (w CI: xvfb-run).
const { spawn, spawnSync } = require('child_process');
const fs = require('fs');
const http = require('http');
const os = require('os');
const path = require('path');

const BIN = path.resolve(process.argv[2] || path.join(__dirname, '../../target/release/vizrs'));
const SHELL = path.resolve(process.argv[3] || path.join(__dirname, '../../shell/target/release/vizrs-shell'));
const PORT = 19600 + Math.floor(Math.random() * 300);
let failures = 0;
const ok = (cond, msg, info) => { console.log((cond ? 'PASS ' : 'FAIL ') + msg + (!cond && info !== undefined ? ' → ' + JSON.stringify(info) : '')); if (!cond) failures++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-shell-e2e-'));
const data = path.join(tmp, 'data');

// sonda: czeka na połączenie z serwerem, zwraca tytuł, wskaźnik i liczbę scen/stron
const PROBE = path.join(tmp, 'probe.js');
fs.writeFileSync(PROBE, `(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  for (let i = 0; i < 50 && document.querySelector('#conn')?.textContent !== 'połączono'; i++) await sleep(100);
  await sleep(500);
  const r = { title: document.title, conn: document.querySelector('#conn')?.textContent, scenes: document.querySelectorAll('#sceneSel option').length, pages: document.querySelectorAll('#pages .item').length };
  window.ipc.postMessage('probe:' + JSON.stringify(r));
})();`);
const health = () => new Promise((res) => {
  http.get(`http://127.0.0.1:${PORT}/health`, (r) => { let b = ''; r.on('data', (c) => (b += c)); r.on('end', () => res(JSON.parse(b))); }).on('error', () => res(null));
});
const probe = (kind) => {
  const out = path.join(tmp, kind + '.json');
  const r = spawnSync(SHELL, [kind, '--data', data, '--port', String(PORT), '--vizrs', BIN, '--probe', PROBE, '--probe-out', out], { encoding: 'utf8', timeout: 60000 });
  return fs.existsSync(out) ? JSON.parse(fs.readFileSync(out, 'utf8')) : { error: r.status, stderr: r.stderr };
};

(async () => {
  let obs;
  try {
    // 1. Artist: powłoka uruchamia serwer, okno łączy się z nim
    const a = probe('artist');
    ok(a.title === 'VizRS Artist' && a.conn === 'połączono' && a.scenes > 0, 'Artist w oknie natywnym: strona, połączenie, sceny', a);
    const h1 = await health();
    ok(!!h1, 'serwer uruchomiony przez powłokę działa po zamknięciu okna (czeka na --idle-exit)');
    ok(fs.existsSync(path.join(data, '.vizrs.lock')), 'serwer trzyma blokadę katalogu danych');

    // 2. Trio dołącza do tego samego serwera (ta sama instancja)
    const t = probe('trio');
    ok(t.title === 'VizRS Trio' && t.conn === 'połączono' && t.pages > 0, 'Trio w oknie natywnym dołącza do działającego serwera', t);
    const h2 = await health();
    ok(h2 && h1 && h2.instance === h1.instance, 'to ten sam serwer (instance)', [h1?.instance, h2?.instance]);

    // 3. „OBS” podłączony – serwer żyje dłużej niż --idle-exit; po odłączeniu kończy pracę
    obs = http.get(`http://127.0.0.1:${PORT}/events`);
    obs.on('error', () => {});
    await sleep(14000);
    ok(!!(await health()), 'z podłączonym wyjściem serwer działa dalej');
    obs.destroy();
    let gone = false;
    for (let i = 0; i < 40 && !gone; i++) { await sleep(1000); gone = !(await health()); }
    ok(gone, 'bez okien i wyjść serwer kończy pracę sam');
    ok(!fs.existsSync(path.join(data, '.vizrs.lock')), 'blokada zwolniona');
  } catch (e) {
    console.error(e);
    failures++;
  } finally {
    obs?.destroy();
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  console.log(failures ? `\n${failures} NIEUDANYCH` : '\nWszystkie testy okna natywnego zaliczone');
  process.exit(failures ? 1 : 0);
})();
