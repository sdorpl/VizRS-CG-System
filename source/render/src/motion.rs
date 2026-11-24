//! Ruch węzła (`node.motion`) – odpowiednik `web/motion.js` dla renderera natywnego: krzywe GSAP, presety wejścia
//! i wyjścia, składanie, sekwencja „po węźle”. Stan składa się z właściwościami węzła tak jak w `renderer.js`:
//! przesunięcie, obrót, pochylenie i rozmycie się dodają, skala i krycie mnożą, przycięcie – część wspólna.
//!
//! Natywnie: presety, które dają się wyrazić przekształceniem 2D, kryciem, przycięciem prostokątnym i rozmyciem.
//! Reszta (odwrócenia 3D z perspektywą, maska ukośna, żaluzja, glitch i rozdzielenie RGB, efekty tekstu, pętle uwagi,
//! efekty tła) – `unsupported` zgłasza ją, a strona wraca do Chromium (zasada „czego nie umie, zgłasza”).

use serde_json::Value;
use std::collections::HashMap;
use std::f64::consts::PI;

/// Stan ruchu: przesunięcie (px), skala, obrót i pochylenie (°), krycie, punkt odniesienia (ułamki pola),
/// widoczny obszar `[l, t, r, b]` (ułamki pola), rozmycie (px).
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub tx: f64,
    pub ty: f64,
    pub sx: f64,
    pub sy: f64,
    pub rot: f64,
    pub skew: f64,
    pub opacity: f64,
    pub origin: Option<[f64; 2]>,
    pub clip: Option<[f64; 4]>,
    pub blur: f64,
}

impl Default for State {
    fn default() -> Self {
        State { tx: 0.0, ty: 0.0, sx: 1.0, sy: 1.0, rot: 0.0, skew: 0.0, opacity: 1.0, origin: None, clip: None, blur: 0.0 }
    }
}

