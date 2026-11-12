//! vizrs-shell – natywne okno VizRS Trio i wyjścia (podglądu) na Tauri 2 (WebView2 / WebKitGTK / WKWebView).
//!
//!   vizrs-shell [trio|output|connect] [--data KAT] [--port N] [--token T] [--connect URL] [--vizrs ŚCIEŻKA]
//!
//! Strony UI są te same co w przeglądarce (serwuje je `vizrs`), więc jeden renderer (`web/renderer.js`)
//! obsługuje okno Trio i wyjście OBS. Okno dodaje to, czego przeglądarka nie ma: menu, systemowe „Zapisz jako”,
//! pytanie przy zamknięciu, stan anteny w tytule, okno połączenia, jedną instancję, wyjście na monitorze.
//! Strona nie ma dostępu do systemu – tylko komunikaty z białej listy (`bridge`) przez komendę `ipc`, i tylko
//! od strony bieżącego serwera (uprawnienie w działaniu: `allow_server`). Artist (projektowanie scen) to osobny
//! program VizRS CG Artist (`vizrs-cg-artist`); `/artist` serwera zostaje w przeglądarce jako zamrożony zapas.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod bridge;
mod connect;
mod desktop;
mod paths;
mod server;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::ipc::CapabilityBuilder;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::webview::{DownloadEvent, NewWindowResponse, PageLoadEvent};
#[cfg(not(target_os = "linux"))]
use tauri::window::Color;
use tauri::{AppHandle, Manager, Monitor, RunEvent, Theme, UserAttentionType, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

const ICON_PNG: &[u8] = include_bytes!("../../../deploy/vizrs-256.png");
/// Po takim czasie bez odpowiedzi strony (zawieszona, serwer padł) kolejne zamknięcie nie pyta.
const CLOSE_PATIENCE: Duration = Duration::from_secs(5);
/// Okno czeka ukryte na pierwszą klatkę strony (bez białego błysku), ale nie dłużej niż tyle.
const REVEAL_AFTER: Duration = Duration::from_millis(2500);
/// Tło WebView przed narysowaniem strony – kolor `--bg` z `vizrs-ui.css` (Windows, macOS).
#[cfg(not(target_os = "linux"))]
const BACKGROUND: Color = Color(0x0e, 0x10, 0x15, 0xff);
const RELEASES: &str = "https://github.com/sdorpl/VizRS-CG-System/releases";
const CONNECT: &str = "connect";
/// Artist jest osobnym programem (od 2026-11-07).
const ARTIST_MOVED: &str = "Artist (projektowanie scen) to osobny program VizRS CG Artist (vizrs-cg-artist):
https://github.com/sdorpl/VizRS-CG-Artist/releases – pliki .cgpack i .vizrs.json otwiera on.
W przeglądarce zostaje zamrożony /artist serwera.";
/// Most strony: `window.__vizrsPost(tekst)` → komenda `ipc` (`window.ipc` należy w Tauri do Tauri).
/// `menu: true` – strona Trio prosi o systemowe menu pod prawym przyciskiem.
const IPC_SHIM: &str = r#"(() => {
  window.__vizrsPost = (m) => { window.__TAURI_INTERNALS__.invoke('ipc', { body: String(m) }).catch((e) => console.error('ipc', e)); };
  window.__vizrsShell = { version: '__VERSION__', menu: true, os: '__OS__' };
})();"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Trio,
    Output,
    Other,
    /// okno „Połącz z serwerem” (strona okna `ui/connect.html`)
    Connect,
}

impl Kind {
    fn path(self) -> &'static str {
        match self {
            Kind::Trio => "/trio",
            Kind::Output => "/output?checker=1&status=1",
            Kind::Other | Kind::Connect => "/",
        }
    }
    /// Klucz w `window.json` i przedrostek etykiety okna.
    fn key(self) -> &'static str {
        match self {
            Kind::Trio => "trio",
            Kind::Output => "output",
            Kind::Other | Kind::Connect => "inne",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Kind::Trio => "VizRS Trio",
            Kind::Output => "VizRS Wyjście",
            Kind::Other => "VizRS",
            Kind::Connect => "VizRS – połącz z serwerem",
        }
    }
    fn of_label(label: &str) -> Kind {
        match label.split('-').next() {
            Some("trio") => Kind::Trio,
            Some("output") => Kind::Output,
            _ if label == CONNECT => Kind::Connect,
            _ => Kind::Other,
        }
    }
}

struct Args {
    kind: Kind,
    data: PathBuf,
    port: u16,
    /// port podany jawnie (--port / VIZRS_PORT) – bez szukania wolnego
    port_set: bool,
    token: Option<String>,
    connect: Option<String>,
    vizrs: Option<String>,
    local: bool,
    probe: Option<(PathBuf, PathBuf)>,
    /// akceleracja GPU stron: tak, nie, None = domyślnie silnika
    gpu: Option<bool>,
}

fn help() -> &'static str {
    "vizrs-shell – natywne okno VizRS (Tauri 2)

Użycie: vizrs-shell [trio|output|connect] [opcje]   (domyślnie trio; connect – najpierw wybór serwera)

  --data KAT      katalog danych serwera (VIZRS_DATA; domyślnie ./vizrs-data, jeśli jest, inaczej ~/.local/share/vizrs)
  --port N        port serwera uruchamianego przez okno (VIZRS_PORT; domyślnie 8088, a gdy zajęty – następny wolny)
  --token T       token serwera (VIZRS_TOKEN)
  --connect URL   okno dla serwera zdalnego, np. https://cg.example.pl (VIZRS_CONNECT)
  --local         serwer lokalny, nawet gdy zapisano łączenie z serwerem zdalnym przy uruchomieniu
  --gpu TRYB      akceleracja GPU stron: on, off, auto (VIZRS_GPU; Linux)
  --vizrs PLIK    plik vizrs do uruchomienia serwera (VIZRS_BIN; domyślnie obok vizrs-shell albo w PATH)
  --probe JS --probe-out JSON   narzędzie testowe: po wczytaniu strony wykonaj skrypt, zapisz wynik, zakończ

Plik → Połącz z serwerem… (Ctrl+Shift+K): adres i token serwera zdalnego (VPS), lista zapisanych połączeń.

Jeśli serwer na tym katalogu danych już działa, okno dołącza do niego. Inaczej okno uruchamia
`vizrs serve` jako osobny proces: zamknięcie okna nie zdejmuje grafiki z anteny, a serwer kończy pracę
sam, gdy przez 10 s nie ma żadnego okna ani wyjścia (OBS). Działa jedna instancja programu – kolejne
uruchomienie wysuwa istniejące okno (albo otwiera okno wybranego rodzaju).

