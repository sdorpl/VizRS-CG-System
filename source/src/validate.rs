//! Walidacja danych show, scen, stron i playlist (F1.4). Dane z sieci i plików są traktowane jako wrogie:
//! sprawdzamy typy, zakresy, długości i głębokość, zanim cokolwiek trafi do silnika lub na dysk.

use crate::json::Json;

pub const MAX_SCENES: usize = 5_000;
pub const MAX_PAGES: usize = 50_000;
pub const MAX_PLAYLISTS: usize = 1_000;
pub const MAX_ITEMS: usize = 10_000;
pub const MAX_NODES: usize = 20_000;
pub const MAX_DEPTH: usize = 64;
pub const MAX_STOPS: usize = 200;
pub const MAX_TEXT: usize = 200_000;
pub const MAX_NAME: usize = 300;
pub const MAX_PAGE_NUMBER: f64 = 99_999.0;

type R = Result<(), String>;

fn text_ok(v: &Json, what: &str, max: usize) -> R {
    match v {
        Json::Str(s) if s.chars().count() <= max => Ok(()),
        Json::Str(_) => Err(format!("{what}: tekst dłuższy niż {max} znaków")),
        _ => Err(format!("{what}: oczekiwano tekstu")),
    }
}

/// Tryby dopasowania tekstu do ramki (`props.textFit`).
const TEXT_FIT: [&str; 6] = ["squeeze", "squeeze-shrink", "squeeze-lines", "squeeze-each", "shrink", "none"];
/// Automatyczne łamanie wierszy w `squeeze-lines`/`squeeze-each` (`props.lineBreak`).
const LINE_BREAK: [&str; 2] = ["balance", "fill"];

fn num_in(v: &Json, what: &str, lo: f64, hi: f64) -> R {
    match v {
        Json::Num(n) if n.is_finite() && *n >= lo && *n <= hi => Ok(()),
        Json::Num(_) => Err(format!("{what}: liczba poza zakresem {lo}…{hi}")),
        _ => Err(format!("{what}: oczekiwano liczby")),
    }
}

fn opt<'a>(o: &'a Json, k: &str) -> Option<&'a Json> {
    o.get(k).filter(|v| **v != Json::Null)
}

/// Nazwa warstwy: litery, cyfry, `-` i `_`.
pub fn layer_name_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 50 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn values(v: &Json) -> R {
    let Json::Obj(items) = v else { return Err("values: oczekiwano obiektu".into()) };
    if items.len() > 500 {
        return Err("values: za dużo pól (maks. 500)".into());
    }
    for (k, val) in items {
        if k.is_empty() || k.chars().count() > 100 {
            return Err("values: nieprawidłowa nazwa pola".into());
        }
        match val {
            Json::Null | Json::Bool(_) => {}
            Json::Num(n) if n.is_finite() => {}
            Json::Num(_) => return Err(format!("values.{k}: liczba nieskończona")),
            Json::Str(s) if s.chars().count() <= MAX_TEXT => {}
            Json::Str(_) => return Err(format!("values.{k}: tekst za długi")),
            _ => return Err(format!("values.{k}: dozwolone tylko wartości proste (tekst, liczba, tak/nie)")),
        }
    }
    Ok(())
}

