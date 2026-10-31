//! Czcionki: wbudowane fonty serwera (`/fonts/fonts.css` + pliki WOFF2), fonty z assets (rodzina = nazwa pliku,
//! jak `new FontFace(name, …)` w `renderer.js`) i katalogi z plikami. Dobór odmiany jak w CSS (grubość, kursywa),
//! łańcuch zastępczy jak w rendererze: `"<font>", "Barlow Condensed", Inter, Arial, sans-serif`.
//!
//! Dane fontów żyją do końca procesu (`Box::leak`): kształtowanie (`rustybuzz::Face<'static>`) ich nie kopiuje,
//! a nowe pliki pojawiają się rzadko (wgranie fontu do assets).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use rustybuzz::ttf_parser;
use tiny_skia::{Path as SkPath, PathBuilder};

/// Jedna odmiana fontu.
pub struct Face {
    /// Rodzina jak w CSS (porównywana bez wielkości liter).
    pub family: String,
    pub weight: u16,
    pub italic: bool,
    pub hb: rustybuzz::Face<'static>,
    pub upem: f32,
    /// Wysokość nad i pod linią bazową w jednostkach fontu (jak Skia + FreeType w Chromium).
    pub ascent: f32,
    pub descent: f32,
    /// Odstęp między wierszami przy `line-height: normal` (jednostki fontu).
    pub line_gap: f32,
    /// Grubość i położenie podkreślenia (jednostki fontu).
    pub underline: (f32, f32),
    glyphs: Mutex<HashMap<u16, Option<SkPath>>>,
}

impl Face {
    fn new(family: &str, weight: Option<u16>, italic: Option<bool>, data: &'static [u8]) -> Option<Face> {
        let hb = rustybuzz::Face::from_slice(data, 0)?;
        let upem = f32::from(hb.units_per_em().max(1) as u16);
        let (mut ascent, mut descent) = (f32::from(hb.ascender()), -f32::from(hb.descender()));
        let mut line_gap = f32::from(hb.line_gap());
        // Skia bierze metryki typograficzne, gdy font ma bit USE_TYPO_METRICS (OS/2 fsSelection, bit 7)
        if let Some(os2) = hb.tables().os2 {
            if os2.use_typographic_metrics() {
                ascent = f32::from(os2.typographic_ascender());
                descent = -f32::from(os2.typographic_descender());
                line_gap = f32::from(os2.typographic_line_gap());
            }
        }
        if ascent == 0.0 && descent == 0.0 {
            if let Some(os2) = hb.tables().os2 {
                ascent = f32::from(os2.windows_ascender());
                descent = f32::from(os2.windows_descender());
            }
        }
        let underline =
            hb.underline_metrics().map(|m| (f32::from(m.thickness), f32::from(m.position))).unwrap_or((upem / 14.0, -upem / 10.0));
        let weight = weight.unwrap_or_else(|| hb.weight().to_number());
        let italic = italic.unwrap_or_else(|| hb.is_italic() || hb.is_oblique());
        Some(Face {
            family: family.to_string(),
            weight,
            italic,
            hb,
            upem,
            ascent,
            descent,
            line_gap: line_gap.max(0.0),
            underline,
            glyphs: Mutex::new(HashMap::new()),
        })
    }

    pub fn has_char(&self, c: char) -> bool {
        self.hb.glyph_index(c).is_some()
    }

    /// Kontur glifu w jednostkach fontu (oś Y w dół), z pamięci podręcznej.
    pub fn glyph_path(&self, id: u16) -> Option<SkPath> {
        let mut cache = self.glyphs.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(id)
            .or_insert_with(|| {
                let mut b = Outline(PathBuilder::new());
                self.hb.outline_glyph(ttf_parser::GlyphId(id), &mut b)?;
                b.0.finish()
            })
            .clone()
    }
}

struct Outline(PathBuilder);

impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, -y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, -y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, -y1, x, -y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, -y1, x2, -y2, x, -y);
    }
    fn close(&mut self) {
        self.0.close();
    }
}

/// Plik fontu (TTF, OTF, WOFF, WOFF2) jako dane sfnt.
pub fn decode_font(data: &[u8]) -> Result<Vec<u8>, String> {
    match data.get(..4) {
        Some(b"wOF2") => wuff::decompress_woff2(data).map_err(|e| format!("WOFF2: {e:?}")),
        Some(b"wOFF") => wuff::decompress_woff1(data).map_err(|e| format!("WOFF: {e:?}")),
        Some([0, 1, 0, 0] | b"OTTO" | b"true") => Ok(data.to_vec()),
        _ => Err("nieznany format fontu".into()),
    }
}

