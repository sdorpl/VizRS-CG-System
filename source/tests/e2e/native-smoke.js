// Test dymny okna natywnego na prawdziwym systemie (CI: Windows z instalatora, macOS z budowy; lokalnie Linux pod xvfb-run):
//   node tests/e2e/native-smoke.js <vizrs> <vizrs-shell> [--installed]
// --installed: programy z instalatora (np. C:\Program Files\VizRS) uruchamiane jak skrót „VizRS Trio” – bez --data i --vizrs.
// Sprawdza: okno Trio wstaje (WebView2 / WKWebView / WebKitGTK), łączy się z serwerem uruchomionym przez okno, most okna
// (menu systemowe), okno pytania zamiast confirm(), czcionka systemu; Windows – dane WebView2 w %LOCALAPPDATA%\VizRS, nie obok programu.
// Bez Playwright: wynik podaje skrypt --probe wykonany w oknie.
const { spawnSync } = require('child_process');
const fs = require('fs');
const http = require('http');
const os = require('os');
const path = require('path');

const BIN = path.resolve(process.argv[2] || '');
const SHELL = path.resolve(process.argv[3] || '');
const INSTALLED = process.argv.includes('--installed');
const PORT = 19900 + Math.floor(Math.random() * 90);
let failures = 0;
const ok = (cond, msg, info) => { console.log((cond ? 'PASS ' : 'FAIL ') + msg + (!cond && info !== undefined ? ' → ' + JSON.stringify(info) : '')); if (!cond) failures++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-smoke-'));

const PROBE = path.join(tmp, 'probe.js');
fs.writeFileSync(PROBE, `(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const r = {};
  try {
    for (let i = 0; i < 300 && (document.querySelector('#conn')?.textContent !== 'połączono' || !document.querySelector('#pages .item')); i++) await sleep(100);
    await sleep(500);
    Object.assign(r, { title: document.title, conn: document.querySelector('#conn')?.textContent, pages: document.querySelectorAll('#pages .item').length,
      shell: window.__vizrsShell || null, native: document.documentElement.classList.contains('native'), font: getComputedStyle(document.body).fontFamily });
    document.querySelector('#bClear').click();
    for (let i = 0; i < 30 && !document.querySelector('.dlg'); i++) await sleep(100);
    r.dialog = document.querySelector('.dlg-text')?.textContent || null;
    dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await sleep(200);
    r.dialogClosed = !document.querySelector('.dlg');
  } catch (e) { r.error = String(e); }
  window.ipc.postMessage('probe:' + JSON.stringify(r));
})();`);

const health = (port) => new Promise((res) => {
  const req = http.get(`http://127.0.0.1:${port}/health`, (r) => { let b = ''; r.on('data', (c) => (b += c)); r.on('end', () => { try { res(JSON.parse(b)); } catch { res(null); } }); });
  req.on('error', () => res(null));
  req.setTimeout(1500, () => { req.destroy(); res(null); });
});

(async () => {
  try {
    ok(fs.existsSync(BIN) && fs.existsSync(SHELL), 'programy istnieją', [BIN, SHELL]);
    const out = path.join(tmp, 'probe.json');
    const args = ['trio', '--port', String(PORT), '--probe', PROBE, '--probe-out', out];
    // katalog danych: podany albo domyślny okna (Windows bez --data: %LOCALAPPDATA%\VizRS, jak skrót instalatora)
    const data = INSTALLED ? (process.platform === 'win32' ? path.join(process.env.LOCALAPPDATA || '', 'VizRS') : null) : path.join(tmp, 'data');
    if (!INSTALLED) args.push('--data', data, '--vizrs', BIN);
    const t0 = Date.now();
    // wyjście okna do pliku, nie do potoku: serwer uruchomiony przez okno mógłby odziedziczyć uchwyt potoku (Windows)
    // i spawnSync czekałby na jego zakończenie zamiast na zamknięcie okna
    const logPath = path.join(tmp, 'okno.log'), log = fs.openSync(logPath, 'w');
    // jak skrót: katalog roboczy = katalog programu
    const run = spawnSync(SHELL, args, { cwd: path.dirname(SHELL), stdio: ['ignore', log, log], timeout: 150000 });
    fs.closeSync(log);
    const stderr = fs.readFileSync(logPath, 'utf8').split('\n').filter((l) => l.trim() && !/libEGL|Gtk-WARNING|^\s*$/.test(l)).join('\n');
    const r = fs.existsSync(out) ? JSON.parse(fs.readFileSync(out, 'utf8')) : null;
    console.log(`okno: ${((Date.now() - t0) / 1000).toFixed(1)} s, kod ${run.status}${stderr ? ', komunikaty: ' + stderr.trim().slice(0, 400) : ''}`);
    ok(!!r && !r.error, 'okno Trio wstaje i wykonuje skrypt w WebView', r || { status: run.status, signal: run.signal, error: run.error?.message });
    if (r) {
      ok(r.title?.startsWith('VizRS Trio') && r.conn === 'połączono' && r.pages > 0, 'Trio łączy się z serwerem uruchomionym przez okno i pokazuje strony', r);
      ok(r.native && r.shell?.menu === true, 'most okna natywnego (menu systemowe pod prawym przyciskiem)', r.shell);
      ok(/Zdjąć wszystkie grafiki/.test(r.dialog || '') && r.dialogClosed, 'okno pytania aplikacji (nie confirm przeglądarki), Esc zamyka', r);
      ok(/system-ui/.test(r.font || ''), 'czcionka systemu w oknie natywnym', r.font);
    }
    if (process.platform === 'win32') {
      const local = process.env.LOCALAPPDATA;
      ok(!!local && fs.existsSync(path.join(local, 'VizRS', 'WebView2')), 'WebView2 trzyma dane w %LOCALAPPDATA%\\VizRS\\WebView2');
      ok(!fs.existsSync(SHELL + '.WebView2'), 'brak danych WebView2 obok programu (Program Files bez prawa zapisu)');
      if (INSTALLED && local) ok(fs.existsSync(path.join(local, 'VizRS', 'show.json')) || fs.existsSync(path.join(local, 'VizRS', '.vizrs.lock')), 'bez --data: dane w %LOCALAPPDATA%\\VizRS');
    }
    // serwer uruchomiony przez okno: adres z jego blokady katalogu danych; po zamknięciu okna działa jeszcze --idle-exit (10 s),
    // potem kończy pracę sam i zwalnia blokadę
    if (data) {
      const lockFile = path.join(data, '.vizrs.lock');
      let lock = null;
      try { lock = JSON.parse(fs.readFileSync(lockFile, 'utf8')); } catch { /* brak blokady */ }
      const port = Number(String(lock?.addr || '').split(':').pop());
      const h = port ? await health(port) : null;
      ok(!!h && h.instance === lock.instance, 'serwer uruchomiony przez okno działa po jego zamknięciu (czeka --idle-exit)', { lock, h });
      let gone = false;
      for (let i = 0; i < 40 && !gone && port; i++) { await sleep(1000); gone = !(await health(port)); }
      ok(gone, 'serwer kończy pracę sam, gdy nie ma okien ani wyjść');
      for (let i = 0; i < 10 && fs.existsSync(lockFile); i++) await sleep(300);
      ok(!fs.existsSync(lockFile), 'serwer zwolnił blokadę katalogu danych');
    }
  } catch (e) {
    console.error(e);
    failures++;
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  console.log(failures ? `\n${failures} NIEUDANYCH` : '\nTest dymny okna natywnego zaliczony');
  process.exit(failures ? 1 : 0);
})();
