//! Serwer dla okien powłoki: dołącz do działającego `vizrs` na tym katalogu danych albo uruchom
//! `vizrs serve --idle-exit N` jako osobny proces. Osobny proces = awaria okna nie zdejmuje anteny,
//! a serwer kończy pracę sam, gdy nie ma już okien ani wyjść (OBS).

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Plik blokady katalogu danych zakładany przez `vizrs` (src/datalock.rs w rdzeniu).
const LOCK_FILE: &str = ".vizrs.lock";
/// Ile sekund bez okien i wyjść, zanim serwer uruchomiony przez powłokę zakończy pracę.
const IDLE_EXIT_S: u32 = 10;

/// `GET` na `addr` (`host:port`), odpowiedź do 1 MB. Zwraca (kod, treść).
pub fn http_get(addr: &str, path: &str, token: Option<&str>, timeout: Duration) -> Result<(u16, String), String> {
    let sa = addr.to_socket_addrs().map_err(|e| format!("{addr}: {e}"))?.next().ok_or(format!("{addr}: brak adresu"))?;
    let mut s = TcpStream::connect_timeout(&sa, timeout).map_err(|e| format!("{addr}: {e}"))?;
    s.set_read_timeout(Some(timeout)).ok();
    let auth = token.map(|t| format!("Authorization: Bearer {t}\r\n")).unwrap_or_default();
    write!(s, "GET {path} HTTP/1.1\r\nHost: {addr}\r\n{auth}Connection: close\r\n\r\n").map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    s.take(1024 * 1024).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&buf);
    let (head, body) = text.split_once("\r\n\r\n").ok_or("niepełna odpowiedź HTTP")?;
    let code = head.split_whitespace().nth(1).and_then(|c| c.parse().ok()).ok_or("to nie jest odpowiedź HTTP")?;
    Ok((code, body.to_string()))
}

/// Adres działającego serwera VizRS, który trzyma katalog danych (blokada + zgodny `instance` w `/health`).
pub fn running_server(data: &Path, token: Option<&str>) -> Result<Option<String>, String> {
    let Ok(text) = std::fs::read_to_string(data.join(LOCK_FILE)) else { return Ok(None) };
    let Ok(lock) = serde_json::from_str::<serde_json::Value>(&text) else { return Ok(None) };
    let (Some(addr), Some(inst)) = (lock["addr"].as_str(), lock["instance"].as_str()) else { return Ok(None) };
    match http_get(addr, "/health", token, Duration::from_millis(1500)) {
        Ok((200, body)) => {
            let same = serde_json::from_str::<serde_json::Value>(&body).map(|h| h["instance"].as_str() == Some(inst)).unwrap_or(false);
            Ok(same.then(|| addr.to_string()))
        }
        Ok((401 | 403, _)) => Err(format!("serwer VizRS na {addr} wymaga tokenu – podaj --token albo ustaw VIZRS_TOKEN")),
        _ => Ok(None),
    }
}

/// Plik wykonywalny `vizrs`: wskazany, obok powłoki, w `PATH`.
pub fn find_vizrs(over: Option<&str>) -> Option<PathBuf> {
    let name = if cfg!(windows) { "vizrs.exe" } else { "vizrs" };
    if let Some(o) = over.filter(|o| !o.is_empty()) {
        return Some(PathBuf::from(o));
    }
    let sibling = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join(name)));
    if let Some(s) = sibling.filter(|s| s.is_file()) {
        return Some(s);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(name)).find(|p| p.is_file())
}

/// Serwer uruchomiony przez powłokę.
pub struct Spawned {
    pub addr: String,
    /// Plik z komunikatami serwera (stdout i stderr).
    pub log: PathBuf,
    // proces potomny nie jest zabijany przy zamknięciu okna: kończy się sam (--idle-exit),
    // gdy nikt z niego nie korzysta – zamknięcie okna nie zdejmuje grafiki z OBS
    _child: Child,
}

/// Uruchamia `vizrs serve` i czeka, aż odpowie. Błąd zawiera komunikat serwera (np. zajęty port).
pub fn spawn(vizrs: &Path, data: &Path, port: u16, token: Option<&str>) -> Result<Spawned, String> {
    // komunikaty serwera do pliku, nie do potoku: serwer żyje dłużej niż okno, a pisanie do
    // zamkniętego potoku kończyłoby się błędem w serwerze
    let log = std::env::temp_dir().join(format!("vizrs-shell-serwer-{}.log", std::process::id()));
    let log_file = std::fs::File::create(&log).map_err(|e| format!("{}: {e}", log.display()))?;
    let mut cmd = Command::new(vizrs);
    cmd.arg("serve")
        .arg("--data")
        .arg(data)
        .arg("--port")
        .arg(port.to_string())
        .arg("--idle-exit")
        .arg(IDLE_EXIT_S.to_string())
        .stdin(Stdio::null())
        .stdout(log_file.try_clone().map_err(|e| e.to_string())?)
        .stderr(log_file);
    if let Some(t) = token {
        // token przez zmienną – nie widać go w liście procesów
        cmd.env("VIZRS_TOKEN", t);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().map_err(|e| format!("nie mogę uruchomić {}: {e}", vizrs.display()))?;
    let errors = || {
        let t = std::fs::read_to_string(&log).unwrap_or_default();
        // ramka z adresami to nie błąd – zostaw tylko komunikaty
        t.lines().filter(|l| !l.starts_with(['┌', '│', '└'])).collect::<Vec<_>>().join("\n").trim().to_string()
    };

    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(15) {
        if let Ok(Some(status)) = child.try_wait() {
            let msg = errors();
            return Err(if msg.is_empty() { format!("serwer VizRS zakończył się ({status})") } else { msg });
        }
        if let Ok(Some(addr)) = running_server(data, token) {
            return Ok(Spawned { addr, log, _child: child });
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Err("serwer VizRS nie odpowiedział w ciągu 15 s".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_lock_means_no_server() {
        let d = std::env::temp_dir().join(format!("vizrs-shell-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(running_server(&d, None).unwrap(), None);
        std::fs::write(d.join(LOCK_FILE), r#"{"pid":1,"addr":"127.0.0.1:1","instance":"x"}"#).unwrap();
        assert_eq!(running_server(&d, None).unwrap(), None, "martwa blokada");
        std::fs::write(d.join(LOCK_FILE), "śmieci").unwrap();
        assert_eq!(running_server(&d, None).unwrap(), None);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn explicit_vizrs_path_wins() {
        assert_eq!(find_vizrs(Some("/opt/x/vizrs")), Some(PathBuf::from("/opt/x/vizrs")));
    }
}
