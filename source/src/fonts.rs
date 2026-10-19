//! Wbudowane fonty offline (OFL-1.1) – `/fonts/fonts.css` i `/fonts/<plik>.woff2`.
//! Dzięki nim grafika nie zależy od Google Fonts (awaria sieci na wizji nie psuje tekstu).

use crate::fonts_data::{Face, FACES, RANGE_LATIN, RANGE_LATIN_EXT};

pub fn find(file: &str) -> Option<&'static Face> {
    FACES.iter().find(|f| f.file == file)
}

/// Nazwy rodzin dostępnych offline (bez powtórzeń).
pub fn families() -> Vec<&'static str> {
    let mut v: Vec<&str> = FACES.iter().map(|f| f.family).collect();
    v.dedup();
    v.sort();
    v.dedup();
    v
}

pub fn css() -> String {
    let mut out = String::new();
    for f in FACES {
        let range = if f.subset == "latin" { RANGE_LATIN } else { RANGE_LATIN_EXT };
        out.push_str(&format!(
            "@font-face{{font-family:\"{}\";font-style:{};font-weight:{};font-display:block;src:url(/fonts/{}) format(\"woff2\");unicode-range:{}}}\n",
            f.family, f.style, f.weight, f.file, range
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_covers_all_faces() {
        let c = css();
        assert_eq!(c.matches("@font-face").count(), FACES.len());
        assert!(c.contains("\"Barlow Condensed\""));
        assert!(c.contains("unicode-range:U+0100"));
    }

    #[test]
    fn files_are_woff2_and_lookup_works() {
        for f in FACES {
            assert_eq!(&f.data[..4], b"wOF2", "{}", f.file);
            assert!(find(f.file).is_some());
        }
        assert!(find("../x").is_none());
        assert!(families().contains(&"Oswald"));
    }
}
