//! Logowanie do Trio i Artist kontami panelu VizRS CG Playout (opcja `--panel-login http://live:8090`).
//!
//! Czy wolno, decyduje przełącznik w panelu („logowanie w pakiecie”, `GET /api/sso`); bez opcji,
//! bez panelu albo z wyłączonym przełącznikiem serwer zna tylko tokeny – jak dotąd (Trio samo, np. dla OBS).
//! Panel sprawdza login i hasło (`POST /api/sso/verify`) i zakłada sesję swojego konta; jej identyfikator
//! zostaje w tym serwerze, a przeglądarka dostaje własną sesję VizRS (`vizrs_session`). Co 15 s
//! (przy użyciu) serwer pyta panel o sesję konta: konto usunięte albo przełącznik wyłączony = koniec sesji.
//! Panel nieosiągalny nie wylogowuje – sterowanie anteną działa dalej. Zalogowany w panelu pod tym samym
//! hostem (ciasteczko `vizrs_live` jedzie też na port VizRS) wchodzi bez ponownego logowania.
//! Sesje są w pamięci: restart serwera = ponowne logowanie (pod tym samym hostem – samo, przez panel).

use crate::client;
use crate::json::Json;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Ciasteczko sesji konta panelu w VizRS.
pub const COOKIE: &str = "vizrs_session";
const TIMEOUT: Duration = Duration::from_secs(3);
/// Sesja bez użycia tyle czasu wygasa.
const IDLE: Duration = Duration::from_secs(30 * 24 * 3600);
/// Co tyle (przy użyciu) sesja jest sprawdzana w panelu – wylogowanie w panelu zamyka Trio najpóźniej po tym czasie.
const RECHECK: Duration = Duration::from_secs(15);
const MAX_SESSIONS: usize = 256;
/// Stan przełącznika w panelu pamiętany tyle czasu (ekran logowania pyta przy każdym wejściu).
const ENABLED_TTL: Duration = Duration::from_secs(10);

struct Sess {
    user: String,
    /// identyfikator sesji konta w panelu (ciasteczko `vizrs_live` panelu)
    panel_sid: String,
    used: Instant,
    checked: Instant,
}

static SESSIONS: Mutex<Option<HashMap<String, Sess>>> = Mutex::new(None);
static ENABLED: Mutex<Option<(Instant, bool)>> = Mutex::new(None);

/// `http://host[:port]` → `host:port` (sieć stosu; bez TLS).
pub fn parse_url(v: &str) -> Result<String, String> {
    let rest = v.trim().strip_prefix("http://").ok_or("tylko http://host:port (sieć wewnętrzna stosu)")?;
    let host = rest.trim_end_matches('/');
    if host.is_empty() || host.contains(['/', '?', '#', '@', ' ']) {
        return Err(format!("zły adres panelu: {v}"));
    }
    Ok(if host.rsplit_once(':').is_some_and(|(_, p)| p.parse::<u16>().is_ok()) && !host.ends_with(']') {
        host.to_string()
    } else {
        format!("{host}:80")
    })
}