fn node(n: &Json, depth: usize, count: &mut usize) -> R {
    if depth > MAX_DEPTH {
        return Err(format!("scena: drzewo głębsze niż {MAX_DEPTH} poziomów"));
    }
    *count += 1;
    if *count > MAX_NODES {
        return Err(format!("scena: więcej niż {MAX_NODES} węzłów"));
    }
    if !n.is_obj() {
        return Err("scena: węzeł musi być obiektem".into());
    }
    match n.get("type") {
        Some(Json::Str(t)) if !t.is_empty() && t.len() <= 30 => {}
        _ => return Err("scena: węzeł bez poprawnego 'type'".into()),
    }
    if let Some(v) = opt(n, "id") {
        text_ok(v, "węzeł.id", 100)?;
    }
    if let Some(v) = opt(n, "name") {
        text_ok(v, "węzeł.name", MAX_NAME)?;
    }
    for k in ["props", "anim", "fields", "showIf"] {
        if let Some(v) = opt(n, k) {
            if !v.is_obj() {
                return Err(format!("węzeł.{k}: oczekiwano obiektu"));
            }
        }
    }
    if let Some(p) = opt(n, "props") {
        // dopasowanie tekstu (renderer.js: PROPS.textFit, squeezeMin)
        match opt(p, "textFit") {
            None => {}
            Some(Json::Str(f)) if TEXT_FIT.contains(&f.as_str()) => {}
            Some(_) => return Err(format!("props.textFit: dozwolone {}", TEXT_FIT.join(", "))),
        }
        if let Some(v) = opt(p, "squeezeMin") {
            num_in(v, "props.squeezeMin", 0.0, 1.0)?;
        }
        match opt(p, "lineBreak") {
            None => {}
            Some(Json::Str(b)) if LINE_BREAK.contains(&b.as_str()) => {}
            Some(_) => return Err(format!("props.lineBreak: dozwolone {}", LINE_BREAK.join(", "))),
        }
        if let Some(v) = opt(p, "fillSqueeze") {
            num_in(v, "props.fillSqueeze", 0.0, 1.0)?;
        }
    }
    if n.str("type") == Some("ticker") {
        if let Some(p) = opt(n, "props") {
            if let Some(t) = p.str("text") {
                crate::ticker::check_text(t)?;
            }
            if let Some(c) = p.str("catColors") {
                crate::ticker::check_cat_colors(c)?;
            }
        }
    }
    if let Some(Json::Obj(tracks)) = opt(n, "anim") {
        for (prop, kf) in tracks {
            let Json::Arr(keys) = kf else { return Err(format!("anim.{prop}: oczekiwano listy kluczy")) };
            if keys.len() > 2_000 {
                return Err(format!("anim.{prop}: za dużo kluczy"));
            }
            for k in keys {
                num_in(k.get("t").unwrap_or(&Json::Null), "klucz.t", 0.0, 3_600.0)?;
            }
        }
    }
    match opt(n, "children") {
        None => Ok(()),
        Some(Json::Arr(ch)) => ch.iter().try_for_each(|c| node(c, depth + 1, count)),
        Some(_) => Err("węzeł.children: oczekiwano listy".into()),
    }
}

