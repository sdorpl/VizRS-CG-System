//! `vizrs-app` – uruchamianie okien VizRS z ikony/skrótu bez czarnego okna konsoli (Windows).
//!
//! Uruchamia leżący obok `vizrs` (`vizrs.exe`) z tymi samymi argumentami (domyślnie `trio`),
//! bez konsoli, i czeka na jego zakończenie. Gdy `vizrs` zakończy się błędem (np. zajęty port),
//! pokazuje komunikat w okienku systemowym. Na Linuksie i macOS działa tak samo, ale konsola nie
//! przeszkadza, więc skróty mogą wołać `vizrs` bezpośrednio.

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::process::{Command, Stdio};

fn main() {
    let exe = std::env::current_exe().ok();
    let name = if cfg!(windows) { "vizrs.exe" } else { "vizrs" };
    let Some(vizrs) = exe.as_deref().and_then(|e| e.parent()).map(|d| d.join(name)).filter(|p| p.is_file()) else {
        show_error(&format!("Nie znaleziono {name} obok vizrs-app – oba pliki muszą leżeć w tym samym katalogu."));
        std::process::exit(1);
    };
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if !args.iter().any(|a| matches!(a.as_str(), "artist" | "trio" | "output" | "serve")) {
        args.push("trio".into());
    }
    let mut cmd = Command::new(&vizrs);
    cmd.args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    match cmd.spawn().and_then(|c| c.wait_with_output()) {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            let err = err.trim();
            show_error(if err.is_empty() { "VizRS zakończył się błędem." } else { err });
            std::process::exit(out.status.code().unwrap_or(1));
        }
        Err(e) => {
            show_error(&format!("Nie mogę uruchomić {}: {e}", vizrs.display()));
            std::process::exit(1);
        }
    }
}

#[cfg(windows)]
fn show_error(msg: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: *mut std::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }
    const MB_ICONERROR: u32 = 0x10;
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (text, caption) = (wide(msg), wide("VizRS"));
    // SAFETY: MessageBoxW z user32; oba napisy są zakończone zerem i żyją do końca wywołania.
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), MB_ICONERROR);
    }
}

#[cfg(not(windows))]
fn show_error(msg: &str) {
    eprintln!("{msg}");
}
