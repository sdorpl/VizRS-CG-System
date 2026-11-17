//! Pliki statyczne: UI wbudowane w binarkę (`include_str!`, tablica `STATIC`), fonty offline
//! i zasoby z katalogu danych (`/assets/*`).

use crate::config::Config;
use crate::engine::Engine;
use crate::fonts;
use crate::http::{Request, Response};

pub(crate) struct Static {
    path: &'static str,
    ctype: &'static str,
    body: &'static str,
}

pub(crate) const STATIC: &[Static] = &[
    Static { path: "/", ctype: "text/html; charset=utf-8", body: include_str!("../web/index.html") },
    Static { path: "/artist", ctype: "text/html; charset=utf-8", body: include_str!("../web/artist.html") },
    Static { path: "/trio", ctype: "text/html; charset=utf-8", body: include_str!("../web/trio.html") },
    Static { path: "/trio.webmanifest", ctype: "application/manifest+json; charset=utf-8", body: include_str!("../web/trio.webmanifest") },
    Static { path: "/trio-sw.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/trio-sw.js") },
    Static { path: "/trio-icon.svg", ctype: "image/svg+xml", body: include_str!("../web/trio-icon.svg") },
    Static { path: "/output", ctype: "text/html; charset=utf-8", body: include_str!("../web/output.html") },
    Static { path: "/renderer.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/renderer.js") },
    Static { path: "/richtext.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/richtext.js") },
    Static { path: "/cgpack.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/cgpack.js") },
    Static { path: "/pack-ui.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/pack-ui.js") },
    Static { path: "/motion.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/motion.js") },
    Static { path: "/text-model.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/text-model.js") },
    Static { path: "/qr.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/qr.js") },
    Static { path: "/field-rules.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/field-rules.js") },
    Static { path: "/ticker-model.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/ticker-model.js") },
    Static { path: "/ticker-items.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/ticker-items.js") },
    Static { path: "/native.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/native.js") },
    Static { path: "/dialogs.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/dialogs.js") },
    Static { path: "/color-picker.js", ctype: "text/javascript; charset=utf-8", body: include_str!("../web/color-picker.js") },
    Static { path: "/ui.css", ctype: "text/css; charset=utf-8", body: include_str!("../web/ui.css") },
    Static { path: "/ui-frozen.css", ctype: "text/css; charset=utf-8", body: include_str!("../web/ui-frozen.css") },
    Static { path: "/vizrs-ui.css", ctype: "text/css; charset=utf-8", body: include_str!("../web/vizrs-ui.css") },
];

/// Ikona dla ekranu początkowego iOS, który nie przyjmuje SVG.
pub(crate) const TRIO_ICON_PNG: &[u8] = include_bytes!("../web/trio-icon.png");
/// Ekran logowania `/trio`: aplikacja z ikony telefonu nie ma jak dostać `?token=`.
pub(crate) const LOGIN: &str = include_str!("../web/login.html");

/// Odpowiedź na GET pliku statycznego albo `None`, gdy ścieżka nie jest plikiem statycznym.
pub(crate) fn serve(req: &Request, s: &[&str], engine: &Engine, cfg: &Config) -> Option<Response> {
    if req.path == "/trio-icon.png" {
        return Some(Response::new(200, "image/png", TRIO_ICON_PNG).header("Cache-Control", "max-age=86400"));
    }
    let p = req.path.trim_end_matches(".html");
    let p = if p.is_empty() { "/" } else { p };
    if let Some(st) = STATIC.iter().find(|st| st.path == p) {
        if let Some(dir) = &cfg.web_dir {
            let file = if st.path == "/" { "index.html".to_string() } else {
                let f = st.path.trim_start_matches('/');
                if f.contains('.') { f.to_string() } else { format!("{f}.html") }
            };
            if let Ok(b) = std::fs::read(dir.join(file)) {
                return Some(Response::new(200, st.ctype, b));
            }
        }
        return Some(Response::new(200, st.ctype, st.body));
    }
    if s.len() == 2 && s[0] == "fonts" {
        if s[1] == "fonts.css" {
            return Some(Response::new(200, "text/css; charset=utf-8", fonts::css()).header("Cache-Control", "max-age=3600"));
        }
        return Some(match fonts::find(s[1]) {
            Some(f) => Response::new(200, "font/woff2", f.data).header("Cache-Control", "max-age=86400"),
            None => Response::err(404, "brak fontu"),
        });
    }
    if s.len() == 2 && s[0] == "assets" {
        if let Some(n) = safe_name(s[1]) {
            if let Ok(b) = std::fs::read(engine.data_dir.join("assets").join(&n)) {
                return Some(
                    Response::new(200, mime(&n), b)
                        .header("Cache-Control", "max-age=60")
                        .header("Content-Security-Policy", "default-src 'none'; style-src 'unsafe-inline'; sandbox"),
                );
            }
        }
        return Some(Response::err(404, "brak pliku"));
    }
    if req.path == "/favicon.ico" {
        return Some(Response::new(204, "text/plain", ""));
    }
    None
}

fn mime(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "json" => "application/json",
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub(crate) fn safe_name(name: &str) -> Option<String> {
    let n: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect();
    let n = n.trim_start_matches('.').to_string();
    if n.is_empty() || n.len() > 120 { None } else { Some(n) }
}
