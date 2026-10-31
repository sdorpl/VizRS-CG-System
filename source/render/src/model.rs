//! Model sceny jak w `web/renderer.js`: właściwości z wartościami domyślnymi (`PROPS`), klucze animacji
//! (`trackValue`, krzywe `EASE`), pola strony (`propAt`), warianty (`showIfOk`) i kolory CSS.
//! Semantyka musi zostać zgodna z rendererem przeglądarkowym – zmiana tam = zmiana tutaj (DOKUMENTACJA §10.1).

use serde_json::{Map, Value};

/// Typ właściwości (jak `t` w `PROPS`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Num,
    Color,
    Bool,
    Str,
}

/// Wartość domyślna i typ właściwości – odpowiednik tabeli `PROPS` z `renderer.js`.
pub fn prop_def(prop: &str) -> (Kind, Value) {
    use Kind::*;
    let n = |v: f64| (Num, Value::from(v));
    let s = |v: &str| (Str, Value::from(v));
    let c = |v: &str| (Color, Value::from(v));
    let b = |v: bool| (Bool, Value::from(v));
    match prop {
        "x" | "y" | "rotation" | "clipL" | "clipR" | "clipT" | "clipB" | "blur" | "skewX" | "blink" => n(0.0),
        "w" => n(400.0),
        "h" => n(100.0),
        "scale" | "opacity" => n(1.0),
        "anchorX" => s("left"),
        "anchorY" => s("top"),
        "follow" => s(""),
        "followAxis" => s("w"),
        "followPad" => n(40.0),
        "followMin" => n(0.0),
        "color" => c("#ffffff"),
        "color2" => c(""),
        "gradAngle" => n(90.0),
        "radius" | "borderWidth" | "shadow" => n(0.0),
        "borderColor" => c("#ffffff"),
        "text" => s("Tekst"),
        "font" => s("Inter"),
        "size" => n(48.0),
        "weight" => s("700"),
        "italic" | "upper" => b(false),
        "spacing" => n(0.0),
        "lineHeight" => n(1.15),
        "align" => s("left"),
        "valign" => s("middle"),
        "textFit" => s("shrink"),
        "squeezeMin" => n(0.6),
        "lineBreak" => s("balance"),
        "fillSqueeze" => n(0.9),
        "maxLines" => n(1.0),
        "padX" => n(0.0),
        "hlColor" => c("#e30613"),
        "hlText" => c("#ffffff"),
        "cycle" => n(0.0),
        "qrBg" => c("#ffffff"),
        "src" => s(""),
        "fitMode" => s("contain"),
        "loop" => b(true),
        "mode" => s("time"),
        "format" => s("HH:mm:ss"),
        "seconds" => n(300.0),
        "target" => s("2027-01-01 00:00:00"),
        "tz" => s("Europe/Warsaw"),
        "speed" => n(140.0),
        "sep" => s("   •   "),
        "sepColor" => c(""),
        "tickerMode" => s("crawl"),
        "sepStyle" => s("text"),
        "crawlFrom" => s("right"),
        "label" | "catColors" => s(""),
        "gleam" => n(0.0),
        "hold" => n(5.0),
        "catW" => n(170.0),
        "labelSize" => n(0.0),
        "labelWeight" => s(""),
        "catBg" => c("#c8102e"),
        "catFg" => c("#ffffff"),
        _ => s(""),
    }
}

/// Czy typ węzła ma autofollow (`FOLLOW` w `TYPES`).
pub fn has_follow(node_type: &str) -> bool {
    matches!(node_type, "group" | "rect" | "ellipse" | "image")
}

