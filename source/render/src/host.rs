//! Host warstw – odpowiednik `LayerHost` z `renderer.js`: zdarzenia anteny z `/events` (snapshot, take, continue,
//! out, update, clear, rescene, relayer, dock), filtr warstw (`?layers=`) i szyny (`?bus=`), powiązania (dock).
//! Grafika schodząca z warstwy gra swoje wyjście dalej, obok nowej (jak stary `ScenePlayer` w tym samym `div`).

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hasher};

use serde_json::{Map, Value};
use tiny_skia::{Pixmap, Transform};

use crate::display::{px_union, PxRect, Sig};
use crate::model::{layer_z, num_of, str_of, Scene};
use crate::raster::Raster;
use crate::res::Resources;
use crate::scene::{out_ms, rest_rect, Clock, DockEase, Output, Player};

struct Layer {
    name: String,
    /// Kolejność utworzenia (remis `z-index` rozstrzyga kolejność w dokumencie).
    order: u64,
    /// Grafiki warstwy: ostatnia to bieżąca (`L.player`), wcześniejsze kończą wyjście.
    players: Vec<Player>,
    current: bool,
    since: Value,
    page: Value,
    /// Obrys bieżącej grafiki w spoczynku (powiązania krawędzi); `None` – jeszcze nie mierzony.
    rest: Option<Option<[f64; 4]>>,
}

/// Grafika na antenie w warstwie spoza filtra (`?layers=`): nie jest rysowana, ale bywa celem powiązań (dock).
/// Bez tego proces jednej warstwy (playout z `native_layers`) nie widzi celów z innych warstw.
struct Foreign {
    layer: String,
    page: String,
    scene: String,
    /// Szablon i wartości – reguły dock tej grafiki (łańcuch) i jej obrys w spoczynku.
    src: Scene,
    values: Map<String, Value>,
    rest: Option<Option<[f64; 4]>>,
}

/// Grafika w trakcie animacji OUT (do `until`, ms zegara klatek) – cel reguł z `back: "end"`.
struct Leaving {
    layer: String,
    page: String,
    src: Scene,
    values: Map<String, Value>,
    until: f64,
    /// Przesunięcie z powiązań w chwili zejścia (łańcuch).
    off: Option<(f64, f64)>,
    rest: Option<Option<[f64; 4]>>,
}

/// Grafika na antenie w przeliczeniu powiązań (`air` z `renderer.js`).
struct AirE {
    layer: String,
    page: String,
    scene: String,
    leaving: bool,
    off: Option<(f64, f64)>,
    rect: Option<[f64; 4]>,
}

/// Wynik reguł jednej grafiki: przesunięcie, czas i krzywa.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shift {
    dx: f64,
    dy: f64,
    dur: f64,
    ease: DockEase,
}

const DOCK_EDGES: [&str; 4] = ["top", "bottom", "left", "right"];

/// Część celu reguły (`layer:` / `page:` / `scene:`) pasuje do grafiki.
fn dock_match(part: &str, a: &AirE) -> bool {
    let (kind, val) = part.split_once(':').unwrap_or((part, ""));
    match kind {
        "layer" => a.layer == val,
        "page" => a.page == val,
        "scene" => a.scene == val,
        _ => false,
    }
}

/// Grafiki wyzwalające regułę (jak `dockHits`): bez własnej warstwy; schodzące tylko przy `back: "end"`.
fn dock_hits(r: &Value, air: &[AirE], me: &str) -> Vec<usize> {
    let target = r.get("target").map(str_of).unwrap_or_default();
    let back_end = r.get("back").and_then(Value::as_str) == Some("end");
    (0..air.len())
        .filter(|&i| {
            let a = &air[i];
            a.layer != me && (!a.leaving || back_end) && target.split('|').filter(|t| !t.is_empty()).any(|t| dock_match(t, a))
        })
        .collect()
}

/// Przesunięcie osi z reguły krawędzi (jak `edgeShift`): tylko odsuwanie, tylko przy nakładaniu na drugiej osi.
fn edge_shift(edge: &str, gap: f64, s: Option<[f64; 4]>, t: Option<[f64; 4]>) -> Option<f64> {
    let (s, t) = (s?, t?);
    Some(match edge {
        "top" | "bottom" => {
            if !(s[0] < t[2] && t[0] < s[2]) {
                0.0
            } else if edge == "top" {
                (t[1] - gap - s[3]).min(0.0)
            } else {
                (t[3] + gap - s[1]).max(0.0)
            }
        }
        _ => {
            if !(s[1] < t[3] && t[1] < s[3]) {
                0.0
            } else if edge == "left" {
                (t[0] - gap - s[2]).min(0.0)
            } else {
                (t[2] + gap - s[0]).max(0.0)
            }
        }
    })
}

fn num_or(v: Option<&Value>, d: f64) -> f64 {
    v.filter(|x| !x.is_null()).map(num_of).filter(|x| x.is_finite()).unwrap_or(d)
}

