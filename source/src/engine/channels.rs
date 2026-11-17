//! Anteny kanałów playoutu (`@N`, wyjście `?bus=ch-N`): lista, nazwy, TAKE/CONTINUE/OUT/CLEAR na kanale.

use super::*;

impl Engine {
    /// Antena kanału; nową zakłada tylko `create` (limit [`MAX_CHANNELS`]).
    pub(super) fn channel_mut<'a>(inner: &'a mut Inner, ch: &str, create: bool, data_dir: &Path) -> Result<Option<&'a mut Vec<(String, LayerState)>>, String> {
        if !valid_channel(ch) {
            return Err("kanał: dozwolone litery, cyfry, . - _ (maks. 32)".into());
        }
        if let Some(i) = inner.channels.iter().position(|(c, _)| c == ch) {
            return Ok(Some(&mut inner.channels[i].1));
        }
        if !create {
            return Ok(None);
        }
        if inner.channels.len() >= MAX_CHANNELS {
            return Err(format!("za dużo kanałów (maks. {MAX_CHANNELS})"));
        }
        inner.channels.push((ch.to_string(), vec![]));
        save_channels(data_dir, &inner.channels);
        Ok(inner.channels.last_mut().map(|c| &mut c.1))
    }

    pub fn channel_names(&self) -> Vec<String> {
        self.inner.lk().channels.iter().map(|(c, _)| c.clone()).collect()
    }

    /// Zakłada antenę kanału (zostaje na liście po restarcie).
    pub fn channel_add(&self, ch: &str, name: Option<&str>) -> Result<Json, String> {
        let mut inner = self.inner.lk();
        Self::channel_mut(&mut inner, ch, true, &self.data_dir)?;
        if let Some(name) = name.map(|n| n.chars().filter(|c| !c.is_control()).take(100).collect::<String>()) {
            inner.channel_labels.retain(|(c, _)| c != ch);
            if !name.trim().is_empty() {
                inner.channel_labels.push((ch.to_string(), name.trim().to_string()));
            }
            save_labels(&self.data_dir, &inner.channel_labels);
        }
        drop(inner);
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(Json::obj().with("ok", true).with("bus", channel_bus(ch)))
    }

    /// Zdejmuje grafikę kanału i usuwa jego antenę.
    pub fn channel_delete(&self, ch: &str) -> Result<Json, String> {
        self.carousel_stop_on(ch, false);
        self.macro_stop(ch);
        self.channel_clear(ch)?;
        let mut inner = self.inner.lk();
        inner.channels.retain(|(c, _)| c != ch);
        inner.channel_dock.retain(|(c, _)| c != ch);
        inner.channel_labels.retain(|(c, _)| c != ch);
        save_labels(&self.data_dir, &inner.channel_labels);
        let prefix = format!("{ch}\0");
        inner.rundowns.retain(|k, _| !k.starts_with(&prefix));
        save_channels(&self.data_dir, &inner.channels);
        drop(inner);
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(Json::obj().with("ok", true))
    }

    pub fn channel_take(&self, ch: &str, key: &str, override_values: Option<&Json>) -> Result<Json, String> {
        let mut inner = self.inner.lk();
        let (layer, st, _) = Self::make_layer(&inner.show, key, override_values)?;
        let bus = channel_bus(ch);
        let ev = Self::layer_event("take", &layer, &st, Some(&bus));
        let list = Self::channel_mut(&mut inner, ch, true, &self.data_dir)?.ok_or("brak kanału")?;
        match list.iter_mut().find(|(n, _)| *n == layer) {
            Some(slot) => slot.1 = st,
            None => list.push((layer.clone(), st)),
        }
        drop(inner);
        self.broadcast(&ev);
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(Json::obj().with("ok", true).with("layer", layer).with("bus", bus))
    }

    /// Warstwa kanału z argumentu (nazwa warstwy albo strona).
    pub(super) fn channel_layer(&self, ch: &str, key: &str) -> Result<String, String> {
        let k = key.trim();
        let mut inner = self.inner.lk();
        let held = Self::channel_mut(&mut inner, ch, false, &self.data_dir)?.is_some_and(|list| list.iter().any(|(n, _)| n == k));
        drop(inner);
        Ok(if held { k.to_string() } else { self.page_layer(k).unwrap_or_else(|| k.to_string()) })
    }

    pub fn channel_cont(&self, ch: &str, key: &str) -> Result<Json, String> {
        let layer = self.channel_layer(ch, key)?;
        let mut inner = self.inner.lk();
        let list = Self::channel_mut(&mut inner, ch, false, &self.data_dir)?.ok_or(format!("warstwa '{layer}' jest pusta"))?;
        let (_, l) = list.iter_mut().find(|(n, _)| *n == layer).ok_or(format!("warstwa '{layer}' jest pusta"))?;
        if l.stop + 1 < l.scene.arr("stops").len() {
            l.stop += 1;
        }
        let stop = l.stop;
        drop(inner);
        self.broadcast(&Json::obj().with("t", "continue").with("bus", channel_bus(ch)).with("layer", layer.as_str()).with("stop", stop));
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(Json::obj().with("ok", true).with("layer", layer).with("stop", stop))
    }

    pub fn channel_out(&self, ch: &str, key: &str) -> Result<Json, String> {
        let layer = self.channel_layer(ch, key)?;
        let mut inner = self.inner.lk();
        let changed = Self::channel_mut(&mut inner, ch, false, &self.data_dir)?.is_some_and(|list| {
            let before = list.len();
            list.retain(|(n, _)| *n != layer);
            before != list.len()
        });
        drop(inner);
        if changed {
            self.broadcast(&Json::obj().with("t", "out").with("bus", channel_bus(ch)).with("layer", layer.as_str()));
            self.broadcast(&Json::obj().with("t", "state"));
        }
        Ok(Json::obj().with("ok", true).with("layer", layer))
    }

    pub fn channel_clear(&self, ch: &str) -> Result<Json, String> {
        let mut inner = self.inner.lk();
        if let Some(list) = Self::channel_mut(&mut inner, ch, false, &self.data_dir)? {
            list.clear();
        }
        drop(inner);
        self.broadcast(&Json::obj().with("t", "clear").with("bus", channel_bus(ch)));
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(Json::obj().with("ok", true))
    }

    /// Stan anteny kanału, w układzie [`Engine::state`].
    pub fn channel_state(&self, ch: &str) -> Result<Json, String> {
        if !valid_channel(ch) {
            return Err("kanał: dozwolone litery, cyfry, . - _ (maks. 32)".into());
        }
        Ok(self.state_on(ch)?.with("bus", channel_bus(ch)))
    }
}