Artist (projektowanie scen) to osobny program VizRS CG Artist (vizrs-cg-artist)."
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
    let mut a = Args {
        kind: Kind::Trio,
        data: env("VIZRS_DATA").map(PathBuf::from).unwrap_or_else(paths::default_data),
        port: env("VIZRS_PORT").map(|p| p.parse().map_err(|_| "VIZRS_PORT: zły numer portu".to_string())).transpose()?.unwrap_or(8088),
        port_set: env("VIZRS_PORT").is_some(),
        token: env("VIZRS_TOKEN"),
        connect: env("VIZRS_CONNECT"),
        vizrs: env("VIZRS_BIN"),
        local: false,
        probe: None,
        gpu: match env("VIZRS_GPU").as_deref() {
            Some("on") => Some(true),
            Some("off") => Some(false),
            _ => None,
        },
    };
    let mut probe_js = None;
    let mut probe_out = None;
    let mut i = 0;
    while i < argv.len() {
        let flag = argv[i].clone();
        let mut val = || -> Result<String, String> {
            i += 1;
            argv.get(i).cloned().ok_or(format!("{flag}: brak wartości"))
        };
        match flag.as_str() {
            "artist" => return Err(ARTIST_MOVED.into()),
            "trio" => a.kind = Kind::Trio,
            "output" => a.kind = Kind::Output,
            "connect" => a.kind = Kind::Connect,
            "--data" => a.data = PathBuf::from(val()?),
            "--port" => {
                a.port = val()?.parse().map_err(|_| "--port: zły numer portu".to_string())?;
                a.port_set = true;
            }
            "--token" => a.token = Some(val()?),
            "--connect" => a.connect = Some(val()?),
            "--vizrs" => a.vizrs = Some(val()?),
            "--local" => a.local = true,
            "--gpu" => {
                a.gpu = match val()?.as_str() {
                    "on" => Some(true),
                    "off" => Some(false),
                    "auto" => None,
                    _ => return Err("--gpu: podaj on, off albo auto".into()),
                }
            }
            "--probe" => probe_js = Some(PathBuf::from(val()?)),
            "--probe-out" => probe_out = Some(PathBuf::from(val()?)),
            "--help" | "-h" => {
                println!("{}", help());
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("vizrs-shell {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            // macOS dopisuje numer procesu przy starcie z Findera
            other if other.starts_with("-psn_") => {}
            other if !other.starts_with('-') && Path::new(other).is_file() => return Err(ARTIST_MOVED.into()),
            other => return Err(format!("nieznany argument {other} (vizrs-shell --help)")),
        }
        i += 1;
    }
    if let Some(c) = &a.connect {
        let c = c.trim().trim_end_matches('/');
        if !(c.starts_with("http://") || c.starts_with("https://")) || c.contains(char::is_whitespace) {
            return Err(format!("--connect: podaj adres http:// albo https://, nie „{c}”"));
        }
        a.connect = Some(c.to_string());
    }
    a.probe = match (probe_js, probe_out) {
        (Some(j), Some(o)) => Some((j, o)),
        (None, None) => None,
        _ => return Err("--probe i --probe-out podaje się razem".into()),
    };
    Ok(a)
}

/// Adres strony okna: baza serwera + ścieżka + token.
fn page_url(base: &str, path: &str, token: Option<&str>) -> String {
    match token {
        Some(t) => format!("{base}{path}{}token={t}", if path.contains('?') { '&' } else { '?' }),
        None => format!("{base}{path}"),
    }
}

/// Adres strony należy do bieżącego serwera (albo jest stroną wewnętrzną WebView).
fn same_origin(base: &str, u: &str) -> bool {
    u.starts_with(&format!("{base}/")) || u == base || u.starts_with("about:") || u.starts_with("blob:") || u.starts_with("data:")
}

/// Adres albo katalog w programie systemowym (odnośniki spoza serwera VizRS, logi, katalog danych).
fn open_external(target: &str) {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", target]).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(target).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(target).spawn();
    if let Err(e) = r {
        eprintln!("nie mogę otworzyć {target}: {e}");
    }
}

/// Stan okna (Trio, wyjście, inne) zgłaszany przez stronę.
struct Win {
    kind: Kind,
    dirty: bool,
    /// scena zgłoszona przez stronę (most); zmiana = sprawdzenie kopii roboczej
    scene: Option<String>,
    /// kiedy zapytano stronę o zamknięcie
    asked: Option<Instant>,
    /// strona zgodziła się zamknąć okno
    closing: bool,
    /// strona zgłosiła stan wymagający uwagi (brak połączenia)
    alert: bool,
    /// pełnoekranowe wyjście na monitorze: bez zapamiętywania wyglądu
    keep_state: bool,
    zoom: f64,
    /// okno pokazuje się zmaksymalizowane (tworzone niewidoczne)
    maximize: bool,
}

struct App {
    /// adres serwera okien
    base: String,
    /// adres serwera lokalnego (uruchomionego albo dołączonego), None przy `--connect`
    local: Option<String>,
    /// token serwera lokalnego (`--token`)
    local_token: Option<String>,
    /// opcje do uruchomienia serwera lokalnego na żądanie (po starcie z serwerem zdalnym)
    local_start: (PathBuf, u16, Option<String>),
    token: Option<String>,
    probe: Option<(String, PathBuf)>,
    wins: HashMap<String, Win>,
    focused: Option<String>,
    server_note: String,
    data: PathBuf,
    /// Zakończ w toku: kolejne okna z pytaniem o zapis
    quitting: bool,
    gpu: Option<bool>,
    next: u32,
    monitors: Vec<Monitor>,
    /// serwery, których strony dostały uprawnienie do mostu
    allowed: HashSet<String>,
    /// otwarte menu pod prawym przyciskiem: okno i identyfikatory pozycji → komendy strony
    ctx: Option<(String, HashMap<String, String>)>,
}

impl App {
    fn url(&self, kind: Kind) -> String {
        page_url(&self.base, kind.path(), self.token.as_deref())
    }

    /// Tytuł okna; przy serwerze zdalnym z nazwą hosta.
    fn title(&self, kind: Kind) -> String {
        match kind {
            Kind::Connect => kind.title().to_string(),
            _ if self.local.as_deref() == Some(self.base.as_str()) => kind.title().to_string(),
            _ => format!("{} – {}", kind.title(), self.base.split("://").nth(1).unwrap_or(&self.base)),
        }
    }

    /// Okno, do którego trafiają komendy menu: aktywne, inaczej dowolne.
    fn target_label(&self) -> Option<String> {
        self.focused.clone().filter(|l| self.wins.contains_key(l)).or_else(|| self.wins.keys().next().cloned())
    }
}

type Shared = Mutex<App>;
/// Pobrania w toku: adres → plik tymczasowy (macOS nie podaje ścieżki w zdarzeniu końca).
type Downloads = Mutex<HashMap<String, PathBuf>>;

fn state(app: &AppHandle) -> std::sync::MutexGuard<'_, App> {
    app.state::<Shared>().inner().lock().unwrap_or_else(|p| p.into_inner())
}