pub fn scene(s: &Json) -> R {
    if !s.is_obj() {
        return Err("scena musi być obiektem".into());
    }
    if let Some(v) = opt(s, "id") {
        text_ok(v, "scena.id", 100)?;
    }
    if let Some(v) = opt(s, "name") {
        text_ok(v, "scena.name", MAX_NAME)?;
    }
    match opt(s, "layer") {
        None => {}
        Some(Json::Str(l)) if layer_name_ok(l) => {}
        Some(_) => return Err("scena.layer: dozwolone litery, cyfry, - i _ (maks. 50)".into()),
    }
    for k in ["width", "height"] {
        if let Some(v) = opt(s, k) {
            num_in(v, &format!("scena.{k}"), 1.0, 16_384.0)?;
        }
    }
    if let Some(v) = opt(s, "duration") {
        num_in(v, "scena.duration", 0.0, 3_600.0)?;
    }
    match opt(s, "stops") {
        None => {}
        Some(Json::Arr(st)) => {
            if st.len() > MAX_STOPS {
                return Err(format!("scena.stops: więcej niż {MAX_STOPS} stop pointów"));
            }
            for v in st {
                num_in(v, "stop point", 0.0, 3_600.0)?;
            }
        }
        Some(_) => return Err("scena.stops: oczekiwano listy liczb".into()),
    }
    if let Some(v) = opt(s, "fieldMeta") {
        field_meta(v)?;
    }
    match opt(s, "markers") {
        None => {}
        Some(Json::Arr(list)) => {
            if list.len() > 500 {
                return Err("scena.markers: maks. 500".into());
            }
            for m in list {
                num_in(m.get("t").unwrap_or(&Json::Null), "marker.t", 0.0, 3_600.0)?;
                if let Some(n) = opt(m, "name") {
                    text_ok(n, "marker.name", 200)?;
                }
            }
        }
        Some(_) => return Err("scena.markers: oczekiwano listy".into()),
    }
    match opt(s, "packAssets") {
        None => {}
        Some(Json::Arr(list)) => {
            if list.len() > 200 {
                return Err("scena.packAssets: maks. 200 plików".into());
            }
            for v in list {
                match v {
                    Json::Str(n) if !n.is_empty() && n.len() <= 200 && !n.contains(['/', '\\', '\0']) => {}
                    _ => return Err("scena.packAssets: oczekiwano nazw plików z assets".into()),
                }
            }
        }
        Some(_) => return Err("scena.packAssets: oczekiwano listy".into()),
    }
    match opt(s, "dock") {
        None => {}
        Some(Json::Arr(list)) => {
            if list.len() > 50 {
                return Err("scena.dock: maks. 50 powiązań".into());
            }
            for d in list {
                match d.get("target") {
                    Some(Json::Str(t)) if t.len() <= 100 && ["layer:", "page:", "scene:"].iter().any(|p| t.starts_with(p) && t.len() > p.len()) => {}
                    _ => return Err("dock.target: oczekiwano layer:<warstwa>, page:<numer> lub scene:<nazwa>".into()),
                }
                for k in ["dx", "dy"] {
                    if let Some(v) = opt(d, k) {
                        num_in(v, &format!("dock.{k}"), -10_000.0, 10_000.0)?;
                    }
                }
                if let Some(v) = opt(d, "dur") {
                    num_in(v, "dock.dur", 0.0, 30.0)?;
                }
            }
        }
        Some(_) => return Err("scena.dock: oczekiwano listy".into()),
    }
    match opt(s, "nodes") {
        None => Ok(()),
        Some(Json::Arr(nodes)) => {
            let mut count = 0;
            nodes.iter().try_for_each(|n| node(n, 1, &mut count))
        }
        Some(_) => Err("scena.nodes: oczekiwano listy".into()),
    }
}

/// Reguły pól kontrolnych (`fieldMeta`): etykieta, warianty oraz reguły operatora (F3.1).
fn field_meta(v: &Json) -> R {
    let Json::Obj(fields) = v else { return Err("scena.fieldMeta: oczekiwano obiektu".into()) };
    if fields.len() > 500 {
        return Err("scena.fieldMeta: za dużo pól".into());
    }
    for (id, m) in fields {
        if !m.is_obj() {
            return Err(format!("fieldMeta.{id}: oczekiwano obiektu"));
        }
        for k in ["label", "help"] {
            if let Some(t) = opt(m, k) {
                text_ok(t, &format!("fieldMeta.{id}.{k}"), 500)?;
            }
        }
        for (k, lo, hi) in [("maxChars", 1.0, 100_000.0), ("softMax", 1.0, 100_000.0), ("maxLines", 1.0, 50.0), ("maxItems", 1.0, 1_000.0), ("tab", 0.0, 999.0)] {
            if let Some(n) = opt(m, k) {
                num_in(n, &format!("fieldMeta.{id}.{k}"), lo, hi)?;
            }
        }
        for k in ["oneLine", "upper", "typo", "readonly", "rich", "spell"] {
            if let Some(b) = opt(m, k) {
                if !matches!(b, Json::Bool(_)) {
                    return Err(format!("fieldMeta.{id}.{k}: oczekiwano tak/nie"));
                }
            }
        }
        if let (Some(Json::Num(soft)), Some(Json::Num(max))) = (opt(m, "softMax"), opt(m, "maxChars")) {
            if soft > max {
                return Err(format!("fieldMeta.{id}: softMax większy niż maxChars"));
            }
        }
        if let Some(o) = opt(m, "cats") {
            let Json::Arr(list) = o else { return Err(format!("fieldMeta.{id}.cats: oczekiwano listy")) };
            if list.len() > 100 || list.iter().any(|x| !matches!(x, Json::Str(s) if !s.is_empty() && s.chars().count() <= 100)) {
                return Err(format!("fieldMeta.{id}.cats: lista kategorii (maks. 100 × 100 znaków)"));
            }
        }
        if let Some(o) = opt(m, "options") {
            let Json::Arr(list) = o else { return Err(format!("fieldMeta.{id}.options: oczekiwano listy")) };
            if list.len() > 200 || list.iter().any(|x| !matches!(x, Json::Str(s) if s.chars().count() <= 200)) {
                return Err(format!("fieldMeta.{id}.options: lista tekstów (maks. 200 × 200 znaków)"));
            }
        }
    }
    Ok(())
}

