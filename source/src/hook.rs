//! Webhook (F2.9): zdarzenia anteny (`onAir`, `offAir`, `error`) wysyłane jako JSON `POST` na adres `http://host:port/ścieżka`.
//! Tylko zwykły HTTP (bez TLS – bez zależności); do HTTPS użyj lokalnego proxy. Błędy dostarczenia nie wpływają na antenę.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use crate::engine::{now_ms, Engine};
use crate::json::Json;

pub struct Target {
    pub host: String,
    pub port: u16,
    pub path: String,
}

pub fn parse_url(u: &str) -> Result<Target, String> {
    let rest = u.strip_prefix("http://").ok_or("webhook: obsługiwany jest tylko http:// (bez TLS)")?;
    let (hostport, path) = rest.split_once('/').map(|(h, p)| (h, format!("/{p}"))).unwrap_or((rest, "/".to_string()));
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().map_err(|_| "webhook: zły port")?),
        None => (hostport.to_string(), 80),
    };
    if host.is_empty() {
        return Err("webhook: pusty host".into());
    }
    Ok(Target { host, port, path })
}

/// Zamienia zdarzenie silnika na komunikat webhooka (None = pomiń).
pub fn translate(ev: &Json) -> Option<Json> {
    let (name, layer) = match ev.str("t")? {
        "take" if ev.str("bus") != Some("preview") => ("onAir", ev.str("layer")),
        "out" => ("offAir", ev.str("layer")),
        "clear" if ev.str("bus") != Some("preview") => ("offAir", Some("*")),
        "error" => ("error", ev.str("scene")),
        _ => return None,
    };
    let mut m = Json::obj().with("event", name).with("ts", now_ms() as f64);
    if let Some(l) = layer {
        m.set("layer", l.into());
    }
    if let Some(p) = ev.get("page") {
        m.set("page", p.clone());
    }
    if let Some(msg) = ev.str("message") {
        m.set("message", msg.into());
    }
    Some(m)
}

fn post(t: &Target, body: &str) -> Result<(), String> {
    let addr = (t.host.as_str(), t.port).to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or("brak adresu")?;
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(Duration::from_secs(3))).ok();
    s.set_write_timeout(Some(Duration::from_secs(3))).ok();
    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        t.path,
        t.host,
        t.port,
        body.len(),
        body
    );
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut sink = [0u8; 256];
    let _ = s.read(&mut sink); // odpowiedź pomijamy; błąd odczytu nie ma znaczenia
    Ok(())
}

pub fn spawn(engine: Arc<Engine>, target: Target) {
    std::thread::spawn(move || {
        let rx = engine.subscribe();
        // pierwsze dwa komunikaty to migawki (program i preview) – pomijamy
        while let Ok(raw) = rx.recv() {
            let Ok(ev) = Json::parse(&raw) else { continue };
            if let Some(m) = translate(&ev) {
                if let Err(e) = post(&target, &m.dump()) {
                    eprintln!("[vizrs] webhook: {e}");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_parsing() {
        let t = parse_url("http://localhost:9000/hook").unwrap();
        assert_eq!((t.host.as_str(), t.port, t.path.as_str()), ("localhost", 9000, "/hook"));
        assert_eq!(parse_url("http://example.com").unwrap().port, 80);
        assert!(parse_url("https://x").is_err());
        assert!(parse_url("http://:80/").is_err());
    }

    #[test]
    fn event_translation() {
        let ev = Json::parse(r#"{"t":"take","layer":"main","page":{"number":1001,"name":"A"}}"#).unwrap();
        let m = translate(&ev).unwrap();
        assert_eq!(m.str("event"), Some("onAir"));
        assert_eq!(m.str("layer"), Some("main"));
        assert!(translate(&Json::parse(r#"{"t":"take","bus":"preview","layer":"main"}"#).unwrap()).is_none());
        assert_eq!(translate(&Json::parse(r#"{"t":"clear"}"#).unwrap()).unwrap().str("event"), Some("offAir"));
        assert!(translate(&Json::parse(r#"{"t":"state"}"#).unwrap()).is_none());
    }

    #[test]
    fn delivers_to_local_server() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            let mut buf = vec![0u8; 2048];
            let n = c.read(&mut buf).unwrap();
            String::from_utf8_lossy(&buf[..n]).to_string()
        });
        post(&Target { host: "127.0.0.1".into(), port, path: "/h".into() }, r#"{"event":"onAir"}"#).unwrap();
        let got = h.join().unwrap();
        assert!(got.starts_with("POST /h HTTP/1.1") && got.contains(r#"{"event":"onAir"}"#));
    }
}
