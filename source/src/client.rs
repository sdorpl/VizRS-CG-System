//! Minimalny klient HTTP/1.1 (GET) do rozmowy z innym procesem VizRS na tej samej maszynie:
//! `--healthcheck`, sprawdzenie blokady katalogu danych. Bez TLS, bez przekierowań.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Największa akceptowana odpowiedź (ochrona przed obcym serwerem na tym porcie).
const MAX_RESPONSE: u64 = 1024 * 1024;

/// `GET` na `addr` (`host:port`). Zwraca (kod, treść).
pub fn get(addr: &str, path: &str, token: Option<&str>, timeout: Duration) -> Result<(u16, String), String> {
    let sa = addr.to_socket_addrs().map_err(|e| format!("{addr}: {e}"))?.next().ok_or(format!("{addr}: brak adresu"))?;
    let mut s = TcpStream::connect_timeout(&sa, timeout).map_err(|e| format!("{addr}: {e}"))?;
    s.set_read_timeout(Some(timeout)).ok();
    s.set_write_timeout(Some(timeout)).ok();
    let auth = token.map(|t| format!("Authorization: Bearer {t}\r\n")).unwrap_or_default();
    let req = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\n{auth}Connection: close\r\n\r\n");
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    s.take(MAX_RESPONSE).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    parse_response(&buf)
}

fn parse_response(buf: &[u8]) -> Result<(u16, String), String> {
    let text = String::from_utf8_lossy(buf);
    let (head, body) = text.split_once("\r\n\r\n").ok_or("niepełna odpowiedź HTTP")?;
    let status = head
        .lines()
        .next()
        .filter(|l| l.starts_with("HTTP/1."))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .ok_or("to nie jest odpowiedź HTTP")?;
    Ok((status, body.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_and_body() {
        assert_eq!(parse_response(b"HTTP/1.1 200 OK\r\nA: b\r\n\r\n{\"x\":1}").unwrap(), (200, "{\"x\":1}".to_string()));
        assert_eq!(parse_response(b"HTTP/1.1 401 Unauthorized\r\n\r\n").unwrap().0, 401);
        assert!(parse_response(b"SSH-2.0-OpenSSH\r\n").is_err());
        assert!(parse_response(b"garbage\r\n\r\nx").is_err());
    }

    #[test]
    fn refused_connection_is_error() {
        // port 1 na loopback praktycznie zawsze zamknięty
        assert!(get("127.0.0.1:1", "/health", None, Duration::from_millis(300)).is_err());
    }
}
