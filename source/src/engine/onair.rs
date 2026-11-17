//! Antena główna i podgląd: TAKE/ARM/CONTINUE/OUT/CLEAR, UPDATE, ticker, deploy, warstwy.

use super::*;

impl Engine {
    pub fn page_layer(&self, key: &str) -> Option<String> {
        let inner = self.inner.lk();
        let p = Self::find_page(&inner.show, key)?;
        let s = Self::find_scene(&inner.show, p.str("scene")?)?;
        Some(s.str("layer").unwrap_or("main").to_string())
    }

    /// Zamienia argument (nazwa warstwy albo numer strony) na nazwę warstwy.
    pub(super) fn resolve_layer(&self, key: &str) -> String {
        let k = key.trim();
        {
            let inner = self.inner.lk();
            if inner.layers.iter().any(|(n, _)| n == k) {
                return k.to_string();
            }
        }
        self.page_layer(k).unwrap_or_else(|| k.to_string())
    }

    // ---------------- playout ----------------

    /// Buduje stan warstwy dla strony (wspólne dla TAKE i ARM).
    pub(super) fn make_layer(show: &Json, key: &str, override_values: Option<&Json>) -> Result<(String, LayerState, Json), String> {
        let page = Self::find_page(show, key).ok_or(format!("brak strony '{key}'"))?;
        let scene = Self::find_scene(show, page.str("scene").unwrap_or("")).ok_or("strona wskazuje nieistniejący szablon")?;
        let layer = scene.str("layer").unwrap_or("main").to_string();
        let mut values = page.get("values").cloned().unwrap_or(Json::obj());
        if let Some(ov) = override_values {
            validate::values(ov)?;
            merge(&mut values, ov);
        }
        let pinfo = Json::obj()
            .with("id", page.str("id").unwrap_or(""))
            .with("number", page.num("number").unwrap_or(0.0))
            .with("name", page.str("name").unwrap_or(""));
        let st = LayerState { page: pinfo.clone(), scene: scene.clone(), values, stop: 0, since: now_ms(), scene_rev: scene.num("rev").unwrap_or(0.0) };
        Ok((layer, st, pinfo))
    }

    pub(super) fn layer_event(t: &str, layer: &str, st: &LayerState, bus: Option<&str>) -> Json {
        let mut ev = Json::obj()
            .with("t", t)
            .with("layer", layer)
            .with("scene", st.scene.clone())
            .with("values", st.values.clone())
            .with("since", st.since as f64)
            .with("page", st.page.clone());
        if let Some(b) = bus {
            ev.set("bus", b.into());
        }
        ev
    }

    pub fn take(&self, key: &str, override_values: Option<&Json>) -> Result<Json, String> {
        let mut inner = self.inner.lk();
        let (layer, st, _) = Self::make_layer(&inner.show, key, override_values)?;
        let ev = Self::layer_event("take", &layer, &st, None);
        match inner.layers.iter_mut().find(|(n, _)| *n == layer) {
            Some(slot) => slot.1 = st,
            None => inner.layers.push((layer.clone(), st)),
        }
        drop(inner);
        self.broadcast(&ev);
        Ok(Json::obj().with("ok", true).with("layer", layer))
    }

    // ---------------- anteny kanałów ----------------

    pub fn take_on(&self, ch: &str, key: &str, ov: Option<&Json>) -> Result<Json, String> {
        if ch.is_empty() { self.take(key, ov) } else { self.channel_take(ch, key, ov) }
    }

    pub fn out_on(&self, ch: &str, key: &str) -> Result<Json, String> {
        if ch.is_empty() { self.out(key) } else { self.channel_out(ch, key) }
    }

    pub fn deploy_on(&self, ch: &str, key: &str) -> Result<Json, String> {
        if ch.is_empty() {
            return self.deploy(key);
        }
        let layer = self.channel_layer(ch, key)?;
        let mut inner = self.inner.lk();
        let show = inner.show.clone();
        let list = Self::channel_mut(&mut inner, ch, false, &self.data_dir)?.ok_or(format!("warstwa '{layer}' jest pusta"))?;
        let (_, l) = list.iter_mut().find(|(n, _)| *n == layer).ok_or(format!("warstwa '{layer}' jest pusta"))?;
        let scene = Self::find_scene(&show, l.scene.str("id").unwrap_or("")).ok_or("szablon został usunięty")?;
        l.scene = scene.clone();
        l.scene_rev = scene.num("rev").unwrap_or(0.0);
        l.stop = l.stop.min(scene.arr("stops").len().saturating_sub(1));
        let stop = l.stop;
        drop(inner);
        self.broadcast(&Json::obj().with("t", "rescene").with("bus", channel_bus(ch)).with("layer", layer.as_str()).with("scene", scene).with("stop", stop));
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(Json::obj().with("ok", true).with("layer", layer))
    }

