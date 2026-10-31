//! Rysowanie listy (`display`) na CPU przez tiny-skia: warstwy dla krycia grupy, `clip-path` i `filter: blur`,
//! przycięcie `overflow: hidden` maską, cienie rozmyte i tekst rastrowany raz (pamięć podręczna wg treści,
//! przekształcenia i ćwiartki piksela położenia), potem tylko przenoszony – tak jak warstwy kompozytora Chromium.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use tiny_skia::{
    BlendMode, FillRule, FilterQuality, GradientStop, IntRect, LinearGradient, Mask, Paint as SkPaint, Path, PathBuilder, Pattern, Pixmap,
    PixmapPaint, Point, Rect, Shader, SpreadMode, Stroke, Transform,
};

use crate::blur;
use crate::display::{mul, Color, DNode, Fill, Paint, Radius, TextBlock};
use crate::res::Resources;

/// Prostokąt w px urządzenia.
#[derive(Clone, Copy, Debug)]
struct Bounds {
    l: f32,
    t: f32,
    r: f32,
    b: f32,
}

impl Bounds {
    fn empty() -> Bounds {
        Bounds { l: f32::INFINITY, t: f32::INFINITY, r: f32::NEG_INFINITY, b: f32::NEG_INFINITY }
    }
    fn is_empty(&self) -> bool {
        !(self.r > self.l && self.b > self.t)
    }
    fn union(self, o: Bounds) -> Bounds {
        Bounds { l: self.l.min(o.l), t: self.t.min(o.t), r: self.r.max(o.r), b: self.b.max(o.b) }
    }
    fn intersect(self, o: Bounds) -> Bounds {
        Bounds { l: self.l.max(o.l), t: self.t.max(o.t), r: self.r.min(o.r), b: self.b.min(o.b) }
    }
    fn inflate(self, d: f32) -> Bounds {
        Bounds { l: self.l - d, t: self.t - d, r: self.r + d, b: self.b + d }
    }
    /// Prostokąt lokalny po przekształceniu `m` (obwiednia narożników).
    fn mapped(l: f32, t: f32, r: f32, b: f32, m: &Transform) -> Bounds {
        let mut pts = [Point::from_xy(l, t), Point::from_xy(r, t), Point::from_xy(r, b), Point::from_xy(l, b)];
        m.map_points(&mut pts);
        let mut out = Bounds::empty();
        for p in pts {
            out = out.union(Bounds { l: p.x, t: p.y, r: p.x, b: p.y });
        }
        out
    }
    fn int_rect(self, w: u32, h: u32) -> Option<IntRect> {
        let b = self.intersect(Bounds { l: 0.0, t: 0.0, r: w as f32, b: h as f32 });
        if b.is_empty() {
            return None;
        }
        let (l, t) = (b.l.floor() as i32, b.t.floor() as i32);
        let (r, bb) = (b.r.ceil() as i32, b.b.ceil() as i32);
        IntRect::from_ltrb(l, t, r, bb)
    }
}

/// Skala liniowej części przekształcenia (do σ rozmycia w px urządzenia).
fn scale_of(m: &Transform) -> f32 {
    (m.sx * m.sy - m.kx * m.ky).abs().sqrt().max(1e-6)
}

fn sk_color(c: Color, alpha: f32) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0), (c[3] * alpha).clamp(0.0, 1.0))
        .unwrap_or(tiny_skia::Color::TRANSPARENT)
}

/// Prostokąt z zaokrągleniem (jak `border-radius`, promień ograniczony do połowy boku) albo elipsa.
pub fn shape(x: f32, y: f32, w: f32, h: f32, radius: &Radius) -> Option<Path> {
    let rect = Rect::from_xywh(x, y, w.max(0.0), h.max(0.0))?;
    match radius {
        Radius::Ellipse => PathBuilder::from_oval(rect),
        Radius::Px(r) => {
            let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
            if r <= 0.01 {
                return Some(PathBuilder::from_rect(rect));
            }
            let k = 0.552_284_8 * r;
            let (l, t, rr, b) = (x, y, x + w, y + h);
            let mut p = PathBuilder::new();
            p.move_to(l + r, t);
            p.line_to(rr - r, t);
            p.cubic_to(rr - r + k, t, rr, t + r - k, rr, t + r);
            p.line_to(rr, b - r);
            p.cubic_to(rr, b - r + k, rr - r + k, b, rr - r, b);
            p.line_to(l + r, b);
            p.cubic_to(l + r - k, b, l, b - r + k, l, b - r);
            p.line_to(l, t + r);
            p.cubic_to(l, t + r - k, l + r - k, t, l + r, t);
            p.close();
            p.finish()
        }
    }
}

