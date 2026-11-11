//! Host warstw – odpowiednik `LayerHost` z `renderer.js`: zdarzenia anteny z `/events` (snapshot, take, continue,
//! out, update, clear, rescene, relayer, dock), filtr warstw (`?layers=`) i szyny (`?bus=`), powiązania (dock).
//! Grafika schodząca z warstwy gra swoje wyjście dalej, obok nowej (jak stary `ScenePlayer` w tym samym `div`).

use std::hash::{DefaultHasher, Hasher};

use serde_json::{Map, Value};
use tiny_skia::{Pixmap, Transform};

use crate::display::{px_union, PxRect, Sig};
use crate::model::{layer_z, num_of, str_of, Scene};
use crate::raster::Raster;
use crate::res::Resources;
use crate::scene::{Clock, Output, Player};

struct Layer {
    name: String,
    /// Kolejność utworzenia (remis `z-index` rozstrzyga kolejność w dokumencie).
    order: u64,
    /// Grafiki warstwy: ostatnia to bieżąca (`L.player`), wcześniejsze kończą wyjście.
    players: Vec<Player>,
    current: bool,
    since: Value,
    page: Value,
}

pub struct Host {
    layers: Vec<Layer>,
    next_order: u64,
    filter: Option<Vec<String>>,
    bus: String,
    dock_off: Vec<String>,
    pub output: Output,
    /// Wymiar sceny (strona /output: `?w=&h=`, domyślnie 1920×1080).
    pub stage: (f64, f64),
    last_tick: Option<f64>,
    pub raster: Raster,
    last_print: Option<u64>,
    /// Rysowanie tylko obszaru zmiany (domyślnie tak; `false` – zawsze cały obraz).
    pub partial: bool,
    /// Obszar (px kanału) ostatniego obrazu rysowanego częściowo; `None` – rysowany w całości.
    /// Poza nim płótna mają piksele poprzedniego obrazu.
    pub last_damage: Option<PxRect>,
    drawn: Option<Drawn>,
}

/// Co narysowano ostatnio – podstawa rysowania częściowego następnej klatki.
struct Drawn {
    /// Skrót tego, co zmienia cały obraz: zasoby (fonty, obrazy), rozmiary płócien, przekształcenia sceny.
    setup: u64,
    sigs: Vec<Sig>,
    /// Klatki narysowane częściowo od ostatniego pełnego rysowania.
    partial_run: u32,
}

/// Po tylu klatkach rysowanych częściowo jedna jest rysowana w całości (przy 50 kl./s – co sekundę):
/// zabezpieczenie na wypadek obwiedni, która czegoś nie objęła – ślad zniknie najpóźniej po tylu klatkach.
const FULL_EVERY: u32 = 50;
/// Obszar zmiany większy niż ta część obrazu – rysowanie całości (taniej niż płótno pomocnicze).
const PARTIAL_MAX_SHARE: f64 = 0.5;

fn values_of(v: Option<&Value>) -> Map<String, Value> {
    v.and_then(Value::as_object).cloned().unwrap_or_default()
}

impl Host {
    pub fn new(filter: Option<Vec<String>>, bus: &str) -> Host {
        Host {
            layers: Vec::new(),
            next_order: 0,
            filter,
            bus: if bus == "preview" || bus.starts_with("ch-") { bus.into() } else { "program".into() },
            dock_off: Vec::new(),
            output: Output::default(),
            stage: (1920.0, 1080.0),
            last_tick: None,
            raster: Raster::new(),
            last_print: None,
            partial: true,
            last_damage: None,
            drawn: None,
        }
    }

    fn accepts(&self, name: &str) -> bool {
        self.filter.as_ref().is_none_or(|f| f.iter().any(|x| x == name))
    }

    fn layer(&mut self, name: &str) -> usize {
        if let Some(i) = self.layers.iter().position(|l| l.name == name) {
            return i;
        }
        self.next_order += 1;
        self.layers.push(Layer {
            name: name.to_string(),
            order: self.next_order,
            players: Vec::new(),
            current: false,
            since: Value::Null,
            page: Value::Null,
        });
        self.layers.len() - 1
    }

    fn current(&mut self, name: &str) -> Option<&mut Player> {
        let l = self.layers.iter_mut().find(|l| l.name == name)?;
        if !l.current {
            return None;
        }
        l.players.last_mut().filter(|p| !p.dead)
    }