// ---------------------------------------------------------------- krzywe (jak `ease` w motion.js)
fn back_out(x: f64, s: f64) -> f64 {
    let t = x - 1.0;
    t * t * ((s + 1.0) * t + s) + 1.0
}
fn back_in(x: f64, s: f64) -> f64 {
    x * x * ((s + 1.0) * x - s)
}
fn bounce_out(mut x: f64) -> f64 {
    let (n1, d1) = (7.5625, 2.75);
    if x < 1.0 / d1 {
        n1 * x * x
    } else if x < 2.0 / d1 {
        x -= 1.5 / d1;
        n1 * x * x + 0.75
    } else if x < 2.5 / d1 {
        x -= 2.25 / d1;
        n1 * x * x + 0.9375
    } else {
        x -= 2.625 / d1;
        n1 * x * x + 0.984375
    }
}
fn elastic_out(x: f64) -> f64 {
    if x <= 0.0 {
        0.0
    } else if x >= 1.0 {
        1.0
    } else {
        2f64.powf(-10.0 * x) * ((x * 10.0 - 0.75) * (2.0 * PI / 3.0)).sin() + 1.0
    }
}
fn pow_out(x: f64, p: f64) -> f64 {
    1.0 - (1.0 - x).powf(p)
}
fn pow_in(x: f64, p: f64) -> f64 {
    x.powf(p)
}
fn pow_in_out(x: f64, p: f64) -> f64 {
    if x < 0.5 {
        0.5 * (2.0 * x).powf(p)
    } else {
        1.0 - 0.5 * (2.0 * (1.0 - x)).powf(p)
    }
}
fn cubic_bezier(args: &str, x: f64) -> Option<f64> {
    if args.contains("1.56") || args.contains("0.64") {
        return Some(back_out(x, 1.7));
    }
    if args.contains("-0.55") {
        return Some(back_in(x, 1.7));
    }
    let v: Vec<f64> = args.split(',').map(|s| s.trim().parse::<f64>().unwrap_or(f64::NAN)).collect();
    if v.len() != 4 || v.iter().any(|n| !n.is_finite()) || !(0.0..=1.0).contains(&v[0]) || !(0.0..=1.0).contains(&v[2]) {
        return None;
    }
    let (x1, y1, x2, y2) = (v[0], v[1], v[2], v[3]);
    let (cx, cy) = (3.0 * x1, 3.0 * y1);
    let (bx, by) = (3.0 * (x2 - x1) - cx, 3.0 * (y2 - y1) - cy);
    let (ax, ay) = (1.0 - cx - bx, 1.0 - cy - by);
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..30 {
        let t = (lo + hi) / 2.0;
        if ((ax * t + bx) * t + cx) * t < x {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) / 2.0;
    Some(((ay * t + by) * t + cy) * t)
}

/// Krzywa o nazwie GSAP (`power2.out`, `back.out(1.7)`, `bounce.out`, `cubic-bezier(…)`, aliasy) dla x ∈ [0, 1].
pub fn ease(name: &str, x: f64) -> f64 {
    let x = if x.is_finite() { x.clamp(0.0, 1.0) } else { 0.0 };
    if let Some(args) = name.strip_prefix("cubic-bezier(") {
        return cubic_bezier(args.trim_end_matches(')'), x).unwrap_or_else(|| pow_out(x, 2.0));
    }
    if name == "linear" || name == "none" {
        return x;
    }
    let name = match name {
        "" => "power2.out",
        "ease-in" => "power2.in",
        "ease-out" => "power2.out",
        "ease-in-out" => "power2.inOut",
        "spring" => "back.out(1.4)",
        "spring-soft" => "back.out(0.8)",
        "spring-hard" => "back.out(2.2)",
        "overshoot" => "back.out(1.7)",
        "broadcast" => "power3.out",
        "back-in" => "back.in",
        "back-out" => "back.out",
        "bounce-out" => "bounce.out",
        "elastic-out" => "elastic.out",
        n => n,
    };
    let (kind, arg) = match name.split_once('(') {
        Some((k, rest)) => (k, rest.trim_end_matches(')').parse::<f64>().ok()),
        None => (name, None),
    };
    match kind {
        "power1.out" | "quad.out" => pow_out(x, 2.0),
        "power1.in" | "quad.in" => pow_in(x, 2.0),
        "power1.inOut" | "quad.inOut" => pow_in_out(x, 2.0),
        "sine.out" => (x * PI / 2.0).sin(),
        "sine.in" => 1.0 - (x * PI / 2.0).cos(),
        "sine.inOut" => -((PI * x).cos() - 1.0) / 2.0,
        "power2.out" | "cubic.out" => pow_out(x, 3.0),
        "power2.in" | "cubic.in" => pow_in(x, 3.0),
        "power2.inOut" | "cubic.inOut" => pow_in_out(x, 3.0),
        "power3.out" | "quart.out" => pow_out(x, 4.0),
        "power3.in" | "quart.in" => pow_in(x, 4.0),
        "power3.inOut" | "quart.inOut" => pow_in_out(x, 4.0),
        "power4.out" | "quint.out" => pow_out(x, 5.0),
        "power4.in" | "quint.in" => pow_in(x, 5.0),
        "power4.inOut" | "quint.inOut" => pow_in_out(x, 5.0),
        "expo.out" => {
            if x >= 1.0 {
                1.0
            } else {
                1.0 - 2f64.powf(-10.0 * x)
            }
        }
        "expo.in" => {
            if x <= 0.0 {
                0.0
            } else {
                2f64.powf(10.0 * x - 10.0)
            }
        }
        "back.out" => back_out(x, arg.unwrap_or(1.70158)),
        "back.in" => back_in(x, arg.unwrap_or(1.70158)),
        "back.inOut" => {
            let s = arg.unwrap_or(1.70158) * 1.525;
            if x < 0.5 {
                (2.0 * x).powi(2) * ((s + 1.0) * 2.0 * x - s) / 2.0
            } else {
                ((2.0 * x - 2.0).powi(2) * ((s + 1.0) * (x * 2.0 - 2.0) + s) + 2.0) / 2.0
            }
        }
        "bounce.out" => bounce_out(x),
        "bounce.in" => 1.0 - bounce_out(1.0 - x),
        "elastic.out" => elastic_out(x),
        "elastic.in" => 1.0 - elastic_out(1.0 - x),
        _ => pow_out(x, 3.0),
    }
}

// ---------------------------------------------------------------- presety
const SLIDE_X: f64 = 1920.0;
const SLIDE_Y: f64 = 1080.0;

/// Presety wejścia/wyjścia, które renderer natywny rysuje.
pub const NATIVE_TYPES: &[&str] = &[
    "none",
    "",
    "slide",
    "fade",
    "zoom",
    "zoom-in",
    "pop",
    "wipe",
    "wipe-y",
    "clip",
    "blur",
    "rotate",
    "skew",
    "drop",
    "slide-overshoot",
    "split-reveal",
    "bar-grow",
];

fn clip_inset(dir: &str, a: f64) -> [f64; 4] {
    match dir {
        "right" => [a, 0.0, 1.0, 1.0],
        "top" => [0.0, 0.0, 1.0, 1.0 - a],
        "bottom" => [0.0, a, 1.0, 1.0],
        _ => [0.0, 0.0, 1.0 - a, 1.0],
    }
}
fn clip_center(dir: &str, a: f64) -> [f64; 4] {
    let h = (1.0 - a) * 0.5;
    if dir == "top" || dir == "bottom" {
        [0.0, 0.5 - h, 1.0, 0.5 + h]
    } else {
        [0.5 - h, 0.0, 0.5 + h, 1.0]
    }
}
fn slide_vec(dir: &str) -> (f64, f64) {
    match dir {
        "right" => (SLIDE_X, 0.0),
        "top" => (0.0, -SLIDE_Y),
        "bottom" => (0.0, SLIDE_Y),
        _ => (-SLIDE_X, 0.0),
    }
}
fn opposite(dir: &str) -> &'static str {
    match dir {
        "left" => "right",
        "right" => "left",
        "top" => "bottom",
        _ => "top",
    }
}

