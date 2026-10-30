//! VizRS – lekki klon duetu Viz Artist (projektowanie szablonów CG)
//! + Viz Trio (playout stron) z wyjściem HTML dla OBS Studio (Browser Source).
//!
//! Zero zależności zewnętrznych: własny HTTP/1.1, SSE i JSON.

mod app;
mod audit;
mod backup;
mod client;
mod config;
mod datalock;
mod engine;
mod fonts;
mod fonts_data;
mod guard;
mod hook;
mod http;
mod json;
mod osc;
mod shutdown;
mod ticker;
mod validate;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use engine::Engine;
use http::{read_request, Request, Response};
use json::Json;

struct Static {
    path: &'static str,
    ctype: &'static str,
    body: &'static str,
}

const STATIC: &[Static] = &[
    Static { path: "/", ctype: "text/html; charset=utf-8", body: include_str!("../web/index.html") },
    Static { path: "/artist", ctype: "text/html; charset=utf-8", body: include_str!("../web/artist.html") },
    Static { path: "/trio", ctype: "text/html; charset=utf-8", body: include_str!("../web/trio.html") },
    Static { path: "/output", ctype: "text/html; charset=utf-8", body: include_str!("../web/output.html") },
    Static { path: "/renderer.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/renderer.js") },
    Static { path: "/richtext.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/richtext.js") },
    Static { path: "/btb-io.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/btb-io.js") },
    Static { path: "/cgpack.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/cgpack.js") },
    Static { path: "/motion.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/motion.js") },
    Static { path: "/text-model.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/text-model.js") },
    Static { path: "/qr.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/qr.js") },
    Static { path: "/ticker-model.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/ticker-model.js") },
    Static { path: "/ticker-items.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/ticker-items.js") },
    Static { path: "/color-picker.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/color-picker.js") },
    Static { path: "/ui.css", ctype: "text/css; charset=utf-8", body: include_str!("../web/ui.css") },
];

const DEMO_SHOW: &str = include_str!("../web/demo_show.json");

/// Zasoby przykładowe kopiowane do katalogu assets przy pierwszym uruchomieniu.
const DEMO_ASSETS: &[(&str, &[u8])] = &[
    ("tlo_hashtag.jpg", include_bytes!("../web/assets/tlo_hashtag.jpg")),
    ("tlo_hashtag2.jpg", include_bytes!("../web/assets/tlo_hashtag2.jpg")),
];

/// Maksymalna liczba jednocześnie obsługiwanych połączeń (HTTP + SSE + TCP).
const MAX_CONNS: usize = 512;
/// Limit łącznego rozmiaru katalogu assets.
const MAX_ASSETS_TOTAL: u64 = 2 * 1024 * 1024 * 1024;
/// Dozwolone rozszerzenia zasobów wgrywanych przez API.
const ASSET_EXT: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "mp4", "webm", "ttf", "otf", "woff", "woff2"];

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
/// Otwarte strumienie zdarzeń (okna Trio/Artist, wyjścia) – po zamknięciu okna aplikacji serwer
/// działa dalej, dopóki ktoś z niego korzysta.
static SSE_OPEN: AtomicUsize = AtomicUsize::new(0);

