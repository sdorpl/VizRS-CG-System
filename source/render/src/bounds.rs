//! Granice strony (`vizrs-render bounds`): gdzie na wyjściu strona cokolwiek rysuje przez całe życie –
//! wejście, kolejne stop pointy z chwilą spoczynku (pętle, tickery, zegary) i wyjście. Strona jest odtwarzana
//! jak na antenie, a każda klatka zaznacza komórki siatki, w których jest choć jeden piksel z alfą. Wynik to
//! prostokąty komórek w pikselach wyjścia – dla sidecara VizRS CG Playout (strefy grafiki, PLAN-strefy S6).

use serde_json::{Map, Value};
use tiny_skia::Pixmap;

use crate::host::Host;
use crate::model::Scene;
use crate::res::Resources;
use crate::scene::Clock;

/// Komórki siatki `cell` px na wyjściu `size`; `on[row * cols + col]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cells {
    /// Rozmiar wyjścia w px.
    pub size: (u32, u32),
    pub cell: u32,
    pub cols: usize,
    pub rows: usize,
    pub on: Vec<bool>,
}

impl Cells {
    pub fn new(size: (u32, u32), cell: u32) -> Cells {
        let cell = cell.max(1);
        let (cols, rows) = (size.0.div_ceil(cell) as usize, size.1.div_ceil(cell) as usize);
        Cells { size, cell, cols, rows, on: vec![false; cols * rows] }
    }

    /// Zaznacza komórki z pikselami o alfie > 0 (dane premultiplied, alfa w 4. bajcie).
    pub fn mark(&mut self, pix: &Pixmap) {
        let (w, h) = (pix.width() as usize, pix.height() as usize);
        let data = pix.data();
        let cell = self.cell as usize;
        for y in 0..h {
            let row = &data[y * w * 4..(y + 1) * w * 4];
            let base = (y / cell) * self.cols;
            let mut x = 0;
            while x < w {
                let col = x / cell;
                let end = ((col + 1) * cell).min(w);
                if !self.on[base + col] && row[x * 4 + 3..end * 4].iter().step_by(4).any(|a| *a > 0) {
                    self.on[base + col] = true;
                }
                x = end;
            }
        }
    }

    /// Prostokąty `[x, y, w, h]` w px wyjścia: poziome serie komórek, sklejone z identycznymi seriami w wierszach
    /// pod nimi (wynik pokrywa dokładnie zaznaczone komórki), przycięte do `size`.
    pub fn rects(&self, size: (u32, u32), scale: u32) -> Vec<[u32; 4]> {
        let mut runs: Vec<(usize, usize, usize, usize)> = Vec::new(); // (c0, c1, r0, r1) – r1 wyłącznie
        for r in 0..self.rows {
            let mut c = 0;
            while c < self.cols {
                if !self.on[r * self.cols + c] {
                    c += 1;
                    continue;
                }
                let c0 = c;
                while c < self.cols && self.on[r * self.cols + c] {
                    c += 1;
                }
                match runs.iter_mut().find(|run| run.0 == c0 && run.1 == c && run.3 == r) {
                    Some(run) => run.3 = r + 1,
                    None => runs.push((c0, c, r, r + 1)),
                }
            }
        }
        let cell = self.cell * scale.max(1);
        runs.into_iter()
            .map(|(c0, c1, r0, r1)| {
                let (x, y) = (c0 as u32 * cell, r0 as u32 * cell);
                [x, y, (c1 as u32 * cell).min(size.0) - x, (r1 as u32 * cell).min(size.1) - y]
            })
            .collect()
    }
}

/// Jak długo strona jest odtwarzana.
#[derive(Clone, Copy, Debug)]
pub struct Timing {
    /// Klatki na sekundę próbkowania.
    pub fps: f64,
    /// Spoczynek na każdym stop poincie (i po końcu sceny bez stop pointów): pętle, tickery, zegary.
    pub hold: f64,
    /// Najdłużej na wyjście po OUT.
    pub out_max: f64,
}

impl Default for Timing {
    fn default() -> Timing {
        Timing { fps: 25.0, hold: 4.0, out_max: 10.0 }
    }
}

/// Wynik dla strony: komórki i to, czego renderer natywny nie umie (granice wtedy niepełne).
pub struct PageBounds {
    pub cells: Cells,
    pub issues: Vec<String>,
}

