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

/// Przesunięcie powiązania (dock): przejście `transform` z krzywą cubic-bezier(.4,0,.2,1).
#[derive(Clone, Copy, Debug, Default)]
struct Dock {
    from: (f64, f64),
    to: (f64, f64),
    start: f64,
    dur: f64,
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

    pub fn set_dock(&mut self, dx: f64, dy: f64, dur: f64, now: f64) {
        if (dx, dy) == self.dock.to {
            return;
        }
        let cur = self.dock_at(now);
        self.dock = Dock { from: if dur > 0.0 { cur } else { (dx, dy) }, to: (dx, dy), start: now, dur: dur.max(0.0) };
    }

    fn dock_at(&self, now: f64) -> (f64, f64) {
        let d = self.dock;
        if d.dur <= 0.0 {
            return d.to;
        }
        let u = ((now - d.start) / 1000.0 / d.dur).clamp(0.0, 1.0);
        let k = cubic_bezier(0.4, 0.0, 0.2, 1.0, u);
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
                if motion_used(m) {
                    self.issue(i, "ruch na wzór BTB (motion)");
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
            let mut op = self.num(i, "opacity") * self.states[i].cy_op;
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
            let rot = self.num(i, "rotation");
            let sc = self.num(i, "scale");
            let sk = self.num(i, "skewX");
            // translate(left, top) · oś(środek) · rotate · scale · skewX · oś⁻¹
            let mut m = Transform::from_translate(left, top);
            if rot != 0.0 || sc != 1.0 || sk != 0.0 {
                let (ox, oy) = (w / 2.0, h / 2.0);
                let r = Transform::from_rotate(rot);
                let s = Transform::from_scale(sc, sc);
                let kx = Transform::from_row(1.0, 0.0, sk.to_radians().tan(), 1.0, 0.0, 0.0);
                let local = mul(mul(r, s), kx);
                m = mul(mul(mul(m, Transform::from_translate(ox, oy)), local), Transform::from_translate(-ox, -oy));
            }
            let mut d = DNode::new(m, w, h);
            d.opacity = op.clamp(0.0, 1.0);
            let (cl, cr, ct, cb) = (self.num(i, "clipL"), self.num(i, "clipR"), self.num(i, "clipT"), self.num(i, "clipB"));
            if cl != 0.0 || cr != 0.0 || ct != 0.0 || cb != 0.0 {
                d.clip = Some([ct, cr, cb, cl]);
            }
            d.blur = self.num(i, "blur").max(0.0);
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
        for i in 0..n {
            if self.parents[i].is_none() {
                if let Some(d) = built[i].take() {
                    if d.opacity > 0.0 {
                        root.children.push(d);
                    }
                }
            }
        }
        self.tree = Some(root);
    }

    fn box_content(&mut self, i: usize, node: &Node) {
        let ellipse = node.kind == "ellipse";
        let radius = if ellipse { Radius::Ellipse } else { Radius::Px(self.num(i, "radius").max(0.0)) };
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
        let r = self.num(i, "radius").max(0.0);
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
            paints.push(Paint::BoxShadow { radius: Radius::Px(r), sigma: sh / 2.0, dy: sh / 3.0, color: [0.0, 0.0, 0.0, 0.5] });
        }
        paints.push(Paint::Qr { text: t, cells, dark, light });
        self.states[i].content = paints;
    }