/// Identyfikator sesji panelu: 64 znaki szesnastkowe (niczego innego nie wkładamy do nagłówka Cookie).
fn sid_ok(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn result(body: &str) -> Option<Json> {
    Json::parse(body).ok()?.get("result").cloned()
}

/// Czy panel pozwala logować się swoimi kontami (krótko pamiętane; panel nieosiągalny = nie).
pub fn enabled(addr: &str) -> bool {
    let now = Instant::now();
    if let Some((t, on)) = *ENABLED.lock().unwrap_or_else(|e| e.into_inner()) {
        if now.duration_since(t) < ENABLED_TTL {
            return on;
        }
    }
    let on = client::request(addr, "GET", "/api/sso", &[], b"", TIMEOUT)
        .ok()
        .filter(|(code, _)| *code == 200)
        .and_then(|(_, b)| result(&b))
        .is_some_and(|r| matches!(r.get("enabled"), Some(Json::Bool(true))));
    *ENABLED.lock().unwrap_or_else(|e| e.into_inner()) = Some((now, on));
    on
}

/// Login i hasło → nowa sesja VizRS (identyfikator do ciasteczka) i nazwa konta. Błąd – komunikat dla ekranu logowania.
pub fn login(addr: &str, user: &str, password: &str) -> Result<(String, String), String> {
    let body = Json::obj().with("user", user).with("password", password).dump();
    let headers = [("Content-Type", "application/json"), ("X-VizRS-Live", "1")];
    let (code, b) = client::request(addr, "POST", "/api/sso/verify", &headers, body.as_bytes(), TIMEOUT)
        .map_err(|_| "panel VizRS CG Playout nie odpowiada – zaloguj się tokenem".to_string())?;
    match code {
        200 => {}
        401 => return Err("zły login albo hasło".into()),
        403 => return Err("panel nie pozwala logować się swoimi kontami do Trio".into()),
        429 => return Err("za dużo nieudanych logowań – odczekaj minutę".into()),
        c => return Err(format!("panel odpowiedział {c}")),
    }
    let r = result(&b).ok_or("zła odpowiedź panelu")?;
    let name = r.get("user").and_then(|u| u.str("name")).filter(|n| !n.is_empty()).ok_or("zła odpowiedź panelu")?;
    let sid = r.str("session").filter(|s| sid_ok(s)).ok_or("zła odpowiedź panelu")?;
    Ok((add(name, sid), name.to_string()))
}

/// Ciasteczko `vizrs_live` panelu (ten sam host) → sesja VizRS bez ponownego logowania.
pub fn from_panel_cookie(addr: &str, panel_sid: &str) -> Option<(String, String)> {
    if !sid_ok(panel_sid) {
        return None;
    }
    let name = check(addr, panel_sid).ok()??;
    Some((add(&name, panel_sid), name))
}

/// Sesja konta w panelu: `Ok(Some(konto))` ważna, `Ok(None)` nieważna albo przełącznik wyłączony, `Err` – panel nie odpowiada.
fn check(addr: &str, panel_sid: &str) -> Result<Option<String>, ()> {
    let cookie = format!("vizrs_live={panel_sid}");
    let (code, b) = client::request(addr, "GET", "/api/sso/session", &[("Cookie", &cookie)], b"", TIMEOUT).map_err(|_| ())?;
    match code {
        200 => Ok(result(&b).and_then(|r| r.get("user").and_then(|u| u.str("name")).map(String::from)).filter(|n| !n.is_empty())),
        401 | 403 => Ok(None),
        _ => Err(()),
    }
}

fn add(user: &str, panel_sid: &str) -> String {
    let id = crate::guard::random_token(32);
    let now = Instant::now();
    let mut g = SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
    let map = g.get_or_insert_with(HashMap::new);
    if map.len() >= MAX_SESSIONS {
        map.retain(|_, s| now.duration_since(s.used) < IDLE);
        if map.len() >= MAX_SESSIONS {
            if let Some(old) = map.iter().min_by_key(|(_, s)| s.used).map(|(k, _)| k.clone()) {
                map.remove(&old);
            }
        }
    }
    map.insert(id.clone(), Sess { user: user.into(), panel_sid: panel_sid.into(), used: now, checked: now });
    id
}

/// Ważna sesja VizRS → nazwa konta. Co [`RECHECK`] pyta panel (poza blokadą); panel nieosiągalny = sesja ważna.
pub fn session_user(addr: &str, id: &str) -> Option<String> {
    let now = Instant::now();
    let (user, panel_sid, due) = {
        let mut g = SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
        let map = g.as_mut()?;
        let s = map.get(id)?;
        if now.duration_since(s.used) >= IDLE {
            map.remove(id);
            return None;
        }
        (s.user.clone(), s.panel_sid.clone(), now.duration_since(s.checked) >= RECHECK)
    };
    if due {
        match check(addr, &panel_sid) {
            Ok(Some(_)) | Err(()) => {}
            Ok(None) => {
                end(id);
                return None;
            }
        }
    }
    let mut g = SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
    let s = g.as_mut()?.get_mut(id)?;
    s.used = now;
    if due {
        s.checked = now;
    }
    Some(user)
}

pub fn end(id: &str) {
    if let Some(m) = SESSIONS.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        m.remove(id);
    }
}