    /// Uzbraja stronę na busie Preview (`output?bus=preview`) – bez wejścia na antenę.
    pub fn arm(&self, key: &str, override_values: Option<&Json>) -> Result<Json, String> {
        let mut inner = self.inner.lk();
        let (layer, st, _) = Self::make_layer(&inner.show, key, override_values)?;
        let ev = Self::layer_event("take", &layer, &st, Some("preview"));
        match inner.preview.iter_mut().find(|(n, _)| *n == layer) {
            Some(slot) => slot.1 = st,
            None => inner.preview.push((layer.clone(), st)),
        }
        drop(inner);
        self.broadcast(&ev);
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(Json::obj().with("ok", true).with("layer", layer).with("bus", "preview"))
    }

    pub fn preview_clear(&self) {
        self.inner.lk().preview.clear();
        self.broadcast(&Json::obj().with("t", "clear").with("bus", "preview"));
        self.broadcast(&Json::obj().with("t", "state"));
    }

    pub fn cont(&self, key: &str) -> Result<Json, String> {
        let layer = self.resolve_layer(key);
        let mut inner = self.inner.lk();
        let (_, l) = inner.layers.iter_mut().find(|(n, _)| *n == layer).ok_or(format!("warstwa '{layer}' jest pusta"))?;
        let stops = l.scene.arr("stops").len();
        if l.stop + 1 < stops {
            l.stop += 1;
        }
        let stop = l.stop;
        drop(inner);
        self.broadcast(&Json::obj().with("t", "continue").with("layer", layer.as_str()).with("stop", stop));
        Ok(Json::obj().with("ok", true).with("layer", layer).with("stop", stop))
    }

    pub fn out(&self, key: &str) -> Result<Json, String> {
        let layer = self.resolve_layer(key);
        let mut inner = self.inner.lk();
        let before = inner.layers.len();
        inner.layers.retain(|(n, _)| *n != layer);
        let changed = before != inner.layers.len();
        drop(inner);
        if changed {
            self.broadcast(&Json::obj().with("t", "out").with("layer", layer.as_str()));
        }
        Ok(Json::obj().with("ok", true).with("layer", layer))
    }

    pub fn clear(&self) {
        self.inner.lk().layers.clear();
        self.broadcast(&Json::obj().with("t", "clear"));
    }

    /// Aktualizuje pola strony (opcjonalnie zapisuje) i – jeśli strona jest
    /// on-air – podmienia dane na żywo w wyjściu (bez ponownej animacji).
    pub fn update(&self, key: &str, values: &Json, persist: bool) -> Result<Json, String> {
        validate::values(values)?;
        let mut inner = self.inner.lk();
        let page = Self::find_page(&inner.show, key).ok_or(format!("brak strony '{key}'"))?;
        let pid = page.str("id").unwrap_or("").to_string();
        if persist {
            if let Some(Json::Arr(pages)) = inner.show.get_mut("pages") {
                if let Some(p) = pages.iter_mut().find(|p| p.str("id") == Some(&pid)) {
                    let mut v = p.get("values").cloned().unwrap_or(Json::obj());
                    merge(&mut v, values);
                    p.set("values", v);
                    let r = p.num("rev").unwrap_or(0.0);
                    p.set("rev", (r + 1.0).into());
                }
            }
            bump_rev(&mut inner.show);
            let show = inner.show.clone();
            self.save(&show);
        }
        let mut events = vec![];
        for (name, l) in inner.layers.iter_mut() {
            if l.page.str("id") == Some(&pid) {
                merge(&mut l.values, values);
                events.push(Json::obj().with("t", "update").with("layer", name.as_str()).with("values", l.values.clone()));
            }
        }
        for (ch, list) in inner.channels.iter_mut() {
            for (name, l) in list.iter_mut().filter(|(_, l)| l.page.str("id") == Some(&pid)) {
                merge(&mut l.values, values);
                events.push(Json::obj().with("t", "update").with("bus", channel_bus(ch)).with("layer", name.as_str()).with("values", l.values.clone()));
            }
        }
        drop(inner);
        let live = !events.is_empty();
        for e in &events {
            self.broadcast(e);
        }
        if persist {
            self.broadcast(&Json::obj().with("t", "show").with("kind", "pages").with("id", pid.as_str()));
        }
        Ok(Json::obj().with("ok", true).with("onAir", live))
    }

