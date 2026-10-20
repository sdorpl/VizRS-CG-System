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

/// Tokeny serwera: operator (pełne sterowanie) i opcjonalny token wyjścia (tylko odczyt – OBS, vMix).
#[derive(Default, Clone)]
pub struct Tokens {
    pub operator: Option<String>,
    pub output: Option<String>,
}

#[derive(PartialEq, PartialOrd, Clone, Copy)]
enum Role {
    None,
    Output,
    Operator,
}

impl Tokens {
    fn role(&self, given: &str) -> Role {
        if self.operator.as_deref().map(|w| ct_eq(given, w)).unwrap_or(false) {
            Role::Operator
        } else if self.output.as_deref().map(|w| ct_eq(given, w)).unwrap_or(false) {
            Role::Output
        } else {
            Role::None
        }
    }
}

/// Co wolno z tokenem wyjścia: strona `/output`, jej skrypty, fonty, grafiki, strumień zdarzeń
/// i zgłaszanie błędów scen. Żadnych zmian show ani komend playoutu.
fn output_allowed(req: &Request) -> bool {
    let p = req.path.as_str();
    match req.method.as_str() {
        "GET" => {
            matches!(p, "/output" | "/output.html" | "/events" | "/health" | "/favicon.ico" | "/api/assets")
                || p.starts_with("/fonts/")
                || p.starts_with("/assets/")
                || (p.matches('/').count() == 1 && (p.ends_with(".js") || p.ends_with(".css")))
        }
        "POST" => p == "/api/report",
        _ => false,
    }
}

pub fn check(req: &Request, tokens: &Tokens) -> Verdict {
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
    // 2. token: najsilniejsza rola spośród nagłówka, ciasteczka i ?token=
    if tokens.operator.is_none() {
        return Verdict::Allow;
    }
    let bearer = req.header("authorization").and_then(|a| a.strip_prefix("Bearer "));
    let hdr = bearer.or_else(|| req.header("x-token")).map(|g| tokens.role(g)).unwrap_or(Role::None);
    let cookie = req.cookie("vizrs_token").map(|g| tokens.role(g)).unwrap_or(Role::None);
    let query = req.q("token").map(|g| tokens.role(g)).unwrap_or(Role::None);
    let role = [hdr, cookie, query].into_iter().fold(Role::None, |a, b| if b > a { b } else { a });
    if role == Role::None {
        return Verdict::Deny(401, "wymagany token (?token=…, nagłówek Authorization: Bearer lub X-Token)");
    }
    if role == Role::Output && !output_allowed(req) {
        return Verdict::Deny(403, "token wyjścia pozwala tylko na odczyt (/output)");
    }
    // ciasteczko ustawiamy tylko przy wejściu na stronę z ?token=, gdy to on daje najwyższą rolę;
    // wywołania API z ?token= (Companion) go nie potrzebują
    if query == role && !is_api(req) && hdr != role && cookie != role {
        if let Some(t) = req.q("token") {
            return Verdict::AllowSetCookie(t.to_string());
        }
    }
    Verdict::Allow
}

// ---------------- ograniczanie prób zgadywania tokenu ----------------

/// Po tylu nieudanych próbach z jednego adresu odpowiedź na kolejne jest opóźniana.
const FREE_FAILURES: u32 = 5;
/// Okno, po którym licznik nieudanych prób adresu się zeruje.
const FAILURE_WINDOW: std::time::Duration = std::time::Duration::from_secs(600);
/// Najwyżej tyle adresów pamiętamy (ochrona pamięci przy ataku z wielu adresów).
const MAX_TRACKED: usize = 10_000;

static FAILURES: std::sync::Mutex<Option<std::collections::HashMap<String, (u32, std::time::Instant)>>> = std::sync::Mutex::new(None);