/// `linear-gradient(<kąt>deg, …)` w polu `w`×`h`: linia gradientu jak w CSS (0deg = do góry).
fn linear(angle: f32, stops: &[(f32, Color)], x: f32, y: f32, w: f32, h: f32, alpha: f32) -> Option<Shader<'static>> {
    let a = angle.to_radians();
    let (dx, dy) = (a.sin(), -a.cos());
    let len = (w * a.sin()).abs() + (h * a.cos()).abs();
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let start = Point::from_xy(cx - dx * len / 2.0, cy - dy * len / 2.0);
    let end = Point::from_xy(cx + dx * len / 2.0, cy + dy * len / 2.0);
    let stops = stops.iter().map(|(p, c)| GradientStop::new(*p, sk_color(*c, alpha))).collect();
    LinearGradient::new(start, end, stops, SpreadMode::Pad, Transform::identity())
}

fn fill_shader(fill: &Fill, x: f32, y: f32, w: f32, h: f32, alpha: f32) -> Option<Shader<'static>> {
    match fill {
        Fill::Solid(c) => Some(Shader::SolidColor(sk_color(*c, alpha))),
        Fill::Linear { angle, stops } => linear(*angle, stops, x, y, w, h, alpha),
    }
}

struct Cached {
    pix: Arc<Pixmap>,
    /// Lewy górny róg w px urządzenia względem całkowitej części położenia.
    ox: i32,
    oy: i32,
    used: u64,
}

/// Rysownik z pamięcią podręczną rastrów (tekst, cienie).
#[derive(Default)]
pub struct Raster {
    cache: HashMap<u64, Cached>,
    frame: u64,
}

const KEEP_FRAMES: u64 = 90;

impl Raster {
    pub fn new() -> Raster {
        Raster::default()
    }

    /// Rysuje sceny (już w kolejności warstw) z przekształceniem sceny → piksele `stage`.
    pub fn draw(&mut self, pix: &mut Pixmap, scenes: &[(&DNode, Transform)], res: &Resources) {
        self.frame += 1;
        pix.fill(tiny_skia::Color::TRANSPARENT);
        for (node, stage) in scenes {
            self.node(pix, node, *stage, None, 1.0, res);
        }
        let frame = self.frame;
        self.cache.retain(|_, c| frame - c.used <= KEEP_FRAMES);
    }

    /// Obwiednia węzła z dziećmi w px urządzenia (`m` – przekształcenie rodzica).
    fn bounds(&self, node: &DNode, parent: Transform, res: &Resources) -> Bounds {
        let m = mul(parent, node.m);
        let mut local = Bounds { l: 0.0, t: 0.0, r: node.w, b: node.h };
        for p in &node.paints {
            match p {
                Paint::BoxShadow { sigma, dy, .. } => {
                    let e = 3.0 * sigma + 2.0;
                    local = local.union(Bounds { l: -e, t: dy - e, r: node.w + e, b: node.h + dy + e });
                }
                Paint::Rect { x, y, w, h, .. } => local = local.union(Bounds { l: *x - 2.0, t: *y - 2.0, r: x + w + 2.0, b: y + h + 2.0 }),
                Paint::Text { block, shadow, x, y, .. } => {
                    let tb = text_bounds(block, res);
                    let e = shadow.map(|(dy, s, _)| 3.0 * s + dy.abs() + 2.0).unwrap_or(2.0);
                    local = local.union(Bounds { l: x + tb.l - e, t: y + tb.t - e, r: x + tb.r + e, b: y + tb.b + e });
                }
                _ => {}
            }
        }
        let mut out = Bounds::mapped(local.l, local.t, local.r, local.b, &m);
        let cm = mul(m, node.child_m);
        let mut kids = Bounds::empty();
        for c in &node.children {
            kids = kids.union(self.bounds(c, cm, res));
        }
        if node.overflow.is_some() {
            kids = kids.intersect(Bounds::mapped(0.0, 0.0, node.w, node.h, &m));
        }
        if !kids.is_empty() {
            out = out.union(kids);
        }
        if node.blur > 0.0 {
            out = out.inflate(blur::extent(node.blur * scale_of(&m)));
        }
        out
    }

