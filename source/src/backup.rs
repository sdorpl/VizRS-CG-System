//! Rotacyjne kopie zapasowe show: bieżące (ostatnie 20, nie częściej niż co 30 s),
//! godzinowe (ostatnie 24) i dzienne (ostatnie 14). Plik godzinowy zawiera ostatni stan z danej
//! godziny (czas warszawski, `warsaw_offset`), dzienny – z danego dnia; kopia bieżąca pozwala cofnąć świeży błąd.

use std::fs;
use std::path::Path;

pub const KEEP_RECENT: usize = 20;
pub const RECENT_EVERY_MS: u64 = 30_000;
pub const KEEP_HOURLY: usize = 24;
pub const KEEP_DAILY: usize = 14;

/// Data cywilna (rok, miesiąc, dzień) dla liczby dni od 1970-01-01 (algorytm H. Hinnanta).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Liczba dni od 1970-01-01 dla daty cywilnej (odwrotność `civil_from_days`).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// Sekunda (UTC) zmiany czasu w UE: ostatnia niedziela miesiąca `m`, 01:00 UTC.
fn eu_switch(y: i64, m: u32) -> i64 {
    let last = days_from_civil(y, m, 31);
    let dow = (last + 4).rem_euclid(7); // 0 = niedziela (1970-01-01 to czwartek)
    (last - dow) * 86_400 + 3_600
}

/// Przesunięcie czasu warszawskiego (Europe/Warsaw) względem UTC w sekundach dla chwili `secs` od epoki:
/// CET +1 h, CEST +2 h od ostatniej niedzieli marca do ostatniej niedzieli października (reguła UE).
pub fn warsaw_offset(secs: i64) -> i64 {
    let (y, _, _) = civil_from_days(secs.div_euclid(86_400));
    if secs >= eu_switch(y, 3) && secs < eu_switch(y, 10) { 7_200 } else { 3_600 }
}

/// (godzinowa, dzienna, bieżąca) nazwa pliku dla czasu w ms od epoki – data i godzina warszawska.
pub fn names(now_ms: u64) -> (String, String, String) {
    let secs = (now_ms / 1000) as i64;
    let secs = secs + warsaw_offset(secs);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let sod = secs.rem_euclid(86_400);
    let (h, mi, se) = (sod / 3600, sod % 3600 / 60, sod % 60);
    (
        format!("show-{y:04}{m:02}{d:02}-{h:02}.json"),
        format!("show-{y:04}{m:02}{d:02}.json"),
        format!("show-{y:04}{m:02}{d:02}-{h:02}{mi:02}{se:02}.json"),
    )
}

/// Dozwolona nazwa kopii (ochrona przed ścieżkami).
pub fn valid_name(n: &str) -> bool {
    let Some(rest) = n.strip_prefix("show-").and_then(|r| r.strip_suffix(".json")) else { return false };
    let ok = |s: &str| s.chars().all(|c| c.is_ascii_digit() || c == '-');
    ok(rest) && matches!(rest.len(), 8 | 11 | 15) && !rest.is_empty()
}

fn write_sync(path: &Path, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");
    let mut f = fs::File::create(&tmp)?;
    f.write_all(content.as_bytes())?;
    f.sync_all()?;
    fs::rename(&tmp, path)
}

/// Zapisuje kopię (godzinową i dzienną) i usuwa najstarsze ponad limit.
/// `recent` = zapisz też kopię bieżącą (wywołujący pilnuje odstępu `RECENT_EVERY_MS`).
pub fn rotate(dir: &Path, content: &str, now_ms: u64, recent: bool) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (hourly, daily, cur) = names(now_ms);
    if recent {
        write_sync(&dir.join(&cur), content).map_err(|e| e.to_string())?;
    }
    write_sync(&dir.join(&hourly), content).map_err(|e| e.to_string())?;
    write_sync(&dir.join(&daily), content).map_err(|e| e.to_string())?;
    prune(dir);
    Ok(())
}

