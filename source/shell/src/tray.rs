//! Zasobnik systemowy i skróty globalne okna Trio.
//!
//! Ikona w zasobniku: lewy klik – okno Trio na wierzch; menu – akcje anteny (TAKE, CONTINUE, OUT, następna
//! pozycja rundownu, ZDEJMIJ wszystko), „Połącz z serwerem…”, „Zakończ”. Skróty globalne działają, gdy okno
//! nie ma fokusu (np. operator pracuje w OBS): te same akcje co F1–F8 w Trio, w oknie Trio z fokusem albo
//! pierwszym otwartym. Strona wykonuje je przez `window.__vizrsAction` (`web/trio/system.js`) – te same funkcje
//! co klawiatura, więc z tymi samymi pytaniami i blokadami.
//!
//! `VIZRS_GLOBAL_KEYS`: `off` – bez skrótów; `take=Ctrl+Alt+F1,clear=` – własne (puste = bez tego skrótu),
//! reszta domyślna. Na Waylandzie system nie pozwala programom na skróty globalne – zostaje zasobnik.

use super::*;

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// (akcja, opis w menu, domyślny skrót)
pub(crate) const ACTIONS: [(&str, &str, &str); 6] = [
    ("take", "TAKE wybranej strony", "CommandOrControl+Alt+F1"),
    ("cont", "CONTINUE", "CommandOrControl+Alt+F2"),
    ("out", "OUT wybranej strony", "CommandOrControl+Alt+F3"),
    ("next", "Następna pozycja rundownu", "CommandOrControl+Alt+F8"),
    ("clear", "ZDEJMIJ wszystko z anteny", "CommandOrControl+Alt+Shift+F3"),
    ("show", "Pokaż Trio", "CommandOrControl+Alt+T"),
];

/// Skróty z `VIZRS_GLOBAL_KEYS` (brak – domyślne): lista (akcja, skrót); `off` – pusta.
pub(crate) fn keys_config(env: Option<&str>) -> Result<Vec<(String, String)>, String> {
    let env = env.map(str::trim).unwrap_or("");
    if env.eq_ignore_ascii_case("off") {
        return Ok(Vec::new());
    }
    let mut keys: Vec<(String, String)> = ACTIONS.iter().map(|(a, _, k)| (a.to_string(), k.to_string())).collect();
    for part in env.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (a, k) = part.split_once('=').ok_or(format!("VIZRS_GLOBAL_KEYS: „{part}” – oczekiwano akcja=skrót"))?;
        let slot = keys
            .iter_mut()
            .find(|(x, _)| x == a.trim())
            .ok_or(format!("VIZRS_GLOBAL_KEYS: nieznana akcja „{}” (take, cont, out, next, clear, show)", a.trim()))?;
        slot.1 = k.trim().to_string();
    }
    keys.retain(|(_, k)| !k.is_empty());
    Ok(keys)
}

/// Okno Trio dla akcji: aktywne Trio, inaczej pierwsze otwarte.
fn trio_label(app: &AppHandle) -> Option<String> {
    let a = state(app);
    let is_trio = |l: &String| Kind::of_label(l) == Kind::Trio;
    a.focused.clone().filter(|l| a.wins.contains_key(l) && is_trio(l)).or_else(|| a.wins.keys().find(|l| is_trio(l)).cloned())
}

/// Wykonuje akcję zasobnika albo skrótu globalnego.
pub(crate) fn action(app: &AppHandle, what: &str) {
    let Some(label) = trio_label(app) else {
        if what == "show" {
            let url = state(app).url(Kind::Trio);
            open_window(app, Kind::Trio, url, None);
        } else {
            eprintln!("skrót globalny „{what}”: brak otwartego okna Trio");
        }
        return;
    };
    if what == "show" {
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.show();
            let _ = w.unminimize();
            let _ = w.set_focus();
        }
        return;
    }
    if ACTIONS.iter().any(|(a, _, _)| *a == what) {
        eval_in(app, &label, &format!("window.__vizrsAction && window.__vizrsAction({})", Value::from(what)));
    }
}

