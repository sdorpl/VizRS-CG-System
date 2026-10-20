//! Blokada katalogu danych: jeden serwer na jeden `show.json`.
//!
//! Plik `<data>/.vizrs.lock` (JSON: pid, adres, identyfikator instancji) powstaje atomowo (`create_new`).
//! Gdy już istnieje, sprawdzamy, czy jego właściciel żyje: `GET /health` pod zapisanym adresem musi
//! zwrócić ten sam identyfikator instancji. Martwy plik (awaria, `kill -9`) jest zastępowany.
//! Brak możliwości zapisu pliku (np. katalog tylko do odczytu) nie zatrzymuje serwera – tylko ostrzeżenie.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::client;
use crate::json::Json;

pub const FILE: &str = ".vizrs.lock";

/// Trzymana blokada; usuwa plik przy zwolnieniu (tylko jeśli nadal jest nasz).
pub struct DataLock {
    path: Option<PathBuf>,
    pub instance: String,
}

/// Kto trzyma katalog danych.
#[derive(Debug, Clone)]
pub struct Owner {
    pub pid: u32,
    pub addr: String,
}

impl DataLock {
    /// Usuwa plik blokady, jeśli nadal jest nasz (można wołać wielokrotnie, np. przy zatrzymaniu sygnałem).
    pub fn release(&self) {
        if let Some(p) = &self.path {
            if read(p).map(|j| j.str("instance") == Some(self.instance.as_str())).unwrap_or(false) {
                std::fs::remove_file(p).ok();
            }
        }
    }
}

impl Drop for DataLock {
    fn drop(&mut self) {
        self.release();
    }
}

fn read(p: &Path) -> Option<Json> {
    Json::parse(&std::fs::read_to_string(p).ok()?).ok().filter(|j| j.str("instance").is_some())
}

/// Czy proces o tym pid na pewno nie istnieje (tylko Linux potrafi to sprawdzić bez zależności).
fn pid_dead(pid: u32) -> bool {
    if cfg!(target_os = "linux") && pid != 0 {
        return !Path::new(&format!("/proc/{pid}")).exists();
    }
    false
}

/// Czy właściciel pliku blokady nadal działa.
fn alive(lock: &Json, token: Option<&str>) -> bool {
    let pid = lock.num("pid").unwrap_or(0.0) as u32;
    if pid == std::process::id() || pid_dead(pid) {
        return false;
    }
    let (Some(addr), Some(inst)) = (lock.str("addr"), lock.str("instance")) else { return false };
    match client::get(addr, "/health", token, Duration::from_millis(1500)) {
        Ok((200, body)) => Json::parse(&body).ok().and_then(|h| h.str("instance").map(|i| i == inst)).unwrap_or(false),
        // serwer pod tym adresem odpowiada, ale wymaga innego tokenu – nie ryzykujemy dwóch zapisujących
        Ok((401 | 403, _)) => true,
        _ => false,
    }
}