    pub fn take(&mut self, name: &str, scene: &Value, values: Map<String, Value>, since: Value, page: Value, clk: Clock) {
        if !self.accepts(name) || !scene.is_object() {
            return;
        }
        let i = self.layer(name);
        let l = &mut self.layers[i];
        if l.current {
            if let Some(p) = l.players.last_mut() {
                p.out();
            }
        }
        let mut p = Player::new(Scene::from_json(scene), values, clk);
        p.take(clk);
        l.players.push(p);
        l.current = true;
        l.since = since;
        l.page = page;
    }

    pub fn out(&mut self, name: &str) {
        if let Some(l) = self.layers.iter_mut().find(|l| l.name == name) {
            if l.current {
                if let Some(p) = l.players.last_mut() {
                    p.out();
                }
            }
            l.current = false;
        }
    }

    pub fn clear(&mut self) {
        for l in &mut self.layers {
            l.players.clear();
            l.current = false;
        }
    }

    /// Stan anteny po (ponownym) połączeniu: te same strony (`since`) zostają nietknięte.
    pub fn snapshot(&mut self, list: &[Value], clk: Clock) {
        let mut keep = Vec::new();
        for l in list {
            let name = l.get("layer").map(str_of).unwrap_or_default();
            if !self.accepts(&name) {
                continue;
            }
            let i = self.layer(&name);
            keep.push(name.clone());
            let since = l.get("since").cloned().unwrap_or(Value::Null);
            let values = values_of(l.get("values"));
            let layer = &mut self.layers[i];
            let same = layer.current && !since.is_null() && layer.since == since && layer.players.last().is_some_and(|p| !p.dead);
            if same {
                if let Some(p) = layer.players.last_mut() {
                    p.update(values);
                }
                continue;
            }
            layer.players.clear();
            let Some(scene) = l.get("scene").filter(|s| s.is_object()) else {
                layer.current = false;
                continue;
            };
            let mut p = Player::new(Scene::from_json(scene), values, clk);
            p.seek_stop(l.get("stop").map(num_of).filter(|s| s.is_finite() && *s >= 0.0).unwrap_or(0.0) as usize);
            layer.players.push(p);
            layer.current = true;
            layer.since = since;
            layer.page = l.get("page").cloned().unwrap_or(Value::Null);
        }
        for l in &mut self.layers {
            if !keep.contains(&l.name) {
                l.players.clear();
                l.current = false;
            }
        }
    }

    fn relayer(&mut self, ev: &Value, clk: Clock) {
        let from = ev.get("layer").map(str_of).unwrap_or_default();
        let to = ev.get("to").map(str_of).unwrap_or_default();
        if from.is_empty() || to.is_empty() || from == to {
            return;
        }
        if let Some(pos) = self.dock_off.iter().position(|x| *x == from) {
            self.dock_off.remove(pos);
            self.dock_off.push(to.clone());
        } else {
            self.dock_off.retain(|x| *x != to);
        }
        let live = self.layers.iter().any(|l| l.name == from && l.current && l.players.last().is_some_and(|p| !p.dead));
        if !self.accepts(&to) {
            if live {
                self.out(&from);
            }
            return;
        }
        if !live {
            let scene = ev.get("scene").cloned().unwrap_or(Value::Null);
            self.take(
                &to,
                &scene,
                values_of(ev.get("values")),
                ev.get("since").cloned().unwrap_or(Value::Null),
                ev.get("page").cloned().unwrap_or(Value::Null),
                clk,
            );
            let stop = ev.get("stop").map(num_of).unwrap_or(0.0);
            if stop > 0.0 {
                if let Some(p) = self.current(&to) {
                    p.seek_stop(stop as usize);
                }
            }
            return;
        }
        self.out(&to);
        let a = self.layer(&from);
        let b = self.layer(&to);
        // warstwy zamieniają się miejscami: grafika zostaje, zmienia się nazwa (a więc i kolejność)
        let (oa, ob) = (self.layers[a].order, self.layers[b].order);
        self.layers[a].name = to;
        self.layers[b].name = from;
        self.layers[a].order = ob;
        self.layers[b].order = oa;
        self.layers[b].players.retain(|p| !p.dead);
    }