/// Biblioteka presetów animacji: `[ {id, name, duration, tracks:{właściwość:[{t,v,e}]}} ]`.
pub fn anim_presets(v: &Json) -> R {
    let Json::Arr(list) = v else { return Err("presety: oczekiwano listy".into()) };
    if list.len() > 500 {
        return Err("presety: maks. 500".into());
    }
    for p in list {
        if !p.is_obj() {
            return Err("preset musi być obiektem".into());
        }
        text_ok(p.get("id").unwrap_or(&Json::Null), "preset.id", 100)?;
        text_ok(p.get("name").unwrap_or(&Json::Null), "preset.name", MAX_NAME)?;
        num_in(p.get("duration").unwrap_or(&Json::Null), "preset.duration", 0.0, 3_600.0)?;
        let Some(Json::Obj(tracks)) = p.get("tracks") else { return Err("preset.tracks: oczekiwano obiektu".into()) };
        if tracks.len() > 40 {
            return Err("preset: za dużo ścieżek".into());
        }
        for (prop, keys) in tracks {
            let Json::Arr(keys) = keys else { return Err(format!("preset.tracks.{prop}: oczekiwano listy kluczy")) };
            if keys.len() > 2_000 {
                return Err(format!("preset.tracks.{prop}: za dużo kluczy"));
            }
            for k in keys {
                num_in(k.get("t").unwrap_or(&Json::Null), "klucz.t", 0.0, 3_600.0)?;
            }
        }
    }
    Ok(())
}

/// Lista zamian słów (Word Replace): `{ "klucz": ["słowo", …] }`, klucz 1–8 znaków bez spacji.
pub fn word_replace(v: &Json) -> R {
    let Json::Obj(map) = v else { return Err("lista zamian: oczekiwano obiektu".into()) };
    if map.len() > 2_000 {
        return Err("lista zamian: maks. 2000 kluczy".into());
    }
    for (k, words) in map {
        let n = k.chars().count();
        if !(1..=8).contains(&n) || k.chars().any(char::is_whitespace) {
            return Err(format!("klucz '{k}': 1–8 znaków bez spacji"));
        }
        let Json::Arr(list) = words else { return Err(format!("klucz '{k}': oczekiwano listy słów")) };
        if list.is_empty() || list.len() > 50 || list.iter().any(|w| !matches!(w, Json::Str(s) if !s.trim().is_empty() && s.chars().count() <= 200)) {
            return Err(format!("klucz '{k}': 1–50 niepustych słów (maks. 200 znaków)"));
        }
    }
    Ok(())
}

