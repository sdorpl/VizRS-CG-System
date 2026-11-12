//! Lista rysowania: drzewo elementów jak DOM strony `/output` (pole, przekształcenie, krycie, maska, rozmycie,
//! przycięcie do pola) z farbami w układzie elementu. Budowana co klatkę przez `scene`, rysowana przez `raster`.
//! Skrót (`fingerprint`) listy pozwala odpowiedzieć „bez zmian” bez rysowania.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use tiny_skia::Transform;

use crate::text::Line;

pub type Color = [f32; 4]; // RGBA 0–1 (bez przemnożenia)

#[derive(Clone, Debug, PartialEq)]
pub enum Radius {
    Px(f32),
    Ellipse,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Fill {
    Solid(Color),
    /// `linear-gradient(<kąt>deg, …)`: punkty (położenie 0–1, kolor).
    Linear {
        angle: f32,
        stops: Vec<(f32, Color)>,
    },
}

/// Tekst ułożony do narysowania (wspólny dla klatek, póki nie zmieni się treść ani styl).
#[derive(Debug)]
pub struct TextBlock {
    pub key: u64,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug)]
pub enum Paint {
    /// `box-shadow: 0 dy blur color` (rozmycie σ = blur / 2), tylko na zewnątrz pola.
    BoxShadow { radius: Radius, sigma: f32, dy: f32, color: Color },
    /// Tło pola (`background`).
    Box { radius: Radius, fill: Fill },
    /// Prostokąt w polu (separatory, plakietki): x, y, w, h, zaokrąglenie, pochylenie, wypełnienie.
    Rect { x: f32, y: f32, w: f32, h: f32, radius: Radius, skew: f32, fill: Fill },
    /// `border: w solid color` (wewnątrz pola, `box-sizing: border-box`).
    Border { radius: Radius, width: f32, color: Color },
    /// Obraz z assets dopasowany `object-fit`, przycięty do pola z zaokrągleniem.
    Image { src: String, fit: String, radius: f32 },
    /// Kod QR (`qrSvg`): kwadrat wpisany w pole i wyśrodkowany, margines 2 moduły, krawędzie bez wygładzania.
    Qr { text: Arc<str>, cells: Option<Arc<Vec<Vec<bool>>>>, dark: Color, light: Color },
    /// Tekst: blok, kolor, cień (dy, σ, kolor) i położenie elementu tekstu (`span`) w polu.
    Text { block: Arc<TextBlock>, color: Color, shadow: Option<(f32, f32, Color)>, x: f32, y: f32 },
}

#[derive(Clone, Debug)]
pub struct DNode {
    /// Układ elementu → układ treści rodzica (położenie, przekształcenie CSS z osią w środku pola).
    pub m: Transform,
    pub w: f32,
    pub h: f32,
    pub opacity: f32,
    /// `clip-path: inset(t% r% b% l%)`.
    pub clip: Option<[f32; 4]>,
    /// `filter: blur(px)`.
    pub blur: f32,
    /// `overflow: hidden` (z zaokrągleniem rogów).
    pub overflow: Option<f32>,
    pub paints: Vec<Paint>,
    /// Układ dzieci względem pola (ramka przesuwa pole wypełnienia).
    pub child_m: Transform,
    pub children: Vec<DNode>,
}

impl DNode {
    pub fn new(m: Transform, w: f32, h: f32) -> DNode {
        DNode {
            m,
            w,
            h,
            opacity: 1.0,
            clip: None,
            blur: 0.0,
            overflow: None,
            paints: Vec::new(),
            child_m: Transform::identity(),
            children: Vec::new(),
        }
    }

    pub fn fingerprint(&self, h: &mut impl Hasher) {
        self.own_fingerprint(h);
        for c in &self.children {
            c.fingerprint(h);
        }
    }

    /// Skrót samego węzła (bez treści dzieci, z ich liczbą): położenie, pole, efekty, malowanie.
    pub fn own_fingerprint(&self, h: &mut impl Hasher) {
        hash_tf(&self.m, h);
        hash_f(&[self.w, self.h, self.opacity, self.blur, self.overflow.unwrap_or(-1.0)], h);
        if let Some(c) = self.clip {
            hash_f(&c, h);
        }
        hash_tf(&self.child_m, h);
        self.paints.len().hash(h);
        for p in &self.paints {
            p.fingerprint(h);
        }
        self.children.len().hash(h);
    }
}

