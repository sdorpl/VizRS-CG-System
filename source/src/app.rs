//! Okna aplikacji: Artist, Trio i Output w osobnym oknie przeglądarki z silnikiem Chromium
//! (Edge, Chrome, Chromium, Brave, Vivaldi) w trybie `--app` – bez pasków, z własnym profilem.
//!
//! Chromium, bo wyjście w OBS/vMix też rysuje Chromium: Artist pokazuje to samo co antena
//! (pomiar tekstu, ściskanie, łamanie wierszy). Bez Chromium – zwykła przeglądarka systemowa.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// Które okno otworzyć.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Artist,
    Trio,
    Output,
}

impl Window {
    pub fn path(self) -> &'static str {
        match self {
            Window::Artist => "/artist",
            Window::Trio => "/trio",
            Window::Output => "/output",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Window::Artist => "Artist",
            Window::Trio => "Trio",
            Window::Output => "Output",
        }
    }
}

/// Ustawienia okna z linii poleceń.
#[derive(Debug, Clone, Default)]
pub struct WindowOpts {
    /// Pełny ekran bez ramek (wyjście na monitor/HDMI); wyjście: Alt+F4.
    pub fullscreen: bool,
    /// Pozycja okna `X,Y` (np. `1920,0` = drugi monitor).
    pub position: Option<String>,
    /// Dodatkowe parametry adresu, np. `mode=key` dla wyjścia.
    pub query: Option<String>,
    /// Przeglądarka wskazana ręcznie (`--browser` / `VIZRS_BROWSER`).
    pub browser: Option<String>,
}

/// Adres okna: `base` (http://host:port) + ścieżka okna + parametry + token.
pub fn window_url(base: &str, w: Window, query: Option<&str>, token: Option<&str>) -> String {
    let mut params: Vec<String> = vec![];
    if let Some(q) = query.map(|q| q.trim_start_matches(['?', '&'])).filter(|q| !q.is_empty()) {
        params.push(q.to_string());
    }
    if let Some(t) = token {
        params.push(format!("token={t}"));
    }
    let base = base.trim_end_matches('/');
    if params.is_empty() {
        format!("{base}{}", w.path())
    } else {
        format!("{base}{}?{}", w.path(), params.join("&"))
    }
}

/// Sprawdza adres serwera zdalnego (`--connect`): tylko http(s), bez spacji i fragmentów.
pub fn check_connect(url: &str) -> Result<String, String> {
    let u = url.trim().trim_end_matches('/');
    let rest = u.strip_prefix("https://").or_else(|| u.strip_prefix("http://")).ok_or("--connect: adres musi zaczynać się od http:// albo https://")?;
    if rest.is_empty() || u.chars().any(|c| c.is_whitespace() || matches!(c, '?' | '#' | '"' | '\'' | '<' | '>')) {
        return Err(format!("--connect: nieprawidłowy adres „{url}” (podaj np. https://cg.example.pl)"));
    }
    Ok(u.to_string())
}

/// Katalog konfiguracji użytkownika VizRS (profil okien przeglądarki).
pub fn config_dir() -> PathBuf {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(windows) {
        if let Some(a) = env("APPDATA") {
            return a.join("VizRS");
        }
    } else if cfg!(target_os = "macos") {
        if let Some(h) = env("HOME") {
            return h.join("Library/Application Support/VizRS");
        }
    } else if let Some(x) = env("XDG_CONFIG_HOME") {
        return x.join("vizrs");
    } else if let Some(h) = env("HOME") {
        return h.join(".config/vizrs");
    }
    std::env::temp_dir().join("vizrs")
}

/// Profil przeglądarki: osobny dla wyjścia (pełny ekran i pozycja okna działają tylko w nowym procesie),
/// wspólny dla Artist i Trio (te same ustawienia, np. imię operatora).
pub fn profile_dir(w: Window) -> PathBuf {
    config_dir().join(if w == Window::Output { "browser-output" } else { "browser" })
}

const LINUX_NAMES: &[&str] = &["chromium", "chromium-browser", "google-chrome", "google-chrome-stable", "microsoft-edge", "microsoft-edge-stable", "brave-browser", "brave", "vivaldi"];