pub fn page(p: &Json) -> R {
    if !p.is_obj() {
        return Err("strona musi być obiektem".into());
    }
    if let Some(v) = opt(p, "id") {
        text_ok(v, "strona.id", 100)?;
    }
    if let Some(v) = opt(p, "name") {
        text_ok(v, "strona.name", MAX_NAME)?;
    }
    if let Some(v) = opt(p, "scene") {
        text_ok(v, "strona.scene", 100)?;
    }
    if let Some(v) = opt(p, "number") {
        num_in(v, "numer strony", 1.0, MAX_PAGE_NUMBER)?;
        if let Json::Num(n) = v {
            if n.fract() != 0.0 {
                return Err("numer strony musi być liczbą całkowitą".into());
            }
        }
    }
    if let Some(v) = opt(p, "hotkey") {
        text_ok(v, "strona.hotkey", 40)?;
    }
    if let Some(v) = opt(p, "note") {
        text_ok(v, "strona.note", 2_000)?;
    }
    if let Some(v) = opt(p, "values") {
        values(v)?;
    }
    Ok(())
}

pub fn playlist(p: &Json) -> R {
    if !p.is_obj() {
        return Err("playlista musi być obiektem".into());
    }
    if let Some(v) = opt(p, "id") {
        text_ok(v, "playlista.id", 100)?;
    }
    if let Some(v) = opt(p, "name") {
        text_ok(v, "playlista.name", MAX_NAME)?;
    }
    if let Some(v) = opt(p, "loop") {
        if !matches!(v, Json::Bool(_)) {
            return Err("playlista.loop: oczekiwano tak/nie".into());
        }
    }
    match opt(p, "items") {
        None => Ok(()),
        Some(Json::Arr(items)) => {
            if items.len() > MAX_ITEMS {
                return Err("playlista: za dużo pozycji".into());
            }
            for it in items {
                if !it.is_obj() {
                    return Err("playlista.items: pozycja musi być obiektem".into());
                }
                text_ok(it.get("page").unwrap_or(&Json::Null), "pozycja.page", 100)?;
                if let Some(d) = opt(it, "duration") {
                    num_in(d, "pozycja.duration", 0.0, 86_400.0)?;
                }
            }
            Ok(())
        }
        Some(_) => Err("playlista.items: oczekiwano listy".into()),
    }
}

pub fn rundown(r: &Json) -> R {
    if !r.is_obj() {
        return Err("rundown musi być obiektem".into());
    }
    if let Some(v) = opt(r, "id") {
        text_ok(v, "rundown.id", 100)?;
    }
    if let Some(v) = opt(r, "name") {
        text_ok(v, "rundown.name", MAX_NAME)?;
    }
    match opt(r, "items") {
        None => Ok(()),
        Some(Json::Arr(items)) => {
            if items.len() > MAX_ITEMS {
                return Err("rundown: za dużo pozycji".into());
            }
            let mut seen = std::collections::HashSet::new();
            for it in items {
                if !it.is_obj() {
                    return Err("rundown.items: pozycja musi być obiektem".into());
                }
                text_ok(it.get("id").unwrap_or(&Json::Null), "pozycja.id", 100)?;
                if !seen.insert(it.str("id").unwrap_or("")) {
                    return Err("rundown: powtórzone id pozycji".into());
                }
                text_ok(it.get("page").unwrap_or(&Json::Null), "pozycja.page", 100)?;
                if let Some(v) = opt(it, "note") {
                    text_ok(v, "pozycja.note", 2_000)?;
                }
            }
            Ok(())
        }
        Some(_) => Err("rundown.items: oczekiwano listy".into()),
    }
}

