//! Tryb `stream`: renderer jako proces podrzędny sidecara (VizRS CG Playout) – zamiast bezgłowego Chromium.
//! Sam łączy się z VizRS jak strona `/output` (zdarzenia, fonty, assets), a klatki rysuje na żądanie.
//!
//! Protokół (DOKUMENTACJA §10.1): na stdin wiersze tekstu, na stdout rekordy binarne.
//! * `F <czas_ms> <ścienny_ms> [cf]` – klatka w chwili `czas_ms` (zegar odtwarzania, monotoniczny) przy czasie
//!   ściennym `ścienny_ms` (ms od epoki, do zegarów); `c` – rysuj tylko, gdy całą antenę da się narysować natywnie,
//!   `f` – wyślij obraz także bez zmian (odbiorca wraca po przerwie).
//! * `Q` – koniec.
//!
//! Rekord: 16 bajtów nagłówka `VZR1`, rodzaj (u8), flagi (u8), 2 bajty zera, szerokość i wysokość (u32 LE),
//! potem przy rodzaju 1 piksele RGBA (bez przemnożenia alfy), wiersz po wierszu.
//! Rodzaje: 0 – bez zmian, 1 – nowy obraz, 2 – pusto (przezroczyście), 3 – nie narysowano (`c`, antena niepełna),
//! 4 – nowy obraz, tylko prostokąt z grafiką: po nagłówku x, y, szerokość, wysokość (u32 LE), potem jego piksele
//! (reszta obrazu przezroczysta). Renderer wysyła nowy obraz jako rodzaj 4; odbiorca czyta też rodzaj 1.
//! 5 – nowy obraz przy kilku `--area`: po nagłówku liczba prostokątów (u32 LE), potem każdy jak w rodzaju 4,
//! w px kanału; szerokość i wysokość nagłówka to rozmiar kanału.
//! Flagi: bit 0 – wszystko na antenie natywnie, bit 1 – połączenie z VizRS działa.
//! Zmiana listy braków trafia na stderr (`vizrs-render: …`).

use std::borrow::Cow;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tiny_skia::Pixmap;

use crate::font::{parse_fonts_css, FontDb};
use crate::host::Host;
use crate::net::OutputUrl;
use crate::raster::unpremultiply;
use crate::res::Resources;
use crate::scene::{Clock, Output};

pub const MAGIC: &[u8; 4] = b"VZR1";
pub const UNCHANGED: u8 = 0;
/// Cały obraz (odbiorca musi go czytać; renderer wysyła `REGION`).
pub const IMAGE: u8 = 1;
pub const EMPTY: u8 = 2;
pub const SKIPPED: u8 = 3;
pub const REGION: u8 = 4;
pub const REGIONS: u8 = 5;
pub const FLAG_COMPLETE: u8 = 1;
pub const FLAG_CONNECTED: u8 = 2;

pub struct Options {
    pub url: String,
    /// Rozmiar kanału (płótno sceny w px).
    pub size: (u32, u32),
    /// Obszary grafiki w px kanału (x, y, w, h); pusto = cały kanał.
    pub areas: Vec<(i32, i32, u32, u32)>,
    pub font_dirs: Vec<std::path::PathBuf>,
}

pub fn header(kind: u8, flags: u8, w: u32, h: u32) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[..4].copy_from_slice(MAGIC);
    out[4] = kind;
    out[5] = flags;
    out[8..12].copy_from_slice(&w.to_le_bytes());
    out[12..16].copy_from_slice(&h.to_le_bytes());
    out
}

/// Prostokąt z niezerową alfą (x, y, w, h); pusty obraz – `None`.
fn alpha_box(pix: &Pixmap) -> Option<(usize, usize, usize, usize)> {
    let (w, h) = (pix.width() as usize, pix.height() as usize);
    let data = pix.data();
    // wiersz po 16 px naraz (kompilator wektoryzuje sumę bitową alf), więc puste wiersze kosztują mało
    let row_used = |y: usize| data[y * w * 4..(y + 1) * w * 4].chunks(64).any(|b| b.chunks_exact(4).fold(0, |m, p| m | p[3]) != 0);
    let top = (0..h).find(|&y| row_used(y))?;
    let bottom = (top..h).rev().find(|&y| row_used(y)).unwrap_or(top) + 1;
    let (mut left, mut right) = (w, 0);
    for y in top..bottom {
        let row = &data[y * w * 4..(y + 1) * w * 4];
        if let Some(l) = (0..left).find(|&x| row[x * 4 + 3] != 0) {
            left = l;
        }
        if let Some(r) = (right..w).rev().find(|&x| row[x * 4 + 3] != 0) {
            right = right.max(r + 1);
        }
    }
    Some((left, top, right.saturating_sub(left), bottom - top))
}

