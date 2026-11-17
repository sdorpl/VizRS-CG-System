//! Zapis show (atomowy, z kopią zapasową), biblioteka show (`shows/`) i przywracanie kopii.

use super::*;

impl Engine {
    pub(super) fn new_id(&self, prefix: &str) -> String {
        let n = self.id_ctr.fetch_add(1, Ordering::Relaxed);
        format!("{}{:x}{:x}", prefix, now_ms() % 0xffff_ffff, n)
    }

    /// Zapis show: plik tymczasowy + fsync + rename, potem kopie rotacyjne.
    /// Błąd nie ginie po cichu – jest zapamiętany (`save_error`) i pokazany w `state()`.
    pub(super) fn save(&self, show: &Json) {
        self.save_with(show, false);
    }

    pub(super) fn save_with(&self, show: &Json, force_recent: bool) {
        use std::io::Write;
        let path = self.data_dir.join("show.json");
        let tmp = self.data_dir.join("show.json.tmp");
        let text = show.pretty();
        let r = std::fs::File::create(&tmp)
            .and_then(|mut f| f.write_all(text.as_bytes()).and_then(|_| f.sync_all()))
            .and_then(|_| std::fs::rename(&tmp, &path))
            .map_err(|e| e.to_string())
            .and_then(|_| {
                let now = now_ms();
                let last = self.last_recent.load(Ordering::Relaxed);
                let recent = force_recent || now.saturating_sub(last) >= crate::backup::RECENT_EVERY_MS;
                if recent {
                    self.last_recent.store(now, Ordering::Relaxed);
                }
                crate::backup::rotate(&self.data_dir.join("backups"), &text, now, recent)
            })
            .and_then(|_| {
                // kopia w bibliotece show (lista „Show / paczka” w Trio)
                let id = self.active.lk().clone();
                if id.is_empty() {
                    return Ok(());
                }
                write_atomic(&self.data_dir.join("shows"), &format!("{id}.json"), &text)
            });
        let prev = std::mem::replace(&mut *self.save_err.lk(), r.err());
        if prev != *self.save_err.lk() {
            self.broadcast(&Json::obj().with("t", "state"));
        }
    }

    pub fn save_error(&self) -> Option<String> {
        self.save_err.lk().clone()
    }

    pub(super) fn check_saved(&self) -> Result<(), String> {
        match self.save_error() {
            Some(e) => Err(format!("zapis na dysk nieudany: {e}")),
            None => Ok(()),
        }
    }

    pub fn backups(&self) -> Vec<String> {
        crate::backup::list(&self.data_dir.join("backups"))
    }

    /// Przywraca show z kopii (bieżący stan trafia najpierw do kopii).
    pub fn restore_backup(&self, name: &str) -> Result<(), String> {
        let text = crate::backup::read(&self.data_dir.join("backups"), name)?;
        let show = Json::parse(&text).map_err(|e| format!("kopia uszkodzona: {e}"))?;
        let cur = self.show();
        self.save_with(&cur, true); // zachowaj bieżący stan jako najnowszą kopię (do cofnięcia przywrócenia)
        self.put_show(show, true)
    }

    // ---------------- biblioteka show (osobna paczka grafiki na audycję) ----------------

    pub(super) fn set_active(&self, id: &str) {
        *self.active.lk() = id.to_string();
        write_atomic(&self.data_dir.join("shows"), ".active", id).ok();
    }

    pub fn active_show(&self) -> String {
        self.active.lk().clone()
    }

    /// Wolny identyfikator pliku dla nazwy (`Fakty Wieczór` → `fakty-wieczor`, `-2`, `-3`…).
    pub(super) fn unique_show_id(&self, name: &str) -> String {
        let base = show_slug(name);
        let dir = self.data_dir.join("shows");
        (1..).map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") }).find(|id| !dir.join(format!("{id}.json")).exists()).unwrap_or(base)
    }

