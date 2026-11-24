//! Odtwarzacz sceny – odpowiednik `ScenePlayer` z `renderer.js`: stop pointy (TAKE / CONTINUE / OUT), czas sceny,
//! pola strony, dwa przebiegi `_apply` (treść i pomiar tekstu, potem autofollow i geometria) i ticker `crawl`.
//! Zamiast DOM powstaje lista rysowania (`display`). Czego renderer natywny nie umie, trafia do `issues`.

use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use serde_json::{Map, Value};
use tiny_skia::Transform;

use crate::clock;
use crate::display::{mul, Color, DNode, Fill, Paint, Radius, TextBlock};
use crate::model::{has_follow, num_of, parse_color, str_of, truthy, Node, Scene};
use crate::res::Resources;
use crate::text::{self, Fit, Layout, Style};
use crate::ticker::{self, Item};

mod text_node;
mod ticker_node;

use self::ticker_node::*;

/// Czas klatki: zegar odtwarzania (ms, monotoniczny) i czas ścienny (ms od epoki – zegary `time`, `target`).
#[derive(Clone, Copy, Debug)]
pub struct Clock {
    pub now: f64,
    pub wall: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Idle,
    In,
    Cont,
    Out,
    Hold,
    Play,
}

/// Opcje wyjścia (`?fx=step` strony /output).
#[derive(Clone, Copy, Debug, Default)]
pub struct Output {
    pub step: bool,
}

/// Odcinek taśmy crawla: [plakietka] treść [separator].
struct Seg {
    item: Item,
    idx: usize,
    pos: f64,
    w: f64,
    node: Arc<DNode>,
}

/// Stan crawla (jak `it.tk`): przesunięcie, kursor doklejania, odcinki na taśmie.
struct Crawl {
    segs: VecDeque<Seg>,
    d: f64,
    cursor: f64,
    next_idx: usize,
    items: Vec<Item>,
    style_key: String,
    last: Option<f64>,
    fresh: bool,
}

#[derive(Default)]
struct NState {
    key: String,
    layout: Option<Arc<Layout>>,
    block: Option<Arc<TextBlock>>,
    mw: Option<f32>,
    mh: Option<f32>,
    fw: Option<f32>,
    fh: Option<f32>,
    vis: bool,
    w: f32,
    h: f32,
    fol: String,
    cy_op: f32,
    tk_raw: Option<(String, String)>,
    tk_items: Vec<Item>,
    tk_page: HashMap<String, String>,
    crawl: Option<Crawl>,
    slots: Option<Slots>,
    /// Kod QR: tekst i moduły (liczone przy zmianie tekstu).
    qr: Option<(Arc<str>, Option<QrCells>)>,
    /// Treść węzła w tej klatce (budowana w przebiegu 1).
    content: Vec<Paint>,
    extra: Vec<DNode>,
}

/// Krzywa ruchu powiązania (dock), pole reguły `ease` (jak `DOCK_EASES` w `renderer.js`):
/// `std` (domyślna) – cubic-bezier(.4,0,.2,1), `out` – cubic-bezier(.22,1,.36,1) (szybki start, miękkie hamowanie).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DockEase {
    #[default]
    Std,
    Out,
}

impl DockEase {
    pub fn of(v: Option<&Value>) -> DockEase {
        match v.and_then(Value::as_str) {
            Some("out") => DockEase::Out,
            _ => DockEase::Std,
        }
    }

    fn at(self, u: f64) -> f64 {
        match self {
            DockEase::Std => cubic_bezier(0.4, 0.0, 0.2, 1.0, u),
            DockEase::Out => cubic_bezier(0.22, 1.0, 0.36, 1.0, u),
        }
    }
}

/// Przesunięcie powiązania (dock): przejście `transform` z krzywą reguły (`DockEase`).
#[derive(Clone, Copy, Debug, Default)]
struct Dock {
    from: (f64, f64),
    to: (f64, f64),
    start: f64,
    dur: f64,
    ease: DockEase,
}

pub struct Player {
    pub scene: Scene,
    pub values: Map<String, Value>,
    pub t: f64,
    pub target: f64,
    pub playing: bool,
    pub mode: Mode,
    pub speed: f64,
    wall_start: f64,
    pub dead: bool,
    nodes: Vec<Node>,
    parents: Vec<Option<usize>>,
    states: Vec<NState>,
    by_name: HashMap<String, usize>,
    last_apply: Option<f64>,
    dock: Dock,
    /// Przesunięcie dock już ustawione: pierwsze (po TAKE, snapshot) bez przejścia.
    dock_set: bool,
    /// Czego nie da się narysować natywnie (scena, węzeł, powód) – liczone przy każdej klatce.
    pub issues: Vec<String>,
    /// Coś zmienia się z czasem także bez ruchu sceny (zegar, crawl, miganie…).
    pub live: bool,
    pub tree: Option<DNode>,
}

