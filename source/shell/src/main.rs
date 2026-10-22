//! vizrs-shell – natywne okno VizRS Artist (i Trio/Wyjście) z systemowym WebView.
//!
//!   vizrs-shell [artist|trio|output|connect] [--data KAT] [--port N] [--token T] [--connect URL] [--vizrs ŚCIEŻKA]
//!
//! Strony UI są te same co w przeglądarce (serwuje je `vizrs`), więc jeden renderer (`web/renderer.js`)
//! obsługuje okno, Trio i wyjście OBS. Powłoka dodaje okno systemowe, menu, zapisywanie plików
//! (eksport sceny, paczka `.cgpack`) przez systemowe okno dialogowe i otwieranie kolejnych okien.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod connect;
mod server;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::sync::{Arc, Mutex};

use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Fullscreen, Icon, Window, WindowBuilder, WindowId};
use wry::{NewWindowResponse, PageLoadEvent, WebView, WebViewBuilder};

const ICON_PNG: &[u8] = include_bytes!("../../../deploy/vizrs-256.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Artist,
    Trio,
    Output,
    Other,
    /// okno „Połącz z serwerem” (strona wbudowana w powłokę)
    Connect,
}

impl Kind {
    fn path(self) -> &'static str {
        match self {
            Kind::Artist => "/artist",
            Kind::Trio => "/trio",
            Kind::Output => "/output?checker=1&status=1",
            Kind::Other | Kind::Connect => "/",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Kind::Artist => "VizRS Artist",
            Kind::Trio => "VizRS Trio",
            Kind::Output => "VizRS Wyjście",
            Kind::Other => "VizRS",
            Kind::Connect => "VizRS – połącz z serwerem",
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
}

fn help() -> &'static str {
    "vizrs-shell – natywne okno VizRS

Użycie: vizrs-shell [artist|trio|output|connect] [opcje]   (domyślnie artist; connect – najpierw wybór serwera)

  --data KAT      katalog danych serwera (VIZRS_DATA, domyślnie vizrs-data)
  --port N        port serwera uruchamianego przez okno (VIZRS_PORT; domyślnie 8088, a gdy zajęty – następny wolny)
  --token T       token serwera (VIZRS_TOKEN)
  --connect URL   okno dla serwera zdalnego, np. https://cg.example.pl (VIZRS_CONNECT)
  --local         serwer lokalny, nawet gdy zapisano łączenie z serwerem zdalnym przy uruchomieniu
  --vizrs PLIK    plik vizrs do uruchomienia serwera (VIZRS_BIN; domyślnie obok vizrs-shell albo w PATH)
  --probe JS --probe-out JSON   narzędzie testowe: po wczytaniu strony wykonaj skrypt, zapisz wynik, zakończ

Plik → Połącz z serwerem… (Ctrl+Shift+K): adres i token serwera zdalnego (VPS), lista zapisanych połączeń.

Jeśli serwer na tym katalogu danych już działa, okno dołącza do niego. Inaczej powłoka uruchamia
`vizrs serve` jako osobny proces: zamknięcie okna nie zdejmuje grafiki z anteny, a serwer kończy pracę
sam, gdy przez 10 s nie ma żadnego okna ani wyjścia (OBS)."
}