fn eval_in(app: &AppHandle, label: &str, js: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.eval(js);
    }
}

fn error(app: &AppHandle, msg: &str) {
    eprintln!("{msg}");
    app.dialog().message(msg.to_string()).title("VizRS").kind(MessageDialogKind::Error).show(|_| {});
}

fn script() -> String {
    IPC_SHIM.replace("__VERSION__", env!("CARGO_PKG_VERSION")).replace("__OS__", std::env::consts::OS)
}

/// Strony bieżącego serwera mogą wołać komendę `ipc` (uprawnienie dodawane w działaniu – serwer wybiera
/// operator, więc nie da się go wpisać do `capabilities/`). Komenda i tak sprawdza okno i adres strony.
fn allow_server(app: &AppHandle, base: &str) {
    let n = {
        let mut a = state(app);
        if !a.allowed.insert(base.to_string()) {
            return;
        }
        a.allowed.len()
    };
    let cap = CapabilityBuilder::new(format!("serwer-{n}")).remote(format!("{base}/*")).windows(["trio-*", "output-*", "inne-*"]).permission("allow-ipc");
    if let Err(e) = app.add_capability(cap) {
        eprintln!("uprawnienie mostu dla {base}: {e}");
    }
}

/// Wygląd okna: ciemny pasek tytułu i menu (Windows: DWM, Linux: ciemny wariant GTK, macOS: ciemny wygląd),
/// tło jak strona, bez funkcji przeglądarki (autouzupełnianie WebView2, podgląd odnośników macOS).
fn native_look<'a>(b: WebviewWindowBuilder<'a, tauri::Wry, AppHandle>) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    let b = b.theme(Some(Theme::Dark)).devtools(true).general_autofill_enabled(false);
    #[cfg(not(target_os = "linux"))]
    let b = b.background_color(BACKGROUND);
    // Windows: dane silnika (pamięć podręczna, localStorage stron) tam, gdzie trzymało je okno przed Tauri;
    // skróty przeglądarki WebView2 (Ctrl+F, Ctrl+P…) wyłącza strona (`web/native.js`)
    #[cfg(windows)]
    let b = b.data_directory(paths::state_dir().join("WebView2"));
    #[cfg(target_os = "macos")]
    let b = b.allow_link_preview(false);
    b
}

/// Pokaż okno (tworzone niewidoczne, żeby nie mignęło pustą stroną).
fn reveal(app: &AppHandle, label: &str) {
    let maximize = state(app).wins.get(label).is_some_and(|w| w.maximize);
    if let Some(w) = app.get_webview_window(label) {
        if !w.is_visible().unwrap_or(true) {
            let _ = w.show();
            if maximize {
                let _ = w.maximize();
            }
            let _ = w.set_focus();
        }
    }
}

/// Ustawienia WebKitGTK: pisownia polska w polach tekstowych, akceleracja GPU stron (`--gpu`).
fn webkit_settings(w: &WebviewWindow, gpu: Option<bool>) {
    #[cfg(target_os = "linux")]
    let _ = w.with_webview(move |pv| {
        use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebContextExt, WebViewExt};
        let wv = pv.inner();
        if let (Some(on), Some(st)) = (gpu, WebViewExt::settings(&wv)) {
            st.set_hardware_acceleration_policy(if on { HardwareAccelerationPolicy::Always } else { HardwareAccelerationPolicy::Never });
        }
        if let Some(ctx) = wv.context() {
            ctx.set_spell_checking_enabled(true);
            ctx.set_spell_checking_languages(&["pl_PL"]);
        }
    });
    #[cfg(not(target_os = "linux"))]
    let _ = (w, gpu);
}

/// Okno strony serwera; `monitor` – pełnoekranowe wyjście na wskazanym monitorze (bez menu i zapamiętywania).
fn open_window(app: &AppHandle, kind: Kind, url: String, monitor: Option<Monitor>) {
    let parsed = match url.parse() {
        Ok(u) => u,
        Err(_) => return error(app, &format!("zły adres strony: {url}")),
    };
    let keep_state = matches!(kind, Kind::Trio | Kind::Output) && monitor.is_none();
    let st = if keep_state { paths::load(kind.key()) } else { paths::WinState::default() };
    // Trio to stanowisko pracy – bez zapisanego wyglądu od razu na cały ekran (wyjście: okno 960×540)
    let maximize = monitor.is_none() && st.maximized.unwrap_or(kind == Kind::Trio);
    let (label, title, base, gpu) = {
        let mut a = state(app);
        a.next += 1;
        let label = format!("{}-{}", kind.key(), a.next);
        a.wins.insert(label.clone(), Win { kind, dirty: false, scene: None, asked: None, closing: false, alert: false, keep_state, zoom: st.zoom.unwrap_or(1.0), maximize });
        (label, a.title(kind), a.base.clone(), a.gpu)
    };
    allow_server(app, &base);
    let (w, h) = st.size.unwrap_or(if kind == Kind::Output { (960.0, 540.0) } else { (1600.0, 900.0) });
    let (app_dl, app_nav, app_new) = (app.clone(), app.clone(), app.clone());
    let mut b = native_look(WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed)))
        .title(&title)
        .inner_size(w, h)
        .min_inner_size(480.0, 270.0)
        .visible(false)
        .initialization_script(script())
        .on_download(move |_, ev| on_download(&app_dl, ev))
        .on_page_load(|w, p| {
            if p.event() == PageLoadEvent::Finished {
                page_loaded(w.app_handle(), w.label());
            }
        })
        // odnośniki poza serwer VizRS otwiera przeglądarka systemowa
        .on_navigation(move |u| {
            let base = state(&app_nav).base.clone();
            if same_origin(&base, u.as_str()) {
                true
            } else {
                open_external(u.as_str());
                false
            }
        })
        // window.open / target=_blank: strony serwera w nowym oknie, reszta w przeglądarce
        .on_new_window(move |u, _| {
            let u = u.to_string();
            let base = state(&app_new).base.clone();
            if same_origin(&base, &u) {
                let app2 = app_new.clone();
                let _ = app_new.run_on_main_thread(move || {
                    let kind = if u.contains("/output") { Kind::Output } else { Kind::Other };
                    open_window(&app2, kind, u, None);
                });
            } else {
                open_external(&u);
            }
            NewWindowResponse::Deny
        });
    if let (Some((x, y)), None) = (st.pos, &monitor) {
        b = b.position(x as f64, y as f64);
    }
    if let Some(m) = &monitor {
        let (p, f) = (m.position(), m.scale_factor());
        b = b.position(p.x as f64 / f, p.y as f64 / f).fullscreen(true);
    }
    match b.build() {
        Ok(w) => {
            // wyjście na monitorze to czysty obraz, bez paska menu
            #[cfg(not(target_os = "macos"))]
            if monitor.is_some() {
                let _ = w.remove_menu();
            }
            if let Some(z) = st.zoom.filter(|z| (*z - 1.0).abs() > 0.01) {
                let _ = w.set_zoom(z);
            }
            webkit_settings(&w, gpu);
            // Linux: nowe okno ustawia wariant GTK z ustawień systemu – przywróć ciemny
            app.set_theme(Some(Theme::Dark));
            state(app).focused = Some(label.clone());
            // strona nie wczytała się szybko (serwer nie odpowiada) – okno i tak się pokazuje
            let (app2, l2) = (app.clone(), label.clone());
            std::thread::spawn(move || {
                std::thread::sleep(REVEAL_AFTER);
                let app3 = app2.clone();
                let _ = app2.run_on_main_thread(move || reveal(&app3, &l2));
            });
        }
        Err(e) => {
            state(app).wins.remove(&label);
            error(app, &format!("Nie udało się otworzyć okna: {e}"));
        }
    }
}

