// E2E okien aplikacji (`vizrs artist|trio|output`): zamiast Chromium „udawana przeglądarka” (VIZRS_BROWSER),
// która zapisuje argumenty i otwiera adres z --app= w Playwright bez okna.
//   node tests/e2e/app.js [ścieżka-do-vizrs]
// Sprawdza: adres i token okna, profil i klasę okna, dołączenie drugiego okna do działającego serwera,
// --connect, to, że serwer żyje po zamknięciu okna, dopóki podłączone jest wyjście (OBS), i kończy pracę potem.
const { spawn } = require('child_process');
const fs = require('fs');
const http = require('http');
const os = require('os');
const path = require('path');

function playwrightPath() {
  const dirs = [process.env.PLAYWRIGHT_DIR, path.join(os.homedir(), 'Nextcloud/GitHub/BTB-CG-System-alpha')].filter(Boolean);
  for (const d of dirs) { try { return require.resolve('playwright', { paths: [d] }); } catch (e) { /* dalej */ } }
  return require.resolve('playwright');
}

const BIN = path.resolve(process.argv[2] || path.join(__dirname, '../../target/release/vizrs'));
const PORT = 19000 + Math.floor(Math.random() * 500);
const TOKEN = 'e2e-app-token';
let failures = 0;
const ok = (cond, msg, info) => { console.log((cond ? 'PASS ' : 'FAIL ') + msg + (!cond && info !== undefined ? ' → ' + JSON.stringify(info) : '')); if (!cond) failures++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-app-e2e-'));
const data = path.join(tmp, 'data');
const LOG = path.join(tmp, 'browser.log');
const FAKE = path.join(tmp, 'fake-chromium');
fs.writeFileSync(FAKE, `#!/usr/bin/env node
const fs = require('fs');
const args = process.argv.slice(2);
fs.appendFileSync(${JSON.stringify(LOG)}, JSON.stringify({ args }) + '\\n');
const url = (args.find((a) => a.startsWith('--app=')) || '').slice(6);
(async () => {
  const { chromium } = require(${JSON.stringify(playwrightPath())});
  const b = await chromium.launch();
  const p = await b.newPage();
  const errs = [];
  p.on('pageerror', (e) => errs.push(e.message));
  await p.goto(url);
  await new Promise((r) => setTimeout(r, +(process.env.FAKE_MS || 3000)));
  const info = { url, title: await p.title(), conn: await p.evaluate(() => document.querySelector('#conn')?.textContent || null), errs };
  fs.appendFileSync(${JSON.stringify(LOG)}, JSON.stringify(info) + '\\n');
  await b.close();
})().catch((e) => { fs.appendFileSync(${JSON.stringify(LOG)}, JSON.stringify({ error: String(e) }) + '\\n'); process.exit(1); });
`);
fs.chmodSync(FAKE, 0o755);

const log = () => (fs.existsSync(LOG) ? fs.readFileSync(LOG, 'utf8').trim().split('\n').filter(Boolean).map((l) => JSON.parse(l)) : []);
const run = (args, env = {}) => {
  const p = spawn(BIN, args, { env: { ...process.env, VIZRS_BROWSER: FAKE, XDG_CONFIG_HOME: path.join(tmp, 'cfg'), ...env } });
  p.out = '';
  p.stdout.on('data', (d) => (p.out += d));
  p.stderr.on('data', (d) => (p.out += d));
  p.done = new Promise((r) => p.on('exit', (code) => r(code)));
  return p;
};
// przeglądarkę uruchamia proces, który może się już zakończyć – jej wpis pojawia się chwilę później
const waitLog = async (pred, ms = 5000) => { for (let t = 0; t < ms; t += 100) { const hit = log().find(pred); if (hit) return hit; await sleep(100); } return null; };
const exited = (p, ms) => Promise.race([p.done, sleep(ms).then(() => 'działa')]);

(async () => {
  const procs = [];
  try {
    // 1. Artist: serwer w tym procesie + okno
    const artist = run(['artist', '--port', String(PORT), '--tcp', '0', '--data', data, '--token', TOKEN, '--linger', '1'], { FAKE_MS: '7000' });
    procs.push(artist);
    await sleep(2500);
    const a0 = log().find((l) => l.args);
    ok(!!a0, 'artist: przeglądarka uruchomiona', artist.out);
    const arg = (pre) => a0?.args.find((x) => x.startsWith(pre));
    ok(arg('--app=') === `--app=http://127.0.0.1:${PORT}/artist?token=${TOKEN}`, 'artist: adres okna z tokenem', arg('--app='));
    ok((arg('--user-data-dir=') || '').endsWith(path.join('vizrs', 'browser')), 'artist: własny profil przeglądarki', arg('--user-data-dir='));
    ok(arg('--class=') === '--class=VizRS-Artist', 'artist: klasa okna');
    ok(!artist.out.includes(TOKEN), 'artist: token nie trafia do konsoli', artist.out);
    ok(fs.existsSync(path.join(data, '.vizrs.lock')), 'artist: blokada katalogu danych');

    // 2. Trio na tym samym --data dołącza do działającego serwera
    const trio = run(['trio', '--data', data, '--token', TOKEN], { FAKE_MS: '1500' });
    procs.push(trio);
    const tcode = await exited(trio, 5000);
    ok(tcode === 0 && trio.out.includes('już działa'), 'trio: dołącza do działającego serwera i kończy się od razu', [tcode, trio.out]);
    ok(!!(await waitLog((l) => l.args && l.args.includes(`--app=http://127.0.0.1:${PORT}/trio?token=${TOKEN}`))), 'trio: okno na serwerze z artist');

    // 3. „OBS” podłączony do wyjścia – serwer musi przeżyć zamknięcie okna
    const obs = http.get(`http://127.0.0.1:${PORT}/events?token=${TOKEN}`);
    obs.on('error', () => {});
    await sleep(6000); // okno Artist zamyka się po 7 s
    const infos = log().filter((l) => l.title);
    const ai = infos.find((l) => l.url.includes('/artist'));
    ok(ai && ai.title === 'VizRS Artist' && ai.conn === 'połączono' && ai.errs.length === 0, 'artist: strona załadowana, wskaźnik połączenia', ai);
    const ti = infos.find((l) => l.url.includes('/trio'));
    ok(ti && ti.title === 'VizRS Trio' && ti.errs.length === 0, 'trio: strona załadowana', ti);
    ok(artist.out.includes('Okno zamknięte'), 'artist: proces zauważył zamknięcie okna', artist.out);
    ok((await exited(artist, 8000)) === 'działa', 'serwer działa dalej, dopóki podłączone jest wyjście (OBS)');
    obs.destroy();
    const code = await exited(artist, 30000);
    ok(code === 0, 'po odłączeniu wyjścia serwer kończy pracę', [code, artist.out]);
    ok(!fs.existsSync(path.join(data, '.vizrs.lock')), 'blokada zwolniona po zakończeniu');

    // 4. --connect: okno dla serwera zdalnego, bez serwera lokalnego
    const remote = spawn(BIN, ['--port', String(PORT + 1), '--tcp', '0', '--data', path.join(tmp, 'remote')], { stdio: 'ignore' });
    procs.push(remote);
    await sleep(600);
    const con = run(['output', '--connect', `http://127.0.0.1:${PORT + 1}/`, '--query', 'mode=key', '--fullscreen', '--position', '1920,0', '--data', path.join(tmp, 'nieuzywany')], { FAKE_MS: '1000' });
    procs.push(con);
    ok((await exited(con, 5000)) === 0, '--connect: okno otwarte, proces kończy się od razu', con.out);
    const c0 = (await waitLog((l) => l.args && l.args.some((a) => a.includes('/output')))) || { args: [] };
    ok(c0.args.includes(`--app=http://127.0.0.1:${PORT + 1}/output?mode=key`), '--connect: adres serwera zdalnego z parametrami', c0.args);
    ok(c0.args.includes('--kiosk') && c0.args.includes('--window-position=1920,0') && c0.args.some((a) => a.endsWith(path.join('vizrs', 'browser-output'))), 'output: pełny ekran, pozycja, osobny profil', c0.args);
    ok(!fs.existsSync(path.join(tmp, 'nieuzywany')), '--connect: brak lokalnego katalogu danych');
    await sleep(2000);
  } catch (e) {
    console.error(e);
    failures++;
  } finally {
    for (const p of procs) p.kill();
    await sleep(300);
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  console.log(failures ? `\n${failures} NIEUDANYCH` : '\nWszystkie testy okien zaliczone');
  process.exit(failures ? 1 : 0);
})();
