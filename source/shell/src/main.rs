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
mod conn_url;
mod connect;
mod desktop;
mod gpu;
mod paths;
mod scale;
mod screen;
mod server;
mod menu;
mod window;
mod tray;
mod ipc;

use menu::*;
use window::*;
use ipc::*;

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
const RELEASES: &str = "https://github.com/sdorpl/VizRS-CG-System/releases";
const CONNECT: &str = "connect";
/// Artist jest osobnym programem (od 2026-11-07).
const ARTIST_MOVED: &str = "Artist (projektowanie scen) to osobny program VizRS CG Artist (vizrs-cg-artist):
https://github.com/sdorpl/VizRS-CG-Artist/releases – pliki .cgpack i .vizrs.json otwiera on.
W przeglądarce zostaje zamrożony /artist serwera.";
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
    /// tryb `--gpu` dla gpu.rs (zmienne WebKitGTK przed startem GTK); None – z `VIZRS_GPU`
    gpu_mode: Option<String>,
}

fn help() -> &'static str {
    "vizrs-shell – natywne okno VizRS (Tauri 2)

Użycie: vizrs-shell [trio|output|connect] [opcje]   (domyślnie trio; connect – najpierw wybór serwera)

  --data KAT      katalog danych serwera (VIZRS_DATA; domyślnie ./vizrs-data, jeśli jest, inaczej ~/.local/share/vizrs)
  --port N        port serwera uruchamianego przez okno (VIZRS_PORT; domyślnie 8088, a gdy zajęty – następny wolny)
  --token T       token serwera (VIZRS_TOKEN)
  --connect URL   okno dla serwera zdalnego, np. https://cg.example.pl (VIZRS_CONNECT)
  --local         serwer lokalny, nawet gdy zapisano łączenie z serwerem zdalnym przy uruchomieniu
  --gpu TRYB      akceleracja GPU stron: auto, on, off, x11, wayland, system (VIZRS_GPU; Linux)
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
        gpu_mode: None,
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
                let v = val()?;
                a.gpu = match v.as_str() {
                    "on" => Some(true),
                    "off" => Some(false),
                    "auto" | "x11" | "system" => None,
                    _ => return Err("--gpu: podaj auto, on, off, x11 albo system".into()),
                };
                a.gpu_mode = Some(v);
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
    // akceleracja WebKitGTK: zmienne środowiska przed startem GTK i przed powstaniem wątków (gpu.rs)
    let gpu_note = gpu::prepare(args.gpu_mode.as_deref());
    if !gpu_note.is_empty() {
        eprintln!("GPU: {gpu_note}");
    }
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
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, sc, ev| tray::on_shortcut(app, sc, ev.state())).build())
        .manage(tray::Shortcuts::default())
        .manage(Mutex::new(app_state))
        .manage(Downloads::default())
        .invoke_handler(tauri::generate_handler![ipc::ipc])
        .setup(move |app| {
            let h = app.handle().clone();
            // interfejs jest ciemny, więc i to, co rysuje system (menu, okna dialogowe, „Zapisz jako”)
            h.set_theme(Some(Theme::Dark));
            let monitors = h.available_monitors().unwrap_or_default();
            let menu = build_menu(&h, &monitors)?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, ev| on_menu(app, ev.id().as_ref()));
            // zasobnik i skróty globalne (akcje anteny bez fokusu okna)
            tray::setup_tray(&h);
            let keys = tray::setup_shortcuts(&h);
            *h.state::<tray::Shortcuts>().inner().lock().unwrap_or_else(|p| p.into_inner()) = keys;
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
