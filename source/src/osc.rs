//! Most OSC (UDP) → komendy tekstowe (F2.9). Adresy: `/vizrs/take 1001`, `/vizrs/out main`, `/vizrs/continue main`,
//! `/vizrs/clear`, `/vizrs/arm 1001`, `/vizrs/next <rundown>`, `/vizrs/update 1001 pole wartość`, `/vizrs/macro <nazwa>`.
//! Przy `--token` adres musi mieć postać `/vizrs/<token>/take …` (OSC nie ma nagłówków).

#[derive(Debug, PartialEq, Clone)]
pub enum Arg {
    Int(i64),
    Float(f64),
    Str(String),
}

impl Arg {
    pub fn text(&self) -> String {
        match self {
            Arg::Int(i) => i.to_string(),
            Arg::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{}", *f as i64)
                } else {
                    f.to_string()
                }
            }
            Arg::Str(s) => s.clone(),
        }
    }
}

fn pad4(n: usize) -> usize {
    (n + 4) & !3 // łącznie z kończącym NUL
}

fn read_str(b: &[u8], at: usize) -> Option<(String, usize)> {
    let rest = b.get(at..)?;
    let len = rest.iter().position(|&c| c == 0)?;
    let s = std::str::from_utf8(&rest[..len]).ok()?.to_string();
    Some((s, at + pad4(len)))
}

/// Parsuje pojedynczy komunikat OSC (bundle nie są obsługiwane). Dane z sieci – zero paniki.
pub fn parse(b: &[u8]) -> Option<(String, Vec<Arg>)> {
    if b.len() > 8192 {
        return None;
    }
    let (addr, mut at) = read_str(b, 0)?;
    if !addr.starts_with('/') {
        return None;
    }
    let (tags, next) = match read_str(b, at) {
        Some((t, n)) if t.starts_with(',') => (t, n),
        _ => return Some((addr, vec![])),
    };
    at = next;
    let mut args = vec![];
    for t in tags.chars().skip(1) {
        match t {
            'i' => {
                let v = i32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?);
                args.push(Arg::Int(v as i64));
                at += 4;
            }
            'f' => {
                let v = f32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?);
                args.push(Arg::Float(v as f64));
                at += 4;
            }
            'h' => {
                let v = i64::from_be_bytes(b.get(at..at + 8)?.try_into().ok()?);
                args.push(Arg::Int(v));
                at += 8;
            }
            'd' => {
                let v = f64::from_be_bytes(b.get(at..at + 8)?.try_into().ok()?);
                args.push(Arg::Float(v));
                at += 8;
            }
            's' | 'S' => {
                let (s, n) = read_str(b, at)?;
                args.push(Arg::Str(s));
                at = n;
            }
            'T' | 'F' | 'N' | 'I' => {}
            _ => return None,
        }
    }
    Some((addr, args))
}

/// Tłumaczy komunikat na linię komendy TCP. `Err` = odrzucony (zły adres/token).
pub fn to_command(addr: &str, args: &[Arg], token: &Option<String>) -> Result<String, String> {
    let rest = addr.strip_prefix("/vizrs/").ok_or("adres musi zaczynać się od /vizrs/")?;
    let cmd = match token {
        Some(t) => rest.strip_prefix(&format!("{t}/")).ok_or("zły token w adresie")?,
        None => rest,
    };
    let a: Vec<String> = args.iter().map(|x| x.text()).collect();
    let join = |from: usize| a.get(from..).map(|s| s.join(" ")).unwrap_or_default();
    Ok(match cmd {
        "take" => format!("TAKE {}", join(0)),
        "arm" => format!("ARM {}", join(0)),
        "out" => format!("OUT {}", join(0)),
        "continue" => format!("CONTINUE {}", join(0)),
        "clear" => "CLEAR".to_string(),
        "next" => format!("NEXT {}", join(0)),
        "macro" => format!("MACRO {}", join(0)),
        "update" => {
            let page = a.first().ok_or("update: brak strony")?;
            let kv: Vec<String> = a[1..].chunks(2).filter(|c| c.len() == 2).map(|c| format!("{}={}", c[0], c[1])).collect();
            format!("UPDATE {} {}", page, kv.join("|"))
        }
        _ => return Err(format!("nieznana komenda OSC '{cmd}'")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(addr: &str, tags: &str, args: &[&[u8]]) -> Vec<u8> {
        let mut v = vec![];
        for s in [addr, tags] {
            v.extend(s.as_bytes());
            v.push(0);
            while !v.len().is_multiple_of(4) {
                v.push(0);
            }
        }
        for a in args {
            v.extend(*a);
        }
        v
    }
    fn pstr(s: &str) -> Vec<u8> {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        while !v.len().is_multiple_of(4) {
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_int_float_string() {
        let m = msg("/vizrs/take", ",is", &[&1001i32.to_be_bytes(), &pstr("x")]);
        let (a, args) = parse(&m).unwrap();
        assert_eq!(a, "/vizrs/take");
        assert_eq!(args, vec![Arg::Int(1001), Arg::Str("x".into())]);
        let m = msg("/vizrs/clear", ",", &[]);
        assert_eq!(parse(&m).unwrap().1.len(), 0);
        let m = msg("/vizrs/out", ",f", &[&2.0f32.to_be_bytes()]);
        assert_eq!(parse(&m).unwrap().1[0].text(), "2");
    }

    #[test]
    fn garbage_never_panics() {
        let mut s = 12345u64;
        for _ in 0..20_000 {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let len = (s >> 40) as usize % 60;
            let b: Vec<u8> = (0..len).map(|i| ((s >> (i % 32)) ^ (i as u64 * 31)) as u8).collect();
            let _ = parse(&b);
        }
        // obcięte komunikaty
        let m = msg("/vizrs/take", ",is", &[&1i32.to_be_bytes(), &pstr("abc")]);
        for n in 0..m.len() {
            let _ = parse(&m[..n]);
        }
    }

    #[test]
    fn command_mapping_and_token() {
        assert_eq!(to_command("/vizrs/take", &[Arg::Int(1001)], &None).unwrap(), "TAKE 1001");
        assert_eq!(to_command("/vizrs/update", &[Arg::Int(5001), Arg::Str("homeScore".into()), Arg::Int(3)], &None).unwrap(), "UPDATE 5001 homeScore=3");
        assert!(to_command("/inne/take", &[], &None).is_err());
        assert!(to_command("/vizrs/zla", &[], &None).is_err());
        let t = Some("sek".to_string());
        assert!(to_command("/vizrs/take", &[Arg::Int(1)], &t).is_err());
        assert_eq!(to_command("/vizrs/sek/take", &[Arg::Int(1)], &t).unwrap(), "TAKE 1");
    }
}
