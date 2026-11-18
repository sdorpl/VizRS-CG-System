//! Minimalny klient HTTP/1.1 do rozmowy z innym procesem VizRS na tej samej maszynie albo w sieci stosu:
//! `--healthcheck`, sprawdzenie blokady katalogu danych, logowanie kontami panelu (`panel_login`).
//! Bez TLS, bez przekierowań.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Największa akceptowana odpowiedź (ochrona przed obcym serwerem na tym porcie).
const MAX_RESPONSE: u64 = 1024 * 1024;

/// `GET` na `addr` (`host:port`). Zwraca (kod, treść).
pub fn get(addr: &str, path: &str, token: Option<&str>, timeout: Duration) -> Result<(u16, String), String> {
    let auth = token.map(|t| format!("Bearer {t}"));
    let headers: Vec<(&str, &str)> = auth.as_deref().map(|a| ("Authorization", a)).into_iter().collect();
    request(addr, "GET", path, &headers, b"", timeout)
}

/// Żądanie `method` z nagłówkami i treścią (wartości nagłówków bez CR/LF – sprawdza wołający).
pub fn request(addr: &str, method: &str, path: &str, headers: &[(&str, &str)], body: &[u8], timeout: Duration) -> Result<(u16, String), String> {
    let sa = addr.to_socket_addrs().map_err(|e| format!("{addr}: {e}"))?.next().ok_or(format!("{addr}: brak adresu"))?;
    let mut s = TcpStream::connect_timeout(&sa, timeout).map_err(|e| format!("{addr}: {e}"))?;
    s.set_read_timeout(Some(timeout)).ok();
    s.set_write_timeout(Some(timeout)).ok();
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\n");
    for (k, v) in headers {
        if v.contains(['\r', '\n']) {
            return Err(format!("nagłówek {k}: znak nowej linii"));
        }
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    if !body.is_empty() || method != "GET" {
        req.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    req.push_str("Connection: close\r\n\r\n");
    let mut out = req.into_bytes();
    out.extend_from_slice(body);
    s.write_all(&out).map_err(|e| e.to_string())?;
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
    let chunked = head.lines().any(|l| {
        l.split_once(':').is_some_and(|(k, v)| k.trim().eq_ignore_ascii_case("transfer-encoding") && v.to_ascii_lowercase().contains("chunked"))
    });
    Ok((status, if chunked { dechunk(body)? } else { body.to_string() }))
}

/// Treść `Transfer-Encoding: chunked` (rozmiar szesnastkowo, CRLF, dane, CRLF … aż do 0).
fn dechunk(mut rest: &str) -> Result<String, String> {
    let mut out = String::new();
    loop {
        let (line, after) = rest.split_once("\r\n").ok_or("niepełna odpowiedź chunked")?;
        let size = usize::from_str_radix(line.split(';').next().unwrap_or("").trim(), 16).map_err(|_| "zły rozmiar kawałka")?;
        if size == 0 {
            return Ok(out);
        }
        let chunk = after.get(..size).ok_or("niepełny kawałek")?;
        out.push_str(chunk);
        rest = after.get(size..).and_then(|r| r.strip_prefix("\r\n")).ok_or("niepełny kawałek")?;
    }
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
        let chunked = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n{\"a\":\r\n2\r\n1}\r\n0\r\n\r\n";
        assert_eq!(parse_response(chunked).unwrap().1, "{\"a\":1}");
        assert!(parse_response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n").is_err());
    }

    #[test]
    fn refused_connection_is_error() {
        // port 1 na loopback praktycznie zawsze zamknięty
        assert!(get("127.0.0.1:1", "/health", None, Duration::from_millis(300)).is_err());
    }
}