/// Makro: lista kroków `{cmd, delay}`; `cmd` w składni portu TCP (bez MACRO – brak rekurencji).
pub fn macro_(m: &Json) -> R {
    if !m.is_obj() {
        return Err("makro musi być obiektem".into());
    }
    if let Some(v) = opt(m, "id") {
        text_ok(v, "makro.id", 100)?;
    }
    if let Some(v) = opt(m, "name") {
        text_ok(v, "makro.name", MAX_NAME)?;
    }
    match opt(m, "steps") {
        None => Ok(()),
        Some(Json::Arr(steps)) => {
            if steps.len() > 100 {
                return Err("makro: maks. 100 kroków".into());
            }
            for st in steps {
                text_ok(st.get("cmd").unwrap_or(&Json::Null), "krok.cmd", 500)?;
                let c = st.str("cmd").unwrap_or("").trim().to_ascii_uppercase();
                if c.is_empty() || c.starts_with("MACRO") {
                    return Err("krok.cmd: pusta komenda lub MACRO (rekurencja niedozwolona)".into());
                }
                if let Some(d) = opt(st, "delay") {
                    num_in(d, "krok.delay", 0.0, 3_600.0)?;
                }
            }
            Ok(())
        }
        Some(_) => Err("makro.steps: oczekiwano listy".into()),
    }
}