struct SseSlot;
impl SseSlot {
    fn new() -> SseSlot {
        SSE_OPEN.fetch_add(1, Ordering::SeqCst);
        SseSlot
    }
}
impl Drop for SseSlot {
    fn drop(&mut self) {
        SSE_OPEN.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Licznik połączeń zwalniany przy zakończeniu wątku (także po panice).
struct ConnSlot;
impl ConnSlot {
    fn acquire() -> Option<ConnSlot> {
        if ACTIVE.fetch_add(1, Ordering::SeqCst) >= MAX_CONNS {
            ACTIVE.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(ConnSlot)
    }
}
impl Drop for ConnSlot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Najwyżej tyle jednoczesnych połączeń z jednego adresu (poza pętlą zwrotną i trybem `--trust-proxy`):
/// jeden klient nie zajmie wszystkich `MAX_CONNS` (np. powolnymi nagłówkami).
const MAX_PER_IP: usize = 128;
static PER_IP: Mutex<Option<HashMap<IpAddr, usize>>> = Mutex::new(None);

struct IpSlot(IpAddr);
impl IpSlot {
    fn acquire(ip: IpAddr) -> Option<IpSlot> {
        let mut g = PER_IP.lock().unwrap_or_else(|e| e.into_inner());
        let n = g.get_or_insert_with(HashMap::new).entry(ip).or_insert(0);
        if *n >= MAX_PER_IP {
            return None;
        }
        *n += 1;
        Some(IpSlot(ip))
    }
}
impl Drop for IpSlot {
    fn drop(&mut self) {
        let mut g = PER_IP.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = g.as_mut() {
            if let Some(n) = m.get_mut(&self.0) {
                *n -= 1;
                if *n == 0 {
                    m.remove(&self.0);
                }
            }
        }
    }
}

/// Adres klienta podany przez reverse proxy (ostatni wpis `X-Forwarded-For`, inaczej `X-Real-IP`).
fn forwarded_ip(req: &Request) -> Option<String> {
    let xff = req.header("x-forwarded-for").and_then(|v| v.rsplit(',').map(str::trim).find(|s| !s.is_empty()));
    xff.or_else(|| req.header("x-real-ip").map(str::trim)).and_then(|ip| ip.parse::<IpAddr>().ok()).map(|ip| ip.to_string())
}

use config::{Cli, Config, Mode};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = match config::parse(&args, &|k| std::env::var(k).ok()) {
        Ok(Cli::Run(c)) => *c,
        Ok(Cli::Help) => {
            println!("{}", config::help());
            return;
        }
        Ok(Cli::Version) => {
            println!("VizRS {} ({})", env!("CARGO_PKG_VERSION"), env!("VIZRS_GIT"));
            return;
        }
        Err(e) => {
            eprintln!("{e}\n(vizrs --help – lista opcji)");
            std::process::exit(2);
        }
    };
    match cfg.mode {
        Mode::Healthcheck => std::process::exit(healthcheck(&cfg)),
        Mode::Serve => serve(cfg),
        Mode::App(w) => app_window(cfg, w),
    }
}

/// Działający serwer (wątek nasłuchu w tle).
struct Running {
    cfg: Arc<Config>,
    lock: Arc<datalock::DataLock>,
}

fn serve(cfg: Config) {
    let idle_exit = cfg.idle_exit;
    let running = start(cfg);
    if let Some(s) = idle_exit {
        // serwer uruchomiony przez okno (np. vizrs-shell): kończy pracę, gdy nikt z niego nie korzysta
        exit_when_idle(s, running, std::time::Instant::now());
    }
    loop {
        std::thread::park();
    }
}

/// Kończy proces, gdy przez `linger_s` s nie ma żadnego strumienia zdarzeń (okna, wyjścia OBS).
/// Do pierwszego połączenia czeka co najmniej 30 s (okno mogło jeszcze nie zdążyć się połączyć).
fn exit_when_idle(linger_s: u64, running: Running, started: std::time::Instant) -> ! {
    let (mut seen, mut idle) = (false, 0);
    loop {
        if SSE_OPEN.load(Ordering::SeqCst) > 0 {
            seen = true;
            idle = 0;
        } else if seen || started.elapsed() > Duration::from_secs(30) {
            idle += 1;
            if idle > linger_s {
                break;
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    println!("Brak podłączonych okien i wyjść – serwer kończy pracę.");
    running.lock.release();
    std::process::exit(0);
}

/// Otwiera okno; błąd tylko wypisuje (adres bez tokenu).
fn open_window(url: &str, w: app::Window, opts: &app::WindowOpts) -> Option<app::Opened> {
    match app::open(url, w, opts) {
        Ok(o) => {
            println!("Okno {}: {} – {}", w.name(), audit::scrub(url), o.how);
            Some(o)
        }
        Err(e) => {
            eprintln!("{e}");
            None
        }
    }
}

/// `vizrs artist|trio|output`: okno aplikacji z serwerem w tym procesie, dołączone do działającego
/// serwera na tym samym katalogu danych albo do serwera zdalnego (`--connect`).
fn app_window(cfg: Config, w: app::Window) {
    let (query, token) = (cfg.window.query.clone(), cfg.token.clone());
    let url_for = |base: &str| app::window_url(base, w, query.as_deref(), token.as_deref());
    if let Some(base) = &cfg.connect {
        if open_window(&url_for(base), w, &cfg.window).is_none() {
            std::process::exit(1);
        }
        return;
    }
    if let Some(o) = datalock::owner(&cfg.data, cfg.token.as_deref()) {
        println!("Serwer VizRS dla {} już działa (pid {}, http://{}) – otwieram okno {}.", cfg.data.display(), o.pid, o.addr, w.name());
        if open_window(&url_for(&format!("http://{}", o.addr)), w, &cfg.window).is_none() {
            std::process::exit(1);
        }
        return;
    }
    let (linger_s, opts) = (cfg.linger_s, cfg.window.clone());
    let running = start(cfg);
    let opened = open_window(&url_for(&format!("http://{}", running.cfg.local_addr())), w, &opts);
    let Some(mut opened) = opened else {
        eprintln!("Serwer działa dalej – otwórz adres ręcznie w przeglądarce.");
        loop {
            std::thread::park();
        }
    };
    // czekaj na zamknięcie okna; potem serwer działa, dopóki ktokolwiek korzysta (inne okna, OBS)
    let started = std::time::Instant::now();
    if let Some(ch) = opened.child.as_mut() {
        ch.wait().ok();
        println!("Okno zamknięte. Serwer kończy pracę, gdy przez {linger_s} s nie będzie podłączone żadne okno ani wyjście (OBS) – Ctrl+C kończy od razu.");
    }
    exit_when_idle(linger_s, running, started);
}

/// `--healthcheck`: 0 = serwer odpowiada `status: ok`, 1 = nie (Docker HEALTHCHECK, monitoring).
fn healthcheck(cfg: &Config) -> i32 {
    let addr = cfg.local_addr();
    match client::get(&addr, "/health", cfg.token.as_deref(), Duration::from_secs(3)) {
        Ok((200, body)) if Json::parse(&body).map(|h| h.str("status") == Some("ok")).unwrap_or(false) => {
            println!("ok {addr}");
            0
        }
        Ok((code, body)) => {
            eprintln!("{addr}: HTTP {code} {}", body.trim());
            1
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// Pierwszy port od `from` (do 20 kolejnych), na którym da się nasłuchiwać pod `bind`.
fn free_port(bind: &str, from: u16) -> Option<u16> {
    (from..from.saturating_add(20)).find(|p| TcpListener::bind((bind, *p)).is_ok())
}

/// Uruchamia serwer: blokada danych, silnik, porty; nasłuch HTTP w wątku w tle.
fn start(mut cfg: Config) -> Running {
    // domyślny port zajęty (np. drugi serwer na innym katalogu danych) – następny wolny; jawny --port bez zmian
    if !cfg.port_set {
        if let Some(p) = free_port(&cfg.bind, cfg.port).filter(|p| *p != cfg.port) {
            eprintln!("[vizrs] port {} jest zajęty – serwer używa portu {p} (stały port: --port / VIZRS_PORT)", cfg.port);
            cfg.port = p;
        }
    }
    let lock = match datalock::acquire(&cfg.data, &cfg.local_addr(), cfg.token.as_deref()) {
        Ok(l) => l,
        Err(o) => {
            eprintln!(
                "Katalog danych {} jest używany przez inny serwer VizRS (pid {}, http://{}).\nDwa serwery na jednym show.json nadpisywałyby sobie zmiany – użyj działającego albo wskaż inny --data.",
                cfg.data.display(),
                o.pid,
                o.addr
            );
            std::process::exit(3);
        }
    };
    cfg.instance = lock.instance.clone();
    let cfg = Arc::new(cfg);
    let engine = Engine::new(cfg.data.clone(), DEMO_SHOW);
    engine.audit.echo_stdout(cfg.log_stdout);
    if !config::is_loopback(&cfg.bind) && cfg.token.as_deref().map(|t| t.len() < 16).unwrap_or(false) {
        eprintln!("[vizrs] UWAGA: token krótszy niż 16 znaków przy nasłuchu w sieci – użyj długiego losowego (np. openssl rand -hex 24)");
    }
    for (name, bytes) in DEMO_ASSETS {
        let p = cfg.data.join("assets").join(name);
        if !p.exists() {
            std::fs::write(&p, bytes).ok();
        }
    }

    if cfg.tcp_port != 0 {
        let e = Arc::clone(&engine);
        let port = cfg.tcp_port;
        let c = Arc::clone(&cfg);
        std::thread::spawn(move || tcp_server(e, port, c));
    }

    if cfg.osc_port != 0 {
        let (e, c) = (Arc::clone(&engine), Arc::clone(&cfg));
        std::thread::spawn(move || osc_server(e, c));
    }
    if let Some(Ok(t)) = cfg.webhook.as_deref().map(hook::parse_url) {
        hook::spawn(Arc::clone(&engine), t);
    }
    let addr = format!("{}:{}", cfg.bind, cfg.port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Nie mogę otworzyć portu {addr}: {e}");
            drop(lock);
            std::process::exit(1);
        }
    };
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│ VizRS {:<51}│", env!("CARGO_PKG_VERSION"));
    println!("│  Artist (projektowanie):  http://localhost:{:<14}│", format!("{}/artist", cfg.port));
    println!("│  Trio   (playout):        http://localhost:{:<14}│", format!("{}/trio", cfg.port));
    println!("│  Wyjście OBS (1920x1080): http://localhost:{:<14}│", format!("{}/output", cfg.port));
    if cfg.osc_port != 0 {
        println!("│  OSC (UDP):               port {:<26}│", cfg.osc_port);
    }
    if cfg.tcp_port != 0 {
        println!("│  Komendy TCP:             port {:<26}│", cfg.tcp_port);
    }
    println!("│  Dane: {:<50}│", cfg.data.display().to_string());
    let auth = match (&cfg.token, &cfg.output_token) {
        (Some(_), Some(_)) => " (token + token wyjścia)",
        (Some(_), None) => " (token wymagany)",
        _ => "",
    };
    println!("│  Nasłuch: {:<47}│", format!("{}{}{}", cfg.bind, auth, if cfg.trust_proxy { ", za proxy" } else { "" }));
    println!("└──────────────────────────────────────────────────────────┘");

    // SIGTERM / Ctrl+C: zwolnij blokadę katalogu danych i zakończ (zapisy show są atomowe i synchroniczne)
    let lock = Arc::new(lock);
    {
        let (lock, e) = (Arc::clone(&lock), Arc::clone(&engine));
        shutdown::on_signal(move || {
            // bez println!: przy zamkniętym potoku stdout panika zatrzymałaby wyjście z programu
            use std::io::Write;
            writeln!(std::io::stdout(), "[vizrs] zatrzymanie – kończę pracę").ok();
            e.audit.log("system", "STOP (sygnał)", None);
            lock.release();
            std::process::exit(0);
        });
    }

    let c = Arc::clone(&cfg);
    std::thread::spawn(move || accept_loop(listener, engine, c));
    Running { cfg, lock }
}

fn accept_loop(listener: TcpListener, engine: Arc<Engine>, cfg: Arc<Config>) {
    for stream in listener.incoming().flatten() {
        let Some(slot) = ConnSlot::acquire() else {
            let mut s = stream;
            Response::err(503, "za dużo połączeń").write_to(&mut s).ok();
            continue;
        };
        let ip_slot = match stream.peer_addr().map(|a| a.ip()) {
            Ok(ip) if !cfg.trust_proxy && !ip.is_loopback() => match IpSlot::acquire(ip) {
                Some(s) => Some(s),
                None => {
                    let mut s = stream;
                    Response::err(503, "za dużo połączeń z tego adresu").write_to(&mut s).ok();
                    continue;
                }
            },
            _ => None,
        };
        let e = Arc::clone(&engine);
        let c = Arc::clone(&cfg);
        std::thread::spawn(move || {
            let _slots = (slot, ip_slot);
            handle_conn(stream, e, c)
        });
    }
}

fn handle_conn(mut stream: TcpStream, engine: Arc<Engine>, cfg: Arc<Config>) {
    stream.set_nodelay(true).ok();
    let mut req = match read_request(&mut stream) {
        Ok(r) => r,
        Err(e) => {
            let code = if e == "413" { 413 } else { 400 };
            Response::err(code, &e).write_to(&mut stream).ok();
            return;
        }
    };
    if cfg.trust_proxy {
        if let Some(ip) = forwarded_ip(&req) {
            req.peer = ip;
        }
    }
    let cookie = match guard::check(&req, &cfg.tokens()) {
        guard::Verdict::Allow => None,
        guard::Verdict::AllowSetCookie(t) => Some(guard::cookie_header(&t)),
        guard::Verdict::Deny(code, msg) => {
            if code == 401 {
                engine.audit.log(&format!("http:{}", req.peer), &format!("odmowa: zły token – {} {}", req.method, req.path), Some(401));
                // kolejne nieudane próby z tego adresu odpowiadają coraz wolniej
                std::thread::sleep(guard::note_failure(&req.peer));
            }
            Response::err(code, msg).write_to(&mut stream).ok();
            return;
        }
    };
    if req.path == "/events" {
        sse(stream, engine);
        return;
    }
    // panika w obsłudze jednego żądania nie może zatrzymać serwera
    let resp = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| route(&req, &engine, &cfg)))
        .unwrap_or_else(|_| Response::err(500, "błąd wewnętrzny"));
    if req.method != "GET" && req.method != "OPTIONS" || is_playout_get(&req.path) {
        let q = if req.raw_query.is_empty() { String::new() } else { format!("?{}", req.raw_query) };
        engine.audit.log(&format!("http:{}", req.peer), &format!("{} {}{}", req.method, req.path, q), Some(resp.status));
    }
    let resp = match cookie {
        Some(c) => resp.header("Set-Cookie", &c),
        None => resp,
    };
    resp.write_to(&mut stream).ok();
}

/// Server-Sent Events – kanał zdarzeń dla wyjść i UI.
fn sse(mut stream: TcpStream, engine: Arc<Engine>) {
    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\nX-Accel-Buffering: no\r\n\r\nretry: 1000\n\n";
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    let _open = SseSlot::new();
    let rx = engine.subscribe();
    loop {
        let msg = match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(m) => format!("data: {}\n\n", m),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => ": ping\n\n".to_string(),
            Err(_) => return,
        };
        if stream.write_all(msg.as_bytes()).and_then(|_| stream.flush()).is_err() {
            return; // klient rozłączony – Sender zostanie usunięty przy następnym broadcast
        }
    }
}

fn mime(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "json" => "application/json",
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn safe_name(name: &str) -> Option<String> {
    let n: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect();
    let n = n.trim_start_matches('.').to_string();
    if n.is_empty() || n.len() > 120 { None } else { Some(n) }
}

/// Wykonuje `f` dla każdego klucza z listy rozdzielonej przecinkami (komendy zbiorcze: `TAKE 1001,1101`).
/// Wszystkie elementy są wykonywane; jeśli któryś zawiedzie, zwracany jest zbiorczy błąd.
fn each(keys: &str, f: impl Fn(&str) -> Result<Json, String>) -> Result<Json, String> {
    if !keys.contains(',') {
        return f(keys);
    }
    let mut results = vec![];
    let mut errs = vec![];
    for k in keys.split(',').map(str::trim).filter(|k| !k.is_empty()) {
        match f(k) {
            Ok(j) => results.push(j),
            Err(e) => errs.push(format!("{k}: {e}")),
        }
    }
    if errs.is_empty() {
        Ok(Json::obj().with("ok", true).with("results", Json::Arr(results)))
    } else {
        Err(errs.join("; "))
    }
}

/// Uruchamia makro w osobnym wątku; nowe makro (lub `macro/stop`) przerywa poprzednie.
fn start_macro(engine: &Arc<Engine>, key: &str) -> Result<(), String> {
    let m = engine.find_macro(key).ok_or(format!("brak makra '{key}'"))?;
    let steps: Vec<(String, f64)> = m.arr("steps").iter().map(|s| (s.str("cmd").unwrap_or("").to_string(), s.num("delay").unwrap_or(0.0))).collect();
    if steps.is_empty() {
        return Err("makro nie ma kroków".into());
    }
    let gen = engine.macro_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let e = Arc::clone(engine);
    std::thread::spawn(move || {
        for (cmd, delay) in steps {
            let until = std::time::Instant::now() + Duration::from_secs_f64(delay.clamp(0.0, 3600.0));
            while std::time::Instant::now() < until {
                if e.macro_gen.load(Ordering::SeqCst) != gen {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            if e.macro_gen.load(Ordering::SeqCst) != gen {
                return;
            }
            let out = exec_command(&e, &cmd);
            if out.starts_with("ERR") {
                eprintln!("[vizrs] makro: {cmd} → {out}");
            }
        }
    });
    Ok(())
}

/// GET-y, które zmieniają stan anteny (Companion/Stream Deck) – trafiają do logu audytowego.
fn is_playout_get(path: &str) -> bool {
    ["/api/take/", "/api/arm/", "/api/preview/", "/api/rundown/", "/api/macro/", "/api/continue/", "/api/out/", "/api/clear", "/api/update/", "/api/deploy/", "/api/dock/", "/api/carousel/"].iter().any(|p| path.starts_with(p))
}

fn res<T>(r: Result<T, String>, f: impl FnOnce(T) -> Response) -> Response {
    match r {
        Ok(v) => f(v),
        Err(e) if e.starts_with("zapis") => Response::err(500, &e),
        Err(e) if e.starts_with("konflikt") => Response::err(409, &e),
        Err(e) => Response::err(400, &e),
    }
}

fn route(req: &Request, engine: &Arc<Engine>, cfg: &Config) -> Response {
    let m = req.method.as_str();
    if m == "OPTIONS" {
        return Response::new(204, "text/plain", "");
    }
    let segs = req.segs();
    let s: Vec<&str> = segs.to_vec();

    // ---------- pliki statyczne ----------
    if m == "GET" {
        let p = req.path.trim_end_matches(".html");
        let p = if p.is_empty() { "/" } else { p };
        if let Some(st) = STATIC.iter().find(|st| st.path == p) {
            if let Some(dir) = &cfg.web_dir {
                let file = if st.path == "/" { "index.html".to_string() } else {
                    let f = st.path.trim_start_matches('/');
                    if f.contains('.') { f.to_string() } else { format!("{f}.html") }
                };
                if let Ok(b) = std::fs::read(dir.join(file)) {
                    return Response::new(200, st.ctype, b);
                }
            }
            return Response::new(200, st.ctype, st.body);
        }
        if s.len() == 2 && s[0] == "fonts" {
            if s[1] == "fonts.css" {
                return Response::new(200, "text/css; charset=utf-8", fonts::css()).header("Cache-Control", "max-age=3600");
            }
            return match fonts::find(s[1]) {
                Some(f) => Response::new(200, "font/woff2", f.data).header("Cache-Control", "max-age=86400"),
                None => Response::err(404, "brak fontu"),
            };
        }
        if s.len() == 2 && s[0] == "assets" {
            if let Some(n) = safe_name(s[1]) {
                if let Ok(b) = std::fs::read(engine.data_dir.join("assets").join(&n)) {
                    return Response::new(200, mime(&n), b)
                        .header("Cache-Control", "max-age=60")
                        .header("Content-Security-Policy", "default-src 'none'; style-src 'unsafe-inline'; sandbox");
                }
            }
            return Response::err(404, "brak pliku");
        }
        if req.path == "/favicon.ico" {
            return Response::new(204, "text/plain", "");
        }
    }

    if s == ["health"] && m == "GET" {
        return Response::json(&engine.health().with("instance", cfg.instance.as_str()));
    }
    if s.first() != Some(&"api") {
        return Response::err(404, "nie znaleziono");
    }
    let body = match req.json() {
        Ok(b) => b,
        Err(e) if m != "GET" && !req.path.starts_with("/api/assets") => return Response::err(400, &e),
        Err(_) => Json::Null,
    };
    // wartości pól można też podać w query (?name=Jan&title=...) – wygodne dla Companion/Stream Deck
    let query_values = || {
        let mut v = Json::obj();
        for (k, val) in &req.query {
            v.set(k, val.as_str().into());
        }
        v
    };

    match (m, &s[1..]) {
        ("GET", ["show"]) => Response::json(&engine.show()),
        ("PUT", ["show"]) => {
            let force = req.q("force") == Some("1");
            res(engine.put_show(body, force), |_| Response::ok())
        }
        ("GET", ["state"]) => Response::json(&engine.state()),

        ("POST" | "PUT", [kind]) if engine::KINDS.contains(kind) => {
            res(engine.upsert(kind, body), |it| Response::json(&it))
        }
        ("PUT", [kind, id]) if engine::KINDS.contains(kind) => {
            let mut b = body;
            b.set("id", (*id).into());
            res(engine.upsert(kind, b), |it| Response::json(&it))
        }
        ("DELETE", [kind, id]) if engine::KINDS.contains(kind) => res(engine.delete(kind, id), |_| Response::ok()),

        // ---------- playout (GET dozwolony dla prostych integracji) ----------
        ("POST" | "GET", ["take", key]) | ("POST" | "GET", ["arm", key]) => {
            let arm = s[1] == "arm";
            let mut ov = query_values();
            if let Some(Json::Obj(_)) = body.get("values") {
                ov = body.get("values").cloned().unwrap();
            }
            let ov = if matches!(&ov, Json::Obj(v) if v.is_empty()) { None } else { Some(ov) };
            res(each(key, |k| if arm { engine.arm(k, ov.as_ref()) } else { engine.take(k, ov.as_ref()) }), |j| Response::json(&j))
        }
        ("POST" | "GET", ["preview", "clear"]) => {
            engine.preview_clear();
            Response::ok()
        }
        ("POST" | "GET", ["continue", key]) => res(each(key, |k| engine.cont(k)), |j| Response::json(&j)),
        ("POST" | "GET", ["out", key]) => res(each(key, |k| engine.out(k)), |j| Response::json(&j)),

        // ---------- rundown ----------
        ("POST" | "GET", ["rundown", id, "next"]) => res(engine.rundown_take(id, None), |j| Response::json(&j)),
        ("POST" | "GET", ["rundown", id, "take", item]) => res(engine.rundown_take(id, Some(item)), |j| Response::json(&j)),
        ("POST" | "GET", ["rundown", id, "cursor", item]) => res(engine.rundown_cursor(id, item), |_| Response::ok()),
        ("POST" | "GET", ["rundown", id, "reset"]) => res(engine.rundown_reset(id), |_| Response::ok()),

        // ---------- makra ----------
        ("POST" | "GET", ["macro", "stop"]) => {
            engine.macro_gen.fetch_add(1, Ordering::SeqCst);
            Response::ok()
        }
        ("POST" | "GET", ["macro", key]) => res(start_macro(engine, key), |_| Response::ok()),
        ("GET", ["variables"]) => Response::json(&engine.variables()),
        ("POST" | "GET", ["clear"]) => {
            engine.clear();
            Response::ok()
        }
        // update: zmienia pola strony i aktualizuje na żywo, jeśli on-air.
        // ?save=0 – tylko na żywo, bez zapisu do strony
        ("POST" | "PATCH" | "GET", ["update", key]) => {
            let vals = if body.is_obj() { body.get("values").cloned().unwrap_or(body.clone()) } else { query_values() };
            let mut vals = vals;
            if let Json::Obj(items) = &mut vals {
                items.retain(|(k, _)| k != "save");
            }
            let persist = req.q("save") != Some("0");
            res(engine.update(key, &vals, persist), |j| Response::json(&j))
        }
        ("POST" | "GET", ["deploy", key]) => res(engine.deploy(key), |j| Response::json(&j)),
        // ticker: lista wiadomości strony (GET) i zmiany z automatycznym UPDATE, gdy strona jest na antenie
        ("GET", ["ticker", key]) => res(engine.ticker(key, ticker::Op::List), |j| Response::json(&j)),
        ("POST" | "GET", ["ticker", key, op @ ..]) => {
            let item = body.str("item").map(str::to_string).or_else(|| req.q("item").map(str::to_string)).unwrap_or_default();
            let line = match op {
                ["add"] => format!("ADD {item}"),
                ["set", n] => format!("SET {n} {item}"),
                ["del", n] => format!("DEL {n}"),
                ["clear"] => "CLEAR".to_string(),
                _ => "?".to_string(),
            };
            res(ticker::parse_op(&line).and_then(|o| engine.ticker(key, o)), |j| Response::json(&j))
        }
        ("POST" | "GET", ["carousel", "start", id]) => res(engine.carousel_start(id), |_| Response::ok()),
        ("POST" | "GET", ["carousel", "stop"]) => {
            engine.carousel_stop(req.q("out") != Some("0"));
            Response::ok()
        }

        // ---------- zasoby (obrazy, fonty) ----------
        ("GET", ["assets"]) => {
            let mut list: Vec<String> = std::fs::read_dir(engine.data_dir.join("assets"))
                .map(|rd| rd.flatten().filter_map(|e| e.file_name().into_string().ok()).collect())
                .unwrap_or_default();
            list.sort();
            Response::json(&Json::Arr(
                list.into_iter()
                    .map(|n| Json::obj().with("name", n.as_str()).with("url", format!("/assets/{n}")))
                    .collect(),
            ))
        }
        ("POST", ["assets"]) => {
            let Some(name) = req.q("name").and_then(safe_name) else {
                return Response::err(400, "podaj ?name=plik.png");
            };
            if req.body.is_empty() {
                return Response::err(400, "pusty plik");
            }
            let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
            if !name.contains('.') || !ASSET_EXT.contains(&ext.as_str()) {
                return Response::err(400, &format!("niedozwolony typ pliku (dozwolone: {})", ASSET_EXT.join(", ")));
            }
            if assets_size(&engine.data_dir.join("assets")) + req.body.len() as u64 > MAX_ASSETS_TOTAL {
                return Response::err(413, "przekroczony limit rozmiaru katalogu assets");
            }
            let p: &Path = &engine.data_dir.join("assets").join(&name);
            match std::fs::write(p, &req.body) {
                Ok(_) => Response::json(&Json::obj().with("name", name.as_str()).with("url", format!("/assets/{name}"))),
                Err(e) => Response::err(500, &e.to_string()),
            }
        }
        ("DELETE", ["assets", name]) => {
            if let Some(n) = safe_name(name) {
                std::fs::remove_file(engine.data_dir.join("assets").join(n)).ok();
            }
            Response::ok()
        }

        // ---------- diagnostyka ----------
        ("POST", ["report"]) => {
            engine.report_error(body.str("scene").unwrap_or("?"), body.str("layer").unwrap_or("?"), body.str("message").unwrap_or("?"));
            Response::ok()
        }
        ("GET", ["errors"]) => Response::json(&engine.errors()),
        ("DELETE", ["errors"]) => {
            engine.clear_errors();
            Response::ok()
        }
        ("GET", ["health"]) => Response::json(&engine.health().with("instance", cfg.instance.as_str())),
        ("GET", ["locks"]) => Response::json(&engine.locks_json()),
        ("POST" | "PUT", ["lock", kind, id]) => res(engine.lock(kind, id, req.q("who").unwrap_or("")), |j| Response::json(&j)),
        ("DELETE" | "POST", ["unlock", kind, id]) => Response::json(&engine.unlock(kind, id, req.q("who").unwrap_or(""), req.q("force") == Some("1"))),
        ("GET", ["animpresets"]) => Response::json(&engine.anim_presets()),
        ("PUT", ["animpresets"]) => res(engine.put_anim_presets(body), |j| Response::json(&j)),
        ("POST" | "GET", ["dock", layer, mode]) if *mode == "off" || *mode == "on" => res(engine.set_dock(layer, *mode == "off"), |j| Response::json(&j)),
        ("GET", ["wordreplace"]) => Response::json(&engine.word_replace()),
        ("PUT", ["wordreplace"]) => res(engine.put_word_replace(body), |j| Response::json(&j)),
        ("GET", ["fonts"]) => Response::json(&Json::Arr(fonts::families().into_iter().map(Json::from).collect())),
        ("GET", ["audit"]) => Response::json(&Json::Arr(engine.audit.tail(req.q("n").and_then(|n| n.parse().ok()).unwrap_or(100).min(1000)).into_iter().map(Json::from).collect())),

        // ---------- biblioteka show (paczka grafiki na audycję) ----------
        ("GET", ["shows"]) => Response::json(&engine.shows()),
        ("POST", ["shows", "switch", id]) => res(engine.switch_show(id), |_| Response::json(&engine.shows())),
        ("POST", ["shows"]) => res(engine.new_show(body.str("name").unwrap_or(""), matches!(body.get("copy"), Some(Json::Bool(true)))), |j| Response::json(&j)),
        ("DELETE", ["shows", id]) => res(engine.delete_show(id), |_| Response::json(&engine.shows())),

        // ---------- kopie zapasowe show ----------
        ("GET", ["backups"]) => Response::json(&Json::Arr(engine.backups().into_iter().map(Json::from).collect())),
        ("POST", ["backups", "restore", name]) => res(engine.restore_backup(name), |_| Response::ok()),

        _ => Response::err(404, "nieznany endpoint"),
    }
}

fn assets_size(dir: &Path) -> u64 {
    std::fs::read_dir(dir).map(|rd| rd.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()).unwrap_or(0)
}

// ---------------- port komend tekstowych (w stylu Viz Engine :6100) ----------------

fn osc_server(engine: Arc<Engine>, cfg: Arc<Config>) {
    let Ok(sock) = std::net::UdpSocket::bind((cfg.bind.as_str(), cfg.osc_port)) else {
        eprintln!("[vizrs] nie mogę otworzyć portu OSC {}", cfg.osc_port);
        return;
    };
    let mut buf = [0u8; 8192];
    while let Ok((n, from)) = sock.recv_from(&mut buf) {
        let Some((addr, args)) = osc::parse(&buf[..n]) else { continue };
        match osc::to_command(&addr, &args, &cfg.token) {
            Ok(line) => {
                engine.audit.log(&format!("osc:{}", from.ip()), &line, None);
                let out = exec_command(&engine, &line);
                if out.starts_with("ERR") {
                    eprintln!("[vizrs] OSC {addr}: {out}");
                }
            }
            Err(e) => eprintln!("[vizrs] OSC odrzucony ({}): {e}", from.ip()),
        }
    }
}

fn tcp_server(engine: Arc<Engine>, port: u16, cfg: Arc<Config>) {
    let Ok(l) = TcpListener::bind((cfg.bind.as_str(), port)) else {
        eprintln!("[vizrs] nie mogę otworzyć portu TCP {port}");
        return;
    };
    for mut s in l.incoming().flatten() {
        let Some(slot) = ConnSlot::acquire() else {
            writeln!(s, "ERR za dużo połączeń").ok();
            continue;
        };
        let e = Arc::clone(&engine);
        let tok = cfg.token.clone();
        std::thread::spawn(move || {
            let _slot = slot;
            tcp_client(s, e, tok)
        });
    }
}

fn parse_kv(s: &str) -> Json {
    // "name=Jan Kowalski|title=Reporter"  albo  JSON {"name":"..."}
    let s = s.trim();
    if s.starts_with('{') {
        return Json::parse(s).unwrap_or(Json::obj());
    }
    let mut v = Json::obj();
    for part in s.split('|') {
        if let Some((k, val)) = part.split_once('=') {
            v.set(k.trim(), val.trim().into());
        }
    }
    v
}

pub fn exec_command(engine: &Arc<Engine>, line: &str) -> String {
    let line = line.trim();
    let (cmd, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = rest.trim();
    let (arg, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let r: Result<Json, String> = match cmd.to_ascii_uppercase().as_str() {
        "TAKE" | "ARM" => {
            let ov = parse_kv(tail);
            let ov = if matches!(&ov, Json::Obj(v) if v.is_empty()) { None } else { Some(ov) };
            let arm = cmd.eq_ignore_ascii_case("ARM");
            each(arg, |k| if arm { engine.arm(k, ov.as_ref()) } else { engine.take(k, ov.as_ref()) })
        }
        "CONTINUE" | "CONT" => each(if arg.is_empty() { "main" } else { arg }, |k| engine.cont(k)),
        "OUT" => each(if arg.is_empty() { "main" } else { arg }, |k| engine.out(k)),
        "NEXT" => engine.rundown_take(arg, None),
        "RUNDOWN" => match arg.to_ascii_uppercase().as_str() {
            "TAKE" => {
                let (rid, item) = tail.trim().split_once(char::is_whitespace).unwrap_or((tail.trim(), ""));
                engine.rundown_take(rid, Some(item.trim()).filter(|i| !i.is_empty()))
            }
            "RESET" => engine.rundown_reset(tail.trim()).map(|_| Json::obj()),
            _ => Err("RUNDOWN TAKE <rundown> [pozycja] | RUNDOWN RESET <rundown>".into()),
        },
        "MACRO" => {
            if tail.trim().is_empty() && arg.eq_ignore_ascii_case("STOP") {
                engine.macro_gen.fetch_add(1, Ordering::SeqCst);
                Ok(Json::obj())
            } else {
                start_macro(engine, rest).map(|_| Json::obj())
            }
        }
        "CLEAR" => {
            engine.clear();
            Ok(Json::obj())
        }
        "UPDATE" => engine.update(arg, &parse_kv(tail), true),
        "LIVE" => engine.update(arg, &parse_kv(tail), false),
        "TICKER" => ticker::parse_op(tail).and_then(|op| engine.ticker(arg, op)),
        "CAROUSEL" => match arg.to_ascii_uppercase().as_str() {
            "START" => engine.carousel_start(tail.trim()).map(|_| Json::obj()),
            "STOP" => {
                engine.carousel_stop(true);
                Ok(Json::obj())
            }
            _ => Err("CAROUSEL START <playlista> | CAROUSEL STOP".into()),
        },
        "STATE" => Ok(engine.state()),
        "PING" => Ok(Json::obj().with("pong", true)),
        "HELP" | "" => Err("komendy: TAKE <strona[,strona…]> [k=v|k2=v2], ARM <strona>, NEXT <rundown>, RUNDOWN TAKE|RESET <rundown> [pozycja], MACRO <nazwa>|STOP, CONTINUE <warstwa|strona>, OUT <warstwa|strona>, CLEAR, UPDATE <strona> k=v|..., LIVE <strona> k=v|..., TICKER <strona[:pole]> LIST|ADD <KAT|treść>|SET <n> <KAT|treść>|DEL <n>|CLEAR, CAROUSEL START <playlista>, CAROUSEL STOP, STATE".into()),
        other => Err(format!("nieznana komenda {other}")),
    };
    match r {
        Ok(j) => format!("OK {}", j.dump()),
        Err(e) => format!("ERR {e}"),
    }
}

fn tcp_client(stream: TcpStream, engine: Arc<Engine>, token: Option<String>) {
    use std::io::Read;
    stream.set_read_timeout(Some(Duration::from_secs(3600))).ok();
    let peer = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    let Ok(mut w) = stream.try_clone() else { return };
    let mut r = BufReader::new(stream);
    let mut authed = token.is_none();
    loop {
        let mut line = String::new();
        // linia ograniczona do 64 KB (ochrona przed nieskończoną linią bez \n)
        match r.by_ref().take(64 * 1024).read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(n) if n == 64 * 1024 && !line.ends_with('\n') => {
                writeln!(w, "ERR za długa linia").ok();
                break;
            }
            Ok(_) => {}
        }
        if line.trim().is_empty() {
            continue;
        }
        let out = if !authed {
            match (line.trim().strip_prefix("AUTH "), &token) {
                (Some(t), Some(want)) if t.trim() == want => {
                    authed = true;
                    "OK {}".to_string()
                }
                _ => "ERR wymagane: AUTH <token>".to_string(),
            }
        } else {
            if !matches!(line.trim().to_ascii_uppercase().as_str(), "STATE" | "PING" | "HELP") {
                engine.audit.log(&format!("tcp:{peer}"), line.trim(), None);
            }
            exec_command(&engine, &line)
        };
        if writeln!(w, "{out}").is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn busy_port_falls_back() {
        let busy = TcpListener::bind("127.0.0.1:0").unwrap();
        let p = busy.local_addr().unwrap().port();
        assert!(free_port("127.0.0.1", p).is_some_and(|f| f != p));
        drop(busy);
        assert_eq!(free_port("127.0.0.1", p), Some(p));
    }

    fn setup(name: &str) -> (Arc<Engine>, Config, PathBuf) {
        let d = std::env::temp_dir().join(format!("vizrs-main-{}-{}", std::process::id(), name));
        std::fs::remove_dir_all(&d).ok();
        let e = Engine::new(d.clone(), DEMO_SHOW);
        let cfg = Config { port: 0, tcp_port: 0, data: d.clone(), ..Config::default() };
        (e, cfg, d)
    }

    fn call(e: &Arc<Engine>, cfg: &Config, method: &str, target: &str, body: &str) -> (u16, String) {
        let (path, qs) = target.split_once('?').unwrap_or((target, ""));
        let query = qs
            .split('&')
            .filter(|s| !s.is_empty())
            .map(|kv| {
                let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
                (http::url_decode(k), http::url_decode(v))
            })
            .collect();
        let req = Request { method: method.into(), path: path.into(), query, headers: vec![], body: body.as_bytes().to_vec(), peer: String::new(), raw_query: String::new() };
        let r = route(&req, e, cfg);
        (r.status, String::from_utf8_lossy(&r.body).into_owned())
    }

    #[test]
    fn http_playout_roundtrip() {
        let (e, cfg, d) = setup("playout");
        let (c, b) = call(&e, &cfg, "GET", "/api/take/1001?headline=Test%20A", "");
        assert_eq!(c, 200, "{b}");
        let (_, st) = call(&e, &cfg, "GET", "/api/state", "");
        assert!(st.contains("\"number\":1001"));
        assert_eq!(call(&e, &cfg, "GET", "/api/update/1001?headline=B&save=0", "").0, 200);
        assert_eq!(call(&e, &cfg, "POST", "/api/continue/main", "").0, 200);
        assert_eq!(call(&e, &cfg, "POST", "/api/out/main", "").0, 200);
        assert_eq!(call(&e, &cfg, "GET", "/api/take/777777", "").0, 400);
        assert_eq!(call(&e, &cfg, "GET", "/api/nie-ma", "").0, 404);
        assert_eq!(call(&e, &cfg, "GET", "/nie-ma", "").0, 404);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn http_rejects_bad_json_and_non_object_show() {
        let (e, cfg, d) = setup("badjson");
        assert_eq!(call(&e, &cfg, "PUT", "/api/show", "{nie json").0, 400);
        assert_eq!(call(&e, &cfg, "PUT", "/api/show", "[]").0, 400);
        assert_eq!(call(&e, &cfg, "POST", "/api/pages", "null").0, 400);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn assets_whitelist_and_safe_names() {
        let (e, cfg, d) = setup("assets");
        assert_eq!(call(&e, &cfg, "POST", "/api/assets?name=a.png", "x").0, 200);
        assert_eq!(call(&e, &cfg, "POST", "/api/assets?name=a.html", "<script>").0, 400);
        assert_eq!(call(&e, &cfg, "POST", "/api/assets?name=a.js", "x").0, 400);
        assert_eq!(call(&e, &cfg, "POST", "/api/assets?name=bezrozszerzenia", "x").0, 400);
        let (c, b) = call(&e, &cfg, "POST", "/api/assets?name=..%2f..%2fx.png", "x");
        assert_eq!(c, 200);
        assert!(b.contains("_.._x.png") || b.contains("__x.png") || b.contains("x.png"));
        assert!(!d.join("x.png").exists());
        assert_eq!(call(&e, &cfg, "GET", "/assets/..%2fshow.json", "").0, 404);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn shows_library() {
        let (e, cfg, _d) = setup("shows");
        let (c, b) = call(&e, &cfg, "GET", "/api/shows", "");
        assert_eq!(c, 200);
        let lib = Json::parse(&b).unwrap();
        let first = lib.str("active").unwrap().to_string();
        assert_eq!(lib.arr("shows").len(), 1);
        let (c, b) = call(&e, &cfg, "POST", "/api/shows", r#"{"name":"Fakty Wieczór","copy":false}"#);
        assert_eq!(c, 200, "{b}");
        assert!(b.contains("fakty-wieczor"));
        let (_, sh) = call(&e, &cfg, "GET", "/api/show", "");
        assert!(sh.contains("Fakty Wieczór") && Json::parse(&sh).unwrap().arr("scenes").is_empty());
        assert_eq!(call(&e, &cfg, "DELETE", "/api/shows/fakty-wieczor", "").0, 400); // aktywny
        assert_eq!(call(&e, &cfg, "POST", &format!("/api/shows/switch/{first}"), "").0, 200);
        let (_, sh) = call(&e, &cfg, "GET", "/api/show", "");
        assert!(!Json::parse(&sh).unwrap().arr("scenes").is_empty());
        assert_eq!(call(&e, &cfg, "POST", "/api/shows/switch/..%2fshow", "").0, 400);
        assert_eq!(call(&e, &cfg, "POST", "/api/shows/switch/nie-ma", "").0, 400);
        assert_eq!(call(&e, &cfg, "POST", "/api/shows", r#"{"name":"  "}"#).0, 400);
        assert_eq!(call(&e, &cfg, "DELETE", "/api/shows/fakty-wieczor", "").0, 200);
        assert_eq!(Json::parse(&call(&e, &cfg, "GET", "/api/shows", "").1).unwrap().arr("shows").len(), 1);
    }

    #[test]
    fn backups_endpoint() {
        let (e, cfg, d) = setup("backups");
        let (c, b) = call(&e, &cfg, "GET", "/api/backups", "");
        assert_eq!(c, 200);
        assert!(b.contains("show-"));
        assert_eq!(call(&e, &cfg, "POST", "/api/backups/restore/..%2fshow.json", "").0, 400);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn text_commands() {
        let (e, _cfg, d) = setup("tcp");
        assert!(exec_command(&e, "PING").starts_with("OK"));
        assert!(exec_command(&e, "TAKE 1001 headline=Hej|x=1").starts_with("OK"));
        assert!(exec_command(&e, "STATE").contains("1001"));
        assert!(exec_command(&e, "UPDATE 1001 headline=Zmiana").starts_with("OK"));
        assert!(exec_command(&e, "TAKE 31337").starts_with("ERR"));
        assert!(exec_command(&e, "BLAH").starts_with("ERR"));
        assert!(exec_command(&e, "CLEAR").starts_with("OK"));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn ticker_commands() {
        let (e, cfg, d) = setup("ticker");
        let r = exec_command(&e, "TICKER 3001 LIST");
        assert!(r.starts_with("OK") && r.contains("\"field\":\"items\""), "{r}");
        let r = exec_command(&e, "TICKER 3001 ADD SPORT#1f8a4c|Gol w doliczonym czasie");
        assert!(r.contains("Gol w doliczonym czasie") && r.contains("#1f8a4c") && r.contains("\"onAir\":false"), "{r}");
        assert!(exec_command(&e, "TAKE 3001").starts_with("OK"));
        let r = exec_command(&e, "ticker 3001 set 1 PILNE|Nowa pierwsza");
        assert!(r.contains("\"onAir\":true") && r.contains("Nowa pierwsza"), "{r}");
        assert!(exec_command(&e, "TICKER 3001 DEL 999").starts_with("ERR"));
        assert!(exec_command(&e, "TICKER 3001 FOO").starts_with("ERR"));
        assert!(exec_command(&e, "TICKER 1001 LIST").starts_with("ERR"), "strona bez tickera");
        assert!(exec_command(&e, "TICKER 3001:items LIST").starts_with("OK"), "strona:pole");
        let (c, b) = call(&e, &cfg, "GET", "/api/ticker/3001", "");
        assert!(c == 200 && b.contains("Nowa pierwsza"), "{b}");
        let (c, b) = call(&e, &cfg, "POST", "/api/ticker/3001/add?item=KRAJ%7CZ%20HTTP", "");
        assert!(c == 200 && b.contains("Z HTTP"), "{b}");
        let (c, b) = call(&e, &cfg, "POST", "/api/ticker/3001/del/1", "");
        assert!(c == 200 && !b.contains("Nowa pierwsza"), "{b}");
        assert_eq!(call(&e, &cfg, "POST", "/api/ticker/3001/bogus", "").0, 400);
        let (c, _) = call(&e, &cfg, "POST", "/api/ticker/3001/clear", "");
        assert_eq!(c, 200);
        let show = e.show();
        let p = show.arr("pages").iter().find(|p| p.num("number") == Some(3001.0)).unwrap().clone();
        assert_eq!(p.get("values").and_then(|v| v.str("items")), Some(""), "zmiany zapisane na stronie");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn per_ip_slots_limit_and_release() {
        let ip: IpAddr = "198.51.100.7".parse().unwrap();
        let held: Vec<IpSlot> = (0..MAX_PER_IP).map(|_| IpSlot::acquire(ip).unwrap()).collect();
        assert!(IpSlot::acquire(ip).is_none());
        assert!(IpSlot::acquire("198.51.100.8".parse().unwrap()).is_some());
        drop(held);
        assert!(IpSlot::acquire(ip).is_some());
    }

    #[test]
    fn forwarded_ip_takes_last_proxy_entry() {
        let r = |h: &[(&str, &str)]| Request { method: "GET".into(), path: "/".into(), query: vec![], headers: h.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(), body: vec![], peer: String::new(), raw_query: String::new() };
        assert_eq!(forwarded_ip(&r(&[("x-forwarded-for", "1.2.3.4, 203.0.113.5")])).as_deref(), Some("203.0.113.5"));
        assert_eq!(forwarded_ip(&r(&[("x-real-ip", "2001:db8::1")])).as_deref(), Some("2001:db8::1"));
        assert_eq!(forwarded_ip(&r(&[("x-forwarded-for", "nie-ip")])), None);
        assert_eq!(forwarded_ip(&r(&[])), None);
    }

    #[test]
    fn conn_limit_slots_released() {
        let before = ACTIVE.load(Ordering::SeqCst);
        {
            let _a = ConnSlot::acquire().unwrap();
            assert_eq!(ACTIVE.load(Ordering::SeqCst), before + 1);
        }
        assert_eq!(ACTIVE.load(Ordering::SeqCst), before);
    }

    #[test]
    fn http_conflict_health_report_deploy() {
        let (e, cfg, d) = setup("v12");
        let (c, b) = call(&e, &cfg, "GET", "/health", "");
        assert_eq!(c, 200);
        assert!(b.contains("\"status\":\"ok\""));
        assert_eq!(call(&e, &cfg, "GET", "/api/health", "").0, 200);
        assert_eq!(call(&e, &cfg, "POST", "/api/report", r#"{"scene":"s","layer":"main","message":"x"}"#).0, 200);
        assert!(call(&e, &cfg, "GET", "/api/errors", "").1.contains("\"count\":1"));
        assert_eq!(call(&e, &cfg, "DELETE", "/api/errors", "").0, 200);
        // 409 przy nieaktualnym rev strony
        let (_, p) = call(&e, &cfg, "GET", "/api/show", "");
        let show = Json::parse(&p).unwrap();
        let sid = show.arr("scenes")[0].str("id").unwrap().to_string();
        let (c0, b0) = call(&e, &cfg, "POST", "/api/pages", &format!(r#"{{"scene":"{sid}","name":"Nowa"}}"#));
        assert_eq!(c0, 200, "{b0}");
        let mut page = Json::parse(&b0).unwrap();
        page.set("name", "Zmiana".into());
        let url = format!("/api/pages/{}", page.str("id").unwrap());
        assert_eq!(call(&e, &cfg, "PUT", &url, &page.dump()).0, 200);
        assert_eq!(call(&e, &cfg, "PUT", &url, &page.dump()).0, 409);
        // walidacja → 400
        assert_eq!(call(&e, &cfg, "POST", "/api/pages", r#"{"scene":"nie-ma"}"#).0, 400);
        assert_eq!(call(&e, &cfg, "PUT", "/api/show", r#"{"scenes":"x"}"#).0, 400);
        // import: ze starym rev 409, z force=1 ok
        let mut old = show.clone();
        old.set("rev", 0.0.into());
        assert_eq!(call(&e, &cfg, "PUT", "/api/show", &old.dump()).0, 409);
        assert_eq!(call(&e, &cfg, "PUT", "/api/show?force=1", &old.dump()).0, 200);
        // deploy pustej warstwy
        assert_eq!(call(&e, &cfg, "POST", "/api/deploy/main", "").0, 400);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn batch_take_out_and_arm() {
        let (e, cfg, d) = setup("batch");
        assert_eq!(call(&e, &cfg, "GET", "/api/take/1001,3001", "").0, 200);
        let (_, st) = call(&e, &cfg, "GET", "/api/state", "");
        assert!(st.contains("1001") && st.contains("3001"));
        // część się nie udaje → błąd zbiorczy, ale reszta wykonana
        let (c, b) = call(&e, &cfg, "GET", "/api/take/1002,777777", "");
        assert_eq!(c, 400, "{b}");
        assert!(b.contains("777777"));
        assert!(call(&e, &cfg, "GET", "/api/state", "").1.contains("1002"));
        assert_eq!(call(&e, &cfg, "GET", "/api/out/main,back", "").0, 200);
        assert_eq!(call(&e, &cfg, "POST", "/api/arm/1001", "").0, 200);
        assert!(call(&e, &cfg, "GET", "/api/state", "").1.contains("\"preview\":[{"));
        assert_eq!(call(&e, &cfg, "POST", "/api/preview/clear", "").0, 200);
        assert!(exec_command(&e, "TAKE 1001,3001").starts_with("OK"));
        assert!(exec_command(&e, "OUT main,back").starts_with("OK"));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn macros_run_with_delays_and_can_be_stopped() {
        let (e, cfg, d) = setup("macro");
        let m = r#"{"name":"Segment","steps":[{"cmd":"CLEAR","delay":0},{"cmd":"TAKE 1001","delay":0},{"cmd":"TAKE 1101","delay":0.3}]}"#;
        assert_eq!(call(&e, &cfg, "POST", "/api/macros", m).0, 200);
        assert_eq!(call(&e, &cfg, "POST", "/api/macro/Segment", "").0, 200);
        std::thread::sleep(Duration::from_millis(150));
        let st = call(&e, &cfg, "GET", "/api/state", "").1;
        assert!(st.contains("1001") && !st.contains("1101"), "{st}"); // drugi TAKE jeszcze czeka
        std::thread::sleep(Duration::from_millis(400));
        assert!(call(&e, &cfg, "GET", "/api/state", "").1.contains("1101"));
        // przerwanie
        assert_eq!(call(&e, &cfg, "POST", "/api/macro/Segment", "").0, 200);
        assert_eq!(call(&e, &cfg, "POST", "/api/macro/stop", "").0, 200);
        std::thread::sleep(Duration::from_millis(500));
        assert_eq!(call(&e, &cfg, "POST", "/api/macro/nie-ma", "").0, 400);
        // rekurencja zabroniona już przy zapisie
        assert_eq!(call(&e, &cfg, "POST", "/api/macros", r#"{"name":"X","steps":[{"cmd":"MACRO Segment"}]}"#).0, 400);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn rundown_over_http_and_text() {
        let (e, cfg, d) = setup("rdhttp");
        let (_, sh) = call(&e, &cfg, "GET", "/api/show", "");
        let show = Json::parse(&sh).unwrap();
        let p0 = show.arr("pages")[0].str("id").unwrap().to_string();
        let p1 = show.arr("pages")[1].str("id").unwrap().to_string();
        let body = format!(r#"{{"id":"rd1","name":"Dziennik","items":[{{"id":"a","page":"{p0}"}},{{"id":"b","page":"{p1}"}}]}}"#);
        assert_eq!(call(&e, &cfg, "POST", "/api/rundowns", &body).0, 200);
        assert_eq!(call(&e, &cfg, "POST", "/api/rundown/rd1/next", "").0, 200);
        assert!(exec_command(&e, "NEXT Dziennik").starts_with("OK"));
        assert!(exec_command(&e, "NEXT rd1").starts_with("ERR")); // koniec
        assert!(exec_command(&e, "RUNDOWN RESET rd1").starts_with("OK"));
        assert!(exec_command(&e, "RUNDOWN TAKE rd1 b").starts_with("OK"));
        assert_eq!(call(&e, &cfg, "GET", "/api/variables", "").0, 200);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn locks_over_http() {
        let (e, cfg, d) = setup("lockhttp");
        assert_eq!(call(&e, &cfg, "POST", "/api/lock/pages/p1?who=Anna", "").0, 200);
        assert_eq!(call(&e, &cfg, "POST", "/api/lock/pages/p1?who=Jan", "").0, 409);
        assert_eq!(call(&e, &cfg, "POST", "/api/lock/pages/p1", "").0, 400);
        assert!(call(&e, &cfg, "GET", "/api/locks", "").1.contains("Anna"));
        assert_eq!(call(&e, &cfg, "POST", "/api/unlock/pages/p1?who=Anna", "").0, 200);
        assert_eq!(call(&e, &cfg, "GET", "/api/locks", "").1, "[]");
        std::fs::remove_dir_all(d).ok();
    }
}