    /// Lista show w bibliotece: `{active, shows:[{id,name,scenes,pages}]}`.
    pub fn shows(&self) -> Json {
        let dir = self.data_dir.join("shows");
        let mut list: Vec<(String, Json)> = std::fs::read_dir(&dir)
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| {
                        let f = e.file_name().to_string_lossy().to_string();
                        let id = f.strip_suffix(".json").filter(|id| valid_show_id(id))?.to_string();
                        let j = std::fs::read_to_string(e.path()).ok().and_then(|t| Json::parse(&t).ok())?;
                        let name = j.str("name").unwrap_or(&id).to_string();
                        let item = Json::obj().with("id", id.as_str()).with("name", name.as_str()).with("scenes", j.arr("scenes").len() as f64).with("pages", j.arr("pages").len() as f64);
                        Some((name.to_lowercase(), item))
                    })
                    .collect()
            })
            .unwrap_or_default();
        list.sort_by(|a, b| a.0.cmp(&b.0));
        Json::obj().with("active", self.active_show().as_str()).with("shows", Json::Arr(list.into_iter().map(|x| x.1).collect()))
    }

    /// Przełącza aktywny show na inny z biblioteki. Zdejmuje wszystko z anteny i podglądu
    /// (sceny starego show nie istnieją w nowym); bieżący show jest już zapisany w bibliotece.
    pub fn switch_show(&self, id: &str) -> Result<(), String> {
        if !valid_show_id(id) {
            return Err("niedozwolona nazwa show".into());
        }
        if id == self.active_show() {
            return Ok(());
        }
        let text = std::fs::read_to_string(self.data_dir.join("shows").join(format!("{id}.json"))).map_err(|_| format!("brak show „{id}”"))?;
        let show = Json::parse(&text).map_err(|e| format!("show „{id}” uszkodzony: {e}"))?;
        validate::show(&show)?;
        for ch in self.channel_names().into_iter().chain([String::new()]) {
            self.carousel_stop_on(&ch, false);
            self.macro_stop(&ch);
        }
        {
            let mut inner = self.inner.lk();
            inner.preview.clear();
            inner.rundowns.clear();
        }
        for ch in self.inner.lk().channels.iter().map(|(c, _)| c.clone()).collect::<Vec<_>>() {
            self.channel_clear(&ch).ok();
        }
        self.clear();
        self.set_active(id);
        self.put_show(show, true)?;
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(())
    }

    /// Nowy show w bibliotece: kopia bieżącego (`copy`) albo pusty; od razu staje się aktywny.
    pub fn new_show(&self, name: &str, copy: bool) -> Result<Json, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 120 {
            return Err("podaj nazwę show (do 120 znaków)".into());
        }
        let mut show = if copy { self.show() } else { Json::obj().with("scenes", Json::Arr(vec![])).with("pages", Json::Arr(vec![])) };
        show.set("name", name.into());
        show.set("rev", 0.0.into());
        validate::show(&show)?;
        let id = self.unique_show_id(name);
        write_atomic(&self.data_dir.join("shows"), &format!("{id}.json"), &normalize_show(show).pretty()).map_err(|e| format!("zapis na dysk nieudany: {e}"))?;
        self.switch_show(&id)?;
        Ok(Json::obj().with("id", id.as_str()))
    }

    /// Usuwa show z biblioteki (nie aktywny – ten jest na antenie / w edycji).
    pub fn delete_show(&self, id: &str) -> Result<(), String> {
        if !valid_show_id(id) {
            return Err("niedozwolona nazwa show".into());
        }
        if id == self.active_show() {
            return Err("nie można usunąć aktywnego show – najpierw przełącz na inny".into());
        }
        std::fs::remove_file(self.data_dir.join("shows").join(format!("{id}.json"))).map_err(|_| format!("brak show „{id}”"))
    }

    // ---------------- zdarzenia ----------------
}

pub(super) fn bump_rev(show: &mut Json) {
    let r = show.num("rev").unwrap_or(0.0);
    show.set("rev", (r + 1.0).into());
}

/// Identyfikator show w bibliotece = nazwa pliku bez `.json` (ochrona przed ścieżkami).
pub fn valid_show_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 && !id.starts_with('-') && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Nazwa show → identyfikator pliku (polskie litery bez ogonków, reszta → `-`).
pub fn show_slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        let c = match c {
            'ą' => 'a',
            'ć' => 'c',
            'ę' => 'e',
            'ł' => 'l',
            'ń' => 'n',
            'ó' => 'o',
            'ś' => 's',
            'ź' | 'ż' => 'z',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c != '-' || !(out.is_empty() || out.ends_with('-')) {
            out.push(c);
        }
        if out.len() >= 60 {
            break;
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "show".into() } else { out }
}

pub(super) fn write_atomic(dir: &std::path::Path, file: &str, text: &str) -> Result<(), String> {
    use std::io::Write;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("{file}.tmp"));
    std::fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(text.as_bytes()).and_then(|_| f.sync_all()))
        .and_then(|_| std::fs::rename(&tmp, dir.join(file)))
        .map_err(|e| e.to_string())
}

pub(super) fn normalize_show(mut show: Json) -> Json {
    if show.num("rev").is_none() {
        show.set("rev", 0.0.into());
    }
    if show.str("name").is_none() {
        show.set("name", "Show".into());
    }
    for k in KINDS {
        if !matches!(show.get(k), Some(Json::Arr(_))) {
            show.set(k, Json::Arr(vec![]));
        }
    }
    show
}