/// Dopisuje prostokąt: x, y, w, h (przesunięte o `origin`) i jego piksele bez przemnożenia alfy.
fn push_box(rec: &mut Vec<u8>, pix: &Pixmap, (x, y, w, h): (usize, usize, usize, usize), origin: (i32, i32)) {
    for v in [(x as i64 + origin.0 as i64) as u32, (y as i64 + origin.1 as i64) as u32, w as u32, h as u32] {
        rec.extend_from_slice(&v.to_le_bytes());
    }
    let stride = pix.width() as usize;
    let at = rec.len();
    for row in y..y + h {
        rec.extend_from_slice(&pix.data()[(row * stride + x) * 4..(row * stride + x + w) * 4]);
    }
    unpremultiply(&mut rec[at..]);
}

/// Obraz jako rekord `REGION`: tylko prostokąt z niezerową alfą (reszta przezroczysta); pusty obraz – `EMPTY`.
pub fn region_record(pix: &Pixmap, flags: u8) -> Vec<u8> {
    let (w, h) = (pix.width(), pix.height());
    let Some(found) = alpha_box(pix) else { return header(EMPTY, flags, w, h).to_vec() };
    let mut rec = header(REGION, flags, w, h).to_vec();
    push_box(&mut rec, pix, found, (0, 0));
    rec
}

/// Zasięg alfy wiersza: pierwszy i za ostatnim px z niezerową alfą (`None` – wiersz pusty).
fn row_span(row: &[u8]) -> Option<(usize, usize)> {
    if !row.chunks(64).any(|b| b.chunks_exact(4).fold(0, |m, p| m | p[3]) != 0) {
        return None;
    }
    let px = row.as_chunks::<4>().0;
    let left = px.iter().position(|p| p[3] != 0)?;
    let right = px.iter().rposition(|p| p[3] != 0)? + 1;
    Some((left, right))
}

/// Ostatni rekord `REGION` z zasięgiem alfy każdego wiersza: po rysowaniu częściowym przelicza
/// tylko wiersze obszaru zmiany, a gdy prostokąt alfy się nie zmienił – łata w gotowym rekordzie
/// tylko ten obszar (bez skanu całego obrazu i bez przepisywania całego prostokąta).
#[derive(Default)]
pub struct RegionCache {
    size: (u32, u32),
    rows: Vec<Option<(usize, usize)>>,
    found: Option<(usize, usize, usize, usize)>,
    rec: Vec<u8>,
}

impl RegionCache {
    fn found(&self) -> Option<(usize, usize, usize, usize)> {
        let top = self.rows.iter().position(Option::is_some)?;
        let bottom = self.rows.iter().rposition(Option::is_some)? + 1;
        let left = self.rows[top..bottom].iter().flatten().map(|s| s.0).min()?;
        let right = self.rows[top..bottom].iter().flatten().map(|s| s.1).max()?;
        Some((left, top, right - left, bottom - top))
    }