fn is_exec(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        p.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        p.is_file()
    }
}

/// Szuka pierwszej z `names` w katalogach `path` (wartość zmiennej PATH).
pub fn find_in_path(names: &[&str], path: &str) -> Option<PathBuf> {
    let sep = if cfg!(windows) { ';' } else { ':' };
    names.iter().find_map(|n| path.split(sep).filter(|d| !d.is_empty()).map(|d| Path::new(d).join(n)).find(|p| is_exec(p)))
}

fn known_locations() -> Vec<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let mut v = vec![];
    if cfg!(windows) {
        for base in ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"].iter().filter_map(|k| env(k)) {
            v.push(base.join(r"Microsoft\Edge\Application\msedge.exe"));
            v.push(base.join(r"Google\Chrome\Application\chrome.exe"));
            v.push(base.join(r"BraveSoftware\Brave-Browser\Application\brave.exe"));
            v.push(base.join(r"Chromium\Application\chrome.exe"));
        }
    } else if cfg!(target_os = "macos") {
        let apps = ["Google Chrome.app/Contents/MacOS/Google Chrome", "Microsoft Edge.app/Contents/MacOS/Microsoft Edge", "Chromium.app/Contents/MacOS/Chromium", "Brave Browser.app/Contents/MacOS/Brave Browser"];
        let mut roots = vec![PathBuf::from("/Applications")];
        if let Some(h) = env("HOME") {
            roots.push(h.join("Applications"));
        }
        for r in &roots {
            v.extend(apps.iter().map(|a| r.join(a)));
        }
    } else {
        v.push(PathBuf::from("/var/lib/flatpak/exports/bin/org.chromium.Chromium"));
        v.push(PathBuf::from("/var/lib/flatpak/exports/bin/com.google.Chrome"));
        v.push(PathBuf::from("/snap/bin/chromium"));
    }
    v
}

/// Polecenie przeglądarki: ręcznie wskazana (ścieżka albo polecenie z argumentami), inaczej wyszukana.
pub fn find_browser(over: Option<&str>) -> Option<Vec<String>> {
    if let Some(o) = over.map(str::trim).filter(|o| !o.is_empty()) {
        // ścieżka może zawierać spacje („C:\Program Files\…”); inaczej dziel na polecenie i argumenty
        if Path::new(o).is_file() {
            return Some(vec![o.to_string()]);
        }
        return Some(o.split_whitespace().map(str::to_string).collect());
    }
    if !cfg!(windows) && !cfg!(target_os = "macos") {
        if let Some(p) = std::env::var("PATH").ok().and_then(|path| find_in_path(LINUX_NAMES, &path)) {
            return Some(vec![p.to_string_lossy().into_owned()]);
        }
    }
    known_locations().into_iter().find(|p| is_exec(p)).map(|p| vec![p.to_string_lossy().into_owned()])
}

/// Argumenty Chromium dla okna aplikacji.
pub fn chromium_args(url: &str, w: Window, profile: &Path, o: &WindowOpts) -> Vec<String> {
    let mut a = vec![
        format!("--app={url}"),
        format!("--user-data-dir={}", profile.display()),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        // klasa okna (Linux) – skrót .desktop przypina właściwą ikonę
        format!("--class=VizRS-{}", w.name()),
        // wideo w scenach gra bez kliknięcia; animacje nie zwalniają, gdy okno jest zasłonięte
        "--autoplay-policy=no-user-gesture-required".into(),
        "--disable-background-timer-throttling".into(),
        "--disable-renderer-backgrounding".into(),
        "--disable-backgrounding-occluded-windows".into(),
    ];
    if let Some(p) = &o.position {
        a.push(format!("--window-position={p}"));
    }
    if o.fullscreen {
        a.push("--kiosk".into());
    } else {
        a.push(if w == Window::Output { "--window-size=1920,1080".into() } else { "--window-size=1600,900".into() });
    }
    a
}

/// Uruchomione okno: proces przeglądarki (do czekania na zamknięcie) albo `None`,
/// gdy adres przekazano przeglądarce systemowej (nie wiadomo, kiedy okno się zamknie).
pub struct Opened {
    pub child: Option<Child>,
    pub how: String,
}