fn prune(dir: &Path) {
    let all = list(dir);
    for (len, keep) in [(25, KEEP_RECENT), (21, KEEP_HOURLY), (18, KEEP_DAILY)] {
        let group: Vec<&String> = all.iter().filter(|n| n.len() == len).collect();
        // `list` zwraca od najnowszej
        for old in group.into_iter().skip(keep) {
            fs::remove_file(dir.join(old)).ok();
        }
    }
}

/// Nazwy kopii od najnowszej do najstarszej.
pub fn list(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .map(|rd| rd.flatten().filter_map(|e| e.file_name().into_string().ok()).filter(|n| valid_name(n)).collect())
        .unwrap_or_default();
    v.sort_by(|a, b| b.cmp(a));
    v
}

pub fn read(dir: &Path, name: &str) -> Result<String, String> {
    if !valid_name(name) {
        return Err("zła nazwa kopii".into());
    }
    fs::read_to_string(dir.join(name)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(names(0), ("show-19700101-01.json".into(), "show-19700101.json".into(), "show-19700101-010000.json".into()));
        // 2026-10-19 15:33 UTC = 17:33 czasu letniego
        let ms = 1_792_424_000_000u64;
        assert_eq!(names(ms).1, "show-20261019.json");
        assert_eq!(names(951_782_400_000).1, "show-20000229.json"); // rok przestępny
        // 2026-12-31 23:30 UTC to już 1 stycznia w Warszawie
        assert_eq!(names(1_798_759_800_000).1, "show-20270101.json");
    }

    #[test]
    fn warsaw_dst() {
        for (y, m, d) in [(1970, 1, 1), (2000, 2, 29), (2024, 3, 31), (2026, 10, 25), (2100, 12, 31)] {
            assert_eq!(civil_from_days(days_from_civil(y, m, d)), (y, m, d));
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        // 2026: zmiana 29 marca i 25 października o 01:00 UTC
        let mar = days_from_civil(2026, 3, 29) * 86_400 + 3_600;
        let oct = days_from_civil(2026, 10, 25) * 86_400 + 3_600;
        assert_eq!(eu_switch(2026, 3), mar);
        assert_eq!(eu_switch(2026, 10), oct);
        assert_eq!(warsaw_offset(mar - 1), 3_600);
        assert_eq!(warsaw_offset(mar), 7_200);
        assert_eq!(warsaw_offset(oct - 1), 7_200);
        assert_eq!(warsaw_offset(oct), 3_600);
        assert_eq!(eu_switch(2024, 3), days_from_civil(2024, 3, 31) * 86_400 + 3_600);
    }

    #[test]
    fn name_validation() {
        assert!(valid_name("show-20261019-15.json"));
        assert!(valid_name("show-20261019.json"));
        assert!(valid_name("show-20261019-153300.json"));
        assert!(!valid_name("../show-20261019.json"));
        assert!(!valid_name("show-2026.json"));
        assert!(!valid_name("show.json"));
    }

    #[test]
    fn rotation_prunes() {
        let d = std::env::temp_dir().join(format!("vizrs-bk-{}", std::process::id()));
        fs::remove_dir_all(&d).ok();
        for h in 0..40u64 {
            rotate(&d, &format!("{{\"h\":{h}}}"), h * 3_600_000, true).unwrap();
        }
        let l = list(&d);
        let hourly = l.iter().filter(|n| n.len() == 21).count();
        assert_eq!(hourly, KEEP_HOURLY);
        assert_eq!(l.iter().filter(|n| n.len() == 25).count(), KEEP_RECENT);
        assert!(l.len() <= KEEP_RECENT + KEEP_HOURLY + KEEP_DAILY);
        assert!(l.iter().all(|n| read(&d, n).is_ok()));
        assert!(l.iter().any(|n| read(&d, n).unwrap().contains("\"h\":39")));
        fs::remove_dir_all(&d).ok();
    }
}
