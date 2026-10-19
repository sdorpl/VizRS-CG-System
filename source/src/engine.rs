//! Silnik playout: show (sceny / strony / playlisty), warstwy on-air,
//! logika TAKE / CONTINUE / OUT, karuzela playlist i rozgłaszanie zdarzeń
//! do wyjść (OBS) przez Server-Sent Events.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::audit::Audit;
use crate::json::Json;
use crate::validate;

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
    /// Warstwy z wyłączonym przez operatora powiązaniem (dock) – nadpisanie jak w Viz.
    dock_off: Vec<String>,
    rundowns: std::collections::HashMap<String, RdRun>,
    carousel: Option<(String, usize)>, // (playlist id, indeks elementu)
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
    carousel_gen: AtomicU64,
    pub macro_gen: AtomicU64,
    id_ctr: AtomicU64,
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
            inner: Mutex::new(Inner { show: normalize_show(show), layers: vec![], preview: vec![], dock_off: vec![], rundowns: Default::default(), carousel: None }),
            subs: Mutex::new(vec![]),
            save_err: Mutex::new(None),
            last_recent: AtomicU64::new(0),
            last_event: AtomicU64::new(0),
            started: Instant::now(),
            errors: Mutex::new(vec![]),
            locks: Mutex::new(Default::default()),
            audit: Audit::new(&data_dir),
            data_dir,
            carousel_gen: AtomicU64::new(0),
            macro_gen: AtomicU64::new(0),
            id_ctr: AtomicU64::new(1),
        });
        let show = e.inner.lk().show.clone();
        e.save(&show);
        e
    }

    fn new_id(&self, prefix: &str) -> String {
        let n = self.id_ctr.fetch_add(1, Ordering::Relaxed);
        format!("{}{:x}{:x}", prefix, now_ms() % 0xffff_ffff, n)
    }

    /// Zapis show: plik tymczasowy + fsync + rename, potem kopie rotacyjne.
    /// Błąd nie ginie po cichu – jest zapamiętany (`save_error`) i pokazany w `state()`.
    fn save(&self, show: &Json) {
        self.save_with(show, false);
    }

    fn save_with(&self, show: &Json, force_recent: bool) {
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
            });
        let prev = std::mem::replace(&mut *self.save_err.lk(), r.err());
        if prev != *self.save_err.lk() {
            self.broadcast(&Json::obj().with("t", "state"));
        }
    }

    pub fn save_error(&self) -> Option<String> {
        self.save_err.lk().clone()
    }

    fn check_saved(&self) -> Result<(), String> {
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

    // ---------------- zdarzenia ----------------

    pub fn subscribe(&self) -> Receiver<String> {
        let (tx, rx) = sync_channel(SUB_QUEUE);
        tx.try_send(self.snapshot().dump()).ok();
        tx.try_send(self.snapshot_preview().dump()).ok();
        self.subs.lk().push(tx);
        rx
    }

    fn broadcast(&self, ev: &Json) {
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

    /// Stan dla interfejsu Trio (bez pełnych scen).
    pub fn state(&self) -> Json {
        let inner = self.inner.lk();
        let mut layers: Vec<&(String, LayerState)> = inner.layers.iter().collect();
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
                    .with("dockOff", inner.dock_off.iter().any(|d| d == name))
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
                let run = inner.rundowns.get(id).cloned().unwrap_or_default();
                let items = r.arr("items");
                let cursor = Self::rd_next_item(items, &run);
                Json::obj()
                    .with("id", id)
                    .with("cursor", cursor.map(Json::from).unwrap_or(Json::Null))
                    .with("last", run.last.clone().map(Json::from).unwrap_or(Json::Null))
                    .with("done", Json::Arr(run.done.iter().map(|d| Json::from(d.as_str())).collect()))
            })
            .collect();
        let carousel = match &inner.carousel {
            Some((id, i)) => Json::obj().with("playlist", id.as_str()).with("index", *i),
            None => Json::Null,
        };
        drop(inner);
        Json::obj()
            .with("layers", Json::Arr(layers))
            .with("carousel", carousel)
            .with("outputs", self.clients())
            .with("rev", rev)
            .with("preview", Json::Arr(preview))
            .with("locks", self.locks_json())
            .with("rundowns", Json::Arr(rundowns))
            .with("errors", self.errors.lk().len())
            .with("saveError", self.save_error().map(Json::from).unwrap_or(Json::Null))
    }

    // ---------------- show / CRUD ----------------

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
    fn find_page(show: &Json, key: &str) -> Option<Json> {
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

    fn find_scene(show: &Json, id: &str) -> Option<Json> {
        show.arr("scenes").iter().find(|s| s.str("id") == Some(id)).cloned()
    }

    pub fn page_layer(&self, key: &str) -> Option<String> {
        let inner = self.inner.lk();
        let p = Self::find_page(&inner.show, key)?;
        let s = Self::find_scene(&inner.show, p.str("scene")?)?;
        Some(s.str("layer").unwrap_or("main").to_string())
    }

    /// Zamienia argument (nazwa warstwy albo numer strony) na nazwę warstwy.
    fn resolve_layer(&self, key: &str) -> String {
        let k = key.trim();
        {
            let inner = self.inner.lk();
            if inner.layers.iter().any(|(n, _)| n == k) {
                return k.to_string();
            }
        }
        self.page_layer(k).unwrap_or_else(|| k.to_string())
    }

    // ---------------- playout ----------------

    /// Buduje stan warstwy dla strony (wspólne dla TAKE i ARM).
    fn make_layer(show: &Json, key: &str, override_values: Option<&Json>) -> Result<(String, LayerState, Json), String> {
        let page = Self::find_page(show, key).ok_or(format!("brak strony '{key}'"))?;
        let scene = Self::find_scene(show, page.str("scene").unwrap_or("")).ok_or("strona wskazuje nieistniejący szablon")?;
        let layer = scene.str("layer").unwrap_or("main").to_string();
        let mut values = page.get("values").cloned().unwrap_or(Json::obj());
        if let Some(ov) = override_values {
            validate::values(ov)?;
            merge(&mut values, ov);
        }
        let pinfo = Json::obj()
            .with("id", page.str("id").unwrap_or(""))
            .with("number", page.num("number").unwrap_or(0.0))
            .with("name", page.str("name").unwrap_or(""));
        let st = LayerState { page: pinfo.clone(), scene: scene.clone(), values, stop: 0, since: now_ms(), scene_rev: scene.num("rev").unwrap_or(0.0) };
        Ok((layer, st, pinfo))
    }

    fn layer_event(t: &str, layer: &str, st: &LayerState, bus: Option<&str>) -> Json {
        let mut ev = Json::obj()
            .with("t", t)
            .with("layer", layer)
            .with("scene", st.scene.clone())
            .with("values", st.values.clone())
            .with("since", st.since as f64)
            .with("page", st.page.clone());
        if let Some(b) = bus {
            ev.set("bus", b.into());
        }
        ev
    }

    pub fn take(&self, key: &str, override_values: Option<&Json>) -> Result<Json, String> {
        let mut inner = self.inner.lk();
        let (layer, st, _) = Self::make_layer(&inner.show, key, override_values)?;
        let ev = Self::layer_event("take", &layer, &st, None);
        match inner.layers.iter_mut().find(|(n, _)| *n == layer) {
            Some(slot) => slot.1 = st,
            None => inner.layers.push((layer.clone(), st)),
        }
        drop(inner);
        self.broadcast(&ev);
        Ok(Json::obj().with("ok", true).with("layer", layer))
    }

    /// Uzbraja stronę na busie Preview (`output?bus=preview`) – bez wejścia na antenę.
    pub fn arm(&self, key: &str, override_values: Option<&Json>) -> Result<Json, String> {
        let mut inner = self.inner.lk();
        let (layer, st, _) = Self::make_layer(&inner.show, key, override_values)?;
        let ev = Self::layer_event("take", &layer, &st, Some("preview"));
        match inner.preview.iter_mut().find(|(n, _)| *n == layer) {
            Some(slot) => slot.1 = st,
            None => inner.preview.push((layer.clone(), st)),
        }
        drop(inner);
        self.broadcast(&ev);
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(Json::obj().with("ok", true).with("layer", layer).with("bus", "preview"))
    }

    pub fn preview_clear(&self) {
        self.inner.lk().preview.clear();
        self.broadcast(&Json::obj().with("t", "clear").with("bus", "preview"));
        self.broadcast(&Json::obj().with("t", "state"));
    }

    pub fn cont(&self, key: &str) -> Result<Json, String> {
        let layer = self.resolve_layer(key);
        let mut inner = self.inner.lk();
        let (_, l) = inner.layers.iter_mut().find(|(n, _)| *n == layer).ok_or(format!("warstwa '{layer}' jest pusta"))?;
        let stops = l.scene.arr("stops").len();
        if l.stop + 1 < stops {
            l.stop += 1;
        }
        let stop = l.stop;
        drop(inner);
        self.broadcast(&Json::obj().with("t", "continue").with("layer", layer.as_str()).with("stop", stop));
        Ok(Json::obj().with("ok", true).with("layer", layer).with("stop", stop))
    }

    pub fn out(&self, key: &str) -> Result<Json, String> {
        let layer = self.resolve_layer(key);
        let mut inner = self.inner.lk();
        let before = inner.layers.len();
        inner.layers.retain(|(n, _)| *n != layer);
        let changed = before != inner.layers.len();
        drop(inner);
        if changed {
            self.broadcast(&Json::obj().with("t", "out").with("layer", layer.as_str()));
        }
        Ok(Json::obj().with("ok", true).with("layer", layer))
    }

    pub fn clear(&self) {
        self.inner.lk().layers.clear();
        self.broadcast(&Json::obj().with("t", "clear"));
    }

    /// Aktualizuje pola strony (opcjonalnie zapisuje) i – jeśli strona jest
    /// on-air – podmienia dane na żywo w wyjściu (bez ponownej animacji).
    pub fn update(&self, key: &str, values: &Json, persist: bool) -> Result<Json, String> {
        validate::values(values)?;
        let mut inner = self.inner.lk();
        let page = Self::find_page(&inner.show, key).ok_or(format!("brak strony '{key}'"))?;
        let pid = page.str("id").unwrap_or("").to_string();
        if persist {
            if let Some(Json::Arr(pages)) = inner.show.get_mut("pages") {
                if let Some(p) = pages.iter_mut().find(|p| p.str("id") == Some(&pid)) {
                    let mut v = p.get("values").cloned().unwrap_or(Json::obj());
                    merge(&mut v, values);
                    p.set("values", v);
                    let r = p.num("rev").unwrap_or(0.0);
                    p.set("rev", (r + 1.0).into());
                }
            }
            bump_rev(&mut inner.show);
            let show = inner.show.clone();
            self.save(&show);
        }
        let mut events = vec![];
        for (name, l) in inner.layers.iter_mut() {
            if l.page.str("id") == Some(&pid) {
                merge(&mut l.values, values);
                events.push(Json::obj().with("t", "update").with("layer", name.as_str()).with("values", l.values.clone()));
            }
        }
        drop(inner);
        let live = !events.is_empty();
        for e in &events {
            self.broadcast(e);
        }
        if persist {
            self.broadcast(&Json::obj().with("t", "show").with("kind", "pages").with("id", pid.as_str()));
        }
        Ok(Json::obj().with("ok", true).with("onAir", live))
    }

    /// Wiadomości tickera strony (komenda `TICKER`, `/api/ticker`). `key` = strona albo `strona:pole`.
    /// Zmiana zapisuje stronę i – gdy jest na antenie – podmienia dane na żywo (UPDATE: przewijanie nie startuje od nowa).
    pub fn ticker(&self, key: &str, op: crate::ticker::Op) -> Result<Json, String> {
        let (page_key, fid, text) = {
            let inner = self.inner.lk();
            let split = key.rsplit_once(':').filter(|(p, f)| !f.is_empty() && Self::find_page(&inner.show, p).is_some());
            let (page_key, want) = match split {
                Some((p, f)) => (p, Some(f)),
                None => (key, None),
            };
            let page = Self::find_page(&inner.show, page_key).ok_or(format!("brak strony '{page_key}'"))?;
            let scene = Self::find_scene(&inner.show, page.str("scene").unwrap_or("")).ok_or("strona wskazuje nieistniejący szablon")?;
            let (fid, def) = crate::ticker::find_field(&scene, want).ok_or("ta strona nie ma pola tickera")?;
            let text = page.get("values").and_then(|v| v.str(&fid)).map(str::to_string).unwrap_or(def);
            (page_key.to_string(), fid, text)
        };
        let list = |t: &str| Json::Arr(crate::ticker::parse_items(t).iter().map(|x| x.to_json()).collect());
        if op == crate::ticker::Op::List {
            return Ok(Json::obj().with("field", fid.as_str()).with("items", list(&text)));
        }
        let new = crate::ticker::apply(&text, &op)?;
        let r = self.update(&page_key, &Json::obj().with(&fid, new.as_str()), true)?;
        Ok(r.with("field", fid.as_str()).with("items", list(&new)))
    }

    /// Wdraża aktualną wersję szablonu do warstwy na antenie bez ponownej animacji (F2.4).
    pub fn deploy(&self, key: &str) -> Result<Json, String> {
        let layer = self.resolve_layer(key);
        let mut inner = self.inner.lk();
        let sid = inner
            .layers
            .iter()
            .find(|(n, _)| *n == layer)
            .map(|(_, l)| l.scene.str("id").unwrap_or("").to_string())
            .ok_or(format!("warstwa '{layer}' jest pusta"))?;
        let scene = Self::find_scene(&inner.show, &sid).ok_or("szablon został usunięty")?;
        let (_, l) = inner.layers.iter_mut().find(|(n, _)| *n == layer).ok_or("warstwa zniknęła")?;
        l.scene = scene.clone();
        l.scene_rev = scene.num("rev").unwrap_or(0.0);
        let stops = scene.arr("stops").len();
        if l.stop + 1 > stops && stops > 0 {
            l.stop = stops - 1;
        }
        let stop = l.stop;
        drop(inner);
        self.broadcast(&Json::obj().with("t", "rescene").with("layer", layer.as_str()).with("scene", scene).with("stop", stop));
        Ok(Json::obj().with("ok", true).with("layer", layer))
    }

    // ---------------- blokady edycji (wielu operatorów) ----------------

    fn live_locks(&self) -> Vec<(String, Lock)> {
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

    /// Id pozycji, którą weźmie NEXT: kursor, a gdy go brak – pierwsza nieodegrana.
    fn rd_next_item(items: &[Json], run: &RdRun) -> Option<String> {
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

    fn rd_find(&self, rid: &str) -> Result<Json, String> {
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
        let rd = self.rd_find(rid)?;
        let rid = rd.str("id").unwrap_or("").to_string();
        let items = rd.arr("items");
        let run = self.inner.lk().rundowns.get(&rid).cloned().unwrap_or_default();
        let target = match item_id {
            Some(id) => items.iter().find(|i| i.str("id") == Some(id)).map(|_| id.to_string()).ok_or("brak takiej pozycji")?,
            None => Self::rd_next_item(items, &run).ok_or("koniec rundownu – brak kolejnych pozycji")?,
        };
        let item = items.iter().find(|i| i.str("id") == Some(target.as_str())).ok_or("brak pozycji")?;
        let page = item.str("page").unwrap_or("").to_string();
        let r = self.take(&page, None)?;
        {
            let mut inner = self.inner.lk();
            let st = inner.rundowns.entry(rid.clone()).or_default();
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

    pub fn rundown_cursor(&self, rid: &str, item_id: &str) -> Result<(), String> {
        let rd = self.rd_find(rid)?;
        if !rd.arr("items").iter().any(|i| i.str("id") == Some(item_id)) {
            return Err("brak takiej pozycji".into());
        }
        let rid = rd.str("id").unwrap_or("").to_string();
        self.inner.lk().rundowns.entry(rid).or_default().cursor = Some(item_id.to_string());
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(())
    }

    pub fn rundown_reset(&self, rid: &str) -> Result<(), String> {
        let rd = self.rd_find(rid)?;
        self.inner.lk().rundowns.remove(rd.str("id").unwrap_or(""));
        self.broadcast(&Json::obj().with("t", "state"));
        Ok(())
    }

    // ---------------- Word Replace (lista zamian słów stanowiska) ----------------

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

    pub fn carousel_start(self: &Arc<Self>, playlist_id: &str) -> Result<(), String> {
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
        let gen = self.carousel_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let me = Arc::clone(self);
        let pid = pl.str("id").unwrap_or("").to_string();
        std::thread::spawn(move || me.carousel_loop(gen, pid));
        Ok(())
    }

    pub fn carousel_stop(&self, take_out: bool) {
        self.carousel_gen.fetch_add(1, Ordering::SeqCst);
        let mut inner = self.inner.lk();
        let was = inner.carousel.take();
        drop(inner);
        if take_out && was.is_some() {
            // zdejmij warstwy używane przez playlistę
            let pl = self.show().arr("playlists").iter().find(|p| p.str("id") == was.as_ref().map(|w| w.0.as_str())).cloned();
            if let Some(pl) = pl {
                let mut layers: Vec<String> = pl.arr("items").iter().filter_map(|it| self.page_layer(it.str("page")?)).collect();
                layers.dedup();
                for l in layers {
                    self.out(&l).ok();
                }
            }
        }
        self.broadcast(&Json::obj().with("t", "state"));
    }

    fn carousel_loop(self: Arc<Self>, gen: u64, playlist_id: String) {
        let alive = |e: &Engine| e.carousel_gen.load(Ordering::SeqCst) == gen;
        let mut idx = 0usize;
        let mut prev_layer: Option<String> = None;
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
                inner.carousel = Some((playlist_id.clone(), idx));
            }
            let layer = self.page_layer(&page);
            if let (Some(p), Some(l)) = (&prev_layer, &layer) {
                if p != l {
                    self.out(p).ok();
                }
            }
            if self.take(&page, None).is_err() {
                idx += 1;
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            self.broadcast(&Json::obj().with("t", "state"));
            prev_layer = layer;
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
            if let Some(l) = prev_layer {
                self.out(&l).ok();
            }
            self.inner.lk().carousel = None;
            self.broadcast(&Json::obj().with("t", "state"));
        }
    }
}

fn bump_rev(show: &mut Json) {
    let r = show.num("rev").unwrap_or(0.0);
    show.set("rev", (r + 1.0).into());
}

fn normalize_show(mut show: Json) -> Json {
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

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = include_str!("../web/demo_show.json");

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("vizrs-test-{}-{}", std::process::id(), name));
        std::fs::remove_dir_all(&d).ok();
        d
    }

    fn engine(name: &str) -> (Arc<Engine>, PathBuf) {
        let d = tmp(name);
        (Engine::new(d.clone(), DEMO), d)
    }

    fn on_air(e: &Engine) -> Vec<(String, f64)> {
        e.state()
            .arr("layers")
            .iter()
            .map(|l| (l.str("layer").unwrap_or("").to_string(), l.num("stop").unwrap_or(-1.0)))
            .collect()
    }

    #[test]
    fn take_continue_out_cycle() {
        let (e, d) = engine("cycle");
        let r = e.take("1001", None).unwrap();
        let layer = r.str("layer").unwrap().to_string();
        assert_eq!(on_air(&e), vec![(layer.clone(), 0.0)]);
        e.cont(&layer).unwrap();
        e.cont(&layer).unwrap(); // nie wychodzi poza ostatni stop
        let stops = e.state().arr("layers")[0].num("stops").unwrap();
        assert!(on_air(&e)[0].1 <= (stops - 1.0).max(0.0));
        e.out(&layer).unwrap();
        assert!(on_air(&e).is_empty());
        e.out(&layer).unwrap(); // OUT pustej warstwy nie jest błędem
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn take_unknown_page_is_error() {
        let (e, d) = engine("unknown");
        assert!(e.take("9999", None).is_err());
        assert!(e.cont("brak").is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn same_layer_replaces_previous() {
        let (e, d) = engine("replace");
        e.take("1001", None).unwrap();
        e.take("1002", None).unwrap();
        assert_eq!(on_air(&e).len(), 1);
        assert_eq!(e.state().arr("layers")[0].get("page").unwrap().num("number"), Some(1002.0));
        e.clear();
        assert!(on_air(&e).is_empty());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn update_live_and_persist() {
        let (e, d) = engine("update");
        let vals = Json::parse(r#"{"headline":"NOWY"}"#).unwrap();
        // poza anteną: tylko zapis
        assert_eq!(e.update("1001", &vals, true).unwrap().get("onAir"), Some(&Json::Bool(false)));
        let page = Engine::find_page(&e.show(), "1001").unwrap();
        assert_eq!(page.get("values").unwrap().str("headline"), Some("NOWY"));
        // na antenie, bez zapisu strony
        e.take("1001", None).unwrap();
        let live = Json::parse(r#"{"headline":"NA ZYWO"}"#).unwrap();
        assert_eq!(e.update("1001", &live, false).unwrap().get("onAir"), Some(&Json::Bool(true)));
        let page = Engine::find_page(&e.show(), "1001").unwrap();
        assert_eq!(page.get("values").unwrap().str("headline"), Some("NOWY"));
        assert!(e.update("1001", &Json::Null, true).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn page_numbers_unique_and_scene_required() {
        let (e, d) = engine("numbers");
        let sid = e.show().arr("scenes")[0].str("id").unwrap().to_string();
        let mk = |n: Option<f64>, scene: &str| {
            let mut o = Json::obj().with("scene", scene);
            if let Some(n) = n {
                o.set("number", n.into());
            }
            o
        };
        assert!(e.upsert("pages", mk(Some(1001.0), &sid)).is_err()); // zajęty
        assert!(e.upsert("pages", mk(None, "brak")).is_err()); // nieistniejący szablon
        let p = e.upsert("pages", mk(None, &sid)).unwrap();
        let n1 = p.num("number").unwrap();
        let p2 = e.upsert("pages", mk(None, &sid)).unwrap();
        assert!(p2.num("number").unwrap() > n1);
        assert!(e.upsert("scenes", Json::Null).is_err());
        assert!(e.upsert("zly", Json::obj()).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn deleting_scene_removes_its_pages_and_playlist_items() {
        let (e, d) = engine("cascade");
        let show = e.show();
        let sid = show.arr("pages")[0].str("scene").unwrap().to_string();
        let before = show.arr("pages").len();
        e.delete("scenes", &sid).unwrap();
        let after = e.show();
        assert!(after.arr("pages").len() < before);
        assert!(after.arr("pages").iter().all(|p| p.str("scene") != Some(sid.as_str())));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn poisoned_lock_does_not_break_engine() {
        let (e, d) = engine("poison");
        let e2 = Arc::clone(&e);
        let _ = std::thread::spawn(move || {
            let _g = e2.inner.lock().unwrap();
            panic!("test: panika z trzymaną blokadą");
        })
        .join();
        assert!(e.inner.is_poisoned());
        assert!(e.take("1001", None).is_ok());
        assert_eq!(on_air(&e).len(), 1);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn stalled_sse_client_is_dropped() {
        let (e, d) = engine("sse");
        let _rx = e.subscribe(); // nigdy nie czyta
        assert_eq!(e.clients(), 1);
        for _ in 0..(SUB_QUEUE + 5) {
            e.clear();
        }
        assert_eq!(e.clients(), 0);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn backups_are_written_and_restorable() {
        let (e, d) = engine("backup");
        let start = e.backups().into_iter().find(|n| n.len() == 25).expect("kopia bieżąca przy starcie");
        let sid = e.show().arr("scenes")[0].str("id").unwrap().to_string();
        e.upsert("pages", Json::obj().with("scene", sid.as_str()).with("number", 4242.0)).unwrap();
        assert!(Engine::find_page(&e.show(), "4242").is_some());
        // przywrócenie stanu sprzed zmiany
        e.restore_backup(&start).unwrap();
        assert!(Engine::find_page(&e.show(), "4242").is_none());
        // a stan sprzed przywrócenia da się odzyskać z najnowszej kopii bieżącej
        let undo = e.backups().into_iter().find(|n| n.len() == 25).unwrap();
        e.restore_backup(&undo).unwrap();
        assert!(Engine::find_page(&e.show(), "4242").is_some());
        assert!(e.restore_backup("../../etc/passwd").is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn write_failure_is_reported_not_silent() {
        let (e, d) = engine("diskfail");
        assert!(e.save_error().is_none());
        // zastąp katalog danych plikiem – zapis musi się nie udać
        std::fs::remove_dir_all(&d).unwrap();
        std::fs::write(&d, b"x").unwrap();
        let sid = e.show().arr("scenes")[0].str("id").unwrap().to_string();
        let r = e.upsert("pages", Json::obj().with("scene", sid.as_str()));
        assert!(r.unwrap_err().starts_with("zapis"));
        assert!(e.save_error().is_some());
        assert!(e.state().get("saveError").map(|v| *v != Json::Null).unwrap_or(false));
        std::fs::remove_file(&d).ok();
    }

    #[test]
    fn corrupted_show_falls_back_to_demo_and_keeps_copy() {
        let d = tmp("corrupt");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("show.json"), "{ nie json").unwrap();
        let e = Engine::new(d.clone(), DEMO);
        assert!(!e.show().arr("pages").is_empty());
        assert!(d.join("show.json.bak").exists());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn carousel_cycles_and_stops() {
        let (e, d) = engine("carousel");
        let pages: Vec<String> = e.show().arr("pages").iter().take(2).map(|p| p.str("id").unwrap().to_string()).collect();
        let items = Json::Arr(pages.iter().map(|p| Json::obj().with("page", p.as_str()).with("duration", 0.5)).collect());
        let pl = e.upsert("playlists", Json::obj().with("name", "t").with("loop", true).with("items", items)).unwrap();
        e.carousel_start(pl.str("id").unwrap()).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(e.state().get("carousel").map(|c| c.is_obj()).unwrap_or(false));
        e.carousel_stop(true);
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(e.state().get("carousel"), Some(&Json::Null));
        assert!(e.carousel_start("nie ma").is_err());
        std::fs::remove_dir_all(d).ok();
    }

    fn first_scene_id(e: &Engine) -> String {
        e.show().arr("scenes")[0].str("id").unwrap().to_string()
    }

    #[test]
    fn rev_conflict_and_bump() {
        let (e, d) = engine("rev");
        let sid = first_scene_id(&e);
        let p = e.upsert("pages", Json::obj().with("scene", sid.as_str()).with("name", "A")).unwrap();
        assert_eq!(p.num("rev"), Some(1.0));
        // poprawna edycja z aktualnym rev
        let mut edit = p.clone();
        edit.set("name", "B".into());
        let p2 = e.upsert("pages", edit.clone()).unwrap();
        assert_eq!(p2.num("rev"), Some(2.0));
        // ten sam (już nieaktualny) rev → konflikt
        let err = e.upsert("pages", edit).unwrap_err();
        assert!(err.starts_with("konflikt"), "{err}");
        // bez rev (np. curl) – przyjęte
        let mut norev = p2.clone();
        if let Json::Obj(v) = &mut norev {
            v.retain(|(k, _)| k != "rev");
        }
        assert_eq!(e.upsert("pages", norev).unwrap().num("rev"), Some(3.0));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn show_rev_increments_and_import_conflict() {
        let (e, d) = engine("showrev");
        let r0 = e.show().num("rev").unwrap();
        let sid = first_scene_id(&e);
        e.upsert("pages", Json::obj().with("scene", sid.as_str())).unwrap();
        assert!(e.show().num("rev").unwrap() > r0);
        // import ze starym rev → konflikt, z force → ok
        let mut old = e.show();
        old.set("rev", r0.into());
        assert!(e.put_show(old.clone(), false).unwrap_err().starts_with("konflikt"));
        assert!(e.put_show(old, true).is_ok());
        // plik bez rev (import cudzego show) – ok
        let mut norev = e.show();
        if let Json::Obj(v) = &mut norev {
            v.retain(|(k, _)| k != "rev");
        }
        let r = e.put_show(norev, false);
        assert!(r.is_ok(), "{r:?}");
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn validation_blocks_garbage() {
        let (e, d) = engine("validate");
        let before = e.show();
        assert!(e.put_show(Json::parse(r#"{"scenes":"x"}"#).unwrap(), true).is_err());
        assert!(e.put_show(Json::parse(r#"{"scenes":[1,2,null]}"#).unwrap(), true).is_err());
        assert_eq!(e.show(), before); // show nietknięty
        let sid = first_scene_id(&e);
        assert!(e.upsert("pages", Json::obj().with("scene", sid.as_str()).with("number", 1e300)).is_err());
        assert!(e.upsert("pages", Json::obj().with("scene", sid.as_str()).with("number", -5.0)).is_err());
        assert!(e.upsert("scenes", Json::parse(r#"{"name":"x","stops":[-3]}"#).unwrap()).is_err());
        assert!(e.update("1001", &Json::parse(r#"{"a":{"b":1}}"#).unwrap(), true).is_err());
        assert!(e.take("1001", Some(&Json::parse(r#"{"a":[1]}"#).unwrap())).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn scene_update_is_pending_until_deployed() {
        let (e, d) = engine("deploy");
        let r = e.take("1001", None).unwrap();
        let layer = r.str("layer").unwrap().to_string();
        assert_eq!(e.state().arr("layers")[0].get("pending"), Some(&Json::Bool(false)));
        // zmiana szablonu na antenie: wizja zostaje, pojawia się „pending”
        let mut scene = e.show().arr("scenes").iter().find(|s| s.str("id") == Some(first_scene_id(&e).as_str())).cloned().unwrap();
        let page_scene = Engine::find_page(&e.show(), "1001").unwrap().str("scene").unwrap().to_string();
        scene = e.show().arr("scenes").iter().find(|s| s.str("id") == Some(page_scene.as_str())).cloned().unwrap_or(scene);
        scene.set("name", "Zmieniona".into());
        e.upsert("scenes", scene).unwrap();
        assert_eq!(e.state().arr("layers")[0].get("pending"), Some(&Json::Bool(true)));
        assert_ne!(e.state().arr("layers")[0].str("scene"), Some("Zmieniona"));
        e.deploy(&layer).unwrap();
        assert_eq!(e.state().arr("layers")[0].get("pending"), Some(&Json::Bool(false)));
        assert_eq!(e.state().arr("layers")[0].str("scene"), Some("Zmieniona"));
        assert!(e.deploy("pusta").is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn scene_errors_are_deduplicated_and_capped() {
        let (e, d) = engine("errors");
        e.report_error("s1", "main", "boom");
        e.report_error("s1", "main", "boom");
        let list = e.errors();
        assert_eq!(list.as_arr().len(), 1);
        assert_eq!(list.as_arr()[0].num("count"), Some(2.0));
        assert_eq!(e.state().num("errors"), Some(1.0));
        for i in 0..200 {
            e.report_error("s", "l", &format!("m{i}"));
        }
        assert!(e.errors().as_arr().len() <= MAX_ERRS);
        e.clear_errors();
        assert!(e.errors().as_arr().is_empty());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn health_reports_state() {
        let (e, d) = engine("health");
        let h = e.health();
        assert_eq!(h.str("status"), Some("ok"));
        assert_eq!(h.num("layersOnAir"), Some(0.0));
        assert!(h.str("version").is_some());
        e.take("1001", None).unwrap();
        assert_eq!(e.health().num("layersOnAir"), Some(1.0));
        assert!(e.health().num("lastEventAgeS").is_some());
        std::fs::remove_dir_all(d).ok();
    }

    fn make_rundown(e: &Engine, n: usize) -> String {
        let pages: Vec<String> = e.show().arr("pages").iter().take(n).map(|p| p.str("id").unwrap().to_string()).collect();
        let items = Json::Arr(pages.iter().enumerate().map(|(i, p)| Json::obj().with("id", format!("it{i}")).with("page", p.as_str())).collect());
        let r = e.upsert("rundowns", Json::obj().with("name", "R").with("items", items)).unwrap();
        r.str("id").unwrap().to_string()
    }

    #[test]
    fn rundown_next_walks_list_and_marks_done() {
        let (e, d) = engine("rundown");
        let rid = make_rundown(&e, 3);
        assert_eq!(e.state().arr("rundowns")[0].str("cursor"), Some("it0"));
        e.rundown_take(&rid, None).unwrap();
        let st = e.state();
        let r = &st.arr("rundowns")[0];
        assert_eq!(r.str("cursor"), Some("it1"));
        assert_eq!(r.str("last"), Some("it0"));
        e.rundown_take(&rid, None).unwrap();
        e.rundown_take(&rid, None).unwrap();
        let st = e.state();
        let r = &st.arr("rundowns")[0];
        assert_eq!(r.get("cursor"), Some(&Json::Null)); // za ostatnią pozycją
        let done: Vec<&str> = r.arr("done").iter().filter_map(|d| d.as_str()).collect();
        assert_eq!(done, vec!["it0", "it1"]);
        assert!(e.rundown_take(&rid, None).unwrap_err().contains("koniec"));
        // ręczne ustawienie kursora i reset
        e.rundown_cursor(&rid, "it1").unwrap();
        assert_eq!(e.state().arr("rundowns")[0].str("cursor"), Some("it1"));
        assert!(e.rundown_cursor(&rid, "nie-ma").is_err());
        e.rundown_reset(&rid).unwrap();
        assert_eq!(e.state().arr("rundowns")[0].str("cursor"), Some("it0"));
        // wzięcie wskazanej pozycji
        e.rundown_take(&rid, Some("it2")).unwrap();
        assert!(e.rundown_take(&rid, Some("zly")).is_err());
        assert!(e.rundown_take("brak", None).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn deleting_page_removes_it_from_rundown() {
        let (e, d) = engine("rdcascade");
        let rid = make_rundown(&e, 2);
        let pid = e.show().arr("rundowns")[0].arr("items")[0].str("page").unwrap().to_string();
        e.delete("pages", &pid).unwrap();
        let rd = e.show().arr("rundowns").iter().find(|r| r.str("id") == Some(rid.as_str())).cloned().unwrap();
        assert_eq!(rd.arr("items").len(), 1);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn arm_goes_to_preview_not_program() {
        let (e, d) = engine("arm");
        let rx = e.subscribe();
        // dwie migawki startowe: program i preview
        let first = Json::parse(&rx.recv().unwrap()).unwrap();
        let second = Json::parse(&rx.recv().unwrap()).unwrap();
        assert_eq!(first.get("bus"), None);
        assert_eq!(second.str("bus"), Some("preview"));
        e.arm("1001", None).unwrap();
        assert!(on_air(&e).is_empty());
        let st = e.state();
        assert_eq!(st.arr("preview").len(), 1);
        let ev = Json::parse(&rx.recv().unwrap()).unwrap();
        assert_eq!(ev.str("t"), Some("take"));
        assert_eq!(ev.str("bus"), Some("preview"));
        e.preview_clear();
        assert!(e.state().arr("preview").is_empty());
        assert!(e.arm("999999", None).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn variables_flat_for_companion() {
        let (e, d) = engine("vars");
        assert_eq!(e.variables().num("layers_on_air"), Some(0.0));
        e.take("1001", None).unwrap();
        let v = e.variables();
        assert_eq!(v.num("layers_on_air"), Some(1.0));
        assert_eq!(v.str("layer_main"), Some("1001"));
        assert!(v.str("layer_main_name").unwrap().contains("Belka"));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn dock_override_state_and_snapshot() {
        let (e, d) = engine("dock");
        let mut scene = e.show().arr("scenes").iter().find(|x| x.str("layer") == Some("main")).cloned().unwrap();
        scene.set("dock", Json::parse(r#"[{"target":"layer:back","dx":0,"dy":-90,"dur":0.6}]"#).unwrap());
        e.upsert("scenes", scene).unwrap();
        let r = e.take("1001", None).unwrap();
        let layer = r.str("layer").unwrap().to_string();
        assert_eq!(e.state().arr("layers")[0].get("hasDock"), Some(&Json::Bool(true)));
        assert_eq!(e.state().arr("layers")[0].get("dockOff"), Some(&Json::Bool(false)));
        e.set_dock(&layer, true).unwrap();
        assert_eq!(e.state().arr("layers")[0].get("dockOff"), Some(&Json::Bool(true)));
        assert_eq!(e.snapshot().arr("dockOff").len(), 1);
        e.set_dock(&layer, false).unwrap();
        assert!(e.snapshot().arr("dockOff").is_empty());
        assert!(e.set_dock("zła warstwa!", true).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn anim_presets_roundtrip() {
        let (e, d) = engine("animpresets");
        assert!(matches!(e.anim_presets(), Json::Arr(a) if a.is_empty()));
        let list = Json::parse(r#"[{"id":"a","name":"Wjazd","duration":0.5,"tracks":{"x":[{"t":0,"v":-100},{"t":0.5,"v":0}]}}]"#).unwrap();
        e.put_anim_presets(list.clone()).unwrap();
        assert_eq!(e.anim_presets(), list);
        assert!(e.put_anim_presets(Json::parse(r#"[{"id":"a"}]"#).unwrap()).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn edit_locks_conflict_refresh_and_expire() {
        let (e, d) = engine("locks");
        assert!(e.lock("pages", "p1", "Anna").is_ok());
        assert!(e.lock("pages", "p1", "Anna").is_ok()); // odświeżenie przez właściciela
        let err = e.lock("pages", "p1", "Jan").unwrap_err();
        assert!(err.starts_with("konflikt") && err.contains("Anna"), "{err}");
        assert!(e.lock("scenes", "p1", "Jan").is_ok()); // inny rodzaj obiektu
        assert_eq!(e.locks_json().as_arr().len(), 2);
        assert_eq!(e.state().arr("locks").len(), 2);
        // zwolnienie tylko przez właściciela
        assert_eq!(e.unlock("pages", "p1", "Jan", false).get("released"), Some(&Json::Bool(false)));
        assert_eq!(e.unlock("pages", "p1", "Anna", false).get("released"), Some(&Json::Bool(true)));
        // przejęcie (force)
        e.lock("pages", "p2", "Anna").unwrap();
        assert_eq!(e.unlock("pages", "p2", "Jan", true).get("released"), Some(&Json::Bool(true)));
        // wygaśnięcie po TTL
        e.lock("pages", "p3", "Anna").unwrap();
        e.locks.lk().get_mut("pages:p3").unwrap().at = now_ms() - LOCK_TTL_MS - 1;
        assert!(e.lock("pages", "p3", "Jan").is_ok());
        // walidacja
        assert!(e.lock("zly", "p", "A").is_err());
        assert!(e.lock("pages", "", "A").is_err());
        assert!(e.lock("pages", "p9", "  ").is_err());
        std::fs::remove_dir_all(d).ok();
    }
}