    fn image_content(&mut self, i: usize) {
        let src = self.txt(i, "src");
        let r = self.num(i, "radius").max(0.0);
        let sh = self.num(i, "shadow");
        let mut paints = Vec::new();
        if sh > 0.0 {
            paints.push(Paint::BoxShadow { radius: Radius::Px(r), sigma: sh / 2.0, dy: sh / 3.0, color: [0.0, 0.0, 0.0, 0.5] });
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

    fn text_content(&mut self, i: usize, node: &Node, clk: Clock, res: &Resources, epoch: u64) {
        let mut text = if node.kind == "clock" {
            self.live = true;
            let mode = self.txt(i, "mode");
            let el_s = (clk.now - self.wall_start) / 1000.0;
            let tz = self.txt(i, "tz");
            let secs = match mode.as_str() {
                "up" => el_s,
                "down" => f64::from(self.num(i, "seconds")) - el_s,
                "target" => {
                    let tg = clock::zone_time(&self.txt(i, "target"), &tz);
                    if tg.is_nan() {
                        0.0
                    } else {
                        (tg - clk.wall) / 1000.0
                    }
                }
                _ => 0.0,
            };
            let fmt = self.txt(i, "format");
            clock::fmt_clock(if fmt.is_empty() { "HH:mm:ss" } else { &fmt }, &mode, secs, &tz, clk.wall)
        } else {
            self.txt(i, "text")
        };
        self.states[i].cy_op = 1.0;
        let cyc = if node.kind == "text" { self.num(i, "cycle") } else { 0.0 };
        if cyc > 0.0 && text.contains('\n') {
            self.live = true;
            let lines: Vec<&str> = text.split('\n').filter(|l| !l.trim().is_empty()).collect();
            let el_s = (clk.now - self.wall_start) / 1000.0;
            let cyc = f64::from(cyc);
            let pick = if lines.is_empty() { "" } else { lines[(el_s / cyc).floor() as usize % lines.len()] };
            text = pick.to_string();
            let ph = el_s % cyc;
            let f = (cyc / 4.0).min(0.35);
            self.states[i].cy_op = (ph / f).min((cyc - ph) / f).clamp(0.0, 1.0) as f32;
        }
        if truthy(&self.prop(i, "upper")) {
            text = text.to_uppercase();
        }
        let font = self.txt(i, "font");
        // font z assets jeszcze się wczytuje: tekstu nie ma (`font-display: block`); font spoza wbudowanych –
        // zastępczy z łańcucha (jak Chromium bez Google Fonts), zgłoszony w `issues`
        if !self.font_ok(i, &font, res) && crate::font::is_font_file(&font) {
            return;
        }
        let size = self.num(i, "size");
        let pad_x = self.num(i, "padX");
        let w = self.states[i].w;
        let h = self.states[i].h;
        let align = self.txt(i, "align");
        let valign = self.txt(i, "valign");
        let fit_mode = self.txt(i, "textFit");
        let max_lines = self.num(i, "maxLines");
        let lh = self.num(i, "lineHeight");
        let sq_min = self.num(i, "squeezeMin");
        let line_break = self.txt(i, "lineBreak");
        let fill_sq = self.num(i, "fillSqueeze");
        let mut style = self.style(i, size, true);
        style.hl = Some((
            crate::text::css_color(&self.txt(i, "hlColor")).unwrap_or([0.0; 4]),
            crate::text::css_color(&self.txt(i, "hlText")).unwrap_or([1.0; 4]),
        ));
        let key = format!("{text}|{style:?}|{fit_mode}|{max_lines}|{sq_min}|{line_break}|{fill_sq}|{w}|{pad_x}|{lh}|{align}|{epoch}");
        if self.states[i].key != key || self.states[i].layout.is_none() {
            let fo = Fit {
                avail: w - 2.0 * pad_x,
                size,
                max_lines,
                mode: &fit_mode,
                align: &align,
                line_height: lh,
                min_squeeze: sq_min,
                line_break: &line_break,
                fill_squeeze: fill_sq,
            };
            let db = res.fonts.read().unwrap_or_else(|e| e.into_inner());
            let layout = if fit_mode == "squeeze-lines" || fit_mode == "squeeze-each" {
                text::fit_lines(&db, &style, &text, &fo)
            } else {
                text::fit_text(&db, &style, &text, &fo)
            };
            drop(db);
            // Blink: element z przekształceniem innym niż przesunięcie (tu `scaleX`) ma położenie zaokrąglone do piksela –
            // w `fitLines` każdy ściśnięty wiersz osobno (jego lewa krawędź w układzie pola)
            let x0 = span_x(&layout, pad_x, w, &align);
            let mut lines = layout.lines.clone();
            if layout.per_line {
                for l in lines.iter_mut().filter(|l| l.sx < 0.999) {
                    let d = (x0 + l.x).round() - (x0 + l.x);
                    l.x += d;
                    l.ox += d;
                }
            }
            let st = &mut self.states[i];
            st.mw = Some(layout.w);
            st.mh = Some(layout.h);
            st.block = Some(Arc::new(TextBlock { key: hash_str(&[&key]), lines }));
            if std::env::var_os("VIZRS_RENDER_LAYOUT").is_some() {
                // diagnostyka zgodności z Chromium: rozmiar fontu, ściśnięcie i wymiary elementu tekstu
                let lines: Vec<String> =
                    layout.lines.iter().map(|l| format!("{:.4}px sx {:.6} w {:.0}", l.run.size, l.sx, l.run.offset_width())).collect();
                eprintln!("layout {}: span {}x{} {:?}", self.nodes[i].label(), layout.span_w, layout.span_h, lines);
            }
            st.layout = Some(Arc::new(layout));
            st.key = key;
        }
        let Some(layout) = self.states[i].layout.clone() else { return };
        let Some(block) = self.states[i].block.clone() else { return };
        // flex: wyrównanie elementu tekstu w polu (z marginesem X), w pionie wg `valign`
        let mut x = span_x(&layout, pad_x, w, &align);
        let mut y = match valign.as_str() {
            "top" => 0.0,
            "bottom" => h - layout.span_h,
            _ => (h - layout.span_h) / 2.0,
        };
        if !layout.per_line && layout.lines.first().is_some_and(|l| l.sx < 0.999) {
            // ściśnięty element tekstu (`scaleX`): położenie do pełnego piksela, jak w Blink
            x = x.round();
            y = y.round();
        }
        let Some(col) = color(&self.txt(i, "color")) else { return };
        let sh = self.num(i, "shadow");
        let shadow = (sh > 0.0).then(|| ((sh / 4.0).max(1.0), sh / 2.0, [0.0, 0.0, 0.0, 0.65]));
        self.states[i].content = vec![Paint::Text { block, color: col, shadow, x, y }];
    }

    fn ticker_content(&mut self, i: usize, clk: Clock, res: &Resources, epoch: u64, _dt: f64) {
        let raw = self.txt(i, "text");
        let label = self.txt(i, "label");
        if self.states[i].tk_raw.as_ref() != Some(&(raw.clone(), label.clone())) {
            let items = ticker::parse_items(&strip_markup(&raw));
            self.states[i].tk_page = ticker::page_cat_colors(&items, &label);
            self.states[i].tk_items = items;
            self.states[i].tk_raw = Some((raw, label.clone()));
        }
        let mode = self.txt(i, "tickerMode");
        if !matches!(mode.as_str(), "crawl" | "roll" | "wipe" | "whip") {
            let msg = format!("ticker w trybie {mode}");
            self.issue(i, &msg);
            return;
        }
        self.live = true;
        let font = self.txt(i, "font");
        if !self.font_ok(i, &font, res) && crate::font::is_font_file(&font) {
            return;
        }
        let items = self.states[i].tk_items.clone();
        let up = truthy(&self.prop(i, "upper"));
        let w = self.states[i].w;
        let h = self.states[i].h;
        let mut colors = ticker::parse_cat_colors(&self.txt(i, "catColors"));
        colors.extend(self.states[i].tk_page.clone());
        let cat_bg = self.txt(i, "catBg");
        let cat_fg = self.txt(i, "catFg");
        let lab_w = if label.is_empty() { 0.0 } else { self.num(i, "catW").max(0.0) };
        let size = self.num(i, "size");
        let style = self.style(i, size, false);
        let fg = color(&self.txt(i, "color")).unwrap_or([1.0; 4]);
        let sh = self.num(i, "shadow");
        let shadow = (sh > 0.0).then(|| ((sh / 4.0).max(1.0), sh / 2.0, [0.0, 0.0, 0.0, 0.65f32]));
        let sep = self.txt(i, "sep");
        let sep_style = self.txt(i, "sepStyle");
        let sep_color = self.txt(i, "sepColor");
        let lab_size = {
            let ls = self.num(i, "labelSize");
            if ls > 0.0 {
                ls
            } else {
                size * 0.8
            }
        };
        let lab_weight = {
            let lw = self.txt(i, "labelWeight");
            if lw.is_empty() {
                style.weight
            } else {
                weight_of(&Value::from(lw))
            }
        };
        if mode != "crawl" {
            // roll, wipe, whip (`tickerSlots`): dwie przegródki (bieżąca i następna wiadomość) i wycieraczka
            self.states[i].crawl = None;
            let cat_w = if !label.is_empty() || items.iter().any(|x| !x.cat.is_empty()) { self.num(i, "catW").max(0.0) } else { 0.0 };
            let o = SlotOpts {
                res,
                mode: &mode,
                w,
                h,
                cat_w,
                up,
                label: &label,
                colors: &colors,
                cat_bg: &cat_bg,
                cat_fg: color(&cat_fg).unwrap_or([1.0; 4]),
                style: &style,
                lab_style: Style { size: lab_size, weight: lab_weight, ..style.clone() },
                fg,
                shadow,
                fit: self.txt(i, "textFit"),
                min: text::squeeze_floor(self.num(i, "squeezeMin")),
                hold: f64::from(self.num(i, "hold")).max(1.0),
                gleam: self.num(i, "gleam"),
                now: clk.now,
            };
            let slots = self.states[i].slots.take();
            let (extra, slots) = ticker_slots(&o, &items, slots);
            self.states[i].slots = slots;
            self.states[i].extra = extra;
            return;
        }
        self.states[i].slots = None;
        let style_key = format!(
            "{up}|{sep}|{sep_style}|{sep_color}|{cat_bg}|{cat_fg}|{:?}|{}|{style:?}|{fg:?}|{shadow:?}|{h}|{epoch}",
            self.txt(i, "catColors"),
            items.iter().map(|x| x.color.as_str()).collect::<Vec<_>>().join(",")
        );
        let vw = (w - lab_w).max(1.0) as f64;
        let speed = f64::from(self.num(i, "speed"));
        let crawl_from = self.txt(i, "crawlFrom");
        let now = clk.now;
        let seg_ctx = SegCtx {
            res,
            style: &style,
            up,
            sep: &sep,
            sep_style: &sep_style,
            sep_color: &sep_color,
            colors: &colors,
            cat_bg: &cat_bg,
            cat_fg: &cat_fg,
            h,
            fg,
            shadow,
        };
        let st = &mut self.states[i];
        let tk = st.crawl.get_or_insert_with(|| Crawl {
            segs: VecDeque::new(),
            d: 0.0,
            cursor: 0.0,
            next_idx: 0,
            items: items.clone(),
            style_key: String::new(),
            last: None,
            fresh: true,
        });
        if tk.style_key != style_key {
            tk.style_key = style_key;
            let mut pos = tk.segs.front().map(|s| s.pos).unwrap_or(tk.cursor);
            for sg in tk.segs.iter_mut() {
                if let Some(found) = items.iter().find(|x| x.text == sg.item.text && x.cat == sg.item.cat) {
                    sg.item = found.clone();
                }
                let (node, sw) = segment(&seg_ctx, &sg.item);
                sg.node = Arc::new(node);
                sg.pos = pos;
                sg.w = sw;
                pos += sw;
            }
            tk.cursor = pos;
        }
        if tk.items != items {
            tk.next_idx = match tk.segs.back() {
                Some(last) if !tk.items.is_empty() => ticker::continuation(&tk.items, last.idx, &items),
                _ => 0,
            };
            tk.items = items.clone();
        }
        let dtk = tk.last.map(|l| ((now - l) / 1000.0).clamp(0.0, 0.25)).unwrap_or(0.0);
        tk.last = Some(now);
        tk.d += speed * dtk;
        if tk.segs.is_empty() {
            tk.cursor = tk.cursor.max(tk.d);
        }
        if tk.fresh {
            tk.fresh = false;
            if crawl_from == "left" {
                tk.cursor = tk.d - vw;
            }
        }
        // crawlFill: doklejanie odcinków, aż wypełnią okno (z zapasem 40 px)
        let n = items.len();
        let mut guard = 0;
        while n > 0 && tk.cursor - tk.d < vw + 40.0 && guard < 200 {
            guard += 1;
            let idx = tk.next_idx % n;
            let item = items[idx].clone();
            let (node, sw) = segment(&seg_ctx, &item);
            tk.segs.push_back(Seg { item, idx, pos: tk.cursor, w: sw, node: Arc::new(node) });
            tk.cursor += sw;
            tk.next_idx = (idx + 1) % n;
        }
        while tk.segs.front().is_some_and(|s| s.pos + s.w < tk.d - vw) {
            tk.segs.pop_front();
        }
        if tk.d > 50000.0 {
            let off = tk.segs.front().map(|s| s.pos).unwrap_or(tk.d);
            for sg in tk.segs.iter_mut() {
                sg.pos -= off;
            }
            tk.d -= off;
            tk.cursor -= off;
        }
        // okno taśmy: przycięte, za etykietą
        let mut window = DNode::new(Transform::from_translate(lab_w, 0.0), vw as f32, h);
        window.overflow = Some(0.0);
        let shift = vw - tk.d;
        for sg in &tk.segs {
            let mut s = (*sg.node).clone();
            s.m = Transform::from_translate((shift + sg.pos) as f32, 0.0);
            window.children.push(s);
        }
        let mut extra = vec![window];
        if lab_w > 0.0 {
            let lab_style = Style { size: lab_size, weight: lab_weight, ..style.clone() };
            let bg = ticker::cat_color_of(&Item { cat: label.clone(), color: String::new(), text: String::new() }, &colors, &cat_bg, "");
            let mut lab = DNode::new(Transform::identity(), lab_w, h);
            lab.overflow = Some(0.0);
            if let Some(c) = color(&bg) {
                lab.paints.push(Paint::Box { radius: Radius::Px(0.0), fill: Fill::Solid(c) });
            }
            let shown = if up { label.to_uppercase() } else { label.clone() };
            if let Some(fgc) = color(&cat_fg) {
                if let Some((p, _w)) = label_text(res, &lab_style, &shown, lab_w, h, fgc) {
                    lab.paints.push(p);
                }
            }
            let gp = self.num(i, "gleam");
            if let Some(g) = gleam(gp, now, lab_w, h) {
                lab.children.push(g);
            }
            extra.push(lab);
        }
        self.states[i].extra = extra;
    }
}

/// Moduły kodu QR (true = ciemny).
type QrCells = Arc<Vec<Vec<bool>>>;

/// Stan rotacji wiadomości (`it.tk` w trybie `slots`).
struct Slots {
    items: Vec<Item>,
    cur: Option<Item>,
    cur_idx: Option<usize>,
    prev: Option<Item>,
    next_idx: usize,
    t0: f64,
}

struct SlotOpts<'a> {
    res: &'a Resources,
    mode: &'a str,
    w: f32,
    h: f32,
    cat_w: f32,
    up: bool,
    label: &'a str,
    colors: &'a HashMap<String, String>,
    cat_bg: &'a str,
    cat_fg: Color,
    style: &'a Style,
    lab_style: Style,
    fg: Color,
    shadow: Option<(f32, f32, Color)>,
    fit: String,
    min: f32,
    hold: f64,
    gleam: f32,
    now: f64,
}

fn ease_io(u: f64) -> f64 {
    if u < 0.5 {
        4.0 * u * u * u
    } else {
        1.0 - (-2.0 * u + 2.0).powi(3) / 2.0
    }
}

/// Przegródka: pole tekstu (`box`) i etykieta (`cat`) dla wiadomości `x`.
fn slot_nodes(o: &SlotOpts, x: &Item) -> (DNode, DNode) {
    let (w, h, cat_w) = (o.w, o.h, o.cat_w);
    // etykieta: kategoria albo etykieta domyślna, tło w kolorze kategorii
    let lab = if x.cat.is_empty() { o.label.to_string() } else { x.cat.clone() };
    let mut cat = DNode::new(Transform::identity(), cat_w, h);
    cat.overflow = Some(0.0);
    if !lab.is_empty() {
        if let Some(c) = color(&ticker::cat_color_of(x, o.colors, o.cat_bg, o.label)) {
            cat.paints.push(Paint::Box { radius: Radius::Px(0.0), fill: Fill::Solid(c) });
        }
        let shown = if o.up { lab.to_uppercase() } else { lab };
        if let Some((p, _)) = label_text(o.res, &o.lab_style, &shown, cat_w, h, o.cat_fg) {
            cat.paints.push(p);
        }
    }
    // treść: od catW, margines 18 px, dopasowanie jak `fill` w `tickerSlots`
    let bw = (w - cat_w).max(0.0);
    let mut bx = DNode::new(Transform::from_translate(cat_w, 0.0), bw, h);
    bx.overflow = Some(0.0);
    let body = if o.up { x.text.to_uppercase() } else { x.text.clone() };
    let avail = (w - cat_w - 36.0).max(1.0);
    let (mut block, sw, _) = line_block(o.res, o.style, &body, h);
    let mut sx = 1.0f32;
    if sw > avail && o.fit != "none" {
        let s = avail / sw;
        if o.fit == "shrink" {
            let st = Style { size: o.style.size * s, ..o.style.clone() };
            let (b2, sw2, _) = line_block(o.res, &st, &body, h);
            block = b2;
            let s2 = (avail / sw2.max(1.0)).min(1.0);
            if s2 < 0.999 {
                sx = s2;
            }
        } else if s < o.min && o.fit == "squeeze-shrink" {
            let st = Style { size: o.style.size * s / o.min, ..o.style.clone() };
            let (b2, sw2, _) = line_block(o.res, &st, &body, h);
            block = b2;
            sx = o.min.max((avail / sw2.max(1.0)).min(1.0));
        } else {
            sx = o.min.max(s);
        }
    }
    if sx < 0.999 {
        let mut lines = block.lines.clone();
        for l in &mut lines {
            l.sx = sx;
            l.ox = 0.0;
        }
        block = Arc::new(TextBlock { key: block.key ^ u64::from(sx.to_bits()).rotate_left(17), lines });
    }
    bx.paints.push(Paint::Text { block, color: o.fg, shadow: o.shadow, x: 18.0, y: 0.0 });
    (bx, cat)
}

/// `tickerSlots`: tryby `roll` (góra–dół), `wipe` (wycieraczka) i `whip` (etykieta przejeżdża przez pasek).
fn ticker_slots(o: &SlotOpts, items: &[Item], state: Option<Slots>) -> (Vec<DNode>, Option<Slots>) {
    let n = items.len();
    let now = o.now;
    let mut tk = state.unwrap_or_else(|| Slots {
        items: items.to_vec(),
        cur: items.first().cloned(),
        cur_idx: (n > 0).then_some(0),
        prev: None,
        next_idx: usize::from(n > 1),
        t0: now,
    });
    if tk.items != items {
        if tk.cur.is_none() && n > 0 {
            tk.cur = Some(items[0].clone());
            tk.cur_idx = Some(0);
            tk.next_idx = usize::from(n > 1);
            tk.t0 = now;
        } else if let Some(ci) = tk.cur_idx {
            let next = ticker::continuation(&tk.items, ci, items);
            tk.cur_idx = tk.cur.as_ref().and_then(|c| items.iter().position(|x| x.text == c.text && x.cat == c.cat));
            tk.next_idx = next;
        } else {
            let at = tk.items.get(tk.next_idx).and_then(|c| items.iter().position(|x| x.text == c.text && x.cat == c.cat));
            tk.next_idx = at.unwrap_or_else(|| ticker::continuation(&tk.items, tk.next_idx, items));
        }
        tk.items = items.to_vec();
    }
    let mut out = Vec::new();
    let gleam_on = |cat: &mut DNode| {
        if let Some(g) = gleam(o.gleam, now, o.cat_w, o.h) {
            cat.children.push(g);
        }
    };
    let Some(cur0) = tk.cur.clone().filter(|_| n > 0) else { return (out, Some(tk)) };
    let _ = cur0;
    let single = n == 1 && tk.cur_idx == Some(0);
    let hold = o.hold;
    if single || now - tk.t0 > hold * 50000.0 {
        tk.t0 = now;
    }
    while (now - tk.t0) / 1000.0 >= hold {
        tk.prev = tk.cur.clone();
        let ci = tk.next_idx % n;
        tk.cur_idx = Some(ci);
        tk.cur = Some(items[ci].clone());
        tk.next_idx = (ci + 1) % n;
        tk.t0 += hold * 1000.0;
    }
    let cur = tk.cur.clone().expect("set above");
    let next = items[tk.next_idx % n].clone();
    let ph = (now - tk.t0) / 1000.0;
    let (h, w, cat_w) = (o.h, o.w, o.cat_w);
    let lab_key = |x: &Item| {
        let l = if x.cat.is_empty() { o.label } else { &x.cat };
        format!("{l}#{}", ticker::cat_color_of(x, o.colors, o.cat_bg, o.label))
    };
    let (mut a_box, mut a_cat) = slot_nodes(o, &cur);
    let mut b: Option<(DNode, DNode)> = None;
    let mut a_cat_on = cat_w > 0.0;
    let mut wiper: Option<(f32, String)> = None;
    if !single {
        match o.mode {
            "roll" => {
                let t = (0.55f64).min(hold * 0.3);
                if ph > hold - t {
                    let u = ease_io((ph - (hold - t)) / t) as f32;
                    let (mut b_box, mut b_cat) = slot_nodes(o, &next);
                    a_box.m = Transform::from_translate(cat_w, -u * h);
                    b_box.m = Transform::from_translate(cat_w, (1.0 - u) * h);
                    let mut b_cat_on = cat_w > 0.0;
                    if lab_key(&cur) != lab_key(&next) {
                        a_cat.m = Transform::from_translate(0.0, -u * h);
                        b_cat.m = Transform::from_translate(0.0, (1.0 - u) * h);
                    } else {
                        b_cat_on = false;
                    }
                    if !b_cat_on {
                        b_cat.opacity = 0.0;
                    }
                    b = Some((b_box, b_cat));
                }
            }
            "whip" => {
                let t_in = (0.25f64).min(hold * 0.1);
                let t_out = (0.35f64).min(hold * 0.1);
                let t_w = if cat_w > 0.0 { (1.5f64).min(hold * 0.35) } else { 0.0 };
                let out_at = hold - t_w - t_out;
                a_box.opacity = (if ph < t_in {
                    ph / t_in
                } else if ph < out_at {
                    1.0
                } else {
                    (1.0 - (ph - out_at) / t_out).max(0.0)
                }) as f32;
                if t_w > 0.0 && ph >= hold - t_w {
                    let u = (ph - (hold - t_w)) / t_w;
                    let travel = f64::from((w - cat_w).max(0.0));
                    let x = if u < 0.65 { ease_io(u / 0.65) * travel } else { (1.0 - ease_io((u - 0.65) / 0.35)) * travel };
                    if u < 0.325 {
                        a_cat.m = Transform::from_translate(x as f32, 0.0);
                    } else {
                        // etykieta następnej wiadomości (przegródka B bez treści)
                        a_cat_on = false;
                        let (mut b_box, mut b_cat) = slot_nodes(o, &next);
                        b_box.opacity = 0.0;
                        b_cat.m = Transform::from_translate(x as f32, 0.0);
                        b = Some((b_box, b_cat));
                    }
                }
            }
            _ => {
                // wipe: odsłonięcie od lewej, na końcu zmazanie; krawędź prowadzi wycieraczka w kolorze kategorii
                let t = (0.45f64).min(hold * 0.25);
                let bw = w - cat_w;
                let wc = ticker::cat_color_of(&cur, o.colors, o.cat_bg, o.label);
                if ph < t && tk.prev.is_some() {
                    let u = ease_io(ph / t) as f32;
                    a_box.clip = Some([0.0, (1.0 - u) * 100.0, 0.0, 0.0]);
                    if tk.prev.as_ref().map(lab_key) != Some(lab_key(&cur)) {
                        a_cat.clip = Some([(1.0 - u) * 100.0, 0.0, 0.0, 0.0]);
                    }
                    if u < 0.999 {
                        wiper = Some((cat_w + u * bw - 3.0, wc));
                    }
                } else if ph > hold - t {
                    let u = ease_io((ph - (hold - t)) / t) as f32;
                    a_box.clip = Some([0.0, 0.0, 0.0, u * 100.0]);
                    if lab_key(&cur) != lab_key(&next) {
                        a_cat.clip = Some([0.0, 0.0, u * 100.0, 0.0]);
                    }
                    if u > 0.001 {
                        wiper = Some((cat_w + u * bw - 3.0, wc));
                    }
                }
            }
        }
    }
    // kolejność jak w DOM: pola treści, potem etykiety (z-index 2), na wierzchu wycieraczka (z-index 3)
    out.push(a_box);
    let mut b_cat_node = None;
    if let Some((b_box, b_cat)) = b {
        out.push(b_box);
        b_cat_node = Some(b_cat);
    }
    if a_cat_on {
        gleam_on(&mut a_cat);
        out.push(a_cat);
    }
    if let Some(mut c) = b_cat_node.filter(|_| cat_w > 0.0) {
        if c.opacity > 0.0 {
            gleam_on(&mut c);
        }
        out.push(c);
    }
    if let Some((left, c)) = wiper {
        let mut wp = DNode::new(Transform::from_translate(left, 0.0), 6.0, h);
        if let Some(col) = color(&c) {
            wp.paints.push(Paint::Box { radius: Radius::Px(0.0), fill: Fill::Solid(col) });
        }
        out.push(wp);
    }
    (out, Some(tk))
}

/// Wspólne dane odcinków taśmy crawla.
struct SegCtx<'a> {
    res: &'a Resources,
    style: &'a Style,
    up: bool,
    sep: &'a str,
    sep_style: &'a str,
    sep_color: &'a str,
    colors: &'a HashMap<String, String>,
    cat_bg: &'a str,
    cat_fg: &'a str,
    h: f32,
    fg: Color,
    shadow: Option<(f32, f32, Color)>,
}