fn page_loaded(app: &AppHandle, label: &str) {
    if label != CONNECT {
        reveal(app, label);
    }
    let probe = state(app).probe.as_ref().map(|(c, _)| c.clone());
    if let Some(code) = probe {
        eval_in(app, label, &code);
    }
}

/// Pełnoekranowe wyjście na wskazanym monitorze (czyste `/output`; Fill/Key przez `?mode=`).
fn open_screen(app: &AppHandle, n: usize, query: &str) {
    let (m, url) = {
        let a = state(app);
        let Some(m) = a.monitors.get(n).cloned() else { return };
        (m, page_url(&a.base, &format!("/output{query}"), a.token.as_deref()))
    };
    open_window(app, Kind::Output, url, Some(m));
}

fn open_connect(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(CONNECT) {
        let _ = w.set_focus();
        return;
    }
    let data = {
        let a = state(app);
        connect::page_data(&connect::Saved::load(), &a.base, a.token.as_deref(), a.local.as_deref())
    };
    // okno dialogowe: stały rozmiar, nad oknem Trio (jeśli jest), bez paska menu programu
    let parent = state(app).target_label().and_then(|l| app.get_webview_window(&l));
    let mut b = native_look(WebviewWindowBuilder::new(app, CONNECT, WebviewUrl::App("connect.html".into())))
        .title(Kind::Connect.title())
        .inner_size(560.0, 600.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(parent.is_none())
        .center()
        .initialization_script(format!("{}\n{data}", script()))
        .on_page_load(|w, p| {
            if p.event() == PageLoadEvent::Finished {
                page_loaded(w.app_handle(), w.label());
            }
        });
    if let Some(p) = &parent {
        b = match b.parent(p) {
            Ok(b) => b,
            Err(e) => return error(app, &format!("Nie udało się otworzyć okna połączenia: {e}")),
        };
    }
    match b.build() {
        Ok(w) => {
            #[cfg(not(target_os = "macos"))]
            let _ = w.remove_menu();
            app.set_theme(Some(Theme::Dark));
            let _ = w.set_focus();
        }
        Err(e) => error(app, &format!("Nie udało się otworzyć okna połączenia: {e}")),
    }
}

fn close_connect(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(CONNECT) {
        let _ = w.destroy();
    }
}

fn conn_error(app: &AppHandle, msg: &str) {
    eval_in(app, CONNECT, &format!("window.__vizrsConnError && window.__vizrsConnError({})", Value::from(msg)));
}

/// Przełącza wszystkie okna na inny serwer (okno połączenia się zamyka); bez okien – otwiera Trio.
fn switch(app: &AppHandle, base: String, token: Option<String>, note: String) {
    let targets: Vec<(String, String, String)> = {
        let mut a = state(app);
        a.base = base.clone();
        a.token = token;
        a.server_note = note;
        a.wins.iter().map(|(l, w)| (l.clone(), a.url(w.kind), a.title(w.kind))).collect()
    };
    allow_server(app, &base);
    close_connect(app);
    if targets.is_empty() {
        let url = state(app).url(Kind::Trio);
        return open_window(app, Kind::Trio, url, None);
    }
    for (l, url, title) in targets {
        if let Some(w) = app.get_webview_window(&l) {
            let _ = w.set_title(&title);
            if let Ok(u) = url.parse() {
                let _ = w.navigate(u);
            }
        }
    }
}

/// Prośba kolejnego uruchomienia: istniejące okno na wierzch albo nowe.
fn on_remote(app: &AppHandle, argv: &[String]) {
    let kind = parse_args(argv).map(|a| a.kind).unwrap_or(Kind::Trio);
    if kind == Kind::Connect {
        return open_connect(app);
    }
    let found = state(app).wins.iter().find(|(_, w)| w.kind == kind).map(|(l, _)| l.clone());
    match found.and_then(|l| app.get_webview_window(&l)) {
        Some(w) => {
            let _ = w.unminimize();
            let _ = w.set_focus();
        }
        None => {
            let url = state(app).url(kind);
            open_window(app, kind, url, None);
        }
    }
}

// ------------------------------------------------------------------ most

#[tauri::command]
fn ipc(app: AppHandle, webview: tauri::Webview, body: String) {
    if let Some(rest) = body.strip_prefix("probe:") {
        let out = state(&app).probe.as_ref().map(|(_, o)| o.clone());
        if let Some(out) = out {
            if let Err(e) = std::fs::write(&out, rest) {
                eprintln!("{}: {e}", out.display());
            }
            app.exit(0);
        }
        return;
    }
    let label = webview.label().to_string();
    if label == CONNECT {
        return on_conn(&app, &body);
    }
    let Some(rest) = body.strip_prefix("app:") else { return };
    // tylko okna Trio ze stroną bieżącego serwera (nie „about:” ani „data:”)
    let from = webview.url().map(|u| u.to_string()).unwrap_or_default();
    let base = state(&app).base.clone();
    if Kind::of_label(&label) != Kind::Trio || !from.starts_with(&format!("{base}/")) {
        return;
    }
    let Some(msg) = bridge::parse(rest) else { return };
    match msg {
        bridge::Msg::State { title, dirty, alert, scene, .. } => {
            let (suffix, draft, attention) = {
                let mut a = state(&app);
                let suffix = a.title(Kind::Other).strip_prefix("VizRS").unwrap_or("").to_string();
                let focused = a.focused.as_deref() == Some(label.as_str());
                let Some(w) = a.wins.get_mut(&label) else { return };
                w.dirty = dirty;
                // brak połączenia: okno w tle miga w pasku zadań (Windows), skacze w Docku (macOS), prosi o uwagę (Linux)
                let attention = (alert != w.alert).then(|| {
                    w.alert = alert;
                    (alert && !focused).then_some(UserAttentionType::Critical)
                });
                let mut draft = None;
                if scene.is_some() && scene != w.scene {
                    w.scene = scene.clone();
                    draft = scene.and_then(|s| std::fs::read_to_string(bridge::draft_path(&base, &s)).ok());
                }
                (suffix, draft, attention)
            };
            if let Some(w) = app.get_webview_window(&label) {
                let _ = w.set_title(&format!("{title}{suffix}"));
                if let Some(att) = attention {
                    let _ = w.request_user_attention(att);
                }
            }
            if let Some(text) = draft {
                eval_in(&app, &label, &format!("window.__vizrsCmd && window.__vizrsCmd('draft', {})", Value::from(text)));
            }
        }
        bridge::Msg::Close => {
            let quitting = {
                let mut a = state(&app);
                if let Some(w) = a.wins.get_mut(&label) {
                    w.closing = true;
                }
                a.quitting
            };
            if let Some(w) = app.get_webview_window(&label) {
                let _ = w.close();
            }
            if quitting {
                quit_step(&app);
            }
        }
        bridge::Msg::Stay => {
            let mut a = state(&app);
            a.quitting = false;
            if let Some(w) = a.wins.get_mut(&label) {
                w.asked = None;
            }
        }
        bridge::Msg::Menu(entries) => context_menu(&app, &label, entries),
        bridge::Msg::Draft { id, data } => {
            let p = bridge::draft_path(&base, &id);
            match data {
                Some(d) => {
                    if let Err(e) = paths::write_atomic(&p, &d) {
                        eprintln!("kopia robocza: {e}");
                    }
                }
                None => {
                    std::fs::remove_file(p).ok();
                }
            }
        }
    }
}

/// Systemowe menu pod prawym przyciskiem w miejscu kursora (pozycje podaje strona Trio).
fn context_menu(app: &AppHandle, label: &str, entries: Vec<bridge::MenuEntry>) {
    let Some(w) = app.get_webview_window(label) else { return };
    let Ok(menu) = Menu::new(app) else { return };
    let mut map = HashMap::new();
    for (i, e) in entries.into_iter().enumerate() {
        let ok = match e {
            bridge::MenuEntry::Separator => PredefinedMenuItem::separator(app).and_then(|s| menu.append(&s)),
            bridge::MenuEntry::Item { id: cmd, label, enabled } => {
                let id = format!("ctx:{i}");
                map.insert(id.clone(), cmd);
                MenuItem::with_id(app, &id, label, enabled, None::<&str>).and_then(|it| menu.append(&it))
            }
        };
        if ok.is_err() {
            return;
        }
    }
    state(app).ctx = Some((label.to_string(), map));
    let _ = w.popup_menu(&menu);
}

fn on_conn(app: &AppHandle, body: &str) {
    let msg = match connect::parse_msg(body) {
        Ok(m) => m,
        Err(e) => return conn_error(app, &e),
    };
    match msg {
        connect::Msg::Cancel => {
            close_connect(app);
            if state(app).wins.is_empty() {
                app.exit(0);
            }
        }
        connect::Msg::Forget(url) => {
            let mut saved = connect::Saved::load();
            saved.forget(&url);
            if let Err(e) = saved.save() {
                conn_error(app, &e);
            }
        }
        connect::Msg::Local => {
            let (local, start, token) = {
                let a = state(app);
                (a.local.clone(), a.local_start.clone(), a.local_token.clone())
            };
            let local = match local {
                Some(l) => l,
                None => match local_server(&start.0, start.1, token.as_deref(), start.2.as_deref()) {
                    Ok((base, _)) => {
                        state(app).local = Some(base.clone());
                        base
                    }
                    Err(e) => return conn_error(app, &e),
                },
            };
            switch(app, local, token, "serwer lokalny".into());
        }
        connect::Msg::Connect(mut req) => {
            // puste pole tokenu = token zapamiętany dla tego serwera
            if req.token.is_none() {
                req.token = connect::Saved::load().token_for(&req.url);
            }
            let app = app.clone();
            std::thread::spawn(move || {
                let err = check_server(&req.url, req.token.as_deref()).err();
                let app2 = app.clone();
                let _ = app.run_on_main_thread(move || on_checked(&app2, req, err));
            });
        }
    }
}

fn on_checked(app: &AppHandle, req: connect::Request, err: Option<String>) {
    if let Some(e) = err {
        return conn_error(app, &e);
    }
    let connect::Request { url, token, keep_token, auto } = req;
    let mut saved = connect::Saved::load();
    saved.remember(&url, token.as_deref(), keep_token, auto);
    if let Err(e) = saved.save() {
        eprintln!("zapis listy serwerów: {e}");
    }
    switch(app, url, token, "serwer zdalny (Plik → Połącz z serwerem)".into());
}

// ------------------------------------------------------------------ zamykanie

/// Zamknięcie okna Trio: ze zmianami pyta przez stronę (ona zna stan i umie zapisać); strona bez odpowiedzi
/// nie zatrzyma okna na zawsze. `true` = wstrzymaj zamknięcie.
fn close_request(app: &AppHandle, label: &str) -> bool {
    let mut a = state(app);
    let Some(w) = a.wins.get_mut(label) else { return false };
    let waited = w.asked.is_some_and(|t| t.elapsed() >= CLOSE_PATIENCE);
    if w.dirty && !w.closing && !waited {
        if w.asked.is_none() {
            w.asked = Some(Instant::now());
            drop(a);
            if let Some(win) = app.get_webview_window(label) {
                let _ = win.set_focus();
            }
            eval_in(app, label, "window.__vizrsCmd && window.__vizrsCmd('close')");
        }
        return true;
    }
    false
}

/// Okno zamyka się: zapamiętaj wygląd (rozmiar, położenie, maksymalizacja, powiększenie, scena).
fn remember(app: &AppHandle, label: &str, w: &tauri::Window) {
    let a = state(app);
    let Some(win) = a.wins.get(label) else { return };
    if !win.keep_state {
        return;
    }
    let key = win.kind.key();
    let old = paths::load(key);
    let maximized = w.is_maximized().unwrap_or(false);
    let f = w.scale_factor().unwrap_or(1.0);
    let size = w.inner_size().ok().map(|s| (s.width as f64 / f, s.height as f64 / f)).filter(|(ww, _)| *ww >= 1.0);
    let pos = w.outer_position().ok().map(|p| ((p.x as f64 / f) as i32, (p.y as f64 / f) as i32)).filter(|p| *p != (0, 0));
    paths::save(
        key,
        &paths::WinState {
            // zmaksymalizowane okno nie nadpisuje zwykłego rozmiaru
            size: if maximized { old.size } else { size.or(old.size) },
            pos: if maximized { old.pos } else { pos.or(old.pos) },
            maximized: Some(maximized),
            zoom: Some(win.zoom),
            scene: win.scene.clone().or(old.scene),
        },
    );
}

/// Zakończ: okna bez zmian od razu, ze zmianami po kolei.
fn quit_step(app: &AppHandle) {
    let (clean, asking, next) = {
        let a = state(app);
        let clean: Vec<String> = a.wins.iter().filter(|(_, w)| !w.dirty || w.closing).map(|(l, _)| l.clone()).collect();
        let asking = a.wins.values().any(|w| w.asked.is_some());
        let next = a.wins.iter().find(|(_, w)| w.dirty && !w.closing).map(|(l, _)| l.clone());
        (clean, asking, next)
    };
    for l in clean {
        if let Some(w) = app.get_webview_window(&l) {
            let _ = w.close();
        }
    }
    close_connect(app);
    if !asking {
        if let Some(l) = next.and_then(|l| app.get_webview_window(&l)) {
            let _ = l.close();
        }
    }
    if state(app).wins.is_empty() {
        app.exit(0);
    }
}

// ------------------------------------------------------------------ pobrania

/// Pobranie strony (paczka, kopia show, CSV): najpierw do pliku tymczasowego, po zakończeniu systemowe „Zapisz jako”
/// (blokujące okno w wywołaniu zwrotnym pobrania mogłoby zakleszczyć wątek okna).
fn on_download(app: &AppHandle, ev: DownloadEvent<'_>) -> bool {
    match ev {
        DownloadEvent::Requested { url, destination } => {
            let name = destination
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| url.path().rsplit('/').next().filter(|n| !n.is_empty()).unwrap_or("vizrs-eksport").to_string());
            let dir = std::env::temp_dir().join(format!("vizrs-shell-{}", std::process::id()));
            std::fs::create_dir_all(&dir).ok();
            *destination = dir.join(name);
            if let Ok(mut m) = app.state::<Downloads>().lock() {
                m.insert(url.to_string(), destination.clone());
            }
            true
        }
        DownloadEvent::Finished { url, path, success } => {
            let remembered = app.state::<Downloads>().lock().ok().and_then(|mut m| m.remove(url.as_str()));
            let Some(tmp) = path.or(remembered) else { return true };
            if !success {
                std::fs::remove_file(&tmp).ok();
                error(app, "Nie udało się zapisać pliku ze strony.");
                return true;
            }
            let name = tmp.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let app2 = app.clone();
            app.dialog().file().set_title("Zapisz plik").set_file_name(&name).save_file(move |f| {
                if let Some(dst) = f.and_then(|f| f.into_path().ok()) {
                    let moved = std::fs::rename(&tmp, &dst).or_else(|_| std::fs::copy(&tmp, &dst).map(|_| ()));
                    if let Err(e) = moved {
                        error(&app2, &format!("Nie udało się zapisać {}: {e}", dst.display()));
                    }
                }
                std::fs::remove_file(&tmp).ok();
            });
            true
        }
        _ => true,
    }
}

