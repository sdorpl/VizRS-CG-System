//! Konfiguracja z linii poleceń i zmiennych środowiskowych `VIZRS_*`.
//!
//! Kolejność: wartości domyślne → zmienne środowiskowe → flagi (flagi wygrywają).
//! Parser jest czystą funkcją (argumenty + funkcja odczytu zmiennych) – testowalną bez procesu.

use std::path::PathBuf;

use crate::app::{Window, WindowOpts};
use crate::{app, guard, hook};

/// Co ma zrobić binarka.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Serwer bez GUI (domyślnie, także bez podkomendy).
    Serve,
    /// Sprawdź `/health` działającego serwera (kod wyjścia 0/1) – dla Dockera i monitoringu.
    Healthcheck,
    /// Okno aplikacji (Artist, Trio, Output) z serwerem w tym samym procesie albo zdalnym (`--connect`).
    App(Window),
}

#[derive(Debug, Clone)]
pub struct Config {
    pub mode: Mode,
    pub port: u16,
    pub tcp_port: u16,
    pub bind: String,
    pub token: Option<String>,
    /// Token tylko do odczytu dla wyjść (OBS, vMix): `/output`, zdarzenia, fonty, grafiki.
    pub output_token: Option<String>,
    pub osc_port: u16,
    pub webhook: Option<String>,
    pub data: PathBuf,
    pub web_dir: Option<PathBuf>,
    /// Zgoda na nasłuch w sieci bez tokenu (`--insecure` / `VIZRS_INSECURE=1`).
    pub insecure: bool,
    /// Adres klienta z `X-Forwarded-For` (serwer za reverse proxy); bez limitu połączeń na adres gniazda.
    pub trust_proxy: bool,
    /// Log audytowy także na standardowe wyjście (JSON w linii) – `docker logs`.
    pub log_stdout: bool,
    /// Serwer zdalny dla okna (`--connect https://…`): bez serwera lokalnego.
    pub connect: Option<String>,
    /// Ustawienia okna aplikacji.
    pub window: WindowOpts,
    /// Ile sekund bez żadnego okna/wyjścia, zanim serwer okna aplikacji zakończy pracę.
    pub linger_s: u64,
    /// `serve --idle-exit N`: zakończ serwer po N s bez okien i wyjść (serwer uruchomiony przez okno powłoki).
    pub idle_exit: Option<u64>,
    /// Identyfikator tej instancji serwera (z blokady katalogu danych), widoczny w `/health`.
    pub instance: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mode: Mode::Serve,
            port: 8088,
            tcp_port: 6100,
            bind: "127.0.0.1".into(),
            token: None,
            output_token: None,
            osc_port: 0,
            webhook: None,
            data: PathBuf::from("vizrs-data"),
            web_dir: None,
            insecure: false,
            trust_proxy: false,
            log_stdout: false,
            connect: None,
            window: WindowOpts::default(),
            linger_s: 10,
            idle_exit: None,
            instance: String::new(),
        }
    }
}

impl Config {
    pub fn tokens(&self) -> crate::guard::Tokens {
        crate::guard::Tokens { operator: self.token.clone(), output: self.output_token.clone() }
    }

    /// Adres, pod którym ten serwer jest osiągalny z tej samej maszyny (`--healthcheck`, blokada danych).
    pub fn local_addr(&self) -> String {
        let host = match self.bind.as_str() {
            "" | "0.0.0.0" => "127.0.0.1",
            "::" | "[::]" => "[::1]",
            b => b,
        };
        if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]:{}", self.port)
        } else {
            format!("{host}:{}", self.port)
        }
    }
}

pub enum Cli {
    Run(Box<Config>),
    Help,
    Version,
}

/// Adres pętli zwrotnej – nasłuch niedostępny z sieci.
pub fn is_loopback(bind: &str) -> bool {
    let b = bind.trim_start_matches('[').trim_end_matches(']');
    b == "localhost" || b == "::1" || b.starts_with("127.")
}

fn num(name: &str, v: &str) -> Result<u16, String> {
    v.trim().parse().map_err(|_| format!("{name}: nieprawidłowy numer portu „{v}” (0–65535)"))
}

