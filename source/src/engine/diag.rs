//! Diagnostyka: błędy scen zgłaszane przez renderery i `/health`.

use super::*;

impl Engine {
    pub fn report_error(&self, scene: &str, layer: &str, msg: &str) {
        let cut = |s: &str, n: usize| s.chars().take(n).collect::<String>();
        let (scene, layer, msg) = (cut(scene, 100), cut(layer, 50), cut(msg, 300));
        let (scene_for_ev, msg_for_ev) = (scene.clone(), msg.clone());
        {
            let mut errs = self.errors.lk();
            match errs.iter_mut().find(|e| e.scene == scene && e.msg == msg) {
                Some(e) => {
                    e.count += 1;
                    e.last = now_ms();
                }
                None => {
                    if errs.len() >= MAX_ERRS {
                        errs.remove(0);
                    }
                    errs.push(SceneErr { scene, layer, msg, count: 1, last: now_ms() });
                }
            }
        }
        self.broadcast(&Json::obj().with("t", "error").with("scene", scene_for_ev.as_str()).with("message", msg_for_ev.as_str()));
        self.broadcast(&Json::obj().with("t", "state"));
    }

    pub fn errors(&self) -> Json {
        Json::Arr(
            self.errors
                .lk()
                .iter()
                .map(|e| {
                    Json::obj()
                        .with("scene", e.scene.as_str())
                        .with("layer", e.layer.as_str())
                        .with("message", e.msg.as_str())
                        .with("count", e.count as f64)
                        .with("last", e.last as f64)
                })
                .collect(),
        )
    }

    pub fn clear_errors(&self) {
        self.errors.lk().clear();
        self.broadcast(&Json::obj().with("t", "state"));
    }

    /// Stan zdrowia systemu dla monitoringu (`/health`).
    pub fn health(&self) -> Json {
        let inner = self.inner.lk();
        let on_air = inner.layers.len();
        let rev = inner.show.num("rev").unwrap_or(0.0);
        drop(inner);
        let last = self.last_event.load(Ordering::Relaxed);
        let save_err = self.save_error();
        let errs = self.errors.lk().len();
        let status = if save_err.is_some() { "degraded" } else { "ok" };
        Json::obj()
            .with("status", status)
            .with("version", env!("CARGO_PKG_VERSION"))
            .with("build", env!("VIZRS_GIT"))
            .with("uptimeS", self.started.elapsed().as_secs() as f64)
            .with("rev", rev)
            .with("outputs", self.clients())
            .with("layersOnAir", on_air)
            .with("lastEventAgeS", if last == 0 { Json::Null } else { Json::Num(now_ms().saturating_sub(last) as f64 / 1000.0) })
            .with("sceneErrors", errs)
            .with("saveError", save_err.map(Json::from).unwrap_or(Json::Null))
            .with("backups", self.backups().len())
    }

    // ---------------- karuzela ----------------
}