fn flatten(nodes: &[Node], parent: Option<usize>, out: &mut Vec<Node>, parents: &mut Vec<Option<usize>>) {
    for n in nodes {
        let i = out.len();
        out.push(n.clone());
        parents.push(parent);
        flatten(&n.children, Some(i), out, parents);
    }
}

fn color(v: &str) -> Option<Color> {
    parse_color(v).map(|c| [(c[0] / 255.0) as f32, (c[1] / 255.0) as f32, (c[2] / 255.0) as f32, c[3] as f32])
}

/// Obrys sceny w spoczynku (powiązania krawędzi): pierwszy stop point z wartościami strony, `[l, t, r, b]` w px kadru
/// (z `scene.x/y`, bez dock), jak `restRect` w `renderer.js`; `None` – nic nie rysuje.
pub fn rest_rect(scene: &Scene, values: &Map<String, Value>, res: &Resources) -> Option<[f64; 4]> {
    let clk = Clock { now: 0.0, wall: 0.0 };
    let mut p = Player::new(scene.clone(), values.clone(), clk);
    p.seek_stop(0);
    p.apply(clk, res, Output::default());
    let r = crate::raster::drawn_rect(p.tree.as_ref()?)?;
    Some(r.map(|v| (v * 100.0).round() / 100.0))
}

/// Czas animacji OUT sceny w ms (od ostatniego stop pointu do końca; bez wyjścia – 0), jak `outMs` w `renderer.js`.
pub fn out_ms(scene: &Scene) -> f64 {
    let last = scene.stops.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if last.is_finite() && scene.duration > last + 1e-3 {
        (scene.duration - last) * 1000.0
    } else {
        0.0
    }
}

