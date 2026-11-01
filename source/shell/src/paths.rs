//! Katalogi użytkownika okna natywnego i zapamiętany stan okien (`window.json`).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

fn env(k: &str) -> Option<PathBuf> {
    std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn home() -> Option<PathBuf> {
    env("HOME")
}

/// Domyślny katalog danych: `./vizrs-data`, jeśli już jest (dotychczasowe instalacje), inaczej katalog użytkownika.
/// Względna ścieżka bez sprawdzenia zależałaby od tego, skąd uruchomiono okno.
pub fn default_data() -> PathBuf {
    let cwd = PathBuf::from("vizrs-data");
    if cwd.join("show.json").is_file() {
        return cwd;
    }
    user_data(env("XDG_DATA_HOME"), home(), env("LOCALAPPDATA")).unwrap_or(cwd)
}

fn user_data(xdg: Option<PathBuf>, home: Option<PathBuf>, local: Option<PathBuf>) -> Option<PathBuf> {
    if cfg!(windows) {
        Some(local?.join("VizRS"))
    } else if cfg!(target_os = "macos") {
        Some(home?.join("Library/Application Support/VizRS/data"))
    } else {
        Some(xdg.or_else(|| home.map(|h| h.join(".local/share")))?.join("vizrs"))
    }
}

/// Katalog stanu: logi serwera i kopie robocze (`~/.local/state/vizrs`).
pub fn state_dir() -> PathBuf {
    let dir = if cfg!(windows) {
        env("LOCALAPPDATA").map(|d| d.join("VizRS"))
    } else if cfg!(target_os = "macos") {
        home().map(|h| h.join("Library/Logs/VizRS"))
    } else {
        env("XDG_STATE_HOME").or_else(|| home().map(|h| h.join(".local/state"))).map(|d| d.join("vizrs"))
    };
    dir.unwrap_or_else(|| std::env::temp_dir().join("vizrs"))
}

/// Zapis podmieniany atomowo; awaria w połowie nie zostawia uciętego pliku.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Zapamiętany wygląd okna jednego rodzaju.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WinState {
    pub size: Option<(f64, f64)>,
    /// tylko X11 – Wayland nie podaje pozycji okna
    pub pos: Option<(i32, i32)>,
    pub maximized: Option<bool>,
    pub zoom: Option<f64>,
    pub scene: Option<String>,
}

fn file() -> Option<PathBuf> {
    Some(crate::connect::config_path()?.with_file_name("window.json"))
}

fn parse(v: &Value) -> WinState {
    let f = |k: &str| v[k].as_f64();
    WinState {
        size: f("w").zip(f("h")).filter(|(w, h)| (320.0..=16384.0).contains(w) && (200.0..=16384.0).contains(h)),
        pos: v["x"].as_i64().zip(v["y"].as_i64()).filter(|(x, y)| x.abs() < 100_000 && y.abs() < 100_000).map(|(x, y)| (x as i32, y as i32)),
        maximized: v["maximized"].as_bool(),
        zoom: f("zoom").filter(|z| (0.3..=3.0).contains(z)),
        scene: v["scene"].as_str().filter(|s| s.len() <= 128).map(String::from),
    }
}

/// Stan okna danego rodzaju (uszkodzony plik = domyślny).
pub fn load(kind: &str) -> WinState {
    let text = file().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    parse(&serde_json::from_str::<Value>(&text).unwrap_or(Value::Null)[kind])
}

pub fn save(kind: &str, s: &WinState) {
    let Some(p) = file() else { return };
    let mut all = std::fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).filter(Value::is_object).unwrap_or_else(|| json!({}));
    all[kind] = json!({
        "w": s.size.map(|v| v.0), "h": s.size.map(|v| v.1),
        "x": s.pos.map(|v| v.0), "y": s.pos.map(|v| v.1),
        "maximized": s.maximized, "zoom": s.zoom, "scene": s.scene,
    });
    if let Err(e) = write_atomic(&p, &all.to_string()) {
        eprintln!("zapis stanu okna: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn data_follows_xdg() {
        assert_eq!(user_data(Some("/x".into()), Some("/h".into()), None), Some(PathBuf::from("/x/vizrs")));
        assert_eq!(user_data(None, Some("/h".into()), None), Some(PathBuf::from("/h/.local/share/vizrs")));
        assert_eq!(user_data(None, None, None), None);
    }

    #[test]
    fn hostile_window_state_is_dropped() {
        let s = parse(&json!({"w": 1e9, "h": 10, "x": 1e12, "y": 0, "zoom": 99, "scene": "x".repeat(500), "maximized": "tak"}));
        assert_eq!(s, WinState::default());
        let s = parse(&json!({"w": 800, "h": 600, "x": 10, "y": 20, "zoom": 1.5, "scene": "a", "maximized": true}));
        assert_eq!(s.size, Some((800.0, 600.0)));
        assert_eq!(s.pos, Some((10, 20)));
        assert_eq!(s.scene.as_deref(), Some("a"));
    }
}
