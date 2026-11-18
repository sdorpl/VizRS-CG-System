//! Komunikaty strony i okna połączenia (komenda `ipc`, biała lista w `bridge`), menu kontekstowe, sprawdzenie serwera, druga instancja.

use super::*;

/// Prośba kolejnego uruchomienia: istniejące okno na wierzch albo nowe.
pub(crate) fn on_remote(app: &AppHandle, argv: &[String]) {
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

#[tauri::command]
pub(crate) fn ipc(app: AppHandle, webview: tauri::Webview, body: String) {
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
        bridge::Msg::ConnectWindow { url, token } => open_connect_with(&app, Some((url, token))),
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
pub(crate) fn context_menu(app: &AppHandle, label: &str, entries: Vec<bridge::MenuEntry>) {
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

pub(crate) fn on_conn(app: &AppHandle, body: &str) {
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

pub(crate) fn on_checked(app: &AppHandle, req: connect::Request, err: Option<String>) {
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

/// Przed przełączeniem okien: `/api/state` serwera `http://` (token, dostępność). `https://` sprawdza dopiero strona
/// (okno nie ma TLS) – wskaźnik połączenia w Trio.
pub(crate) fn check_server(url: &str, token: Option<&str>) -> Result<(), String> {
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