/// Rodziny systemowe, którym w rendererze odpowiada font zastępczy (bez pobierania z Google Fonts).
pub const SYSTEM_FAMILIES: [&str; 11] =
    ["Arial", "Helvetica", "Verdana", "Georgia", "Times New Roman", "Courier New", "Tahoma", "Impact", "sans-serif", "serif", "monospace"];

#[derive(Default)]
pub struct FontDb {
    faces: Vec<Face>,
    /// Nazwy plików już dodanych (fonty serwera i assets – bez powtórnego wczytania).
    files: Vec<String>,
}

impl FontDb {
    pub fn new() -> FontDb {
        FontDb::default()
    }

    pub fn len(&self) -> usize {
        self.faces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    pub fn has_file(&self, file: &str) -> bool {
        self.files.iter().any(|f| f == file)
    }

    /// Dodaje font; `family`/`weight`/`italic` = None → z tablic fontu.
    pub fn add(&mut self, file: &str, data: &[u8], family: Option<&str>, weight: Option<u16>, italic: Option<bool>) -> Result<(), String> {
        let sfnt = decode_font(data)?;
        let data: &'static [u8] = Box::leak(sfnt.into_boxed_slice());
        let family = match family {
            Some(f) => f.to_string(),
            None => {
                let face = ttf_parser::Face::parse(data, 0).map_err(|e| format!("{file}: {e}"))?;
                family_name(&face).ok_or_else(|| format!("{file}: brak nazwy rodziny"))?
            }
        };
        let face = Face::new(&family, weight, italic, data).ok_or_else(|| format!("{file}: nieczytelny font"))?;
        self.faces.push(face);
        self.files.push(file.to_string());
        Ok(())
    }

    /// Font z assets: rodzina = nazwa pliku, odmiana normalna (jak `FontFace` bez deskryptorów).
    pub fn add_asset(&mut self, name: &str, data: &[u8]) -> Result<(), String> {
        self.add(name, data, Some(name), Some(400), Some(false))
    }

    /// Wszystkie fonty z katalogu (rekurencyjnie); rodzina i odmiana z tablic fontu.
    pub fn add_dir(&mut self, dir: &Path) -> usize {
        let mut added = 0;
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&d) else { continue };
            let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
            paths.sort();
            for p in paths {
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                if !is_font_file(&name) || self.has_file(&name) {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(&p) {
                    match self.add(&name, &bytes, None, None, None) {
                        Ok(()) => added += 1,
                        Err(e) => eprintln!("vizrs-render: font {}: {e}", p.display()),
                    }
                }
            }
        }
        added
    }

    /// Lista odmian (diagnostyka: `VIZRS_RENDER_DEBUG`).
    pub fn describe(&self) -> Vec<String> {
        self.faces
            .iter()
            .zip(&self.files)
            .map(|(f, file)| format!("{} {} {} ← {file}", f.family, f.weight, if f.italic { "italic" } else { "normal" }))
            .collect()
    }

    pub fn has_family(&self, family: &str) -> bool {
        self.faces.iter().any(|f| f.family.eq_ignore_ascii_case(family))
    }

    /// Odmiany rodziny dobrane jak w CSS (najlepsza pierwsza); zwraca indeksy.
    fn family_faces(&self, family: &str, weight: u16, italic: bool) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..self.faces.len()).filter(|&i| self.faces[i].family.eq_ignore_ascii_case(family)).collect();
        if idx.is_empty() {
            return idx;
        }
        // styl: kursywa, jeśli jest; inaczej normalny (i odwrotnie)
        let want: Vec<usize> = idx.iter().copied().filter(|&i| self.faces[i].italic == italic).collect();
        if !want.is_empty() {
            idx = want;
        }
        // grubość: najbliższa wg reguł CSS Fonts 4 (§5.2)
        let rank = |w: u16| -> (u8, i32) {
            let w = i32::from(w);
            let d = i32::from(weight);
            if (400..=500).contains(&d) {
                if w >= d && w <= 500 {
                    (0, w - d)
                } else if w < d {
                    (1, d - w)
                } else {
                    (2, w - d)
                }
            } else if d < 400 {
                if w <= d {
                    (0, d - w)
                } else {
                    (1, w - d)
                }
            } else if w >= d {
                (0, w - d)
            } else {
                (1, d - w)
            }
        };
        let best = idx.iter().map(|&i| rank(self.faces[i].weight)).min().unwrap_or((0, 0));
        // wszystkie pliki tej odmiany (podzbiory latin, latin-ext …)
        idx.retain(|&i| rank(self.faces[i].weight) == best);
        idx
    }

    /// Łańcuch odmian dla fontu węzła: rodzina węzła, potem zastępcze jak w rendererze.
    pub fn chain(&self, font: &str, weight: u16, italic: bool) -> Vec<usize> {
        let mut out = Vec::new();
        for family in [font, "Barlow Condensed", "Inter", "Arial", "sans-serif", "DejaVu Sans", "Liberation Sans"] {
            for i in self.family_faces(family, weight, italic) {
                if !out.contains(&i) {
                    out.push(i);
                }
            }
        }
        if out.is_empty() && !self.faces.is_empty() {
            out.push(0);
        }
        out
    }

    pub fn face(&self, i: usize) -> &Face {
        &self.faces[i]
    }

    /// Czcionka dla znaku: pierwsza w łańcuchu, która go ma (inaczej pierwsza w łańcuchu).
    pub fn face_for(&self, chain: &[usize], c: char) -> Option<usize> {
        chain.iter().copied().find(|&i| self.faces[i].has_char(c)).or_else(|| chain.first().copied())
    }
}

