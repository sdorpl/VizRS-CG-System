//! Routing HTTP: `/health` i API `/api/*` (show, playout, rundowny, makra, zasoby, diagnostyka, biblioteka show).

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::commands::{each, start_macro};
use crate::config::Config;
use crate::engine::{self, Engine};
use crate::http::{Request, Response};
use crate::json::Json;
use crate::static_files::{self, safe_name};
use crate::{fonts, ticker, HEALTH_PINGS};

/// Limit łącznego rozmiaru katalogu assets.
const MAX_ASSETS_TOTAL: u64 = 2 * 1024 * 1024 * 1024;
/// Dozwolone rozszerzenia zasobów wgrywanych przez API.
const ASSET_EXT: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "mp4", "webm", "ttf", "otf", "woff", "woff2"];

/// GET-y, które zmieniają stan anteny (Companion/Stream Deck) – trafiają do logu audytowego.
pub(crate) fn is_playout_get(path: &str) -> bool {
    ["/api/take/", "/api/arm/", "/api/preview/", "/api/rundown/", "/api/macro/", "/api/continue/", "/api/out/", "/api/clear", "/api/update/", "/api/deploy/", "/api/dock/", "/api/carousel/"].iter().any(|p| path.starts_with(p))
}

fn res<T>(r: Result<T, String>, f: impl FnOnce(T) -> Response) -> Response {
    match r {
        Ok(v) => f(v),
        Err(e) if e.starts_with("zapis") => Response::err(500, &e),
        Err(e) if e.starts_with("konflikt") => Response::err(409, &e),
        Err(e) => Response::err(400, &e),
    }
}

/// `/health`: stan silnika, instancja serwera i to, czy wyjście potrzebuje tokenu (Trio podpowiada wtedy adres dla OBS
/// z `?token=`). Same flagi, nigdy wartości tokenów.
fn health(engine: &Engine, cfg: &Config) -> Json {
    engine.health().with("instance", cfg.instance.as_str()).with("tokenRequired", cfg.token.is_some()).with("outputToken", cfg.output_token.is_some())
}

pub(crate) fn route(req: &Request, engine: &Arc<Engine>, cfg: &Config) -> Response {
    let m = req.method.as_str();
    if m == "OPTIONS" {
        return Response::new(204, "text/plain", "");
    }
    let segs = req.segs();
    let s: Vec<&str> = segs.to_vec();
    // antena komendy: kanał albo "" (główna)
    let ant = req.q("channel").unwrap_or("");

    if m == "GET" {
        if let Some(r) = static_files::serve(req, &s, engine, cfg) {
            return r;
        }
    }

    if s == ["health"] && m == "GET" {
        if req.q("attach").is_some() {
            HEALTH_PINGS.fetch_add(1, Ordering::SeqCst);
        }
        return Response::json(&health(engine, cfg));
    }
    if s.first() != Some(&"api") {
        return Response::err(404, "nie znaleziono");
    }
    let body = match req.json() {
        Ok(b) => b,
        Err(e) if m != "GET" && !req.path.starts_with("/api/assets") => return Response::err(400, &e),
        Err(_) => Json::Null,
    };
    let a = Api { req, engine, cfg, ant, body };
    let rest = &s[1..];
    match rest.first().copied() {
        Some("show" | "state") => show_api(a, m, rest),
        Some(k) if engine::KINDS.contains(&k) => show_api(a, m, rest),
        Some("take" | "arm" | "preview" | "continue" | "out" | "clear" | "update" | "deploy" | "layer" | "dock") => playout_api(a, m, rest),
        Some("rundown" | "macro" | "variables" | "ticker" | "carousel") => automation_api(a, m, rest),
        Some("assets") => assets_api(a, m, rest),
        Some("report" | "errors" | "health" | "locks" | "lock" | "unlock" | "animpresets" | "wordreplace" | "fonts" | "audit") => diag_api(a, m, rest),
        Some("channels" | "shows" | "backups") => library_api(a, m, rest),
        _ => Response::err(404, "nieznany endpoint"),
    }
}

/// Jedno żądanie `/api/*` po odczytaniu treści.
struct Api<'a> {
    req: &'a Request,
    engine: &'a Arc<Engine>,
    cfg: &'a Config,
    /// antena komendy: kanał albo "" (główna)
    ant: &'a str,
    body: Json,
}