/// `Math.round` z JS (połówki w górę).
fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// Przesunięcie z reguł `rules` grafiki z warstwy `me` (jak `dockShift` w `renderer.js`): reguły stałe się sumują,
/// reguły krawędzi – największe przesunięcie osi; `offset(h)` – przesunięcie celu z jego powiązań (łańcuch) albo `None`.
fn dock_shift(
    rules: &[Value],
    air: &[AirE],
    me: &str,
    own: Option<[f64; 4]>,
    offset: &mut dyn FnMut(usize) -> Option<(f64, f64)>,
) -> Shift {
    let (mut dx, mut dy, mut ex, mut ey) = (0.0, 0.0, 0.0f64, 0.0f64);
    let (mut dur, mut ease) = (0.6, DockEase::default());
    if let Some(r) = rules.last() {
        dur = num_or(r.get("dur"), dur);
        ease = DockEase::of(r.get("ease"));
    }
    let far = |a: f64, b: f64| if b.abs() > a.abs() { b } else { a };
    for r in rules {
        let hits = dock_hits(r, air, me);
        if hits.is_empty() {
            continue;
        }
        dur = num_or(r.get("dur"), dur);
        ease = DockEase::of(r.get("ease"));
        let edge = r.get("edge").and_then(Value::as_str).filter(|e| DOCK_EDGES.contains(e));
        let Some(edge) = edge else {
            dx += num_or(r.get("dx"), 0.0);
            dy += num_or(r.get("dy"), 0.0);
            continue;
        };
        let vert = edge == "top" || edge == "bottom";
        let fallback = num_or(r.get(if vert { "dy" } else { "dx" }), 0.0);
        let gap = num_or(r.get("gap"), 0.0);
        for h in hits {
            let t = air[h].rect.map(|t| match offset(h) {
                Some((ox, oy)) => [t[0] + ox, t[1] + oy, t[2] + ox, t[3] + oy],
                None => t,
            });
            let v = edge_shift(edge, gap, own, t).unwrap_or(fallback);
            if vert {
                ey = far(ey, v);
            } else {
                ex = far(ex, v);
            }
        }
    }
    Shift { dx: js_round(dx + ex), dy: js_round(dy + ey), dur, ease }
}

/// Warstwy w pętlach powiązań krawędzi (jak `dockCycles`): warstwa → warstwy tej samej pętli.
fn dock_cycles(deps: &[(String, Vec<String>)]) -> HashMap<String, HashSet<String>> {
    let get = |k: &str| deps.iter().find(|(x, _)| x == k).map(|(_, d)| d.clone()).unwrap_or_default();
    let reach = |from: &str| {
        let mut seen = HashSet::new();
        let mut st = get(from);
        while let Some(x) = st.pop() {
            if seen.insert(x.clone()) {
                st.extend(get(&x));
            }
        }
        seen
    };
    let r: HashMap<String, HashSet<String>> = deps.iter().map(|(k, _)| (k.clone(), reach(k))).collect();
    let mut out = HashMap::new();
    for (k, rk) in &r {
        if rk.contains(k) {
            out.insert(k.clone(), rk.iter().filter(|x| r.get(*x).is_some_and(|s| s.contains(k))).cloned().collect());
        }
    }
    out
}

pub struct Host {
    layers: Vec<Layer>,
    next_order: u64,
    filter: Option<Vec<String>>,
    bus: String,
    dock_off: Vec<String>,
    /// Obecność na antenie warstw spoza filtra (tylko do powiązań).
    foreign: Vec<Foreign>,
    /// Grafiki w trakcie animacji OUT – cele reguł z `back: "end"`.
    leaving: Vec<Leaving>,
    /// Ostatnie przesunięcia warstw (łańcuch dla grafik schodzących).
    shifts: HashMap<String, (f64, f64)>,
    /// Powiązania do przeliczenia w najbliższym `tick` (pomiar obrysów wymaga zasobów).
    dock_dirty: bool,
    /// Ostatnio zgłoszona pętla powiązań.
    loop_seen: String,
    /// Wersja zasobów (fonty, obrazy) przy ostatnim pomiarze obrysów: font wczytany w tle zmienia obrys tekstu.
    rest_epoch: u64,
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
            foreign: Vec::new(),
            leaving: Vec::new(),
            shifts: HashMap::new(),
            dock_dirty: false,
            loop_seen: String::new(),
            rest_epoch: 0,
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
            rest: None,
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

    /// Grafika w warstwie spoza filtra weszła (albo zmieniła scenę) – zapamiętana jako możliwy cel powiązań.
    fn foreign_set(&mut self, layer: &str, scene: &Value, page: &Value, values: Map<String, Value>) {
        self.foreign.retain(|f| f.layer != layer);
        if scene.is_object() {
            self.foreign.push(Foreign {
                layer: layer.to_string(),
                page: page.get("number").map(str_of).unwrap_or_default(),
                scene: scene.get("name").map(str_of).unwrap_or_default(),
                src: Scene::from_json(scene),
                values,
                rest: None,
            });
        }
    }

