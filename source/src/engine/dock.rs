//! Powiązania (dock): włączanie i wyłączanie przez operatora na antenie głównej i kanałach.

use super::*;

impl Engine {
    pub fn set_dock_on(&self, ch: &str, layer: &str, off: bool) -> Result<Json, String> {
        if ch.is_empty() {
            return self.set_dock(layer, off);
        }
        if !validate::layer_name_ok(layer) || !valid_channel(ch) {
            return Err("zła nazwa warstwy".into());
        }
        {
            let mut inner = self.inner.lk();
            inner.channel_dock.retain(|(c, l)| c != ch || l != layer);
            if off {
                inner.channel_dock.push((ch.to_string(), layer.to_string()));
            }
        }
        self.broadcast(&Json::obj().with("t", "dock").with("bus", channel_bus(ch)).with("layer", layer).with("off", off));
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(Json::obj().with("ok", true).with("layer", layer).with("off", off))
    }

    /// Operator włącza/wyłącza powiązanie sceny na warstwie (nadpisanie).
    pub fn set_dock(&self, layer: &str, off: bool) -> Result<Json, String> {
        if !validate::layer_name_ok(layer) {
            return Err("zła nazwa warstwy".into());
        }
        {
            let mut inner = self.inner.lk();
            inner.dock_off.retain(|l| l != layer);
            if off {
                inner.dock_off.push(layer.to_string());
            }
        }
        self.broadcast(&Json::obj().with("t", "dock").with("layer", layer).with("off", off));
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(Json::obj().with("ok", true).with("layer", layer).with("off", off))
    }

    // ---------------- presety animacji (biblioteka użytkownika) ----------------
}
