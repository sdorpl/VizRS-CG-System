//! Edycja show: elementy (sceny, strony, playlisty, rundowny, makra), import całości, presety animacji, Word Replace.

use super::*;

impl Engine {
    pub fn show(&self) -> Json {
        self.inner.lk().show.clone()
    }

    /// Podmienia cały show (import). `rev` w pliku różny od bieżącego → konflikt, chyba że `force`.
    pub fn put_show(&self, show: Json, force: bool) -> Result<(), String> {
        validate::show(&show)?;
        let sent_rev = show.num("rev"); // przed normalizacją (ta dopisuje rev = 0)
        let mut show = normalize_show(show);
        let mut inner = self.inner.lk();
        let cur = inner.show.num("rev").unwrap_or(0.0);
        if !force {
            if let Some(r) = sent_rev {
                if r != cur {
                    return Err(format!("konflikt: show zmieniony w międzyczasie (rev {r} ≠ {cur}); użyj force=1, by nadpisać"));
                }
            }
        }
        show.set("rev", (cur + 1.0).into());
        inner.show = show;
        let show = inner.show.clone();
        drop(inner);
        self.save(&show);
        self.broadcast(&Json::obj().with("t", "show"));
        self.check_saved()
    }

    pub fn upsert(&self, kind: &str, mut item: Json) -> Result<Json, String> {
        if !KINDS.contains(&kind) || !item.is_obj() {
            return Err("zły typ obiektu".into());
        }
        match kind {
            "scenes" => validate::scene(&item)?,
            "pages" => validate::page(&item)?,
            "rundowns" => validate::rundown(&item)?,
            "macros" => validate::macro_(&item)?,
            _ => validate::playlist(&item)?,
        }
        let mut inner = self.inner.lk();
        let has_id = item.str("id").map(|s| !s.is_empty()).unwrap_or(false);
        if !has_id {
            let p = &kind[..1];
            item.set("id", self.new_id(p).into());
        }
        if kind == "pages" {
            let sid = item.str("scene").unwrap_or("").to_string();
            if !inner.show.arr("scenes").iter().any(|s| s.str("id") == Some(&sid)) {
                return Err(format!("strona wskazuje nieistniejący szablon '{sid}'"));
            }
            let id = item.str("id").unwrap_or("").to_string();
            let num = item.num("number").map(|n| n as i64);
            let taken = |n: i64| {
                inner.show.arr("pages").iter().any(|p| {
                    p.num("number").map(|x| x as i64) == Some(n) && p.str("id") != Some(&id)
                })
            };
            let num = match num {
                Some(n) if n > 0 && !taken(n) => n,
                Some(n) if n > 0 => return Err(format!("numer strony {n} jest już zajęty")),
                _ => {
                    let mut n = inner
                        .show
                        .arr("pages")
                        .iter()
                        .filter_map(|p| p.num("number"))
                        .fold(1000.0_f64, f64::max) as i64
                        + 1;
                    while taken(n) {
                        n += 1;
                    }
                    n
                }
            };
            item.set("number", (num as f64).into());
            if !item.get("values").map(|v| v.is_obj()).unwrap_or(false) {
                item.set("values", Json::obj());
            }
        }
        let id = item.str("id").unwrap_or("").to_string();
        // rewizja obiektu: klient przysyła `rev` odczytany z serwera; rozbieżność = ktoś zmienił obiekt w międzyczasie
        let existing_rev = inner.show.arr(kind).iter().find(|x| x.str("id") == Some(&id)).map(|x| x.num("rev").unwrap_or(0.0));
        if let (Some(have), Some(sent)) = (existing_rev, item.num("rev")) {
            if have != sent {
                return Err(format!("konflikt: obiekt zmieniony w międzyczasie (rev {sent} ≠ {have})"));
            }
        }
        item.set("rev", (existing_rev.unwrap_or(0.0) + 1.0).into());
        if let Some(Json::Arr(list)) = inner.show.get_mut(kind) {
            match list.iter_mut().find(|x| x.str("id") == Some(&id)) {
                Some(slot) => *slot = item.clone(),
                None => list.push(item.clone()),
            }
        }
        bump_rev(&mut inner.show);
        let show = inner.show.clone();
        drop(inner);
        self.save(&show);
        self.broadcast(&Json::obj().with("t", "show").with("kind", kind).with("id", id.as_str()));
        self.check_saved()?;
        Ok(item)
    }