    /// Grafika warstwy `name` zaczyna OUT: do końca animacji wyjścia jest celem reguł z `back: "end"`.
    fn leave(&mut self, name: &str, now: f64) {
        let off = self.shifts.get(name).copied();
        let e = if let Some(f) = self.foreign.iter().find(|f| f.layer == name) {
            Some((f.page.clone(), f.src.clone(), f.values.clone(), f.rest))
        } else {
            self.layers.iter().find(|l| l.name == name && l.current).and_then(|l| {
                let p = l.players.last().filter(|p| !p.dead)?;
                Some((l.page.get("number").map(str_of).unwrap_or_default(), p.scene.clone(), p.values.clone(), l.rest))
            })
        };
        if let Some((page, src, values, rest)) = e {
            let ms = out_ms(&src);
            if ms > 0.0 {
                self.leaving.push(Leaving { layer: name.to_string(), page, src, values, until: now + ms, off, rest });
            }
        }
    }

    pub fn take(&mut self, name: &str, scene: &Value, values: Map<String, Value>, since: Value, page: Value, clk: Clock) {
        if !self.accepts(name) {
            self.leave(name, clk.now);
            self.foreign_set(name, scene, &page, values);
            return;
        }
        if !scene.is_object() {
            return;
        }
        self.leave(name, clk.now);
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
        l.rest = None;
    }

    pub fn out(&mut self, name: &str) {
        self.foreign.retain(|f| f.layer != name);
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
        self.foreign.clear();
        self.leaving.clear();
        for l in &mut self.layers {
            l.players.clear();
            l.current = false;
        }
    }