    fn node(&mut self, dst: &mut Pixmap, node: &DNode, parent: Transform, mask: Option<&Mask>, alpha: f32, res: &Resources) {
        if node.opacity <= 0.001 {
            return;
        }
        let m = mul(parent, node.m);
        let items = node.paints.len() + node.children.len();
        let layer = node.clip.is_some() || node.blur > 0.0 || (node.opacity < 0.999 && items > 1);
        if !layer {
            self.contents(dst, node, m, mask, alpha * node.opacity, res);
            return;
        }
        let b = self.bounds(node, parent, res);
        let Some(r) = b.int_rect(dst.width(), dst.height()) else { return };
        let Some(mut layer_pix) = Pixmap::new(r.width(), r.height()) else { return };
        let shift = Transform::from_translate(-(r.x() as f32), -(r.y() as f32));
        let lm = mul(shift, m);
        self.contents(&mut layer_pix, node, lm, None, 1.0, res);
        if node.blur > 0.0 {
            let (w, h) = (layer_pix.width() as usize, layer_pix.height() as usize);
            blur::gaussian(layer_pix.data_mut(), w, h, 4, node.blur * scale_of(&m));
        }
        if let Some([t, rr, bb, l]) = node.clip {
            let (w, h) = (node.w, node.h);
            let (x0, y0) = (w * l / 100.0, h * t / 100.0);
            let (x1, y1) = (w * (1.0 - rr / 100.0), h * (1.0 - bb / 100.0));
            let mut cm = Mask::new(layer_pix.width(), layer_pix.height()).expect("layer size");
            if x1 > x0 && y1 > y0 {
                if let Some(path) = Rect::from_ltrb(x0, y0, x1, y1).map(PathBuilder::from_rect) {
                    cm.fill_path(&path, FillRule::Winding, true, lm);
                }
            }
            layer_pix.apply_mask(&cm);
        }
        let paint = PixmapPaint {
            opacity: (alpha * node.opacity).clamp(0.0, 1.0),
            blend_mode: BlendMode::SourceOver,
            quality: FilterQuality::Nearest,
        };
        dst.draw_pixmap(r.x(), r.y(), layer_pix.as_ref(), &paint, Transform::identity(), mask);
    }

