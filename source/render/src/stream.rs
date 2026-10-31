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
//! Flagi: bit 0 – wszystko na antenie natywnie, bit 1 – połączenie z VizRS działa.
//! Zmiana listy braków trafia na stderr (`vizrs-render: …`).

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
pub const FLAG_COMPLETE: u8 = 1;
pub const FLAG_CONNECTED: u8 = 2;

pub struct Options {
    pub url: String,
    /// Rozmiar kanału (płótno sceny w px).
    pub size: (u32, u32),
    /// Obszar grafiki w px kanału (x, y, w, h); brak = cały kanał.
    pub area: Option<(i32, i32, u32, u32)>,
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

/// Obraz jako rekord `REGION`: tylko prostokąt z niezerową alfą (reszta przezroczysta); pusty obraz – `EMPTY`.
pub fn region_record(pix: &Pixmap, flags: u8) -> Vec<u8> {
    let (w, h) = (pix.width() as usize, pix.height() as usize);
    let data = pix.data();
    let row_used = |y: usize| data[y * w * 4..(y + 1) * w * 4].chunks_exact(4).any(|p| p[3] != 0);
    let Some(top) = (0..h).find(|&y| row_used(y)) else { return header(EMPTY, flags, w as u32, h as u32).to_vec() };
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
    let rw = right.saturating_sub(left);
    let mut rec = header(REGION, flags, w as u32, h as u32).to_vec();
    for v in [left, top, rw, bottom - top] {
        rec.extend_from_slice(&(v as u32).to_le_bytes());
    }
    let at = rec.len();
    for y in top..bottom {
        rec.extend_from_slice(&data[(y * w + left) * 4..(y * w + right) * 4]);
    }
    unpremultiply(&mut rec[at..]);
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
    let (ax, ay, aw, ah) = opts.area.unwrap_or((0, 0, opts.size.0, opts.size.1));
    let mut pix = Pixmap::new(aw.max(1), ah.max(1)).ok_or("zły rozmiar")?;
    let mut out = std::io::stdout().lock();
    let stdin = std::io::stdin();
    let mut issues: Vec<String> = Vec::new();
    let mut was_empty = false;
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
        let rec = if only_complete && !issues.is_empty() {
            was_empty = false;
            header(SKIPPED, flags, aw, ah).to_vec()
        } else if !host.on_air() {
            // nic na antenie: pusty obraz raz, potem „bez zmian”
            let kind = if was_empty && !force { UNCHANGED } else { EMPTY };
            was_empty = true;
            header(kind, flags, aw, ah).to_vec()
        } else if host.draw(&mut pix, opts.size, (ax, ay), &res) || was_empty || force {
            // po pustym obraz trzeba wysłać, nawet gdy lista się nie zmieniła (`pix` ma ostatni obraz)
            was_empty = false;
            region_record(&pix, flags)
        } else {
            header(UNCHANGED, flags, aw, ah).to_vec()
        };
        out.write_all(&rec).and_then(|_| out.flush()).map_err(|e| e.to_string())?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn header_layout() {
        let h = header(IMAGE, FLAG_COMPLETE | FLAG_CONNECTED, 1920, 120);
        assert_eq!(&h[..4], b"VZR1");
        assert_eq!((h[4], h[5]), (1, 3));
        assert_eq!(u32::from_le_bytes(h[8..12].try_into().unwrap()), 1920);
        assert_eq!(u32::from_le_bytes(h[12..16].try_into().unwrap()), 120);
    }
}
