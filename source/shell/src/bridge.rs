//! Most strona → powłoka: komunikaty `app:` stron Artist/Trio z serwera (biała lista, limity).
//! Strona nie dostaje dostępu do systemu: może zmienić tytuł własnego okna, stan pozycji menu,
//! zdecydować o zamknięciu własnego okna i zapisać kopię roboczą w katalogu powłoki pod nazwą z skrótu.

use std::path::PathBuf;

use serde_json::Value;

const MAX_TITLE: usize = 200;
const MAX_ID: usize = 128;
/// Menu pod prawym przyciskiem: najwięcej pozycji i znaków etykiety.
const MAX_MENU: usize = 24;
const MAX_LABEL: usize = 80;
/// Największa kopia robocza sceny.
pub const MAX_DRAFT: usize = 16 * 1024 * 1024;

#[derive(Debug, PartialEq)]
pub enum Msg {
    /// `alert` – strona wymaga uwagi operatora (np. brak połączenia z serwerem): okno miga w pasku zadań / Docku.
    State { title: String, dirty: bool, undo: bool, redo: bool, alert: bool, scene: Option<String> },
    /// Użytkownik zgodził się zamknąć okno.
    Close,
    /// Użytkownik został w oknie.
    Stay,
    Draft { id: String, data: Option<String> },
    /// Systemowe menu pod prawym przyciskiem w miejscu kursora; wybór wraca do strony jako `id`.
    Menu(Vec<MenuEntry>),
}

#[derive(Debug, PartialEq)]
pub enum MenuEntry {
    Item { id: String, label: String, enabled: bool },
    Separator,
}

/// Pozycje menu: `"-"` = separator, `{id, label, enabled}`. Złe pozycje są pomijane, puste menu = brak komunikatu.
fn menu_entries(v: &Value) -> Option<Vec<MenuEntry>> {
    let list: Vec<MenuEntry> = v
        .as_array()?
        .iter()
        .take(MAX_MENU)
        .filter_map(|e| {
            if e.as_str() == Some("-") {
                return Some(MenuEntry::Separator);
            }
            let id = e["id"].as_str().filter(|i| !i.is_empty() && i.len() <= MAX_ID)?;
            let label = clean(e["label"].as_str()?, MAX_LABEL);
            (!label.trim().is_empty()).then(|| MenuEntry::Item { id: id.into(), label, enabled: e["enabled"].as_bool().unwrap_or(true) })
        })
        .collect();
    list.iter().any(|e| matches!(e, MenuEntry::Item { .. })).then_some(list)
}

fn clean(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect()
}

pub fn parse(body: &str) -> Option<Msg> {
    let v: Value = serde_json::from_str(body).ok()?;
    let id = || v["id"].as_str().filter(|i| !i.is_empty() && i.len() <= MAX_ID).map(String::from);
    let b = |k: &str| v[k].as_bool().unwrap_or(false);
    match v["t"].as_str()? {
        "state" => Some(Msg::State { title: clean(v["title"].as_str()?, MAX_TITLE), dirty: b("dirty"), undo: b("undo"), redo: b("redo"), alert: b("alert"), scene: v["scene"].as_str().filter(|s| s.len() <= MAX_ID).map(String::from) }),
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
        "menu" => menu_entries(&v["items"]).map(Msg::Menu),
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
        assert_eq!(m, Msg::State { title: "ab".into(), dirty: true, undo: false, redo: false, alert: false, scene: None });
    }

    #[test]
    fn menu_is_limited() {
        let m = parse(r#"{"t":"menu","items":[{"id":"take","label":"TAKE\u0007"},"-",{"id":"","label":"x"},{"id":"del","label":"Usuń","enabled":false},5]}"#).unwrap();
        assert_eq!(
            m,
            Msg::Menu(vec![
                MenuEntry::Item { id: "take".into(), label: "TAKE".into(), enabled: true },
                MenuEntry::Separator,
                MenuEntry::Item { id: "del".into(), label: "Usuń".into(), enabled: false },
            ])
        );
        assert_eq!(parse(r#"{"t":"menu","items":["-"]}"#), None);
        let many: Vec<String> = (0..100).map(|i| format!(r#"{{"id":"a{i}","label":"x"}}"#)).collect();
        let Some(Msg::Menu(l)) = parse(&format!(r#"{{"t":"menu","items":[{}]}}"#, many.join(","))) else { panic!() };
        assert_eq!(l.len(), MAX_MENU);
    }

    #[test]
    fn draft_name_never_follows_id() {
        let p = draft_path("http://h", "../../etc/passwd");
        assert_eq!(p.parent().unwrap().file_name().unwrap(), "drafts");
        assert_ne!(p, draft_path("http://g", "../../etc/passwd"));
    }
}
