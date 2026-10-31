//! Zegar (`fmtClock`, `zoneParts`, `zoneTime` z `renderer.js`): formaty, liczniki i strefa IANA.
//! Strefy z bazy systemowej (`/usr/share/zoneinfo`, TZif z regułą POSIX na lata po tablicy);
//! bez bazy Europe/… liczone regułą UE (CET/CEST). `local` = strefa procesu (TZ), domyślnie Europe/Warsaw.

use std::collections::HashMap;
use std::sync::Mutex;

pub const DEFAULT_TZ: &str = "Europe/Warsaw";

/// Reguła POSIX (`CET-1CEST,M3.5.0,M10.5.0/3`).
#[derive(Clone, Debug, PartialEq)]
struct Posix {
    std_off: i64,
    dst: Option<(i64, Rule, i64, Rule, i64)>, // przesunięcie letnie, początek (czas lokalny std), koniec (czas lokalny dst)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Rule {
    /// Mm.w.d
    Month(u32, u32, u32),
    /// Jn (1–365, bez 29 lutego)
    Julian1(u32),
    /// n (0–365)
    Julian0(u32),
}

#[derive(Clone, Debug)]
pub struct Zone {
    /// Chwile zmian (s od epoki) i przesunięcia po nich.
    transitions: Vec<(i64, i64)>,
    first_off: i64,
    posix: Option<Posix>,
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Data z dni od epoki: (rok, miesiąc, dzień).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn month_days(y: i64, m: u32) -> u32 {
    match m {
        2 => 28 + u32::from(is_leap(y)),
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

impl Rule {
    /// Dzień (od epoki) reguły w roku `y`.
    fn day(self, y: i64) -> i64 {
        match self {
            Rule::Month(m, w, d) => {
                let first = days_from_civil(y, m, 1);
                let wd_first = (first + 4).rem_euclid(7) as u32; // 1970-01-01 to czwartek
                let mut day = 1 + (d + 7 - wd_first) % 7 + (w.saturating_sub(1)) * 7;
                let md = month_days(y, m);
                while day > md {
                    day -= 7;
                }
                days_from_civil(y, m, day)
            }
            Rule::Julian1(n) => {
                let mut n = i64::from(n);
                if is_leap(y) && n >= 60 {
                    n += 1;
                }
                days_from_civil(y, 1, 1) + n - 1
            }
            Rule::Julian0(n) => days_from_civil(y, 1, 1) + i64::from(n),
        }
    }
}

fn parse_posix(s: &str) -> Option<Posix> {
    let b = s.as_bytes();
    let mut i = 0;
    let name = |i: &mut usize| -> Option<()> {
        if b.get(*i) == Some(&b'<') {
            while *i < b.len() && b[*i] != b'>' {
                *i += 1;
            }
            *i += 1;
        } else {
            let start = *i;
            while *i < b.len() && b[*i].is_ascii_alphabetic() {
                *i += 1;
            }
            if *i - start < 3 {
                return None;
            }
        }
        Some(())
    };
    let time = |i: &mut usize| -> Option<i64> {
        let mut sign = 1;
        if matches!(b.get(*i), Some(b'+' | b'-')) {
            if b[*i] == b'-' {
                sign = -1;
            }
            *i += 1;
        }
        let mut parts = [0i64; 3];
        for (k, part) in parts.iter_mut().enumerate() {
            let start = *i;
            while *i < b.len() && b[*i].is_ascii_digit() {
                *i += 1;
            }
            if start == *i {
                if k == 0 {
                    return None;
                }
                break;
            }
            *part = s[start..*i].parse().ok()?;
            if b.get(*i) == Some(&b':') && k < 2 {
                *i += 1;
            } else {
                break;
            }
        }
        Some(sign * (parts[0] * 3600 + parts[1] * 60 + parts[2]))
    };
    let rule = |i: &mut usize| -> Option<(Rule, i64)> {
        let num = |i: &mut usize| -> Option<u32> {
            let start = *i;
            while *i < b.len() && b[*i].is_ascii_digit() {
                *i += 1;
            }
            s[start..*i].parse().ok()
        };
        let r = match b.get(*i)? {
            b'M' => {
                *i += 1;
                let m = num(i)?;
                (b.get(*i) == Some(&b'.')).then_some(())?;
                *i += 1;
                let w = num(i)?;
                (b.get(*i) == Some(&b'.')).then_some(())?;
                *i += 1;
                Rule::Month(m, w, num(i)?)
            }
            b'J' => {
                *i += 1;
                Rule::Julian1(num(i)?)
            }
            _ => Rule::Julian0(num(i)?),
        };
        let at = if b.get(*i) == Some(&b'/') {
            *i += 1;
            time(i)?
        } else {
            7200
        };
        Some((r, at))
    };
    name(&mut i)?;
    let std_off = -time(&mut i)?;
    if i >= b.len() {
        return Some(Posix { std_off, dst: None });
    }
    name(&mut i)?;
    let dst_off = if i < b.len() && b[i] != b',' { -time(&mut i)? } else { std_off + 3600 };
    (b.get(i) == Some(&b',')).then_some(())?;
    i += 1;
    let (r1, t1) = rule(&mut i)?;
    (b.get(i) == Some(&b',')).then_some(())?;
    i += 1;
    let (r2, t2) = rule(&mut i)?;
    Some(Posix { std_off, dst: Some((dst_off, r1, t1, r2, t2)) })
}

impl Posix {
    fn offset(&self, t: i64) -> i64 {
        let Some((dst_off, r1, t1, r2, t2)) = self.dst else { return self.std_off };
        let (y, ..) = civil_from_days((t + self.std_off).div_euclid(86400));
        let start = r1.day(y) * 86400 + t1 - self.std_off;
        let end = r2.day(y) * 86400 + t2 - dst_off;
        let in_dst = if start < end { t >= start && t < end } else { !(t >= end && t < start) };
        if in_dst {
            dst_off
        } else {
            self.std_off
        }
    }
}

fn be32(b: &[u8], i: usize) -> Option<i64> {
    Some(i64::from(i32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?)))
}

fn be64(b: &[u8], i: usize) -> Option<i64> {
    Some(i64::from_be_bytes(b.get(i..i + 8)?.try_into().ok()?))
}

/// Plik TZif (wersja 1–4).
pub fn parse_tzif(b: &[u8]) -> Option<Zone> {
    if b.get(..4)? != b"TZif" {
        return None;
    }
    let version = *b.get(4)?;
    let header = |at: usize| -> Option<[usize; 6]> {
        let mut c = [0usize; 6];
        for (k, v) in c.iter_mut().enumerate() {
            *v = be32(b, at + 20 + k * 4)? as usize;
        }
        Some(c)
    };
    let [isut, isstd, leap, timecnt, typecnt, charcnt] = header(0)?;
    let v1_len = timecnt * 5 + typecnt * 6 + charcnt + leap * 8 + isstd + isut;
    let (base, tsize, counts) = if version >= b'2' {
        let at = 44 + v1_len;
        (at + 44, 8, header(at)?)
    } else {
        (44, 4, [isut, isstd, leap, timecnt, typecnt, charcnt])
    };
    let [_, _, leap, timecnt, typecnt, charcnt] = counts;
    let times_at = base;
    let idx_at = times_at + timecnt * tsize;
    let types_at = idx_at + timecnt;
    let ty_off = |k: usize| be32(b, types_at + k * 6);
    let mut transitions = Vec::with_capacity(timecnt);
    for k in 0..timecnt {
        let t = if tsize == 8 { be64(b, times_at + k * 8)? } else { be32(b, times_at + k * 4)? };
        let ty = *b.get(idx_at + k)? as usize;
        transitions.push((t, ty_off(ty)?));
    }
    let first_off = ty_off(0)?;
    let mut posix = None;
    if version >= b'2' {
        let end = types_at + typecnt * 6 + charcnt + leap * (tsize + 4) + counts[1] + counts[0];
        if let Some(rest) = b.get(end..) {
            let s = String::from_utf8_lossy(rest);
            let line = s.trim_matches('\n').lines().next().unwrap_or("").trim();
            posix = parse_posix(line);
        }
    }
    Some(Zone { transitions, first_off, posix })
}

impl Zone {
    /// Przesunięcie (s) w chwili `t` (s od epoki).
    pub fn offset(&self, t: i64) -> i64 {
        match self.transitions.iter().rposition(|&(at, _)| at <= t) {
            Some(i) if i + 1 < self.transitions.len() || self.posix.is_none() => self.transitions[i].1,
            Some(i) => {
                let p = self.posix.as_ref().expect("checked");
                if t >= self.transitions[i].0 {
                    p.offset(t)
                } else {
                    self.transitions[i].1
                }
            }
            None => {
                if self.transitions.is_empty() {
                    self.posix.as_ref().map(|p| p.offset(t)).unwrap_or(self.first_off)
                } else {
                    self.first_off
                }
            }
        }
    }
}

fn eu_rule() -> Zone {
    Zone { transitions: Vec::new(), first_off: 3600, posix: parse_posix("CET-1CEST,M3.5.0,M10.5.0/3") }
}

static ZONES: Mutex<Option<HashMap<String, Option<Zone>>>> = Mutex::new(None);

fn load_zone(name: &str) -> Option<Zone> {
    if name.is_empty() || name.contains("..") || name.starts_with('/') {
        return None;
    }
    let dirs = [
        std::env::var("TZDIR").unwrap_or_default(),
        "/usr/share/zoneinfo".into(),
        "/usr/lib/zoneinfo".into(),
        "/usr/share/lib/zoneinfo".into(),
    ];
    for d in dirs.iter().filter(|d| !d.is_empty()) {
        if let Ok(bytes) = std::fs::read(format!("{d}/{name}")) {
            if let Some(z) = parse_tzif(&bytes) {
                return Some(z);
            }
        }
    }
    match name {
        "UTC" | "Etc/UTC" | "GMT" | "Etc/GMT" | "Z" => Some(Zone { transitions: Vec::new(), first_off: 0, posix: None }),
        // strefy Europy Środkowej bez bazy systemowej
        "Europe/Warsaw" | "Europe/Berlin" | "Europe/Prague" | "Europe/Vienna" | "Europe/Budapest" | "Europe/Bratislava"
        | "Europe/Paris" | "Europe/Rome" | "Europe/Madrid" | "Europe/Amsterdam" | "Europe/Brussels" | "Europe/Copenhagen"
        | "Europe/Stockholm" | "Europe/Oslo" | "Europe/Zurich" | "CET" => Some(eu_rule()),
        _ => None,
    }
}

/// Przesunięcie strefy `tz` w chwili `ms`; `local` = zmienna TZ procesu.
pub fn offset_ms(tz: &str, ms: f64) -> f64 {
    let mut name = tz.trim().to_string();
    if name.is_empty() {
        name = DEFAULT_TZ.into();
    }
    if name == "local" {
        name = std::env::var("TZ")
            .ok()
            .map(|s| s.trim_start_matches(':').to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "localtime".into());
    }
    let t = (ms / 1000.0).floor() as i64;
    let mut cache = ZONES.lock().unwrap_or_else(|e| e.into_inner());
    let map = cache.get_or_insert_with(HashMap::new);
    if !map.contains_key(&name) {
        let z = if name == "localtime" {
            std::fs::read("/etc/localtime").ok().and_then(|b| parse_tzif(&b)).or_else(|| load_zone(DEFAULT_TZ))
        } else {
            load_zone(&name)
        };
        map.insert(name.clone(), z);
    }
    match map.get(&name).and_then(Option::as_ref) {
        Some(z) => z.offset(t) as f64 * 1000.0,
        None => {
            if name == DEFAULT_TZ {
                eu_rule().offset(t) as f64 * 1000.0
            } else {
                drop(cache);
                // nieznana nazwa → strefa domyślna (jak w rendererze)
                offset_ms(DEFAULT_TZ, ms)
            }
        }
    }
}

/// Części daty w strefie: (rok, miesiąc, dzień, godz., min., s).
pub fn zone_parts(ms: f64, tz: &str) -> (i64, u32, u32, u32, u32, u32) {
    let local = ((ms + offset_ms(tz, ms)) / 1000.0).floor() as i64;
    let (y, m, d) = civil_from_days(local.div_euclid(86400));
    let s = local.rem_euclid(86400);
    (y, m, d, (s / 3600) as u32, (s % 3600 / 60) as u32, (s % 60) as u32)
}

/// `zoneTime`: „yyyy-MM-dd HH:mm:ss” jako czas ścienny w strefie → ms od epoki (NaN, gdy nieczytelne).
pub fn zone_time(s: &str, tz: &str) -> f64 {
    let s = s.trim().replacen(' ', "T", 1);
    let explicit = explicit_offset(&s);
    let s = match explicit {
        Some((_, len)) => s[..s.len() - len].to_string(),
        None => s,
    };
    let digits = |a: usize, b: usize| s.get(a..b).filter(|x| x.bytes().all(|c| c.is_ascii_digit())).and_then(|x| x.parse::<i64>().ok());
    let (Some(y), Some(mo), Some(d)) = (digits(0, 4), digits(5, 7), digits(8, 10)) else { return f64::NAN };
    if s.as_bytes().get(4) != Some(&b'-') || s.as_bytes().get(7) != Some(&b'-') {
        return f64::NAN;
    }
    let (mut h, mut mi, mut se) = (0, 0, 0);
    if s.len() > 10 {
        if s.as_bytes()[10] != b'T' {
            return f64::NAN;
        }
        h = digits(11, 13).unwrap_or(-1);
        mi = digits(14, 16).unwrap_or(-1);
        if h < 0 || mi < 0 {
            return f64::NAN;
        }
        if s.len() >= 19 && s.as_bytes()[16] == b':' {
            se = digits(17, 19).unwrap_or(0);
        }
    }
    let wall = (days_from_civil(y, mo as u32, d as u32) * 86400 + h * 3600 + mi * 60 + se) as f64 * 1000.0;
    if let Some((off, _)) = explicit {
        return wall - off as f64 * 1000.0;
    }
    let off = |t: f64| offset_ms(tz, t);
    let t = wall - off(wall);
    wall - off(t)
}

/// Jawne przesunięcie na końcu (`Z`, `+02:00`, `+0200`): (sekundy, długość zapisu).
fn explicit_offset(s: &str) -> Option<(i64, usize)> {
    if s.ends_with(['Z', 'z']) {
        return Some((0, 1));
    }
    let b = s.as_bytes();
    for len in [6usize, 5] {
        let tail = s.get(s.len().checked_sub(len)?..)?;
        let t = tail.as_bytes();
        let ok = matches!(t[0], b'+' | b'-')
            && t[1].is_ascii_digit()
            && t[2].is_ascii_digit()
            && if len == 6 { t[3] == b':' } else { t[3].is_ascii_digit() }
            && t[len - 2].is_ascii_digit()
            && t[len - 1].is_ascii_digit();
        if ok && b.len() > len {
            let hh: i64 = tail[1..3].parse().ok()?;
            let mm: i64 = tail[len - 2..].parse().ok()?;
            let sign = if t[0] == b'-' { -1 } else { 1 };
            return Some((sign * (hh * 3600 + mm * 60), len));
        }
    }
    None
}

fn pad(n: f64) -> String {
    format!("{:02}", n.floor() as i64)
}

fn pl_days(d: f64) -> String {
    format!("{} {}", d as i64, if d as i64 == 1 { "dzień" } else { "dni" })
}

/// Zamienia tokeny formatu (najdłuższe dopasowanie z listy) – jak `replace(/a|b|c/g, …)`.
fn replace_tokens(s: &str, tokens: &[&str], map: &dyn Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < s.len() {
        if let Some(t) = tokens.iter().find(|t| s[i..].starts_with(**t)) {
            out.push_str(&map(t));
            i += t.len();
        } else {
            let c = s[i..].chars().next().expect("in bounds");
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

/// `fmtClock`: tekst zegara dla trybu `mode` (`time` – godzina ścienna `now_ms`; inne – licznik `secs`).
pub fn fmt_clock(fmt: &str, mode: &str, secs: f64, tz: &str, now_ms: f64) -> String {
    // części: parzyste – format, nieparzyste – tekst w apostrofach
    let mut parts: Vec<String> = Vec::new();
    let mut rest = fmt;
    while let Some(a) = rest.find('\'') {
        if let Some(b) = rest[a + 1..].find('\'') {
            parts.push(rest[..a].to_string());
            parts.push(rest[a..a + b + 2].to_string());
            rest = &rest[a + b + 2..];
        } else {
            break;
        }
    }
    parts.push(rest.to_string());
    let lit = |s: &str| s[1..s.len() - 1].to_string();
    if mode == "time" {
        let (y, mo, d, h, mi, s) = zone_parts(now_ms, tz);
        let map = |k: &str| match k {
            "yyyy" => y.to_string(),
            "dd" => format!("{d:02}"),
            "MM" => format!("{mo:02}"),
            "HH" => format!("{h:02}"),
            "H" => h.to_string(),
            "mm" => format!("{mi:02}"),
            "ss" => format!("{s:02}"),
            _ => String::new(),
        };
        return parts
            .iter()
            .enumerate()
            .map(|(i, p)| if i % 2 == 1 { lit(p) } else { replace_tokens(p, &["yyyy", "dd", "MM", "HH", "H", "mm", "ss"], &map) })
            .collect();
    }
    let secs = secs.max(0.0);
    let plain: String = parts.iter().step_by(2).cloned().collect();
    let (has_d, has_h) = (plain.contains('D'), plain.contains('H'));
    let days = (secs / 86400.0).floor();
    let h = if has_d { ((secs % 86400.0) / 3600.0).floor() } else { (secs / 3600.0).floor() };
    let m = if has_h || has_d { ((secs % 3600.0) / 60.0).floor() } else { (secs / 60.0).floor() };
    let map = |k: &str| match k {
        "Dn" => pl_days(days),
        "D" => (days as i64).to_string(),
        "HH" => pad(h),
        "H" => (h as i64).to_string(),
        "mm" => pad(m),
        "ss" => pad(secs % 60.0),
        "S" => (((secs % 1.0) * 10.0).floor() as i64).to_string(),
        _ => String::new(),
    };
    parts
        .iter()
        .enumerate()
        .map(|(i, p)| if i % 2 == 1 { lit(p) } else { replace_tokens(p, &["Dn", "D", "HH", "H", "mm", "ss", "S"], &map) })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eu_rule_switches_on_last_sundays() {
        let z = eu_rule();
        // 2026-03-29 01:00 UTC – zmiana na CEST; 2026-10-25 01:00 UTC – powrót na CET
        let spring = days_from_civil(2026, 3, 29) * 86400 + 3600;
        assert_eq!(z.offset(spring - 1), 3600);
        assert_eq!(z.offset(spring), 7200);
        let autumn = days_from_civil(2026, 10, 25) * 86400 + 3600;
        assert_eq!(z.offset(autumn - 1), 7200);
        assert_eq!(z.offset(autumn), 3600);
    }

    #[test]
    fn posix_strings_parse() {
        let p = parse_posix("CET-1CEST,M3.5.0,M10.5.0/3").expect("parsed");
        assert_eq!(p.std_off, 3600);
        assert!(p.dst.is_some());
        assert_eq!(parse_posix("<+03>-3").map(|p| p.std_off), Some(10800));
        assert_eq!(parse_posix("EST5EDT,M3.2.0,M11.1.0").map(|p| p.std_off), Some(-18000));
    }

    #[test]
    fn warsaw_parts_and_wall_time() {
        let ms = (days_from_civil(2026, 7, 1) * 86400 + 10 * 3600) as f64 * 1000.0; // 10:00 UTC
        let (_, _, _, h, ..) = zone_parts(ms, "Europe/Warsaw");
        assert_eq!(h, 12);
        assert_eq!(zone_time("2026-07-01 12:00:00", "Europe/Warsaw"), ms);
        assert_eq!(zone_time("2026-07-01T10:00:00Z", "Europe/Warsaw"), ms);
        assert!(zone_time("jutro", "Europe/Warsaw").is_nan());
    }

    #[test]
    fn formats_like_the_renderer() {
        assert_eq!(fmt_clock("mm:ss", "down", 125.0, "", 0.0), "02:05");
        assert_eq!(fmt_clock("D 'dni' HH:mm:ss", "down", 90061.0, "", 0.0), "1 dni 01:01:01");
        assert_eq!(fmt_clock("Dn", "down", 86400.0, "", 0.0), "1 dzień");
        assert_eq!(fmt_clock("ss.S", "up", 3.25, "", 0.0), "03.2");
        let ms = (days_from_civil(2026, 1, 2) * 86400 + 3600 * 23 + 59 * 60 + 58) as f64 * 1000.0;
        assert_eq!(fmt_clock("yyyy-MM-dd HH:mm:ss", "time", 0.0, "UTC", ms), "2026-01-02 23:59:58");
    }

    #[test]
    fn civil_round_trip() {
        for d in [-1000, 0, 365, 20000, 20454] {
            let (y, m, dd) = civil_from_days(d);
            assert_eq!(days_from_civil(y, m, dd), d);
        }
    }
}
