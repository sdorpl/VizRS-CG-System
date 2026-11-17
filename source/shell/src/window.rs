//! Okna: Trio i wyjście (skrypt mostu, uprawnienie dla adresu serwera, wygląd systemu, ustawienia WebKit, odzyskiwanie po awarii strony, ekrany), okno połączenia, przełączanie serwera, zamykanie, położenie okien, pobrania.

use super::*;

/// Tło WebView przed narysowaniem strony – kolor `--bg` z `vizrs-ui.css` (Windows, macOS).
#[cfg(not(target_os = "linux"))]
pub(crate) const BACKGROUND: Color = Color(0x0e, 0x10, 0x15, 0xff);

/// Most strony: `window.__vizrsPost(tekst)` → komenda `ipc` (`window.ipc` należy w Tauri do Tauri).
/// `menu: true` – strona Trio prosi o systemowe menu pod prawym przyciskiem.
pub(crate) const IPC_SHIM: &str = r#"(() => {
  window.__vizrsPost = (m) => { window.__TAURI_INTERNALS__.invoke('ipc', { body: String(m) }).catch((e) => console.error('ipc', e)); };
  window.__vizrsShell = { version: '__VERSION__', menu: true, os: '__OS__' };
})();"#;

pub(crate) fn script() -> String {
    IPC_SHIM.replace("__VERSION__", env!("CARGO_PKG_VERSION")).replace("__OS__", std::env::consts::OS)
}

/// Strony bieżącego serwera mogą wołać komendę `ipc` (uprawnienie dodawane w działaniu – serwer wybiera
/// operator, więc nie da się go wpisać do `capabilities/`). Komenda i tak sprawdza okno i adres strony.
pub(crate) fn allow_server(app: &AppHandle, base: &str) {
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
pub(crate) fn native_look<'a>(b: WebviewWindowBuilder<'a, tauri::Wry, AppHandle>) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
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
pub(crate) fn reveal(app: &AppHandle, label: &str) {
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
pub(crate) fn webkit_settings(w: &WebviewWindow, gpu: Option<bool>) {
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

/// Linux: proces strony WebKitGTK padł (sterownik GPU, brak pamięci) – WebKitGTK zostawia wtedy puste okno.
/// Strona wczytuje się od nowa z adresu bieżącego serwera (wyjście na monitorze – z adresu, z którym je otwarto),
/// najwyżej 3 razy na minutę – przy awarii za każdym wczytaniem okno zostaje puste zamiast mielić w kółko.
/// Ten sam mechanizm w oknie VizRS CG Artist i oknie panelu VizRS CG Playout.
pub(crate) fn reload_after_crash(app: &AppHandle, w: &WebviewWindow, kind: Kind, fixed: Option<String>) {
    #[cfg(target_os = "linux")]
    {
        let app = app.clone();
        let _ = w.with_webview(move |pv| {
            use webkit2gtk::WebViewExt;
            let recent = std::cell::RefCell::new(Vec::<Instant>::new());
            pv.inner().connect_web_process_terminated(move |v, reason| {
                let mut r = recent.borrow_mut();
                r.retain(|t| t.elapsed() < Duration::from_secs(60));
                let again = r.len() < 3;
                let what = if again { " – wczytuję stronę ponownie" } else { " – okno zostaje puste" };
                eprintln!("proces strony zakończył się ({reason:?}){what}");
                if again {
                    r.push(Instant::now());
                    let url = fixed.clone().unwrap_or_else(|| state(&app).url(kind));
                    v.load_uri(&url);
                }
            });
        });
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (app, w, kind, fixed);
}

/// Okno strony serwera; `monitor` – pełnoekranowe wyjście na wskazanym monitorze (bez menu i zapamiętywania).
pub(crate) fn open_window(app: &AppHandle, kind: Kind, url: String, monitor: Option<Monitor>) {
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
    let (w, h) = screen::fit_to_screen(app, w, h); // skalowanie ułamkowe: okno nie większe niż ekran
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
            reload_after_crash(app, &w, kind, monitor.is_some().then(|| url.clone()));
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

pub(crate) fn page_loaded(app: &AppHandle, label: &str) {
    if label != CONNECT {
        reveal(app, label);
    }
    let probe = state(app).probe.as_ref().map(|(c, _)| c.clone());
    if let Some(code) = probe {
        eval_in(app, label, &code);
    }
}

/// Pełnoekranowe wyjście na wskazanym monitorze (czyste `/output`; Fill/Key przez `?mode=`).
pub(crate) fn open_screen(app: &AppHandle, n: usize, query: &str) {
    let (m, url) = {
        let a = state(app);
        let Some(m) = a.monitors.get(n).cloned() else { return };
        (m, page_url(&a.base, &format!("/output{query}"), a.token.as_deref()))
    };
    open_window(app, Kind::Output, url, Some(m));
}

pub(crate) fn open_connect(app: &AppHandle) {
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

pub(crate) fn close_connect(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(CONNECT) {
        let _ = w.destroy();
    }
}

pub(crate) fn conn_error(app: &AppHandle, msg: &str) {
    eval_in(app, CONNECT, &format!("window.__vizrsConnError && window.__vizrsConnError({})", Value::from(msg)));
}

/// Przełącza wszystkie okna na inny serwer (okno połączenia się zamyka); bez okien – otwiera Trio.
pub(crate) fn switch(app: &AppHandle, base: String, token: Option<String>, note: String) {
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

/// Zamknięcie okna Trio: ze zmianami pyta przez stronę (ona zna stan i umie zapisać); strona bez odpowiedzi
/// nie zatrzyma okna na zawsze. `true` = wstrzymaj zamknięcie.
pub(crate) fn close_request(app: &AppHandle, label: &str) -> bool {
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
pub(crate) fn remember(app: &AppHandle, label: &str, w: &tauri::Window) {
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
pub(crate) fn quit_step(app: &AppHandle) {
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

/// Pobranie strony (paczka, kopia show, CSV): najpierw do pliku tymczasowego, po zakończeniu systemowe „Zapisz jako”
/// (blokujące okno w wywołaniu zwrotnym pobrania mogłoby zakleszczyć wątek okna).
pub(crate) fn on_download(app: &AppHandle, ev: DownloadEvent<'_>) -> bool {
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