/// Blok jednego wiersza tekstu (bez dopasowania) z linią bazową wyśrodkowaną w pionie w `h`.
fn line_block(res: &Resources, style: &Style, text: &str, h: f32) -> (Arc<TextBlock>, f32, f32) {
    let db = res.fonts.read().unwrap_or_else(|e| e.into_inner());
    let run = text::shape(&db, style, text);
    drop(db);
    let (lh, base) = normal_metrics(res, style);
    let _ = lh;
    let w = run.offset_width();
    let baseline = h / 2.0 + base;
    let key = hash_str(&[text, &format!("{style:?}|{h}")]);
    (Arc::new(TextBlock { key, lines: vec![text::Line { run, x: 0.0, baseline, sx: 1.0, ox: 0.0 }] }), w, lh)
}

/// Etykieta tickera: tekst wyśrodkowany w polu `w`×`h`.
fn label_text(res: &Resources, style: &Style, text: &str, w: f32, h: f32, fg: Color) -> Option<(Paint, f32)> {
    let (block, tw, _) = line_block(res, style, text, h);
    Some((Paint::Text { block, color: fg, shadow: None, x: (w - tw) / 2.0, y: 0.0 }, tw))
}

/// Pas błysku (gleam) etykiety: pozycja z czasu.
fn gleam(period: f32, now: f64, w: f32, h: f32) -> Option<DNode> {
    if period <= 0.0 {
        return None;
    }
    let p = f64::from(period);
    let dur = (p * 0.6).min(1.6);
    let u = ((now / 1000.0) % p) / dur;
    if u >= 1.0 {
        return None;
    }
    let gw = w * 0.45;
    let left = w * (-0.6 + u as f32 * 1.7);
    let (ox, oy) = (gw / 2.0, h / 2.0);
    let skew = Transform::from_row(1.0, 0.0, (-20f32).to_radians().tan(), 1.0, 0.0, 0.0);
    let m =
        mul(mul(mul(Transform::from_translate(left, 0.0), Transform::from_translate(ox, oy)), skew), Transform::from_translate(-ox, -oy));
    let mut g = DNode::new(m, gw, h);
    let clear = [1.0, 1.0, 1.0, 0.0];
    g.paints.push(Paint::Box {
        radius: Radius::Px(0.0),
        fill: Fill::Linear { angle: 90.0, stops: vec![(0.0, clear), (0.5, [1.0, 1.0, 1.0, 0.4]), (1.0, clear)] },
    });
    Some(g)
}