    pub fn delete(&self, kind: &str, id: &str) -> Result<(), String> {
        if !KINDS.contains(&kind) {
            return Err("zły typ".into());
        }
        let mut inner = self.inner.lk();
        let mut removed_pages: Vec<String> = vec![];
        if kind == "scenes" {
            // kaskadowo usuń strony korzystające z szablonu
            if let Some(Json::Arr(pages)) = inner.show.get_mut("pages") {
                pages.retain(|p| {
                    let keep = p.str("scene") != Some(id);
                    if !keep {
                        removed_pages.push(p.str("id").unwrap_or("").to_string());
                    }
                    keep
                });
            }
        }
        if kind == "pages" {
            removed_pages.push(id.to_string());
        }
        if let Some(Json::Arr(list)) = inner.show.get_mut(kind) {
            list.retain(|x| x.str("id") != Some(id));
        }
        if !removed_pages.is_empty() {
            for list in ["playlists", "rundowns"] {
                if let Some(Json::Arr(pls)) = inner.show.get_mut(list) {
                    for pl in pls.iter_mut() {
                        if let Some(Json::Arr(items)) = pl.get_mut("items") {
                            items.retain(|it| !removed_pages.iter().any(|r| it.str("page") == Some(r)));
                        }
                    }
                }
            }
        }
        bump_rev(&mut inner.show);
        let show = inner.show.clone();
        drop(inner);
        self.save(&show);
        self.broadcast(&Json::obj().with("t", "show"));
        self.check_saved()
    }

    /// Szuka strony po numerze, id lub nazwie.
    pub(super) fn find_page(show: &Json, key: &str) -> Option<Json> {
        let key = key.trim();
        let pages = show.arr("pages");
        if let Ok(n) = key.parse::<f64>() {
            if let Some(p) = pages.iter().find(|p| p.num("number") == Some(n)) {
                return Some(p.clone());
            }
        }
        pages
            .iter()
            .find(|p| p.str("id") == Some(key))
            .or_else(|| pages.iter().find(|p| p.str("name").map(|s| s.eq_ignore_ascii_case(key)) == Some(true)))
            .cloned()
    }

    pub(super) fn find_scene(show: &Json, id: &str) -> Option<Json> {
        show.arr("scenes").iter().find(|s| s.str("id") == Some(id)).cloned()
    }

    pub fn anim_presets(&self) -> Json {
        std::fs::read_to_string(self.data_dir.join("animpresets.json")).ok().and_then(|s| Json::parse(&s).ok()).filter(|j| matches!(j, Json::Arr(_))).unwrap_or(Json::Arr(vec![]))
    }

    pub fn put_anim_presets(&self, list: Json) -> Result<Json, String> {
        validate::anim_presets(&list)?;
        let path = self.data_dir.join("animpresets.json");
        let tmp = self.data_dir.join("animpresets.json.tmp");
        std::fs::write(&tmp, list.pretty()).and_then(|_| std::fs::rename(&tmp, &path)).map_err(|e| format!("zapis na dysk nieudany: {e}"))?;
        Ok(list)
    }

    // ---------------- rundown ----------------

    pub fn word_replace(&self) -> Json {
        std::fs::read_to_string(self.data_dir.join("wordreplace.json")).ok().and_then(|s| Json::parse(&s).ok()).filter(|j| j.is_obj()).unwrap_or_else(Json::obj)
    }

    pub fn put_word_replace(&self, list: Json) -> Result<Json, String> {
        validate::word_replace(&list)?;
        let path = self.data_dir.join("wordreplace.json");
        let tmp = self.data_dir.join("wordreplace.json.tmp");
        std::fs::write(&tmp, list.pretty()).and_then(|_| std::fs::rename(&tmp, &path)).map_err(|e| format!("zapis na dysk nieudany: {e}"))?;
        self.broadcast(&Json::obj().with("t", "wordreplace"));
        Ok(list)
    }

    // ---------------- makra ----------------
}