fn token(name: &str, v: &str) -> Result<String, String> {
    if guard::token_valid(v) { Ok(v.to_string()) } else { Err(format!("{name}: dozwolone znaki A-Z a-z 0-9 - _ . (max 128)")) }
}

fn webhook(name: &str, v: &str) -> Result<String, String> {
    hook::parse_url(v).map(|_| v.to_string()).map_err(|e| format!("{name}: {e}"))
}

fn flag_true(v: &str) -> bool {
    matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "tak")
}

/// Parsuje argumenty (bez nazwy programu). `env` zwraca wartość zmiennej (puste = brak).
pub fn parse(args: &[String], env: &dyn Fn(&str) -> Option<String>) -> Result<Cli, String> {
    let mut c = Config::default();
    let env = |k: &str| env(k).filter(|v| !v.trim().is_empty());

    // 1. zmienne środowiskowe
    if let Some(v) = env("VIZRS_PORT") {
        c.port = num("VIZRS_PORT", &v)?;
    }
    if let Some(v) = env("VIZRS_TCP") {
        c.tcp_port = num("VIZRS_TCP", &v)?;
    }
    if let Some(v) = env("VIZRS_OSC") {
        c.osc_port = num("VIZRS_OSC", &v)?;
    }
    if let Some(v) = env("VIZRS_BIND") {
        c.bind = v;
    }
    if let Some(v) = env("VIZRS_TOKEN") {
        c.token = Some(token("VIZRS_TOKEN", &v)?);
    }
    if let Some(v) = env("VIZRS_OUTPUT_TOKEN") {
        c.output_token = Some(token("VIZRS_OUTPUT_TOKEN", &v)?);
    }
    if let Some(v) = env("VIZRS_TRUST_PROXY") {
        c.trust_proxy = flag_true(&v);
    }
    if let Some(v) = env("VIZRS_LOG_STDOUT") {
        c.log_stdout = flag_true(&v);
    }
    if let Some(v) = env("VIZRS_WEBHOOK") {
        c.webhook = Some(webhook("VIZRS_WEBHOOK", &v)?);
    }
    if let Some(v) = env("VIZRS_DATA") {
        c.data = PathBuf::from(v);
    }
    if let Some(v) = env("VIZRS_WEB") {
        c.web_dir = Some(PathBuf::from(v));
    }
    if let Some(v) = env("VIZRS_CONNECT") {
        c.connect = Some(app::check_connect(&v)?);
    }
    if let Some(v) = env("VIZRS_BROWSER") {
        c.window.browser = Some(v);
    }
    if let Some(v) = env("VIZRS_INSECURE") {
        c.insecure = flag_true(&v);
    }

    // 2. flagi i podkomenda
    let mut i = 0;
    let mut mode_set = false;
    while i < args.len() {
        let a = args[i].as_str();
        // `--opcja=wartość` albo `--opcja wartość`
        let (name, inline) = match a.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n, Some(v.to_string())),
            _ => (a, None),
        };
        let mut val = || -> Result<String, String> {
            if let Some(v) = inline.clone() {
                return Ok(v);
            }
            i += 1;
            args.get(i).cloned().ok_or(format!("{name}: brak wartości"))
        };
        match name {
            "--port" | "-p" => c.port = num(name, &val()?)?,
            "--tcp" => c.tcp_port = num(name, &val()?)?,
            "--osc" => c.osc_port = num(name, &val()?)?,
            "--bind" => c.bind = val()?,
            "--token" => c.token = Some(token(name, &val()?)?),
            "--output-token" => c.output_token = Some(token(name, &val()?)?),
            "--trust-proxy" => c.trust_proxy = true,
            "--log-stdout" => c.log_stdout = true,
            "--webhook" => c.webhook = Some(webhook(name, &val()?)?),
            "--data" | "-d" => c.data = PathBuf::from(val()?),
            "--web" => c.web_dir = Some(PathBuf::from(val()?)),
            "--insecure" => c.insecure = true,
            "--healthcheck" => c.mode = Mode::Healthcheck,
            "--connect" => c.connect = Some(app::check_connect(&val()?)?),
            "--browser" => c.window.browser = Some(val()?),
            "--fullscreen" => c.window.fullscreen = true,
            "--position" => {
                let v = val()?;
                let ok = v.split_once(',').map(|(x, y)| x.trim().parse::<i32>().is_ok() && y.trim().parse::<i32>().is_ok()).unwrap_or(false);
                if !ok {
                    return Err(format!("--position: podaj X,Y (np. 1920,0), nie „{v}”"));
                }
                c.window.position = Some(v.replace(' ', ""));
            }
            "--query" => c.window.query = Some(val()?),
            "--idle-exit" => c.idle_exit = Some(val()?.trim().parse().map_err(|_| "--idle-exit: liczba sekund".to_string())?),
            "--linger" => c.linger_s = val()?.trim().parse().map_err(|_| "--linger: liczba sekund".to_string())?,
            "--version" | "-V" => return Ok(Cli::Version),
            "--help" | "-h" => return Ok(Cli::Help),
            "serve" if !mode_set => mode_set = true,
            "artist" | "trio" | "output" if !mode_set => {
                mode_set = true;
                c.mode = Mode::App(match a {
                    "artist" => Window::Artist,
                    "trio" => Window::Trio,
                    _ => Window::Output,
                });
            }
            "help" if !mode_set => return Ok(Cli::Help),
            _ if a.starts_with('-') => return Err(format!("nieznana opcja {a}")),
            _ if !mode_set => return Err(format!("nieznana komenda „{a}” (dostępne: serve, artist, trio, output)")),
            _ => return Err(format!("nadmiarowy argument „{a}”")),
        }
        i += 1;
    }

    if c.output_token.is_some() && c.token.is_none() {
        return Err("--output-token wymaga tokenu operatora (--token / VIZRS_TOKEN)".into());
    }
    if c.output_token.is_some() && c.output_token == c.token {
        return Err("token wyjścia musi być inny niż token operatora".into());
    }
    let window_opts = c.window.fullscreen || c.window.position.is_some() || c.window.query.is_some() || c.connect.is_some();
    if window_opts && !matches!(c.mode, Mode::App(_)) {
        return Err("--connect, --fullscreen, --position i --query dotyczą okna: vizrs artist|trio|output …".into());
    }
    if c.idle_exit.is_some() && c.mode != Mode::Serve {
        return Err("--idle-exit dotyczy komendy serve (okna mają --linger)".into());
    }
    let local_server = match c.mode {
        Mode::Serve => true,
        Mode::App(_) => c.connect.is_none(),
        Mode::Healthcheck => false,
    };
    if local_server && c.token.is_none() && !is_loopback(&c.bind) && !c.insecure {
        return Err(format!(
            "nasłuch na {} bez tokenu: każdy w sieci mógłby sterować anteną.\nUstaw --token (albo VIZRS_TOKEN); świadomie bez tokenu: --insecure",
            c.bind
        ));
    }
    Ok(Cli::Run(Box::new(c)))
}

