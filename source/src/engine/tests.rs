#[test]
fn show_slug_and_id() {
    assert_eq!(super::show_slug("Fakty Wieczór – 19:00"), "fakty-wieczor-19-00");
    assert_eq!(super::show_slug("Żółć"), "zolc");
    assert_eq!(super::show_slug("!!!"), "show");
    assert!(super::valid_show_id("info24-2"));
    assert!(!super::valid_show_id("../x") && !super::valid_show_id(".active") && !super::valid_show_id(""));
}

use super::*;

const DEMO: &str = include_str!("../../web/demo_show.json");

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
fn set_layer_moves_on_air_without_new_version() {
    let (e, d) = engine("setlayer");
    e.take("1001", None).unwrap();
    e.arm("1001", None).unwrap();
    e.set_dock("main", true).unwrap();
    let r = e.set_layer("1001", "z5_belka", false).unwrap();
    assert_eq!((r.str("from"), r.get("onAir")), (Some("main"), Some(&Json::Bool(true))));
    assert_eq!(on_air(&e), vec![("z5_belka".to_string(), 0.0)]);
    let st = e.state();
    let l = &st.arr("layers")[0];
    assert_eq!(l.get("pending"), Some(&Json::Bool(false))); // zmiana warstwy to nie nowa wersja szablonu
    assert_eq!(l.get("dockOff"), Some(&Json::Bool(true)));
    assert_eq!(e.page_layer("1001").as_deref(), Some("z5_belka"));
    assert_eq!(e.snapshot_preview().arr("layers")[0].str("layer"), Some("z5_belka"));
    // OUT po numerze strony trafia w nową warstwę
    e.out("1001").unwrap();
    assert!(on_air(&e).is_empty());
    // zmiana bez anteny i ponowne ustawienie tej samej warstwy
    assert_eq!(e.set_layer("1001", "z5_belka", false).unwrap().get("onAir"), Some(&Json::Bool(false)));
    assert!(e.set_layer("1001", "zła nazwa", false).is_err());
    assert!(e.set_layer("9999", "main", false).is_err());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn set_layer_needs_force_to_replace_on_air() {
    let (e, d) = engine("setlayerbusy");
    let show = e.show();
    let other = show.arr("scenes").iter().find(|s| s.str("layer") == Some("back")).unwrap().str("id").unwrap().to_string();
    let page = show.arr("pages").iter().find(|p| p.str("scene") == Some(&other)).unwrap().num("number").unwrap() as i64;
    e.take("1001", None).unwrap();
    e.take(&page.to_string(), None).unwrap();
    assert!(e.set_layer("1001", "back", false).unwrap_err().starts_with("konflikt"));
    assert_eq!(e.page_layer("1001").as_deref(), Some("main")); // odmowa niczego nie zmienia
    assert_eq!(on_air(&e).len(), 2);
    e.set_layer("1001", "back", true).unwrap();
    assert_eq!(on_air(&e), vec![("back".to_string(), 0.0)]);
    assert_eq!(e.state().arr("layers")[0].get("page").unwrap().num("number"), Some(1001.0));
    // gdy szablonu nie ma na antenie, zajęta warstwa nie przeszkadza
    e.clear();
    e.take(&page.to_string(), None).unwrap();
    e.set_layer("1001", "main", false).unwrap();
    assert!(e.set_layer("1001", "back", false).is_ok());
    std::fs::remove_dir_all(&d).ok();
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
fn channels_have_their_own_air() {
    let (e, d) = engine("channels");
    let rx = e.subscribe();
    while rx.try_recv().is_ok() {}
    e.channel_take("2", "1001", None).unwrap();
    let ev = Json::parse(&rx.try_recv().unwrap()).unwrap();
    assert_eq!((ev.str("t"), ev.str("bus")), (Some("take"), Some("ch-2")));
    // główna antena i inne kanały tego nie widzą
    assert!(e.state().arr("layers").is_empty());
    assert!(e.channel_state("3").unwrap().arr("layers").is_empty());
    assert_eq!(e.channel_state("2").unwrap().arr("layers").len(), 1);
    // CLEAR głównej anteny nie dotyka kanału, a nowy klient dostaje jego stan
    e.clear();
    let fresh = e.subscribe();
    let snapshots: Vec<Json> = fresh.try_iter().map(|m| Json::parse(&m).unwrap()).collect();
    assert_eq!(snapshots.iter().find(|s| s.str("bus") == Some("ch-2")).unwrap().arr("layers").len(), 1);

    let update = e.update("1001", &Json::obj().with("x", "y"), false).unwrap();
    assert_eq!(update.get("onAir"), Some(&Json::Bool(true)));
    e.channel_out("2", "1001").unwrap();
    assert!(e.channel_state("2").unwrap().arr("layers").is_empty());
    // rundown: własny kursor anteny
    let rd = e.upsert("rundowns", Json::parse(r#"{"name":"R","items":[{"id":"a","page":"1001"},{"id":"b","page":"1002"}]}"#).unwrap()).unwrap();
    let rid = rd.str("id").unwrap();
    e.rundown_take_on("2", rid, None).unwrap();
    assert_eq!(e.rundown_take_on("3", rid, None).unwrap().str("item"), Some("a"));
    assert!(e.state().arr("layers").is_empty());
    assert_eq!(e.state_on("2").unwrap().arr("rundowns")[0].str("cursor"), Some("b"));
    // dock i lista kanałów po restarcie
    e.set_dock_on("2", "main", true).unwrap();
    assert_eq!(e.state_on("2").unwrap().arr("layers")[0].get("dockOff"), Some(&Json::Bool(true)));
    assert_eq!(load_channels(&d).iter().filter(|(c, _)| c == "2" || c == "3").count(), 2);
    e.channel_add("2", Some("R7\n Info")).unwrap();
    assert_eq!(e.state().get("channelNames").unwrap().str("2"), Some("R7 Info"));
    assert_eq!(load_labels(&d), [("2".to_string(), "R7 Info".to_string())]);
    e.channel_delete("3").unwrap();
    assert!(!e.channel_names().contains(&"3".to_string()));
    assert!(e.channel_take("zła nazwa", "1001", None).is_err());
    for i in 0..MAX_CHANNELS {
        e.channel_take(&format!("n{i}"), "1001", None).ok();
    }
    assert!(e.channel_take("nadmiar", "1001", None).is_err());
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