/// Wartości pól można też podać w query (?name=Jan&title=...) – wygodne dla Companion/Stream Deck.
fn query_values(req: &Request) -> Json {
    let mut v = Json::obj();
    for (k, val) in &req.query {
        v.set(k, val.as_str().into());
    }
    v
}

/// Show, stan anteny i elementy show (sceny, strony, playlisty, rundowny, makra).
fn show_api(a: Api, m: &str, rest: &[&str]) -> Response {
    let Api { req, engine, ant, body, .. } = a;
    match (m, rest) {
        ("GET", ["show"]) => Response::json(&engine.show()),
        ("PUT", ["show"]) => {
            let force = req.q("force") == Some("1");
            res(engine.put_show(body, force), |_| Response::ok())
        }
        ("GET", ["state"]) => res(engine.state_on(ant), |j| Response::json(&j)),

        ("POST" | "PUT", [kind]) if engine::KINDS.contains(kind) => {
            res(engine.upsert(kind, body), |it| Response::json(&it))
        }
        ("PUT", [kind, id]) if engine::KINDS.contains(kind) => {
            let mut b = body;
            b.set("id", (*id).into());
            res(engine.upsert(kind, b), |it| Response::json(&it))
        }
        ("DELETE", [kind, id]) if engine::KINDS.contains(kind) => res(engine.delete(kind, id), |_| Response::ok()),
        _ => Response::err(404, "nieznany endpoint"),
    }
}

/// Playout: TAKE/ARM/CONTINUE/OUT, podgląd, CLEAR, UPDATE, deploy, warstwa, powiązania (GET dozwolony dla prostych integracji).
fn playout_api(a: Api, m: &str, rest: &[&str]) -> Response {
    let Api { req, engine, ant, body, .. } = a;
    match (m, rest) {
        // ---------- playout (GET dozwolony dla prostych integracji) ----------
        ("POST" | "GET", ["take", key]) | ("POST" | "GET", ["arm", key]) => {
            let arm = rest[0] == "arm";
            let mut ov = query_values(req);
            if let Some(Json::Obj(_)) = body.get("values") {
                ov = body.get("values").cloned().unwrap();
            }
            let ov = if matches!(&ov, Json::Obj(v) if v.is_empty()) { None } else { Some(ov) };
            res(
                each(key, |k| match (arm, req.q("channel")) {
                    (true, _) => engine.arm(k, ov.as_ref()),
                    (false, Some(ch)) => engine.channel_take(ch, k, ov.as_ref()),
                    (false, None) => engine.take(k, ov.as_ref()),
                }),
                |j| Response::json(&j),
            )
        }
        ("POST" | "GET", ["preview", "clear"]) => {
            engine.preview_clear();
            Response::ok()
        }
        ("POST" | "GET", ["continue", key]) => res(each(key, |k| req.q("channel").map_or_else(|| engine.cont(k), |ch| engine.channel_cont(ch, k))), |j| Response::json(&j)),
        ("POST" | "GET", ["out", key]) => res(each(key, |k| req.q("channel").map_or_else(|| engine.out(k), |ch| engine.channel_out(ch, k))), |j| Response::json(&j)),
        ("POST" | "GET", ["clear"]) if req.q("channel").is_some() => res(engine.channel_clear(req.q("channel").unwrap_or("")), |j| Response::json(&j)),
        ("POST" | "GET", ["clear"]) => {
            engine.clear();
            Response::ok()
        }
        // update: zmienia pola strony i aktualizuje na żywo, jeśli on-air.
        // ?save=0 – tylko na żywo, bez zapisu do strony
        ("POST" | "PATCH" | "GET", ["update", key]) => {
            let vals = if body.is_obj() { body.get("values").cloned().unwrap_or(body.clone()) } else { query_values(req) };
            let mut vals = vals;
            if let Json::Obj(items) = &mut vals {
                items.retain(|(k, _)| k != "save");
            }
            let persist = req.q("save") != Some("0");
            res(engine.update(key, &vals, persist), |j| Response::json(&j))
        }
        ("POST" | "GET", ["deploy", key]) => res(engine.deploy_on(ant, key), |j| Response::json(&j)),
        // layer: zmienia warstwę szablonu strony; grafika na antenie przechodzi bez ponownej animacji
        ("POST", ["layer", key]) => res(engine.set_layer(key, body.str("layer").unwrap_or(""), req.q("force") == Some("1")), |j| Response::json(&j)),
        ("POST" | "GET", ["dock", layer, mode]) if *mode == "off" || *mode == "on" => res(engine.set_dock_on(ant, layer, *mode == "off"), |j| Response::json(&j)),
        _ => Response::err(404, "nieznany endpoint"),
    }
}

