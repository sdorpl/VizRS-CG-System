//! Linux: ikona okna na Waylandzie. Kompozytor szuka ikony we wpisie `vizrs-shell.desktop`, więc uruchomienie
//! bez instalacji zakłada ukryty wpis użytkownika. Istniejącego wpisu (pakiet, skrypt) nie rusza.

#[cfg(target_os = "linux")]
pub fn ensure_entry(icon_png: &[u8]) {
    use std::path::PathBuf;
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let Some(share) = env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share"))) else { return };
    let entry = share.join("applications/vizrs-shell.desktop");
    let system = env("XDG_DATA_DIRS").map(|d| d.to_string_lossy().into_owned()).unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    if entry.exists() || system.split(':').any(|d| std::path::Path::new(d).join("applications/vizrs-shell.desktop").exists()) {
        return;
    }
    let Ok(exe) = std::env::current_exe() else { return };
    let exe = exe.to_string_lossy().into_owned();
    if exe.contains(['"', '`', '$', '\\', '%', '\n']) {
        return;
    }
    let icon = share.join("icons/hicolor/256x256/apps/vizrs.png");
    if let Some(d) = icon.parent() {
        std::fs::create_dir_all(d).ok();
    }
    std::fs::write(&icon, icon_png).ok();
    let text = format!("[Desktop Entry]\nType=Application\nName=VizRS\nComment=Okno natywne VizRS\nExec=\"{exe}\"\nIcon=vizrs\nTerminal=false\nNoDisplay=true\nStartupWMClass=vizrs-shell\n");
    crate::paths::write_atomic(&entry, &text).ok();
}

#[cfg(not(target_os = "linux"))]
pub fn ensure_entry(_icon_png: &[u8]) {}
