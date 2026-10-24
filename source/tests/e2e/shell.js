// E2E okna natywnego vizrs-shell (WebKitGTK / WebView2 / WKWebView):
//   node tests/e2e/shell.js [ścieżka-do-vizrs] [ścieżka-do-vizrs-shell]
// Sprawdza: Artist i Trio ładują się w WebView i łączą z serwerem; powłoka uruchamia `vizrs serve --idle-exit`,
// który po zamknięciu okna kończy pracę sam (gdy nie ma wyjść), a dopóki OBS jest podłączony – działa;
// drugie okno dołącza do działającego serwera; okno „Połącz z serwerem” przełącza Trio na serwer zdalny. Otwiera na kilka sekund widoczne okna (w CI: xvfb-run).
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

    // 2. Trio dołącza do tego samego serwera (ta sama instancja). Podłączone „wyjście” trzyma serwer:
    //    start WebKitGTK na runnerze CI trwa dłużej niż --idle-exit (10 s) i serwer kończył się w trakcie.
    //    probe() używa spawnSync (blokuje pętlę zdarzeń), więc najpierw czekamy na odpowiedź /events.
    obs = await new Promise((res) => {
      const req = http.get(`http://127.0.0.1:${PORT}/events`, () => res(req));
      req.on('error', () => res(req));
    });
    const t = probe('trio');
    ok(t.title === 'VizRS Trio' && t.conn === 'połączono' && t.pages > 0, 'Trio w oknie natywnym dołącza do działającego serwera', t);
    const h2 = await health();
    ok(h2 && h1 && h2.instance === h1.instance, 'to ten sam serwer (instance)', [h1?.instance, h2?.instance]);

    // 3. „OBS” (podłączony w kroku 2) – serwer żyje dłużej niż --idle-exit; po odłączeniu kończy pracę
    await sleep(14000);
    ok(!!(await health()), 'z podłączonym wyjściem serwer działa dalej');
    obs.destroy();
    let gone = false;
    for (let i = 0; i < 40 && !gone; i++) { await sleep(1000); gone = !(await health()); }
    ok(gone, 'bez okien i wyjść serwer kończy pracę sam');
    ok(!fs.existsSync(path.join(data, '.vizrs.lock')), 'blokada zwolniona');

    // 4. Plik → Połącz z serwerem: zły token → komunikat, dobry → Trio sterujące serwerem „zdalnym”
    const RPORT = PORT + 1, TOKEN = 'zdalny-token-1234567', cfg = path.join(tmp, 'cfg');
    const remote = spawn(BIN, ['--port', String(RPORT), '--tcp', '0', '--data', path.join(tmp, 'remote'), '--token', TOKEN], { stdio: 'ignore' });
    try {
      await sleep(800);
      const CPROBE = path.join(tmp, 'connect.js');
      fs.writeFileSync(CPROBE, `(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const $ = (id) => document.getElementById(id);
  if (document.title === 'Połącz z serwerem') {
    $('url').value = 'http://127.0.0.1:${RPORT}'; $('token').value = 'zly-token-12345678'; $('go').click();
    for (let i = 0; i < 50 && !$('err').textContent; i++) await sleep(100);
    $('token').value = '${TOKEN}'; $('keep').checked = true; $('go').click();
    return;
  }
  // strona serwera nie może przełączyć okien na inny adres
  window.ipc.postMessage(JSON.stringify({ action: 'connect', url: 'http://127.0.0.1:1' }));
  for (let i = 0; i < 50 && document.querySelector('#conn')?.textContent !== 'połączono'; i++) await sleep(100);
  window.ipc.postMessage('probe:' + JSON.stringify({ href: location.href, title: document.title, conn: document.querySelector('#conn')?.textContent, pages: document.querySelectorAll('#pages .item').length }));
})();`);
      const out = path.join(tmp, 'connect.json');
      spawnSync(SHELL, ['connect', '--data', path.join(tmp, 'local'), '--port', String(PORT + 2), '--vizrs', BIN, '--probe', CPROBE, '--probe-out', out], { encoding: 'utf8', timeout: 60000, env: { ...process.env, VIZRS_CONFIG: cfg } });
      const c = fs.existsSync(out) ? JSON.parse(fs.readFileSync(out, 'utf8')) : {};
      ok(c.title?.startsWith('VizRS Trio') && c.conn === 'połączono' && c.pages > 0 && c.href?.startsWith(`http://127.0.0.1:${RPORT}/trio`), 'okno połączenia przełącza Trio na serwer zdalny z tokenem', c);
      const saved = JSON.parse(fs.readFileSync(path.join(cfg, 'connections.json'), 'utf8'));
      ok(saved.servers[0]?.url === `http://127.0.0.1:${RPORT}` && saved.servers[0]?.token === TOKEN, 'serwer i token na liście zapisanych', saved);
      if (process.platform !== 'win32') ok((fs.statSync(path.join(cfg, 'connections.json')).mode & 0o077) === 0, 'lista zapisanych tylko dla właściciela');
    } finally {
      remote.kill();
    }
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