    /// Rekord obrazu `pix`; `damage` – jedyny zmieniony prostokąt od poprzedniego wywołania (px `pix`),
    /// `None` – obraz mógł zmienić się wszędzie. Wynik ten sam co `region_record`.
    pub fn record(&mut self, pix: &Pixmap, flags: u8, damage: Option<(i32, i32, i32, i32)>) -> &[u8] {
        let (w, h) = (pix.width() as usize, pix.height() as usize);
        let row = |y: usize| &pix.data()[y * w * 4..(y + 1) * w * 4];
        let damage = damage.filter(|_| self.size == (pix.width(), pix.height()) && !self.rec.is_empty()).map(|d| {
            (
                d.0.clamp(0, w as i32) as usize,
                d.1.clamp(0, h as i32) as usize,
                d.2.clamp(0, w as i32) as usize,
                d.3.clamp(0, h as i32) as usize,
            )
        });
        let Some((x0, y0, x1, y1)) = damage else {
            self.size = (pix.width(), pix.height());
            self.rows = (0..h).map(|y| row_span(row(y))).collect();
            self.found = self.found();
            self.rec = match self.found {
                Some(found) => {
                    let mut rec = header(REGION, flags, pix.width(), pix.height()).to_vec();
                    push_box(&mut rec, pix, found, (0, 0));
                    rec
                }
                None => header(EMPTY, flags, pix.width(), pix.height()).to_vec(),
            };
            return &self.rec;
        };
        for y in y0..y1 {
            self.rows[y] = row_span(row(y));
        }
        let found = self.found();
        if found != self.found || found.is_none() {
            // inny prostokąt alfy (albo pusty obraz): rekord od nowa, wiersze już policzone
            self.found = found;
            self.rec = match found {
                Some(f) => {
                    let mut rec = header(REGION, flags, pix.width(), pix.height()).to_vec();
                    push_box(&mut rec, pix, f, (0, 0));
                    rec
                }
                None => header(EMPTY, flags, pix.width(), pix.height()).to_vec(),
            };
            return &self.rec;
        }
        let (bx, by, bw, bh) = found.unwrap_or_default();
        self.rec[5] = flags;
        let (cx0, cx1) = (x0.max(bx), x1.min(bx + bw));
        for y in y0.max(by)..y1.min(by + bh) {
            if cx1 <= cx0 {
                break;
            }
            // nagłówek (16 B) + położenie prostokąta (16 B), potem wiersze prostokąta
            let at = 32 + ((y - by) * bw + (cx0 - bx)) * 4;
            let part = &mut self.rec[at..at + (cx1 - cx0) * 4];
            part.copy_from_slice(&row(y)[cx0 * 4..cx1 * 4]);
            unpremultiply(part);
        }
        &self.rec
    }
}

/// Kilka obszarów jako rekord `REGIONS` (px kanału `size`); wszystkie puste – `EMPTY`.
pub fn regions_record(areas: &[(Pixmap, (i32, i32))], flags: u8, size: (u32, u32)) -> Vec<u8> {
    let found: Vec<_> = areas.iter().filter_map(|(pix, at)| alpha_box(pix).map(|b| (pix, *at, b))).collect();
    if found.is_empty() {
        return header(EMPTY, flags, size.0, size.1).to_vec();
    }
    let mut rec = header(REGIONS, flags, size.0, size.1).to_vec();
    rec.extend_from_slice(&(found.len() as u32).to_le_bytes());
    for (pix, at, b) in found {
        push_box(&mut rec, pix, b, at);
    }
    rec
}

/// Wbudowane fonty serwera (`/fonts/fonts.css`), wczytywane przy starcie.
fn server_fonts(url: &OutputUrl, db: &mut FontDb) -> Result<usize, String> {
    let css = url.get("/fonts/fonts.css")?;
    let mut n = 0;
    for f in parse_fonts_css(&String::from_utf8_lossy(&css)) {
        let file = f.url.rsplit('/').next().unwrap_or(&f.url).to_string();
        if db.has_file(&file) {
            continue;
        }
        let bytes = url.get(&f.url)?;
        db.add(&file, &bytes, Some(&f.family), Some(f.weight), Some(f.italic))?;
        n += 1;
    }
    Ok(n)
}