/// Zapisuje nieudaną próbę z adresu `ip`; zwraca, o ile opóźnić odpowiedź 401.
/// Poprawny token nadal działa od razu (operator za tym samym proxy nie jest blokowany).
pub fn note_failure(ip: &str) -> std::time::Duration {
    let mut g = FAILURES.lock().unwrap_or_else(|e| e.into_inner());
    let map = g.get_or_insert_with(Default::default);
    let now = std::time::Instant::now();
    if map.len() >= MAX_TRACKED {
        map.retain(|_, (_, t)| now.duration_since(*t) < FAILURE_WINDOW);
        if map.len() >= MAX_TRACKED {
            map.clear();
        }
    }
    let e = map.entry(ip.to_string()).or_insert((0, now));
    if now.duration_since(e.1) >= FAILURE_WINDOW {
        *e = (0, now);
    }
    e.0 += 1;
    e.1 = now;
    let over = e.0.saturating_sub(FREE_FAILURES);
    std::time::Duration::from_millis((over as u64 * 250).min(3000))
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

    fn op(t: &str) -> Tokens {
        Tokens { operator: Some(t.into()), output: None }
    }

    #[test]
    fn no_token_allows_plain_clients() {
        assert_eq!(code(check(&req("/api/clear", &[], &[]), &Tokens::default())), 200); // curl / Companion
        assert_eq!(code(check(&req("/api/state", &[("sec-fetch-site", "same-origin")], &[]), &Tokens::default())), 200);
    }

    #[test]
    fn cross_site_blocked() {
        assert_eq!(code(check(&req("/api/clear", &[("sec-fetch-site", "cross-site")], &[]), &Tokens::default())), 403);
        assert_eq!(code(check(&req("/api/clear", &[("sec-fetch-site", "same-site")], &[]), &Tokens::default())), 403);
        assert_eq!(code(check(&req("/api/show", &[("origin", "http://evil.example"), ("host", "localhost:8088")], &[]), &Tokens::default())), 403);
        assert_eq!(code(check(&req("/api/show", &[("origin", "http://localhost:8088"), ("host", "localhost:8088")], &[]), &Tokens::default())), 200);
        // nawigacja z linku na stronę UI jest dozwolona, na API nie
        let nav = [("sec-fetch-site", "cross-site"), ("sec-fetch-mode", "navigate")];
        assert_eq!(code(check(&req("/trio", &nav, &[]), &Tokens::default())), 200);
        assert_eq!(code(check(&req("/api/clear", &nav, &[]), &Tokens::default())), 403);
    }

    #[test]
    fn token_rules() {
        let t = op("sekret-1");
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
    fn output_token_is_read_only() {
        let t = Tokens { operator: Some("op-1".into()), output: Some("out-1".into()) };
        let q = [("token", "out-1")];
        // strona wyjścia i to, czego potrzebuje
        assert!(matches!(check(&req("/output", &[], &q), &t), Verdict::AllowSetCookie(ref c) if c == "out-1"));
        let ck = [("cookie", "vizrs_token=out-1")];
        for p in ["/output", "/renderer.js", "/ui.css", "/fonts/fonts.css", "/fonts/x.woff2", "/assets/a.png", "/events", "/api/assets", "/health"] {
            assert_eq!(code(check(&req(p, &ck, &[]), &t)), 200, "{p}");
        }
        let mut rep = req("/api/report", &ck, &[]);
        rep.method = "POST".into();
        assert_eq!(code(check(&rep, &t)), 200);
        // sterowanie i edycja – nie
        for p in ["/trio", "/artist", "/api/take/1001", "/api/clear", "/api/show", "/api/state", "/api/audit"] {
            assert_eq!(code(check(&req(p, &ck, &[]), &t)), 403, "{p}");
        }
        let mut put = req("/api/show", &[("x-token", "out-1")], &[]);
        put.method = "PUT".into();
        assert_eq!(code(check(&put, &t)), 403);
        // operator z ciasteczkiem wyjścia w tej samej przeglądarce: wygrywa silniejszy token i nadpisuje ciasteczko
        assert!(matches!(check(&req("/trio", &ck, &[("token", "op-1")]), &t), Verdict::AllowSetCookie(ref c) if c == "op-1"));
        assert_eq!(code(check(&req("/api/clear", &[("x-token", "op-1")], &[]), &t)), 200);
        assert_eq!(code(check(&req("/output", &[], &[("token", "zly")]), &t)), 401);
    }

    #[test]
    fn failures_are_slowed_down_per_ip() {
        for _ in 0..FREE_FAILURES {
            assert_eq!(note_failure("203.0.113.9"), std::time::Duration::ZERO);
        }
        assert!(note_failure("203.0.113.9") > std::time::Duration::ZERO);
        assert_eq!(note_failure("203.0.113.10"), std::time::Duration::ZERO, "inny adres bez kary");
        let mut d = std::time::Duration::ZERO;
        for _ in 0..50 {
            d = note_failure("203.0.113.9");
        }
        assert_eq!(d, std::time::Duration::from_secs(3), "opóźnienie ograniczone");
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
