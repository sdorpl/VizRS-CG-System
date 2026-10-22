//! Okno „Połącz z serwerem”: adres i token serwera VizRS (np. na VPS), lista zapisanych połączeń.
//!
//! Strona okna jest wbudowana w powłokę (nie pochodzi z serwera), więc tylko ona może wysłać przez most
//! komunikat `connect:` – strony Artist/Trio serwera nie mogą przełączyć okien na inny adres.
//! Lista leży w `connections.json` w katalogu konfiguracji użytkownika (Linux `~/.config/vizrs`,
//! Windows `%APPDATA%\VizRS`, macOS `~/Library/Application Support/VizRS`); token tylko na życzenie.

use std::path::PathBuf;

use serde_json::{json, Value};

/// Najwięcej zapisanych serwerów.
const MAX_SAVED: usize = 20;

/// Zapisane połączenia.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Saved {
    /// (adres, token, jeśli zapamiętany)
    pub servers: Vec<(String, Option<String>)>,
    /// adres, z którym okno łączy się przy uruchomieniu (bez `--connect`)
    pub auto: Option<String>,
}

/// Plik `connections.json`.
pub fn config_path() -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(p) = env("VIZRS_CONFIG") {
        return Some(p.join("connections.json"));
    }
    let dir = if cfg!(windows) {
        env("APPDATA")?.join("VizRS")
    } else if cfg!(target_os = "macos") {
        env("HOME")?.join("Library/Application Support/VizRS")
    } else {
        env("XDG_CONFIG_HOME").or_else(|| env("HOME").map(|h| h.join(".config")))?.join("vizrs")
    };
    Some(dir.join("connections.json"))
}

impl Saved {
    pub fn parse(text: &str) -> Saved {
        let Ok(v) = serde_json::from_str::<Value>(text) else { return Saved::default() };
        let servers = v["servers"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|s| {
                        let url = normalize_url(s["url"].as_str()?).ok()?;
                        let token = s["token"].as_str().filter(|t| valid_token(t)).map(str::to_string);
                        Some((url, token))
                    })
                    .take(MAX_SAVED)
                    .collect()
            })
            .unwrap_or_default();
        let auto = v["auto"].as_str().and_then(|u| normalize_url(u).ok());
        Saved { servers, auto }
    }

    pub fn to_json(&self) -> String {
        let servers: Vec<Value> = self.servers.iter().map(|(u, t)| json!({ "url": u, "token": t })).collect();
        serde_json::to_string_pretty(&json!({ "servers": servers, "auto": self.auto })).unwrap_or_default()
    }

    pub fn load() -> Saved {
        config_path().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| Saved::parse(&t)).unwrap_or_default()
    }

    /// Zapis z prawami tylko dla właściciela (plik może zawierać token).
    pub fn save(&self) -> Result<(), String> {
        let p = config_path().ok_or("brak katalogu konfiguracji użytkownika")?;
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        let mut opt = std::fs::OpenOptions::new();
        opt.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opt.mode(0o600);
        }
        use std::io::Write;
        opt.open(&p).and_then(|mut f| f.write_all(self.to_json().as_bytes())).map_err(|e| format!("{}: {e}", p.display()))
    }

    /// Serwer na początek listy (ostatnio używany).
    pub fn remember(&mut self, url: &str, token: Option<&str>, keep_token: bool, auto: bool) {
        self.servers.retain(|(u, _)| u != url);
        self.servers.insert(0, (url.to_string(), token.filter(|_| keep_token).map(str::to_string)));
        self.servers.truncate(MAX_SAVED);
        if auto {
            self.auto = Some(url.to_string());
        } else if self.auto.as_deref() == Some(url) {
            self.auto = None;
        }
    }

    pub fn forget(&mut self, url: &str) {
        self.servers.retain(|(u, _)| u != url);
        if self.auto.as_deref() == Some(url) {
            self.auto = None;
        }
    }

    pub fn token_for(&self, url: &str) -> Option<String> {
        self.servers.iter().find(|(u, _)| u == url).and_then(|(_, t)| t.clone())
    }
}

