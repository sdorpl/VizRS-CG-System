//! Minimalny parser/serializer JSON (bez zależności zewnętrznych).
//! Obiekty zachowują kolejność kluczy – ważne dla drzewa sceny.

use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn obj() -> Json {
        Json::Obj(Vec::new())
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(v) => v.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Json> {
        match self {
            Json::Obj(v) => v.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, val: Json) {
        if let Json::Obj(v) = self {
            if let Some(slot) = v.iter_mut().find(|(k, _)| k == key) {
                slot.1 = val;
            } else {
                v.push((key.to_string(), val));
            }
        }
    }

    pub fn with(mut self, key: &str, val: impl Into<Json>) -> Json {
        self.set(key, val.into());
        self
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(Json::Str(s)) => Some(s),
            _ => None,
        }
    }

    pub fn num(&self, key: &str) -> Option<f64> {
        match self.get(key) {
            Some(Json::Num(n)) => Some(*n),
            Some(Json::Str(s)) => s.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn arr(&self, key: &str) -> &[Json] {
        match self.get(key) {
            Some(Json::Arr(a)) => a,
            _ => &[],
        }
    }

    pub fn as_arr(&self) -> &[Json] {
        match self {
            Json::Arr(a) => a,
            _ => &[],
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Json::Str(s) = self { Some(s) } else { None }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            Json::Str(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn is_obj(&self) -> bool {
        matches!(self, Json::Obj(_))
    }

    // ---------------- serializacja ----------------

    pub fn dump(&self) -> String {
        let mut s = String::new();
        self.write(&mut s, None, 0);
        s
    }

    pub fn pretty(&self) -> String {
        let mut s = String::new();
        self.write(&mut s, Some(2), 0);
        s
    }

    fn write(&self, out: &mut String, indent: Option<usize>, lvl: usize) {
        let nl = |out: &mut String, l: usize| {
            if let Some(i) = indent {
                out.push('\n');
                for _ in 0..i * l {
                    out.push(' ');
                }
            }
        };
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                if !n.is_finite() {
                    out.push_str("null")
                } else if n.fract() == 0.0 && n.abs() < 1e15 {
                    let _ = write!(out, "{}", *n as i64);
                } else {
                    let _ = write!(out, "{}", n);
                }
            }
            Json::Str(s) => write_str(out, s),
            Json::Arr(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    nl(out, lvl + 1);
                    v.write(out, indent, lvl + 1);
                }
                if !a.is_empty() {
                    nl(out, lvl);
                }
                out.push(']');
            }
            Json::Obj(o) => {
                out.push('{');
                for (i, (k, v)) in o.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    nl(out, lvl + 1);
                    write_str(out, k);
                    out.push(':');
                    if indent.is_some() {
                        out.push(' ');
                    }
                    v.write(out, indent, lvl + 1);
                }
                if !o.is_empty() {
                    nl(out, lvl);
                }
                out.push('}');
            }
        }
    }

    // ---------------- parsowanie ----------------

    pub fn parse(src: &str) -> Result<Json, String> {
        let mut p = Parser { b: src.as_bytes(), i: 0 };
        p.ws();
        let v = p.value(0)?;
        p.ws();
        if p.i != p.b.len() {
            return Err(format!("nadmiarowe dane na pozycji {}", p.i));
        }
        Ok(v)
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl From<&str> for Json {
    fn from(s: &str) -> Self {
        Json::Str(s.to_string())
    }
}
impl From<String> for Json {
    fn from(s: String) -> Self {
        Json::Str(s)
    }
}
impl From<f64> for Json {
    fn from(n: f64) -> Self {
        Json::Num(n)
    }
}
impl From<i64> for Json {
    fn from(n: i64) -> Self {
        Json::Num(n as f64)
    }
}
impl From<usize> for Json {
    fn from(n: usize) -> Self {
        Json::Num(n as f64)
    }
}
impl From<bool> for Json {
    fn from(b: bool) -> Self {
        Json::Bool(b)
    }
}
impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(o: Option<T>) -> Self {
        o.map(Into::into).unwrap_or(Json::Null)
    }
}
impl From<Vec<Json>> for Json {
    fn from(v: Vec<Json>) -> Self {
        Json::Arr(v)
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\r' | b'\t') {
            self.i += 1;
        }
    }

    fn err<T>(&self, msg: &str) -> Result<T, String> {
        Err(format!("JSON: {} (pozycja {})", msg, self.i))
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > 256 {
            return self.err("zbyt głębokie zagnieżdżenie");
        }
        match self.b.get(self.i) {
            None => self.err("nieoczekiwany koniec"),
            Some(b'{') => {
                self.i += 1;
                let mut o = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(o));
                }
                loop {
                    self.ws();
                    if self.b.get(self.i) != Some(&b'"') {
                        return self.err("oczekiwano klucza");
                    }
                    let k = self.string()?;
                    self.ws();
                    if self.b.get(self.i) != Some(&b':') {
                        return self.err("oczekiwano ':'");
                    }
                    self.i += 1;
                    self.ws();
                    let v = self.value(depth + 1)?;
                    o.push((k, v));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(o));
                        }
                        _ => return self.err("oczekiwano ',' lub '}'"),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(a));
                }
                loop {
                    self.ws();
                    a.push(self.value(depth + 1)?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(a));
                        }
                        _ => return self.err("oczekiwano ',' lub ']'"),
                    }
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') if self.b[self.i..].starts_with(b"true") => {
                self.i += 4;
                Ok(Json::Bool(true))
            }
            Some(b'f') if self.b[self.i..].starts_with(b"false") => {
                self.i += 5;
                Ok(Json::Bool(false))
            }
            Some(b'n') if self.b[self.i..].starts_with(b"null") => {
                self.i += 4;
                Ok(Json::Null)
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let st = self.i;
                self.i += 1;
                while self.i < self.b.len()
                    && matches!(self.b[self.i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
                {
                    self.i += 1;
                }
                let s = std::str::from_utf8(&self.b[st..self.i]).unwrap();
                s.parse::<f64>().map(Json::Num).or_else(|_| self.err("zła liczba"))
            }
            _ => self.err("nieoczekiwany znak"),
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        if self.i + 4 > self.b.len() {
            return self.err("ucięte \\u");
        }
        let s = std::str::from_utf8(&self.b[self.i..self.i + 4]).map_err(|e| e.to_string())?;
        self.i += 4;
        u32::from_str_radix(s, 16).or_else(|_| self.err("zły \\u"))
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // "
        let mut out = String::new();
        loop {
            let st = self.i;
            while self.i < self.b.len() && self.b[self.i] != b'"' && self.b[self.i] != b'\\' {
                self.i += 1;
            }
            out.push_str(std::str::from_utf8(&self.b[st..self.i]).map_err(|e| e.to_string())?);
            match self.b.get(self.i) {
                None => return self.err("niezamknięty string"),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                _ => {
                    self.i += 1;
                    let c = *self.b.get(self.i).ok_or("ucięty escape")?;
                    self.i += 1;
                    match c {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let mut cp = self.hex4()?;
                            if (0xD800..0xDC00).contains(&cp) && self.b[self.i..].starts_with(b"\\u") {
                                self.i += 2;
                                let lo = self.hex4()?;
                                cp = 0x10000 + ((cp - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                        }
                        _ => return self.err("nieznany escape"),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let s = r#"{"a":[1,2.5,-3e2,true,null],"b":"zażółć \"x\"\n","c":{},"d":"😀"}"#;
        let j = Json::parse(s).unwrap();
        let j2 = Json::parse(&j.dump()).unwrap();
        assert_eq!(j, j2);
        assert_eq!(j.str("d"), Some("😀"));
        assert_eq!(j.arr("a")[2], Json::Num(-300.0));
    }

    /// Prosty LCG – deterministyczne testy losowe bez zależności.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            self.0 >> 33
        }
    }

    #[test]
    fn fuzz_parser_never_panics() {
        let alphabet: Vec<char> = "{}[]\":,.-+eE0123456789 tfnulrasn\\\n\u{0}ż😀".chars().collect();
        let mut r = Lcg(42);
        for _ in 0..20_000 {
            let len = (r.next() % 40) as usize;
            let src: String = (0..len).map(|_| alphabet[(r.next() as usize) % alphabet.len()]).collect();
            let _ = Json::parse(&src); // wynik dowolny, byle bez paniki
        }
    }

    #[test]
    fn mutated_valid_json_never_panics() {
        let base = r#"{"scenes":[{"id":"s","stops":[0.5,1],"root":{"children":[{"t":"x"}]}}],"pages":[]}"#.as_bytes().to_vec();
        let mut r = Lcg(7);
        for _ in 0..5_000 {
            let mut b = base.clone();
            for _ in 0..(1 + r.next() % 4) {
                let i = (r.next() as usize) % b.len();
                b[i] = (r.next() % 128) as u8;
            }
            if let Ok(s) = String::from_utf8(b) {
                let _ = Json::parse(&s);
            }
        }
    }

    #[test]
    fn depth_limit() {
        assert!(Json::parse(&"[".repeat(100_000)).is_err());
    }
}