/// Krzywa przejścia CSS `cubic-bezier(x1, y1, x2, y2)` w punkcie `u`.
pub fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, u: f64) -> f64 {
    let bez = |a: f64, b: f64, s: f64| 3.0 * a * s * (1.0 - s).powi(2) + 3.0 * b * s * s * (1.0 - s) + s.powi(3);
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if bez(x1, x2, mid) < u {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    bez(y1, y2, (lo + hi) / 2.0)
}

/// Ma znaczniki tekstu sformatowanego (`hasMarkup` z `richtext.js`).
pub fn has_markup(t: &str) -> bool {
    crate::richtext::has_markup(t)
}

fn weight_of(v: &Value) -> u16 {
    let s = str_of(v);
    match s.trim() {
        "normal" | "" => 400,
        "bold" => 700,
        "bolder" => 700,
        "lighter" => 300,
        x => x.parse::<f64>().ok().filter(|w| (1.0..=1000.0).contains(w)).map(|w| w as u16).unwrap_or(400),
    }
}

fn hash_str(parts: &[&str]) -> u64 {
    let mut h = DefaultHasher::new();
    for p in parts {
        p.hash(&mut h);
        0u8.hash(&mut h);
    }
    h.finish()
}

/// Lewa krawędź elementu tekstu w polu: flex z marginesem `padX`, wyrównanie wg `align`.
fn span_x(layout: &Layout, pad_x: f32, w: f32, align: &str) -> f32 {
    let cw = w - 2.0 * pad_x;
    pad_x
        + match align {
            "center" => (cw - layout.span_w) / 2.0,
            "right" => cw - layout.span_w,
            _ => 0.0,
        }
}

/// Jednostki `em` wysokości wiersza `normal` (A + D + odstęp) i linii bazowej względem środka pola.
fn normal_metrics(res: &Resources, style: &Style) -> (f32, f32) {
    let db = res.fonts.read().unwrap_or_else(|e| e.into_inner());
    let chain = db.chain(&style.font, style.weight, style.italic);
    let Some(&f) = chain.first() else { return (style.size * 1.2, style.size * 0.35) };
    let face = db.face(f);
    let k = style.size / face.upem;
    let (a, d, g) = ((face.ascent * k).round(), (face.descent * k).round(), (face.line_gap * k).round());
    // wiersz `normal` (A + D + odstęp) wyśrodkowany w pionie; połowa odstępu nad wierszem w dół do piksela (Blink)
    (a + d + g, -(a + d + g) / 2.0 + (g / 2.0).floor() + a)
}

impl Player {
    pub fn new(scene: Scene, values: Map<String, Value>, clk: Clock) -> Player {
        let mut p = Player {
            scene,
            values,
            t: 0.0,
            target: 0.0,
            playing: false,
            mode: Mode::Idle,
            speed: 1.0,
            wall_start: clk.now,
            dead: false,
            nodes: Vec::new(),
            parents: Vec::new(),
            states: Vec::new(),
            by_name: HashMap::new(),
            last_apply: None,
            dock: Dock::default(),
            dock_set: false,
            issues: Vec::new(),
            live: false,
            tree: None,
        };
        p.build();
        p
    }

    fn build(&mut self) {
        self.nodes.clear();
        self.parents.clear();
        flatten(&self.scene.nodes, None, &mut self.nodes, &mut self.parents);
        self.states = (0..self.nodes.len()).map(|_| NState { cy_op: 1.0, ..NState::default() }).collect();
        self.by_name.clear();
        for (i, n) in self.nodes.iter().enumerate() {
            if !n.name.is_empty() {
                self.by_name.entry(n.name.clone()).or_insert(i);
            }
            self.by_name.insert(n.id.clone(), i);
        }
        self.last_apply = None;
    }

    pub fn stops(&self) -> &[f64] {
        &self.scene.stops
    }

    pub fn take(&mut self, clk: Clock) {
        self.t = 0.0;
        self.wall_start = clk.now;
        for s in &mut self.states {
            s.crawl = None;
        }
        self.target = self.scene.stops.first().copied().unwrap_or(self.scene.duration);
        self.mode = Mode::In;
        self.playing = true;
    }

    pub fn cont(&mut self) {
        if let Some(next) = self.scene.stops.iter().copied().find(|s| *s > self.t + 1e-4) {
            self.target = next;
            self.playing = true;
            self.mode = Mode::Cont;
        }
    }

    pub fn out(&mut self) {
        let last = self.scene.stops.last().copied();
        match last {
            Some(ls) if self.scene.duration > ls + 1e-3 => {
                if self.t < ls {
                    self.t = ls;
                }
                self.target = self.scene.duration;
                self.playing = true;
                self.mode = Mode::Out;
            }
            _ => self.dead = true,
        }
    }

    pub fn seek_stop(&mut self, i: usize) {
        let s = &self.scene.stops;
        self.t = if s.is_empty() { self.scene.duration } else { s[i.min(s.len() - 1)] };
        self.playing = false;
        self.mode = Mode::Hold;
    }

    pub fn seek(&mut self, t: f64) {
        self.t = t;
        self.playing = false;
    }

    pub fn update(&mut self, values: Map<String, Value>) {
        self.values = values;
        for s in &mut self.states {
            s.key.clear();
        }
    }

    pub fn set_scene(&mut self, scene: Scene) {
        self.scene = scene;
        self.build();
    }

    pub fn set_dock(&mut self, dx: f64, dy: f64, dur: f64, ease: DockEase, now: f64) {
        // pierwsze ustawienie: od razu na miejscu (grafika wchodząca przy celu na antenie nie dojeżdża) – jak `renderer.js`
        if !self.dock_set {
            self.dock_set = true;
            self.dock = Dock { from: (dx, dy), to: (dx, dy), start: now, dur: 0.0, ease };
            return;
        }
        if (dx, dy) == self.dock.to {
            return;
        }
        let cur = self.dock_at(now);
        self.dock = Dock { from: if dur > 0.0 { cur } else { (dx, dy) }, to: (dx, dy), start: now, dur: dur.max(0.0), ease };
    }

    /// Przesunięcie dock w chwili `now` (ms zegara klatek).
    pub(crate) fn dock_at(&self, now: f64) -> (f64, f64) {
        let d = self.dock;
        if d.dur <= 0.0 {
            return d.to;
        }
        let u = ((now - d.start) / 1000.0 / d.dur).clamp(0.0, 1.0);
        let k = d.ease.at(u);
        (d.from.0 + (d.to.0 - d.from.0) * k, d.from.1 + (d.to.1 - d.from.1) * k)
    }

    /// Krok pętli animacji (`tick`): czas sceny o `dt` s, potem budowa listy rysowania.
    pub fn tick(&mut self, dt: f64, clk: Clock, res: &Resources, out: Output) {
        if self.dead {
            return;
        }
        if self.playing {
            self.t = self.target.min(self.t + dt * self.speed);
            if self.t >= self.target - 1e-6 {
                self.t = self.target;
                self.playing = false;
                if self.mode == Mode::Out {
                    self.dead = true;
                    self.tree = None;
                    return;
                }
            }
        }
        self.apply(clk, res, out);
    }

    fn prop(&self, i: usize, k: &str) -> Value {
        self.nodes[i].prop_at(k, self.t, &self.values)
    }

    fn num(&self, i: usize, k: &str) -> f32 {
        let v = num_of(&self.prop(i, k));
        if v.is_finite() {
            v as f32
        } else {
            0.0
        }
    }

    /// Maska warstwy (`mask` = nazwa węzła z tej samej grupy): prostokąt węzła maski (x / y / w / h z punktem zaczepienia,
    /// bez obrotu i skali) przycina węzeł i – wcięcia w % jego ramki, łączone z clipL… (większe wygrywa). Jak renderer.js.
    fn mask_clip(&self, i: usize, w: f32, h: f32, left: f32, top: f32, c: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        let name = self.txt(i, "mask");
        let name = name.trim();
        if name.is_empty() || w <= 0.0 || h <= 0.0 {
            return c;
        }
        let Some(&j) = self.by_name.get(name) else { return c };
        if j == i || self.parents[j] != self.parents[i] {
            return c;
        }
        let (tw, th) = (self.num(j, "w").max(0.0), self.num(j, "h").max(0.0));
        let (tax, tay) = (self.txt(j, "anchorX"), self.txt(j, "anchorY"));
        let mx = self.num(j, "x")
            - if tax == "right" {
                tw
            } else if tax == "center" {
                tw / 2.0
            } else {
                0.0
            };
        let my = self.num(j, "y")
            - if tay == "bottom" {
                th
            } else if tay == "middle" {
                th / 2.0
            } else {
                0.0
            };
        let cp = |v: f32| v.clamp(0.0, 100.0);
        let (cl, cr, ct, cb) = c;
        (
            cl.max(cp((mx - left) / w * 100.0)),
            cr.max(cp((left + w - mx - tw) / w * 100.0)),
            ct.max(cp((my - top) / h * 100.0)),
            cb.max(cp((top + h - my - th) / h * 100.0)),
        )
    }

    fn txt(&self, i: usize, k: &str) -> String {
        str_of(&self.prop(i, k))
    }

    fn issue(&mut self, i: usize, why: &str) {
        let msg = format!("{} / {}: {why}", self.scene.label(), self.nodes[i].label());
        if !self.issues.contains(&msg) {
            self.issues.push(msg);
        }
    }

    /// Font węzła dostępny natywnie (wbudowany, z assets albo systemowy).
    fn font_ok(&mut self, i: usize, font: &str, res: &Resources) -> bool {
        if crate::font::is_font_file(font) {
            if !res.asset_font(font) {
                self.issue(i, &format!("font {font} jeszcze niewczytany"));
                return false;
            }
            return true;
        }
        let db = res.fonts.read().unwrap_or_else(|e| e.into_inner());
        if db.has_family(font) {
            return true;
        }
        let system = crate::font::SYSTEM_FAMILIES.iter().any(|f| f.eq_ignore_ascii_case(font));
        drop(db);
        if !system {
            self.issue(i, &format!("font „{font}” spoza wbudowanych (Chromium pobrałby go z Google Fonts)"));
        }
        system
    }

    fn style(&self, i: usize, size: f32, tnum: bool) -> Style {
        Style {
            font: self.txt(i, "font"),
            weight: weight_of(&self.prop(i, "weight")),
            italic: truthy(&self.prop(i, "italic")),
            size,
            spacing: self.num(i, "spacing"),
            tnum,
            hl: None,
        }
    }

    /// `_apply`: oba przebiegi i lista rysowania tej sceny.
    #[allow(clippy::needless_range_loop)]
    pub fn apply(&mut self, clk: Clock, res: &Resources, out: Output) {
        let now = clk.now;
        let dt = match self.last_apply {
            Some(l) => ((now - l) / 1000.0).clamp(0.0, 0.25),
            None => 10.0,
        };
        self.last_apply = Some(now);
        self.issues.clear();
        self.live = self.playing || self.dock.dur > 0.0 && now - self.dock.start < self.dock.dur * 1000.0 + 50.0;
        let epoch = res.epoch();
        let n = self.nodes.len();

        // przebieg 1: widoczność i treść (pomiar tekstów dla autofollow)
        for i in 0..n {
            let node = self.nodes[i].clone();
            let parent_vis = self.parents[i].is_none_or(|p| self.states[p].vis);
            let vis = parent_vis && !node.hidden && self.scene.show_if_ok(&node, &self.values);
            self.states[i].vis = vis;
            self.states[i].content.clear();
            self.states[i].extra.clear();
            if !vis {
                continue;
            }
            if let Some(m) = &node.motion {
                for why in crate::motion::unsupported(m) {
                    self.issue(i, &why);
                }
            }
            self.states[i].w = self.num(i, "w").max(0.0);
            self.states[i].h = self.num(i, "h").max(0.0);
            self.states[i].fol = if has_follow(&node.kind) { self.txt(i, "follow").trim().to_string() } else { String::new() };
            match node.kind.as_str() {
                "rect" | "ellipse" => self.box_content(i, &node),
                "image" => self.image_content(i),
                "text" | "clock" => self.text_content(i, &node, clk, res, epoch),
                "ticker" => self.ticker_content(i, clk, res, epoch, dt),
                "qr" => self.qr_content(i),
                "group" | "" => {}
                other => {
                    let msg = format!("nieznany typ węzła {other}");
                    self.issue(i, &msg);
                }
            }
        }

        // przebieg 2: geometria (autofollow, punkty zaczepienia, przekształcenia)
        let k = 1.0 - (-dt * 12.0).exp();
        // ruch (node.motion): wyjście od ostatniego stop pointu, „po węźle” przesuwa wejście – jak `_apply` w renderer.js
        let out_start = match self.scene.stops.last() {
            Some(&l) if self.scene.duration > l + 1e-3 => l,
            _ => f64::INFINITY,
        };
        let shifts = if self.nodes.iter().any(|n| n.motion.as_ref().is_some_and(|m| m.get("after").is_some())) {
            crate::motion::sequence_shifts(self.nodes.iter().map(|n| (n.id.as_str(), n.motion.as_ref())))
        } else {
            std::collections::HashMap::new()
        };
        let mut built: Vec<Option<DNode>> = (0..n).map(|_| None).collect();
        for i in 0..n {
            if !self.states[i].vis {
                continue;
            }
            let node = self.nodes[i].clone();
            let (mut w, mut h) = (self.states[i].w, self.states[i].h);
            let fol = self.states[i].fol.clone();
            if !fol.is_empty() {
                let (mut mw, mut mh, mut found) = (0.0f32, 0.0f32, false);
                for name in fol.split(',') {
                    if let Some(&t) = self.by_name.get(name.trim()) {
                        let st = &self.states[t];
                        if st.vis {
                            if let (Some(a), Some(b)) = (st.mw, st.mh) {
                                found = true;
                                mw = mw.max(a);
                                mh = mh.max(b);
                            }
                        }
                    }
                }
                if found {
                    let ax = self.txt(i, "followAxis");
                    let (pad, mn) = (self.num(i, "followPad"), self.num(i, "followMin"));
                    if ax.contains('w') {
                        w = mn.max(mw + pad);
                    }
                    if ax.contains('h') {
                        h = mn.max(mh + pad);
                    }
                }
                let st = &mut self.states[i];
                match (st.fw, st.fh) {
                    (Some(fw), Some(fh)) if k <= 0.99 => {
                        let (mut nw, mut nh) = (fw + (w - fw) * k as f32, fh + (h - fh) * k as f32);
                        if (w - nw).abs() < 0.4 {
                            nw = w;
                        }
                        if (h - nh).abs() < 0.4 {
                            nh = h;
                        }
                        if nw != w || nh != h {
                            self.live = true;
                        }
                        st.fw = Some(nw);
                        st.fh = Some(nh);
                    }
                    _ => {
                        st.fw = Some(w);
                        st.fh = Some(h);
                    }
                }
                w = st.fw.unwrap_or(w);
                h = st.fh.unwrap_or(h);
                // pole śledzące zmienia rozmiar: tło i ramka liczą się od nowa
                if matches!(node.kind.as_str(), "rect" | "ellipse") {
                    self.states[i].content.clear();
                    self.box_content(i, &node);
                }
            }
            let ax = self.txt(i, "anchorX");
            let ay = self.txt(i, "anchorY");
            let left = self.num(i, "x")
                - if ax == "right" {
                    w
                } else if ax == "center" {
                    w / 2.0
                } else {
                    0.0
                };
            let top = self.num(i, "y")
                - if ay == "bottom" {
                    h
                } else if ay == "middle" {
                    h / 2.0
                } else {
                    0.0
                };
            let mo = node.motion.as_ref().map(|m| {
                (crate::motion::motion_at(m, self.t, out_start, shifts.get(&node.id).copied().unwrap_or(0.0)), crate::motion::origin_of(m))
            });
            let mut op = self.num(i, "opacity") * self.states[i].cy_op * mo.as_ref().map_or(1.0, |(s, _)| s.opacity as f32);
            let bl = self.num(i, "blink");
            if bl > 0.0 {
                self.live = true;
                let c = ((now / 1000.0) * 2.0 * std::f64::consts::PI / f64::from(bl)).cos() as f32;
                op *= if out.step {
                    if c >= 0.0 {
                        1.0
                    } else {
                        0.15
                    }
                } else {
                    0.15 + 0.85 * (0.5 + 0.5 * c)
                };
            }
            let ms = mo.as_ref().map(|(s, _)| s.clone()).unwrap_or_default();
            let rot = self.num(i, "rotation") + ms.rot as f32;
            let sc = self.num(i, "scale");
            let sk = self.num(i, "skewX") + ms.skew as f32;
            let (sx, sy) = (sc * ms.sx as f32, sc * ms.sy as f32);
            let (tx, ty) = (ms.tx as f32, ms.ty as f32);
            // translate(left, top) · oś · translate(ruch) · rotate · scale · skewX · oś⁻¹ (oś: punkt odniesienia ruchu, inaczej środek)
            let mut m = Transform::from_translate(left, top);
            if rot != 0.0 || sx != 1.0 || sy != 1.0 || sk != 0.0 || tx != 0.0 || ty != 0.0 {
                let org = ms.origin.or(mo.as_ref().and_then(|(_, o)| *o)).unwrap_or([0.5, 0.5]);
                let (ox, oy) = (w * org[0] as f32, h * org[1] as f32);
                let r = Transform::from_rotate(rot);
                let s = Transform::from_scale(sx, sy);
                let kx = Transform::from_row(1.0, 0.0, sk.to_radians().tan(), 1.0, 0.0, 0.0);
                let local = mul(mul(mul(Transform::from_translate(tx, ty), r), s), kx);
                m = mul(mul(mul(m, Transform::from_translate(ox, oy)), local), Transform::from_translate(-ox, -oy));
            }
            let mut d = DNode::new(m, w, h);
            d.opacity = op.clamp(0.0, 1.0);
            let (cl, cr, ct, cb) = (self.num(i, "clipL"), self.num(i, "clipR"), self.num(i, "clipT"), self.num(i, "clipB"));
            let (mut cl, mut cr, mut ct, mut cb) = self.mask_clip(i, w, h, left + tx, top + ty, (cl, cr, ct, cb));
            if let Some(c) = ms.clip {
                cl = cl.max((c[0] * 100.0) as f32);
                ct = ct.max((c[1] * 100.0) as f32);
                cr = cr.max((100.0 - c[2] * 100.0) as f32);
                cb = cb.max((100.0 - c[3] * 100.0) as f32);
            }
            if cl != 0.0 || cr != 0.0 || ct != 0.0 || cb != 0.0 {
                d.clip = Some([ct, cr, cb, cl]);
            }
            d.blur = (self.num(i, "blur") + ms.blur as f32).max(0.0);
            let kind = node.kind.clone();
            if matches!(kind.as_str(), "ticker" | "image" | "qr") {
                d.overflow = Some(if kind == "ticker" { 0.0 } else { self.num(i, "radius").max(0.0) });
            }
            if kind == "rect" {
                let bw = self.num(i, "borderWidth").max(0.0);
                d.child_m = Transform::from_translate(bw, bw);
            }
            d.paints = std::mem::take(&mut self.states[i].content);
            d.children = std::mem::take(&mut self.states[i].extra);
            if d.opacity <= 0.001 {
                // visibility: hidden – nic z tego węzła ani dzieci nie widać (krycie grupy)
                d.paints.clear();
                d.children.clear();
                d.opacity = 0.0;
            }
            built[i] = Some(d);
        }
        // drzewo: dzieci w kolejności dokumentu
        for i in (0..n).rev() {
            let Some(p) = self.parents[i] else { continue };
            if let Some(child) = built[i].take() {
                if let Some(parent) = built[p].as_mut() {
                    if child.opacity > 0.0 {
                        parent.children.insert(0, child);
                    }
                }
            }
        }
        let mut root = DNode::new(Transform::identity(), self.scene.width as f32, self.scene.height as f32);
        root.overflow = Some(0.0);
        let (dx, dy) = self.dock_at(now);
        root.m = Transform::from_translate(dx as f32, dy as f32);
        // przesunięcie całej sceny (scene.x / scene.y) – wewnątrz przycięcia kadru, jak .vz-root w renderer.js
        let mut content = DNode::new(
            Transform::from_translate(self.scene.x as f32, self.scene.y as f32),
            self.scene.width as f32,
            self.scene.height as f32,
        );
        for i in 0..n {
            if self.parents[i].is_none() {
                if let Some(d) = built[i].take() {
                    if d.opacity > 0.0 {
                        content.children.push(d);
                    }
                }
            }
        }
        root.children.push(content);
        self.tree = Some(root);
    }

    /// Zaokrąglenie rogów jak `cornerRadii` w `renderer.js`: róg (`radiusTL`…) → boki (oba = róg eliptyczny,
    /// poziomo bok górny/dolny, pionowo lewy/prawy; jeden = jego wartość) → `radius`. Ujemne = brak nadpisania.
    fn radius_of(&self, i: usize) -> Radius {
        let r = self.num(i, "radius").max(0.0);
        let o = |k: &str| Some(self.num(i, k)).filter(|v| *v >= 0.0);
        let (t, rt, b, l) = (o("radiusT"), o("radiusR"), o("radiusB"), o("radiusL"));
        let mut any = t.is_some() || rt.is_some() || b.is_some() || l.is_some();
        let mut c = [[r, r]; 4];
        for (k, (name, sy, sx)) in
            [("radiusTL", t, l), ("radiusTR", t, rt), ("radiusBR", b, rt), ("radiusBL", b, l)].into_iter().enumerate()
        {
            c[k] = match (o(name), sy, sx) {
                (Some(v), _, _) => {
                    any = true;
                    [v, v]
                }
                (None, Some(a), Some(bb)) => [a, bb],
                (None, Some(v), None) | (None, None, Some(v)) => [v, v],
                (None, None, None) => [r, r],
            };
        }
        if any {
            Radius::Corners(c)
        } else {
            Radius::Px(r)
        }
    }

    fn box_content(&mut self, i: usize, node: &Node) {
        let ellipse = node.kind == "ellipse";
        let radius = if ellipse { Radius::Ellipse } else { self.radius_of(i) };
        let sh = self.num(i, "shadow");
        let mut paints = Vec::new();
        if sh > 0.0 {
            paints.push(Paint::BoxShadow { radius: radius.clone(), sigma: sh / 2.0, dy: sh / 3.0, color: [0.0, 0.0, 0.0, 0.5] });
        }
        let (c, c2) = (self.txt(i, "color"), self.txt(i, "color2"));
        let fill = if !c2.is_empty() {
            match (color(&c), color(&c2)) {
                (Some(a), Some(b)) => Some(Fill::Linear { angle: self.num(i, "gradAngle"), stops: vec![(0.0, a), (1.0, b)] }),
                _ => None,
            }
        } else {
            color(&c).map(Fill::Solid)
        };
        if let Some(fill) = fill {
            paints.push(Paint::Box { radius: radius.clone(), fill });
        }
        let bw = self.num(i, "borderWidth");
        if bw > 0.0 {
            if let Some(bc) = color(&self.txt(i, "borderColor")) {
                paints.push(Paint::Border { radius, width: bw, color: bc });
            }
        }
        self.states[i].content = paints;
    }

    /// Kod QR (`qrSvg`): moduły z `qr.rs`, kolory `color` i `qrBg`, zaokrąglenie i cień pola.
    fn qr_content(&mut self, i: usize) {
        let text = self.txt(i, "text");
        let (dark, light) = (color(&self.txt(i, "color")).unwrap_or([0.0, 0.0, 0.0, 1.0]), color(&self.txt(i, "qrBg")).unwrap_or([1.0; 4]));
        let r = self.radius_of(i);
        let sh = self.num(i, "shadow");
        if self.states[i].key != text || self.states[i].qr.is_none() {
            self.states[i].key = text.clone();
            self.states[i].qr = Some((Arc::from(text.as_str()), crate::qr::matrix(&text).map(Arc::new)));
        }
        let Some((t, cells)) = self.states[i].qr.clone() else { return };
        if cells.is_none() {
            self.issue(i, "kod QR: tekst za długi (Chromium pokazuje „za długi”)");
        }
        let mut paints = Vec::new();
        if sh > 0.0 {
            paints.push(Paint::BoxShadow { radius: r, sigma: sh / 2.0, dy: sh / 3.0, color: [0.0, 0.0, 0.0, 0.5] });
        }
        paints.push(Paint::Qr { text: t, cells, dark, light });
        self.states[i].content = paints;
    }

    fn image_content(&mut self, i: usize) {
        let src = self.txt(i, "src");
        let r = self.radius_of(i);
        let sh = self.num(i, "shadow");
        let mut paints = Vec::new();
        if sh > 0.0 {
            paints.push(Paint::BoxShadow { radius: r.clone(), sigma: sh / 2.0, dy: sh / 3.0, color: [0.0, 0.0, 0.0, 0.5] });
        }
        if !src.is_empty() {
            let remote = src.starts_with("http:") || src.starts_with("https:") || src.starts_with("data:") || src.starts_with('/');
            if remote || !crate::image::supported_name(&src) {
                self.issue(i, &format!("obraz {src} (format albo adres spoza assets)"));
            } else {
                paints.push(Paint::Image { src, fit: self.txt(i, "fitMode"), radius: r });
            }
        }
        self.states[i].content = paints;
    }
}

/// Moduły kodu QR (true = ciemny).
type QrCells = Arc<Vec<Vec<bool>>>;

/// Tekst bez znaczników (`stripMarkup`).
pub fn strip_markup(t: &str) -> String {
    crate::richtext::strip(t)
}

/// Czy ruch (`node.motion`) coś robi.
pub fn motion_used(m: &Value) -> bool {
    let Some(o) = m.as_object() else { return false };
    o.iter().any(|(k, v)| match k.as_str() {
        "origin" | "after" => false,
        "in" | "out" | "textIn" | "textOut" | "bg" => match v {
            Value::String(s) => !s.is_empty() && s != "none",
            Value::Object(x) => x.get("type").map(str_of).is_some_and(|t| !t.is_empty() && t != "none"),
            Value::Null => false,
            _ => true,
        },
        "loop" => v.get("type").map(str_of).is_some_and(|t| !t.is_empty() && t != "none"),
        _ => false,
    })
}

/// Statyczny przegląd sceny: co na pewno nie jest obsługiwane natywnie (bez rysowania).
pub fn check_scene(scene: &Scene, fonts: &crate::font::FontDb) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(nodes: &[Node], scene: &Scene, fonts: &crate::font::FontDb, out: &mut Vec<String>) {
        for n in nodes {
            let mut add = |why: String| {
                let msg = format!("{} / {}: {why}", scene.label(), n.label());
                if !out.contains(&msg) {
                    out.push(msg);
                }
            };
            for why in n.motion.as_ref().map(crate::motion::unsupported).unwrap_or_default() {
                add(why);
            }
            let p = |k: &str| n.prop_at(k, 0.0, &Map::new());
            match n.kind.as_str() {
                "image" => {
                    let src = str_of(&p("src"));
                    if !src.is_empty() && !crate::image::supported_name(&src) {
                        add(format!("obraz {src} (format)"));
                    }
                }
                "ticker" => {
                    let m = str_of(&p("tickerMode"));
                    if !matches!(m.as_str(), "crawl" | "roll" | "wipe" | "whip") {
                        add(format!("ticker w trybie {m}"));
                    }
                }
                _ => {}
            }
            if matches!(n.kind.as_str(), "text" | "clock" | "ticker") {
                let font = str_of(&p("font"));
                let known = crate::font::is_font_file(&font)
                    || fonts.has_family(&font)
                    || crate::font::SYSTEM_FAMILIES.iter().any(|f| f.eq_ignore_ascii_case(&font));
                if !known {
                    add(format!("font „{font}” spoza wbudowanych (Chromium pobrałby go z Google Fonts)"));
                }
            }
            walk(&n.children, scene, fonts, out);
        }
    }
    walk(&scene.nodes, scene, fonts, &mut out);
    out
}

