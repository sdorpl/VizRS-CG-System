//! Most strona → powłoka: komunikaty `app:` stron Artist/Trio z serwera (biała lista, limity).
//! Strona nie dostaje dostępu do systemu: może zmienić tytuł własnego okna, stan pozycji menu,
//! zdecydować o zamknięciu własnego okna i zapisać kopię roboczą w katalogu powłoki pod nazwą z skrótu.

use std::path::PathBuf;

use serde_json::Value;

const MAX_TITLE: usize = 200;
const MAX_ID: usize = 128;
/// Największa kopia robocza sceny.
pub const MAX_DRAFT: usize = 16 * 1024 * 1024;

#[derive(Debug, PartialEq)]
pub enum Msg {
    State { title: String, dirty: bool, undo: bool, redo: bool, scene: Option<String> },
    /// Użytkownik zgodził się zamknąć okno.
    Close,
    /// Użytkownik został w oknie.
    Stay,
    Draft { id: String, data: Option<String> },
}

fn clean(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect()
}

pub fn parse(body: &str) -> Option<Msg> {
    let v: Value = serde_json::from_str(body).ok()?;
    let id = || v["id"].as_str().filter(|i| !i.is_empty() && i.len() <= MAX_ID).map(String::from);
    let b = |k: &str| v[k].as_bool().unwrap_or(false);
    match v["t"].as_str()? {
        "state" => Some(Msg::State { title: clean(v["title"].as_str()?, MAX_TITLE), dirty: b("dirty"), undo: b("undo"), redo: b("redo"), scene: v["scene"].as_str().filter(|s| s.len() <= MAX_ID).map(String::from) }),
        "close" => Some(Msg::Close),
        "stay" => Some(Msg::Stay),
        "draft" => {
            let data = match &v["data"] {
                Value::Null => None,
                Value::String(s) if s.len() <= MAX_DRAFT => Some(s.clone()),
                _ => return None,
            };
            Some(Msg::Draft { id: id()?, data })
        }
        _ => None,
    }
}

/// Plik kopii roboczej: nazwa to skrót serwera i sceny, więc identyfikator ze strony nie wskaże ścieżki.
pub fn draft_path(base: &str, id: &str) -> PathBuf {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in base.bytes().chain([0]).chain(id.bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }
    crate::paths::state_dir().join("drafts").join(format!("{h:016x}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_and_bad_messages_are_ignored() {
        assert_eq!(parse("śmieci"), None);
        assert_eq!(parse(r#"{"t":"connect","url":"http://zly"}"#), None);
        assert_eq!(parse(r#"{"t":"draft","id":"","data":"x"}"#), None);
        assert_eq!(parse(r#"{"t":"draft","id":"a","data":5}"#), None);
        let big = format!(r#"{{"t":"draft","id":"a","data":"{}"}}"#, "x".repeat(MAX_DRAFT + 1));
        assert_eq!(parse(&big), None);
    }

    #[test]
    fn title_is_cleaned() {
        let m = parse(r#"{"t":"state","title":"a\nb","dirty":true}"#).unwrap();
        assert_eq!(m, Msg::State { title: "ab".into(), dirty: true, undo: false, redo: false, scene: None });
    }

    #[test]
    fn draft_name_never_follows_id() {
        let p = draft_path("http://h", "../../etc/passwd");
        assert_eq!(p.parent().unwrap().file_name().unwrap(), "drafts");
        assert_ne!(p, draft_path("http://g", "../../etc/passwd"));
    }
}