pub fn cookie_header(id: &str, max_age: u64) -> String {
    format!("{COOKIE}={id}; Path=/; SameSite=Strict; HttpOnly; Max-Age={max_age}")
}

/// Ważność ciasteczka sesji w sekundach (jak bezczynność sesji).
pub const COOKIE_AGE: u64 = 30 * 24 * 3600;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn urls() {
        assert_eq!(parse_url("http://live:8090").unwrap(), "live:8090");
        assert_eq!(parse_url("http://live:8090/").unwrap(), "live:8090");
        assert_eq!(parse_url("http://live").unwrap(), "live:80");
        assert!(parse_url("https://live:8090").is_err());
        assert!(parse_url("http://a/b").is_err());
        assert!(parse_url("http://u@live:1").is_err());
        assert!(parse_url("http://").is_err());
    }

    /// Udawany panel: odpowiada po kolei podanymi (kod, treść) i zapisuje żądania.
    fn fake_panel(replies: Vec<(u16, &'static str)>) -> (String, std::sync::mpsc::Receiver<String>) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for (code, body) in replies {
                let (mut s, _) = l.accept().unwrap();
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap_or(0);
                tx.send(String::from_utf8_lossy(&buf[..n]).into_owned()).ok();
                let r = format!("HTTP/1.1 {code} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                s.write_all(r.as_bytes()).ok();
            }
        });
        (addr, rx)
    }

    const SID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn login_check_and_revoke() {
        let ok = r#"{"ok":true,"result":{"user":{"name":"jan","role":"admin"},"session":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}}"#;
        let (addr, rx) = fake_panel(vec![(401, r#"{"ok":false}"#), (200, ok), (200, r#"{"ok":true,"result":{"user":{"name":"jan"}}}"#), (403, "{}")]);
        assert_eq!(login(&addr, "jan", "zle").unwrap_err(), "zły login albo hasło");
        let first = rx.recv().unwrap();
        assert!(first.starts_with("POST /api/sso/verify ") && first.contains("X-VizRS-Live: 1") && first.contains(r#""password":"zle""#));
        let (id, name) = login(&addr, "jan", "dobre").unwrap();
        rx.recv().unwrap();
        assert_eq!(name, "jan");
        assert_eq!(session_user(&addr, &id).as_deref(), Some("jan"), "świeża sesja – bez pytania panelu");
        // po czasie sprawdzenia: panel potwierdza, potem odmawia (konto usunięte / przełącznik wyłączony)
        let age = |id: &str| {
            let mut g = SESSIONS.lock().unwrap();
            g.as_mut().unwrap().get_mut(id).unwrap().checked -= RECHECK;
        };
        age(&id);
        assert_eq!(session_user(&addr, &id).as_deref(), Some("jan"));
        assert!(rx.recv().unwrap().contains(&format!("Cookie: vizrs_live={SID}")));
        age(&id);
        assert_eq!(session_user(&addr, &id), None, "panel odmówił – koniec sesji");
        assert_eq!(session_user(&addr, &id), None);
    }

    #[test]
    fn panel_down_keeps_session_and_rejects_bad_ids() {
        let id = add("ola", SID);
        {
            let mut g = SESSIONS.lock().unwrap();
            g.as_mut().unwrap().get_mut(&id).unwrap().checked -= RECHECK;
        }
        assert_eq!(session_user("127.0.0.1:1", &id).as_deref(), Some("ola"), "panel leży – sterowanie działa dalej");
        assert_eq!(session_user("127.0.0.1:1", "nie-ma"), None);
        assert!(from_panel_cookie("127.0.0.1:1", "x\r\nEvil: 1").is_none());
        assert!(login("127.0.0.1:1", "a", "b").is_err());
        assert!(!enabled("127.0.0.1:1"));
        end(&id);
        assert_eq!(session_user("127.0.0.1:1", &id), None);
    }
}