/// Odcinek taśmy (`tickerSegment`): [plakietka kategorii] treść [separator]; zwraca węzeł i szerokość (`offsetWidth`).
fn segment(c: &SegCtx, x: &Item) -> (DNode, f64) {
    let size = c.style.size;
    let mut node = DNode::new(Transform::identity(), 0.0, c.h);
    let mut cx = 0.0f32;
    if !x.cat.is_empty() {
        let bst = Style { size: size * 0.78, ..c.style.clone() };
        let cat = if c.up { x.cat.to_uppercase() } else { x.cat.clone() };
        let (block, tw, lh) = line_block(c.res, &bst, &cat, c.h);
        let (px, py) = (bst.size * 0.45, bst.size * 0.05);
        let bw = tw + 2.0 * px;
        let bh = lh + 2.0 * py;
        let bg = ticker::cat_color_of(x, c.colors, c.cat_bg, "");
        if let Some(col) = color(&bg) {
            node.paints.push(Paint::Rect {
                x: cx,
                y: (c.h - bh) / 2.0,
                w: bw,
                h: bh,
                radius: Radius::Px(3.0),
                skew: 0.0,
                fill: Fill::Solid(col),
            });
        }
        if let Some(fg) = color(c.cat_fg) {
            node.paints.push(Paint::Text { block, color: fg, shadow: None, x: cx + px, y: 0.0 });
        }
        cx += bw + bst.size * 0.5; // margin-right: .5em plakietki (jej własny rozmiar fontu)
    }
    let body = if c.up { x.text.to_uppercase() } else { x.text.clone() };
    let (block, tw, _) = line_block(c.res, c.style, &body, c.h);
    node.paints.push(Paint::Text { block, color: c.fg, shadow: c.shadow, x: cx, y: 0.0 });
    cx += tw;
    if !c.sep_style.is_empty() && c.sep_style != "text" {
        let (w, h, round, skew) = match c.sep_style {
            "square" => (0.3, 0.3, false, 0.0),
            "pipe" => (0.12, 0.95, false, 0.0),
            "skew" => (0.16, 1.05, false, -20.0),
            _ => (0.32, 0.32, true, 0.0),
        };
        let (w, h) = (w * size, h * size);
        let m = size * 0.9;
        let (col, alpha) = match color(c.sep_color) {
            Some(col) if !c.sep_color.is_empty() => (col, 1.0),
            _ => (c.fg, 0.55),
        };
        let col = [col[0], col[1], col[2], col[3] * alpha];
        let radius = if round { Radius::Ellipse } else { Radius::Px(0.0) };
        node.paints.push(Paint::Rect { x: cx + m, y: (c.h - h) / 2.0, w, h, radius, skew, fill: Fill::Solid(col) });
        cx += w + 2.0 * m;
    } else {
        let sep_fg = color(c.sep_color).filter(|_| !c.sep_color.is_empty()).unwrap_or(c.fg);
        let (block, tw, _) = line_block(c.res, c.style, c.sep, c.h);
        node.paints.push(Paint::Text { block, color: sep_fg, shadow: c.shadow, x: cx, y: 0.0 });
        cx += tw;
    }
    // offsetWidth odcinka (całkowite px), najmniej 1
    let sw = f64::from(cx.round().max(1.0));
    if std::env::var_os("VIZRS_RENDER_LAYOUT").is_some() {
        eprintln!("odcinek „{}”: {cx:.3} → {sw}", x.text.chars().take(20).collect::<String>());
    }
    node.w = sw as f32;
    (node, sw)
}

/// Tekst bez znaczników (`stripMarkup`).
pub fn strip_markup(t: &str) -> String {
    crate::richtext::strip(t)
}

/// Czy ruch na wzór BTB (`node.motion`) coś robi.
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
            if n.motion.as_ref().is_some_and(motion_used) {
                add("ruch na wzór BTB (motion)".into());
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
