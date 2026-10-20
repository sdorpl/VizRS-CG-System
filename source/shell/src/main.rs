//! vizrs-shell – natywne okno VizRS Artist (i Trio/Wyjście) z systemowym WebView.
//!
//!   vizrs-shell [artist|trio|output] [--data KAT] [--port N] [--token T] [--connect URL] [--vizrs ŚCIEŻKA]
//!
//! Strony UI są te same co w przeglądarce (serwuje je `vizrs`), więc jeden renderer (`web/renderer.js`)
//! obsługuje okno, Trio i wyjście OBS. Powłoka dodaje okno systemowe, menu, zapisywanie plików
//! (eksport sceny, paczka `.cgpack`) przez systemowe okno dialogowe i otwieranie kolejnych okien.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod server;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

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
}

impl Kind {
    fn path(self) -> &'static str {
        match self {
            Kind::Artist => "/artist",
            Kind::Trio => "/trio",
            Kind::Output => "/output?checker=1&status=1",
            Kind::Other => "/",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Kind::Artist => "VizRS Artist",
            Kind::Trio => "VizRS Trio",
            Kind::Output => "VizRS Wyjście",
            Kind::Other => "VizRS",
        }
    }
}

struct Args {
    kind: Kind,
    data: PathBuf,
    port: u16,
    token: Option<String>,
    connect: Option<String>,
    vizrs: Option<String>,
    probe: Option<(PathBuf, PathBuf)>,
}

fn help() -> &'static str {
    "vizrs-shell – natywne okno VizRS

Użycie: vizrs-shell [artist|trio|output] [opcje]   (domyślnie artist)

  --data KAT      katalog danych serwera (VIZRS_DATA, domyślnie vizrs-data)
  --port N        port serwera uruchamianego przez okno (VIZRS_PORT, domyślnie 8088)
  --token T       token serwera (VIZRS_TOKEN)
  --connect URL   okno dla serwera zdalnego, np. https://cg.example.pl (VIZRS_CONNECT)
  --vizrs PLIK    plik vizrs do uruchomienia serwera (VIZRS_BIN; domyślnie obok vizrs-shell albo w PATH)
  --probe JS --probe-out JSON   narzędzie testowe: po wczytaniu strony wykonaj skrypt, zapisz wynik, zakończ

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
        token: env("VIZRS_TOKEN"),
        connect: env("VIZRS_CONNECT"),
        vizrs: env("VIZRS_BIN"),
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
            "--data" => a.data = PathBuf::from(val()?),
            "--port" => a.port = val()?.parse().map_err(|_| "--port: zły numer portu".to_string())?,
            "--token" => a.token = Some(val()?),
            "--connect" => a.connect = Some(val()?),
            "--vizrs" => a.vizrs = Some(val()?),
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
}

struct Win {
    window: Window,
    webview: WebView,
    zoom: f64,
}

struct MenuIds {
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
    let file = Submenu::with_items("Plik", true, &[&artist, &trio, &output, &sep(), &close, &quit]).expect("menu Plik");
    let view = Submenu::with_items("Widok", true, &[&reload, &fullscreen, &sep(), &zoom_in, &zoom_out, &zoom_reset, &sep(), &devtools]).expect("menu Widok");
    let help = Submenu::with_items("Pomoc", true, &[&docs, &about]).expect("menu Pomoc");
    let menu = Menu::with_items(&[&file, &view, &help]).expect("menu");
    (menu, ids)
}

struct App {
    base: String,
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
        let window = WindowBuilder::new()
            .with_title(kind.title())
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
        let same_origin = move |u: &str| u.starts_with(&format!("{base}/")) || u == base || u.starts_with("about:") || u.starts_with("blob:") || u.starts_with("data:");
        let nav_ok = same_origin.clone();
        let p_new = proxy.clone();
        let p_load = proxy.clone();
        let p_ipc = proxy.clone();
        let builder = WebViewBuilder::new()
            .with_url(url)
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
            // most strona → powłoka: tylko wynik skryptu testowego; innych komunikatów nie obsługujemy
            .with_ipc_handler(move |req| {
                let body = req.body();
                if let Some(rest) = body.strip_prefix("probe:") {
                    p_ipc.send_event(UserEvent::Probe(rest.to_string())).ok();
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
        self.wins.insert(id, Win { window, webview, zoom: 1.0 });
        self.focused = Some(id);
        Ok(())
    }

    fn url(&self, kind: Kind) -> String {
        page_url(&self.base, kind.path(), self.token.as_deref())
    }

    fn on_menu(&mut self, id: &MenuId, target: &EventLoopWindowTarget<UserEvent>, proxy: &EventLoopProxy<UserEvent>, flow: &mut ControlFlow) {
        let ids = &self.ids;
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
            let msg = format!("vizrs-shell {}\nSerwer: {}\n{}", env!("CARGO_PKG_VERSION"), self.base, self.server_note);
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

    // serwer: zdalny, działający na tym katalogu danych albo uruchamiany teraz
    let (base, server_note) = if let Some(c) = &args.connect {
        (c.clone(), "serwer zdalny (--connect)".to_string())
    } else {
        match server::running_server(&args.data, args.token.as_deref()) {
            Ok(Some(addr)) => (format!("http://{addr}"), format!("dołączono do działającego serwera ({})", args.data.display())),
            Err(e) => {
                error_dialog(&e);
                std::process::exit(1);
            }
            Ok(None) => {
                let Some(vizrs) = server::find_vizrs(args.vizrs.as_deref()) else {
                    error_dialog("Nie znaleziono programu vizrs (obok vizrs-shell ani w PATH) – wskaż go: --vizrs ŚCIEŻKA albo VIZRS_BIN.");
                    std::process::exit(1);
                };
                match server::spawn(&vizrs, &args.data, args.port, args.token.as_deref()) {
                    Ok(s) => (format!("http://{}", s.addr), format!("serwer uruchomiony przez okno ({}), log: {}", args.data.display(), s.log.display())),
                    Err(e) => {
                        error_dialog(&format!("Serwer VizRS nie wystartował:\n{e}"));
                        std::process::exit(1);
                    }
                }
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
    let mut app = App { base, token: args.token.clone(), probe, menu, ids, wins: HashMap::new(), focused: None, server_note };
    let first = args.kind;

    event_loop.run(move |event, target, flow| {
        *flow = ControlFlow::Wait;
        match event {
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