    /// Wiadomości tickera strony (komenda `TICKER`, `/api/ticker`). `key` = strona albo `strona:pole`.
    /// Zmiana zapisuje stronę i – gdy jest na antenie – podmienia dane na żywo (UPDATE: przewijanie nie startuje od nowa).
    pub fn ticker(&self, key: &str, op: crate::ticker::Op) -> Result<Json, String> {
        let (page_key, fid, text) = {
            let inner = self.inner.lk();
            let split = key.rsplit_once(':').filter(|(p, f)| !f.is_empty() && Self::find_page(&inner.show, p).is_some());
            let (page_key, want) = match split {
                Some((p, f)) => (p, Some(f)),
                None => (key, None),
            };
            let page = Self::find_page(&inner.show, page_key).ok_or(format!("brak strony '{page_key}'"))?;
            let scene = Self::find_scene(&inner.show, page.str("scene").unwrap_or("")).ok_or("strona wskazuje nieistniejący szablon")?;
            let (fid, def) = crate::ticker::find_field(&scene, want).ok_or("ta strona nie ma pola tickera")?;
            let text = page.get("values").and_then(|v| v.str(&fid)).map(str::to_string).unwrap_or(def);
            (page_key.to_string(), fid, text)
        };
        let list = |t: &str| Json::Arr(crate::ticker::parse_items(t).iter().map(|x| x.to_json()).collect());
        if op == crate::ticker::Op::List {
            return Ok(Json::obj().with("field", fid.as_str()).with("items", list(&text)));
        }
        let new = crate::ticker::apply(&text, &op)?;
        let r = self.update(&page_key, &Json::obj().with(&fid, new.as_str()), true)?;
        Ok(r.with("field", fid.as_str()).with("items", list(&new)))
    }

    /// Wdraża aktualną wersję szablonu do warstwy na antenie bez ponownej animacji (F2.4).
    pub fn deploy(&self, key: &str) -> Result<Json, String> {
        let layer = self.resolve_layer(key);
        let mut inner = self.inner.lk();
        let sid = inner
            .layers
            .iter()
            .find(|(n, _)| *n == layer)
            .map(|(_, l)| l.scene.str("id").unwrap_or("").to_string())
            .ok_or(format!("warstwa '{layer}' jest pusta"))?;
        let scene = Self::find_scene(&inner.show, &sid).ok_or("szablon został usunięty")?;
        let (_, l) = inner.layers.iter_mut().find(|(n, _)| *n == layer).ok_or("warstwa zniknęła")?;
        l.scene = scene.clone();
        l.scene_rev = scene.num("rev").unwrap_or(0.0);
        let stops = scene.arr("stops").len();
        if l.stop + 1 > stops && stops > 0 {
            l.stop = stops - 1;
        }
        let stop = l.stop;
        drop(inner);
        self.broadcast(&Json::obj().with("t", "rescene").with("layer", layer.as_str()).with("scene", scene).with("stop", stop));
        Ok(Json::obj().with("ok", true).with("layer", layer))
    }