/// Stan wejścia przy postępie p ∈ [0, 1] (0 – przed wejściem, 1 – na miejscu); `stateIn` w motion.js.
pub fn state_in(kind: &str, dir: &str, easing: &str, p: f64) -> State {
    let mut t = State::default();
    let m = ease(easing, p);
    match kind {
        "none" | "" => {}
        "fade" => t.opacity = m,
        "zoom" => {
            t.opacity = m;
            t.sx = 0.8 + 0.2 * m;
            t.sy = t.sx;
        }
        "zoom-in" => {
            t.opacity = m;
            t.sx = 1.15 - 0.15 * m;
            t.sy = t.sx;
        }
        "wipe" => {
            t.origin = Some([if dir == "right" { 1.0 } else { 0.0 }, 0.5]);
            t.opacity = m;
            t.sx = m.max(0.0001);
        }
        "wipe-y" => {
            t.origin = Some([0.5, if dir == "bottom" { 1.0 } else { 0.0 }]);
            t.opacity = m;
            t.sy = m.max(0.0001);
        }
        "clip" => t.clip = Some(clip_inset(dir, 1.0 - m)),
        "blur" => {
            t.opacity = m;
            t.blur = 24.0 * (1.0 - m);
        }
        "pop" => {
            let k = ease("back.out(2)", p);
            t.opacity = k.clamp(0.0, 1.0);
            t.sx = 0.3 + 0.7 * k;
            t.sy = t.sx;
        }
        "drop" => {
            let k = ease("bounce.out", p);
            let (dx, dy) = match dir {
                "bottom" => (0.0, 160.0),
                "left" => (-160.0, 0.0),
                "right" => (160.0, 0.0),
                _ => (0.0, -160.0),
            };
            t.opacity = k.clamp(0.0, 1.0);
            t.tx = dx * (1.0 - k);
            t.ty = dy * (1.0 - k);
        }
        "rotate" => {
            let r = dir == "right";
            t.origin = Some([if r { 1.0 } else { 0.0 }, 1.0]);
            t.opacity = m;
            t.rot = if r { 90.0 } else { -90.0 } * (1.0 - m);
        }
        "skew" => {
            let r = dir == "right";
            t.opacity = m;
            t.tx = if r { SLIDE_X } else { -SLIDE_X } * (1.0 - m);
            t.skew = if r { 25.0 } else { -25.0 } * (1.0 - m);
        }
        "slide-overshoot" => {
            let k = ease("back.out(1.2)", p);
            let (dx, dy) = slide_vec(dir);
            t.opacity = k.clamp(0.0, 1.0);
            t.tx = dx * (1.0 - k);
            t.ty = dy * (1.0 - k);
        }
        "split-reveal" => t.clip = Some(clip_center(dir, 1.0 - m)),
        "bar-grow" => {
            t.origin = Some([0.5, 0.5]);
            t.sy = (0.04 + 0.96 * m).max(0.0001);
        }
        _ => {
            let (dx, dy) = slide_vec(dir);
            t.opacity = m;
            t.tx = dx * (1.0 - m);
            t.ty = dy * (1.0 - m);
        }
    }
    t
}