/// `parseFloat` z JS: liczba z początku tekstu, inaczej NaN.
pub fn parse_float(s: &str) -> f64 {
    let s = s.trim_start();
    let b = s.as_bytes();
    let mut end = 0;
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_e = false;
    while end < b.len() {
        let ch = b[end];
        let ok = match ch {
            b'0'..=b'9' => {
                seen_digit = true;
                true
            }
            b'+' | b'-' => end == 0 || (seen_e && matches!(b[end - 1], b'e' | b'E')),
            b'.' if !seen_dot && !seen_e => {
                seen_dot = true;
                true
            }
            b'e' | b'E' if seen_digit && !seen_e => {
                seen_e = true;
                true
            }
            _ => false,
        };
        if !ok {
            break;
        }
        end += 1;
    }
    let mut t = &s[..end];
    // „1e” albo „1e-” bez wykładnika – parseFloat bierze samo „1”
    while !t.is_empty() && t.parse::<f64>().is_err() {
        t = &t[..t.len() - 1];
    }
    if s.starts_with("Infinity") || s.starts_with("+Infinity") {
        return f64::INFINITY;
    }
    if s.starts_with("-Infinity") {
        return f64::NEG_INFINITY;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// Liczba z wartości JSON tak jak `+v` w JS (pusty tekst = 0, zły = NaN).
pub fn num_of(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Null => 0.0,
        _ => f64::NAN,
    }
}

/// `String(v)` z JS dla wartości z show.
pub fn str_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js_number(n.as_f64().unwrap_or(0.0)),
        other => other.to_string(),
    }
}

/// Liczba jak `String(n)` w JS (bez „.0” przy całkowitych).
pub fn js_number(n: f64) -> String {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e21 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Prawdziwość jak `coerce(v, 'bool')` i jak test `if (v)` w rendererze.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Null => false,
        _ => true,
    }
}

/// `coerce` z renderera: wartość pola strony do typu właściwości.
fn coerce(v: &Value, kind: Kind) -> Value {
    match kind {
        Kind::Num => {
            let n = match v {
                Value::Number(_) => num_of(v),
                _ => parse_float(&str_of(v)),
            };
            Value::from(if n.is_nan() { 0.0 } else { n })
        }
        Kind::Bool => Value::from(matches!(v, Value::Bool(true)) || matches!(v.as_str(), Some("true" | "1")) || v.as_f64() == Some(1.0)),
        _ => Value::from(str_of(v)),
    }
}

/// Krzywe kluczy (`EASE`); nieznana nazwa = `inout`.
pub fn ease(name: &str, u: f64) -> f64 {
    match name {
        "linear" => u,
        "in" => u * u * u,
        "out" => 1.0 - (1.0 - u).powi(3),
        "back" => {
            let c1 = 1.70158;
            let c3 = c1 + 1.0;
            1.0 + c3 * (u - 1.0).powi(3) + c1 * (u - 1.0).powi(2)
        }
        "hold" => 0.0,
        _ => {
            if u < 0.5 {
                4.0 * u * u * u
            } else {
                1.0 - (-2.0 * u + 2.0).powi(3) / 2.0
            }
        }
    }
}

/// Kolor RGBA (kanały 0–255, alfa 0–1) jak `parseColor`; obsługuje też nazwy CSS.
pub type Rgba = [f64; 4];

pub fn parse_color(c: &str) -> Option<Rgba> {
    let c = c.trim();
    if let Some(h) = c.strip_prefix('#') {
        let h: String = if h.len() == 3 || h.len() == 4 { h.chars().flat_map(|x| [x, x]).collect() } else { h.to_string() };
        let n = |i: usize| h.get(i..i + 2).and_then(|s| u8::from_str_radix(s, 16).ok()).map(f64::from);
        if h.len() != 6 && h.len() != 8 {
            return None;
        }
        let a = if h.len() >= 8 { n(6)? / 255.0 } else { 1.0 };
        return Some([n(0)?, n(2)?, n(4)?, a]);
    }
    let lower = c.to_ascii_lowercase();
    if let Some(inner) = lower.strip_prefix("rgba(").or_else(|| lower.strip_prefix("rgb(")).and_then(|s| s.strip_suffix(')')) {
        let p: Vec<f64> = inner
            .split([',', ' ', '/'])
            .filter(|s| !s.is_empty())
            .map(|s| if let Some(pc) = s.strip_suffix('%') { parse_float(pc) / 100.0 * 255.0 } else { parse_float(s) })
            .collect();
        if p.len() < 3 || p.iter().any(|x| x.is_nan()) {
            return None;
        }
        let a = p.get(3).copied().unwrap_or(1.0);
        return Some([p[0].clamp(0.0, 255.0), p[1].clamp(0.0, 255.0), p[2].clamp(0.0, 255.0), a.clamp(0.0, 1.0)]);
    }
    named_color(&lower)
}