/// Adres serwera: `http(s)://host[:port][/ścieżka]`, bez końcowego `/`. Sam host → `https://`.
pub fn normalize_url(s: &str) -> Result<String, String> {
    let s = s.trim().trim_end_matches('/');
    if s.is_empty() {
        return Err("podaj adres serwera".into());
    }
    let s = if s.contains("://") { s.to_string() } else { format!("https://{s}") };
    let rest = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://")).ok_or(format!("adres musi zaczynać się od http:// albo https:// („{s}”)"))?;
    let host = rest.split('/').next().unwrap_or("");
    let ok_char = |c: char| c.is_ascii_alphanumeric() || "-._:[]/~%".contains(c);
    if host.is_empty() || !rest.chars().all(ok_char) || s.len() > 300 {
        return Err(format!("niepoprawny adres serwera „{s}”"));
    }
    Ok(s)
}

/// Znaki tokenu takie jak w serwerze (`--token`).
pub fn valid_token(t: &str) -> bool {
    !t.is_empty() && t.len() <= 256 && t.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}

/// Dla `http://`: `host:port` i ścieżka `/api/state` (token operatora – token wyjścia dostaje 403; do sprawdzenia przed przełączeniem). `https://` – None (bez TLS w powłoce).
pub fn check_target(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("http://")?;
    let (host, prefix) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let addr = if host.rsplit_once(':').is_some_and(|(_, p)| p.parse::<u16>().is_ok()) && !host.ends_with(']') { host.to_string() } else { format!("{host}:80") };
    Some((addr, format!("{prefix}/api/state")))
}

/// Żądanie połączenia z serwerem.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub url: String,
    pub token: Option<String>,
    /// zapamiętaj token w `connections.json`
    pub keep_token: bool,
    /// łącz przy uruchomieniu
    pub auto: bool,
}

/// Komunikat z okna połączenia.
#[derive(Debug, PartialEq)]
pub enum Msg {
    Connect(Request),
    Local,
    Forget(String),
    Cancel,
}

pub fn parse_msg(body: &str) -> Result<Msg, String> {
    let v: Value = serde_json::from_str(body).map_err(|_| "zły komunikat".to_string())?;
    match v["action"].as_str() {
        Some("connect") => {
            let url = normalize_url(v["url"].as_str().unwrap_or(""))?;
            let token = v["token"].as_str().map(str::trim).filter(|t| !t.is_empty());
            if let Some(t) = token {
                if !valid_token(t) {
                    return Err("token może zawierać tylko litery A–Z a–z, cyfry i znaki - _ .".into());
                }
            }
            Ok(Msg::Connect(Request { url, token: token.map(str::to_string), keep_token: v["keepToken"].as_bool().unwrap_or(false), auto: v["auto"].as_bool().unwrap_or(false) }))
        }
        Some("local") => Ok(Msg::Local),
        Some("forget") => Ok(Msg::Forget(v["url"].as_str().unwrap_or("").to_string())),
        Some("cancel") => Ok(Msg::Cancel),
        _ => Err("zły komunikat".into()),
    }
}

/// Dane dla strony okna (`window.__vizrsConn`).
pub fn page_data(saved: &Saved, current: &str, current_token: Option<&str>, local: Option<&str>) -> String {
    let servers: Vec<Value> = saved.servers.iter().map(|(u, t)| json!({ "url": u, "hasToken": t.is_some() })).collect();
    let remote = local != Some(current);
    let data = json!({
        "current": current,
        "remote": remote,
        "token": if remote { current_token } else { None },
        "local": local,
        "auto": saved.auto,
        "servers": servers,
    });
    format!("window.__vizrsConn = {data};")
}