// ------------------------------------------------------------------ menu

fn monitor_label(i: usize, m: &Monitor) -> String {
    let s = m.size();
    format!("Monitor {}: {} ({}×{})", i + 1, m.name().cloned().unwrap_or_default(), s.width, s.height)
}

fn build_menu(app: &AppHandle, monitors: &[Monitor]) -> tauri::Result<Menu<tauri::Wry>> {
    type Item = dyn tauri::menu::IsMenuItem<tauri::Wry>;
    let mac = cfg!(target_os = "macos");
    let item = |id: &str, label: &str, accel: Option<&str>| MenuItem::with_id(app, id, label, true, accel);
    let sep = || PredefinedMenuItem::separator(app);
    // Windows zamyka program Alt+F4 (system); Ctrl+Q to zwyczaj Linuksa, Cmd+Q – macOS
    let quit = item("quit", if mac { "Zakończ VizRS" } else { "Zakończ" }, if cfg!(target_os = "windows") { None } else { Some("CmdOrCtrl+Q") })?;
    let connect = item("connect", "Połącz z serwerem…", Some("CmdOrCtrl+Shift+K"))?;
    let trio = item("trio", "Nowe okno Trio", None)?;
    let output = item("output", "Okno wyjścia (podgląd)", None)?;
    let close = item("close", "Zamknij okno", Some("CmdOrCtrl+W"))?;
    let (s1, s2, s3) = (sep()?, sep()?, sep()?);
    let mut file: Vec<&Item> = vec![&connect, &s1, &trio, &output, &s2, &close];
    if !mac {
        file.extend([&s3 as &Item, &quit]);
    }

    let reload = item("reload", "Odśwież", Some("F5"))?;
    let full = item("fullscreen", "Pełny ekran", Some("F11"))?;
    let zin = item("zoom-in", "Powiększ", Some("CmdOrCtrl+="))?;
    let zout = item("zoom-out", "Pomniejsz", Some("CmdOrCtrl+-"))?;
    let z0 = item("zoom-reset", "Rozmiar 100%", Some("CmdOrCtrl+0"))?;
    let dev = item("devtools", "Narzędzia deweloperskie", Some("CmdOrCtrl+Shift+I"))?;
    let (s4, s5) = (sep()?, sep()?);
    let view: Vec<&Item> = vec![&reload, &full, &s4, &zin, &zout, &z0, &s5, &dev];

    let obs = item("obs", "Adres dla OBS…", None)?;
    let screens: Vec<MenuItem<tauri::Wry>> = monitors.iter().enumerate().map(|(i, m)| item(&format!("screen:{i}"), &monitor_label(i, m), None)).collect::<tauri::Result<_>>()?;
    let fill_key = if monitors.len() >= 2 { Some(item("fill-key", "Fill + Key na monitorach 1 i 2", None)?) } else { None };
    let (s6, s7) = (sep()?, sep()?);
    let mut out: Vec<&Item> = vec![&obs, &s6];
    out.extend(screens.iter().map(|m| m as &Item));
    if let Some(fk) = &fill_key {
        out.extend([&s7 as &Item, fk]);
    }

    let docs = item("docs", "Strona startowa i API", None)?;
    let logs = item("logs", "Pokaż logi", None)?;
    let data_dir = item("data-dir", "Otwórz katalog danych", None)?;
    let updates = item("updates", "Sprawdź aktualizacje", None)?;
    let about = item("about", if mac { "O programie VizRS" } else { "O programie" }, None)?;
    let (s8, s9, s10) = (sep()?, sep()?, sep()?);
    let mut help: Vec<&Item> = vec![&docs, &s8, &logs, &data_dir, &s9, &updates];
    if !mac {
        help.extend([&s10 as &Item, &about]);
    }

    let help_menu = Submenu::with_items(app, "Pomo&c", true, &help)?;
    #[allow(unused_mut)]
    let mut subs = vec![
        Submenu::with_items(app, "&Plik", true, &file)?,
        Submenu::with_items(app, "&Widok", true, &view)?,
        Submenu::with_items(app, "W&yjście", true, &out)?,
    ];
    // macOS: menu aplikacji, Edycja (bez niej Cmd+C/V/X/A/Z nie działają w polach WKWebView) i Okno
    #[cfg(target_os = "macos")]
    {
        let app_menu = Submenu::with_items(
            app,
            "VizRS",
            true,
            &[
                &about as &Item,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::services(app, Some("Usługi"))?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::hide(app, Some("Ukryj VizRS"))?,
                &PredefinedMenuItem::hide_others(app, Some("Ukryj pozostałe"))?,
                &PredefinedMenuItem::show_all(app, Some("Pokaż wszystko"))?,
                &PredefinedMenuItem::separator(app)?,
                &quit,
            ],
        )?;
        let edit = Submenu::with_items(
            app,
            "Edycja",
            true,
            &[
                &PredefinedMenuItem::undo(app, Some("Cofnij"))? as &Item,
                &PredefinedMenuItem::redo(app, Some("Ponów"))?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::cut(app, Some("Wytnij"))?,
                &PredefinedMenuItem::copy(app, Some("Kopiuj"))?,
                &PredefinedMenuItem::paste(app, Some("Wklej"))?,
                &PredefinedMenuItem::select_all(app, Some("Zaznacz wszystko"))?,
            ],
        )?;
        let window_menu = Submenu::with_items(app, "Okno", true, &[&PredefinedMenuItem::minimize(app, Some("Zminimalizuj"))? as &Item, &PredefinedMenuItem::maximize(app, Some("Powiększ okno"))?])?;
        window_menu.set_as_windows_menu_for_nsapp()?;
        help_menu.set_as_help_menu_for_nsapp()?;
        subs.insert(0, app_menu);
        subs.insert(2, edit);
        subs.push(window_menu);
    }
    subs.push(help_menu);
    let refs: Vec<&Item> = subs.iter().map(|s| s as &Item).collect();
    Menu::with_items(app, &refs)
}