/// Cały show (import / `PUT /api/show`).
pub fn show(s: &Json) -> R {
    if !s.is_obj() {
        return Err("show musi być obiektem".into());
    }
    if let Some(v) = opt(s, "name") {
        text_ok(v, "show.name", MAX_NAME)?;
    }
    for (k, max, f) in [
        ("scenes", MAX_SCENES, scene as fn(&Json) -> R),
        ("pages", MAX_PAGES, page),
        ("playlists", MAX_PLAYLISTS, playlist),
        ("rundowns", MAX_PLAYLISTS, rundown),
        ("macros", MAX_PLAYLISTS, macro_),
    ] {
        match s.get(k) {
            None | Some(Json::Null) => {}
            Some(Json::Arr(list)) => {
                if list.len() > max {
                    return Err(format!("show.{k}: więcej niż {max} pozycji"));
                }
                for (i, it) in list.iter().enumerate() {
                    f(it).map_err(|e| format!("show.{k}[{i}]: {e}"))?;
                }
            }
            Some(_) => return Err(format!("show.{k}: oczekiwano listy")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Json {
        Json::parse(s).unwrap()
    }

    #[test]
    fn demo_and_examples_are_valid() {
        show(&p(include_str!("../web/demo_show.json"))).unwrap();
        show(&p(include_str!("../../examples/INFO24.show.json"))).unwrap();
        show(&p(include_str!("../../examples/R7_NEWS.show.json"))).unwrap();
    }

    #[test]
    fn rejects_wrong_types() {
        assert!(show(&p(r#"{"scenes":"x"}"#)).is_err());
        assert!(show(&p(r#"{"scenes":[1]}"#)).is_err());
        assert!(show(&p(r#"{"pages":[{"number":"a"}]}"#)).is_err());
        assert!(show(&p(r#"{"playlists":[{"items":"z"}]}"#)).is_err());
        assert!(show(&p("[]")).is_err());
    }

    #[test]
    fn page_number_range() {
        assert!(page(&p(r#"{"number":1001}"#)).is_ok());
        assert!(page(&p(r#"{"number":0}"#)).is_err());
        assert!(page(&p(r#"{"number":-5}"#)).is_err());
        assert!(page(&p(r#"{"number":1e300}"#)).is_err());
        assert!(page(&p(r#"{"number":100000}"#)).is_err());
        assert!(page(&p(r#"{"number":1.5}"#)).is_err());
    }

    #[test]
    fn scene_limits() {
        assert!(scene(&p(r#"{"stops":[0.5,1],"duration":2}"#)).is_ok());
        assert!(scene(&p(r#"{"stops":[-1]}"#)).is_err());
        assert!(scene(&p(r#"{"duration":99999}"#)).is_err());
        assert!(scene(&p(r#"{"width":0}"#)).is_err());
        assert!(scene(&p(r#"{"layer":"a b"}"#)).is_err());
        assert!(scene(&p(r#"{"packAssets":["logo.png","Font.ttf"]}"#)).is_ok());
        assert!(scene(&p(r#"{"packAssets":["../show.json"]}"#)).is_err());
        assert!(scene(&p(r#"{"packAssets":[1]}"#)).is_err());
        assert!(scene(&p(r#"{"packAssets":"x"}"#)).is_err());
        assert!(scene(&p(r#"{"nodes":[{"type":"rect","props":5}]}"#)).is_err());
        assert!(scene(&p(r#"{"nodes":[{"props":{}}]}"#)).is_err());
        let mut deep = String::from(r#"{"nodes":["#);
        for _ in 0..70 {
            deep.push_str(r#"{"type":"group","children":["#);
        }
        for _ in 0..70 {
            deep.push_str("]}");
        }
        deep.push_str("]}");
        assert!(scene(&p(&deep)).is_err());
    }

    #[test]
    fn rundown_and_macro_rules() {
        assert!(rundown(&p(r#"{"name":"R","items":[{"id":"a","page":"p1","note":"n"},{"id":"b","page":"p2"}]}"#)).is_ok());
        assert!(rundown(&p(r#"{"items":[{"id":"a","page":"p1"},{"id":"a","page":"p2"}]}"#)).is_err());
        assert!(rundown(&p(r#"{"items":[{"page":"p1"}]}"#)).is_err());
        assert!(macro_(&p(r#"{"name":"M","steps":[{"cmd":"TAKE 1001","delay":0},{"cmd":"OUT main","delay":2.5}]}"#)).is_ok());
        assert!(macro_(&p(r#"{"steps":[{"cmd":"MACRO inne"}]}"#)).is_err());
        assert!(macro_(&p(r#"{"steps":[{"cmd":""}]}"#)).is_err());
        assert!(macro_(&p(r#"{"steps":[{"cmd":"TAKE 1","delay":-1}]}"#)).is_err());
    }

    #[test]
    fn field_rules_and_word_replace() {
        assert!(scene(&p(r#"{"fieldMeta":{"t":{"label":"Tytuł","maxChars":80,"softMax":60,"maxLines":2,"oneLine":false,"upper":true,"typo":true,"tab":1,"help":"podpowiedź"}}}"#)).is_ok());
        assert!(scene(&p(r#"{"fieldMeta":{"t":{"maxChars":0}}}"#)).is_err());
        assert!(scene(&p(r#"{"fieldMeta":{"t":{"softMax":90,"maxChars":80}}}"#)).is_err());
        assert!(scene(&p(r#"{"fieldMeta":{"t":{"upper":"tak"}}}"#)).is_err());
        assert!(scene(&p(r#"{"fieldMeta":{"t":5}}"#)).is_err());
        assert!(scene(&p(r#"{"fieldMeta":{"t":{"type":"select","options":["a","b"],"default":"a"}}}"#)).is_ok());
        assert!(word_replace(&p(r#"{"@p":["Prezydent","Premier"],"..m":["Minister"]}"#)).is_ok());
        assert!(word_replace(&p(r#"{"za dlugi klucz":["x"]}"#)).is_err());
        assert!(word_replace(&p(r#"{"a b":["x"]}"#)).is_err());
        assert!(word_replace(&p(r#"{"@p":[]}"#)).is_err());
        assert!(word_replace(&p(r#"{"@p":[""]}"#)).is_err());
        assert!(word_replace(&p("[]")).is_err());
    }

    #[test]
    fn markers_and_dock() {
        assert!(scene(&p(r#"{"markers":[{"t":0.5,"name":"a"}],"dock":[{"target":"layer:back","dx":0,"dy":-90,"dur":0.6}]}"#)).is_ok());
        assert!(scene(&p(r#"{"markers":[{"t":-1}]}"#)).is_err());
        assert!(scene(&p(r#"{"markers":"x"}"#)).is_err());
        assert!(scene(&p(r#"{"dock":[{"target":"zle"}]}"#)).is_err());
        assert!(scene(&p(r#"{"dock":[{"target":"layer:"}]}"#)).is_err());
        assert!(scene(&p(r#"{"dock":[{"target":"page:3001","dx":99999}]}"#)).is_err());
        assert!(scene(&p(r#"{"dock":[{"target":"scene:Ticker","dur":-1}]}"#)).is_err());
    }

    #[test]
    fn anim_preset_rules() {
        assert!(anim_presets(&p(r#"[{"id":"a","name":"Wjazd","duration":0.5,"tracks":{"x":[{"t":0,"v":-100},{"t":0.5,"v":0,"e":"out"}]}}]"#)).is_ok());
        assert!(anim_presets(&p(r#"[{"id":"a","name":"x","duration":0.5,"tracks":{"x":[{"t":-1,"v":0}]}}]"#)).is_err());
        assert!(anim_presets(&p(r#"[{"id":"a","name":"x","duration":0.5}]"#)).is_err());
        assert!(anim_presets(&p(r#"{}"#)).is_err());
    }

    #[test]
    fn ticker_limits() {
        assert!(scene(&p(r#"{"nodes":[{"type":"ticker","props":{"text":"a\nb","catColors":"PILNE=#e00000; INFO=#0050c0"}}]}"#)).is_ok());
        let many = vec!["x"; crate::ticker::MAX_ITEMS + 1].join("\\n");
        assert!(scene(&p(&format!(r#"{{"nodes":[{{"type":"ticker","props":{{"text":"{many}"}}}}]}}"#))).is_err());
        assert!(scene(&p(r#"{"nodes":[{"type":"ticker","props":{"catColors":"PILNE"}}]}"#)).is_err());
        assert!(scene(&p(r#"{"nodes":[{"type":"text","props":{"textFit":"squeeze-shrink","squeezeMin":0.3}}]}"#)).is_ok());
        assert!(scene(&p(r#"{"nodes":[{"type":"text","props":{"textFit":"squeeze-each","maxLines":2}}]}"#)).is_ok());
        assert!(scene(&p(r#"{"nodes":[{"type":"text","props":{"textFit":"rozciągnij"}}]}"#)).is_err());
        assert!(scene(&p(r#"{"nodes":[{"type":"text","props":{"textFit":3}}]}"#)).is_err());
        assert!(scene(&p(r#"{"nodes":[{"type":"text","props":{"squeezeMin":1.5}}]}"#)).is_err());
        assert!(scene(&p(r#"{"nodes":[{"type":"text","props":{"squeezeMin":"0.5"}}]}"#)).is_err());
        assert!(scene(&p(r#"{"nodes":[{"type":"text","props":{"textFit":"squeeze-lines","lineBreak":"fill","fillSqueeze":0.9}}]}"#)).is_ok());
        assert!(scene(&p(r#"{"nodes":[{"type":"text","props":{"lineBreak":"wrap"}}]}"#)).is_err());
        assert!(scene(&p(r#"{"nodes":[{"type":"text","props":{"fillSqueeze":2}}]}"#)).is_err());
        assert!(scene(&p(r#"{"fieldMeta":{"news":{"maxItems":20,"cats":["PILNE","SPORT"]}}}"#)).is_ok());
        assert!(scene(&p(r#"{"fieldMeta":{"news":{"cats":"PILNE"}}}"#)).is_err());
        assert!(scene(&p(r#"{"fieldMeta":{"news":{"maxItems":0}}}"#)).is_err());
    }

    #[test]
    fn values_are_scalar_only() {
        assert!(values(&p(r#"{"a":"x","b":1,"c":true,"d":null}"#)).is_ok());
        assert!(values(&p(r#"{"a":{"b":1}}"#)).is_err());
        assert!(values(&p(r#"{"a":[1]}"#)).is_err());
        assert!(values(&p("[]")).is_err());
    }
}
