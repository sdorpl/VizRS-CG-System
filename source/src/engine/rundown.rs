//! Rundowny (NEXT, kursor), makra (pokolenia, zmienne) i karuzele playlist.

use super::*;

impl Engine {
    pub(super) fn bump(gens: &Mutex<std::collections::HashMap<String, u64>>, ch: &str) -> u64 {
        let mut gens = gens.lk();
        let gen = gens.entry(ch.to_string()).or_insert(0);
        *gen += 1;
        *gen
    }

    /// Nowe pokolenie makra anteny: przerywa makro, które na niej działa.
    pub fn macro_stop(&self, ch: &str) -> u64 {
        Self::bump(&self.macro_gens, ch)
    }

    pub fn macro_gen(&self, ch: &str) -> u64 {
        self.macro_gens.lk().get(ch).copied().unwrap_or(0)
    }

    /// Id pozycji, którą weźmie NEXT: kursor, a gdy go brak – pierwsza nieodegrana.
    pub(super) fn rd_next_item(items: &[Json], run: &RdRun) -> Option<String> {
        if let Some(c) = &run.cursor {
            if items.iter().any(|i| i.str("id") == Some(c.as_str())) {
                return Some(c.clone());
            }
        }
        items
            .iter()
            .filter_map(|i| i.str("id"))
            .find(|id| !run.done.iter().any(|d| d == id) && run.last.as_deref() != Some(*id))
            .map(|s| s.to_string())
    }

    pub(super) fn rd_find(&self, rid: &str) -> Result<Json, String> {
        self.inner
            .lk()
            .show
            .arr("rundowns")
            .iter()
            .find(|r| r.str("id") == Some(rid) || r.str("name") == Some(rid))
            .cloned()
            .ok_or(format!("brak rundownu '{rid}'"))
    }

    /// Bierze pozycję `item_id` (albo następną) na antenę i przesuwa kursor.
    pub fn rundown_take(&self, rid: &str, item_id: Option<&str>) -> Result<Json, String> {
        self.rundown_take_on("", rid, item_id)
    }

    /// Rundown na antenie `ch`: każda antena ma własny kursor.
    pub fn rundown_take_on(&self, ch: &str, rid: &str, item_id: Option<&str>) -> Result<Json, String> {
        let rd = self.rd_find(rid)?;
        let rid = rd.str("id").unwrap_or("").to_string();
        let items = rd.arr("items");
        let key = rd_key(ch, &rid);
        let run = self.inner.lk().rundowns.get(&key).cloned().unwrap_or_default();
        let target = match item_id {
            Some(id) => items.iter().find(|i| i.str("id") == Some(id)).map(|_| id.to_string()).ok_or("brak takiej pozycji")?,
            None => Self::rd_next_item(items, &run).ok_or("koniec rundownu – brak kolejnych pozycji")?,
        };
        let item = items.iter().find(|i| i.str("id") == Some(target.as_str())).ok_or("brak pozycji")?;
        let page = item.str("page").unwrap_or("").to_string();
        let r = self.take_on(ch, &page, None)?;
        {
            let mut inner = self.inner.lk();
            let st = inner.rundowns.entry(key).or_default();
            if let Some(prev) = st.last.take() {
                if prev != target && !st.done.contains(&prev) {
                    st.done.push(prev);
                }
            }
            st.done.retain(|d| *d != target);
            st.last = Some(target.clone());
            let pos = items.iter().position(|i| i.str("id") == Some(target.as_str())).unwrap_or(0);
            st.cursor = items.get(pos + 1).and_then(|i| i.str("id")).map(|s| s.to_string());
        }
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(r.with("item", target.as_str()))
    }

    #[cfg(test)]
    pub fn rundown_cursor(&self, rid: &str, item_id: &str) -> Result<(), String> {
        self.rundown_cursor_on("", rid, item_id)
    }

    pub fn rundown_cursor_on(&self, ch: &str, rid: &str, item_id: &str) -> Result<(), String> {
        let rd = self.rd_find(rid)?;
        if !rd.arr("items").iter().any(|i| i.str("id") == Some(item_id)) {
            return Err("brak takiej pozycji".into());
        }
        let rid = rd.str("id").unwrap_or("").to_string();
        self.inner.lk().rundowns.entry(rd_key(ch, &rid)).or_default().cursor = Some(item_id.to_string());
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(())
    }

    pub fn rundown_reset(&self, rid: &str) -> Result<(), String> {
        self.rundown_reset_on("", rid)
    }

    pub fn rundown_reset_on(&self, ch: &str, rid: &str) -> Result<(), String> {
        let rd = self.rd_find(rid)?;
        self.inner.lk().rundowns.remove(&rd_key(ch, rd.str("id").unwrap_or("")));
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(())
    }

    // ---------------- Word Replace (lista zamian słów stanowiska) ----------------

    pub fn find_macro(&self, key: &str) -> Option<Json> {
        let k = key.trim();
        self.inner.lk().show.arr("macros").iter().find(|m| m.str("id") == Some(k) || m.str("name").map(|n| n.eq_ignore_ascii_case(k)) == Some(true)).cloned()
    }

    // ---------------- zmienne dla Companion / Stream Deck ----------------