pub const PAGE: &str = r#"<!doctype html>
<html lang="pl"><head><meta charset="utf-8"><title>Połącz z serwerem</title>
<style>
:root { color-scheme: dark; --bg:#15181d; --panel:#1e232b; --line:#2e3540; --text:#e6e9ee; --dim:#8d97a5; --acc:#3d8bfd; --err:#ff6b6b; }
* { box-sizing: border-box; }
body { margin:0; background:var(--bg); color:var(--text); font:14px system-ui, sans-serif; padding:20px; }
h1 { font-size:18px; margin:0 0 4px; }
.now { color:var(--dim); margin-bottom:16px; font-size:13px; }
label { display:block; margin:12px 0 4px; color:var(--dim); font-size:12px; text-transform:uppercase; letter-spacing:.04em; }
input[type=text], input[type=password] { width:100%; padding:8px 10px; background:var(--panel); border:1px solid var(--line); color:var(--text); border-radius:4px; font:inherit; }
input:focus { outline:none; border-color:var(--acc); }
.row { display:flex; gap:8px; align-items:center; }
.check { display:flex; gap:8px; align-items:center; margin:10px 0 0; color:var(--text); text-transform:none; letter-spacing:0; font-size:13px; }
button { padding:8px 14px; background:var(--panel); border:1px solid var(--line); color:var(--text); border-radius:4px; font:inherit; cursor:pointer; }
button:hover { border-color:var(--acc); }
button.primary { background:var(--acc); border-color:var(--acc); color:#fff; }
button:disabled { opacity:.5; cursor:default; }
.actions { display:flex; gap:8px; margin-top:18px; }
.actions .sp { flex:1; }
#err { color:var(--err); min-height:1.3em; margin-top:12px; white-space:pre-wrap; }
#list { margin-top:6px; border:1px solid var(--line); border-radius:4px; max-height:150px; overflow:auto; }
#list:empty::after { content:'brak zapisanych serwerów'; display:block; padding:8px 10px; color:var(--dim); }
.srv { display:flex; align-items:center; padding:6px 10px; cursor:pointer; border-bottom:1px solid var(--line); }
.srv:last-child { border-bottom:0; }
.srv:hover { background:var(--panel); }
.srv .u { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.srv .t { color:var(--dim); font-size:12px; margin:0 8px; }
.srv .x { padding:0 6px; border:0; background:none; color:var(--dim); }
.hint { color:var(--dim); font-size:12px; margin-top:6px; }
</style></head><body>
<h1>Połącz z serwerem VizRS</h1>
<div class="now" id="now"></div>
<label for="url">Adres serwera</label>
<input type="text" id="url" placeholder="https://cg.example.pl" autocomplete="off" spellcheck="false">
<label for="token">Token operatora</label>
<div class="row"><input type="password" id="token" autocomplete="off" spellcheck="false"><button id="show" type="button" title="Pokaż token">👁</button></div>
<div class="hint">Token z <code>--token</code> / <code>VIZRS_TOKEN</code> serwera (nie token wyjścia OBS).</div>
<label class="check"><input type="checkbox" id="keep"> zapamiętaj token na tym komputerze</label>
<label class="check"><input type="checkbox" id="auto"> łącz z tym serwerem przy uruchomieniu</label>
<label>Zapisane serwery</label>
<div id="list"></div>
<div id="err"></div>
<div class="actions">
  <button id="local" type="button">Serwer lokalny</button>
  <span class="sp"></span>
  <button id="cancel" type="button">Anuluj</button>
  <button id="go" class="primary" type="button">Połącz</button>
</div>
<script>
const C = window.__vizrsConn || { servers: [] };
const $ = (id) => document.getElementById(id);
const send = (m) => window.ipc.postMessage(JSON.stringify(m));
$('now').textContent = 'Teraz: ' + (C.remote ? C.current : 'serwer lokalny (' + C.current + ')');
if (!C.local) $('local').hidden = true;
if (C.remote) { $('url').value = C.current; $('token').value = C.token || ''; $('auto').checked = C.auto === C.current; }
function fill(s) {
  $('url').value = s.url; $('token').value = ''; $('keep').checked = s.hasToken;
  $('token').placeholder = s.hasToken ? 'zapamiętany – zostaw puste' : '';
  $('auto').checked = C.auto === s.url; $('err').textContent = '';
}
for (const s of C.servers) {
  const d = document.createElement('div'); d.className = 'srv';
  const u = document.createElement('span'); u.className = 'u'; u.textContent = s.url;
  const t = document.createElement('span'); t.className = 't'; t.textContent = s.hasToken ? 'token zapisany' : '';
  const x = document.createElement('button'); x.className = 'x'; x.textContent = '×'; x.title = 'Usuń z listy';
  x.onclick = (e) => { e.stopPropagation(); send({ action: 'forget', url: s.url }); d.remove(); };
  d.onclick = () => fill(s); d.ondblclick = () => { fill(s); go(); };
  d.append(u, t, x); $('list').append(d);
}
function go() {
  $('err').textContent = ''; $('go').disabled = true; $('go').textContent = 'Łączenie…';
  send({ action: 'connect', url: $('url').value, token: $('token').value, keepToken: $('keep').checked, auto: $('auto').checked });
}
window.__vizrsConnError = (msg) => { $('err').textContent = msg; $('go').disabled = false; $('go').textContent = 'Połącz'; };
$('go').onclick = go;
$('local').onclick = () => send({ action: 'local' });
$('cancel').onclick = () => send({ action: 'cancel' });
$('show').onclick = () => { $('token').type = $('token').type === 'password' ? 'text' : 'password'; };
document.addEventListener('keydown', (e) => { if (e.key === 'Enter') go(); if (e.key === 'Escape') send({ action: 'cancel' }); });
($('url').value ? $('token') : $('url')).focus();
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(normalize_url(" https://cg.example.pl/ ").unwrap(), "https://cg.example.pl");
        assert_eq!(normalize_url("cg.example.pl").unwrap(), "https://cg.example.pl");
        assert_eq!(normalize_url("http://10.0.0.5:8088").unwrap(), "http://10.0.0.5:8088");
        assert!(normalize_url("").is_err());
        assert!(normalize_url("ftp://x").is_err());
        assert!(normalize_url("https://a b").is_err());
        assert!(normalize_url("https://x/'><script>").is_err());
    }

    #[test]
    fn health() {
        assert_eq!(check_target("http://10.0.0.5:8088"), Some(("10.0.0.5:8088".into(), "/api/state".into())));
        assert_eq!(check_target("http://cg.lan/vizrs"), Some(("cg.lan:80".into(), "/vizrs/api/state".into())));
        assert_eq!(check_target("https://cg.example.pl"), None);
    }

    #[test]
    fn messages() {
        let m = parse_msg(r#"{"action":"connect","url":"cg.pl","token":" abc ","keepToken":true}"#).unwrap();
        assert_eq!(m, Msg::Connect(Request { url: "https://cg.pl".into(), token: Some("abc".into()), keep_token: true, auto: false }));
        assert!(parse_msg(r#"{"action":"connect","url":"cg.pl","token":"a&b"}"#).is_err());
        assert_eq!(parse_msg(r#"{"action":"local"}"#).unwrap(), Msg::Local);
        assert!(parse_msg("probe:{}").is_err());
    }

    #[test]
    fn saved_roundtrip() {
        let mut s = Saved::default();
        s.remember("https://a", Some("t1"), true, true);
        s.remember("https://b", Some("t2"), false, false);
        s.remember("https://a", Some("t3"), true, false);
        assert_eq!(s.servers, vec![("https://a".into(), Some("t3".into())), ("https://b".into(), None)]);
        assert_eq!(s.auto, None);
        s.remember("https://b", None, false, true);
        let back = Saved::parse(&s.to_json());
        assert_eq!(back, s);
        s.forget("https://b");
        assert_eq!(s.auto, None);
        assert_eq!(Saved::parse("{zły"), Saved::default());
        assert_eq!(Saved::parse(r#"{"servers":[{"url":"https://x","token":"z ł y"}]}"#).servers, vec![("https://x".into(), None)]);
    }
}