fn parse_args() -> Result<Args, String> {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
    let mut a = Args {
        kind: Kind::Artist,
        data: PathBuf::from(env("VIZRS_DATA").unwrap_or_else(|| "vizrs-data".into())),
        port: env("VIZRS_PORT").map(|p| p.parse().map_err(|_| "VIZRS_PORT: zły numer portu".to_string())).transpose()?.unwrap_or(8088),
        port_set: env("VIZRS_PORT").is_some(),
        token: env("VIZRS_TOKEN"),
        connect: env("VIZRS_CONNECT"),
        vizrs: env("VIZRS_BIN"),
        local: false,
        probe: None,
    };
    let mut probe_js = None;
    let mut probe_out = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].clone();
        let mut val = || -> Result<String, String> {
            i += 1;
            args.get(i).cloned().ok_or(format!("{flag}: brak wartości"))
        };
        match flag.as_str() {
            "artist" => a.kind = Kind::Artist,
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

/// Komunikat błędu w okienku systemowym (i na stderr).
fn error_dialog(msg: &str) {
    eprintln!("{msg}");
    rfd::MessageDialog::new().set_title("VizRS").set_description(msg).set_level(rfd::MessageLevel::Error).show();
}

/// Adres strony okna: baza serwera + ścieżka + token.
fn page_url(base: &str, path: &str, token: Option<&str>) -> String {
    match token {
        Some(t) => format!("{base}{path}{}token={t}", if path.contains('?') { '&' } else { '?' }),
        None => format!("{base}{path}"),
    }
}

/// Otwiera adres w przeglądarce systemowej (odnośniki spoza serwera VizRS).
fn open_external(url: &str) {
    let r = if cfg!(windows) {
        std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if let Err(e) = r {
        eprintln!("nie mogę otworzyć {url}: {e}");
    }
}

fn icon() -> Option<Icon> {
    let mut r = png::Decoder::new(std::io::Cursor::new(ICON_PNG)).read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    Icon::from_rgba(buf, info.width, info.height).ok()
}

enum UserEvent {
    Menu(MenuId),
    /// Nowe okno dla adresu z serwera (window.open, target=_blank).
    OpenUrl(String),
    PageLoaded(WindowId),
    /// Wynik skryptu `--probe` (JSON).
    Probe(String),
    /// Komunikat okna połączenia (JSON).
    Conn(String),
    /// Wynik sprawdzenia serwera przed przełączeniem (błąd albo None).
    Checked(connect::Request, Option<String>),
}

struct Win {
    window: Window,
    webview: WebView,
    zoom: f64,
    kind: Kind,
}

struct MenuIds {
    connect: MenuId,
    artist: MenuId,
    trio: MenuId,
    output: MenuId,
    close: MenuId,
    quit: MenuId,
    reload: MenuId,
    fullscreen: MenuId,
    zoom_in: MenuId,
    zoom_out: MenuId,
    zoom_reset: MenuId,
    devtools: MenuId,
    docs: MenuId,
    about: MenuId,
}

fn item(text: &str, acc: Option<Accelerator>) -> MenuItem {
    MenuItem::new(text, true, acc)
}

fn build_menu() -> (Menu, MenuIds) {
    let ctrl = if cfg!(target_os = "macos") { Modifiers::META } else { Modifiers::CONTROL };
    let connect = item("Połącz z serwerem…", Some(Accelerator::new(ctrl | Modifiers::SHIFT, Code::KeyK)));
    let artist = item("Nowe okno Artist", None);
    let trio = item("Nowe okno Trio", None);
    let output = item("Okno wyjścia (podgląd)", None);
    let close = item("Zamknij okno", Some(Accelerator::new(ctrl, Code::KeyW)));
    let quit = item("Zakończ", Some(Accelerator::new(ctrl, Code::KeyQ)));
    let reload = item("Odśwież", Some(Accelerator::new(Modifiers::empty(), Code::F5)));
    let fullscreen = item("Pełny ekran", Some(Accelerator::new(Modifiers::empty(), Code::F11)));
    let zoom_in = item("Powiększ", Some(Accelerator::new(ctrl, Code::Equal)));
    let zoom_out = item("Pomniejsz", Some(Accelerator::new(ctrl, Code::Minus)));
    let zoom_reset = item("Rozmiar 100%", Some(Accelerator::new(ctrl, Code::Digit0)));
    let devtools = item("Narzędzia deweloperskie", Some(Accelerator::new(ctrl | Modifiers::SHIFT, Code::KeyI)));
    let docs = item("Strona startowa i API", None);
    let about = item("O programie", None);
    let ids = MenuIds {
        connect: connect.id().clone(),
        artist: artist.id().clone(),
        trio: trio.id().clone(),
        output: output.id().clone(),
        close: close.id().clone(),
        quit: quit.id().clone(),
        reload: reload.id().clone(),
        fullscreen: fullscreen.id().clone(),
        zoom_in: zoom_in.id().clone(),
        zoom_out: zoom_out.id().clone(),
        zoom_reset: zoom_reset.id().clone(),
        devtools: devtools.id().clone(),
        docs: docs.id().clone(),
        about: about.id().clone(),
    };
    let sep = PredefinedMenuItem::separator;
    let file = Submenu::with_items("Plik", true, &[&connect, &sep(), &artist, &trio, &output, &sep(), &close, &quit]).expect("menu Plik");
    let view = Submenu::with_items("Widok", true, &[&reload, &fullscreen, &sep(), &zoom_in, &zoom_out, &zoom_reset, &sep(), &devtools]).expect("menu Widok");
    let help = Submenu::with_items("Pomoc", true, &[&docs, &about]).expect("menu Pomoc");
    let menu = Menu::with_items(&[&file, &view, &help]).expect("menu");
    (menu, ids)
}

struct App {
    /// adres serwera okien; wspólny z obsługą nawigacji (po przełączeniu serwera)
    base: Arc<Mutex<String>>,
    /// adres serwera lokalnego (uruchomionego albo dołączonego), None przy `--connect`
    local: Option<String>,
    /// token serwera lokalnego (`--token`)
    local_token: Option<String>,
    /// opcje do uruchomienia serwera lokalnego na żądanie (po starcie z serwerem zdalnym)
    local_start: (PathBuf, u16, Option<String>),
    token: Option<String>,
    probe: Option<(String, PathBuf)>,
    menu: Menu,
    ids: MenuIds,
    wins: HashMap<WindowId, Win>,
    focused: Option<WindowId>,
    server_note: String,
}

impl App {
    fn open(&mut self, target: &EventLoopWindowTarget<UserEvent>, proxy: &EventLoopProxy<UserEvent>, kind: Kind, url: String) -> Result<(), String> {
        let size = if kind == Kind::Output { LogicalSize::new(960.0, 540.0) } else { LogicalSize::new(1600.0, 900.0) };
        let size = if kind == Kind::Connect { LogicalSize::new(560.0, 600.0) } else { size };
        let window = WindowBuilder::new()
            .with_title(self.title(kind))
            .with_inner_size(size)
            .with_min_inner_size(LogicalSize::new(480.0, 270.0))
            // Artist i Trio to stanowiska pracy – od razu na cały ekran (wyjście: okno 960×540)
            .with_maximized(matches!(kind, Kind::Artist | Kind::Trio))
            .with_window_icon(icon())
            .build(target)
            .map_err(|e| format!("okno: {e}"))?;
        let id = window.id();

        #[cfg(target_os = "windows")]
        {
            use tao::platform::windows::WindowExtWindows;
            // SAFETY: uchwyt okna żyje razem z `window`, które trzymamy w `self.wins`.
            unsafe { self.menu.init_for_hwnd(window.hwnd()) }.ok();
        }
        #[cfg(target_os = "linux")]
        {
            use tao::platform::unix::WindowExtUnix;
            self.menu.init_for_gtk_window(window.gtk_window(), window.default_vbox()).ok();
        }
        #[cfg(target_os = "macos")]
        self.menu.init_for_nsapp();

        let base = self.base.clone();
        let same_origin = move |u: &str| {
            let b = base.lock().map(|b| b.clone()).unwrap_or_default();
            u.starts_with(&format!("{b}/")) || u == b || u.starts_with("about:") || u.starts_with("blob:") || u.starts_with("data:")
        };
        let nav_ok = same_origin.clone();
        let p_new = proxy.clone();
        let p_load = proxy.clone();
        let p_ipc = proxy.clone();
        let builder = if kind == Kind::Connect {
            let data = connect::page_data(&connect::Saved::load(), &self.base(), self.token.as_deref(), self.local.as_deref());
            WebViewBuilder::new().with_html(connect::PAGE).with_initialization_script(data)
        } else {
            WebViewBuilder::new().with_url(url)
        };
        let builder = builder
            .with_devtools(true)
            // znacznik okna natywnego: strony chowają zakładki „Artist / Trio / Wyjście” (są w menu) i menu przeglądarki
            .with_initialization_script(concat!("window.__vizrsShell = { version: '", env!("CARGO_PKG_VERSION"), "' };"))
            .with_clipboard(true)
            // odnośniki poza serwer VizRS otwiera przeglądarka systemowa
            .with_navigation_handler(move |u| {
                if nav_ok(&u) {
                    true
                } else {
                    open_external(&u);
                    false
                }
            })
            // window.open / target=_blank: strony serwera w nowym oknie powłoki, reszta w przeglądarce
            .with_new_window_req_handler(move |u, _| {
                if same_origin(&u) {
                    p_new.send_event(UserEvent::OpenUrl(u)).ok();
                } else {
                    open_external(&u);
                }
                NewWindowResponse::Deny
            })
            // eksport sceny, paczka .cgpack: systemowe okno „Zapisz jako”
            .with_download_started_handler(|_url, path| {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "vizrs-eksport".into());
                let mut dlg = rfd::FileDialog::new().set_title("Zapisz plik").set_file_name(&name);
                if let Some(dir) = path.parent().filter(|d| d.is_dir()) {
                    dlg = dlg.set_directory(dir);
                }
                match dlg.save_file() {
                    Some(p) => {
                        *path = p;
                        true
                    }
                    None => false,
                }
            })
            .with_download_completed_handler(|_url, path, ok| {
                if !ok {
                    error_dialog(&format!("Nie udało się zapisać pliku{}", path.map(|p| format!(" {}", p.display())).unwrap_or_default()));
                }
            })
            .with_on_page_load_handler(move |ev, _| {
                if matches!(ev, PageLoadEvent::Finished) {
                    p_load.send_event(UserEvent::PageLoaded(id)).ok();
                }
            })
            // most strona → powłoka: wynik skryptu testowego; wybór serwera tylko z wbudowanego okna połączenia
            .with_ipc_handler(move |req| {
                let body = req.body();
                if let Some(rest) = body.strip_prefix("probe:") {
                    p_ipc.send_event(UserEvent::Probe(rest.to_string())).ok();
                } else if kind == Kind::Connect {
                    p_ipc.send_event(UserEvent::Conn(body.to_string())).ok();
                }
            });

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let webview = builder.build(&window).map_err(|e| format!("WebView: {e}"))?;
        #[cfg(target_os = "linux")]
        let webview = {
            use tao::platform::unix::WindowExtUnix;
            use wry::WebViewBuilderExtUnix;
            let vbox = window.default_vbox().ok_or("brak kontenera GTK okna")?;
            let wv = builder.build_gtk(vbox).map_err(|e| format!("WebView: {e}"))?;
            // pisownia polska w polach tekstowych (słownik hunspell pl_PL z systemu, przez enchant)
            use webkit2gtk::{WebContextExt, WebViewExt};
            use wry::WebViewExtUnix;
            if let Some(ctx) = wv.webview().context() {
                ctx.set_spell_checking_enabled(true);
                ctx.set_spell_checking_languages(&["pl_PL"]);
            }
            wv
        };
        self.wins.insert(id, Win { window, webview, zoom: 1.0, kind });
        self.focused = Some(id);
        Ok(())
    }

    fn base(&self) -> String {
        self.base.lock().map(|b| b.clone()).unwrap_or_default()
    }

    fn url(&self, kind: Kind) -> String {
        page_url(&self.base(), kind.path(), self.token.as_deref())
    }

    /// Tytuł okna; przy serwerze zdalnym z nazwą hosta.
    fn title(&self, kind: Kind) -> String {
        let base = self.base();
        match kind {
            Kind::Connect => kind.title().to_string(),
            _ if self.local.as_deref() == Some(base.as_str()) => kind.title().to_string(),
            _ => format!("{} – {}", kind.title(), base.split("://").nth(1).unwrap_or(&base)),
        }
    }

    /// Przełącza wszystkie okna na inny serwer (okno połączenia się zamyka).
    fn switch(&mut self, base: String, token: Option<String>, note: String) {
        if let Ok(mut b) = self.base.lock() {
            *b = base;
        }
        self.token = token;
        self.server_note = note;
        self.wins.retain(|_, w| w.kind != Kind::Connect);
        let targets: Vec<(WindowId, Kind)> = self.wins.iter().map(|(id, w)| (*id, w.kind)).collect();
        for (id, kind) in targets {
            let (url, title) = (self.url(kind), self.title(kind));
            if let Some(w) = self.wins.get(&id) {
                w.window.set_title(&title);
                w.webview.load_url(&url).ok();
            }
        }
    }

    /// Komunikat okna połączenia.
    fn on_conn(&mut self, body: &str, target: &EventLoopWindowTarget<UserEvent>, proxy: &EventLoopProxy<UserEvent>, flow: &mut ControlFlow) {
        let msg = match connect::parse_msg(body) {
            Ok(m) => m,
            Err(e) => return self.conn_error(&e),
        };
        match msg {
            connect::Msg::Cancel => {
                self.wins.retain(|_, w| w.kind != Kind::Connect);
                if self.wins.is_empty() {
                    *flow = ControlFlow::Exit;
                }
            }
            connect::Msg::Forget(url) => {
                let mut saved = connect::Saved::load();
                saved.forget(&url);
                if let Err(e) = saved.save() {
                    self.conn_error(&e);
                }
            }
            connect::Msg::Local => {
                if self.local.is_none() {
                    let (data, port, vizrs) = self.local_start.clone();
                    match local_server(&data, port, self.local_token.as_deref(), vizrs.as_deref()) {
                        Ok((base, _)) => self.local = Some(base),
                        Err(e) => return self.conn_error(&e),
                    }
                }
                if let Some(l) = self.local.clone() {
                    self.switch(l, self.local_token.clone(), "serwer lokalny".into());
                    self.ensure_window(target, proxy);
                }
            }
            connect::Msg::Connect(mut req) => {
                // puste pole tokenu = token zapamiętany dla tego serwera
                if req.token.is_none() {
                    req.token = connect::Saved::load().token_for(&req.url);
                }
                let p = proxy.clone();
                std::thread::spawn(move || {
                    let err = check_server(&req.url, req.token.as_deref()).err();
                    p.send_event(UserEvent::Checked(req, err)).ok();
                });
            }
        }
    }

    fn on_checked(&mut self, req: connect::Request, err: Option<String>, target: &EventLoopWindowTarget<UserEvent>, proxy: &EventLoopProxy<UserEvent>) {
        if let Some(e) = err {
            return self.conn_error(&e);
        }
        let connect::Request { url, token, keep_token, auto } = req;
        let mut saved = connect::Saved::load();
        saved.remember(&url, token.as_deref(), keep_token, auto);
        if let Err(e) = saved.save() {
            eprintln!("zapis listy serwerów: {e}");
        }
        self.switch(url, token, "serwer zdalny (Plik → Połącz z serwerem)".into());
        self.ensure_window(target, proxy);
    }

    /// Po przełączeniu musi zostać choć jedno okno – domyślnie Trio.
    fn ensure_window(&mut self, target: &EventLoopWindowTarget<UserEvent>, proxy: &EventLoopProxy<UserEvent>) {
        if self.wins.is_empty() {
            let url = self.url(Kind::Trio);
            if let Err(e) = self.open(target, proxy, Kind::Trio, url) {
                error_dialog(&e);
            }
        }
    }

    fn conn_error(&self, msg: &str) {
        let js = format!("window.__vizrsConnError && window.__vizrsConnError({})", serde_json::Value::from(msg));
        for w in self.wins.values().filter(|w| w.kind == Kind::Connect) {
            w.webview.evaluate_script(&js).ok();
        }
    }

    fn open_connect(&mut self, target: &EventLoopWindowTarget<UserEvent>, proxy: &EventLoopProxy<UserEvent>) {
        if let Some(w) = self.wins.values().find(|w| w.kind == Kind::Connect) {
            w.window.set_focus();
            return;
        }
        if let Err(e) = self.open(target, proxy, Kind::Connect, String::new()) {
            error_dialog(&e);
        }
    }

    fn on_menu(&mut self, id: &MenuId, target: &EventLoopWindowTarget<UserEvent>, proxy: &EventLoopProxy<UserEvent>, flow: &mut ControlFlow) {
        let ids = &self.ids;
        if *id == ids.connect {
            return self.open_connect(target, proxy);
        }
        let kind = if *id == ids.artist {
            Some(Kind::Artist)
        } else if *id == ids.trio {
            Some(Kind::Trio)
        } else if *id == ids.output {
            Some(Kind::Output)
        } else if *id == ids.docs {
            Some(Kind::Other)
        } else {
            None
        };
        if let Some(k) = kind {
            let url = self.url(k);
            if let Err(e) = self.open(target, proxy, k, url) {
                error_dialog(&e);
            }
            return;
        }
        if *id == ids.quit {
            *flow = ControlFlow::Exit;
            return;
        }
        if *id == ids.about {
            let msg = format!("vizrs-shell {}\nSerwer: {}\n{}", env!("CARGO_PKG_VERSION"), self.base(), self.server_note);
            rfd::MessageDialog::new().set_title("O programie").set_description(msg).show();
            return;
        }
        let Some(fid) = self.focused else { return };
        if *id == ids.close {
            self.wins.remove(&fid);
            if self.wins.is_empty() {
                *flow = ControlFlow::Exit;
            }
            return;
        }
        let Some(w) = self.wins.get_mut(&fid) else { return };
        if *id == ids.reload {
            w.webview.reload().ok();
        } else if *id == ids.fullscreen {
            let fs = w.window.fullscreen().is_some();
            w.window.set_fullscreen(if fs { None } else { Some(Fullscreen::Borderless(None)) });
        } else if *id == ids.zoom_in || *id == ids.zoom_out || *id == ids.zoom_reset {
            w.zoom = if *id == ids.zoom_reset { 1.0 } else if *id == ids.zoom_in { (w.zoom * 1.1).min(3.0) } else { (w.zoom / 1.1).max(0.3) };
            w.webview.zoom(w.zoom).ok();
        } else if *id == ids.devtools {
            w.webview.open_devtools();
        }
    }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            error_dialog(&e);
            std::process::exit(2);
        }
    };

    // serwer: zdalny (--connect albo zapisany „przy uruchomieniu”), działający na tym katalogu danych albo uruchamiany teraz
    // domyślny port zajęty (np. serwer na innym katalogu danych) – następny wolny; jawny --port bez zmian
    let port = if args.port_set { args.port } else { server::free_port(args.port) };
    let auto = if args.connect.is_none() && !args.local { connect::Saved::load().auto } else { None };
    let mut token = args.token.clone();
    let (base, local, server_note) = if let Some(c) = &args.connect {
        (c.clone(), None, "serwer zdalny (--connect)".to_string())
    } else if let Some(a) = auto {
        token = connect::Saved::load().token_for(&a);
        (a, None, "serwer zdalny (łączenie przy uruchomieniu; serwer lokalny: --local)".to_string())
    } else {
        match local_server(&args.data, port, args.token.as_deref(), args.vizrs.as_deref()) {
            Ok((b, note)) => (b.clone(), Some(b), note),
            Err(e) => {
                error_dialog(&e);
                std::process::exit(1);
            }
        }
    };

    let probe = match &args.probe {
        Some((js, out)) => match std::fs::read_to_string(js) {
            Ok(code) => Some((code, out.clone())),
            Err(e) => {
                error_dialog(&format!("{}: {e}", js.display()));
                std::process::exit(2);
            }
        },
        None => None,
    };

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    {
        // obsługa menu może przyjść z innego wątku – przekaż do pętli zdarzeń
        let p = Mutex::new(proxy.clone());
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if let Ok(p) = p.lock() {
                p.send_event(UserEvent::Menu(e.id)).ok();
            }
        }));
    }
    let (menu, ids) = build_menu();
    let local_start = (args.data.clone(), port, args.vizrs.clone());
    let mut app = App { base: Arc::new(Mutex::new(base)), local, local_token: args.token.clone(), local_start, token, probe, menu, ids, wins: HashMap::new(), focused: None, server_note };
    let first = args.kind;

    event_loop.run(move |event, target, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) if first == Kind::Connect => app.open_connect(target, &proxy),
            Event::NewEvents(StartCause::Init) => {
                let url = app.url(first);
                if let Err(e) = app.open(target, &proxy, first, url) {
                    error_dialog(&e);
                    *flow = ControlFlow::Exit;
                }
            }
            Event::WindowEvent { window_id, event: WindowEvent::Focused(true), .. } => app.focused = Some(window_id),
            Event::WindowEvent { window_id, event: WindowEvent::CloseRequested, .. } => {
                app.wins.remove(&window_id);
                if app.wins.is_empty() {
                    *flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(UserEvent::Menu(id)) => app.on_menu(&id, target, &proxy, flow),
            Event::UserEvent(UserEvent::Conn(body)) => app.on_conn(&body, target, &proxy, flow),
            Event::UserEvent(UserEvent::Checked(req, err)) => app.on_checked(req, err, target, &proxy),
            Event::UserEvent(UserEvent::OpenUrl(u)) => {
                let kind = if u.contains("/output") { Kind::Output } else { Kind::Other };
                if let Err(e) = app.open(target, &proxy, kind, u) {
                    error_dialog(&e);
                }
            }
            Event::UserEvent(UserEvent::PageLoaded(id)) => {
                if let (Some((code, _)), Some(w)) = (&app.probe, app.wins.get(&id)) {
                    w.webview.evaluate_script(code).ok();
                }
            }
            Event::UserEvent(UserEvent::Probe(json)) => {
                if let Some((_, out)) = &app.probe {
                    if let Err(e) = std::fs::write(out, json) {
                        eprintln!("{}: {e}", out.display());
                    }
                }
                *flow = ControlFlow::Exit;
            }
            _ => {}
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
/// (powłoka nie ma TLS) – wskaźnik połączenia w Trio/Artist.
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

    #[test]
    fn urls_with_token() {
        assert_eq!(page_url("http://h:1", "/artist", None), "http://h:1/artist");
        assert_eq!(page_url("http://h:1", "/artist", Some("t")), "http://h:1/artist?token=t");
        assert_eq!(page_url("http://h:1", "/output?checker=1", Some("t")), "http://h:1/output?checker=1&token=t");
    }
}