fn on_menu(app: &AppHandle, id: &str) {
    if id.starts_with("ctx:") {
        let hit = state(app).ctx.as_ref().and_then(|(l, map)| map.get(id).map(|c| (l.clone(), c.clone())));
        if let Some((l, cmd)) = hit {
            eval_in(app, &l, &format!("window.__vizrsCmd && window.__vizrsCmd('menu', {})", Value::from(cmd)));
        }
        return;
    }
    if let Some(n) = id.strip_prefix("screen:").and_then(|n| n.parse().ok()) {
        return open_screen(app, n, "");
    }
    let target = state(app).target_label();
    let win = target.as_deref().and_then(|l| app.get_webview_window(l));
    let zoom = |f: fn(f64) -> f64| {
        let (Some(l), Some(w)) = (target.as_deref(), win.as_ref()) else { return };
        let z = {
            let mut a = state(app);
            let Some(st) = a.wins.get_mut(l) else { return };
            st.zoom = f(st.zoom).clamp(0.3, 3.0);
            st.zoom
        };
        let _ = w.set_zoom(z);
    };
    let open_kind = |k: Kind| {
        let url = state(app).url(k);
        open_window(app, k, url, None);
    };
    match id {
        "connect" => open_connect(app),
        "trio" => open_kind(Kind::Trio),
        "output" => open_kind(Kind::Output),
        "docs" => open_kind(Kind::Other),
        "fill-key" => {
            open_screen(app, 0, "?mode=fill");
            open_screen(app, 1, "?mode=key");
        }
        "obs" => {
            // adres zna strona Trio (host, port, antena, token); okno wyjścia go nie pokazuje
            let trio = {
                let a = state(app);
                a.focused.clone().filter(|f| a.wins.get(f).is_some_and(|w| w.kind == Kind::Trio)).or_else(|| a.wins.iter().find(|(_, w)| w.kind == Kind::Trio).map(|(l, _)| l.clone()))
            };
            match trio.and_then(|l| app.get_webview_window(&l)) {
                Some(w) => {
                    let _ = w.set_focus();
                    let _ = w.eval("window.__vizrsCmd && window.__vizrsCmd('obs')");
                }
                None => error(app, "Adres dla OBS pokazuje okno Trio – otwórz je: Plik → Nowe okno Trio."),
            }
        }
        "close" => {
            if let Some(w) = win {
                let _ = w.close();
            }
        }
        "quit" => {
            state(app).quitting = true;
            quit_step(app);
        }
        "reload" => {
            if let Some(w) = win {
                let _ = w.eval("location.reload()");
            }
        }
        "fullscreen" => {
            if let Some(w) = win {
                let _ = w.set_fullscreen(!w.is_fullscreen().unwrap_or(false));
            }
        }
        "zoom-in" => zoom(|z| z * 1.1),
        "zoom-out" => zoom(|z| z / 1.1),
        "zoom-reset" => zoom(|_| 1.0),
        "devtools" => {
            if let Some(w) = win {
                w.open_devtools();
            }
        }
        "logs" => open_external(&paths::state_dir().to_string_lossy()),
        "data-dir" => {
            let d = state(app).data.clone();
            open_external(&std::fs::canonicalize(&d).unwrap_or(d).to_string_lossy());
        }
        "updates" => open_external(RELEASES),
        "about" => {
            let msg = {
                let a = state(app);
                format!("VizRS Trio – okno natywne (vizrs-shell {}, Tauri 2)\nSerwer: {}\n{}", env!("CARGO_PKG_VERSION"), a.base, a.server_note)
            };
            app.dialog().message(msg).title("O programie").show(|_| {});
        }
        _ => {}
    }
}