fn named_color(name: &str) -> Option<Rgba> {
    let hex = match name {
        "transparent" => return Some([0.0, 0.0, 0.0, 0.0]),
        "black" => "000000",
        "white" => "ffffff",
        "red" => "ff0000",
        "green" => "008000",
        "lime" => "00ff00",
        "blue" => "0000ff",
        "yellow" => "ffff00",
        "orange" => "ffa500",
        "gold" => "ffd700",
        "gray" | "grey" => "808080",
        "silver" => "c0c0c0",
        "navy" => "000080",
        "purple" => "800080",
        "magenta" | "fuchsia" => "ff00ff",
        "cyan" | "aqua" => "00ffff",
        "maroon" => "800000",
        "teal" => "008080",
        "olive" => "808000",
        "pink" => "ffc0cb",
        "crimson" => "dc143c",
        "darkred" => "8b0000",
        "darkblue" => "00008b",
        "darkgreen" => "006400",
        "darkgray" | "darkgrey" => "a9a9a9",
        "lightgray" | "lightgrey" => "d3d3d3",
        "whitesmoke" => "f5f5f5",
        "dimgray" | "dimgrey" => "696969",
        "royalblue" => "4169e1",
        "dodgerblue" => "1e90ff",
        "deepskyblue" => "00bfff",
        "tomato" => "ff6347",
        "orangered" => "ff4500",
        "firebrick" => "b22222",
        "steelblue" => "4682b4",
        _ => return None,
    };
    parse_color(&format!("#{hex}"))
}

/// `rgba(...)` jak w rendererze (mieszanie kolorów w kluczach).
fn rgba_css(c: Rgba) -> String {
    let a = (c[3] * 1000.0).round() / 1000.0;
    format!("rgba({},{},{},{})", c[0].round(), c[1].round(), c[2].round(), js_number(a))
}

fn mix(a: &Value, b: &Value, u: f64, kind: Kind) -> Value {
    match kind {
        Kind::Num => {
            let (x, y) = (num_of(a), num_of(b));
            Value::from(x + (y - x) * u)
        }
        Kind::Color => match (parse_color(&str_of(a)), parse_color(&str_of(b))) {
            (Some(ca), Some(cb)) => Value::from(rgba_css([0, 1, 2, 3].map(|i| ca[i] + (cb[i] - ca[i]) * u))),
            _ => {
                if u < 0.5 {
                    a.clone()
                } else {
                    b.clone()
                }
            }
        },
        _ => {
            if u < 1.0 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

/// `trackValue`: wartość ścieżki kluczy `[{t, v, e}]` w czasie `t`.
pub fn track_value(track: &[Value], t: f64, kind: Kind) -> Option<Value> {
    let key_t = |k: &Value| k.get("t").map(num_of).unwrap_or(0.0);
    let key_v = |k: &Value| k.get("v").cloned().unwrap_or(Value::Null);
    let first = track.first()?;
    if t <= key_t(first) {
        return Some(key_v(first));
    }
    let last = track.last()?;
    if t >= key_t(last) {
        return Some(key_v(last));
    }
    for pair in track.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if t < key_t(b) {
            let span = key_t(b) - key_t(a);
            let e = a.get("e").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("inout");
            let u = if span <= 0.0 { 1.0 } else { ease(e, (t - key_t(a)) / span) };
            return Some(mix(&key_v(a), &key_v(b), u, kind));
        }
    }
    Some(key_v(last))
}

/// Węzeł sceny (część potrzebna do rysowania).
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub hidden: bool,
    pub props: Map<String, Value>,
    pub anim: Map<String, Value>,
    pub fields: Map<String, Value>,
    pub show_if: Option<(String, String)>,
    pub motion: Option<Value>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn from_json(v: &Value) -> Node {
        let obj = |k: &str| v.get(k).and_then(Value::as_object).cloned().unwrap_or_default();
        let show_if = v.get("showIf").and_then(|c| {
            let field = c.get("field").map(str_of).filter(|f| !f.is_empty())?;
            Some((field, c.get("value").map(str_of).unwrap_or_default()))
        });
        let motion = v.get("motion").filter(|m| m.is_object()).cloned();
        Node {
            id: v.get("id").map(str_of).unwrap_or_default(),
            name: v.get("name").map(str_of).unwrap_or_default(),
            kind: v.get("type").map(str_of).unwrap_or_default(),
            hidden: v.get("hidden").is_some_and(truthy),
            props: obj("props"),
            anim: obj("anim"),
            fields: obj("fields"),
            show_if,
            motion,
            children: v.get("children").and_then(Value::as_array).map(|c| c.iter().map(Node::from_json).collect()).unwrap_or_default(),
        }
    }

    /// Nazwa do komunikatów (jak `n.name || n.id`).
    pub fn label(&self) -> &str {
        if self.name.is_empty() {
            &self.id
        } else {
            &self.name
        }
    }

    /// `propAt`: wartość właściwości w czasie `t` z polami strony `values`.
    pub fn prop_at(&self, prop: &str, t: f64, values: &Map<String, Value>) -> Value {
        let (kind, default) = prop_def(prop);
        if let Some(fid) = self.fields.get(prop).map(str_of).filter(|f| !f.is_empty()) {
            if let Some(v) = values.get(&fid).filter(|v| !v.is_null()) {
                return coerce(v, kind);
            }
        }
        if let Some(track) = self.anim.get(prop).and_then(Value::as_array).filter(|t| !t.is_empty()) {
            if let Some(v) = track_value(track, t, kind) {
                return v;
            }
        }
        match self.props.get(prop) {
            Some(v) => v.clone(),
            None => {
                if prop == "textFit" && self.props.get("fit") == Some(&Value::Bool(false)) {
                    return Value::from("none");
                }
                if prop == "textFit" && self.kind == "ticker" {
                    return Value::from("squeeze-shrink");
                }
                default
            }
        }
    }
}

/// Scena (szablon z Artist).
#[derive(Debug, Clone)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub duration: f64,
    pub stops: Vec<f64>,
    pub nodes: Vec<Node>,
    pub field_meta: Map<String, Value>,
    pub dock: Vec<Value>,
}