pub fn is_font_file(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    l.ends_with(".ttf") || l.ends_with(".otf") || l.ends_with(".woff") || l.ends_with(".woff2")
}

fn family_name(face: &ttf_parser::Face) -> Option<String> {
    let names = face.names();
    let get =
        |id: u16| names.into_iter().filter(|n| n.name_id == id && n.is_unicode()).find_map(|n| n.to_string()).filter(|s| !s.is_empty());
    get(ttf_parser::name_id::TYPOGRAPHIC_FAMILY).or_else(|| get(ttf_parser::name_id::FAMILY).map(|f| strip_style_words(&f)))
}

/// Rodzina bez słów odmiany („Barlow Condensed SemiBold” → „Barlow Condensed”) – fonty bez nazwy typograficznej
/// (np. podzbiory @fontsource) mają grubość w nazwie rodziny.
fn strip_style_words(family: &str) -> String {
    const STYLE: [&str; 14] = [
        "thin",
        "hairline",
        "extralight",
        "ultralight",
        "light",
        "regular",
        "book",
        "medium",
        "semibold",
        "demibold",
        "bold",
        "extrabold",
        "ultrabold",
        "black",
    ];
    let mut words: Vec<&str> = family.split_whitespace().collect();
    while words.len() > 1 {
        let last = words[words.len() - 1].to_ascii_lowercase().replace('-', "");
        if STYLE.contains(&last.as_str()) || last == "italic" || last == "heavy" {
            words.pop();
        } else {
            break;
        }
    }
    words.join(" ")
}

/// Wpis `@font-face` z `/fonts/fonts.css` serwera VizRS.
#[derive(Debug, PartialEq)]
pub struct CssFace {
    pub family: String,
    pub weight: u16,
    pub italic: bool,
    pub url: String,
}

/// Odczyt `fonts.css` (format generowany przez `source/src/fonts.rs`).
pub fn parse_fonts_css(css: &str) -> Vec<CssFace> {
    let mut out = Vec::new();
    for block in css.split("@font-face").skip(1) {
        let body = block.split('}').next().unwrap_or("");
        let field = |key: &str| {
            body.split(';').find_map(|part| {
                let (k, v) = part.split_once(':')?;
                (k.trim().trim_start_matches('{').trim() == key).then(|| v.trim().to_string())
            })
        };
        let family = field("font-family").map(|f| f.trim_matches(['"', '\'']).to_string());
        let url = field("src").and_then(|s| {
            let start = s.find("url(")? + 4;
            let end = s[start..].find(')')? + start;
            Some(s[start..end].trim_matches(['"', '\'']).to_string())
        });
        let (Some(family), Some(url)) = (family, url) else { continue };
        let weight = field("font-weight").and_then(|w| w.parse().ok()).unwrap_or(400);
        let italic = field("font-style").is_some_and(|s| s == "italic" || s == "oblique");
        out.push(CssFace { family, weight, italic, url });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonts_css_is_read() {
        let css = "@font-face{font-family:\"Barlow Condensed\";font-style:normal;font-weight:700;font-display:block;src:url(/fonts/b.woff2) format(\"woff2\");unicode-range:U+0000-00FF}\n";
        assert_eq!(
            parse_fonts_css(css),
            vec![CssFace { family: "Barlow Condensed".into(), weight: 700, italic: false, url: "/fonts/b.woff2".into() }]
        );
    }

    #[test]
    fn style_words_are_stripped() {
        assert_eq!(strip_style_words("Barlow Condensed SemiBold"), "Barlow Condensed");
        assert_eq!(strip_style_words("Inter Extra-Bold Italic"), "Inter");
        assert_eq!(strip_style_words("Black"), "Black");
    }

    #[test]
    fn unknown_data_is_refused() {
        assert!(decode_font(b"nope").is_err());
    }
}
