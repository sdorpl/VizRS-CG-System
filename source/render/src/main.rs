//! vizrs-render – natywny renderer grafiki VizRS.
//!
//!   vizrs-render stream --url <adres /output> --size 1920x1080 [--area x,y,w,h] [--fonts katalog]
//!   vizrs-render frame  --show show.json (--scene NAZWA | --page N) [--values JSON] [--stop I | --time S]
//!                       [--size WxH] [--fonts katalog] [--assets katalog] [--wall-ms N] -o klatka.png
//!   vizrs-render check  --show show.json [--fonts katalog] [--assets katalog]
//!   vizrs-render bench  --show show.json --page N [--seconds 10] [--fps 25] [--size WxH] [--area x,y,w,h]
//!
//! Opis: docs/DOKUMENTACJA.md §10.1.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Map, Value};
use tiny_skia::Pixmap;
use vizrs_render::font::FontDb;
use vizrs_render::host::Host;
use vizrs_render::model::{num_of, str_of, Scene};
use vizrs_render::res::Resources;
use vizrs_render::scene::{check_scene, Clock};
use vizrs_render::stream;

const USAGE: &str = "użycie: vizrs-render stream|frame|check|bench … (docs/DOKUMENTACJA.md §10.1)";

struct Args {
    cmd: String,
    opts: Vec<(String, String)>,
}

impl Args {
    fn parse() -> Result<Args, String> {
        let mut it = std::env::args().skip(1);
        let cmd = it.next().ok_or(USAGE)?;
        let mut opts = Vec::new();
        while let Some(a) = it.next() {
            let key = a.trim_start_matches('-').to_string();
            if !a.starts_with('-') {
                return Err(format!("nieznany argument {a}"));
            }
            let v = it.next().ok_or_else(|| format!("{a}: brak wartości"))?;
            opts.push((key, v));
        }
        Ok(Args { cmd, opts })
    }
    fn get(&self, k: &str) -> Option<&str> {
        self.opts.iter().rev().find(|(key, _)| key == k).map(|(_, v)| v.as_str())
    }
    fn all(&self, k: &str) -> Vec<String> {
        self.opts.iter().filter(|(key, _)| key == k).map(|(_, v)| v.clone()).collect()
    }
}

fn size_of(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s.split_once(['x', 'X']).ok_or_else(|| format!("rozmiar {s}: oczekiwano WxH"))?;
    let (w, h): (u32, u32) = (w.trim().parse().map_err(|_| format!("rozmiar {s}"))?, h.trim().parse().map_err(|_| format!("rozmiar {s}"))?);
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return Err(format!("rozmiar {s} poza zakresem"));
    }
    Ok((w, h))
}

fn area_of(s: &str) -> Result<(i32, i32, u32, u32), String> {
    let p: Vec<i64> =
        s.split(',').map(|v| v.trim().parse::<i64>().map_err(|_| format!("obszar {s}: oczekiwano x,y,w,h"))).collect::<Result<_, _>>()?;
    if p.len() != 4 || p[2] <= 0 || p[3] <= 0 {
        return Err(format!("obszar {s}: oczekiwano x,y,w,h"));
    }
    Ok((p[0] as i32, p[1] as i32, p[2] as u32, p[3] as u32))
}

/// Katalogi fontów: z `--fonts`, z VIZRS_FONTS, inaczej `web/fonts` obok (repozytorium VizRS).
fn font_dirs(a: &Args) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = a.all("fonts").into_iter().map(PathBuf::from).collect();
    if let Ok(v) = std::env::var("VIZRS_FONTS") {
        dirs.extend(v.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    }
    if dirs.is_empty() {
        for d in ["web/fonts", "source/web/fonts", "../web/fonts"] {
            if std::path::Path::new(d).is_dir() {
                dirs.push(d.into());
                break;
            }
        }
    }
    dirs
}

