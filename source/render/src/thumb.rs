//! Miniatury stron (`vizrs-render thumb`): kadr klatki przycięty do miejsca, w którym strona rysuje,
//! i zmniejszony do zadanej szerokości. Używa ich VizRS CG Playout (miniatury elementów graficznych
//! w panelu VizRS Live) – opis: docs/DOKUMENTACJA.md §10.1.

use serde_json::{Map, Value};
use tiny_skia::Pixmap;

use crate::host::Host;
use crate::model::Scene;
use crate::res::Resources;
use crate::scene::Clock;

/// Prostokąt (x, y, w, h) w pikselach klatki.
pub type Rect = (u32, u32, u32, u32);

/// Najmniejsza alfa, od której piksel liczy się jako narysowany (cień i wygładzanie krawędzi pomija).
const ALPHA_MIN: u8 = 8;

/// Obszar narysowanych pikseli klatki RGBA (`w` × `h`); `None`, gdy klatka jest pusta.
pub fn drawn(rgba: &[u8], w: u32, h: u32) -> Option<Rect> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..h {
        let row = &rgba[(y * w * 4) as usize..((y + 1) * w * 4) as usize];
        let mut first = None;
        let mut last = 0;
        for x in 0..w {
            if row[(x * 4 + 3) as usize] >= ALPHA_MIN {
                first.get_or_insert(x);
                last = x;
            }
        }
        if let Some(f) = first {
            x0 = x0.min(f);
            x1 = x1.max(last);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
    }
    (x0 != u32::MAX).then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

/// Kadr miniatury: obszar `r` z marginesem `pad`, proporcje w granicach 1:1 … 4:1 (pasek dolny nie robi się
/// kreską, a pionowa grafika wąskim paskiem) – dłuższy bok zostaje, krótszy rośnie wokół środka; wszystko w klatce.
pub fn frame((x, y, w, h): Rect, pad: u32, (fw, fh): (u32, u32)) -> Rect {
    let grow = |start: u32, len: u32, want: u32, limit: u32| -> (u32, u32) {
        let want = want.min(limit).max(len);
        let extra = want - len;
        let s = start.saturating_sub(extra / 2).min(limit - want);
        (s, want)
    };
    let (x, w) = grow(x, w, w + 2 * pad, fw);
    let (y, h) = grow(y, h, h + 2 * pad, fh);
    if w > 4 * h {
        let (y, h) = grow(y, h, w.div_ceil(4), fh);
        (x, y, w, h)
    } else if h > w {
        let (x, w) = grow(x, w, h, fw);
        (x, y, w, h)
    } else {
        (x, y, w, h)
    }
}

/// Wycinek `r` klatki RGBA (`fw` szerokości) zmniejszony do szerokości `out_w` (bez powiększania)
/// średnią z pól (premultiplied – bez obwódek na krawędziach przezroczystości); (dane, szerokość, wysokość).
pub fn scale(rgba: &[u8], fw: u32, (x, y, w, h): Rect, out_w: u32) -> (Vec<u8>, u32, u32) {
    let ow = out_w.clamp(1, w);
    let oh = ((h as u64 * ow as u64 + w as u64 / 2) / w as u64).max(1) as u32;
    let mut out = vec![0u8; (ow * oh * 4) as usize];
    for oy in 0..oh {
        let (sy0, sy1) = (y + oy * h / oh, (y + (oy + 1) * h / oh).max(y + oy * h / oh + 1));
        for ox in 0..ow {
            let (sx0, sx1) = (x + ox * w / ow, (x + (ox + 1) * w / ow).max(x + ox * w / ow + 1));
            let mut acc = [0u64; 4];
            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let i = ((sy * fw + sx) * 4) as usize;
                    for c in 0..4 {
                        acc[c] += rgba[i + c] as u64;
                    }
                }
            }
            let n = ((sy1 - sy0) * (sx1 - sx0)) as u64;
            let o = ((oy * ow + ox) * 4) as usize;
            for c in 0..4 {
                out[o + c] = ((acc[c] + n / 2) / n) as u8;
            }
        }
    }
    (out, ow, oh)
}