// ------------------------------------------------------------------ start

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let probe = args.probe.as_ref().map(|(js, out)| match std::fs::read_to_string(js) {
        Ok(code) => (code, out.clone()),
        Err(e) => {
            eprintln!("{}: {e}", js.display());
            std::process::exit(2);
        }
    });
    if args.probe.is_none() {
        desktop::ensure_entry(ICON_PNG);
    }
    let auto = if args.connect.is_none() && !args.local { connect::Saved::load().auto } else { None };
    let app_state = App {
        base: String::new(),
        local: None,
        local_token: args.token.clone(),
        local_start: (args.data.clone(), args.port, args.vizrs.clone()),
        token: args.token.clone(),
        probe,
        wins: HashMap::new(),
        focused: None,
        server_note: String::new(),
        data: args.data.clone(),
        quitting: false,
        gpu: args.gpu,
        next: 0,
        monitors: vec![],
        allowed: HashSet::new(),
        ctx: None,
    };
    let mut builder = tauri::Builder::default();
    // jedna instancja: kolejne uruchomienie przekazuje prośbę (rodzaj okna, okno połączenia) i kończy pracę
    if args.connect.is_none() && args.probe.is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let argv: Vec<String> = argv.into_iter().skip(1).collect();
            on_remote(app, &argv);
        }));
    }
    let first = args.kind;
    let built = builder
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(app_state))
        .manage(Downloads::default())
        .invoke_handler(tauri::generate_handler![ipc])
        .setup(move |app| {
            let h = app.handle().clone();
            // interfejs jest ciemny, więc i to, co rysuje system (menu, okna dialogowe, „Zapisz jako”)
            h.set_theme(Some(Theme::Dark));
            let monitors = h.available_monitors().unwrap_or_default();
            let menu = build_menu(&h, &monitors)?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, ev| on_menu(app, ev.id().as_ref()));
            // serwer: zdalny (--connect albo zapisany „przy uruchomieniu”), działający na tym katalogu danych albo uruchamiany teraz;
            // domyślny port zajęty (np. serwer na innym katalogu danych) – następny wolny; jawny --port bez zmian
            let port = if args.port_set { args.port } else { server::free_port(args.port) };
            let started = if let Some(c) = &args.connect {
                Ok((c.clone(), None, args.token.clone(), "serwer zdalny (--connect)".to_string()))
            } else if let Some(a) = auto {
                let t = connect::Saved::load().token_for(&a);
                Ok((a, None, t, "serwer zdalny (łączenie przy uruchomieniu; serwer lokalny: --local)".to_string()))
            } else {
                local_server(&args.data, port, args.token.as_deref(), args.vizrs.as_deref()).map(|(b, note)| (b.clone(), Some(b), args.token.clone(), note))
            };
            let (base, local, token, note) = match started {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    let h2 = h.clone();
                    h.dialog().message(e).title("VizRS").kind(MessageDialogKind::Error).show(move |_| h2.exit(1));
                    return Ok(());
                }
            };
            {
                let mut a = state(&h);
                (a.base, a.local, a.token, a.server_note, a.monitors) = (base, local, token, note, monitors);
                a.local_start.1 = port;
            }
            if first == Kind::Connect {
                open_connect(&h);
            } else {
                let url = state(&h).url(first);
                open_window(&h, first, url, None);
            }
            Ok(())
        })
        .on_window_event(|w, ev| {
            let label = w.label().to_string();
            let app = w.app_handle();
            match ev {
                WindowEvent::CloseRequested { api, .. } if label != CONNECT => {
                    if close_request(app, &label) {
                        api.prevent_close();
                    } else {
                        remember(app, &label, w);
                    }
                }
                WindowEvent::Focused(true) if label != CONNECT => state(app).focused = Some(label),
                WindowEvent::Destroyed => {
                    let quitting = {
                        let mut a = state(app);
                        a.wins.remove(&label);
                        a.quitting
                    };
                    if quitting && label != CONNECT {
                        quit_step(app);
                    }
                    if app.webview_windows().is_empty() {
                        app.exit(0);
                    }
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!());
    let app = match built {
        Ok(a) => a,
        Err(e) => {
            eprintln!("vizrs-shell: {e}");
            std::process::exit(1);
        }
    };
    app.run(|_, ev| {
        if let RunEvent::ExitRequested { .. } = ev {
            // nic do zwolnienia: serwer `vizrs` to osobny proces, kończy pracę sam (--idle-exit)
        }
    });
}

/// Serwer lokalny: działający na tym katalogu danych albo uruchamiany teraz. Zwraca (adres, opis).
fn local_server(data: &Path, port: u16, token: Option<&str>, vizrs: Option<&str>) -> Result<(String, String), String> {
    if let Some(addr) = server::running_server(data, token)? {
        return Ok((format!("http://{addr}"), format!("dołączono do działającego serwera ({})", data.display())));
    }
    let vizrs = server::find_vizrs(vizrs).ok_or("Nie znaleziono programu vizrs (obok vizrs-shell ani w PATH) – wskaż go: --vizrs ŚCIEŻKA albo VIZRS_BIN.")?;
    let s = server::spawn(&vizrs, data, port, token).map_err(|e| format!("Serwer VizRS nie wystartował:\n{e}"))?;
    Ok((format!("http://{}", s.addr), format!("serwer uruchomiony przez okno ({}), log: {}", data.display(), s.log.display())))
}

/// Przed przełączeniem okien: `/api/state` serwera `http://` (token, dostępność). `https://` sprawdza dopiero strona
/// (okno nie ma TLS) – wskaźnik połączenia w Trio.
fn check_server(url: &str, token: Option<&str>) -> Result<(), String> {
    let Some((addr, path)) = connect::check_target(url) else { return Ok(()) };
    match server::http_get(&addr, &path, token, Duration::from_secs(3)) {
        Ok((200, body)) if body.trim_start().starts_with('{') => Ok(()),
        Ok((200, _)) => Err(format!("{url} odpowiada, ale to nie jest serwer VizRS")),
        Ok((401, _)) => Err(if token.is_some() { "Serwer odrzucił token.".into() } else { "Serwer wymaga tokenu.".into() }),
        Ok((403, _)) => Err("Ten token nie pozwala sterować (to token wyjścia OBS?).".into()),
        Ok((code, _)) => Err(format!("{url}: odpowiedź HTTP {code}")),
        Err(e) => Err(format!("Brak połączenia z serwerem: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Result<Args, String> {
        parse_args(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn urls_with_token() {
        assert_eq!(page_url("http://h:1", "/trio", None), "http://h:1/trio");
        assert_eq!(page_url("http://h:1", "/trio", Some("t")), "http://h:1/trio?token=t");
        assert_eq!(page_url("http://h:1", "/output?checker=1", Some("t")), "http://h:1/output?checker=1&token=t");
    }

    #[test]
    fn command_line() {
        let a = args(&["output", "--data", "/tmp/x", "--token", "t", "--port", "9000"]).unwrap();
        assert_eq!(a.kind, Kind::Output);
        assert_eq!(a.data, PathBuf::from("/tmp/x"));
        assert_eq!((a.port, a.port_set), (9000, true));
        assert_eq!(args(&["--connect", "https://cg.example.pl/"]).unwrap().connect.as_deref(), Some("https://cg.example.pl"));
        assert!(args(&["--connect", "cg.example.pl"]).is_err());
        assert!(args(&["artist"]).err().is_some_and(|e| e.contains("VizRS CG Artist")));
        assert!(args(&["--probe", "a.js"]).is_err());
        assert!(args(&["--nieznana"]).is_err());
        assert!(args(&["-psn_0_12345"]).is_ok());
    }

    #[test]
    fn labels_and_origin() {
        assert_eq!(Kind::of_label("trio-3"), Kind::Trio);
        assert_eq!(Kind::of_label("output-1"), Kind::Output);
        assert_eq!(Kind::of_label("inne-2"), Kind::Other);
        assert_eq!(Kind::of_label(CONNECT), Kind::Connect);
        assert!(same_origin("http://127.0.0.1:8088", "http://127.0.0.1:8088/trio?token=x"));
        assert!(!same_origin("http://127.0.0.1:8088", "http://127.0.0.1:80889/trio"));
        assert!(!same_origin("http://127.0.0.1:8088", "https://example.com/"));
    }
}
