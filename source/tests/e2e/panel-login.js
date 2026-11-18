// E2E logowania do Trio i Artist kontami panelu VizRS CG Playout (opcja --panel-login) z udawanym panelem:
// przełącznik w panelu wyłączony = tylko token; włączony – login i hasło, sesja konta, wejście z ciasteczkiem panelu,
// koniec sesji po wyłączeniu przełącznika (sprawdzenie co minutę – tu skrócone przez świeże logowanie), panel nieosiągalny.
//   node tests/e2e/panel-login.js [ścieżka-do-vizrs]
const { spawn } = require('child_process');
const fs = require('fs');
const http = require('http');
const os = require('os');
const path = require('path');

function loadPlaywright() {
  const dirs = [process.env.PLAYWRIGHT_DIR].filter(Boolean);
  for (const d of dirs) { try { return require(require.resolve('playwright', { paths: [d] })); } catch (e) { /* dalej */ } }
  return require('playwright');
}

const BIN = path.resolve(process.argv[2] || path.join(__dirname, '../../target/release/vizrs'));
const PORT = 20000 + Math.floor(Math.random() * 1000);
const PANEL = PORT + 1000;
const B = `http://127.0.0.1:${PORT}`;
const TOKEN = 'e2e-token-1';
const SID = 'a'.repeat(64), BROWSER_SID = 'b'.repeat(64);
let failures = 0;
const ok = (cond, msg, info) => { console.log((cond ? 'PASS ' : 'FAIL ') + msg + (!cond && info !== undefined ? ' → ' + JSON.stringify(info) : '')); if (!cond) failures++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// udawany panel: konto ola / haslo-oli-1; sesje SID (z verify) i BROWSER_SID (przeglądarka panelu)
const fake = { enabled: false, verified: [] };
const panel = http.createServer((req, res) => {
  let body = '';
  req.on('data', (c) => (body += c));
  req.on('end', () => {
    const send = (code, obj) => { res.writeHead(code, { 'Content-Type': 'application/json' }); res.end(JSON.stringify(obj)); };
    if (req.url.startsWith('/trio')) { res.writeHead(200, { 'Content-Type': 'text/html' }); return res.end('<p>zdalny</p>'); }
    if (req.url === '/api/sso') return send(200, { ok: true, result: { available: true, enabled: fake.enabled } });
    if (!fake.enabled) return send(403, { ok: false, error: 'wyłączone' });
    if (req.url === '/api/sso/verify' && req.method === 'POST') {
      if (req.headers['x-vizrs-live'] !== '1') return send(403, { ok: false });
      const b = JSON.parse(body || '{}');
      fake.verified.push(b.user);
      if (b.user !== 'ola' || b.password !== 'haslo-oli-1') return send(401, { ok: false, error: 'zły login albo hasło' });
      return send(200, { ok: true, result: { user: { name: 'ola', role: 'operator' }, session: SID } });
    }
    if (req.url === '/api/sso/session') {
      const c = req.headers.cookie || '';
      if (c === `vizrs_live=${SID}` || c === `vizrs_live=${BROWSER_SID}`) return send(200, { ok: true, result: { user: { name: 'ola', role: 'operator' } } });
      return send(401, { ok: false });
    }
    send(404, { ok: false });
  });
});

(async () => {
  const { chromium } = loadPlaywright();
  await new Promise((r) => panel.listen(PANEL, '127.0.0.1', r));
  const data = fs.mkdtempSync(path.join(os.tmpdir(), 'vizrs-e2e-pl-'));
  const srv = spawn(BIN, ['--port', String(PORT), '--tcp', '0', '--data', data, '--token', TOKEN, '--panel-login', `http://127.0.0.1:${PANEL}`], { stdio: 'ignore' });
  let browser;
  try {
    for (let i = 0; i < 100; i++) { try { if ((await fetch(B + '/health', { headers: { 'X-Token': TOKEN } })).ok) break; } catch { /* jeszcze nie */ } await sleep(100); }
    ok((await (await fetch(B + '/login/info')).json()).panel === false, 'przełącznik w panelu wyłączony: /login/info panel=false');
    browser = await chromium.launch();
    const p = await browser.newPage();
    await p.goto(B + '/trio');
    await p.waitForSelector('#tok input[name=token]');
    await sleep(300);
    ok(await p.isHidden('#acc'), 'ekran logowania bez kont panelu, gdy przełącznik wyłączony');
    // pole serwera: domyślnie ten serwer; zły adres – komunikat; inny serwer – przejście na jego /trio z tokenem
    ok(await p.inputValue('#server') === B, 'pole serwera: bieżący serwer', await p.inputValue('#server'));
    await p.fill('#server', 'ftp://zly');
    await p.fill('#token', 'x');
    await p.click('#tok button');
    await p.waitForSelector('#bad:not([hidden])');
    ok(/Zły adres serwera/.test(await p.textContent('#bad')), 'pole serwera: zły adres – komunikat');
    await p.fill('#server', `127.0.0.1:${PANEL}`.replace(/^/, 'http://'));
    await p.fill('#token', 'tok-zdalny');
    await Promise.all([p.waitForURL(`http://127.0.0.1:${PANEL}/trio?token=tok-zdalny`), p.click('#tok button')]);
    ok(true, 'pole serwera: inny serwer – przejście na jego /trio z tokenem');
    await p.goto(B + '/trio');
    await p.waitForSelector('#tok input[name=token]');
    const login = (user, password) => fetch(B + '/login', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ user, password }) });
    ok((await login('ola', 'haslo-oli-1')).status === 401, 'logowanie kontem odrzucone, gdy panel nie pozwala');

    fake.enabled = true;
    await sleep(10500); // VizRS pamięta stan przełącznika 10 s
    await p.goto(B + '/trio');
    await p.waitForSelector('#acc:not([hidden])');
    ok(true, 'przełącznik włączony: login i hasło na ekranie logowania');
    await p.fill('#acc-user', 'ola');
    await p.fill('#acc-pass', 'zle');
    await p.click('#acc-go');
    await p.waitForSelector('#acc-bad:not([hidden])');
    ok(/zły login albo hasło/.test(await p.textContent('#acc-bad')), 'złe hasło: komunikat');
    await p.fill('#acc-pass', 'haslo-oli-1');
    await p.press('#acc-pass', 'Enter');
    await p.waitForSelector('#pages .item');
    ok(true, 'login i hasło konta panelu otwierają Trio');
    const cookies = (await p.context().cookies(B)).map((c) => c.name);
    ok(cookies.includes('vizrs_session') && !cookies.includes('vizrs_token'), 'ciasteczko sesji konta, bez tokenu', cookies);
    ok(await p.evaluate(async () => (await fetch('/api/state')).status) === 200, 'API z sesją konta');
    await p.goto(B + '/artist');
    await p.waitForTimeout(500);
    ok(await p.evaluate(async () => (await fetch('/api/show')).status) === 200, 'ta sama sesja otwiera Artist');
    const log = fs.readFileSync(path.join(data, 'log.jsonl'), 'utf8');
    ok(/logowanie kontem panelu: ola/.test(log), 'log audytowy: logowanie kontem');
    ok(!/haslo-oli-1/.test(log), 'hasło nie trafia do logu');

    // zalogowany w panelu pod tym samym hostem: wejście bez logowania
    const c2 = await browser.newContext();
    await c2.addCookies([{ name: 'vizrs_live', value: BROWSER_SID, url: B }]);
    const p2 = await c2.newPage();
    await p2.goto(B + '/trio');
    await p2.waitForSelector('#pages .item');
    ok(true, 'ciasteczko sesji panelu – Trio bez ponownego logowania');
    const c3 = await browser.newContext();
    await c3.addCookies([{ name: 'vizrs_live', value: 'c'.repeat(64), url: B }]);
    const p3 = await c3.newPage();
    await p3.goto(B + '/trio');
    await p3.waitForSelector('#tok input[name=token]');
    ok(true, 'nieważna sesja panelu – ekran logowania');

    // token działa dalej
    ok((await fetch(B + '/api/state', { headers: { Authorization: `Bearer ${TOKEN}` } })).status === 200, 'token serwera działa dalej');
    // wylogowanie kończy sesję konta
    await p.evaluate(async () => fetch('/logout', { method: 'POST' }));
    ok(await p.evaluate(async () => (await fetch('/api/state')).status) === 401, 'po /logout sesja konta nie działa');
    // przycisk Wyloguj w Trio kasuje też zapamiętany token operatora
    await p.goto(`${B}/trio?token=${TOKEN}`);
    await p.waitForSelector('#pages .item');
    ok((await p.context().cookies(B)).some((c) => c.name === 'vizrs_token'), 'token operatora zapamiętany w ciasteczku');
    await p.evaluate(() => document.getElementById('bLogout').click());
    await p.click('.dlg-btns button.acc');
    await p.waitForSelector('#tok input[name=token]');
    ok(!(await p.context().cookies(B)).some((c) => c.name === 'vizrs_token' || c.name === 'vizrs_session'), 'Wyloguj w Trio: bez tokenu i sesji, ekran logowania');
  } catch (e) {
    ok(false, 'wyjątek: ' + e.message);
  } finally {
    if (browser) await browser.close();
    srv.kill();
    panel.close();
    fs.rmSync(data, { recursive: true, force: true });
  }
  console.log(failures ? `${failures} błędów` : 'Wszystkie testy logowania kontami panelu zaliczone');
  process.exit(failures ? 1 : 0);
})();