/// Rundowny, makra, zmienne, ticker i karuzela.
fn automation_api(a: Api, m: &str, rest: &[&str]) -> Response {
    let Api { req, engine, ant, body, .. } = a;
    match (m, rest) {
        // ---------- rundown ----------
        ("POST" | "GET", ["rundown", id, "next"]) => res(engine.rundown_take_on(ant, id, None), |j| Response::json(&j)),
        ("POST" | "GET", ["rundown", id, "take", item]) => res(engine.rundown_take_on(ant, id, Some(item)), |j| Response::json(&j)),
        ("POST" | "GET", ["rundown", id, "cursor", item]) => res(engine.rundown_cursor_on(ant, id, item), |_| Response::ok()),
        ("POST" | "GET", ["rundown", id, "reset"]) => res(engine.rundown_reset_on(ant, id), |_| Response::ok()),

        // ---------- makra ----------
        ("POST" | "GET", ["macro", "stop"]) => {
            engine.macro_stop(ant);
            Response::ok()
        }
        ("POST" | "GET", ["macro", key]) => res(start_macro(engine, ant, key), |_| Response::ok()),
        ("GET", ["variables"]) => Response::json(&engine.variables()),
        // ticker: lista wiadomości strony (GET) i zmiany z automatycznym UPDATE, gdy strona jest na antenie
        ("GET", ["ticker", key]) => res(engine.ticker(key, ticker::Op::List), |j| Response::json(&j)),
        ("POST" | "GET", ["ticker", key, op @ ..]) => {
            let item = body.str("item").map(str::to_string).or_else(|| req.q("item").map(str::to_string)).unwrap_or_default();
            let line = match op {
                ["add"] => format!("ADD {item}"),
                ["set", n] => format!("SET {n} {item}"),
                ["del", n] => format!("DEL {n}"),
                ["clear"] => "CLEAR".to_string(),
                _ => "?".to_string(),
            };
            res(ticker::parse_op(&line).and_then(|o| engine.ticker(key, o)), |j| Response::json(&j))
        }
        ("POST" | "GET", ["carousel", "start", id]) => res(engine.carousel_start_on(ant, id), |_| Response::ok()),
        ("POST" | "GET", ["carousel", "stop"]) => {
            engine.carousel_stop_on(ant, req.q("out") != Some("0"));
            Response::ok()
        }
        _ => Response::err(404, "nieznany endpoint"),
    }
}

/// Zasoby (obrazy, fonty).
fn assets_api(a: Api, m: &str, rest: &[&str]) -> Response {
    let Api { req, engine, .. } = a;
    match (m, rest) {
        // ---------- zasoby (obrazy, fonty) ----------
        ("GET", ["assets"]) => {
            let mut list: Vec<String> = std::fs::read_dir(engine.data_dir.join("assets"))
                .map(|rd| rd.flatten().filter_map(|e| e.file_name().into_string().ok()).collect())
                .unwrap_or_default();
            list.sort();
            Response::json(&Json::Arr(
                list.into_iter()
                    .map(|n| Json::obj().with("name", n.as_str()).with("url", format!("/assets/{n}")))
                    .collect(),
            ))
        }
        ("POST", ["assets"]) => {
            let Some(name) = req.q("name").and_then(safe_name) else {
                return Response::err(400, "podaj ?name=plik.png");
            };
            if req.body.is_empty() {
                return Response::err(400, "pusty plik");
            }
            let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
            if !name.contains('.') || !ASSET_EXT.contains(&ext.as_str()) {
                return Response::err(400, &format!("niedozwolony typ pliku (dozwolone: {})", ASSET_EXT.join(", ")));
            }
            if assets_size(&engine.data_dir.join("assets")) + req.body.len() as u64 > MAX_ASSETS_TOTAL {
                return Response::err(413, "przekroczony limit rozmiaru katalogu assets");
            }
            let p: &Path = &engine.data_dir.join("assets").join(&name);
            match std::fs::write(p, &req.body) {
                Ok(_) => Response::json(&Json::obj().with("name", name.as_str()).with("url", format!("/assets/{name}"))),
                Err(e) => Response::err(500, &e.to_string()),
            }
        }
        ("DELETE", ["assets", name]) => {
            if let Some(n) = safe_name(name) {
                std::fs::remove_file(engine.data_dir.join("assets").join(n)).ok();
            }
            Response::ok()
        }
        _ => Response::err(404, "nieznany endpoint"),
    }
}