/// Stan wyjścia przy postępie p ∈ [0, 1] (0 – na miejscu, 1 – schowany); `stateOut` w motion.js.
pub fn state_out(kind: &str, dir: &str, easing: &str, p: f64) -> State {
    let mut t = State::default();
    let m = ease(easing, p);
    match kind {
        "none" | "" => t.opacity = 0.0,
        "fade" => t.opacity = 1.0 - m,
        "zoom" => {
            t.opacity = 1.0 - m;
            t.sx = 1.0 - 0.2 * m;
            t.sy = t.sx;
        }
        "zoom-in" => {
            t.opacity = 1.0 - m;
            t.sx = 1.0 + 0.15 * m;
            t.sy = t.sx;
        }
        "wipe" => {
            t.origin = Some([if dir == "left" { 0.0 } else { 1.0 }, 0.5]);
            t.opacity = 1.0 - m;
            t.sx = (1.0 - m).max(0.0001);
        }
        "wipe-y" => {
            t.origin = Some([0.5, if dir == "top" { 0.0 } else { 1.0 }]);
            t.opacity = 1.0 - m;
            t.sy = (1.0 - m).max(0.0001);
        }
        "clip" => t.clip = Some(clip_inset(opposite(dir), m)),
        "blur" => {
            t.opacity = 1.0 - m;
            t.blur = 24.0 * m;
        }
        "pop" => {
            let k = ease("back.in(2)", p);
            t.opacity = (1.0 - k).clamp(0.0, 1.0);
            t.sx = 1.0 - 0.7 * k;
            t.sy = t.sx;
        }
        "drop" => {
            let k = ease("power2.in", p);
            let (dx, dy) = match dir {
                "top" => (0.0, -120.0),
                "left" => (-120.0, 0.0),
                "right" => (120.0, 0.0),
                _ => (0.0, 120.0),
            };
            t.opacity = 1.0 - k;
            t.tx = dx * k;
            t.ty = dy * k;
        }
        "rotate" => {
            let r = dir == "right";
            t.origin = Some([if r { 1.0 } else { 0.0 }, 1.0]);
            t.opacity = 1.0 - m;
            t.rot = if r { 90.0 } else { -90.0 } * m;
        }
        "skew" => {
            let r = dir == "right";
            t.opacity = 1.0 - m;
            t.tx = if r { SLIDE_X } else { -SLIDE_X } * m;
            t.skew = if r { -25.0 } else { 25.0 } * m;
        }
        "slide-overshoot" => {
            let k = ease("back.in(1.2)", p);
            let (dx, dy) = slide_vec(dir);
            t.opacity = (1.0 - k).clamp(0.0, 1.0);
            t.tx = dx * k;
            t.ty = dy * k;
        }
        "split-reveal" => t.clip = Some(clip_center(dir, m)),
        "bar-grow" => {
            t.origin = Some([0.5, 0.5]);
            t.sy = (1.0 - 0.96 * m).max(0.0001);
        }
        _ => {
            let (dx, dy) = slide_vec(dir);
            t.opacity = 1.0 - m;
            t.tx = dx * m;
            t.ty = dy * m;
        }
    }
    t
}

fn num(v: Option<&Value>, d: f64) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().filter(|x| x.is_finite()).unwrap_or(d),
        Some(Value::String(s)) if !s.trim().is_empty() => s.trim().parse::<f64>().ok().filter(|x| x.is_finite()).unwrap_or(d),
        _ => d,
    }
}
fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}
fn kind(spec: Option<&Value>) -> Option<&str> {
    spec.and_then(|x| x.get("type")).and_then(Value::as_str).filter(|t| !t.is_empty())
}
pub fn spec_dur(spec: &Value) -> f64 {
    num(spec.get("dur"), 0.5).max(0.0)
}
pub fn spec_delay(spec: &Value) -> f64 {
    num(spec.get("delay"), 0.0).max(0.0)
}