/// Wątek zdarzeń: fonty serwera, potem `/events` z ponawianiem co sekundę (`retry: 1000`).
fn events_thread(url: OutputUrl, res: Arc<Resources>, connected: Arc<AtomicBool>) -> Receiver<Value> {
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new().name("events".into()).spawn(move || {
        let mut fonts_loaded = false;
        let mut warned = false;
        loop {
            if !fonts_loaded {
                let mut db = res.fonts.write().unwrap_or_else(|e| e.into_inner());
                match server_fonts(&url, &mut db) {
                    Ok(n) => {
                        fonts_loaded = true;
                        drop(db);
                        eprintln!("vizrs-render: {n} fontów z serwera");
                    }
                    Err(e) => {
                        drop(db);
                        if !warned {
                            eprintln!("vizrs-render: fonty serwera: {e}");
                        }
                    }
                }
            }
            let r = url.events(
                &mut || {
                    connected.store(true, Ordering::Relaxed);
                    eprintln!("vizrs-render: połączono z {}:{}", url.host, url.port);
                },
                &mut |data| {
                    if let Ok(v) = serde_json::from_str::<Value>(data) {
                        let _ = tx.send(v);
                    }
                },
            );
            connected.store(false, Ordering::Relaxed);
            if let Err(e) = r {
                if !warned {
                    eprintln!("vizrs-render: zdarzenia: {e}");
                    warned = true;
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
    rx
}

pub fn run(opts: Options) -> Result<(), String> {
    let url = OutputUrl::parse(&opts.url)?;
    let mut db = FontDb::new();
    for d in &opts.font_dirs {
        db.add_dir(d);
    }
    crate::add_system_fonts(&mut db);
    let fetch_url = url.clone();
    let res = Resources::new(db, Some(Arc::new(move |path: &str| fetch_url.get(path))), false);
    let connected = Arc::new(AtomicBool::new(false));
    let events = events_thread(url.clone(), Arc::clone(&res), Arc::clone(&connected));
    let layers = url.param("layers").map(|l| l.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect());
    let mut host = Host::new(layers, url.param("bus").unwrap_or("program"));
    host.output = Output { step: url.param("fx") == Some("step") };
    let dim = |k: &str, d: f64| url.param(k).and_then(|v| v.parse::<f64>().ok()).filter(|v| *v > 0.0).unwrap_or(d);
    host.stage = (dim("w", 1920.0), dim("h", 1080.0));
    let whole = [(0, 0, opts.size.0, opts.size.1)];
    let list: &[(i32, i32, u32, u32)] = if opts.areas.is_empty() { &whole } else { &opts.areas };
    let mut pixes = Vec::with_capacity(list.len());
    for (x, y, w, h) in list {
        pixes.push((Pixmap::new((*w).max(1), (*h).max(1)).ok_or("zły rozmiar")?, (*x, *y)));
    }
    // przy kilku obszarach rekordy mówią w px kanału, przy jednym – w px obszaru, jak dotąd
    let (aw, ah) = if list.len() == 1 { (list[0].2, list[0].3) } else { opts.size };
    let mut out = std::io::stdout().lock();
    let stdin = std::io::stdin();
    let mut issues: Vec<String> = Vec::new();
    let mut was_empty = false;
    let mut region_cache = RegionCache::default();
    let mut last_clk: Option<Clock> = None;
    let mut line = String::new();
    loop {
        line.clear();
        if stdin.lock().read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Ok(());
        }
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("Q") => return Ok(()),
            Some("F") => {}
            _ => continue,
        }
        let now: f64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let wall: f64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(now);
        let want = parts.next().unwrap_or("");
        let only_complete = want.contains('c');
        let force = want.contains('f');
        let clk = Clock { now, wall };
        // zdarzenie przyszło między klatkami: strona obsługuje je od razu, z czasem poprzedniej klatki
        // (`performance.now()` przed następną) – stąd początek rotacji tickera i zegara `up`/`down`
        let at = last_clk.unwrap_or(clk);
        while let Ok(ev) = events.try_recv() {
            host.handle(&ev, at);
        }
        last_clk = Some(clk);
        host.tick(clk, &res);
        let now_issues = host.issues();
        if now_issues != issues {
            if now_issues.is_empty() {
                eprintln!("vizrs-render: cała antena natywnie");
            } else {
                eprintln!("vizrs-render: poza rendererem natywnym: {}", now_issues.join("; "));
            }
            issues = now_issues;
        }
        let mut flags = 0;
        if issues.is_empty() {
            flags |= FLAG_COMPLETE;
        }
        if connected.load(Ordering::Relaxed) {
            flags |= FLAG_CONNECTED;
        }
        let rec: Cow<[u8]> = if only_complete && !issues.is_empty() {
            was_empty = false;
            header(SKIPPED, flags, aw, ah).to_vec().into()
        } else if !host.on_air() {
            // nic na antenie: pusty obraz raz, potem „bez zmian”
            let kind = if was_empty && !force { UNCHANGED } else { EMPTY };
            was_empty = true;
            header(kind, flags, aw, ah).to_vec().into()
        } else if host.draw_areas(&mut pixes.iter_mut().map(|(p, at)| (p, *at)).collect::<Vec<_>>(), opts.size, &res) || was_empty || force
        {
            // po pustym obraz trzeba wysłać, nawet gdy lista się nie zmieniła (`pixes` mają ostatni obraz)
            was_empty = false;
            if pixes.len() == 1 {
                // płótno zmienia się tylko tutaj, więc obszar ostatniego rysowania częściowego (albo żaden,
                // gdy nic nie narysowano) to jedyna zmiana od poprzedniego rekordu
                let (pix, at) = &pixes[0];
                let damage = host.last_damage.map(|d| (d.0 - at.0, d.1 - at.1, d.2 - at.0, d.3 - at.1));
                region_cache.record(pix, flags, damage).into()
            } else {
                regions_record(&pixes, flags, opts.size).into()
            }
        } else {
            header(UNCHANGED, flags, aw, ah).to_vec().into()
        };
        out.write_all(&rec).and_then(|_| out.flush()).map_err(|e| e.to_string())?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_cache_patches_like_a_full_record() {
        let fill = |pix: &mut Pixmap, (x0, y0, x1, y1): (usize, usize, usize, usize), v: [u8; 4]| {
            let w = pix.width() as usize;
            for y in y0..y1 {
                for x in x0..x1 {
                    pix.data_mut()[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&v);
                }
            }
        };
        let mut pix = Pixmap::new(40, 30).unwrap();
        let mut cache = RegionCache::default();
        fill(&mut pix, (5, 5, 30, 20), [100, 50, 0, 200]);
        assert_eq!(cache.record(&pix, 1, None), &region_record(&pix, 1)[..]);
        // zmiana wewnątrz prostokąta alfy, półprzezroczysta – łata
        fill(&mut pix, (10, 8, 14, 12), [64, 0, 0, 128]);
        assert_eq!(cache.record(&pix, 3, Some((10, 8, 14, 12))), &region_record(&pix, 3)[..]);
        // obraz rośnie poza prostokąt i maleje
        fill(&mut pix, (30, 25, 38, 29), [9, 9, 9, 255]);
        assert_eq!(cache.record(&pix, 1, Some((28, 22, 40, 30))), &region_record(&pix, 1)[..]);
        fill(&mut pix, (5, 5, 30, 20), [0, 0, 0, 0]);
        assert_eq!(cache.record(&pix, 1, Some((0, 0, 30, 20))), &region_record(&pix, 1)[..]);
        // pusty obraz i powrót
        fill(&mut pix, (30, 25, 38, 29), [0, 0, 0, 0]);
        assert_eq!(cache.record(&pix, 0, Some((30, 25, 38, 29)))[4], EMPTY);
        fill(&mut pix, (1, 1, 3, 3), [5, 5, 5, 255]);
        assert_eq!(cache.record(&pix, 0, Some((0, 0, 4, 4))), &region_record(&pix, 0)[..]);
    }

    #[test]
    fn region_is_the_alpha_box() {
        let mut pix = Pixmap::new(8, 6).unwrap();
        let w = 8;
        for (x, y) in [(2, 1), (5, 3)] {
            pix.data_mut()[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
        let rec = region_record(&pix, 1);
        assert_eq!(rec[4], REGION);
        let v = |i: usize| u32::from_le_bytes(rec[16 + i * 4..20 + i * 4].try_into().unwrap());
        assert_eq!((v(0), v(1), v(2), v(3)), (2, 1, 4, 3));
        assert_eq!(rec.len(), 32 + 4 * 3 * 4);
        assert_eq!(&rec[32..36], &[255, 0, 0, 255]);
        assert_eq!(region_record(&Pixmap::new(4, 4).unwrap(), 0)[4], EMPTY);
    }

    #[test]
    fn regions_are_in_channel_pixels() {
        let mut a = Pixmap::new(4, 2).unwrap();
        a.data_mut()[(4 + 1) * 4..(4 + 1) * 4 + 4].copy_from_slice(&[0, 255, 0, 255]);
        let empty = Pixmap::new(2, 2).unwrap();
        let rec = regions_record(&[(a, (10, 20)), (empty.clone(), (0, 0))], 1, (100, 50));
        assert_eq!(rec[4], REGIONS);
        let v = |i: usize| u32::from_le_bytes(rec[16 + i * 4..20 + i * 4].try_into().unwrap());
        assert_eq!((v(0), v(1), v(2), v(3), v(4)), (1, 11, 21, 1, 1));
        assert_eq!(rec.len(), 16 + 4 + 16 + 4);
        assert_eq!(regions_record(&[(empty, (0, 0))], 0, (100, 50))[4], EMPTY);
    }

    #[test]
    fn header_layout() {
        let h = header(IMAGE, FLAG_COMPLETE | FLAG_CONNECTED, 1920, 120);
        assert_eq!(&h[..4], b"VZR1");
        assert_eq!((h[4], h[5]), (1, 3));
        assert_eq!(u32::from_le_bytes(h[8..12].try_into().unwrap()), 1920);
        assert_eq!(u32::from_le_bytes(h[12..16].try_into().unwrap()), 120);
    }
}
