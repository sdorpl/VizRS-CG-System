//! Tekst sformatowany – odpowiednik `web/richtext.js`: `[[wyróżnienie]]`, `{b}`, `{i}`, `{u}`, `{c=kolor}`, `{sz=%}`
//! (zagnieżdżanie, niedomknięte zamykają się na końcu, wielkość liter bez znaczenia), `carryLines` dla podziału wierszy.

/// Stan znaczników (jak `emptyState()`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    pub b: u32,
    pub i: u32,
    pub u: u32,
    pub hl: u32,
    pub cs: Vec<String>,
    pub ss: Vec<f32>,
}

/// Przebieg tekstu jednego stylu.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub t: String,
    pub b: bool,
    pub i: bool,
    pub u: bool,
    pub hl: bool,
    /// Kolor `{c=…}` (pusty – kolor tekstu).
    pub c: String,
    /// Rozmiar `{sz=…}` w % (0 – bez zmiany).
    pub sz: f32,
}

fn color_ok(v: &str) -> bool {
    if let Some(h) = v.strip_prefix('#') {
        return (3..=8).contains(&h.len()) && h.bytes().all(|c| c.is_ascii_hexdigit());
    }
    (3..=20).contains(&v.len()) && v.bytes().all(|c| c.is_ascii_alphabetic())
}

/// Znacznik na początku `s`: (długość, zamykający, nazwa, wartość).
fn token(s: &str) -> Option<(usize, bool, &'static str, String)> {
    if s.starts_with("[[") {
        return Some((2, false, "[[", String::new()));
    }
    if s.starts_with("]]") {
        return Some((2, false, "]]", String::new()));
    }
    let rest = s.strip_prefix('{')?;
    let (close, rest) = match rest.strip_prefix('/') {
        Some(r) => (true, r),
        None => (false, rest),
    };
    let lower = rest.to_ascii_lowercase();
    for name in ["sz", "b", "i", "u", "c"] {
        if let Some(after) = lower.strip_prefix(name) {
            let used = s.len() - rest.len() + name.len();
            if after.starts_with('}') {
                return Some((used + 1, close, name, String::new()));
            }
            if after.starts_with('=') {
                let end = after.find('}')?;
                let val = &s[used + 1..used + end];
                return Some((used + end + 1, close, name, val.trim().to_string()));
            }
        }
    }
    None
}

/// Długość znacznika na początku `s` (None – zwykły znak).
pub fn token_len(s: &str) -> Option<usize> {
    token(s).map(|t| t.0)
}

/// `hasMarkup`: czy tekst ma jakikolwiek znacznik.
pub fn has_markup(t: &str) -> bool {
    t.char_indices().any(|(i, c)| (c == '[' || c == ']' || c == '{') && token(&t[i..]).is_some())
}

/// `stripMarkup`: tekst bez znaczników.
pub fn strip(t: &str) -> String {
    parse(t, None).0.into_iter().map(|s| s.t).collect()
}

/// `parseMarkup`: przebiegi i stan na końcu.
pub fn parse(text: &str, init: Option<&State>) -> (Vec<Span>, State) {
    let mut st = init.cloned().unwrap_or_default();
    let mut runs = Vec::new();
    let mut cur = String::new();
    let push = |cur: &mut String, st: &State, runs: &mut Vec<Span>| {
        if !cur.is_empty() {
            runs.push(Span {
                t: std::mem::take(cur),
                b: st.b > 0,
                i: st.i > 0,
                u: st.u > 0,
                hl: st.hl > 0,
                c: st.cs.last().cloned().unwrap_or_default(),
                sz: st.ss.last().copied().unwrap_or(0.0),
            });
        }
    };
    let mut i = 0;
    while i < text.len() {
        if let Some((len, close, name, val)) = token(&text[i..]) {
            push(&mut cur, &st, &mut runs);
            match name {
                "[[" => st.hl += 1,
                "]]" => st.hl = st.hl.saturating_sub(1),
                "b" | "i" | "u" => {
                    let f = match name {
                        "b" => &mut st.b,
                        "i" => &mut st.i,
                        _ => &mut st.u,
                    };
                    *f = if close { f.saturating_sub(1) } else { *f + 1 };
                }
                "c" => {
                    if close {
                        st.cs.pop();
                    } else {
                        let v = if color_ok(&val) { val } else { st.cs.last().cloned().unwrap_or_default() };
                        st.cs.push(v);
                    }
                }
                _ => {
                    if close {
                        st.ss.pop();
                    } else {
                        let n = crate::model::parse_float(&val) as f32;
                        let v = if (30.0..=300.0).contains(&n) { n } else { st.ss.last().copied().unwrap_or(0.0) };
                        st.ss.push(v);
                    }
                }
            }
            i += len;
            continue;
        }
        let c = text[i..].chars().next().expect("in bounds");
        cur.push(c);
        i += c.len_utf8();
    }
    push(&mut cur, &st, &mut runs);
    (runs, st)
}

fn open_tags(st: &State) -> String {
    let mut s = "{b}".repeat(st.b as usize) + &"{i}".repeat(st.i as usize) + &"{u}".repeat(st.u as usize);
    for c in &st.cs {
        s += &format!("{{c={}}}", if c.is_empty() { "inherit" } else { c });
    }
    for z in &st.ss {
        s += &format!("{{sz={}}}", if *z == 0.0 { 100.0 } else { *z });
    }
    s
}

fn close_tags(st: &State) -> String {
    "{/b}".repeat(st.b as usize)
        + &"{/i}".repeat(st.i as usize)
        + &"{/u}".repeat(st.u as usize)
        + &"{/c}".repeat(st.cs.len())
        + &"{/sz}".repeat(st.ss.len())
}

/// `carryLines`: każdy wiersz samodzielnie poprawny (znaczniki domknięte i otwarte ponownie, bez `[[`).
pub fn carry_lines(lines: &[String]) -> Vec<String> {
    let mut st = State::default();
    lines
        .iter()
        .map(|line| {
            let prefix = open_tags(&st);
            let (_, mut state) = parse(line, Some(&st));
            state.hl = 0;
            let out = format!("{prefix}{line}{}", close_tags(&state));
            st = state;
            out
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_like_richtext() {
        let (r, _) = parse("a [[b {B}c{/b}]] {c=#f00}d{/c} {sz=150}e", None);
        let t: Vec<&str> = r.iter().map(|s| s.t.as_str()).collect();
        assert_eq!(t, ["a ", "b ", "c", " ", "d", " ", "e"]);
        assert!(r[1].hl && !r[1].b && r[2].hl && r[2].b);
        assert_eq!(r[4].c, "#f00");
        assert_eq!(r[6].sz, 150.0);
        assert_eq!(parse("{c=url(x)}z", None).0[0].c, "");
        assert_eq!(parse("{bez}", None).0[0].t, "{bez}");
    }

    #[test]
    fn lines_carry_markup() {
        let out = carry_lines(&["{b}ab".into(), "cd{/b}".into()]);
        assert_eq!(out, ["{b}ab{/b}", "{b}cd{/b}"]);
        assert_eq!(carry_lines(&["[[x".into(), "y]]".into()]), ["[[x", "y]]"]);
    }
}
