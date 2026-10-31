//! Zasoby renderera: fonty i obrazy. Pobierane w tle (klatka nigdy nie czeka na sieć – jak `font-display: block`
//! tekst bez swojego fontu nie jest rysowany, obraz pojawia się po wczytaniu). Każda nowa rzecz zwiększa `epoch`,
//! więc teksty mierzą się od nowa (jak `fontEpoch` w `renderer.js`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use tiny_skia::Pixmap;

use crate::font::{self, FontDb};

/// Pobranie pliku: ścieżka serwera (`/assets/x.png`, `/fonts/x.woff2`) → bajty.
pub type Fetch = dyn Fn(&str) -> Result<Vec<u8>, String> + Send + Sync;

#[derive(Clone)]
enum Img {
    Loading,
    Ready(Arc<Pixmap>),
    /// Nieudane wczytanie: następna próba nie wcześniej niż …
    Failed(Instant),
}

enum Job {
    Image(String),
    AssetFont(String),
}

pub struct Resources {
    pub fonts: RwLock<FontDb>,
    images: Mutex<HashMap<String, Img>>,
    asset_fonts: Mutex<HashMap<String, Option<Instant>>>,
    epoch: AtomicU64,
    fetch: Option<Arc<Fetch>>,
    jobs: Mutex<Option<Sender<Job>>>,
    /// Wczytywanie od razu w wątku rysowania (polecenie `frame`, testy).
    blocking: bool,
}

const RETRY: Duration = Duration::from_secs(10);

impl Resources {
    /// Zasoby bez sieci (fonty dodaje wywołujący, obrazy z `fetch`, o ile podano).
    pub fn new(fonts: FontDb, fetch: Option<Arc<Fetch>>, blocking: bool) -> Arc<Resources> {
        let res = Arc::new(Resources {
            fonts: RwLock::new(fonts),
            images: Mutex::new(HashMap::new()),
            asset_fonts: Mutex::new(HashMap::new()),
            epoch: AtomicU64::new(0),
            fetch,
            jobs: Mutex::new(None),
            blocking,
        });
        if res.fetch.is_some() && !blocking {
            let (tx, rx) = mpsc::channel::<Job>();
            *res.jobs.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
            let weak = Arc::downgrade(&res);
            let _ = std::thread::Builder::new().name("assets".into()).spawn(move || {
                for job in rx {
                    let Some(res) = weak.upgrade() else { break };
                    res.run(job);
                }
            });
        }
        res
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Relaxed)
    }

    fn bump(&self) {
        self.epoch.fetch_add(1, Ordering::Relaxed);
    }

    fn run(&self, job: Job) {
        let Some(fetch) = &self.fetch else { return };
        match job {
            Job::Image(src) => {
                let state = fetch(&asset_path(&src)).and_then(|b| crate::image::decode(&b));
                let mut images = self.images.lock().unwrap_or_else(|e| e.into_inner());
                match state {
                    Ok(p) => {
                        images.insert(src, Img::Ready(Arc::new(p)));
                    }
                    Err(e) => {
                        eprintln!("vizrs-render: obraz {src}: {e}");
                        images.insert(src, Img::Failed(Instant::now()));
                    }
                }
                drop(images);
                self.bump();
            }
            Job::AssetFont(name) => {
                let r = fetch(&asset_path(&name)).and_then(|b| self.fonts.write().unwrap_or_else(|e| e.into_inner()).add_asset(&name, &b));
                let mut fonts = self.asset_fonts.lock().unwrap_or_else(|e| e.into_inner());
                match r {
                    Ok(()) => {
                        fonts.insert(name, None);
                    }
                    Err(e) => {
                        eprintln!("vizrs-render: font {name}: {e}");
                        fonts.insert(name, Some(Instant::now()));
                    }
                }
                drop(fonts);
                self.bump();
            }
        }
    }

    fn submit(&self, job: Job) {
        if self.blocking {
            self.run(job);
        } else if let Some(tx) = self.jobs.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let _ = tx.send(job);
        }
    }

    /// Obraz z assets (adres `http…`/`data:` – nieobsługiwany); `None`, dopóki się nie wczyta.
    pub fn image(&self, src: &str) -> Option<Arc<Pixmap>> {
        let mut images = self.images.lock().unwrap_or_else(|e| e.into_inner());
        match images.get(src).cloned() {
            Some(Img::Ready(p)) => return Some(p),
            Some(Img::Loading) => return None,
            Some(Img::Failed(at)) if at.elapsed() < RETRY => return None,
            _ => {}
        }
        self.fetch.as_ref()?;
        images.insert(src.to_string(), Img::Loading);
        drop(images);
        self.submit(Job::Image(src.to_string()));
        if self.blocking {
            if let Some(Img::Ready(p)) = self.images.lock().unwrap_or_else(|e| e.into_inner()).get(src) {
                return Some(Arc::clone(p));
            }
        }
        None
    }

    /// Font węzła z assets (`*.ttf` …) – zamawiany przy pierwszym użyciu. Czy jest gotowy.
    pub fn asset_font(&self, name: &str) -> bool {
        if !font::is_font_file(name) {
            return true;
        }
        let mut fonts = self.asset_fonts.lock().unwrap_or_else(|e| e.into_inner());
        match fonts.get(name) {
            Some(None) => return true,
            Some(Some(at)) if at.elapsed() < RETRY => return false,
            _ => {}
        }
        if self.fetch.is_none() {
            return self.fonts.read().unwrap_or_else(|e| e.into_inner()).has_family(name);
        }
        fonts.insert(name.to_string(), Some(Instant::now() + Duration::from_secs(3600)));
        drop(fonts);
        self.submit(Job::AssetFont(name.to_string()));
        self.blocking && self.fonts.read().unwrap_or_else(|e| e.into_inner()).has_family(name)
    }
}

/// `/assets/<nazwa>` z kodowaniem jak `encodeURIComponent`.
pub fn asset_path(name: &str) -> String {
    format!("/assets/{}", encode_component(name))
}

pub fn encode_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_paths_are_encoded() {
        assert_eq!(asset_path("logo ż.png"), "/assets/logo%20%C5%BC.png");
    }

    #[test]
    fn missing_fetch_gives_nothing() {
        let r = Resources::new(FontDb::new(), None, true);
        assert!(r.image("a.png").is_none());
        assert!(!r.asset_font("x.ttf"));
        assert!(r.asset_font("Inter"));
    }
}