/// Ikona w zasobniku. Błąd (np. Linux bez obsługi zasobnika) – komunikat na wyjściu błędów, okno działa dalej.
pub(crate) fn setup_tray(app: &AppHandle) {
    let build = || -> tauri::Result<()> {
        let mut items: Vec<MenuItem<tauri::Wry>> = Vec::new();
        for (id, label, _) in ACTIONS.iter().rev().take(1).chain(ACTIONS.iter().take(5)) {
            items.push(MenuItem::with_id(app, format!("tray:{id}"), *label, true, None::<&str>)?);
        }
        let connect = MenuItem::with_id(app, "connect", "Połącz z serwerem…", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", "Zakończ", true, None::<&str>)?;
        let (s1, s2) = (PredefinedMenuItem::separator(app)?, PredefinedMenuItem::separator(app)?);
        let mut refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = vec![&items[0], &s1];
        refs.extend(items[1..].iter().map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>));
        refs.extend([&s2 as &dyn tauri::menu::IsMenuItem<tauri::Wry>, &connect, &quit]);
        let menu = Menu::with_items(app, &refs)?;
        let mut b = TrayIconBuilder::with_id("vizrs-trio").menu(&menu).show_menu_on_left_click(false).tooltip("VizRS Trio");
        if let Some(icon) = app.default_window_icon() {
            b = b.icon(icon.clone());
        }
        b.on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                action(tray.app_handle(), "show");
            }
        })
        .build(app)?;
        Ok(())
    };
    if let Err(e) = build() {
        eprintln!("zasobnik systemowy niedostępny: {e}");
    }
}

/// Podpis ikony w zasobniku: serwer okna.
pub(crate) fn set_tooltip(app: &AppHandle, text: &str) {
    if let Some(t) = app.tray_by_id("vizrs-trio") {
        let _ = t.set_tooltip(Some(text));
    }
}

/// Skróty globalne: rejestracja w systemie; zajęte albo niedozwolone (Wayland) – komunikat, reszta działa.
pub(crate) fn setup_shortcuts(app: &AppHandle) -> Vec<(Shortcut, String)> {
    let keys = match keys_config(std::env::var("VIZRS_GLOBAL_KEYS").ok().as_deref()) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{e}; skróty domyślne");
            keys_config(None).unwrap_or_default()
        }
    };
    let mut done = Vec::new();
    for (what, accel) in keys {
        match accel.parse::<Shortcut>() {
            Ok(sc) => match app.global_shortcut().register(sc) {
                Ok(()) => done.push((sc, what)),
                Err(e) => eprintln!("skrót globalny {accel} ({what}): {e}"),
            },
            Err(e) => eprintln!("skrót globalny „{accel}” ({what}): {e}"),
        }
    }
    if !done.is_empty() {
        eprintln!("skróty globalne: {}", done.iter().map(|(s, w)| format!("{w}={s}")).collect::<Vec<_>>().join(", "));
    }
    done
}

/// Obsługa naciśnięcia skrótu globalnego (wtyczka woła ją dla każdego zarejestrowanego).
pub(crate) fn on_shortcut(app: &AppHandle, sc: &Shortcut, st: ShortcutState) {
    if st != ShortcutState::Pressed {
        return;
    }
    let what = app.state::<Shortcuts>().inner().lock().unwrap_or_else(|p| p.into_inner()).iter().find(|(s, _)| s == sc).map(|(_, w)| w.clone());
    if let Some(w) = what {
        action(app, &w);
    }
}

pub(crate) type Shortcuts = Mutex<Vec<(Shortcut, String)>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn konfiguracja_skrotow() {
        let d = keys_config(None).unwrap();
        assert_eq!(d.len(), 6);
        assert!(d.iter().all(|(_, k)| k.parse::<Shortcut>().is_ok()), "domyślne skróty da się zarejestrować");
        assert!(keys_config(Some("off")).unwrap().is_empty());
        let c = keys_config(Some("take=Ctrl+Shift+T, clear=")).unwrap();
        assert_eq!(c.iter().find(|(a, _)| a == "take").unwrap().1, "Ctrl+Shift+T");
        assert!(!c.iter().any(|(a, _)| a == "clear"));
        assert!(keys_config(Some("zle=Ctrl+X")).is_err());
        assert!(keys_config(Some("take")).is_err());
    }
}