    /// Powiązania: scena z regułami `dock` przesuwa się, gdy wskazana grafika jest na antenie.
    fn recompute_dock(&mut self, now: f64) {
        let air: Vec<(String, String, String)> = self
            .layers
            .iter()
            .filter(|l| l.current && l.players.last().is_some_and(|p| !p.dead))
            .map(|l| {
                let page = l.page.get("number").map(str_of).unwrap_or_default();
                (l.name.clone(), page, l.players.last().map(|p| p.scene.name.clone()).unwrap_or_default())
            })
            .collect();
        for l in &mut self.layers {
            if !l.current {
                continue;
            }
            let name = l.name.clone();
            let off = self.dock_off.contains(&name);
            let Some(p) = l.players.last_mut().filter(|p| !p.dead) else { continue };
            let (mut dx, mut dy, mut dur) = (0.0, 0.0, 0.6);
            if !off {
                for r in &p.scene.dock {
                    let target = r.get("target").map(str_of).unwrap_or_default();
                    let hit = target.split('|').any(|t| {
                        let (kind, val) = t.split_once(':').unwrap_or((t, ""));
                        air.iter().any(|(layer, page, scene)| {
                            *layer != name
                                && match kind {
                                    "layer" => layer == val,
                                    "page" => page == val,
                                    "scene" => scene == val,
                                    _ => false,
                                }
                        })
                    });
                    if hit {
                        dx += r.get("dx").map(num_of).filter(|x| x.is_finite()).unwrap_or(0.0);
                        dy += r.get("dy").map(num_of).filter(|x| x.is_finite()).unwrap_or(0.0);
                        if let Some(d) = r.get("dur").filter(|d| !d.is_null()) {
                            dur = num_of(d);
                        }
                    }
                }
            }
            p.set_dock(dx, dy, dur, now);
        }
    }

    /// Zdarzenie z `/events`.
    pub fn handle(&mut self, ev: &Value, clk: Clock) {
        let bus = ev.get("bus").map(str_of).filter(|b| !b.is_empty()).unwrap_or_else(|| "program".into());
        if bus != self.bus {
            return;
        }
        let layer = ev.get("layer").map(str_of).unwrap_or_default();
        match ev.get("t").and_then(Value::as_str).unwrap_or("") {
            "snapshot" => {
                self.dock_off = ev.get("dockOff").and_then(Value::as_array).map(|a| a.iter().map(str_of).collect()).unwrap_or_default();
                let list = ev.get("layers").and_then(Value::as_array).cloned().unwrap_or_default();
                self.snapshot(&list, clk);
                self.recompute_dock(clk.now);
            }
            "take" => {
                let scene = ev.get("scene").cloned().unwrap_or(Value::Null);
                self.take(
                    &layer,
                    &scene,
                    values_of(ev.get("values")),
                    ev.get("since").cloned().unwrap_or(Value::Null),
                    ev.get("page").cloned().unwrap_or(Value::Null),
                    clk,
                );
                self.recompute_dock(clk.now);
            }
            "dock" => {
                if ev.get("off").is_some_and(crate::model::truthy) {
                    if !self.dock_off.contains(&layer) {
                        self.dock_off.push(layer);
                    }
                } else {
                    self.dock_off.retain(|x| *x != layer);
                }
                self.recompute_dock(clk.now);
            }
            "rescene" => {
                if let Some(scene) = ev.get("scene").filter(|s| s.is_object()) {
                    let stop = ev.get("stop").map(num_of).filter(|s| s.is_finite() && *s >= 0.0).unwrap_or(0.0) as usize;
                    if let Some(p) = self.current(&layer) {
                        p.set_scene(Scene::from_json(scene));
                        if !p.playing {
                            p.seek_stop(stop);
                        }
                    }
                }
            }
            "relayer" => {
                self.relayer(ev, clk);
                self.recompute_dock(clk.now);
            }
            "continue" => {
                if let Some(p) = self.current(&layer) {
                    p.cont();
                }
            }
            "out" => {
                self.out(&layer);
                self.recompute_dock(clk.now);
            }
            "update" => {
                let v = values_of(ev.get("values"));
                if let Some(p) = self.current(&layer) {
                    p.update(v);
                }
            }
            "clear" => {
                self.clear();
                self.recompute_dock(clk.now);
            }
            _ => {}
        }
    }