/// Najwięcej anten kanałów: nazwy przychodzą z sieci.
pub(super) const MAX_CHANNELS: usize = 64;

pub(super) static EMPTY_AIR: Vec<(String, LayerState)> = Vec::new();

/// Lista kanałów przeżywa restart; ich grafika, jak na antenie głównej, nie.
pub(super) fn load_channels(data_dir: &Path) -> Vec<(String, Vec<(String, LayerState)>)> {
    let text = std::fs::read_to_string(data_dir.join("channels.json")).unwrap_or_default();
    let list = Json::parse(&text).unwrap_or(Json::Arr(vec![]));
    let Json::Arr(items) = list else { return vec![] };
    items.iter().filter_map(|c| if let Json::Str(s) = c { Some(s.clone()) } else { None }).filter(|c| valid_channel(c)).take(MAX_CHANNELS).map(|c| (c, vec![])).collect()
}

pub(super) fn save_channels(data_dir: &Path, channels: &[(String, Vec<(String, LayerState)>)]) {
    let list = Json::Arr(channels.iter().map(|(c, _)| Json::from(c.as_str())).collect());
    if let Err(e) = std::fs::write(data_dir.join("channels.json"), list.dump()) {
        eprintln!("[vizrs] zapis listy kanałów: {e}");
    }
}

pub(super) fn load_labels(data_dir: &Path) -> Vec<(String, String)> {
    let text = std::fs::read_to_string(data_dir.join("channel-names.json")).unwrap_or_default();
    match Json::parse(&text) {
        Ok(Json::Obj(items)) => items.into_iter().filter_map(|(c, n)| if let (true, Json::Str(n)) = (valid_channel(&c), n) { Some((c, n.chars().take(100).collect())) } else { None }).take(MAX_CHANNELS).collect(),
        _ => vec![],
    }
}

pub(super) fn save_labels(data_dir: &Path, labels: &[(String, String)]) {
    let mut list = Json::obj();
    for (c, name) in labels {
        list.set(c, name.as_str().into());
    }
    if let Err(e) = std::fs::write(data_dir.join("channel-names.json"), list.dump()) {
        eprintln!("[vizrs] zapis nazw kanałów: {e}");
    }
}

pub(super) fn valid_channel(ch: &str) -> bool {
    !ch.is_empty() && ch.len() <= 32 && ch.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// Szyna zdarzeń i wyjścia kanału (`/output?bus=ch-N`).
pub(super) fn channel_bus(ch: &str) -> String {
    format!("ch-{ch}")
}