/// Diagnostyka, blokady edycji, presety animacji, Word Replace, fonty, log audytowy.
fn diag_api(a: Api, m: &str, rest: &[&str]) -> Response {
    let Api { req, engine, cfg, body, .. } = a;
    match (m, rest) {
        // ---------- diagnostyka ----------
        ("POST", ["report"]) => {
            engine.report_error(body.str("scene").unwrap_or("?"), body.str("layer").unwrap_or("?"), body.str("message").unwrap_or("?"));
            Response::ok()
        }
        ("GET", ["errors"]) => Response::json(&engine.errors()),
        ("DELETE", ["errors"]) => {
            engine.clear_errors();
            Response::ok()
        }
        ("GET", ["health"]) => Response::json(&health(engine, cfg)),
        ("GET", ["locks"]) => Response::json(&engine.locks_json()),
        ("POST" | "PUT", ["lock", kind, id]) => res(engine.lock(kind, id, req.q("who").unwrap_or("")), |j| Response::json(&j)),
        ("DELETE" | "POST", ["unlock", kind, id]) => Response::json(&engine.unlock(kind, id, req.q("who").unwrap_or(""), req.q("force") == Some("1"))),
        ("GET", ["animpresets"]) => Response::json(&engine.anim_presets()),
        ("PUT", ["animpresets"]) => res(engine.put_anim_presets(body), |j| Response::json(&j)),
        ("GET", ["wordreplace"]) => Response::json(&engine.word_replace()),
        ("PUT", ["wordreplace"]) => res(engine.put_word_replace(body), |j| Response::json(&j)),
        ("GET", ["fonts"]) => Response::json(&Json::Arr(fonts::families().into_iter().map(Json::from).collect())),
        ("GET", ["audit"]) => Response::json(&Json::Arr(engine.audit.tail(req.q("n").and_then(|n| n.parse().ok()).unwrap_or(100).min(1000)).into_iter().map(Json::from).collect())),
        _ => Response::err(404, "nieznany endpoint"),
    }
}

/// Anteny kanałów, biblioteka show (paczka grafiki na audycję) i kopie zapasowe.
fn library_api(a: Api, m: &str, rest: &[&str]) -> Response {
    let Api { engine, body, .. } = a;
    match (m, rest) {
        // ---------- biblioteka show (paczka grafiki na audycję) ----------
        ("POST", ["channels", ch]) => res(engine.channel_add(ch, body.str("name")), |j| Response::json(&j)),
        ("DELETE", ["channels", ch]) => res(engine.channel_delete(ch), |j| Response::json(&j)),
        ("GET", ["shows"]) => Response::json(&engine.shows()),
        ("POST", ["shows", "switch", id]) => res(engine.switch_show(id), |_| Response::json(&engine.shows())),
        ("POST", ["shows"]) => res(engine.new_show(body.str("name").unwrap_or(""), matches!(body.get("copy"), Some(Json::Bool(true)))), |j| Response::json(&j)),
        ("DELETE", ["shows", id]) => res(engine.delete_show(id), |_| Response::json(&engine.shows())),

        // ---------- kopie zapasowe show ----------
        ("GET", ["backups"]) => Response::json(&Json::Arr(engine.backups().into_iter().map(Json::from).collect())),
        ("POST", ["backups", "restore", name]) => res(engine.restore_backup(name), |_| Response::ok()),
        _ => Response::err(404, "nieznany endpoint"),
    }
}

fn assets_size(dir: &Path) -> u64 {
    std::fs::read_dir(dir).map(|rd| rd.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()).unwrap_or(0)
}
