//! Silnik playout: show (sceny / strony / playlisty), warstwy on-air,
//! logika TAKE / CONTINUE / OUT, karuzela playlist i rozgłaszanie zdarzeń
//! do wyjść (OBS) przez Server-Sent Events.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::audit::Audit;
use crate::json::Json;
use crate::validate;

mod persist;
mod sse;
mod show;
mod onair;
mod dock;
mod channels;
mod rundown;
mod locks;
mod diag;
#[cfg(test)]
mod tests;

use self::channels::*;
use self::persist::*;
use self::rundown::*;

pub const KINDS: [&str; 5] = ["scenes", "pages", "playlists", "rundowns", "macros"];

/// Pojemność kolejki zdarzeń jednego klienta SSE; wolny klient jest odłączany.
const SUB_QUEUE: usize = 256;

/// Blokada odporna na zatrucie (panika w innym wątku nie wywraca serwera).
pub trait LockExt<T> {
    fn lk(&self) -> MutexGuard<'_, T>;
}
impl<T> LockExt<T> for Mutex<T> {
    fn lk(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Clone)]
struct LayerState {
    page: Json,
    scene: Json,
    values: Json,
    stop: usize,
    since: u64,
    /// `rev` sceny w chwili TAKE – do wykrycia „czekającej” aktualizacji szablonu.
    scene_rev: f64,
}

/// Blokada edycji obiektu przez operatora (miękka – ostrzega, nie blokuje na stałe).
#[derive(Clone)]
struct Lock {
    who: String,
    at: u64,
}

/// Czas życia blokady bez odświeżenia (klient odświeża co ~15 s).
pub const LOCK_TTL_MS: u64 = 45_000;

/// Błąd sceny zgłoszony przez renderer (wyjście lub podgląd).
#[derive(Clone)]
struct SceneErr {
    scene: String,
    layer: String,
    msg: String,
    count: u64,
    last: u64,
}

const MAX_ERRS: usize = 50;

/// Stan wykonania rundownu (nie jest zapisywany w show – żyje do restartu).
#[derive(Default, Clone)]
struct RdRun {
    /// Id pozycji, którą weźmie następne NEXT (None = pierwsza nieodegrana).
    cursor: Option<String>,
    /// Ostatnio wzięta pozycja (po następnym NEXT zostanie „odegrana”).
    last: Option<String>,
    done: Vec<String>,
}

struct Inner {
    show: Json,
    layers: Vec<(String, LayerState)>,
    /// Bus Preview (uzbrojone strony; osobne wyjście `?bus=preview`).
    preview: Vec<(String, LayerState)>,
    /// Anteny kanałów playoutu (`@N` w TCP, wyjście `?bus=ch-N`); niezależne od głównej.
    channels: Vec<(String, Vec<(String, LayerState)>)>,
    /// Warstwy z wyłączonym przez operatora powiązaniem (dock) – nadpisanie jak w Viz.
    dock_off: Vec<String>,
    rundowns: std::collections::HashMap<String, RdRun>,
    /// Karuzele anten, klucz: kanał ("" = główna); (playlist id, indeks elementu).
    carousel: std::collections::HashMap<String, (String, usize)>,
    /// Nazwy kanałów do pokazania operatorowi: (kanał, nazwa).
    channel_labels: Vec<(String, String)>,
    /// Wyłączone powiązania (dock) kanałów: (kanał, warstwa).
    channel_dock: Vec<(String, String)>,
}

pub struct Engine {
    inner: Mutex<Inner>,
    subs: Mutex<Vec<SyncSender<String>>>,
    pub data_dir: PathBuf,
    save_err: Mutex<Option<String>>,
    last_recent: AtomicU64,
    last_event: AtomicU64,
    started: Instant,
    errors: Mutex<Vec<SceneErr>>,
    locks: Mutex<std::collections::HashMap<String, Lock>>,
    pub audit: Audit,
    /// Pokolenia karuzel i makr anten: nowe przerywa poprzednie tej samej anteny.
    carousel_gens: Mutex<std::collections::HashMap<String, u64>>,
    macro_gens: Mutex<std::collections::HashMap<String, u64>>,
    id_ctr: AtomicU64,
    /// Identyfikator aktywnego show w bibliotece `shows/` (plik `shows/<id>.json`).
    active: Mutex<String>,
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn merge(dst: &mut Json, src: &Json) {
    if !dst.is_obj() {
        *dst = Json::obj();
    }
    if let Json::Obj(items) = src {
        for (k, v) in items {
            dst.set(k, v.clone());
        }
    }
}

fn layer_z(name: &str) -> i32 {
    match name {
        "back" => 10,
        "main" => 20,
        "front" => 30,
        _ => 25,
    }
}

impl Engine {
    pub fn new(data_dir: PathBuf, demo: &str) -> Arc<Engine> {
        std::fs::create_dir_all(data_dir.join("assets")).ok();
        let path = data_dir.join("show.json");
        let show = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| match Json::parse(&s) {
                Ok(j) => Some(j),
                Err(e) => {
                    eprintln!("[vizrs] show.json uszkodzony ({e}) – wczytuję demo, stary plik -> show.json.bak");
                    std::fs::copy(&path, data_dir.join("show.json.bak")).ok();
                    None
                }
            })
            .unwrap_or_else(|| Json::parse(demo).expect("demo_show.json"));
        let e = Arc::new(Engine {
            inner: Mutex::new(Inner { show: normalize_show(show), layers: vec![], preview: vec![], channels: load_channels(&data_dir), dock_off: vec![], rundowns: Default::default(), carousel: Default::default(), channel_labels: load_labels(&data_dir), channel_dock: vec![] }),
            subs: Mutex::new(vec![]),
            save_err: Mutex::new(None),
            last_recent: AtomicU64::new(0),
            last_event: AtomicU64::new(0),
            started: Instant::now(),
            errors: Mutex::new(vec![]),
            locks: Mutex::new(Default::default()),
            audit: Audit::new(&data_dir),
            data_dir,
            carousel_gens: Mutex::new(Default::default()),
            macro_gens: Mutex::new(Default::default()),
            id_ctr: AtomicU64::new(1),
            active: Mutex::new(String::new()),
        });
        let show = e.inner.lk().show.clone();
        let active = std::fs::read_to_string(e.data_dir.join("shows").join(".active"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| valid_show_id(s))
            .unwrap_or_else(|| e.unique_show_id(show.str("name").unwrap_or("show")));
        e.set_active(&active);
        e.save(&show);
        e
    }
}
