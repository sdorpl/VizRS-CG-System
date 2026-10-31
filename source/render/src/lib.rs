//! vizrs-render – natywny renderer grafiki VizRS (bez przeglądarki). Ta sama semantyka co `web/renderer.js`
//! dla obsłużonego podzbioru; resztę zgłasza (`issues`), żeby sidecar mógł wrócić do Chromium.
//! Opis działania i protokołu: docs/DOKUMENTACJA.md §10.1, plan: docs/plany/PLAN-native.md.

pub mod blur;
pub mod clock;
pub mod display;
pub mod font;
pub mod host;
pub mod image;
pub mod model;
pub mod net;
pub mod qr;
pub mod raster;
pub mod res;
pub mod richtext;
pub mod scene;
pub mod stream;
pub mod text;
pub mod ticker;

use std::path::Path;

/// Fonty zastępcze z systemu (Chromium przez fontconfig: Arial/Helvetica → Liberation Sans, sans-serif → DejaVu Sans).
pub fn add_system_fonts(db: &mut font::FontDb) -> usize {
    let mut added = 0;
    let mut files = Vec::new();
    let mut stack = vec![Path::new("/usr/share/fonts").to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Some(n) = p.file_name().and_then(|n| n.to_str()) {
                if (n.starts_with("LiberationSans-") || n.starts_with("DejaVuSans.") || n.starts_with("DejaVuSans-Bold."))
                    && n.ends_with(".ttf")
                {
                    files.push(p.clone());
                }
            }
        }
    }
    files.sort();
    for p in files {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let Ok(bytes) = std::fs::read(&p) else { continue };
        if name.starts_with("Liberation") {
            for alias in ["Arial", "Helvetica", "Liberation Sans"] {
                if db.add(&format!("{alias}:{name}"), &bytes, Some(alias), None, None).is_ok() {
                    added += 1;
                }
            }
        } else {
            for alias in ["sans-serif", "DejaVu Sans"] {
                if db.add(&format!("{alias}:{name}"), &bytes, Some(alias), None, None).is_ok() {
                    added += 1;
                }
            }
        }
    }
    added
}
