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
    fn token_charset() {
        assert!(token_valid("abc-DEF_1.2"));
        assert!(!token_valid("a b"));
        assert!(!token_valid("a;b"));
        assert!(!token_valid(""));
    }
}
