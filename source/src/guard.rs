//! Ochrona sterowania: token API oraz blokada żądań z obcych stron (CSRF).
//!
//! * Przeglądarka wysyła `Sec-Fetch-Site` / `Origin` – żądania z innych witryn są odrzucane,
//!   więc strona otwarta w przeglądarce operatora nie może wywołać TAKE/CLEAR/PUT show.
//! * Gdy ustawiono `--token`, wszystko (UI, `/events`, `/api`) wymaga tokenu: nagłówek
//!   `Authorization: Bearer`, `X-Token`, `?token=` albo ciasteczko ustawione po pierwszym wejściu z `?token=`.

use crate::http::Request;

pub enum Verdict {
    /// Przepuść.
    Allow,
    /// Przepuść i ustaw ciasteczko z tokenem (pierwsze wejście z `?token=`).
    AllowSetCookie(String),
    /// Odrzuć: (kod, komunikat).
    Deny(u16, &'static str),
}

/// Token może zawierać tylko bezpieczne znaki (trafia do nagłówka `Set-Cookie`).
pub fn token_valid(t: &str) -> bool {
    !t.is_empty() && t.len() <= 128 && t.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut d = (a.len() ^ b.len()) as u8;
    for i in 0..a.len().max(b.len()) {
        d |= a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0);
    }
    d == 0
}

fn is_api(req: &Request) -> bool {
    req.path == "/events" || req.path == "/api" || req.path.starts_with("/api/")
}

fn origin_host(o: &str) -> &str {
    o.split_once("://").map(|(_, r)| r).unwrap_or(o).split('/').next().unwrap_or("")
}

pub fn check(req: &Request, token: &Option<String>) -> Verdict {
    // 1. żądania z obcych witryn
    let site = req.header("sec-fetch-site");
    let navigate = req.header("sec-fetch-mode") == Some("navigate");
    if is_api(req) {
        if !matches!(site, None | Some("same-origin") | Some("none")) {
            return Verdict::Deny(403, "żądanie z obcej witryny");
        }
    } else if site == Some("cross-site") && !navigate {
        return Verdict::Deny(403, "żądanie z obcej witryny");
    }
    if let Some(o) = req.header("origin") {
        let host = req.header("host").unwrap_or("");
        if o == "null" || !origin_host(o).eq_ignore_ascii_case(host) {
            return Verdict::Deny(403, "niedozwolony Origin");
        }
    }
    // 2. token
    let Some(want) = token else { return Verdict::Allow };
    let bearer = req.header("authorization").and_then(|a| a.strip_prefix("Bearer "));
    let given_hdr = bearer.or_else(|| req.header("x-token"));
    if given_hdr.map(|g| ct_eq(g, want)).unwrap_or(false) {
        return Verdict::Allow;
    }
    if req.cookie("vizrs_token").map(|g| ct_eq(g, want)).unwrap_or(false) {
        return Verdict::Allow;
    }
    if req.q("token").map(|g| ct_eq(g, want)).unwrap_or(false) {
        // ciasteczko ustawiamy tylko przy wejściu na stronę; wywołania API z ?token= (Companion) go nie potrzebują
        return if is_api(req) { Verdict::Allow } else { Verdict::AllowSetCookie(want.clone()) };
    }
    Verdict::Deny(401, "wymagany token (?token=…, nagłówek Authorization: Bearer lub X-Token)")
}

/// Losowy ciąg szesnastkowy (`bytes` bajtów): token sesji okna, identyfikator instancji.
/// Unix: `/dev/urandom`; inaczej SipHash z kluczami losowanymi przez system (`RandomState`).
pub fn random_token(bytes: usize) -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut buf = vec![0u8; bytes];
    if !urandom(&mut buf) {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        for (i, chunk) in buf.chunks_mut(8).enumerate() {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u128(t);
            h.write_usize(i);
            h.write_u32(std::process::id());
            let v = h.finish().to_le_bytes();
            chunk.copy_from_slice(&v[..chunk.len()]);
        }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(unix)]
fn urandom(buf: &mut [u8]) -> bool {
    use std::io::Read;
    std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(buf)).is_ok()
}
#[cfg(not(unix))]
fn urandom(_: &mut [u8]) -> bool {
    false
}

pub fn cookie_header(token: &str) -> String {
    format!("vizrs_token={token}; Path=/; SameSite=Strict; HttpOnly; Max-Age=31536000")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(path: &str, hdr: &[(&str, &str)], q: &[(&str, &str)]) -> Request {
        Request {
            method: "GET".into(),
            path: path.into(),
            query: q.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
            headers: hdr.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
            body: vec![],
            peer: String::new(),
            raw_query: String::new(),
        }
    }
    fn code(v: Verdict) -> u16 {
        match v {
            Verdict::Allow | Verdict::AllowSetCookie(_) => 200,
            Verdict::Deny(c, _) => c,
        }
    }

    #[test]
    fn no_token_allows_plain_clients() {
        assert_eq!(code(check(&req("/api/clear", &[], &[]), &None)), 200); // curl / Companion
        assert_eq!(code(check(&req("/api/state", &[("sec-fetch-site", "same-origin")], &[]), &None)), 200);
    }

    #[test]
    fn cross_site_blocked() {
        assert_eq!(code(check(&req("/api/clear", &[("sec-fetch-site", "cross-site")], &[]), &None)), 403);
        assert_eq!(code(check(&req("/api/clear", &[("sec-fetch-site", "same-site")], &[]), &None)), 403);
        assert_eq!(code(check(&req("/api/show", &[("origin", "http://evil.example"), ("host", "localhost:8088")], &[]), &None)), 403);
        assert_eq!(code(check(&req("/api/show", &[("origin", "http://localhost:8088"), ("host", "localhost:8088")], &[]), &None)), 200);
        // nawigacja z linku na stronę UI jest dozwolona, na API nie
        let nav = [("sec-fetch-site", "cross-site"), ("sec-fetch-mode", "navigate")];
        assert_eq!(code(check(&req("/trio", &nav, &[]), &None)), 200);
        assert_eq!(code(check(&req("/api/clear", &nav, &[]), &None)), 403);
    }

    #[test]
    fn token_rules() {
        let t = Some("sekret-1".to_string());
        assert_eq!(code(check(&req("/api/state", &[], &[]), &t)), 401);
        assert_eq!(code(check(&req("/trio", &[], &[]), &t)), 401);
        assert_eq!(code(check(&req("/api/state", &[("authorization", "Bearer sekret-1")], &[]), &t)), 200);
        assert_eq!(code(check(&req("/api/state", &[("x-token", "zly")], &[]), &t)), 401);
        assert_eq!(code(check(&req("/api/state", &[("cookie", "a=b; vizrs_token=sekret-1")], &[]), &t)), 200);
        assert_eq!(code(check(&req("/api/take/1", &[], &[("token", "sekret-1")]), &t)), 200);
        assert!(matches!(check(&req("/trio", &[], &[("token", "sekret-1")]), &t), Verdict::AllowSetCookie(_)));
        assert_eq!(code(check(&req("/trio", &[], &[("token", "sekret-2")]), &t)), 401);
    }

    #[test]
    fn random_tokens_differ_and_are_valid() {
        let a = random_token(16);
        let b = random_token(16);
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(token_valid(&a));
        assert_eq!(random_token(3).len(), 6);
    }

    #[test]
    fn token_charset() {
        assert!(token_valid("abc-DEF_1.2"));
        assert!(!token_valid("a b"));
        assert!(!token_valid("a;b"));
        assert!(!token_valid(""));
    }
}