fn eval_in(spec: &Value, t: f64, shift: f64) -> State {
    let Some(k) = kind(Some(spec)) else { return State::default() };
    let d = spec_delay(spec) + shift;
    if k == "none" {
        let mut st = State::default();
        if t < d {
            st.opacity = 0.0;
        }
        return st;
    }
    if t < d {
        return state_in(k, s(spec, "dir"), s(spec, "ease"), 0.0);
    }
    let dur = spec_dur(spec);
    state_in(k, s(spec, "dir"), s(spec, "ease"), if dur <= 0.0 { 1.0 } else { ((t - d) / dur).clamp(0.0, 1.0) })
}
fn eval_out(spec: &Value, to: f64) -> State {
    let Some(k) = kind(Some(spec)) else { return State::default() };
    if to < spec_delay(spec) {
        return State::default();
    }
    let dur = spec_dur(spec);
    state_out(k, s(spec, "dir"), s(spec, "ease"), if dur <= 0.0 { 1.0 } else { ((to - spec_delay(spec)) / dur).clamp(0.0, 1.0) })
}

/// Złożenie stanów (`compose` w motion.js).
pub fn compose(a: &State, b: &State) -> State {
    let mut r = a.clone();
    r.tx += b.tx;
    r.ty += b.ty;
    r.rot += b.rot;
    r.skew += b.skew;
    r.blur += b.blur;
    r.sx *= b.sx;
    r.sy *= b.sy;
    r.opacity *= b.opacity;
    r.origin = b.origin.or(a.origin);
    if let Some(bc) = b.clip {
        r.clip = Some(match a.clip {
            Some(ac) => [ac[0].max(bc[0]), ac[1].max(bc[1]), ac[2].min(bc[2]), ac[3].min(bc[3])],
            None => bc,
        });
    }
    r
}

/// Stan ruchu węzła w chwili `t` sceny; `out_start` – ostatni stop (albo ∞), `shift` – przesunięcie sekwencji.
pub fn motion_at(m: &Value, t: f64, out_start: f64, shift: f64) -> State {
    let mut st = match m.get("in") {
        Some(spec) if kind(Some(spec)).is_some() => eval_in(spec, t.min(out_start), shift),
        _ => State::default(),
    };
    if let Some(spec) = m.get("out").filter(|x| kind(Some(x)).is_some()) {
        if t > out_start {
            st = compose(&st, &eval_out(spec, t - out_start));
        }
    }
    st
}

/// Punkt odniesienia z `motion.origin` (gdy preset go nie ustala).
pub fn origin_of(m: &Value) -> Option<[f64; 2]> {
    let a = m.get("origin")?.as_array()?;
    Some([num(a.first(), 0.5), num(a.get(1), 0.5)])
}

/// Koniec wejścia węzła (s od początku sceny), bez efektów tekstu (te zgłasza `unsupported`).
fn in_end(m: &Value, shift: f64) -> f64 {
    match m.get("in") {
        Some(spec) if kind(Some(spec)).is_some() => {
            shift + spec_delay(spec) + if kind(Some(spec)) == Some("none") { 0.0 } else { spec_dur(spec) }
        }
        _ => 0.0,
    }
}

/// Przesunięcia sekwencji („po węźle”, `motion.after`) – `sequenceShifts` w motion.js; cykle i brakujące węzły → 0.
pub fn sequence_shifts<'a>(nodes: impl Iterator<Item = (&'a str, Option<&'a Value>)>) -> HashMap<String, f64> {
    let by_id: HashMap<&str, Option<&Value>> = nodes.collect();
    let mut out: HashMap<String, f64> = HashMap::new();
    fn shift(id: &str, by_id: &HashMap<&str, Option<&Value>>, out: &mut HashMap<String, f64>, busy: &mut Vec<String>) -> f64 {
        if let Some(v) = out.get(id) {
            return *v;
        }
        let after = by_id.get(id).copied().flatten().and_then(|m| m.get("after")).and_then(Value::as_str).unwrap_or("");
        if after.is_empty() || !by_id.contains_key(after) || busy.iter().any(|b| b == id) {
            out.insert(id.to_string(), 0.0);
            return 0.0;
        }
        busy.push(id.to_string());
        let prev = shift(after, by_id, out, busy);
        let s = by_id.get(after).copied().flatten().map_or(0.0, |m| in_end(m, prev));
        busy.pop();
        out.insert(id.to_string(), s);
        s
    }
    let ids: Vec<&str> = by_id.keys().copied().collect();
    let mut busy = Vec::new();
    for id in ids {
        shift(id, &by_id, &mut out, &mut busy);
    }
    out
}