/// Otwiera okno; bez Chromium – przeglądarka systemowa.
pub fn open(url: &str, w: Window, o: &WindowOpts) -> Result<Opened, String> {
    if let Some(cmd) = find_browser(o.browser.as_deref()) {
        let profile = profile_dir(w);
        std::fs::create_dir_all(&profile).ok();
        let (prog, pre) = cmd.split_first().ok_or("pusta nazwa przeglądarki")?;
        let child = Command::new(prog)
            .args(pre)
            .args(chromium_args(url, w, &profile, o))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("nie mogę uruchomić {prog}: {e}"))?;
        return Ok(Opened { child: Some(child), how: prog.clone() });
    }
    open_default(url)?;
    Ok(Opened { child: None, how: "przeglądarka systemowa (brak Chromium/Edge/Chrome – okno z paskami)".into() })
}

fn open_default(url: &str) -> Result<(), String> {
    let mut c = if cfg!(windows) {
        // bez powłoki cmd – adres z & nie jest interpretowany
        let mut c = Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(url);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map(|_| ()).map_err(|e| format!("nie mogę otworzyć przeglądarki: {e}. Otwórz ręcznie: {url}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(window_url("http://127.0.0.1:8088", Window::Trio, None, None), "http://127.0.0.1:8088/trio");
        assert_eq!(window_url("https://cg.pl/", Window::Artist, None, Some("t1")), "https://cg.pl/artist?token=t1");
        assert_eq!(window_url("http://h:1", Window::Output, Some("?mode=key&bg=black"), Some("o")), "http://h:1/output?mode=key&bg=black&token=o");
    }

    #[test]
    fn connect_urls_validated() {
        assert_eq!(check_connect("https://cg.example.pl/").unwrap(), "https://cg.example.pl");
        assert_eq!(check_connect("http://10.0.0.5:8088").unwrap(), "http://10.0.0.5:8088");
        assert!(check_connect("cg.example.pl").is_err());
        assert!(check_connect("ftp://x").is_err());
        assert!(check_connect("https://").is_err());
        assert!(check_connect("https://a b").is_err());
        assert!(check_connect("https://x/?token=1").is_err());
    }

    #[test]
    fn args_for_windows() {
        let p = Path::new("/tmp/prof");
        let a = chromium_args("http://h/trio", Window::Trio, p, &WindowOpts::default());
        assert!(a.contains(&"--app=http://h/trio".to_string()));
        assert!(a.contains(&"--user-data-dir=/tmp/prof".to_string()));
        assert!(a.contains(&"--window-size=1600,900".to_string()));
        assert!(!a.iter().any(|x| x == "--kiosk"));
        let o = WindowOpts { fullscreen: true, position: Some("1920,0".into()), ..Default::default() };
        let a = chromium_args("http://h/output", Window::Output, p, &o);
        assert!(a.contains(&"--kiosk".to_string()) && a.contains(&"--window-position=1920,0".to_string()));
        assert!(!a.iter().any(|x| x.starts_with("--window-size")));
        assert_ne!(profile_dir(Window::Output), profile_dir(Window::Trio));
        assert_eq!(profile_dir(Window::Artist), profile_dir(Window::Trio));
    }

    #[cfg(unix)]
    #[test]
    fn finds_executable_in_path() {
        use std::os::unix::fs::PermissionsExt;
        let d = std::env::temp_dir().join(format!("vizrs-app-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("google-chrome");
        std::fs::write(&f, "#!/bin/sh\n").unwrap();
        let path = format!("/nie/ma:{}", d.display());
        assert_eq!(find_in_path(LINUX_NAMES, &path), None, "plik bez prawa wykonania");
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(find_in_path(LINUX_NAMES, &path), Some(f.clone()));
        // ręcznie wskazana: ścieżka (ze spacją też) albo polecenie z argumentami
        assert_eq!(find_browser(Some(f.to_str().unwrap())), Some(vec![f.to_string_lossy().into_owned()]));
        assert_eq!(find_browser(Some("flatpak run org.chromium.Chromium")), Some(vec!["flatpak".into(), "run".into(), "org.chromium.Chromium".into()]));
        std::fs::remove_dir_all(d).ok();
    }
}
