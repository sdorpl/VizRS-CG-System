//! Menu okna Trio i wyjścia: ekrany (monitory), budowa menu, akcje.

use super::*;

pub(crate) fn monitor_label(i: usize, m: &Monitor) -> String {
    let s = m.size();
    format!("Monitor {}: {} ({}×{})", i + 1, m.name().cloned().unwrap_or_default(), s.width, s.height)
}

pub(crate) fn build_menu(app: &AppHandle, monitors: &[Monitor]) -> tauri::Result<Menu<tauri::Wry>> {
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

pub(crate) fn on_menu(app: &AppHandle, id: &str) {
    if let Some(a) = id.strip_prefix("tray:") {
        return tray::action(app, a);
    }
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