/// Zakłada blokadę dla serwera słuchającego pod `addr`. Błąd = katalog zajęty przez działający serwer.
pub fn acquire(data: &Path, addr: &str, token: Option<&str>) -> Result<DataLock, Owner> {
    let instance = crate::guard::random_token(12);
    if std::fs::create_dir_all(data).is_err() {
        return Ok(DataLock { path: None, instance });
    }
    let path = data.join(FILE);
    let body = Json::obj()
        .with("pid", std::process::id() as f64)
        .with("addr", addr)
        .with("instance", instance.as_str())
        .with("version", env!("CARGO_PKG_VERSION"))
        .pretty();
    for _ in 0..3 {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                if f.write_all(body.as_bytes()).and_then(|_| f.sync_all()).is_err() {
                    eprintln!("[vizrs] nie mogę zapisać {} – działam bez blokady katalogu danych", path.display());
                    return Ok(DataLock { path: None, instance });
                }
                return Ok(DataLock { path: Some(path), instance });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // plik mógł dopiero powstać – daj właścicielowi chwilę na zapis treści
                let lock = read(&path).or_else(|| {
                    std::thread::sleep(Duration::from_millis(200));
                    read(&path)
                });
                match lock {
                    Some(l) if alive(&l, token) => {
                        return Err(Owner { pid: l.num("pid").unwrap_or(0.0) as u32, addr: l.str("addr").unwrap_or("").to_string() });
                    }
                    _ => {
                        std::fs::remove_file(&path).ok();
                    }
                }
            }
            Err(e) => {
                eprintln!("[vizrs] nie mogę założyć {}: {e} – działam bez blokady katalogu danych", path.display());
                return Ok(DataLock { path: None, instance });
            }
        }
    }
    Ok(DataLock { path: None, instance })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("vizrs-lock-{}-{name}", std::process::id()));
        std::fs::remove_dir_all(&d).ok();
        d
    }

    /// Udaje serwer VizRS: na każde żądanie odpowiada `/health` z podanym identyfikatorem.
    fn fake_server(instance: String, status: u16) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let mut s = s;
                let mut buf = [0u8; 1024];
                let _ = std::io::Read::read(&mut s, &mut buf);
                let body = format!("{{\"status\":\"ok\",\"instance\":\"{instance}\"}}");
                let _ = write!(s, "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        addr
    }

    #[test]
    fn fresh_dir_lock_and_release() {
        let d = dir("fresh");
        let l = acquire(&d, "127.0.0.1:1", None).unwrap();
        assert!(d.join(FILE).exists());
        drop(l);
        assert!(!d.join(FILE).exists());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn stale_lock_is_replaced() {
        let d = dir("stale");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(FILE), r#"{"pid":1,"addr":"127.0.0.1:1","instance":"stary"}"#).unwrap();
        let l = acquire(&d, "127.0.0.1:2", None).unwrap();
        assert!(std::fs::read_to_string(d.join(FILE)).unwrap().contains(&l.instance));
        drop(l);
        // śmieci w pliku też są martwą blokadą
        std::fs::write(d.join(FILE), "xx").unwrap();
        assert!(acquire(&d, "127.0.0.1:2", None).is_ok());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn live_owner_blocks_second_server() {
        let d = dir("live");
        std::fs::create_dir_all(&d).unwrap();
        let addr = fake_server("inst-A".into(), 200);
        // pid 0 = nieznany (nie sprawdzamy /proc), rozstrzyga odpowiedź /health
        std::fs::write(d.join(FILE), format!(r#"{{"pid":0,"addr":"{addr}","instance":"inst-A"}}"#)).unwrap();
        let o = acquire(&d, "127.0.0.1:2", None).err().expect("katalog zajęty");
        assert_eq!(o.addr, addr);
        // pod tym adresem inny serwer (inna instancja) → plik martwy
        std::fs::write(d.join(FILE), format!(r#"{{"pid":0,"addr":"{addr}","instance":"inst-B"}}"#)).unwrap();
        assert!(acquire(&d, "127.0.0.1:2", None).is_ok());
        // serwer wymaga innego tokenu → traktujemy jako żywy
        let addr = fake_server("x".into(), 401);
        std::fs::write(d.join(FILE), format!(r#"{{"pid":0,"addr":"{addr}","instance":"inst-C"}}"#)).unwrap();
        assert!(acquire(&d, "127.0.0.1:2", None).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn foreign_lock_not_removed_on_drop() {
        let d = dir("foreign");
        let l = acquire(&d, "127.0.0.1:1", None).unwrap();
        std::fs::write(d.join(FILE), r#"{"pid":0,"addr":"x","instance":"cudzy"}"#).unwrap();
        drop(l);
        assert!(d.join(FILE).exists(), "nie usuwamy cudzej blokady");
        std::fs::remove_dir_all(d).ok();
    }
}