    /// Stan anteny po (ponownym) połączeniu: te same strony (`since`) zostają nietknięte.
    pub fn snapshot(&mut self, list: &[Value], clk: Clock) {
        let mut keep = Vec::new();
        self.foreign.clear();
        self.leaving.clear();
        for l in list {
            let name = l.get("layer").map(str_of).unwrap_or_default();
            if !self.accepts(&name) {
                self.foreign_set(
                    &name,
                    l.get("scene").unwrap_or(&Value::Null),
                    l.get("page").unwrap_or(&Value::Null),
                    values_of(l.get("values")),
                );
                continue;
            }
            let i = self.layer(&name);
            keep.push(name.clone());
            let since = l.get("since").cloned().unwrap_or(Value::Null);
            let values = values_of(l.get("values"));
            let layer = &mut self.layers[i];
            layer.rest = None;
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
        // obecność warstw spoza filtra: grafika przechodzi z `from` na `to`, dotychczasowa z `to` schodzi
        let was_foreign = self.foreign.iter().any(|f| f.layer == from);
        self.foreign.retain(|f| f.layer != from && f.layer != to);
        self.leaving.retain(|x| x.layer != from && x.layer != to);
        if !self.accepts(&to) {
            if live {
                self.out(&from);
            }
            if live || was_foreign {
                self.foreign_set(
                    &to,
                    ev.get("scene").unwrap_or(&Value::Null),
                    ev.get("page").unwrap_or(&Value::Null),
                    values_of(ev.get("values")),
                );
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

    /// Powiązania: scena z regułami `dock` przesuwa się, gdy wskazana grafika jest na antenie (jak `recomputeDock`
    /// w `renderer.js`). Liczone dla wszystkich warstw anteny (także spoza filtra – łańcuch), przesuwane tylko rysowane tutaj.
    fn recompute_dock(&mut self, now: f64, res: &Resources) {
        self.dock_dirty = false;
        self.leaving.retain(|x| x.until > now);
        let off_of =
            |dock_off: &[String], name: &str, rules: &[Value]| if dock_off.iter().any(|x| x == name) { Vec::new() } else { rules.to_vec() };
        // obrysy potrzebne tylko przy regułach krawędzi
        let edges = |rules: &[Value]| rules.iter().any(|r| r.get("edge").and_then(Value::as_str).is_some_and(|e| DOCK_EDGES.contains(&e)));
        let need = self.layers.iter().any(|l| l.current && l.players.last().is_some_and(|p| !p.dead && edges(&p.scene.dock)))
            || self.foreign.iter().any(|f| edges(&f.src.dock));
        if need {
            for l in &mut self.layers {
                if l.current && l.rest.is_none() {
                    if let Some(p) = l.players.last().filter(|p| !p.dead) {
                        l.rest = Some(rest_rect(&p.scene, &p.values, res));
                    }
                }
            }
            for f in &mut self.foreign {
                if f.rest.is_none() {
                    f.rest = Some(rest_rect(&f.src, &f.values, res));
                }
            }
            for x in &mut self.leaving {
                if x.rest.is_none() {
                    x.rest = Some(rest_rect(&x.src, &x.values, res));
                }
            }
        }
        // grafiki na antenie: najpierw bieżące (z regułami), potem schodzące
        let mut air = Vec::new();
        let mut rules: Vec<Vec<Value>> = Vec::new();
        for l in &self.layers {
            if let Some(p) = l.players.last().filter(|p| l.current && !p.dead) {
                air.push(AirE {
                    layer: l.name.clone(),
                    page: l.page.get("number").map(str_of).unwrap_or_default(),
                    scene: p.scene.name.clone(),
                    leaving: false,
                    off: None,
                    rect: l.rest.flatten(),
                });
                rules.push(off_of(&self.dock_off, &l.name, &p.scene.dock));
            }
        }
        for f in &self.foreign {
            air.push(AirE {
                layer: f.layer.clone(),
                page: f.page.clone(),
                scene: f.scene.clone(),
                leaving: false,
                off: None,
                rect: f.rest.flatten(),
            });
            rules.push(off_of(&self.dock_off, &f.layer, &f.src.dock));
        }
        let live = air.len();
        for x in &self.leaving {
            air.push(AirE {
                layer: x.layer.clone(),
                page: x.page.clone(),
                scene: x.src.name.clone(),
                leaving: true,
                off: x.off,
                rect: x.rest.flatten(),
            });
        }
        // pętle reguł krawędzi: łańcuch tylko poza pętlą
        let deps: Vec<(String, Vec<String>)> = (0..live)
            .map(|i| {
                let d = rules[i]
                    .iter()
                    .filter(|r| r.get("edge").and_then(Value::as_str).is_some_and(|e| DOCK_EDGES.contains(&e)))
                    .flat_map(|r| dock_hits(r, &air, &air[i].layer))
                    .filter(|&h| !air[h].leaving)
                    .map(|h| air[h].layer.clone())
                    .collect();
                (air[i].layer.clone(), d)
            })
            .collect();
        let cyc = dock_cycles(&deps);
        let mut lp: Vec<&String> = cyc.keys().collect();
        lp.sort();
        let lp = lp.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",");
        if !lp.is_empty() && lp != self.loop_seen {
            eprintln!("vizrs-render: pętla powiązań krawędzi ({lp}) – przesunięcie celów w pętli pominięte");
        }
        self.loop_seen = lp;
        struct Ctx<'a> {
            air: &'a [AirE],
            rules: &'a [Vec<Value>],
            cyc: &'a HashMap<String, HashSet<String>>,
            live: usize,
            memo: HashMap<usize, Shift>,
            busy: HashSet<usize>,
        }
        fn shift_of(c: &mut Ctx, i: usize) -> Shift {
            if let Some(s) = c.memo.get(&i) {
                return *s;
            }
            if !c.busy.insert(i) {
                return Shift { dx: 0.0, dy: 0.0, dur: 0.6, ease: DockEase::default() };
            }
            let (air, rules, cyc) = (c.air, c.rules, c.cyc);
            let me = air[i].layer.as_str();
            let same = cyc.get(me);
            let mut offset = |h: usize| -> Option<(f64, f64)> {
                if air[h].leaving {
                    return air[h].off;
                }
                if same.is_some_and(|s| s.contains(&air[h].layer)) || h >= c.live {
                    return None;
                }
                let s = shift_of(c, h);
                Some((s.dx, s.dy))
            };
            let s = dock_shift(&rules[i], air, me, air[i].rect, &mut offset);
            c.busy.remove(&i);
            c.memo.insert(i, s);
            s
        }
        let mut ctx = Ctx { air: &air, rules: &rules, cyc: &cyc, live, memo: HashMap::new(), busy: HashSet::new() };
        let shifts: Vec<Shift> = (0..live).map(|i| shift_of(&mut ctx, i)).collect();
        if std::env::var_os("VIZRS_RENDER_DOCK").is_some() {
            // diagnostyka powiązań: obrys w spoczynku i wynik każdej grafiki na antenie
            for (i, a) in air.iter().enumerate() {
                let s = shifts.get(i).map(|s| format!("{} {}", s.dx, s.dy)).unwrap_or_else(|| "schodzi".into());
                eprintln!("dock {} [{}] strona {} obrys {:?} przesunięcie {s}", a.layer, a.scene, a.page, a.rect);
            }
        }
        self.shifts = (0..live).map(|i| (air[i].layer.clone(), (shifts[i].dx, shifts[i].dy))).collect();
        for l in &mut self.layers {
            if !l.current {
                continue;
            }
            let Some(i) = air[..live].iter().position(|a| a.layer == l.name) else { continue };
            if let Some(p) = l.players.last_mut().filter(|p| !p.dead) {
                let s = shifts[i];
                p.set_dock(s.dx, s.dy, s.dur, s.ease, now);
            }
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
                self.dock_dirty = true;
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
                self.dock_dirty = true;
            }
            "dock" => {
                if ev.get("off").is_some_and(crate::model::truthy) {
                    if !self.dock_off.contains(&layer) {
                        self.dock_off.push(layer);
                    }
                } else {
                    self.dock_off.retain(|x| *x != layer);
                }
                self.dock_dirty = true;
            }
            "rescene" => {
                if !self.accepts(&layer) {
                    // nowa wersja szablonu może mieć inną nazwę – cele `scene:` liczą się od nowej
                    if let (Some(f), Some(scene)) =
                        (self.foreign.iter_mut().find(|f| f.layer == layer), ev.get("scene").filter(|s| s.is_object()))
                    {
                        f.scene = scene.get("name").map(str_of).unwrap_or_default();
                        f.src = Scene::from_json(scene);
                        f.rest = None;
                    }
                    self.dock_dirty = true;
                    return;
                }
                if let Some(scene) = ev.get("scene").filter(|s| s.is_object()) {
                    let stop = ev.get("stop").map(num_of).filter(|s| s.is_finite() && *s >= 0.0).unwrap_or(0.0) as usize;
                    if let Some(p) = self.current(&layer) {
                        p.set_scene(Scene::from_json(scene));
                        if !p.playing {
                            p.seek_stop(stop);
                        }
                    }
                    if let Some(l) = self.layers.iter_mut().find(|l| l.name == layer) {
                        l.rest = None;
                    }
                }
                // nowa wersja szablonu: inne reguły dock albo inna nazwa celu
                self.dock_dirty = true;
            }
            "relayer" => {
                self.relayer(ev, clk);
                self.dock_dirty = true;
            }
            "continue" => {
                if let Some(p) = self.current(&layer) {
                    p.cont();
                }
            }
            "out" => {
                self.leave(&layer, clk.now);
                self.out(&layer);
                self.dock_dirty = true;
            }
            "update" => {
                let v = values_of(ev.get("values"));
                // nowe wartości – inny obrys (np. dwa wiersze tekstu zamiast jednego)
                if let Some(f) = self.foreign.iter_mut().find(|f| f.layer == layer) {
                    f.values = v.clone();
                    f.rest = None;
                }
                if let Some(l) = self.layers.iter_mut().find(|l| l.name == layer) {
                    l.rest = None;
                }
                if let Some(p) = self.current(&layer) {
                    p.update(v);
                }
                self.dock_dirty = true;
            }
            "clear" => {
                self.clear();
                self.dock_dirty = true;
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
        // nowy font albo obraz (wczytany w tle, `stream`): obrysy mierzone bez niego są nieaktualne – jak `rectHosts` w renderer.js
        let epoch = res.epoch();
        if epoch != self.rest_epoch {
            self.rest_epoch = epoch;
            let mut had = false;
            for l in &mut self.layers {
                had |= l.rest.take().is_some();
            }
            for f in &mut self.foreign {
                had |= f.rest.take().is_some();
            }
            for x in &mut self.leaving {
                had |= x.rest.take().is_some();
            }
            self.dock_dirty |= had;
        }
        // powiązania: po zdarzeniach anteny i po końcu animacji OUT celu (back: "end"); przed ruchem klatki
        if self.dock_dirty || self.leaving.iter().any(|x| x.until <= clk.now) {
            self.recompute_dock(clk.now, res);
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

    fn test_res() -> std::sync::Arc<Resources> {
        Resources::new(crate::font::FontDb::new(), None, true)
    }

    /// Zdarzenie anteny i klatka (powiązania liczy `tick`: pomiar obrysów wymaga zasobów).
    fn ev(h: &mut Host, e: &Value, c: Clock) {
        h.handle(e, c);
        h.tick(c, &test_res());
    }

    fn rect(id: &str, x: f64, y: f64, w: f64, hh: f64) -> Value {
        json!({"id":id,"type":"rect","props":{"x":x,"y":y,"w":w,"h":hh,"color":"#ffffff"}})
    }

    /// B1: przesunięcie z obrysu celu i własnego (`edge`, `gap`), tylko przy kolizji; nowe wartości celu – nowy obrys.
    #[test]
    fn dock_edge_uses_rest_rects() {
        let mut belka = json!({"name":"Belka","duration":1,"stops":[0.5],"nodes":[rect("bar", 220.0, 859.0, 1650.0, 116.0)]});
        belka["nodes"][0]["fields"] = json!({"y":"top"});
        let ws = json!({"name":"WS","duration":1,"stops":[0.5],"x":0,"nodes":[rect("ws", 220.0, 930.0, 340.0, 44.0)],
            "dock":[{"target":"page:1001","edge":"top","gap":15,"dy":-50,"dur":0.6}]});
        let mut h = Host::new(None, "program");
        let at = |h: &mut Host, now: f64| h.current("z2_ws").map(|p| p.dock_at(now)).unwrap_or_default().1;
        ev(&mut h, &json!({"t":"take","layer":"z2_ws","scene":ws}), clk(0.0));
        ev(&mut h, &json!({"t":"take","layer":"main","scene":belka,"page":{"number":1001},"values":{"top":"859"}}), clk(1000.0));
        ev(&mut h, &json!({"t":"take","layer":"x"}), clk(1700.0));
        assert_eq!(at(&mut h, 1700.0), -130.0, "dół WS 15 px nad górą belki: 859 − 15 − 974");
        // wyższa belka (dwa wiersze) – dalej w górę
        ev(&mut h, &json!({"t":"update","layer":"main","values":{"top":"800"}}), clk(2000.0));
        assert_eq!(at(&mut h, 2700.0), -189.0);
        // cel bez kolizji (obok w poziomie) – bez ruchu
        let obok = json!({"name":"Obok","duration":1,"stops":[0.5],"nodes":[rect("o", 1000.0, 859.0, 300.0, 116.0)]});
        ev(&mut h, &json!({"t":"take","layer":"main","scene":obok,"page":{"number":1001}}), clk(3000.0));
        assert_eq!(at(&mut h, 3700.0), 0.0);
        // cel bez obrysu (nic nie rysuje) – zapasowe dy reguły
        let pusta = json!({"name":"Pusta","duration":1,"stops":[0.5],"nodes":[]});
        ev(&mut h, &json!({"t":"take","layer":"main","scene":pusta,"page":{"number":1001}}), clk(4000.0));
        assert_eq!(at(&mut h, 4700.0), -50.0);
    }

    /// B2 i B3: łańcuch (belka nad WS jedzie razem z nim), kilka celów krawędzi – największe przesunięcie, nie suma;
    /// pętla reguł krawędzi nie zawiesza przeliczenia; proces jednej warstwy liczy tak samo jak całe wyjście.
    #[test]
    fn dock_edge_chain_max_and_loops() {
        let pas = json!({"name":"Pas","duration":1,"stops":[0.5],"nodes":[rect("p", 0.0, 1000.0, 1920.0, 80.0)]});
        let belka = json!({"name":"Belka","duration":1,"stops":[0.5],"nodes":[rect("b", 200.0, 950.0, 1500.0, 40.0)],
            "dock":[{"target":"scene:Pas","edge":"top","gap":10}]});
        let ws = json!({"name":"WS","duration":1,"stops":[0.5],"nodes":[rect("w", 200.0, 930.0, 300.0, 40.0)],
            "dock":[{"target":"scene:Belka|scene:Pas","edge":"top","gap":5}]});
        let run = |filter: Option<Vec<String>>| {
            let mut h = Host::new(filter, "program");
            ev(&mut h, &json!({"t":"take","layer":"back","scene":pas}), clk(0.0));
            ev(&mut h, &json!({"t":"take","layer":"main","scene":belka}), clk(0.0));
            ev(&mut h, &json!({"t":"take","layer":"z2_ws","scene":ws}), clk(0.0));
            h
        };
        let mut h = run(None);
        let at = |h: &mut Host, l: &str| h.current(l).map(|p| p.dock_at(5000.0)).unwrap_or_default().1;
        // belka: 1000 − 10 − 990 = −0 → tylko gdy koliduje: 990 > 990? nie; dosuwamy do 950..990 → bez ruchu
        assert_eq!(at(&mut h, "main"), 0.0);
        // WS: przy belce (950 − 5 − 970 = −25) i pasie (1000 − 5 − 970 = 0 → 0) – największe: −25
        assert_eq!(at(&mut h, "z2_ws"), -25.0);
        // wyższa belka nachodzi na pas: belka w górę o 1000 − 10 − 1020 = −30, WS jedzie z nią: 920 − 5 − 970 = −55
        let belka2 = json!({"name":"Belka","duration":1,"stops":[0.5],"nodes":[rect("b", 200.0, 950.0, 1500.0, 70.0)],
            "dock":[{"target":"scene:Pas","edge":"top","gap":10}]});
        ev(&mut h, &json!({"t":"take","layer":"main","scene":belka2}), clk(0.0));
        ev(&mut h, &json!({"t":"take","layer":"y"}), clk(5000.0));
        assert_eq!((at(&mut h, "main"), at(&mut h, "z2_ws")), (-30.0, -55.0));
        // proces samej warstwy WS liczy łańcuch tak samo (obce warstwy: reguły i obrysy z treści zdarzeń)
        let mut solo = run(Some(vec!["z2_ws".into()]));
        ev(&mut solo, &json!({"t":"take","layer":"main","scene":belka2}), clk(0.0));
        ev(&mut solo, &json!({"t":"take","layer":"y"}), clk(5000.0));
        assert_eq!(at(&mut solo, "z2_ws"), -55.0);
        // pętla: A nad B, B nad A – przeliczenie się kończy, łańcuch w pętli pominięty
        let a = json!({"name":"A","duration":1,"stops":[0.5],"nodes":[rect("a", 0.0, 500.0, 100.0, 100.0)],"dock":[{"target":"scene:B","edge":"top"}]});
        let b = json!({"name":"B","duration":1,"stops":[0.5],"nodes":[rect("b", 0.0, 550.0, 100.0, 100.0)],"dock":[{"target":"scene:A","edge":"bottom"}]});
        let mut l = Host::new(None, "program");
        ev(&mut l, &json!({"t":"take","layer":"main","scene":a}), clk(0.0));
        ev(&mut l, &json!({"t":"take","layer":"back","scene":b}), clk(0.0));
        assert_eq!((at(&mut l, "main"), at(&mut l, "back")), (-50.0, 50.0));
        assert_eq!(l.loop_seen, "back,main");
    }

    /// B4: `back: "end"` – grafika wraca dopiero po animacji OUT celu (czas OUT: od ostatniego stop pointu do końca sceny).
    #[test]
    fn dock_back_after_target_out() {
        let lt = json!({"name":"LT","duration":2,"stops":[0.5],"nodes":[rect("l", 0.0, 900.0, 1920.0, 100.0)]});
        let ws = |back: &str| {
            json!({"name":"WS","duration":1,"stops":[0.5],"nodes":[rect("w", 0.0, 850.0, 300.0, 40.0)],
            "dock":[{"target":"scene:LT","dy":-100,"dur":0.5,"back":back}]})
        };
        let run = |back: &str| {
            let mut h = Host::new(Some(vec!["z2_ws".into()]), "program");
            ev(&mut h, &json!({"t":"take","layer":"main","scene":lt}), clk(0.0));
            ev(&mut h, &json!({"t":"take","layer":"z2_ws","scene":ws(back)}), clk(0.0));
            ev(&mut h, &json!({"t":"out","layer":"main"}), clk(1000.0));
            let mid = h.current("z2_ws").map(|p| p.dock_at(2400.0)).unwrap_or_default().1;
            ev(&mut h, &json!({"t":"x"}), clk(2600.0)); // koniec OUT celu: 1000 + 1500 ms
            let end = h.current("z2_ws").map(|p| p.dock_at(3200.0)).unwrap_or_default().1;
            (mid, end)
        };
        assert_eq!(run("start"), (0.0, 0.0), "domyślnie powrót w chwili OUT celu");
        assert_eq!(run("end"), (-100.0, 0.0), "back: end – na miejscu do końca OUT celu, potem powrót");
    }

    /// Font celu wczytywany w tle (`stream`): obrys zmierzony bez niego jest odświeżany po wczytaniu – reguła krawędzi
    /// przelicza się sama (dotąd do następnego zdarzenia zostawało przesunięcie z obrysu bez tekstu).
    #[test]
    fn dock_edge_remeasures_after_font_loads() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../web/fonts/barlow/barlow-latin-700-normal.woff2")).unwrap();
        let fetch: std::sync::Arc<crate::res::Fetch> = std::sync::Arc::new(move |_p: &str| {
            std::thread::sleep(std::time::Duration::from_millis(150));
            Ok(bytes.clone())
        });
        let res = Resources::new(crate::font::FontDb::new(), Some(fetch), false);
        let belka = json!({"name":"Belka","duration":1,"stops":[0.5],"nodes":[
            {"id":"t","type":"text","props":{"x":0,"y":800,"w":1000,"h":200,"text":"TEKST","font":"Test.woff2","size":60,"color":"#ffffff"}}]});
        let kafel = json!({"name":"Kafel","duration":1,"stops":[0.5],"nodes":[rect("k", 0.0, 950.0, 100.0, 50.0)],
            "dock":[{"target":"page:1001","edge":"top","gap":0,"dy":-5}]});
        let mut h = Host::new(Some(vec!["prog".into()]), "program");
        let step = |h: &mut Host, e: &Value, now: f64| {
            h.handle(e, clk(now));
            h.tick(clk(now), &res);
        };
        step(&mut h, &json!({"t":"take","layer":"prog","scene":kafel}), 0.0);
        step(&mut h, &json!({"t":"take","layer":"main","scene":belka,"page":{"number":1001}}), 0.0);
        let first = h.current("prog").map(|p| p.dock_at(1e9).1).unwrap_or_default();
        let mut now = 0.0;
        let mut last = first;
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            now += 50.0;
            h.tick(clk(now), &res);
            last = h.current("prog").map(|p| p.dock_at(1e9).1).unwrap_or_default();
            if last != first {
                break;
            }
        }
        assert_eq!(first, -5.0, "bez fontu – zapasowe dy");
        assert!(last < -20.0, "po wczytaniu fontu – obrys tekstu: {last}");
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

    /// Proces jednej warstwy (playout z `native_layers`, `?layers=prog`) widzi cel powiązania z innej warstwy:
    /// rysuje tylko swoją, ale obecność pozostałych na antenie zna z tych samych zdarzeń (take, out, snapshot, relayer, clear).
    #[test]
    fn dock_target_on_filtered_out_layer() {
        let belka = json!({"name":"Belka","duration":1,"stops":[0.5],"nodes":[]});
        let kafel = json!({"name":"Kafel","duration":1,"stops":[0.5],"dock":[{"target":"page:1001","dy":-90,"dur":0.6}],"nodes":[]});
        let mut h = Host::new(Some(vec!["prog".into()]), "program");
        let at = |h: &mut Host, now: f64| h.current("prog").map(|p| p.dock_at(now)).unwrap_or_default();
        ev(&mut h, &json!({"t":"take","layer":"prog","scene":kafel,"page":{"number":1405}}), clk(0.0));
        ev(&mut h, &json!({"t":"take","layer":"main","scene":belka,"page":{"number":1001}}), clk(1000.0));
        assert!(h.current("main").is_none(), "warstwa spoza filtra nie jest rysowana");
        assert_eq!(at(&mut h, 1700.0), (0.0, -90.0), "cel z innej warstwy przesuwa kafel");
        ev(&mut h, &json!({"t":"out","layer":"main"}), clk(2000.0));
        assert_eq!(at(&mut h, 2700.0), (0.0, 0.0), "zejście celu – powrót");
        // cel przeniesiony na inną warstwę spoza filtra – nadal na antenie
        ev(&mut h, &json!({"t":"take","layer":"main","scene":belka,"page":{"number":1001}}), clk(3000.0));
        ev(&mut h, &json!({"t":"relayer","layer":"main","to":"front","scene":belka,"page":{"number":1001}}), clk(4000.0));
        assert_eq!(at(&mut h, 4700.0), (0.0, -90.0));
        ev(&mut h, &json!({"t":"clear"}), clk(5000.0));
        assert!(h.foreign.is_empty());
        // odświeżenie wyjścia przy celu na antenie: kafel od razu na miejscu
        let mut h2 = Host::new(Some(vec!["prog".into()]), "program");
        ev(
            &mut h2,
            &json!({"t":"snapshot","layers":[{"layer":"main","scene":belka,"page":{"number":1001}},{"layer":"prog","scene":kafel,"page":{"number":1405}}]}),
            clk(0.0),
        );
        assert_eq!(at(&mut h2, 0.0), (0.0, -90.0));
        // kolejny snapshot bez celu – cel zszedł
        ev(&mut h2, &json!({"t":"snapshot","layers":[{"layer":"prog","scene":kafel,"page":{"number":1405}}]}), clk(1000.0));
        assert_eq!(at(&mut h2, 1700.0), (0.0, 0.0));
    }

    /// Krzywa `ease: "out"` rusza szybciej niż domyślna; powrót z czasem i krzywą reguły.
    #[test]
    fn dock_ease_and_return_use_rule() {
        let lt = json!({"name":"LT","duration":1,"stops":[0.5],"nodes":[]});
        let ws = |ease: &str| json!({"name":"WS","duration":1,"stops":[0.5],"dock":[{"target":"scene:LT","dy":-100,"dur":1.0,"ease":ease}],"nodes":[]});
        let at = |h: &mut Host, now: f64| h.current("z2_ws").map(|p| p.dock_at(now)).unwrap_or_default().1;
        let run = |ease: &str| {
            let mut h = Host::new(None, "program");
            ev(&mut h, &json!({"t":"take","layer":"z2_ws","scene":ws(ease)}), clk(0.0));
            ev(&mut h, &json!({"t":"take","layer":"main","scene":lt}), clk(1000.0));
            let early = at(&mut h, 1100.0);
            ev(&mut h, &json!({"t":"out","layer":"main"}), clk(3000.0));
            let back = at(&mut h, 3800.0);
            (early, back, at(&mut h, 4000.0))
        };
        let (std_early, std_back, std_end) = run("std");
        let (out_early, out_back, out_end) = run("out");
        assert!(out_early < -30.0 && std_early > -10.0, "out {out_early}, std {std_early}");
        // powrót trwa `dur` reguły (1 s), nie domyślne 0,6 s: po 0,8 s jeszcze w drodze
        assert!(std_back < 0.0 && out_back < 0.0, "std {std_back}, out {out_back}");
        assert_eq!((std_end, out_end), (0.0, 0.0));
    }

    /// Grafika z regułą dock wchodząca, gdy cel już jest na antenie, stoi od razu na miejscu;
    /// zejście celu przesuwa ją płynnie (jak `setDock` w `renderer.js`).
    #[test]
    fn dock_first_placement_is_instant() {
        let mut h = Host::new(None, "program");
        let lt = json!({"name":"LT","duration":1,"stops":[0.5],"nodes":[]});
        let ws = json!({"name":"WS","duration":1,"stops":[0.5],"dock":[{"target":"scene:LT","dy":-130,"dur":0.6}],"nodes":[]});
        ev(&mut h, &json!({"t":"take","layer":"main","scene":lt}), clk(0.0));
        ev(&mut h, &json!({"t":"take","layer":"z2_ws","scene":ws}), clk(1000.0));
        let at = |h: &mut Host, now: f64| h.current("z2_ws").map(|p| p.dock_at(now)).unwrap_or_default();
        assert_eq!(at(&mut h, 1000.0), (0.0, -130.0), "od razu na miejscu, bez dojazdu");
        ev(&mut h, &json!({"t":"out","layer":"main"}), clk(2000.0));
        let mid = at(&mut h, 2300.0).1;
        assert!(mid > -130.0 && mid < 0.0, "powrót płynny: {mid}");
        assert_eq!(at(&mut h, 2600.0), (0.0, 0.0));
        // nowe wyjście przy celu na antenie (snapshot) – też od razu na miejscu
        let mut h2 = Host::new(None, "program");
        ev(
            &mut h2,
            &json!({"t":"snapshot","layers":[{"layer":"main","scene":lt,"stop":0},{"layer":"z2_ws","scene":ws,"stop":0}]}),
            clk(0.0),
        );
        assert_eq!(at(&mut h2, 0.0), (0.0, -130.0));
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
        let (mut patching, mut seen) = (crate::stream::RegionCache::default(), Vec::new());
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
                let full = crate::stream::region_record(y, 0);
                assert!(cache.record(x, 0, a.last_damage, false) == full, "klatka {f}: inny rekord");
                // odbiorca łat (VizRS CG Playout) dochodzi do tego samego rekordu
                let synced = !seen.is_empty();
                crate::stream::tests::apply(&mut seen, &patching.record(x, 0, a.last_damage, synced));
                assert!(seen == full, "klatka {f}: inny obraz z łat");
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
