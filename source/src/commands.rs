//! Komendy tekstowe (w stylu Viz Engine :6100): port TCP, OSC (UDP), makra i komendy zbiorcze.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use crate::config::Config;
use crate::engine::Engine;
use crate::json::Json;
use crate::{osc, ticker, ConnSlot};

/// Wykonuje `f` dla każdego klucza z listy rozdzielonej przecinkami (komendy zbiorcze: `TAKE 1001,1101`).
/// Wszystkie elementy są wykonywane; jeśli któryś zawiedzie, zwracany jest zbiorczy błąd.
pub(crate) fn each(keys: &str, f: impl Fn(&str) -> Result<Json, String>) -> Result<Json, String> {
    if !keys.contains(',') {
        return f(keys);
    }
    let mut results = vec![];
    let mut errs = vec![];
    for k in keys.split(',').map(str::trim).filter(|k| !k.is_empty()) {
        match f(k) {
            Ok(j) => results.push(j),
            Err(e) => errs.push(format!("{k}: {e}")),
        }
    }
    if errs.is_empty() {
        Ok(Json::obj().with("ok", true).with("results", Json::Arr(results)))
    } else {
        Err(errs.join("; "))
    }
}

/// Uruchamia makro w osobnym wątku; nowe makro (lub `macro/stop`) przerywa poprzednie.
pub(crate) fn start_macro(engine: &Arc<Engine>, ch: &str, key: &str) -> Result<(), String> {
    let m = engine.find_macro(key).ok_or(format!("brak makra '{key}'"))?;
    let steps: Vec<(String, f64)> = m.arr("steps").iter().map(|s| (s.str("cmd").unwrap_or("").to_string(), s.num("delay").unwrap_or(0.0))).collect();
    if steps.is_empty() {
        return Err("makro nie ma kroków".into());
    }
    let gen = engine.macro_stop(ch);
    let e = Arc::clone(engine);
    let ch = ch.to_string();
    std::thread::spawn(move || {
        for (cmd, delay) in steps {
            let until = std::time::Instant::now() + Duration::from_secs_f64(delay.clamp(0.0, 3600.0));
            while std::time::Instant::now() < until {
                if e.macro_gen(&ch) != gen {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            if e.macro_gen(&ch) != gen {
                return;
            }
            // krok makra działa na antenie, na której makro uruchomiono
            let line = if ch.is_empty() || cmd.trim_start().starts_with('@') { cmd.clone() } else { format!("@{ch} {cmd}") };
            let out = exec_command(&e, &line);
            if out.starts_with("ERR") {
                eprintln!("[vizrs] makro: {cmd} → {out}");
            }
        }
    });
    Ok(())
}

pub(crate) fn osc_server(engine: Arc<Engine>, cfg: Arc<Config>) {
    let Ok(sock) = std::net::UdpSocket::bind((cfg.bind.as_str(), cfg.osc_port)) else {
        eprintln!("[vizrs] nie mogę otworzyć portu OSC {}", cfg.osc_port);
        return;
    };
    let mut buf = [0u8; 8192];
    while let Ok((n, from)) = sock.recv_from(&mut buf) {
        let Some((addr, args)) = osc::parse(&buf[..n]) else { continue };
        match osc::to_command(&addr, &args, &cfg.token) {
            Ok(line) => {
                engine.audit.log(&format!("osc:{}", from.ip()), &line, None);
                let out = exec_command(&engine, &line);
                if out.starts_with("ERR") {
                    eprintln!("[vizrs] OSC {addr}: {out}");
                }
            }
            Err(e) => eprintln!("[vizrs] OSC odrzucony ({}): {e}", from.ip()),
        }
    }
}

pub(crate) fn tcp_server(engine: Arc<Engine>, port: u16, cfg: Arc<Config>) {
    let Ok(l) = TcpListener::bind((cfg.bind.as_str(), port)) else {
        eprintln!("[vizrs] nie mogę otworzyć portu TCP {port}");
        return;
    };
    for mut s in l.incoming().flatten() {
        let Some(slot) = ConnSlot::acquire() else {
            writeln!(s, "ERR za dużo połączeń").ok();
            continue;
        };
        let e = Arc::clone(&engine);
        let tok = cfg.token.clone();
        std::thread::spawn(move || {
            let _slot = slot;
            tcp_client(s, e, tok)
        });
    }
}

fn parse_kv(s: &str) -> Json {
    // "name=Jan Kowalski|title=Reporter"  albo  JSON {"name":"..."}
    let s = s.trim();
    if s.starts_with('{') {
        return Json::parse(s).unwrap_or(Json::obj());
    }
    let mut v = Json::obj();
    for part in s.split('|') {
        if let Some((k, val)) = part.split_once('=') {
            v.set(k.trim(), val.trim().into());
        }
    }
    v
}

/// `@<kanał> <komenda>`: komenda na antenie kanału zamiast głównej.
fn exec_channel(engine: &Arc<Engine>, line: &str) -> Result<Json, String> {
    let (ch, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let (cmd, rest) = rest.trim().split_once(char::is_whitespace).unwrap_or((rest.trim(), ""));
    let rest = rest.trim();
    if ch.is_empty() {
        return Err("brak numeru kanału".into());
    }
    let (arg, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    match cmd.to_ascii_uppercase().as_str() {
        "TAKE" => {
            let ov = parse_kv(tail);
            let ov = if matches!(&ov, Json::Obj(v) if v.is_empty()) { None } else { Some(ov) };
            each(arg, |k| engine.channel_take(ch, k, ov.as_ref()))
        }
        "CONTINUE" | "CONT" => each(if arg.is_empty() { "main" } else { arg }, |k| engine.channel_cont(ch, k)),
        "OUT" => each(if arg.is_empty() { "main" } else { arg }, |k| engine.channel_out(ch, k)),
        "CLEAR" => engine.channel_clear(ch),
        "ARM" => {
            let ov = parse_kv(tail);
            let ov = if matches!(&ov, Json::Obj(v) if v.is_empty()) { None } else { Some(ov) };
            each(arg, |k| engine.arm(k, ov.as_ref()))
        }
        "NEXT" => engine.rundown_take_on(ch, arg, None),
        "RUNDOWN" => match arg.to_ascii_uppercase().as_str() {
            "TAKE" => {
                let (rid, item) = tail.trim().split_once(char::is_whitespace).unwrap_or((tail.trim(), ""));
                engine.rundown_take_on(ch, rid, Some(item.trim()).filter(|i| !i.is_empty()))
            }
            "RESET" => engine.rundown_reset_on(ch, tail.trim()).map(|_| Json::obj()),
            _ => Err("RUNDOWN TAKE <rundown> [pozycja] | RUNDOWN RESET <rundown>".into()),
        },
        "MACRO" => {
            if tail.trim().is_empty() && arg.eq_ignore_ascii_case("STOP") {
                engine.macro_stop(ch);
                Ok(Json::obj())
            } else {
                start_macro(engine, ch, rest).map(|_| Json::obj())
            }
        }
        "CAROUSEL" => match arg.to_ascii_uppercase().as_str() {
            "START" => engine.carousel_start_on(ch, tail.trim()).map(|_| Json::obj()),
            "STOP" => {
                engine.carousel_stop_on(ch, true);
                Ok(Json::obj())
            }
            _ => Err("CAROUSEL START <playlista> | CAROUSEL STOP".into()),
        },
        "STATE" => engine.channel_state(ch),
        // dane strony są wspólne: UPDATE i LIVE sięgają też anten kanałów bez przedrostka
        "UPDATE" => engine.update(arg, &parse_kv(tail), true),
        "LIVE" => engine.update(arg, &parse_kv(tail), false),
        "TICKER" => ticker::parse_op(tail).and_then(|op| engine.ticker(arg, op)),
        other => Err(format!("komenda {other} nie działa na kanale")),
    }
}

pub fn exec_command(engine: &Arc<Engine>, line: &str) -> String {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix('@') {
        return match exec_channel(engine, rest) {
            Ok(j) => format!("OK {}", j.dump()),
            Err(e) => format!("ERR {e}"),
        };
    }
    let (cmd, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = rest.trim();
    let (arg, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let r: Result<Json, String> = match cmd.to_ascii_uppercase().as_str() {
        "TAKE" | "ARM" => {
            let ov = parse_kv(tail);
            let ov = if matches!(&ov, Json::Obj(v) if v.is_empty()) { None } else { Some(ov) };
            let arm = cmd.eq_ignore_ascii_case("ARM");
            each(arg, |k| if arm { engine.arm(k, ov.as_ref()) } else { engine.take(k, ov.as_ref()) })
        }
        "CONTINUE" | "CONT" => each(if arg.is_empty() { "main" } else { arg }, |k| engine.cont(k)),
        "OUT" => each(if arg.is_empty() { "main" } else { arg }, |k| engine.out(k)),
        "NEXT" => engine.rundown_take(arg, None),
        "RUNDOWN" => match arg.to_ascii_uppercase().as_str() {
            "TAKE" => {
                let (rid, item) = tail.trim().split_once(char::is_whitespace).unwrap_or((tail.trim(), ""));
                engine.rundown_take(rid, Some(item.trim()).filter(|i| !i.is_empty()))
            }
            "RESET" => engine.rundown_reset(tail.trim()).map(|_| Json::obj()),
            _ => Err("RUNDOWN TAKE <rundown> [pozycja] | RUNDOWN RESET <rundown>".into()),
        },
        "MACRO" => {
            if tail.trim().is_empty() && arg.eq_ignore_ascii_case("STOP") {
                engine.macro_stop("");
                Ok(Json::obj())
            } else {
                start_macro(engine, "", rest).map(|_| Json::obj())
            }
        }
        "CLEAR" => {
            engine.clear();
            Ok(Json::obj())
        }
        "UPDATE" => engine.update(arg, &parse_kv(tail), true),
        "LIVE" => engine.update(arg, &parse_kv(tail), false),
        "TICKER" => ticker::parse_op(tail).and_then(|op| engine.ticker(arg, op)),
        "CAROUSEL" => match arg.to_ascii_uppercase().as_str() {
            "START" => engine.carousel_start(tail.trim()).map(|_| Json::obj()),
            "STOP" => {
                engine.carousel_stop(true);
                Ok(Json::obj())
            }
            _ => Err("CAROUSEL START <playlista> | CAROUSEL STOP".into()),
        },
        "STATE" => Ok(engine.state()),
        "PING" => Ok(Json::obj().with("pong", true)),
        "HELP" | "" => Err("komendy: TAKE <strona[,strona…]> [k=v|k2=v2], ARM <strona>, NEXT <rundown>, RUNDOWN TAKE|RESET <rundown> [pozycja], MACRO <nazwa>|STOP, CONTINUE <warstwa|strona>, OUT <warstwa|strona>, CLEAR, UPDATE <strona> k=v|..., LIVE <strona> k=v|..., TICKER <strona[:pole]> LIST|ADD <KAT|treść>|SET <n> <KAT|treść>|DEL <n>|CLEAR, CAROUSEL START <playlista>, CAROUSEL STOP, STATE".into()),
        other => Err(format!("nieznana komenda {other}")),
    };
    match r {
        Ok(j) => format!("OK {}", j.dump()),
        Err(e) => format!("ERR {e}"),
    }
}

fn tcp_client(stream: TcpStream, engine: Arc<Engine>, token: Option<String>) {
    use std::io::Read;
    stream.set_read_timeout(Some(Duration::from_secs(3600))).ok();
    let peer = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    let Ok(mut w) = stream.try_clone() else { return };
    let mut r = BufReader::new(stream);
    let mut authed = token.is_none();
    loop {
        let mut line = String::new();
        // linia ograniczona do 64 KB (ochrona przed nieskończoną linią bez \n)
        match r.by_ref().take(64 * 1024).read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(n) if n == 64 * 1024 && !line.ends_with('\n') => {
                writeln!(w, "ERR za długa linia").ok();
                break;
            }
            Ok(_) => {}
        }
        if line.trim().is_empty() {
            continue;
        }
        let out = if !authed {
            match (line.trim().strip_prefix("AUTH "), &token) {
                (Some(t), Some(want)) if t.trim() == want => {
                    authed = true;
                    "OK {}".to_string()
                }
                _ => "ERR wymagane: AUTH <token>".to_string(),
            }
        } else {
            if !matches!(line.trim().to_ascii_uppercase().as_str(), "STATE" | "PING" | "HELP") {
                engine.audit.log(&format!("tcp:{peer}"), line.trim(), None);
            }
            exec_command(&engine, &line)
        };
        if writeln!(w, "{out}").is_err() {
            break;
        }
    }
}
