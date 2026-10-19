//! Prosty serwer HTTP/1.1 na std::net (wątek na połączenie).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::json::Json;

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Adres IP klienta (do logu audytowego).
    pub peer: String,
    /// Nieprzetworzony fragment po `?` (do logu).
    pub raw_query: String,
}

impl Request {
    pub fn header(&self, k: &str) -> Option<&str> {
        self.headers.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
    /// Wartość ciasteczka o danej nazwie.
    pub fn cookie(&self, name: &str) -> Option<&str> {
        self.header("cookie")?.split(';').filter_map(|c| c.trim().split_once('=')).find(|(k, _)| *k == name).map(|(_, v)| v)
    }
    pub fn q(&self, k: &str) -> Option<&str> {
        self.query.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
    pub fn json(&self) -> Result<Json, String> {
        if self.body.is_empty() {
            return Ok(Json::Null);
        }
        let s = std::str::from_utf8(&self.body).map_err(|e| e.to_string())?;
        Json::parse(s)
    }
    /// Segmenty ścieżki, np. "/api/take/1001" -> ["api","take","1001"]
    pub fn segs(&self) -> Vec<&str> {
        self.path.split('/').filter(|s| !s.is_empty()).collect()
    }
}

pub struct Response {
    pub status: u16,
    pub ctype: String,
    pub body: Vec<u8>,
    pub extra: Vec<(String, String)>,
}

impl Response {
    pub fn new(status: u16, ctype: &str, body: impl Into<Vec<u8>>) -> Self {
        Response { status, ctype: ctype.into(), body: body.into(), extra: vec![] }
    }
    pub fn json(j: &Json) -> Self {
        Self::new(200, "application/json; charset=utf-8", j.dump())
    }
    pub fn ok() -> Self {
        Self::json(&Json::obj().with("ok", true))
    }
    pub fn err(status: u16, msg: &str) -> Self {
        Self::new(status, "application/json; charset=utf-8", Json::obj().with("error", msg).dump())
    }
    pub fn header(mut self, k: &str, v: &str) -> Self {
        self.extra.push((k.into(), v.into()));
        self
    }

    pub fn write_to(&self, s: &mut TcpStream) -> std::io::Result<()> {
        let reason = match self.status {
            200 => "OK",
            204 => "No Content",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            413 => "Payload Too Large",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "Error",
        };
        let mut head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\nConnection: close\r\n",
            self.status, reason, self.ctype, self.body.len()
        );
        for (k, v) in &self.extra {
            head.push_str(&format!("{}: {}\r\n", k, v));
        }
        head.push_str("\r\n");
        s.write_all(head.as_bytes())?;
        s.write_all(&self.body)?;
        s.flush()
    }
}

pub fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

const MAX_LINE: u64 = 16 * 1024;

/// Czyta linię nie dłuższą niż `MAX_LINE` (ochrona przed nieskończoną linią bez `\n`).
fn read_line_limited<R: BufRead>(r: &mut R, buf: &mut String) -> Result<usize, String> {
    let n = r.by_ref().take(MAX_LINE).read_line(buf).map_err(|e| e.to_string())?;
    if n as u64 == MAX_LINE && !buf.ends_with('\n') {
        return Err("za długa linia nagłówka".into());
    }
    Ok(n)
}

const MAX_BODY: usize = 64 * 1024 * 1024;

pub fn read_request(stream: &mut TcpStream) -> Result<Request, String> {
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    let mut r = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut line = String::new();
    read_line_limited(&mut r, &mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().ok_or("brak metody")?.to_uppercase();
    let target = parts.next().ok_or("brak ścieżki")?.to_string();
    let mut headers = Vec::new();
    loop {
        let mut h = String::new();
        let n = read_line_limited(&mut r, &mut h)?;
        if n == 0 || h == "\r\n" || h == "\n" {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
        if headers.len() > 100 {
            return Err("za dużo nagłówków".into());
        }
    }
    let len: usize = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    if len > MAX_BODY {
        return Err("413".into());
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body).map_err(|e| e.to_string())?;

    let (path, qs) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.clone(), String::new()),
    };
    let query = qs
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|kv| match kv.split_once('=') {
            Some((k, v)) => (url_decode(k), url_decode(v)),
            None => (url_decode(kv), String::new()),
        })
        .collect();
    let peer = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    Ok(Request { method, path: url_decode(&path), query, headers, body, peer, raw_query: qs })
}