    /// Zmienia warstwę szablonu strony. Warstwa należy do sceny, więc zmiana obejmuje wszystkie jej strony;
    /// grafika na antenie i na Preview przechodzi na nową warstwę bez ponownej animacji. Zajętej warstwy
    /// anteny nie zastępuje bez `force`.
    pub fn set_layer(&self, key: &str, layer: &str, force: bool) -> Result<Json, String> {
        if !validate::layer_name_ok(layer) {
            return Err("warstwa: dozwolone litery, cyfry, - i _ (maks. 50)".into());
        }
        let mut inner = self.inner.lk();
        let page = Self::find_page(&inner.show, key).ok_or(format!("brak strony '{key}'"))?;
        let sid = page.str("scene").unwrap_or("").to_string();
        let scene = Self::find_scene(&inner.show, &sid).ok_or("strona wskazuje nieistniejący szablon")?;
        let from = scene.str("layer").unwrap_or("main").to_string();
        let old_rev = scene.num("rev").unwrap_or(0.0);
        let holds = |list: &[(String, LayerState)]| list.iter().find(|(n, l)| l.scene.str("id") == Some(&sid) && n != layer).map(|(n, _)| n.clone());
        let air = holds(&inner.layers);
        let prv = holds(&inner.preview);
        if air.is_some() && !force {
            if let Some((_, l)) = inner.layers.iter().find(|(n, _)| n == layer) {
                return Err(format!("konflikt: na warstwie {layer} jest na antenie strona {} {}", l.page.num("number").unwrap_or(0.0), l.page.str("name").unwrap_or("")));
            }
        }
        for (ch, list) in &inner.channels {
            if holds(list).is_some() && !force {
                if let Some((_, l)) = list.iter().find(|(n, _)| n == layer) {
                    return Err(format!("konflikt: na warstwie {layer} kanału {ch} jest strona {} {}", l.page.num("number").unwrap_or(0.0), l.page.str("name").unwrap_or("")));
                }
            }
        }
        let new_rev = if from == layer { old_rev } else { old_rev + 1.0 };
        if from != layer {
            if let Some(Json::Arr(list)) = inner.show.get_mut("scenes") {
                if let Some(s) = list.iter_mut().find(|s| s.str("id") == Some(&sid)) {
                    s.set("layer", layer.into());
                    s.set("rev", new_rev.into());
                }
            }
            bump_rev(&mut inner.show);
        }
        let mut events = vec![];
        for (old, bus) in [(air.as_deref(), None), (prv.as_deref(), Some("preview"))] {
            let Some(old) = old else { continue };
            let list = if bus.is_some() { &mut inner.preview } else { &mut inner.layers };
            list.retain(|(n, _)| n != layer);
            if let Some((n, l)) = list.iter_mut().find(|(n, _)| n == old) {
                *n = layer.to_string();
                events.push(Self::layer_event("relayer", old, l, bus).with("to", layer).with("stop", l.stop));
            }
        }
        for (ch, list) in inner.channels.iter_mut() {
            let Some(old) = list.iter().find(|(n, l)| l.scene.str("id") == Some(&sid) && n != layer).map(|(n, _)| n.clone()) else { continue };
            list.retain(|(n, _)| n != layer);
            if let Some((n, l)) = list.iter_mut().find(|(n, _)| *n == old) {
                *n = layer.to_string();
                events.push(Self::layer_event("relayer", &old, l, Some(&channel_bus(ch))).with("to", layer).with("stop", l.stop));
            }
        }
        if let Some(old) = &air {
            inner.dock_off.retain(|d| d != layer);
            for d in inner.dock_off.iter_mut().filter(|d| *d == old) {
                *d = layer.to_string();
            }
        }
        // sama zmiana warstwy to nie nowa wersja szablonu: bez znacznika ⏳, chyba że już czekała inna zmiana
        let st = &mut *inner;
        for (_, l) in st.layers.iter_mut().chain(st.preview.iter_mut()).chain(st.channels.iter_mut().flat_map(|(_, list)| list.iter_mut())).filter(|(_, l)| l.scene.str("id") == Some(&sid) && l.scene_rev == old_rev) {
            l.scene.set("layer", layer.into());
            l.scene_rev = new_rev;
        }
        let show = (from != layer).then(|| inner.show.clone());
        let pages = inner.show.arr("pages").iter().filter(|p| p.str("scene") == Some(&sid)).count();
        drop(inner);
        if let Some(show) = &show {
            self.save(show);
            self.broadcast(&Json::obj().with("t", "show").with("kind", "scenes").with("id", sid.as_str()).with("layer", layer).with("rev", new_rev));
        }
        let moved = !events.is_empty();
        for e in &events {
            self.broadcast(e);
        }
        if moved {
            self.broadcast(&Json::obj().with("t", "state"));
        }
        self.check_saved()?;
        Ok(Json::obj().with("ok", true).with("layer", layer).with("from", from.as_str()).with("pages", pages).with("onAir", air.is_some()))
    }

    // ---------------- blokady edycji (wielu operatorów) ----------------
}
