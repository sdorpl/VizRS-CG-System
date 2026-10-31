//! Model wiadomości tickera – odpowiednik `web/ticker-model.js` (te same reguły co `src/ticker.rs` serwera).

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub cat: String,
    pub color: String,
    pub text: String,
}

fn is_hex_color(s: &str) -> bool {
    let Some(h) = s.strip_prefix('#') else { return false };
    matches!(h.len(), 3 | 4 | 6 | 8) && h.bytes().all(|c| c.is_ascii_hexdigit())
}

fn unesc(s: &str) -> String {
    s.replace("\\|", "|")
}

/// `parseTickerItem`: „SPORT#1f8a4c|Treść”.
pub fn parse_item(line: &str) -> Item {
    let b = line.as_bytes();
    let at = (0..b.len()).find(|&i| b[i] == b'|' && (i == 0 || b[i - 1] != b'\\'));
    let Some(i) = at else { return Item { cat: String::new(), color: String::new(), text: unesc(line.trim()) } };
    let mut cat = line[..i].trim().to_string();
    let mut color = String::new();
    // kolor na końcu kategorii: najdłuższy poprawny zapis #…
    if let Some(h) = cat.rfind('#') {
        if is_hex_color(&cat[h..]) {
            color = cat[h..].to_string();
            cat = cat[..h].trim().to_string();
        }
    }
    Item { cat: unesc(&cat), color, text: unesc(line[i + 1..].trim()) }
}

/// `parseTickerItems`: wiadomości z tekstu pola (puste wiersze pomijane).
pub fn parse_items(text: &str) -> Vec<Item> {
    text.split('\n').map(str::trim).filter(|s| !s.is_empty()).map(parse_item).collect()
}

/// `parseCatColors`: „PILNE=#e00000; INFO=#0050c0”.
pub fn parse_cat_colors(s: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for part in s.split([';', '\n']) {
        let Some(i) = part.find('=') else { continue };
        if i < 1 {
            continue;
        }
        let (k, v) = (part[..i].trim().to_uppercase(), part[i + 1..].trim());
        if !k.is_empty() && !v.is_empty() {
            out.insert(k, v.to_string());
        }
    }
    out
}

/// `pageCatColors`: pierwszy kolor kategorii z listy obowiązuje jej wiadomości bez koloru.
pub fn page_cat_colors(list: &[Item], label: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for it in list {
        let k = if it.cat.is_empty() { label } else { &it.cat }.trim().to_uppercase();
        if !k.is_empty() && !it.color.is_empty() {
            out.entry(k).or_insert_with(|| it.color.clone());
        }
    }
    out
}

/// `catColorOf`: kolor z wiadomości > mapa kategorii > domyślny.
pub fn cat_color_of(item: &Item, map: &HashMap<String, String>, fallback: &str, label: &str) -> String {
    if !item.color.is_empty() {
        return item.color.clone();
    }
    let k = if item.cat.is_empty() { label } else { &item.cat }.trim().to_uppercase();
    if !k.is_empty() {
        if let Some(c) = map.get(&k) {
            return c.clone();
        }
    }
    fallback.to_string()
}

/// `continuation`: po zmianie listy – która wiadomość ma być następna.
pub fn continuation(old: &[Item], old_idx: usize, new: &[Item]) -> usize {
    let n = new.len();
    if n == 0 {
        return 0;
    }
    if let Some(cur) = old.get(old_idx) {
        if let Some(at) = new.iter().position(|x| x == cur || (x.text == cur.text && x.cat == cur.cat)) {
            return (at + 1) % n;
        }
    }
    for k in 1..old.len() {
        let o = &old[(old_idx + k) % old.len()];
        if let Some(j) = new.iter().position(|x| x.text == o.text && x.cat == o.cat) {
            return j;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_parse_like_the_model() {
        assert_eq!(parse_item("SPORT#1f8a4c|Gol!"), Item { cat: "SPORT".into(), color: "#1f8a4c".into(), text: "Gol!".into() });
        assert_eq!(parse_item("Bez kategorii"), Item { cat: String::new(), color: String::new(), text: "Bez kategorii".into() });
        assert_eq!(parse_item("a \\| b").text, "a | b");
        assert_eq!(parse_item("#c00|x").color, "#c00");
        assert_eq!(parse_items("a\n\n b \n").len(), 2);
    }

    #[test]
    fn colors_and_continuation() {
        let m = parse_cat_colors("pilne=#e00; INFO = #00f");
        assert_eq!(m.get("PILNE").map(String::as_str), Some("#e00"));
        let list = parse_items("PILNE|a\nINFO#123456|b\nINFO|c");
        let page = page_cat_colors(&list, "");
        assert_eq!(cat_color_of(&list[2], &page, "#c8102e", ""), "#123456");
        assert_eq!(cat_color_of(&list[0], &m, "#c8102e", ""), "#e00");
        let new = parse_items("x\nINFO|b\ny");
        assert_eq!(continuation(&list, 1, &new), 2);
        assert_eq!(continuation(&list, 0, &new), 1);
    }
}
