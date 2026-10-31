//! Połączenie z serwerem VizRS jak strona `/output`: adres i token z URL wyjścia, GET plików (`/fonts`, `/assets`)
//! i strumień zdarzeń SSE `/events` z ponawianiem. Własny minimalny klient HTTP/1.1 (tylko `http://`).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Odpowiedź: kod, nagłówki (nazwy małymi literami) i treść do czytania.
type Response = (u16, Vec<(String, String)>, BufReader<TcpStream>);

/// Adres wyjścia VizRS (`http://host:port/output?token=…&layers=…&bus=…&w=…&h=…&fx=step`).
#[derive(Clone, Debug, PartialEq)]
pub struct OutputUrl {
    pub host: String,
    pub port: u16,
    pub token: Option<String>,
    pub query: Vec<(String, String)>,
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
                out.push(b'%');
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl OutputUrl {
    pub fn parse(url: &str) -> Result<OutputUrl, String> {
        let rest = url.strip_prefix("http://").ok_or("adres wyjścia musi zaczynać się od http://")?;
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let (host, port) = match authority.rsplit_once(':') {
            Some((h, p)) if !h.ends_with(']') || h.starts_with('[') => {
                (h.trim_matches(['[', ']']).to_string(), p.parse().map_err(|_| format!("zły port w {url}"))?)
            }
            _ => (authority.to_string(), 80),
        };
        if host.is_empty() {
            return Err(format!("brak hosta w {url}"));
        }
        let query: Vec<(String, String)> = path
            .split_once('?')
            .map(|(_, q)| {
                q.split('&')
                    .filter(|p| !p.is_empty())
                    .map(|p| {
                        let (k, v) = p.split_once('=').unwrap_or((p, ""));
                        (decode(k), decode(v))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let token = query.iter().find(|(k, _)| k == "token").map(|(_, v)| v.clone()).filter(|t| !t.is_empty());
        Ok(OutputUrl { host, port, token, query })
    }

    pub fn param(&self, key: &str) -> Option<&str> {
        self.query.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// Ścieżka z tokenem (`?token=` – jak pierwsze wejście strony wyjścia).
    pub fn path(&self, path: &str) -> String {
        match &self.token {
            Some(t) => format!("{path}{}token={}", if path.contains('?') { '&' } else { '?' }, crate::res::encode_component(t)),
            None => path.to_string(),
        }
    }

    fn connect(&self, timeout: Duration) -> Result<TcpStream, String> {
        let addrs = (self.host.as_str(), self.port).to_socket_addrs().map_err(|e| format!("{}:{}: {e}", self.host, self.port))?;
        let mut last = String::from("brak adresu");
        for a in addrs {
            match TcpStream::connect_timeout(&a, timeout) {
                Ok(s) => {
                    let _ = s.set_nodelay(true);
                    return Ok(s);
                }
                Err(e) => last = format!("{a}: {e}"),
            }
        }
        Err(last)
    }

    fn request(&self, path: &str, timeout: Duration) -> Result<Response, String> {
        let mut s = self.connect(timeout)?;
        s.set_read_timeout(Some(timeout)).map_err(|e| e.to_string())?;
        s.set_write_timeout(Some(timeout)).map_err(|e| e.to_string())?;
        let req = format!(
            "GET {} HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: vizrs-render\r\nAccept: */*\r\nConnection: close\r\n\r\n",
            self.path(path),
            self.host,
            self.port
        );
        s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
        let mut r = BufReader::new(s);
        let mut line = String::new();
        r.read_line(&mut line).map_err(|e| e.to_string())?;
        let code: u16 =
            line.split_whitespace().nth(1).and_then(|c| c.parse().ok()).ok_or_else(|| format!("zła odpowiedź: {}", line.trim()))?;
        let mut headers = Vec::new();
        loop {
            line.clear();
            r.read_line(&mut line).map_err(|e| e.to_string())?;
            let l = line.trim_end();
            if l.is_empty() {
                break;
            }
            if let Some((k, v)) = l.split_once(':') {
                headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
            }
        }
        Ok((code, headers, r))
    }

    /// GET pliku (najwyżej 64 MiB).
    pub fn get(&self, path: &str) -> Result<Vec<u8>, String> {
        let (code, headers, mut r) = self.request(path, Duration::from_secs(15))?;
        if code != 200 {
            return Err(format!("GET {path}: HTTP {code}"));
        }
        let header = |k: &str| headers.iter().find(|(h, _)| h == k).map(|(_, v)| v.as_str());
        let mut body = Vec::new();
        if header("transfer-encoding").is_some_and(|t| t.eq_ignore_ascii_case("chunked")) {
            loop {
                let mut size = String::new();
                r.read_line(&mut size).map_err(|e| e.to_string())?;
                let n = usize::from_str_radix(size.trim().split(';').next().unwrap_or(""), 16).map_err(|_| "zły fragment".to_string())?;
                if n == 0 {
                    break;
                }
                if body.len() + n > 64 << 20 {
                    return Err("za duży plik".into());
                }
                let at = body.len();
                body.resize(at + n, 0);
                r.read_exact(&mut body[at..]).map_err(|e| e.to_string())?;
                let mut crlf = [0u8; 2];
                r.read_exact(&mut crlf).map_err(|e| e.to_string())?;
            }
        } else if let Some(len) = header("content-length").and_then(|l| l.parse::<usize>().ok()) {
            if len > 64 << 20 {
                return Err("za duży plik".into());
            }
            body.resize(len, 0);
            r.read_exact(&mut body).map_err(|e| e.to_string())?;
        } else {
            r.take(64 << 20).read_to_end(&mut body).map_err(|e| e.to_string())?;
        }
        Ok(body)
    }

    /// Strumień zdarzeń: wywołuje `on_event` z każdym `data:` (JSON jako tekst) do zerwania połączenia.
    pub fn events(&self, on_open: &mut dyn FnMut(), on_event: &mut dyn FnMut(&str)) -> Result<(), String> {
        // serwer wysyła „: ping” co 10 s – 30 s ciszy to zerwane połączenie
        let (code, _, mut r) = self.request("/events", Duration::from_secs(30))?;
        if code != 200 {
            return Err(format!("/events: HTTP {code}"));
        }
        on_open();
        let mut data = String::new();
        let mut line = String::new();
        loop {
            line.clear();
            let n = r.read_line(&mut line).map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("serwer zamknął strumień zdarzeń".into());
            }
            let l = line.trim_end_matches(['\r', '\n']);
            if l.is_empty() {
                if !data.is_empty() {
                    on_event(&data);
                    data.clear();
                }
            } else if let Some(d) = l.strip_prefix("data:") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(d.strip_prefix(' ').unwrap_or(d));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_url_parts() {
        let u = OutputUrl::parse("http://vizrs:8088/output?token=a%2Bb&layers=main,front&w=1280").unwrap();
        assert_eq!((u.host.as_str(), u.port), ("vizrs", 8088));
        assert_eq!(u.token.as_deref(), Some("a+b"));
        assert_eq!(u.param("layers"), Some("main,front"));
        assert_eq!(u.path("/events"), "/events?token=a%2Bb");
        assert_eq!(OutputUrl::parse("http://127.0.0.1/output").unwrap().port, 80);
        assert!(OutputUrl::parse("https://x/").is_err());
    }
}