/// Skrót listy rysowania sceny (porównanie klatek).
pub fn fingerprint(tree: &DNode, h: &mut impl Hasher) {
    tree.fingerprint(h);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_detection_matches_richtext() {
        assert!(has_markup("a [[b]]"));
        assert!(has_markup("{b}x{/b}"));
        assert!(has_markup("{C=#fff}x"));
        assert!(!has_markup("{bez} znaczników"));
        assert!(!has_markup("zwykły"));
        assert_eq!(strip_markup("a {b}b{/b} [[c]]"), "a b c");
    }

    #[test]
    fn bezier_ends() {
        assert!((cubic_bezier(0.4, 0.0, 0.2, 1.0, 0.0)).abs() < 1e-6);
        assert!((cubic_bezier(0.4, 0.0, 0.2, 1.0, 1.0) - 1.0).abs() < 1e-6);
        assert!(cubic_bezier(0.4, 0.0, 0.2, 1.0, 0.5) > 0.5);
    }

    #[test]
    fn motion_none_is_not_used() {
        assert!(!motion_used(&serde_json::json!({"in":"none","origin":[0,0]})));
        assert!(motion_used(&serde_json::json!({"in":{"type":"slide"}})));
        assert!(motion_used(&serde_json::json!({"loop":{"type":"breathe"}})));
    }

    #[test]
    fn stops_drive_take_continue_out() {
        let scene = Scene::from_json(&serde_json::json!({"duration":3,"stops":[1,2],"nodes":[]}));
        let clk = Clock { now: 0.0, wall: 0.0 };
        let mut p = Player::new(scene, Map::new(), clk);
        p.take(clk);
        assert_eq!(p.target, 1.0);
        p.t = 1.0;
        p.cont();
        assert_eq!(p.target, 2.0);
        p.t = 2.0;
        p.out();
        assert_eq!((p.mode, p.target), (Mode::Out, 3.0));
        let mut q = Player::new(Scene::from_json(&serde_json::json!({"duration":1,"stops":[1]})), Map::new(), clk);
        q.out();
        assert!(q.dead);
    }
}