impl Scene {
    pub fn from_json(v: &Value) -> Scene {
        let positive = |k: &str, d: f64| v.get(k).map(num_of).filter(|x| *x > 0.0 && x.is_finite()).unwrap_or(d);
        let mut stops: Vec<f64> =
            v.get("stops").and_then(Value::as_array).map(|s| s.iter().map(num_of).filter(|x| x.is_finite()).collect()).unwrap_or_default();
        stops.sort_by(|a, b| a.total_cmp(b));
        Scene {
            id: v.get("id").map(str_of).unwrap_or_default(),
            name: v.get("name").map(str_of).unwrap_or_default(),
            width: positive("width", 1920.0),
            height: positive("height", 1080.0),
            duration: v.get("duration").map(num_of).filter(|x| x.is_finite()).unwrap_or(0.0),
            stops,
            nodes: v.get("nodes").and_then(Value::as_array).map(|n| n.iter().map(Node::from_json).collect()).unwrap_or_default(),
            field_meta: v.get("fieldMeta").and_then(Value::as_object).cloned().unwrap_or_default(),
            dock: v.get("dock").and_then(Value::as_array).cloned().unwrap_or_default(),
        }
    }

    pub fn label(&self) -> &str {
        if self.name.is_empty() {
            &self.id
        } else {
            &self.name
        }
    }

    /// `showIfOk`: czy węzeł jest widoczny w wariancie strony.
    pub fn show_if_ok(&self, node: &Node, values: &Map<String, Value>) -> bool {
        let Some((field, want)) = &node.show_if else { return true };
        let v = match values.get(field).filter(|v| !v.is_null()) {
            Some(v) => str_of(v),
            None => {
                let meta = self.field_meta.get(field);
                meta.and_then(|m| m.get("default"))
                    .filter(|d| !d.is_null())
                    .map(str_of)
                    .or_else(|| meta.and_then(|m| m.get("options")).and_then(Value::as_array).and_then(|o| o.first()).map(str_of))
                    .unwrap_or_default()
            }
        };
        want.split(',').map(str::trim).any(|w| w == v)
    }
}

