//! Subskrypcje zdarzeń (SSE) i snapshoty stanu anten dla wyjść i UI.

use super::*;

impl Engine {
    pub fn subscribe(&self) -> Receiver<String> {
        let (tx, rx) = sync_channel(SUB_QUEUE);
        tx.try_send(self.snapshot().dump()).ok();
        tx.try_send(self.snapshot_preview().dump()).ok();
        for snapshot in self.snapshot_channels() {
            tx.try_send(snapshot.dump()).ok();
        }
        self.subs.lk().push(tx);
        rx
    }

    pub(super) fn broadcast(&self, ev: &Json) {
        let s = ev.dump();
        self.last_event.store(now_ms(), Ordering::Relaxed);
        // klient z pełną kolejką (zawieszony) jest odłączany – zamknięcie kanału kończy jego wątek SSE
        self.subs.lk().retain(|tx| match tx.try_send(s.clone()) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => false,
            Err(TrySendError::Disconnected(_)) => false,
        });
    }

    pub fn clients(&self) -> usize {
        self.subs.lk().len()
    }

    pub fn snapshot(&self) -> Json {
        let inner = self.inner.lk();
        let layers: Vec<Json> = inner
            .layers
            .iter()
            .map(|(name, l)| {
                Json::obj()
                    .with("layer", name.as_str())
                    .with("scene", l.scene.clone())
                    .with("values", l.values.clone())
                    .with("page", l.page.clone())
                    .with("stop", l.stop)
                    .with("since", l.since as f64)
            })
            .collect();
        Json::obj().with("t", "snapshot").with("layers", Json::Arr(layers)).with("dockOff", Json::Arr(inner.dock_off.iter().map(|l| Json::from(l.as_str())).collect()))
    }

    pub fn snapshot_preview(&self) -> Json {
        let inner = self.inner.lk();
        let layers: Vec<Json> = inner
            .preview
            .iter()
            .map(|(name, l)| {
                Json::obj()
                    .with("layer", name.as_str())
                    .with("scene", l.scene.clone())
                    .with("values", l.values.clone())
                    .with("page", l.page.clone())
                    .with("stop", 0usize)
                    .with("since", l.since as f64)
            })
            .collect();
        Json::obj().with("t", "snapshot").with("bus", "preview").with("layers", Json::Arr(layers))
    }

    pub(super) fn snapshot_channels(&self) -> Vec<Json> {
        let inner = self.inner.lk();
        let layer = |(name, l): &(String, LayerState)| {
            Json::obj().with("layer", name.as_str()).with("scene", l.scene.clone()).with("values", l.values.clone()).with("page", l.page.clone()).with("stop", l.stop).with("since", l.since as f64)
        };
        inner.channels.iter().map(|(ch, layers)| Json::obj().with("t", "snapshot").with("bus", channel_bus(ch)).with("layers", Json::Arr(layers.iter().map(layer).collect())).with("dockOff", Json::Arr(inner.channel_dock.iter().filter(|(c, _)| c == ch).map(|(_, d)| Json::from(d.as_str())).collect()))).collect()
    }

    /// Stan dla interfejsu Trio (bez pełnych scen).
    pub fn state(&self) -> Json {
        self.state_on("").expect("antena główna")
    }

    /// Stan z punktu widzenia anteny `ch` ("" = główna): warstwy, karuzela i rundowny tej anteny.
    pub fn state_on(&self, ch: &str) -> Result<Json, String> {
        if !ch.is_empty() && !valid_channel(ch) {
            return Err("kanał: dozwolone litery, cyfry, . - _ (maks. 32)".into());
        }
        let inner = self.inner.lk();
        let air = if ch.is_empty() { &inner.layers } else { inner.channels.iter().find(|(c, _)| c == ch).map_or(&EMPTY_AIR, |(_, l)| l) };
        let mut layers: Vec<&(String, LayerState)> = air.iter().collect();
        layers.sort_by_key(|(n, _)| layer_z(n));
        let layers: Vec<Json> = layers
            .iter()
            .map(|(name, l)| {
                let stops = l.scene.arr("stops").len();
                let pending = Self::find_scene(&inner.show, l.scene.str("id").unwrap_or(""))
                    .map(|cur| cur.num("rev").unwrap_or(0.0) != l.scene_rev)
                    .unwrap_or(false);
                Json::obj()
                    .with("layer", name.as_str())
                    .with("page", l.page.clone())
                    .with("scene", l.scene.str("name").unwrap_or(""))
                    .with("stop", l.stop)
                    .with("stops", stops)
                    .with("since", l.since as f64)
                    .with("pending", pending)
                    .with("hasDock", !l.scene.arr("dock").is_empty())
                    .with("dockOff", if ch.is_empty() { inner.dock_off.iter().any(|d| d == name) } else { inner.channel_dock.iter().any(|(c, d)| c == ch && d == name) })
            })
            .collect();
        let rev = inner.show.num("rev").unwrap_or(0.0);
        let preview: Vec<Json> = inner
            .preview
            .iter()
            .map(|(n, l)| Json::obj().with("layer", n.as_str()).with("page", l.page.clone()).with("scene", l.scene.str("name").unwrap_or("")))
            .collect();
        let rundowns: Vec<Json> = inner
            .show
            .arr("rundowns")
            .iter()
            .map(|r| {
                let id = r.str("id").unwrap_or("");
                let run = inner.rundowns.get(&rd_key(ch, id)).cloned().unwrap_or_default();
                let items = r.arr("items");
                let cursor = Self::rd_next_item(items, &run);
                Json::obj()
                    .with("id", id)
                    .with("cursor", cursor.map(Json::from).unwrap_or(Json::Null))
                    .with("last", run.last.clone().map(Json::from).unwrap_or(Json::Null))
                    .with("done", Json::Arr(run.done.iter().map(|d| Json::from(d.as_str())).collect()))
            })
            .collect();
        let channels = Json::Arr(inner.channels.iter().map(|(c, _)| Json::from(c.as_str())).collect());
        let mut channel_names = Json::obj();
        for (c, name) in &inner.channel_labels {
            channel_names.set(c, name.as_str().into());
        }
        let carousel = match inner.carousel.get(ch) {
            Some((id, i)) => Json::obj().with("playlist", id.as_str()).with("index", *i),
            None => Json::Null,
        };
        drop(inner);
        Ok(Json::obj()
            .with("layers", Json::Arr(layers))
            .with("carousel", carousel)
            .with("channels", channels)
            .with("channelNames", channel_names)
            .with("outputs", self.clients())
            .with("rev", rev)
            .with("preview", Json::Arr(preview))
            .with("locks", self.locks_json())
            .with("rundowns", Json::Arr(rundowns))
            .with("errors", self.errors.lk().len())
            .with("saveError", self.save_error().map(Json::from).unwrap_or(Json::Null)))
    }

    // ---------------- show / CRUD ----------------
}