    /// Płaski słownik `nazwa → tekst` (łatwy do odpytywania z Companion: moduł HTTP, zmienne z JSON).
    pub fn variables(&self) -> Json {
        let st = self.state();
        let mut v = Json::obj();
        let layers = st.arr("layers");
        v.set("layers_on_air", layers.len().into());
        let mut names = vec![];
        for l in layers {
            let ln = l.str("layer").unwrap_or("");
            names.push(ln.to_string());
            let p = l.get("page");
            v.set(&format!("layer_{ln}"), p.and_then(|p| p.num("number")).map(|n| format!("{}", n as i64)).unwrap_or_default().into());
            v.set(&format!("layer_{ln}_name"), p.and_then(|p| p.str("name")).unwrap_or("").into());
            v.set(&format!("layer_{ln}_stop"), format!("{}/{}", l.num("stop").unwrap_or(0.0) as i64 + 1, l.num("stops").unwrap_or(1.0).max(1.0) as i64).into());
        }
        v.set("layers", names.join(",").into());
        v.set("carousel", st.get("carousel").and_then(|c| c.str("playlist")).unwrap_or("").into());
        for r in st.arr("rundowns") {
            let rid = r.str("id").unwrap_or("");
            v.set(&format!("rundown_{rid}_next"), r.str("cursor").unwrap_or("").into());
        }
        v.set("outputs", st.num("outputs").unwrap_or(0.0).into());
        v
    }

    // ---------------- błędy scen zgłaszane przez renderer ----------------

    pub fn carousel_start(self: &Arc<Self>, playlist_id: &str) -> Result<(), String> {
        self.carousel_start_on("", playlist_id)
    }

    /// Karuzela anteny `ch`; każda antena ma własną.
    pub fn carousel_start_on(self: &Arc<Self>, ch: &str, playlist_id: &str) -> Result<(), String> {
        let pl = {
            let inner = self.inner.lk();
            inner
                .show
                .arr("playlists")
                .iter()
                .find(|p| p.str("id") == Some(playlist_id) || p.str("name") == Some(playlist_id))
                .cloned()
                .ok_or(format!("brak playlisty '{playlist_id}'"))?
        };
        if pl.arr("items").is_empty() {
            return Err("playlista jest pusta".into());
        }
        let gen = Self::bump(&self.carousel_gens, ch);
        let me = Arc::clone(self);
        let pid = pl.str("id").unwrap_or("").to_string();
        let ch = ch.to_string();
        std::thread::spawn(move || me.carousel_loop(ch, gen, pid));
        Ok(())
    }

    pub fn carousel_stop(&self, take_out: bool) {
        self.carousel_stop_on("", take_out)
    }

    pub fn carousel_stop_on(&self, ch: &str, take_out: bool) {
        Self::bump(&self.carousel_gens, ch);
        let mut inner = self.inner.lk();
        let was = inner.carousel.remove(ch);
        drop(inner);
        if take_out && was.is_some() {
            // zdejmij warstwy używane przez playlistę
            let pl = self.show().arr("playlists").iter().find(|p| p.str("id") == was.as_ref().map(|w| w.0.as_str())).cloned();
            if let Some(pl) = pl {
                let mut layers: Vec<String> = pl.arr("items").iter().filter_map(|it| self.page_layer(it.str("page")?)).collect();
                layers.dedup();
                for l in layers {
                    self.out_on(ch, &l).ok();
                }
            }
        }
        self.broadcast(&Json::obj().with("t", "state"));
    }

    pub(super) fn carousel_loop(self: Arc<Self>, ch: String, gen: u64, playlist_id: String) {
        let alive = |e: &Engine| e.carousel_gens.lk().get(&ch).copied() == Some(gen);
        let mut idx = 0usize;
        let mut prev_page: Option<String> = None;
        loop {
            if !alive(&self) {
                return;
            }
            let pl = self.show().arr("playlists").iter().find(|p| p.str("id") == Some(&playlist_id)).cloned();
            let Some(pl) = pl else { break };
            let items = pl.arr("items").to_vec();
            if items.is_empty() {
                break;
            }
            if idx >= items.len() {
                if pl.get("loop") == Some(&Json::Bool(false)) {
                    break;
                }
                idx = 0;
            }
            let it = &items[idx];
            let page = it.str("page").unwrap_or("").to_string();
            let dur = it.num("duration").unwrap_or(5.0).max(0.5);
            {
                let mut inner = self.inner.lk();
                inner.carousel.insert(ch.clone(), (playlist_id.clone(), idx));
            }
            let layer = self.page_layer(&page);
            // warstwa poprzedniej strony liczona teraz: operator mógł ją zmienić w trakcie karuzeli
            if let (Some(p), Some(l)) = (prev_page.as_deref().and_then(|p| self.page_layer(p)), &layer) {
                if p != *l {
                    self.out_on(&ch, &p).ok();
                }
            }
            if self.take_on(&ch, &page, None).is_err() {
                idx += 1;
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            self.broadcast(&Json::obj().with("t", "state"));
            prev_page = Some(page);
            let end = now_ms() + (dur * 1000.0) as u64;
            while now_ms() < end {
                if !alive(&self) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            idx += 1;
        }
        if alive(&self) {
            if let Some(l) = prev_page.and_then(|p| self.page_layer(&p)) {
                self.out_on(&ch, &l).ok();
            }
            self.inner.lk().carousel.remove(&ch);
            self.broadcast(&Json::obj().with("t", "state"));
        }
    }
}

/// Klucz stanu rundownu: każda antena ma własny kursor.
pub(super) fn rd_key(ch: &str, rid: &str) -> String {
    if ch.is_empty() { rid.to_string() } else { format!("{ch}\0{rid}") }
}