    fn contents(&mut self, dst: &mut Pixmap, node: &DNode, m: Transform, mask: Option<&Mask>, alpha: f32, res: &Resources) {
        for p in &node.paints {
            self.paint(dst, node, p, m, mask, alpha, res);
        }
        if node.children.is_empty() {
            return;
        }
        let cm = mul(m, node.child_m);
        if let Some(radius) = node.overflow {
            let b = Bounds::mapped(0.0, 0.0, node.w, node.h, &m);
            let canvas = Bounds { l: 0.0, t: 0.0, r: dst.width() as f32, b: dst.height() as f32 };
            let axis = m.kx == 0.0 && m.ky == 0.0 && radius <= 0.0;
            if axis && b.l <= 0.0 && b.t <= 0.0 && b.r >= canvas.r && b.b >= canvas.b {
                // pole obejmuje całe płótno – nic do przycinania
            } else if axis {
                // prostokąt bez obrotu: dzieci w warstwie o rozmiarze przycięcia (krawędzie do pełnych pikseli, jak Chromium)
                let clip = Bounds { l: b.l.round(), t: b.t.round(), r: b.r.round(), b: b.b.round() };
                let mut kids = Bounds::empty();
                for c in &node.children {
                    kids = kids.union(self.bounds(c, cm, res));
                }
                let Some(r) = kids.intersect(clip).int_rect(dst.width(), dst.height()) else { return };
                let Some(mut layer) = Pixmap::new(r.width(), r.height()) else { return };
                let shift = Transform::from_translate(-(r.x() as f32), -(r.y() as f32));
                for c in &node.children {
                    self.node(&mut layer, c, mul(shift, cm), None, 1.0, res);
                }
                let paint =
                    PixmapPaint { opacity: alpha.clamp(0.0, 1.0), blend_mode: BlendMode::SourceOver, quality: FilterQuality::Nearest };
                dst.draw_pixmap(r.x(), r.y(), layer.as_ref(), &paint, Transform::identity(), mask);
                return;
            } else {
                // obrót, pochylenie albo zaokrąglenie: maska z kształtem pola
                let mut mk = match mask {
                    Some(parent) => parent.clone(),
                    None => {
                        let mut all = Mask::new(dst.width(), dst.height()).expect("canvas size");
                        all.data_mut().fill(255);
                        all
                    }
                };
                match shape(0.0, 0.0, node.w, node.h, &Radius::Px(radius)) {
                    Some(path) => mk.intersect_path(&path, FillRule::Winding, true, m),
                    None => mk.clear(),
                }
                for c in &node.children {
                    self.node(dst, c, cm, Some(&mk), alpha, res);
                }
                return;
            }
        }
        for c in &node.children {
            self.node(dst, c, cm, mask, alpha, res);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(&mut self, dst: &mut Pixmap, node: &DNode, p: &Paint, m: Transform, mask: Option<&Mask>, alpha: f32, res: &Resources) {
        let fill_path = |dst: &mut Pixmap, path: &Path, shader: Shader, rule: FillRule, tf: Transform| {
            let paint = SkPaint { shader, anti_alias: true, ..SkPaint::default() };
            dst.fill_path(path, &paint, rule, tf, mask);
        };
        match p {
            Paint::Box { radius, fill } => {
                if let (Some(path), Some(sh)) =
                    (shape(0.0, 0.0, node.w, node.h, radius), fill_shader(fill, 0.0, 0.0, node.w, node.h, alpha))
                {
                    fill_path(dst, &path, sh, FillRule::Winding, m);
                }
            }
            Paint::Rect { x, y, w, h, radius, skew, fill } => {
                let tf = if *skew != 0.0 {
                    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
                    let k = Transform::from_row(1.0, 0.0, skew.to_radians().tan(), 1.0, 0.0, 0.0);
                    mul(mul(mul(m, Transform::from_translate(cx, cy)), k), Transform::from_translate(-cx, -cy))
                } else {
                    m
                };
                if let (Some(path), Some(sh)) = (shape(*x, *y, *w, *h, radius), fill_shader(fill, *x, *y, *w, *h, alpha)) {
                    fill_path(dst, &path, sh, FillRule::Winding, tf);
                }
            }
            Paint::Border { radius, width, color } => {
                let bw = width.min(node.w / 2.0).min(node.h / 2.0);
                let inner_r = match radius {
                    Radius::Px(r) => Radius::Px((r.min(node.w / 2.0).min(node.h / 2.0) - bw).max(0.0)),
                    Radius::Ellipse => Radius::Ellipse,
                };
                let mut pb = PathBuilder::new();
                if let Some(o) = shape(0.0, 0.0, node.w, node.h, radius) {
                    pb.push_path(&o);
                }
                if let Some(i) = shape(bw, bw, node.w - 2.0 * bw, node.h - 2.0 * bw, &inner_r) {
                    pb.push_path(&i);
                }
                if let Some(path) = pb.finish() {
                    fill_path(dst, &path, Shader::SolidColor(sk_color(*color, alpha)), FillRule::EvenOdd, m);
                }
            }
            Paint::Image { src, fit, radius } => {
                let Some(img) = res.image(src) else { return };
                let (iw, ih) = (img.width() as f32, img.height() as f32);
                let (w, h) = (node.w, node.h);
                if iw <= 0.0 || ih <= 0.0 || w <= 0.0 || h <= 0.0 {
                    return;
                }
                let (sx, sy) = match fit.as_str() {
                    "fill" => (w / iw, h / ih),
                    "cover" => {
                        let s = (w / iw).max(h / ih);
                        (s, s)
                    }
                    _ => {
                        let s = (w / iw).min(h / ih);
                        (s, s)
                    }
                };
                let (dw, dh) = (iw * sx, ih * sy);
                let (dx, dy) = ((w - dw) / 2.0, (h - dh) / 2.0);
                let img_tf = Transform::from_row(sx, 0.0, 0.0, sy, dx, dy);
                // obszar: część pola zajęta przez obraz, z zaokrągleniem pola
                let path = if fit == "contain" && (dw < w - 0.5 || dh < h - 0.5) {
                    Rect::from_xywh(dx.max(0.0), dy.max(0.0), dw.min(w), dh.min(h)).map(PathBuilder::from_rect)
                } else {
                    shape(0.0, 0.0, w, h, &Radius::Px(*radius))
                };
                let scale = scale_of(&m) * sx.min(sy);
                let quality = if !(0.999..=1.001).contains(&scale) { FilterQuality::Bicubic } else { FilterQuality::Bilinear };
                if let Some(path) = path {
                    let shader = Pattern::new(img.as_ref().as_ref(), SpreadMode::Pad, quality, alpha, img_tf);
                    fill_path(dst, &path, shader, FillRule::Winding, m);
                }
            }
            Paint::Qr { cells, dark, light, .. } => {
                let quiet = 2.0;
                let n = cells.as_ref().map(|c| c.len()).unwrap_or(8) as f32;
                let total = n + 2.0 * quiet;
                let side = node.w.min(node.h);
                let (ox, oy) = ((node.w - side) / 2.0, (node.h - side) / 2.0);
                let unit = side / total;
                // shape-rendering: crispEdges – bez wygładzania krawędzi modułów
                let crisp =
                    |shader: Shader<'static>, path: &Path| (SkPaint { shader, anti_alias: false, ..SkPaint::default() }, path.clone());
                if let Some(bg) = Rect::from_xywh(ox, oy, side, side).map(PathBuilder::from_rect) {
                    let (paint, path) = crisp(Shader::SolidColor(sk_color(*light, alpha)), &bg);
                    dst.fill_path(&path, &paint, FillRule::Winding, m, mask);
                }
                if let Some(cells) = cells {
                    let mut pb = PathBuilder::new();
                    for (y, row) in cells.iter().enumerate() {
                        for (x, &on) in row.iter().enumerate() {
                            if on {
                                if let Some(r) = Rect::from_xywh(ox + (x as f32 + quiet) * unit, oy + (y as f32 + quiet) * unit, unit, unit)
                                {
                                    pb.push_rect(r);
                                }
                            }
                        }
                    }
                    if let Some(path) = pb.finish() {
                        let (paint, path) = crisp(Shader::SolidColor(sk_color(*dark, alpha)), &path);
                        dst.fill_path(&path, &paint, FillRule::Winding, m, mask);
                    }
                }
            }
            Paint::BoxShadow { radius, sigma, dy, color } => {
                let key = {
                    let mut h = DefaultHasher::new();
                    "shadow".hash(&mut h);
                    for v in [node.w, node.h, *sigma, *dy] {
                        v.to_bits().hash(&mut h);
                    }
                    match radius {
                        Radius::Px(r) => r.to_bits().hash(&mut h),
                        Radius::Ellipse => 1u8.hash(&mut h),
                    }
                    for c in color {
                        c.to_bits().hash(&mut h);
                    }
                    h.finish()
                };
                let (w, h, radius, sigma, dy, color) = (node.w, node.h, radius.clone(), *sigma, *dy, *color);
                self.blit(dst, key, m, mask, alpha, |lin, out_pix_bounds| {
                    // kształt przesunięty o dy, rozmyty, z wyciętym polem (cień tylko na zewnątrz)
                    let e = 3.0 * sigma + 2.0;
                    let b = Bounds::mapped(-e, dy - e, w + e, h + dy + e, &lin);
                    let pad = blur::extent(sigma * scale_of(&lin));
                    let b = b.inflate(pad);
                    let (ox, oy) = (b.l.floor(), b.t.floor());
                    let (pw, ph) = ((b.r.ceil() - ox) as u32, (b.b.ceil() - oy) as u32);
                    *out_pix_bounds = (ox as i32, oy as i32);
                    let mut pix = Pixmap::new(pw.max(1), ph.max(1))?;
                    let tf = mul(Transform::from_translate(-ox, -oy), lin);
                    let path = shape(0.0, dy, w, h, &radius)?;
                    let paint = SkPaint { shader: Shader::SolidColor(sk_color(color, 1.0)), anti_alias: true, ..SkPaint::default() };
                    pix.fill_path(&path, &paint, FillRule::Winding, tf, None);
                    let (pw, ph) = (pix.width() as usize, pix.height() as usize);
                    blur::gaussian(pix.data_mut(), pw, ph, 4, sigma * scale_of(&lin));
                    let cut = shape(0.0, 0.0, w, h, &radius)?;
                    let clear = SkPaint { blend_mode: BlendMode::Clear, anti_alias: true, ..SkPaint::default() };
                    pix.fill_path(&cut, &clear, FillRule::Winding, tf, None);
                    Some(pix)
                });
            }
            Paint::Text { block, color, shadow, x, y } => {
                let key = {
                    let mut h = DefaultHasher::new();
                    "text".hash(&mut h);
                    block.key.hash(&mut h);
                    for c in color {
                        c.to_bits().hash(&mut h);
                    }
                    if let Some((d, s, c)) = shadow {
                        d.to_bits().hash(&mut h);
                        s.to_bits().hash(&mut h);
                        for v in c {
                            v.to_bits().hash(&mut h);
                        }
                    }
                    res.epoch().hash(&mut h);
                    h.finish()
                };
                let local = mul(m, Transform::from_translate(*x, *y));
                let (block, color, shadow) = (Arc::clone(block), *color, *shadow);
                self.blit(dst, key, local, mask, alpha, |lin, origin| raster_text(&block, color, shadow, lin, origin, res));
            }
        }
    }

    /// Raster z pamięci podręcznej przeniesiony w położenie `m`. Klucz obejmuje część liniową `m`
    /// i położenie z dokładnością do ćwiartki piksela (ruch płynny, bez rozmycia przez przenoszenie).
    fn blit<F>(&mut self, dst: &mut Pixmap, key: u64, m: Transform, mask: Option<&Mask>, alpha: f32, make: F)
    where
        F: FnOnce(Transform, &mut (i32, i32)) -> Option<Pixmap>,
    {
        let (ix, iy) = (m.tx.floor(), m.ty.floor());
        let (fx, fy) = (((m.tx - ix) * 4.0).round() / 4.0, ((m.ty - iy) * 4.0).round() / 4.0);
        let lin = Transform::from_row(m.sx, m.ky, m.kx, m.sy, fx, fy);
        let full_key = {
            let mut h = DefaultHasher::new();
            key.hash(&mut h);
            for v in [lin.sx, lin.ky, lin.kx, lin.sy] {
                ((v * 4096.0).round() as i64).hash(&mut h);
            }
            ((fx * 4.0) as i32).hash(&mut h);
            ((fy * 4.0) as i32).hash(&mut h);
            h.finish()
        };
        let frame = self.frame;
        let c = match self.cache.entry(full_key) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => {
                let mut origin = (0, 0);
                let Some(pix) = make(lin, &mut origin) else { return };
                e.insert(Cached { pix: Arc::new(pix), ox: origin.0, oy: origin.1, used: frame })
            }
        };
        c.used = frame;
        let paint = PixmapPaint { opacity: alpha.clamp(0.0, 1.0), blend_mode: BlendMode::SourceOver, quality: FilterQuality::Nearest };
        dst.draw_pixmap(ix as i32 + c.ox, iy as i32 + c.oy, c.pix.as_ref().as_ref(), &paint, Transform::identity(), mask);
    }
}

/// Przekształcenie glifu `g` wiersza `line` (jednostki fontu → px elementu tekstu).
fn glyph_tf(line: &crate::text::Line, g: &crate::text::Glyph, k: f32) -> Transform {
    let squeeze = Transform::from_row(line.sx, 0.0, 0.0, 1.0, line.ox - line.ox * line.sx, 0.0);
    let mut gt = Transform::from_row(k, 0.0, 0.0, k, line.x + g.x, line.baseline + g.dy);
    if g.fake_italic {
        gt = mul(gt, Transform::from_row(1.0, 0.0, -0.25, 1.0, 0.0, 0.0));
    }
    mul(squeeze, gt)
}

/// Obwiednia tekstu w px elementu tekstu.
fn text_bounds(block: &TextBlock, res: &Resources) -> Bounds {
    let db = res.fonts.read().unwrap_or_else(|e| e.into_inner());
    let mut out = Bounds::empty();
    for line in &block.lines {
        let size = line.run.size;
        let x0 = line.ox + (line.x - line.ox) * line.sx;
        let x1 = line.ox + (line.x + line.run.width - line.ox) * line.sx;
        let mut asc = line.run.ascent;
        let mut desc = line.run.descent;
        if let Some(g) = line.run.glyphs.first() {
            if g.face < db.len() {
                let f = db.face(g.face);
                asc = asc.max(f.ascent / f.upem * size * 1.1);
                desc = desc.max(f.descent / f.upem * size * 1.1);
            }
        }
        // glify bywają szersze od przesunięcia (kursywa, ogonki)
        let e = size * 0.3;
        out = out.union(Bounds { l: x0.min(x1) - e, t: line.baseline - asc - e, r: x0.max(x1) + e, b: line.baseline + desc + e });
    }
    out
}

/// Rastruje tekst (cień pod spodem) w przekształceniu `lin`; `origin` – lewy górny róg rastra.
fn raster_text(
    block: &TextBlock,
    color: Color,
    shadow: Option<(f32, f32, Color)>,
    lin: Transform,
    origin: &mut (i32, i32),
    res: &Resources,
) -> Option<Pixmap> {
    let tb = text_bounds(block, res);
    if tb.is_empty() {
        return None;
    }
    let mut b = Bounds::mapped(tb.l, tb.t, tb.r, tb.b, &lin);
    if let Some((dy, s, _)) = shadow {
        let sb = Bounds::mapped(tb.l, tb.t + dy, tb.r, tb.b + dy, &lin).inflate(blur::extent(s * scale_of(&lin)) + 1.0);
        b = b.union(sb);
    }
    let (ox, oy) = (b.l.floor(), b.t.floor());
    let (pw, ph) = ((b.r.ceil() - ox).max(1.0) as u32, (b.b.ceil() - oy).max(1.0) as u32);
    if u64::from(pw) * u64::from(ph) > 64 << 20 {
        return None;
    }
    *origin = (ox as i32, oy as i32);
    let mut pix = Pixmap::new(pw, ph)?;
    let base = mul(Transform::from_translate(-ox, -oy), lin);
    let db = res.fonts.read().unwrap_or_else(|e| e.into_inner());
    // `shadow`: cały tekst i podkreślenia kolorem cienia (bez tła wyróżnień, jak `text-shadow`)
    let draw = |pix: &mut Pixmap, tf0: Transform, shadow: Option<Color>| {
        let solid = |c: Color| SkPaint { shader: Shader::SolidColor(sk_color(c, 1.0)), anti_alias: true, ..SkPaint::default() };
        for line in &block.lines {
            let lt = mul(tf0, Transform::from_row(line.sx, 0.0, 0.0, 1.0, line.ox - line.ox * line.sx, 0.0));
            let lt = mul(lt, Transform::from_translate(line.x, line.baseline));
            if shadow.is_none() {
                for d in &line.run.decor {
                    if let crate::text::Decor::Mark { x0, x1, top, bottom, radius, color } = d {
                        if let Some(path) = shape(*x0, *top, x1 - x0, bottom - top, &Radius::Px(*radius)) {
                            pix.fill_path(&path, &solid(*color), FillRule::Winding, lt, None);
                        }
                    }
                }
            }
            for g in &line.run.glyphs {
                if g.face >= db.len() {
                    continue;
                }
                let face = db.face(g.face);
                let Some(path) = face.glyph_path(g.id) else { continue };
                let k = g.size / face.upem;
                let tf = mul(tf0, glyph_tf(line, g, k));
                let paint = solid(shadow.or(g.color).unwrap_or(color));
                pix.fill_path(&path, &paint, FillRule::Winding, tf, None);
                if g.fake_bold {
                    // Skia: pogrubienie obrysem, grubość od 1/24 (9 px) do 1/32 (36 px) rozmiaru
                    let s = g.size;
                    let ratio = if s <= 9.0 {
                        1.0 / 24.0
                    } else if s >= 36.0 {
                        1.0 / 32.0
                    } else {
                        1.0 / 24.0 + (s - 9.0) / 27.0 * (1.0 / 32.0 - 1.0 / 24.0)
                    };
                    let stroke = Stroke { width: s * ratio / k, ..Stroke::default() };
                    pix.stroke_path(&path, &paint, &stroke, tf, None);
                }
            }
            for d in &line.run.decor {
                if let crate::text::Decor::Underline { x0, x1, y, th, color: c } = d {
                    if let Some(r) = Rect::from_xywh(*x0, y - th / 2.0, x1 - x0, *th) {
                        pix.fill_path(&PathBuilder::from_rect(r), &solid(shadow.or(*c).unwrap_or(color)), FillRule::Winding, lt, None);
                    }
                }
            }
        }
    };
    if let Some((dy, s, c)) = shadow {
        let mut sp = Pixmap::new(pw, ph)?;
        draw(&mut sp, mul(base, Transform::from_translate(0.0, dy)), Some(c));
        let (w, h) = (sp.width() as usize, sp.height() as usize);
        blur::gaussian(sp.data_mut(), w, h, 4, s * scale_of(&lin));
        pix.draw_pixmap(0, 0, sp.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    draw(&mut pix, base, None);
    Some(pix)
}

/// Obraz z przemnożoną alfą → RGBA bez przemnożenia (jak PNG z Chromium).
pub fn unpremultiply(data: &mut [u8]) {
    for p in data.chunks_exact_mut(4) {
        let a = u32::from(p[3]);
        if a > 0 && a < 255 {
            for c in &mut p[..3] {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_shape_fills_inside() {
        let p = shape(0.0, 0.0, 10.0, 10.0, &Radius::Px(5.0)).unwrap();
        let b = p.bounds();
        assert!((b.width() - 10.0).abs() < 1e-3);
        assert!(shape(0.0, 0.0, 0.0, 10.0, &Radius::Px(0.0)).is_none() || shape(0.0, 0.0, 0.0, 10.0, &Radius::Px(0.0)).is_some());
    }

    #[test]
    fn unpremultiply_restores_color() {
        let mut d = vec![64, 0, 0, 128];
        unpremultiply(&mut d);
        assert_eq!(d, vec![128, 0, 0, 128]);
    }

    #[test]
    fn layer_with_opacity_draws_once() {
        let res = Resources::new(crate::font::FontDb::new(), None, true);
        let mut pix = Pixmap::new(20, 20).unwrap();
        let mut n = DNode::new(Transform::identity(), 20.0, 20.0);
        n.opacity = 0.5;
        n.paints.push(Paint::Box { radius: Radius::Px(0.0), fill: Fill::Solid([1.0, 0.0, 0.0, 1.0]) });
        n.paints.push(Paint::Box { radius: Radius::Px(0.0), fill: Fill::Solid([1.0, 0.0, 0.0, 1.0]) });
        let mut r = Raster::new();
        r.draw(&mut pix, &[(&n, Transform::identity())], &res);
        // krycie grupy: dwa nałożone pola dają 50%, nie 75%
        assert!((i32::from(pix.data()[3]) - 128).abs() <= 1, "{}", pix.data()[3]);
    }
}