/// Prostokąt w całkowitych px płótna: lewo, góra, prawo, dół (prawo i dół – poza prostokątem).
pub type PxRect = (i32, i32, i32, i32);

/// Suma dwóch prostokątów (`None` – pusty).
pub fn px_union(a: Option<PxRect>, b: Option<PxRect>) -> Option<PxRect> {
    match (a, b) {
        (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))),
        (a, None) => a,
        (None, b) => b,
    }
}

/// Podpis drzewa rysowania do szukania obszaru zmiany między klatkami: skrót węzła, skrót poddrzewa
/// i obwiednia poddrzewa w px płótna (z cieniami i rozmyciem), dla każdego dziecka osobno.
#[derive(Debug)]
pub struct Sig {
    pub own: u64,
    pub sub: u64,
    pub area: Option<PxRect>,
    pub kids: Vec<Sig>,
}

impl Sig {
    /// Obszar płótna, w którym `self` (nowa klatka) różni się od `old`: obwiednie węzłów, które
    /// się zmieniły (stara i nowa), a w węzłach bez zmiany – tylko w zmienionych dzieciach.
    pub fn damage(&self, old: &Sig) -> Option<PxRect> {
        if self.sub == old.sub {
            return None;
        }
        if self.own != old.own || self.kids.len() != old.kids.len() {
            return px_union(self.area, old.area);
        }
        self.kids.iter().zip(&old.kids).fold(None, |d, (a, b)| px_union(d, a.damage(b)))
    }
}

fn hash_f(v: &[f32], h: &mut impl Hasher) {
    for x in v {
        x.to_bits().hash(h);
    }
}

fn hash_tf(t: &Transform, h: &mut impl Hasher) {
    hash_f(&[t.sx, t.kx, t.ky, t.sy, t.tx, t.ty], h);
}

fn hash_radius(r: &Radius, h: &mut impl Hasher) {
    match r {
        Radius::Px(x) => hash_f(&[*x], h),
        Radius::Ellipse => (-7i32).hash(h),
    }
}

fn hash_fill(f: &Fill, h: &mut impl Hasher) {
    match f {
        Fill::Solid(c) => hash_f(c, h),
        Fill::Linear { angle, stops } => {
            hash_f(&[*angle], h);
            for (p, c) in stops {
                hash_f(&[*p], h);
                hash_f(c, h);
            }
        }
    }
}

impl Paint {
    fn fingerprint(&self, h: &mut impl Hasher) {
        match self {
            Paint::BoxShadow { radius, sigma, dy, color } => {
                1u8.hash(h);
                hash_radius(radius, h);
                hash_f(&[*sigma, *dy], h);
                hash_f(color, h);
            }
            Paint::Box { radius, fill } => {
                2u8.hash(h);
                hash_radius(radius, h);
                hash_fill(fill, h);
            }
            Paint::Rect { x, y, w, h: hh, radius, skew, fill } => {
                3u8.hash(h);
                hash_f(&[*x, *y, *w, *hh, *skew], h);
                hash_radius(radius, h);
                hash_fill(fill, h);
            }
            Paint::Border { radius, width, color } => {
                4u8.hash(h);
                hash_radius(radius, h);
                hash_f(&[*width], h);
                hash_f(color, h);
            }
            Paint::Image { src, fit, radius } => {
                5u8.hash(h);
                src.hash(h);
                fit.hash(h);
                hash_f(&[*radius], h);
            }
            Paint::Qr { text, dark, light, .. } => {
                7u8.hash(h);
                text.hash(h);
                hash_f(dark, h);
                hash_f(light, h);
            }
            Paint::Text { block, color, shadow, x, y } => {
                6u8.hash(h);
                block.key.hash(h);
                hash_f(color, h);
                if let Some((dy, s, c)) = shadow {
                    hash_f(&[*dy, *s], h);
                    hash_f(c, h);
                }
                hash_f(&[*x, *y], h);
            }
        }
    }
}

/// `a ∘ b`: najpierw `b`, potem `a`.
pub fn mul(a: Transform, b: Transform) -> Transform {
    a.pre_concat(b)
}