/// Kolejność warstw (`layerZ`): back, main, front i `z<N>_nazwa`.
pub fn layer_z(name: &str) -> f64 {
    match name {
        "back" => 1000.0,
        "main" => 2000.0,
        "front" => 3000.0,
        _ => {
            let n = name.strip_prefix('z').and_then(|r| {
                let digits: String = r.chars().take_while(char::is_ascii_digit).collect();
                (!digits.is_empty() && digits.len() <= 2 && r[digits.len()..].starts_with('_'))
                    .then(|| digits.parse::<f64>().unwrap_or(0.0))
            });
            2500.0 + n.unwrap_or(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn eases_match_renderer() {
        assert_eq!(ease("linear", 0.3), 0.3);
        assert!((ease("inout", 0.25) - 0.0625).abs() < 1e-12);
        assert!((ease("out", 0.5) - 0.875).abs() < 1e-12);
        assert_eq!(ease("hold", 0.9), 0.0);
        assert!((ease("back", 1.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn tracks_interpolate_numbers_and_colors() {
        let tr = vec![json!({"t":0,"v":0,"e":"linear"}), json!({"t":1,"v":100})];
        assert_eq!(track_value(&tr, 0.5, Kind::Num), Some(json!(50.0)));
        assert_eq!(track_value(&tr, -1.0, Kind::Num), Some(json!(0)));
        assert_eq!(track_value(&tr, 2.0, Kind::Num), Some(json!(100)));
        let tc = vec![json!({"t":0,"v":"#000000","e":"linear"}), json!({"t":1,"v":"#ffffff"})];
        assert_eq!(track_value(&tc, 0.5, Kind::Color), Some(json!("rgba(128,128,128,1)")));
    }

    #[test]
    fn page_values_override_props_and_keys() {
        let n = Node::from_json(
            &json!({"id":"a","type":"text","props":{"text":"x","size":"40"},"fields":{"text":"f1"},"anim":{"x":[{"t":0,"v":5}]}}),
        );
        let mut v = Map::new();
        assert_eq!(n.prop_at("text", 0.0, &v), json!("x"));
        v.insert("f1".into(), json!(12));
        assert_eq!(n.prop_at("text", 0.0, &v), json!("12"));
        assert_eq!(n.prop_at("x", 0.0, &v), json!(5));
        assert_eq!(n.prop_at("w", 0.0, &v), json!(400.0));
        let t = Node::from_json(&json!({"type":"ticker"}));
        assert_eq!(t.prop_at("textFit", 0.0, &v), json!("squeeze-shrink"));
    }

    #[test]
    fn colors_parse_like_css() {
        assert_eq!(parse_color("#fff"), Some([255.0, 255.0, 255.0, 1.0]));
        assert_eq!(parse_color("#ff000080").map(|c| (c[3] * 255.0).round()), Some(128.0));
        assert_eq!(parse_color("rgba(1,2,3,.5)"), Some([1.0, 2.0, 3.0, 0.5]));
        assert_eq!(parse_color("white"), Some([255.0, 255.0, 255.0, 1.0]));
        assert_eq!(parse_color("nonsense"), None);
    }

    #[test]
    fn show_if_uses_variant_default() {
        let s = Scene::from_json(&json!({"fieldMeta":{"v":{"type":"select","options":["a","b"]}},"nodes":[]}));
        let n = Node::from_json(&json!({"showIf":{"field":"v","value":"a, c"}}));
        assert!(s.show_if_ok(&n, &Map::new()));
        let mut vals = Map::new();
        vals.insert("v".into(), json!("b"));
        assert!(!s.show_if_ok(&n, &vals));
    }

    #[test]
    fn parse_float_like_js() {
        assert_eq!(parse_float("12px"), 12.0);
        assert_eq!(parse_float(" -3.5e2x"), -350.0);
        assert!(parse_float("abc").is_nan());
        assert_eq!(parse_float("1e"), 1.0);
    }

    #[test]
    fn layers_order() {
        assert!(layer_z("back") < layer_z("main"));
        assert_eq!(layer_z("z07_logo"), 2507.0);
        assert_eq!(layer_z("bug"), 2500.0);
    }
}