/// Chwila zrzutu od TAKE (s): pierwszy stop point i pół sekundy spoczynku (wejście skończone), strona bez
/// stop pointów – połowa jej długości (przed zejściem), bez długości – 2 s.
pub fn moment(scene: &Scene) -> f64 {
    match scene.stops.iter().copied().filter(|s| *s >= 0.0).reduce(f64::min) {
        Some(stop) => stop + 0.5,
        None if scene.duration > 0.0 => scene.duration / 2.0,
        None => 2.0,
    }
}

/// Strona zagrana jak na antenie (TAKE w rytmie `fps`) do chwili `moment`; klatka i to, czego renderer natywny nie umie.
pub fn render(
    mut host: Host,
    scene_v: &Value,
    values: &Map<String, Value>,
    layer: &str,
    size: (u32, u32),
    fps: f64,
    res: &Resources,
) -> Result<(Pixmap, Vec<String>), String> {
    let until = moment(&Scene::from_json(scene_v)) * 1000.0;
    let step = 1000.0 / fps.max(1.0);
    host.handle(&serde_json::json!({"t":"take","layer":layer,"scene":scene_v,"values":values,"since":1}), Clock { now: 0.0, wall: 0.0 });
    let mut now = 0.0;
    while now <= until {
        host.tick(Clock { now, wall: now }, res);
        now += step;
    }
    let mut pix = Pixmap::new(size.0, size.1).ok_or("zły rozmiar")?;
    host.draw(&mut pix, size, (0, 0), res);
    Ok((pix, host.issues()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(w: u32, h: u32, rects: &[Rect]) -> Vec<u8> {
        let mut d = vec![0u8; (w * h * 4) as usize];
        for &(x, y, rw, rh) in rects {
            for yy in y..y + rh {
                for xx in x..x + rw {
                    let i = ((yy * w + xx) * 4) as usize;
                    d[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
                }
            }
        }
        d
    }

    #[test]
    fn drawn_area_and_empty_frame() {
        assert_eq!(drawn(&canvas(100, 50, &[]), 100, 50), None);
        assert_eq!(drawn(&canvas(100, 50, &[(10, 5, 20, 4), (60, 30, 5, 5)]), 100, 50), Some((10, 5, 55, 30)));
    }

    #[test]
    fn faint_pixels_do_not_count() {
        let mut d = canvas(10, 10, &[(2, 2, 2, 2)]);
        d[(9 * 10 + 9) * 4 + 3] = ALPHA_MIN - 1;
        assert_eq!(drawn(&d, 10, 10), Some((2, 2, 2, 2)));
    }

    #[test]
    fn frame_keeps_the_proportions_within_bounds() {
        // pasek dolny 1600×100: 4:1 wokół środka, przesunięty w górę do krawędzi klatki
        assert_eq!(frame((160, 940, 1600, 100), 0, (1920, 1080)), (160, 680, 1600, 400));
        // pasek przy dolnej krawędzi nie wychodzi poza klatkę
        assert_eq!(frame((0, 1040, 1920, 40), 0, (1920, 1080)), (0, 600, 1920, 480));
        // wąska pionowa grafika – kwadrat
        assert_eq!(frame((1800, 100, 40, 400), 0, (1920, 1080)), (1520, 100, 400, 400));
        // margines przycięty do klatki, proporcje zwykłe – bez zmian
        assert_eq!(frame((10, 10, 300, 200), 24, (1920, 1080)), (0, 0, 348, 248));
    }

    #[test]
    fn scaling_averages_and_never_enlarges() {
        let d = canvas(4, 2, &[(0, 0, 2, 2)]);
        let (out, w, h) = scale(&d, 4, (0, 0, 4, 2), 2);
        assert_eq!((w, h), (2, 1));
        assert_eq!(&out[..], &[255, 255, 255, 255, 0, 0, 0, 0]);
        let (_, w, h) = scale(&d, 4, (0, 0, 4, 2), 400);
        assert_eq!((w, h), (4, 2));
        let (out, w, h) = scale(&d, 4, (1, 0, 2, 2), 1);
        assert_eq!((w, h, out[3]), (1, 1, 128));
    }
}