    /// Krok animacji całej anteny (jak `loop` z `renderer.js`: dt najwyżej 0,1 s).
    pub fn tick(&mut self, clk: Clock, res: &Resources) {
        // bez grafik pętla zasypia i zapomina czas (`last = 0`): pierwsza klatka po TAKE ma dt = 0
        if self.layers.iter().all(|l| l.players.is_empty()) {
            self.last_tick = None;
            return;
        }
        let dt = self.last_tick.map(|l| ((clk.now - l) / 1000.0).clamp(0.0, 0.1)).unwrap_or(0.0);
        self.last_tick = Some(clk.now);
        let out = self.output;
        for l in &mut self.layers {
            for p in &mut l.players {
                p.tick(dt, clk, res, out);
            }
            l.players.retain(|p| !p.dead);
        }
    }

    /// Czego nie da się narysować natywnie na antenie (puste = wszystko natywnie).
    pub fn issues(&self) -> Vec<String> {
        let mut out = Vec::new();
        for l in &self.layers {
            for p in &l.players {
                for i in &p.issues {
                    if !out.contains(i) {
                        out.push(i.clone());
                    }
                }
            }
        }
        out
    }

    /// Drzewa rysowania na stderr (VIZRS_RENDER_DEBUG).
    pub fn debug(&self) {
        for l in &self.layers {
            for p in &l.players {
                eprintln!("warstwa {} scena {} t={} {:?}", l.name, p.scene.label(), p.t, p.tree);
            }
        }
    }

    /// Czy na antenie jest jakakolwiek grafika.
    pub fn on_air(&self) -> bool {
        self.layers.iter().any(|l| l.players.iter().any(|p| !p.dead && p.tree.is_some()))
    }

    /// Przekształcenie sceny → piksele płótna `size` (jak `fitStage`: `contain`, wyśrodkowanie), z obszarem `area`.
    pub fn stage_transform(&self, size: (u32, u32), area: (i32, i32)) -> Transform {
        let (w, h) = (size.0 as f64, size.1 as f64);
        let s = (w / self.stage.0).min(h / self.stage.1);
        let ox = (w - self.stage.0 * s) / 2.0;
        let oy = (h - self.stage.1 * s) / 2.0;
        Transform::from_row(s as f32, 0.0, 0.0, s as f32, (ox - f64::from(area.0)) as f32, (oy - f64::from(area.1)) as f32)
    }

    /// Rysuje klatkę na `pix`. Zwraca false, gdy obraz nie zmienił się od poprzedniego (nic nie narysowano).
    pub fn draw(&mut self, pix: &mut Pixmap, size: (u32, u32), area: (i32, i32), res: &Resources) -> bool {
        self.draw_areas(&mut [(pix, area)], size, res)
    }

