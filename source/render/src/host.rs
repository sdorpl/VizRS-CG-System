//! Host warstw – odpowiednik `LayerHost` z `renderer.js`: zdarzenia anteny z `/events` (snapshot, take, continue,
//! out, update, clear, rescene, relayer, dock), filtr warstw (`?layers=`) i szyny (`?bus=`), powiązania (dock).
//! Grafika schodząca z warstwy gra swoje wyjście dalej, obok nowej (jak stary `ScenePlayer` w tym samym `div`).

use std::hash::{DefaultHasher, Hasher};

use serde_json::{Map, Value};
use tiny_skia::{Pixmap, Transform};

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
}

fn values_of(v: Option<&Value>) -> Map<String, Value> {
    v.and_then(Value::as_object).cloned().unwrap_or_default()
}

impl Host {
    pub fn new(filter: Option<Vec<String>>, bus: &str) -> Host {
        Host {
            layers: Vec::new(),
            next_order: 0,
            filter,
            bus: if bus == "preview" { "preview".into() } else { "program".into() },
            dock_off: Vec::new(),
            output: Output::default(),
            stage: (1920.0, 1080.0),
            last_tick: None,
            raster: Raster::new(),
            last_print: None,
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
        let stage = self.stage_transform(size, area);
        let mut order: Vec<(f64, u64, usize)> = self.layers.iter().enumerate().map(|(i, l)| (layer_z(&l.name), l.order, i)).collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut h = DefaultHasher::new();
        h.write_u64(res.epoch());
        for v in [stage.sx, stage.sy, stage.tx, stage.ty] {
            h.write_u32(v.to_bits());
        }
        h.write_u32(pix.width());
        h.write_u32(pix.height());
        let mut scenes = Vec::new();
        for (_, _, i) in &order {
            for p in &self.layers[*i].players {
                if let Some(t) = &p.tree {
                    t.fingerprint(&mut h);
                    scenes.push((t, stage));
                }
            }
        }
        let print = h.finish();
        if self.last_print == Some(print) {
            return false;
        }
        self.last_print = Some(print);
        self.raster.draw(pix, &scenes, res);
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
    fn filter_skips_other_layers() {
        let mut h = Host::new(Some(vec!["front".into()]), "program");
        h.handle(&json!({"t":"take","layer":"main","scene":{"nodes":[]}}), clk(0.0));
        assert!(h.layers.is_empty());
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