fn offline_resources(a: &Args) -> Arc<Resources> {
    let mut db = FontDb::new();
    for d in font_dirs(a) {
        db.add_dir(&d);
    }
    vizrs_render::add_system_fonts(&mut db);
    let fetch: Option<Arc<vizrs_render::res::Fetch>> = a.get("assets").map(|dir| {
        let dir = PathBuf::from(dir);
        Arc::new(move |path: &str| {
            let name = path.strip_prefix("/assets/").ok_or("tylko /assets")?;
            let name = percent_decode(name);
            if name.contains('/') || name.contains("..") {
                return Err("zła nazwa".to_string());
            }
            std::fs::read(dir.join(&name)).map_err(|e| format!("{name}: {e}"))
        }) as Arc<vizrs_render::res::Fetch>
    });
    Resources::new(db, fetch, true)
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 3 <= b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_show(a: &Args) -> Result<Value, String> {
    let path = a.get("show").ok_or("brak --show")?;
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

fn scenes(show: &Value) -> Vec<Value> {
    show.get("scenes").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn check(a: &Args) -> Result<(), String> {
    let show = read_show(a)?;
    let res = offline_resources(a);
    let db = res.fonts.read().unwrap_or_else(|e| e.into_inner());
    let (mut ok, mut all) = (0, 0);
    for s in scenes(&show) {
        let scene = Scene::from_json(&s);
        let issues = check_scene(&scene, &db);
        all += 1;
        if issues.is_empty() {
            ok += 1;
            println!("OK      {}", scene.label());
        } else {
            println!("CHROMIUM {}", scene.label());
            for i in issues {
                println!("        - {i}");
            }
        }
    }
    println!("{ok}/{all} scen natywnie");
    Ok(())
}

/// Scena i wartości strony (`--page N`) albo scena z nazwy (`--scene`), z `--values` i `--hide`.
fn page_scene(a: &Args) -> Result<(Value, Map<String, Value>), String> {
    let show = read_show(a)?;
    let list = scenes(&show);
    let (mut scene_v, mut values) = if let Some(n) = a.get("page") {
        let pages = show.get("pages").and_then(Value::as_array).cloned().unwrap_or_default();
        let page = pages.iter().find(|p| p.get("number").map(str_of).as_deref() == Some(n)).ok_or_else(|| format!("brak strony {n}"))?;
        let sid = page.get("scene").map(str_of).unwrap_or_default();
        let scene = list
            .iter()
            .find(|s| s.get("id").map(str_of) == Some(sid.clone()) || s.get("name").map(str_of) == Some(sid.clone()))
            .ok_or_else(|| format!("brak sceny {sid}"))?;
        (scene.clone(), page.get("values").and_then(Value::as_object).cloned().unwrap_or_default())
    } else {
        let name = a.get("scene").ok_or("brak --scene albo --page")?;
        let scene = list
            .iter()
            .find(|s| s.get("name").map(str_of).as_deref() == Some(name) || s.get("id").map(str_of).as_deref() == Some(name))
            .ok_or_else(|| format!("brak sceny {name}"))?;
        (scene.clone(), Map::new())
    };
    if let Some(v) = a.get("values") {
        let extra: Value = serde_json::from_str(v).map_err(|e| format!("--values: {e}"))?;
        if let Some(o) = extra.as_object() {
            for (k, v) in o {
                values.insert(k.clone(), v.clone());
            }
        }
    }
    if let Some(hide) = a.get("hide") {
        // ukryte typy węzłów (np. zegary i tickery przy porównaniu z Chromium)
        let kinds: Vec<&str> = hide.split(',').map(str::trim).collect();
        fn mark(nodes: &mut Value, kinds: &[&str]) {
            for n in nodes.as_array_mut().into_iter().flatten() {
                if n.get("type").and_then(Value::as_str).is_some_and(|t| kinds.contains(&t)) {
                    n["hidden"] = Value::Bool(true);
                }
                if let Some(c) = n.get_mut("children") {
                    mark(c, kinds);
                }
            }
        }
        if let Some(nodes) = scene_v.get_mut("nodes") {
            mark(nodes, &kinds);
        }
    }
    Ok((scene_v, values))
}

fn wall_now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0)
}

fn new_host(a: &Args) -> Host {
    let mut host = Host::new(None, "program");
    let stage = |k: &str, d: f64| a.get(k).and_then(|v| v.parse::<f64>().ok()).filter(|v| *v > 0.0).unwrap_or(d);
    host.stage = (stage("stage-w", 1920.0), stage("stage-h", 1080.0));
    host
}

fn layer_of(scene: &Value) -> String {
    scene.get("layer").map(str_of).filter(|l| !l.is_empty()).unwrap_or_else(|| "main".into())
}

fn frame(a: &Args) -> Result<(), String> {
    let (scene_v, values) = page_scene(a)?;
    let size = size_of(a.get("size").unwrap_or("1920x1080"))?;
    let res = offline_resources(a);
    let wall = a.get("wall-ms").and_then(|v| v.parse::<f64>().ok()).unwrap_or_else(wall_now);
    let layer = layer_of(&scene_v);
    let mut host = new_host(a);
    let mut clk = Clock { now: 0.0, wall };
    if let Some(t) = a.get("time") {
        // od TAKE przez `t` sekund w rytmie `fps` (wejście, crawl, zegary jak na antenie)
        let t: f64 = t.parse().map_err(|_| "--time: liczba sekund")?;
        let fps: f64 = a.get("fps").and_then(|v| v.parse().ok()).filter(|f: &f64| *f > 0.0).unwrap_or(25.0);
        host.handle(&serde_json::json!({"t":"take","layer":layer,"scene":scene_v,"values":values,"since":1}), clk);
        let frames = (t * fps).round() as u64;
        for k in 0..=frames {
            clk = Clock { now: k as f64 * 1000.0 / fps, wall: wall + k as f64 * 1000.0 / fps };
            host.tick(clk, &res);
        }
    } else {
        let stop = a.get("stop").map(|v| num_of(&Value::from(v))).filter(|v| v.is_finite()).unwrap_or(0.0);
        host.handle(
            &serde_json::json!({"t":"snapshot","layers":[{"layer":layer,"scene":scene_v,"values":values,"since":1,"stop":stop}]}),
            clk,
        );
        host.tick(clk, &res);
    }
    for i in host.issues() {
        eprintln!("vizrs-render: poza rendererem natywnym: {i}");
    }
    if std::env::var("VIZRS_RENDER_DEBUG").is_ok() {
        for f in res.fonts.read().unwrap_or_else(|e| e.into_inner()).describe() {
            eprintln!("font: {f}");
        }
        host.debug();
    }
    let mut pix = Pixmap::new(size.0, size.1).ok_or("zły rozmiar")?;
    host.draw(&mut pix, size, (0, 0), &res);
    let out = a.get("o").or(a.get("out")).ok_or("brak -o plik.png")?;
    let mut data = pix.data().to_vec();
    vizrs_render::raster::unpremultiply(&mut data);
    let file = std::fs::File::create(out).map_err(|e| format!("{out}: {e}"))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size.0, size.1);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut w| w.write_image_data(&data)).map_err(|e| format!("{out}: {e}"))?;
    Ok(())
}