    /// Ta sama antena na kilku płótnach (obszary kanału): stan sceny jeden, rysowanie osobno na każdym.
    pub fn draw_areas(&mut self, areas: &mut [(&mut Pixmap, (i32, i32))], size: (u32, u32), res: &Resources) -> bool {
        let mut order: Vec<(f64, u64, usize)> = self.layers.iter().enumerate().map(|(i, l)| (layer_z(&l.name), l.order, i)).collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut h = DefaultHasher::new();
        h.write_u64(res.epoch());
        for (pix, area) in areas.iter() {
            let stage = self.stage_transform(size, *area);
            for v in [stage.sx, stage.sy, stage.tx, stage.ty] {
                h.write_u32(v.to_bits());
            }
            h.write_u32(pix.width());
            h.write_u32(pix.height());
        }
        let setup = h.finish();
        let trees: Vec<_> = order.iter().flat_map(|o| &self.layers[o.2].players).filter_map(|p| p.tree.as_ref()).collect();
        for t in &trees {
            t.fingerprint(&mut h);
        }
        let print = h.finish();
        if self.last_print == Some(print) {
            return false;
        }
        // obszar zmiany w px kanału (obszar (0, 0)); na płótnie obszaru – przesunięty o jego położenie
        let channel = self.stage_transform(size, (0, 0));
        let sigs: Vec<Sig> = trees.iter().map(|t| self.raster.sig(t, channel, res)).collect();
        let damage = match &self.drawn {
            Some(d) if self.partial && d.setup == setup && d.sigs.len() == sigs.len() && d.partial_run < FULL_EVERY => {
                sigs.iter().zip(&d.sigs).fold(None, |acc, (a, b)| px_union(acc, a.damage(b)))
            }
            _ => None,
        };
        let damage = damage.filter(|r| {
            let share = f64::from((r.2 - r.0).max(0)) * f64::from((r.3 - r.1).max(0)) / (f64::from(size.0) * f64::from(size.1));
            share <= PARTIAL_MAX_SHARE
        });
        let reach = damage.map_or(0.0, |_| trees.iter().fold(0.0f32, |r, t| r.max(Raster::effect_reach(t, channel))));
        for (pix, area) in areas.iter_mut() {
            let stage = self.stage_transform(size, *area);
            let scenes: Vec<_> = trees.iter().map(|t| (*t, stage)).collect();
            match damage {
                Some(r) => self.raster.draw_part(pix, &scenes, res, (r.0 - area.0, r.1 - area.1, r.2 - area.0, r.3 - area.1), reach),
                None => self.raster.draw(pix, &scenes, res),
            }
        }
        let partial_run = match (&self.drawn, damage) {
            (Some(d), Some(_)) => d.partial_run + 1,
            _ => 0,
        };
        self.last_damage = damage;
        self.drawn = Some(Drawn { setup, sigs, partial_run });
        self.last_print = Some(print);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn clk(now: f64) -> Clock {
        Clock { now, wall: now }
    }

    #[test]
    fn take_out_and_snapshot() {
        let mut h = Host::new(None, "program");
        let scene = json!({"name":"A","duration":2,"stops":[1],"nodes":[{"id":"r","type":"rect","props":{"w":10,"h":10}}]});
        h.handle(&json!({"t":"take","layer":"main","scene":scene,"values":{},"since":1}), clk(0.0));
        assert_eq!(h.layers.len(), 1);
        h.handle(&json!({"t":"out","layer":"main"}), clk(10.0));
        assert!(h.layers[0].players[0].mode == crate::scene::Mode::Out);
        h.handle(&json!({"t":"snapshot","layers":[{"layer":"front","scene":scene,"since":5,"stop":0}]}), clk(20.0));
        assert!(h.layers.iter().find(|l| l.name == "main").is_some_and(|l| l.players.is_empty()));
        assert!(h.current("front").is_some());
        h.handle(&json!({"t":"take","layer":"x","bus":"preview","scene":scene}), clk(30.0));
        assert!(h.layers.iter().all(|l| l.name != "x"));
    }

    #[test]
    fn first_frame_after_idle_has_no_time_step() {
        let res = Resources::new(crate::font::FontDb::new(), None, true);
        let mut h = Host::new(None, "program");
        h.tick(clk(0.0), &res);
        h.tick(clk(40.0), &res);
        let scene = json!({"duration":2,"stops":[1],"nodes":[]});
        h.handle(&json!({"t":"take","layer":"main","scene":scene}), clk(80.0));
        h.tick(clk(80.0), &res);
        assert_eq!(h.current("main").map(|p| p.t), Some(0.0));
        h.tick(clk(120.0), &res);
        assert!((h.current("main").map(|p| p.t).unwrap() - 0.04).abs() < 1e-9);
    }

    #[test]
    fn filter_skips_other_layers() {
        let mut h = Host::new(Some(vec!["front".into()]), "program");
        h.handle(&json!({"t":"take","layer":"main","scene":{"nodes":[]}}), clk(0.0));
        assert!(h.layers.is_empty());
    }

    /// Duża stała plansza z cieniem, stała rozmyta belka pod ruchomym prostokątem, mały ruchomy prostokąt z rozmyciem i cieniem, ruchoma grupa
    /// oraz koło pojawiające się w połowie – jak plansza z animowaną belką.
    fn moving_scene() -> Value {
        json!({"duration":4,"stops":[3],"nodes":[
            {"id":"board","type":"rect","props":{"x":200,"y":150,"w":1500,"h":500,"color":"#203040","shadow":20}},
            {"id":"bar","type":"rect","props":{"x":0,"y":880,"w":1920,"h":100,"color":"#4060a0","blur":8}},
            {"id":"dot","type":"rect","props":{"y":900,"w":120,"h":60,"color":"#e02020","blur":6,"shadow":10},
             "anim":{"x":[{"t":0,"v":0},{"t":3,"v":1700}]}},
            {"id":"g","type":"group","props":{"x":0,"y":700},"anim":{"y":[{"t":0,"v":700},{"t":3,"v":760}]},"children":[
                {"id":"in","type":"rect","props":{"x":300,"w":200,"h":40,"color":"#30c030"}}]},
            {"id":"late","type":"ellipse","props":{"x":1600,"y":40,"w":200,"h":200,"color":"#ffffff"},
             "anim":{"opacity":[{"t":0,"v":0},{"t":1.5,"v":0},{"t":2.5,"v":1}]}}
        ]})
    }

    /// Rysowanie częściowe daje te same piksele co rysowanie całego obrazu, klatka po klatce.
    fn partial_matches_full(size: (u32, u32), areas: &[(i32, i32, u32, u32)]) {
        let res = Resources::new(crate::font::FontDb::new(), None, true);
        let (mut a, mut b) = (Host::new(None, "program"), Host::new(None, "program"));
        b.partial = false;
        let mut pa: Vec<Pixmap> = areas.iter().map(|r| Pixmap::new(r.2, r.3).unwrap()).collect();
        let mut pb: Vec<Pixmap> = areas.iter().map(|r| Pixmap::new(r.2, r.3).unwrap()).collect();
        for h in [&mut a, &mut b] {
            h.handle(&json!({"t":"take","layer":"main","scene":moving_scene()}), clk(0.0));
        }
        let mut partial = 0;
        let mut cache = crate::stream::RegionCache::default();
        for f in 0..40 {
            let c = clk(f64::from(f) * 120.0);
            for (h, pixes) in [(&mut a, &mut pa), (&mut b, &mut pb)] {
                h.tick(c, &res);
                let mut list: Vec<_> = pixes.iter_mut().zip(areas).map(|(p, r)| (p, (r.0, r.1))).collect();
                h.draw_areas(&mut list, size, &res);
            }
            partial += usize::from(a.last_damage.is_some());
            if let ([x], [y]) = (&pa[..], &pb[..]) {
                // rekord z łatania (strumień) = rekord z całego obrazu
                assert!(cache.record(x, 0, a.last_damage) == crate::stream::region_record(y, 0), "klatka {f}: inny rekord");
            }
            for (x, y) in pa.iter().zip(&pb) {
                let bad = x.data().iter().zip(y.data()).filter(|(p, q)| p != q).count();
                assert_eq!(bad, 0, "klatka {f}: {bad} różnych bajtów");
            }
        }
        // ruch małych węzłów rysuje się częściowo; całość – przy wejściu i co FULL_EVERY klatek
        assert!(partial > 25, "rysowanie częściowe w {partial} klatkach");
    }

    #[test]
    fn partial_redraw_matches_full_redraw() {
        // płótna mniejsze niż kanał (szybki test w trybie debug): skala ¼ i ⅙ (ułamkowe położenia), dwa obszary
        partial_matches_full((480, 270), &[(0, 0, 480, 270)]);
        partial_matches_full((320, 180), &[(0, 0, 320, 180)]);
        partial_matches_full((480, 270), &[(0, 220, 480, 50), (375, 0, 105, 75)]);
    }

    #[test]
    fn damage_is_the_changed_node_only() {
        let res = Resources::new(crate::font::FontDb::new(), None, true);
        let mut h = Host::new(None, "program");
        h.handle(&json!({"t":"take","layer":"main","scene":moving_scene()}), clk(0.0));
        let mut pix = Pixmap::new(1920, 1080).unwrap();
        h.tick(clk(400.0), &res);
        h.draw(&mut pix, (1920, 1080), (0, 0), &res);
        h.tick(clk(440.0), &res);
        let channel = h.stage_transform((1920, 1080), (0, 0));
        let tree = h.layers[0].players[0].tree.as_ref().unwrap();
        let new = h.raster.sig(tree, channel, &res);
        let d = new.damage(&h.drawn.as_ref().unwrap().sigs[0]).unwrap();
        // ruchomy prostokąt i grupa (y ok. 700–990 z cieniem i rozmyciem), bez planszy (y 150–650)
        assert!(d.1 > 650 && d.3 < 1010, "obszar zmiany {d:?}");
        assert!(h.draw(&mut pix, (1920, 1080), (0, 0), &res) && h.last_damage.is_some());
    }

    #[test]
    fn stage_fits_like_fit_stage() {
        let h = Host::new(None, "program");
        let t = h.stage_transform((1280, 720), (0, 0));
        assert!((t.sx - 2.0 / 3.0).abs() < 1e-6 && t.tx == 0.0);
        let t = h.stage_transform((1920, 1080), (0, 900));
        assert_eq!(t.ty, -900.0);
    }
}