/// Odtwarza stronę (`scene_v` z wartościami `values`) na warstwie `layer` i zaznacza komórki pustej siatki `cells`
/// (rysuje w jej rozmiarze). `host` ma już ustawioną scenę wyjścia (`stage`).
pub fn page(
    mut host: Host,
    scene_v: &Value,
    values: &Map<String, Value>,
    layer: &str,
    mut cells: Cells,
    timing: Timing,
    res: &Resources,
) -> Result<PageBounds, String> {
    let scene = Scene::from_json(scene_v);
    let size = cells.size;
    let mut pix = Pixmap::new(size.0, size.1).ok_or("zły rozmiar")?;
    let step = 1000.0 / timing.fps.max(1.0);
    let mut now = 0.0f64;
    let mut run = |host: &mut Host, cells: &mut Cells, now: &mut f64, seconds: f64| {
        let end = *now + seconds * 1000.0;
        while *now <= end {
            host.tick(Clock { now: *now, wall: *now }, res);
            pix.fill(tiny_skia::Color::TRANSPARENT);
            host.draw(&mut pix, size, (0, 0), res);
            cells.mark(&pix);
            if !host.on_air() {
                break;
            }
            *now += step;
        }
    };
    host.handle(&serde_json::json!({"t":"take","layer":layer,"scene":scene_v,"values":values,"since":1}), Clock { now, wall: now });
    // kolejne stop pointy: dojście do nich i spoczynek, potem CONTINUE; po ostatnim – OUT
    let stops: Vec<f64> = scene.stops.iter().copied().filter(|s| *s >= 0.0 && (scene.duration <= 0.0 || *s <= scene.duration)).collect();
    let mut at = 0.0;
    if stops.is_empty() {
        run(&mut host, &mut cells, &mut now, scene.duration.max(0.0) + timing.hold);
    }
    for (i, stop) in stops.iter().enumerate() {
        run(&mut host, &mut cells, &mut now, (stop - at).max(0.0) + timing.hold);
        at = *stop;
        if i + 1 < stops.len() {
            host.handle(&serde_json::json!({"t":"continue","layer":layer}), Clock { now, wall: now });
        }
    }
    let issues = host.issues();
    host.handle(&serde_json::json!({"t":"out","layer":layer}), Clock { now, wall: now });
    run(&mut host, &mut cells, &mut now, timing.out_max);
    Ok(PageBounds { cells, issues })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FontDb;

    fn res() -> std::sync::Arc<Resources> {
        Resources::new(FontDb::new(), None, true)
    }

    #[test]
    fn cells_and_rects() {
        let mut pix = Pixmap::new(64, 32).unwrap();
        // piksel w komórce (1,0) i blok w komórkach (0..2, 1) przy komórce 16 px
        pix.data_mut()[(2 * 64 + 20) * 4 + 3] = 255;
        for x in 0..32 {
            pix.data_mut()[(20 * 64 + x) * 4 + 3] = 9;
        }
        let mut c = Cells::new((64, 32), 16);
        c.mark(&pix);
        assert_eq!(c.on, vec![false, true, false, false, true, true, false, false]);
        assert_eq!(c.rects((64, 32), 1), vec![[16, 0, 16, 16], [0, 16, 32, 16]]);
        // te same komórki przy rysowaniu w połowie rozmiaru: prostokąty w px wyjścia
        assert_eq!(c.rects((128, 64), 2), vec![[32, 0, 32, 32], [0, 32, 64, 32]]);
    }

    #[test]
    fn identical_runs_merge_down() {
        let mut c = Cells::new((48, 48), 16);
        for r in 0..3 {
            c.on[r * 3 + 1] = true;
        }
        assert_eq!(c.rects((48, 48), 1), vec![[16, 0, 16, 48]]);
    }

    #[test]
    fn a_moving_box_covers_its_whole_path() {
        // prostokąt 100x50 jedzie z x=0 do x=1700 w 3 s (stop na 3 s), potem OUT
        let scene = serde_json::json!({"duration":4,"stops":[3],"nodes":[
            {"id":"r","type":"rect","props":{"x":0,"y":900,"w":100,"h":50,"color":"#ff0000"},
             "anim":{"x":[{"t":0,"v":0},{"t":3,"v":1700}]}}]});
        let mut host = Host::new(None, "program");
        host.stage = (1920.0, 1080.0);
        let b =
            page(host, &scene, &Map::new(), "main", Cells::new((1920, 1080), 16), Timing { fps: 25.0, hold: 0.5, out_max: 2.0 }, &res())
                .unwrap();
        assert!(b.issues.is_empty(), "{:?}", b.issues);
        let rects = b.cells.rects((1920, 1080), 1);
        let x0 = rects.iter().map(|r| r[0]).min().unwrap();
        let x1 = rects.iter().map(|r| r[0] + r[2]).max().unwrap();
        let (y0, y1) = (rects.iter().map(|r| r[1]).min().unwrap(), rects.iter().map(|r| r[1] + r[3]).max().unwrap());
        assert!(x0 == 0 && x1 >= 1800, "the whole path: {rects:?}");
        assert!(y0 >= 880 && y1 <= 960, "only the strip: {rects:?}");
    }

    #[test]
    fn nothing_drawn_nothing_marked() {
        let scene = serde_json::json!({"duration":1,"nodes":[]});
        let b =
            page(Host::new(None, "program"), &scene, &Map::new(), "main", Cells::new((320, 180), 16), Timing::default(), &res()).unwrap();
        assert!(b.cells.rects((320, 180), 1).is_empty());
    }
}
