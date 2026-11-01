//! Pojedyncza instancja: kolejne uruchomienie na tym samym katalogu danych przekazuje prośbę
//! (rodzaj okna, plik) do działającej powłoki przez gniazdo użytkownika i kończy pracę. Windows: jeszcze nie.

#[cfg(unix)]
mod imp {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};

    fn socket(data: &Path) -> PathBuf {
        let full = std::fs::canonicalize(data).unwrap_or_else(|_| data.to_path_buf());
        let name = crate::bridge::draft_path("instancja", &full.to_string_lossy());
        let stem = name.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let dir = std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()).map(PathBuf::from).unwrap_or_else(crate::paths::state_dir);
        dir.join(format!("vizrs-shell-{stem}.sock"))
    }

    /// Wysyła prośbę do działającej powłoki; `true` = przyjęta.
    pub fn forward(data: &Path, msg: &str) -> bool {
        let Ok(mut s) = UnixStream::connect(socket(data)) else { return false };
        writeln!(s, "{msg}").is_ok()
    }

    /// Nasłuch próśb; każdą linię przekazuje `on_msg`.
    pub fn listen(data: &Path, on_msg: impl Fn(String) + Send + 'static) {
        let path = socket(data);
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).ok();
        }
        std::fs::remove_file(&path).ok(); // po awarii: `forward` wcześniej stwierdził, że nikt nie słucha
        let Ok(l) = UnixListener::bind(&path) else { return };
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).ok();
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let mut line = String::new();
                if BufReader::new(s).take_line(&mut line) {
                    on_msg(line);
                }
            }
        });
    }

    trait TakeLine {
        fn take_line(self, out: &mut String) -> bool;
    }
    impl<R: BufRead> TakeLine for R {
        fn take_line(self, out: &mut String) -> bool {
            let mut lim = self.take(8192);
            lim.read_line(out).map(|n| n > 0).unwrap_or(false)
        }
    }
}

#[cfg(unix)]
pub use imp::{forward, listen};

#[cfg(not(unix))]
pub fn forward(_data: &std::path::Path, _msg: &str) -> bool {
    false
}
#[cfg(not(unix))]
pub fn listen(_data: &std::path::Path, _on_msg: impl Fn(String) + Send + 'static) {}
