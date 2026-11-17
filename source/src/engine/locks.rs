//! Miękkie blokady edycji obiektów przez operatorów.

use super::*;

impl Engine {
    pub(super) fn live_locks(&self) -> Vec<(String, Lock)> {
        let now = now_ms();
        let mut m = self.locks.lk();
        m.retain(|_, l| now.saturating_sub(l.at) < LOCK_TTL_MS);
        let mut v: Vec<(String, Lock)> = m.iter().map(|(k, l)| (k.clone(), l.clone())).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }

    /// Zakłada lub odświeża blokadę. Gdy trzyma ją ktoś inny → `Err("konflikt: …")` z jego nazwą.
    pub fn lock(&self, kind: &str, id: &str, who: &str) -> Result<Json, String> {
        if !KINDS.contains(&kind) || id.is_empty() || id.len() > 100 {
            return Err("zły obiekt blokady".into());
        }
        let who: String = who.trim().chars().take(40).collect();
        if who.is_empty() {
            return Err("podaj nazwę operatora (who)".into());
        }
        let key = format!("{kind}:{id}");
        let now = now_ms();
        let changed;
        {
            let mut m = self.locks.lk();
            m.retain(|_, l| now.saturating_sub(l.at) < LOCK_TTL_MS);
            if let Some(l) = m.get(&key) {
                if l.who != who {
                    return Err(format!("konflikt: edytuje {} (blokada wygasa po {} s bez aktywności)", l.who, LOCK_TTL_MS / 1000));
                }
            }
            changed = m.get(&key).map(|l| l.who != who).unwrap_or(true);
            m.insert(key, Lock { who: who.clone(), at: now });
        }
        if changed {
            self.broadcast(&Json::obj().with("t", "state"));
        }
        Ok(Json::obj().with("ok", true).with("who", who).with("ttlS", (LOCK_TTL_MS / 1000) as f64))
    }

    /// Zwalnia blokadę (tylko właściciel; `force` zwalnia cudzą – „przejęcie”).
    pub fn unlock(&self, kind: &str, id: &str, who: &str, force: bool) -> Json {
        let key = format!("{kind}:{id}");
        let removed = {
            let mut m = self.locks.lk();
            match m.get(&key) {
                Some(l) if force || l.who == who.trim() => m.remove(&key).is_some(),
                _ => false,
            }
        };
        if removed {
            self.broadcast(&Json::obj().with("t", "state"));
        }
        Json::obj().with("ok", true).with("released", removed)
    }

    pub fn locks_json(&self) -> Json {
        let now = now_ms();
        Json::Arr(
            self.live_locks()
                .into_iter()
                .map(|(k, l)| {
                    let (kind, id) = k.split_once(':').unwrap_or(("", ""));
                    Json::obj().with("kind", kind).with("id", id).with("who", l.who.as_str()).with("ageS", now.saturating_sub(l.at) as f64 / 1000.0)
                })
                .collect(),
        )
    }

    // ---------------- powiązania (dock) ----------------
}
