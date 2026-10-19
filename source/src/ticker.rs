//! Wiadomości tickera po stronie serwera (komendy `TICKER`, walidacja sceny).
//! Zapis jak w `web/ticker-model.js`: wiadomość w wierszu, `KAT#kolor|treść`; `\|` = dosłowny `|`.
//! Zgodność obu implementacji pilnują wspólne wektory `tests/vectors/ticker.json`.

use crate::json::Json;

/// Maks. liczba wiadomości w jednym tickerze.
pub const MAX_ITEMS: usize = 200;
/// Maks. długość jednej wiadomości (znaki).
pub const MAX_CHARS: usize = 500;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Item {
    pub cat: String,
    pub color: String,
    pub text: String,
}

impl Item {
    pub fn to_json(&self) -> Json {
        Json::obj().with("cat", self.cat.as_str()).with("color", self.color.as_str()).with("text", self.text.as_str())
    }
}

/// Indeks bajtowy pierwszego `|`, przed którym nie stoi `\`.
fn split_at(line: &str) -> Option<usize> {
    let b = line.as_bytes();
    (0..b.len()).find(|&i| b[i] == b'|' && (i == 0 || b[i - 1] != b'\\'))
}

fn unesc(s: &str) -> String {
    s.replace("\\|", "|")
}

fn is_color(s: &str) -> bool {
    s.starts_with('#') && (4..=9).contains(&s.len()) && s[1..].bytes().all(|c| c.is_ascii_hexdigit())
}

pub fn parse_item(line: &str) -> Item {
    let Some(i) = split_at(line) else {
        return Item { text: unesc(line.trim()), ..Item::default() };
    };
    let mut cat = line[..i].trim().to_string();
    let mut color = String::new();
    if let Some(h) = cat.rfind('#') {
        if is_color(&cat[h..]) {
            color = cat[h..].to_string();
            cat = cat[..h].trim().to_string();
        }
    }
    Item { cat: unesc(&cat), color, text: unesc(line[i + 1..].trim()) }
}

pub fn serialize_item(it: &Item) -> String {
    let text = it.text.split('\n').map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
    let cat = it.cat.replace(['|', '\n'], " ").trim().to_string();
    let color = if is_color(&it.color) { it.color.as_str() } else { "" };
    if cat.is_empty() && color.is_empty() {
        return text.replace('|', "\\|");
    }
    format!("{cat}{color}|{text}")
}

pub fn parse_items(text: &str) -> Vec<Item> {
    text.split('\n').map(str::trim).filter(|l| !l.is_empty()).map(parse_item).collect()
}

pub fn serialize_items(items: &[Item]) -> String {
    items.iter().filter(|x| !x.text.trim().is_empty()).map(serialize_item).collect::<Vec<_>>().join("\n")
}

/// Limity systemu dla tekstu pola tickera.
pub fn check_text(text: &str) -> Result<(), String> {
    let items = parse_items(text);
    if items.len() > MAX_ITEMS {
        return Err(format!("ticker: maks. {MAX_ITEMS} wiadomości"));
    }
    if let Some(n) = items.iter().position(|x| x.text.chars().count() > MAX_CHARS) {
        return Err(format!("ticker: wiadomość {} dłuższa niż {MAX_CHARS} znaków", n + 1));
    }
    Ok(())
}

/// Mapa kolorów kategorii `PILNE=#e00000; INFO=#0050c0`: każda para `nazwa=kolor`.
pub fn check_cat_colors(s: &str) -> Result<(), String> {
    for part in s.split([';', '\n']).map(str::trim).filter(|p| !p.is_empty()) {
        match part.split_once('=') {
            Some((k, v)) if !k.trim().is_empty() && !v.trim().is_empty() && k.chars().count() <= 100 && v.len() <= 100 => {}
            _ => return Err(format!("catColors: „{part}” – oczekiwano NAZWA=#kolor")),
        }
    }
    Ok(())
}

/// Operacja na liście wiadomości strony (`TICKER <strona> …`).
#[derive(Debug, PartialEq)]
pub enum Op {
    List,
    Add(String),
    Set(usize, String),
    Del(usize),
    Clear,
}

/// `LIST` | `ADD KAT|treść` | `SET n KAT|treść` | `DEL n` | `CLEAR` (n liczone od 1).
pub fn parse_op(s: &str) -> Result<Op, String> {
    let s = s.trim();
    let (cmd, rest) = s.split_once(char::is_whitespace).unwrap_or((s, ""));
    let rest = rest.trim();
    let idx = |v: &str| v.parse::<usize>().ok().filter(|&n| n >= 1).ok_or_else(|| format!("ticker: numer wiadomości (od 1), nie „{v}”"));
    match cmd.to_ascii_uppercase().as_str() {
        "" | "LIST" => Ok(Op::List),
        "ADD" if !rest.is_empty() => Ok(Op::Add(rest.to_string())),
        "SET" => {
            let (n, line) = rest.split_once(char::is_whitespace).ok_or("ticker: SET <n> <KAT|treść>")?;
            Ok(Op::Set(idx(n)?, line.trim().to_string()))
        }
        "DEL" => Ok(Op::Del(idx(rest)?)),
        "CLEAR" => Ok(Op::Clear),
        _ => Err("ticker: LIST | ADD <KAT|treść> | SET <n> <KAT|treść> | DEL <n> | CLEAR".into()),
    }
}