/// Koszt rysowania: strona od TAKE przez `--seconds` w rytmie kanału, czas CPU na klatkę (etap N0 planu).
fn bench(a: &Args) -> Result<(), String> {
    let (scene_v, values) = page_scene(a)?;
    let size = size_of(a.get("size").unwrap_or("1920x1080"))?;
    let area = a.get("area").map(area_of).transpose()?.unwrap_or((0, 0, size.0, size.1));
    let fps: f64 = a.get("fps").and_then(|v| v.parse().ok()).filter(|f: &f64| *f > 0.0).unwrap_or(25.0);
    let seconds: f64 = a.get("seconds").and_then(|v| v.parse().ok()).filter(|f: &f64| *f > 0.0).unwrap_or(10.0);
    let res = offline_resources(a);
    let mut host = new_host(a);
    let wall = wall_now();
    let layer = layer_of(&scene_v);
    host.handle(&serde_json::json!({"t":"take","layer":layer,"scene":scene_v,"values":values,"since":1}), Clock { now: 0.0, wall });
    let mut pix = Pixmap::new(area.2, area.3).ok_or("zły rozmiar")?;
    let frames = (seconds * fps).round() as u64;
    let (mut drawn, mut worst) = (0u64, 0f64);
    let mut rgba = vec![0u8; pix.data().len()];
    let started = std::time::Instant::now();
    for k in 0..frames {
        let t0 = std::time::Instant::now();
        let clk = Clock { now: k as f64 * 1000.0 / fps, wall: wall + k as f64 * 1000.0 / fps };
        host.tick(clk, &res);
        if host.draw(&mut pix, size, (area.0, area.1), &res) {
            drawn += 1;
            // jak w trybie stream: kopia z rozdzieleniem alfy
            rgba.copy_from_slice(pix.data());
            vizrs_render::raster::unpremultiply(&mut rgba);
        }
        worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
    }
    let total = started.elapsed().as_secs_f64();
    let per = total * 1000.0 / frames.max(1) as f64;
    println!(
        "{frames} klatek ({drawn} narysowanych) {}x{} w {:.2} s: średnio {per:.2} ms, najdłużej {worst:.2} ms, {:.1}% jednego rdzenia przy {fps} kl./s",
        area.2,
        area.3,
        total,
        per * fps / 10.0
    );
    for i in host.issues() {
        println!("poza rendererem natywnym: {i}");
    }
    Ok(())
}

fn main() {
    let args = match Args::parse() {
        Ok(a) => a,
        Err(e) => {
            if std::env::args().any(|a| a == "--version" || a == "-V") {
                println!("vizrs-render {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            eprintln!("vizrs-render: {e}");
            std::process::exit(2);
        }
    };
    let r = match args.cmd.as_str() {
        "stream" => (|| {
            let url = args.get("url").ok_or("brak --url")?.to_string();
            let size = size_of(args.get("size").unwrap_or("1920x1080"))?;
            let area = args.get("area").map(area_of).transpose()?;
            // fonty z serwera (`/fonts/fonts.css`) – katalogi tylko jawnie (`--fonts`, VIZRS_FONTS)
            let mut dirs: Vec<PathBuf> = args.all("fonts").into_iter().map(PathBuf::from).collect();
            if let Ok(v) = std::env::var("VIZRS_FONTS") {
                dirs.extend(v.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
            }
            stream::run(stream::Options { url, size, area, font_dirs: dirs })
        })(),
        "frame" => frame(&args),
        "bench" => bench(&args),
        "check" => check(&args),
        "--version" | "-V" | "version" => {
            println!("vizrs-render {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    };
    if let Err(e) = r {
        eprintln!("vizrs-render: {e}");
        std::process::exit(1);
    }
}
