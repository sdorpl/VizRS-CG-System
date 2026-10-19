//! Log audytowy (F2.7): `log.jsonl` w katalogu danych – kto, kiedy i jaką komendą sterował systemem.
//! Rotacja po 5 MB (`log.1.jsonl`, jedna poprzednia generacja). Tokeny nie trafiają do logu.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::json::Json;

const MAX_BYTES: u64 = 5 * 1024 * 1024;

pub struct Audit {
    path: PathBuf,
    lock: Mutex<()>,
}

/// Usuwa `token=…` z fragmentu URL/komendy.
pub fn scrub(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("token=") {
        out.push_str(&rest[..i]);
        out.push_str("token=***");
        let tail = &rest[i + 6..];
        let end = tail.find(['&', ' ', '|']).unwrap_or(tail.len());
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

impl Audit {
    pub fn new(dir: &std::path::Path) -> Audit {
        Audit { path: dir.join("log.jsonl"), lock: Mutex::new(()) }
    }

    pub fn log(&self, src: &str, what: &str, status: Option<u16>) {
        let _g = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        if std::fs::metadata(&self.path).map(|m| m.len() > MAX_BYTES).unwrap_or(false) {
            std::fs::rename(&self.path, self.path.with_file_name("log.1.jsonl")).ok();
        }
        let mut rec = Json::obj().with("t", crate::engine::now_ms() as f64).with("src", src).with("cmd", scrub(what).chars().take(500).collect::<String>());
        if let Some(s) = status {
            rec.set("status", (s as f64).into());
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
            writeln!(f, "{}", rec.dump()).ok();
        }
    }

    /// Ostatnie `n` wpisów (najnowsze na końcu).
    pub fn tail(&self, n: usize) -> Vec<String> {
        let _g = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let s = std::fs::read_to_string(&self.path).unwrap_or_default();
        let lines: Vec<&str> = s.lines().collect();
        lines[lines.len().saturating_sub(n)..].iter().map(|l| l.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_tokens() {
        assert_eq!(scrub("/api/take/1?token=abc&x=1"), "/api/take/1?token=***&x=1");
        assert_eq!(scrub("AUTH token=zzz"), "AUTH token=***");
        assert_eq!(scrub("nic"), "nic");
    }

    #[test]
    fn writes_and_tails() {
        let d = std::env::temp_dir().join(format!("vizrs-audit-{}", std::process::id()));
        std::fs::remove_dir_all(&d).ok();
        std::fs::create_dir_all(&d).unwrap();
        let a = Audit::new(&d);
        a.log("http:127.0.0.1", "GET /api/take/1001?token=SEKRET", Some(200));
        a.log("tcp:127.0.0.1", "CLEAR", None);
        let t = a.tail(10);
        assert_eq!(t.len(), 2);
        assert!(t[0].contains("take/1001") && !t[0].contains("SEKRET"));
        assert!(t[1].contains("CLEAR"));
        std::fs::remove_dir_all(&d).ok();
    }
}