/// Wykonuje operację na tekście pola; zwraca nowy tekst (dla `List` – bez zmian).
pub fn apply(text: &str, op: &Op) -> Result<String, String> {
    let mut items = parse_items(text);
    let check = |n: usize, len: usize| if n > len { Err(format!("ticker: brak wiadomości {n} (jest {len})")) } else { Ok(()) };
    match op {
        Op::List => return Ok(text.to_string()),
        Op::Add(line) => items.push(parse_item(line)),
        Op::Set(n, line) => {
            check(*n, items.len())?;
            items[n - 1] = parse_item(line);
        }
        Op::Del(n) => {
            check(*n, items.len())?;
            items.remove(n - 1);
        }
        Op::Clear => items.clear(),
    }
    let out = serialize_items(&items);
    check_text(&out)?;
    Ok(out)
}

/// Pole tickera w scenie: pierwsze `fields.text` węzła typu `ticker` (albo wskazane `want`, jeśli należy do tickera)
/// i domyślny tekst tego węzła.
pub fn find_field(scene: &Json, want: Option<&str>) -> Option<(String, String)> {
    fn walk(nodes: &[Json], want: Option<&str>) -> Option<(String, String)> {
        for n in nodes {
            if n.str("type") == Some("ticker") {
                if let Some(f) = n.get("fields").and_then(|f| f.str("text")) {
                    if want.is_none_or(|w| w == f) {
                        let def = n.get("props").and_then(|p| p.str("text")).unwrap_or("").to_string();
                        return Some((f.to_string(), def));
                    }
                }
            }
            if let Some(r) = walk(n.arr("children"), want) {
                return Some(r);
            }
        }
        None
    }
    walk(scene.arr("nodes"), want)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wspólne wektory z web/ticker-model.js (tests/vectors/ticker.json).
    #[test]
    fn shared_vectors() {
        let v = Json::parse(include_str!("../tests/vectors/ticker.json")).unwrap();
        let mut n = 0;
        for c in v.arr("parse") {
            let it = parse_item(c.str("in").unwrap());
            let o = c.get("out").unwrap();
            assert_eq!((it.cat.as_str(), it.color.as_str(), it.text.as_str()), (o.str("cat").unwrap(), o.str("color").unwrap(), o.str("text").unwrap()), "parse {:?}", c.str("in"));
            n += 1;
        }
        for c in v.arr("serialize") {
            let o = c.get("in").unwrap();
            let it = Item { cat: o.str("cat").unwrap_or("").into(), color: o.str("color").unwrap_or("").into(), text: o.str("text").unwrap_or("").into() };
            assert_eq!(serialize_item(&it), c.str("out").unwrap(), "serialize {:?}", o.dump());
            if let Some(rt) = c.str("roundtrip") {
                assert_eq!(parse_item(&serialize_item(&it)).text, rt, "round-trip");
            }
            n += 1;
        }
        for c in v.arr("items") {
            assert_eq!(serialize_items(&parse_items(c.str("in").unwrap())), c.str("out").unwrap());
            n += 1;
        }
        assert!(n > 10);
    }

    #[test]
    fn ops() {
        let t = "A|jeden\ndwa";
        assert_eq!(apply(t, &parse_op("ADD SPORT#1f8a4c|trzy").unwrap()).unwrap(), "A|jeden\ndwa\nSPORT#1f8a4c|trzy");
        assert_eq!(apply(t, &parse_op("SET 2 B|nowe dwa").unwrap()).unwrap(), "A|jeden\nB|nowe dwa");
        assert_eq!(apply(t, &parse_op("del 1").unwrap()).unwrap(), "dwa");
        assert_eq!(apply(t, &Op::Clear).unwrap(), "");
        assert!(apply(t, &parse_op("DEL 3").unwrap()).is_err());
        assert!(parse_op("DEL 0").is_err());
        assert!(parse_op("ADD").is_err());
        assert!(parse_op("FOO").is_err());
        let long = "x".repeat(MAX_CHARS + 1);
        assert!(apply("", &Op::Add(long)).is_err());
        let many = vec!["x"; MAX_ITEMS].join("\n");
        assert!(apply(&many, &Op::Add("y".into())).is_err());
    }

    #[test]
    fn cat_colors() {
        assert!(check_cat_colors("PILNE=#e00000; INFO=#0050c0").is_ok());
        assert!(check_cat_colors("").is_ok());
        assert!(check_cat_colors("PILNE").is_err());
        assert!(check_cat_colors("=#fff").is_err());
    }

    #[test]
    fn field_lookup() {
        let s = Json::parse(r#"{"nodes":[{"type":"group","children":[{"type":"text","fields":{"text":"t"}},{"type":"ticker","fields":{"text":"news"},"props":{"text":"a\nb"}}]}]}"#).unwrap();
        assert_eq!(find_field(&s, None), Some(("news".into(), "a\nb".into())));
        assert_eq!(find_field(&s, Some("news")).map(|x| x.0), Some("news".into()));
        assert_eq!(find_field(&s, Some("t")), None);
    }
}
