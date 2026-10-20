//! Łagodne zamykanie bez zależności: SIGTERM/SIGINT (Docker, systemd, Ctrl+C) na unix,
//! zdarzenia konsoli (Ctrl+C, zamknięcie okna, wylogowanie) na Windows.
//!
//! Obsługa sygnału tylko ustawia flagę (bezpieczne w kontekście sygnału); resztę robi zwykły wątek.
//! Bez tego proces jako PID 1 w kontenerze ignoruje SIGTERM i `docker stop` czeka 10 s na SIGKILL.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static STOP: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
mod sys {
    use std::sync::atomic::Ordering;

    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }

    extern "C" fn on_signal(_: i32) {
        super::STOP.store(true, Ordering::SeqCst);
    }

    pub fn install() {
        const SIGINT: i32 = 2;
        const SIGTERM: i32 = 15;
        let h = on_signal as extern "C" fn(i32) as usize;
        // SAFETY: `signal` z libc (linkowanej przez std); procedura tylko zapisuje atomową flagę.
        unsafe {
            signal(SIGINT, h);
            signal(SIGTERM, h);
        }
    }
}

#[cfg(windows)]
mod sys {
    use std::sync::atomic::Ordering;

    #[link(name = "kernel32")]
    extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<unsafe extern "system" fn(u32) -> i32>, add: i32) -> i32;
    }

    unsafe extern "system" fn on_ctrl(_: u32) -> i32 {
        super::STOP.store(true, Ordering::SeqCst);
        // przy zamknięciu okna konsoli system kończy proces po powrocie – daj wątkowi czas na sprzątanie
        std::thread::sleep(std::time::Duration::from_secs(2));
        1
    }

    pub fn install() {
        // SAFETY: rejestracja procedury obsługi konsoli z kernel32; procedura ustawia atomową flagę.
        unsafe {
            SetConsoleCtrlHandler(Some(on_ctrl), 1);
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod sys {
    pub fn install() {}
}

/// Instaluje obsługę sygnałów; `on_stop` wykona się raz, w osobnym wątku, po otrzymaniu sygnału.
pub fn on_signal(on_stop: impl FnOnce() + Send + 'static) {
    sys::install();
    std::thread::spawn(move || {
        while !STOP.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(100));
        }
        on_stop();
    });
}

#[cfg(test)]
pub fn trigger() {
    STOP.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    #[test]
    fn stop_runs_callback_once() {
        let (tx, rx) = std::sync::mpsc::channel();
        super::on_signal(move || tx.send(()).unwrap());
        super::trigger();
        assert!(rx.recv_timeout(std::time::Duration::from_secs(2)).is_ok());
    }
}