/// Czego z ruchu węzła renderer natywny nie narysuje (puste – wszystko natywnie).
pub fn unsupported(m: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for k in ["in", "out"] {
        if let Some(t) = kind(m.get(k)) {
            if !NATIVE_TYPES.contains(&t) {
                out.push(format!("ruch {k}: {t}"));
            }
        }
    }
    for k in ["textIn", "textOut"] {
        if kind(m.get(k)).is_some_and(|t| t != "none") {
            out.push(format!("ruch {k} (efekt tekstu)"));
        }
    }
    if kind(m.get("loop")).is_some_and(|t| t != "none") {
        out.push("ruch: pętla uwagi".into());
    }
    if m.get("bg").and_then(Value::as_str).is_some_and(|b| !b.is_empty() && b != "none") {
        out.push("ruch: efekt tła".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn krzywe_jak_w_motion_js() {
        assert!(close(ease("linear", 0.3), 0.3));
        assert!(close(ease("power2.out", 0.5), 1.0 - 0.5f64.powi(3)));
        assert!(close(ease("", 0.5), ease("power2.out", 0.5)));
        assert!(close(ease("power2.inOut", 0.25), 0.5 * 0.5f64.powi(3)));
        assert!(close(ease("bounce.out", 1.0), 1.0) && close(ease("elastic.out", 0.0), 0.0));
        assert!(ease("back.out(1.7)", 0.6) > 1.0, "przeskok");
        assert!(close(ease("cubic-bezier(0,0,1,1)", 0.4), 0.4) || (ease("cubic-bezier(0,0,1,1)", 0.4) - 0.4).abs() < 1e-6);
        assert!(close(ease("nieznana", 0.5), ease("power2.out", 0.5)));
    }

    #[test]
    fn wejscie_wyjscie_i_skladanie() {
        let m =
            json!({"in":{"type":"slide","dir":"left","dur":1,"delay":0.5,"ease":"linear"},"out":{"type":"fade","dur":1,"ease":"linear"}});
        let a = motion_at(&m, 0.0, 3.0, 0.0);
        assert!(close(a.tx, -1920.0) && close(a.opacity, 0.0), "przed wejściem: poza kadrem");
        let b = motion_at(&m, 1.0, 3.0, 0.0);
        assert!(close(b.tx, -960.0) && close(b.opacity, 0.5));
        let c = motion_at(&m, 2.0, 3.0, 0.0);
        assert_eq!(c, State::default());
        let d = motion_at(&m, 3.5, 3.0, 0.0);
        assert!(close(d.opacity, 0.5) && close(d.tx, 0.0), "wyjście od ostatniego stopu");
        // sekwencja: b wchodzi po a (a: opóźnienie 0,5 + 1 s)
        let mb = json!({"in":{"type":"fade","dur":1},"after":"a"});
        let sh = sequence_shifts([("a", Some(&m)), ("b", Some(&mb))].into_iter());
        assert!(close(sh["b"], 1.5) && close(sh["a"], 0.0));
        let cyc = json!({"after":"y"});
        let cyc2 = json!({"after":"x"});
        let sh = sequence_shifts([("x", Some(&cyc)), ("y", Some(&cyc2))].into_iter());
        assert!(sh.values().all(|v| v.is_finite()));
    }

    #[test]
    fn co_zostaje_w_chromium() {
        assert!(unsupported(&json!({"in":{"type":"wipe"},"out":{"type":"clip"},"origin":[0,0]})).is_empty());
        let u = unsupported(&json!({"in":{"type":"flip"},"textIn":{"type":"chars-up"},"loop":{"type":"pulse"},"bg":"glow"}));
        assert_eq!(u.len(), 4, "{u:?}");
        assert!(unsupported(&json!({"in":{"type":"none"},"loop":{"type":"none"},"bg":"none"})).is_empty());
    }
}