pub fn help() -> String {
    format!(
        "VizRS {}

Użycie: vizrs [serve] [opcje]
        vizrs artist|trio|output [opcje] [--connect https://serwer]
        vizrs --healthcheck [--port …] [--token …]

Komendy:
  serve          serwer bez GUI (domyślnie): UI w przeglądarce, wyjście OBS, API
  artist, trio   okno aplikacji (Edge/Chrome/Chromium w trybie --app, bez pasków) + serwer w tym samym
                 procesie; jeśli serwer na tym --data już działa – tylko nowe okno. Po zamknięciu okna serwer
                 działa dalej, dopóki podłączone jest jakiekolwiek okno lub wyjście (OBS)
  output         okno wyjścia 1920×1080 (np. na drugi monitor / HDMI: --fullscreen --position 1920,0)

Okna:
  --connect URL  okno dla serwera zdalnego, np. https://cg.example.pl (VIZRS_CONNECT); bez serwera lokalnego
  --browser CMD  przeglądarka z Chromium (VIZRS_BROWSER), np. /usr/bin/chromium; domyślnie wyszukiwana
  --fullscreen   pełny ekran bez ramek (wyjście: Alt+F4)
  --position X,Y pozycja okna (np. 1920,0 = drugi monitor po prawej)
  --query PARAM  parametry adresu okna, np. mode=key albo bg=black (wyjście)
  --linger N     sekundy bez okien i wyjść przed zakończeniem serwera okna (domyślnie 10)

Opcje (w nawiasie zmienna środowiskowa; flaga ma pierwszeństwo):
  --port N       port HTTP – UI + wyjście OBS (VIZRS_PORT, domyślnie 8088)
  --tcp N        port komend tekstowych, 0 = wyłączony (VIZRS_TCP, domyślnie 6100)
  --osc N        port UDP OSC, 0 = wyłączony (VIZRS_OSC); przy tokenie: /vizrs/<token>/take
  --bind ADRES   adres nasłuchu (VIZRS_BIND, domyślnie 127.0.0.1); inny niż lokalny wymaga tokenu
  --token HASŁO  wymagaj tokenu (VIZRS_TOKEN). UI: otwórz raz z ?token=…; API: Authorization: Bearer,
                 X-Token albo ?token=; TCP: pierwsza linia AUTH …
  --output-token HASŁO  drugi token, tylko do odczytu: /output, zdarzenia, fonty, grafiki – dla OBS/vMix
                 (VIZRS_OUTPUT_TOKEN); wyciek adresu wyjścia nie daje sterowania anteną
  --insecure     pozwól na nasłuch w sieci bez tokenu (VIZRS_INSECURE=1) – tylko w zaufanej sieci
  --trust-proxy  serwer za reverse proxy: adres klienta z X-Forwarded-For (log, limity) (VIZRS_TRUST_PROXY=1)
  --log-stdout   log audytowy także na standardowe wyjście, JSON w linii (VIZRS_LOG_STDOUT=1)
  --webhook URL  http://host:port/ścieżka dla zdarzeń onAir/offAir/error (VIZRS_WEBHOOK)
  --data KAT     katalog z show.json i assets/ (VIZRS_DATA, domyślnie vizrs-data)
  --web KAT      serwuj pliki UI z dysku zamiast wbudowanych (VIZRS_WEB)
  --idle-exit N  serve: zakończ po N s bez podłączonych okien i wyjść (serwer uruchamiany przez okno)
  --healthcheck  sprawdź /health działającego serwera i zakończ (0 = ok, 1 = błąd)
  --version      wersja i skrót commita",
        env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str], env: &[(&str, &str)]) -> Result<Config, String> {
        let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let e: Vec<(String, String)> = env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        match parse(&a, &|k| e.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()))? {
            Cli::Run(c) => Ok(*c),
            _ => Err("help/version".into()),
        }
    }

    #[test]
    fn defaults_without_args() {
        let c = run(&[], &[]).unwrap();
        assert_eq!((c.port, c.tcp_port, c.osc_port, c.bind.as_str(), c.mode), (8088, 6100, 0, "127.0.0.1", Mode::Serve));
        assert!(c.token.is_none());
        assert_eq!(c.data, PathBuf::from("vizrs-data"));
    }

    #[test]
    fn env_then_flags_win() {
        let env = [("VIZRS_PORT", "9000"), ("VIZRS_TCP", "0"), ("VIZRS_DATA", "/data"), ("VIZRS_TOKEN", "abc"), ("VIZRS_BIND", "0.0.0.0")];
        let c = run(&[], &env).unwrap();
        assert_eq!((c.port, c.tcp_port, c.bind.as_str()), (9000, 0, "0.0.0.0"));
        assert_eq!(c.data, PathBuf::from("/data"));
        assert_eq!(c.token.as_deref(), Some("abc"));
        let c = run(&["serve", "--port", "9100", "--token=xyz"], &env).unwrap();
        assert_eq!((c.port, c.token.as_deref()), (9100, Some("xyz")));
        // pusta zmienna = brak
        assert_eq!(run(&[], &[("VIZRS_PORT", "")]).unwrap().port, 8088);
    }

    #[test]
    fn bad_values_are_errors() {
        assert!(run(&["--port", "abc"], &[]).is_err());
        assert!(run(&["--port", "70000"], &[]).is_err());
        assert!(run(&["--port"], &[]).is_err());
        assert!(run(&["--token", "a b"], &[]).is_err());
        assert!(run(&["--webhook", "ftp://x"], &[]).is_err());
        assert!(run(&["--prot", "1"], &[]).is_err(), "literówka w opcji");
        assert!(run(&["sevre"], &[]).is_err());
        assert!(run(&[], &[("VIZRS_PORT", "x")]).is_err());
        assert!(run(&[], &[("VIZRS_TOKEN", "a;b")]).is_err());
    }

    #[test]
    fn network_bind_requires_token() {
        assert!(run(&["--bind", "0.0.0.0"], &[]).is_err());
        assert!(run(&["--bind", "192.168.1.5"], &[]).is_err());
        assert!(run(&["--bind", "0.0.0.0", "--token", "t"], &[]).is_ok());
        assert!(run(&["--bind", "0.0.0.0", "--insecure"], &[]).is_ok());
        assert!(run(&[], &[("VIZRS_BIND", "0.0.0.0"), ("VIZRS_INSECURE", "1")]).is_ok());
        assert!(run(&["--bind", "127.0.0.2"], &[]).is_ok());
        assert!(run(&["--bind", "localhost"], &[]).is_ok());
        assert!(run(&["--bind", "::1"], &[]).is_ok());
        // healthcheck nie nasłuchuje – nie wymaga tokenu
        assert!(run(&["--healthcheck"], &[("VIZRS_BIND", "0.0.0.0")]).is_ok());
    }

    #[test]
    fn output_token_and_proxy_options() {
        let c = run(&["--token", "op", "--output-token", "out", "--trust-proxy", "--log-stdout"], &[]).unwrap();
        assert_eq!(c.output_token.as_deref(), Some("out"));
        assert!(c.trust_proxy && c.log_stdout);
        let c = run(&[], &[("VIZRS_TOKEN", "op"), ("VIZRS_OUTPUT_TOKEN", "out"), ("VIZRS_TRUST_PROXY", "1"), ("VIZRS_LOG_STDOUT", "true")]).unwrap();
        assert!(c.output_token.is_some() && c.trust_proxy && c.log_stdout);
        assert!(run(&["--output-token", "out"], &[]).is_err(), "bez tokenu operatora");
        assert!(run(&["--token", "x", "--output-token", "x"], &[]).is_err(), "taki sam token");
    }

    #[test]
    fn window_modes() {
        let c = run(&["--data", "x", "trio"], &[]).unwrap();
        assert_eq!(c.mode, Mode::App(Window::Trio));
        let c = run(&["output", "--fullscreen", "--position", "1920, 0", "--query", "mode=key"], &[]).unwrap();
        assert_eq!(c.mode, Mode::App(Window::Output));
        assert!(c.window.fullscreen);
        assert_eq!(c.window.position.as_deref(), Some("1920,0"));
        assert!(run(&["output", "--position", "lewo"], &[]).is_err());
        let c = run(&["artist", "--connect", "https://cg.example.pl/"], &[]).unwrap();
        assert_eq!(c.connect.as_deref(), Some("https://cg.example.pl"));
        assert!(run(&["artist", "--connect", "cg.example.pl"], &[]).is_err());
        // opcje okna bez okna – błąd
        assert!(run(&["--fullscreen"], &[]).is_err());
        assert!(run(&["serve", "--connect", "https://x.pl"], &[]).is_err());
        // okno zdalne nie nasłuchuje – nie wymaga tokenu przy bind z sieci; lokalne – wymaga
        assert!(run(&["trio", "--connect", "https://x.pl"], &[("VIZRS_BIND", "0.0.0.0")]).is_ok());
        assert!(run(&["trio"], &[("VIZRS_BIND", "0.0.0.0")]).is_err());
        assert_eq!(run(&["trio", "--linger", "3"], &[]).unwrap().linger_s, 3);
        assert!(run(&["trio", "artist"], &[]).is_err(), "jedno okno na wywołanie");
        assert_eq!(run(&["serve", "--idle-exit", "5"], &[]).unwrap().idle_exit, Some(5));
        assert!(run(&["trio", "--idle-exit", "5"], &[]).is_err());
    }

    #[test]
    fn healthcheck_mode_and_local_addr() {
        let c = run(&["--healthcheck", "--port", "8099"], &[("VIZRS_BIND", "0.0.0.0")]).unwrap();
        assert_eq!(c.mode, Mode::Healthcheck);
        assert_eq!(c.local_addr(), "127.0.0.1:8099");
        let c = run(&["--bind", "::1", "--port", "1"], &[]).unwrap();
        assert_eq!(c.local_addr(), "[::1]:1");
    }
}
