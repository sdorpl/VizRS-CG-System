# PLAN – okno natywne `vizrs-shell` na GTK 4

Stan: **propozycja, kod nieruszony** (2026-11-01, gałąź `artist-next`). Dotyczy tylko Linuksa: Windows i macOS zostają na tao + wry.

## Po co i kiedy warto

Korzyści, których spodziewamy się po GTK 4:

1. skalowanie ułamkowe – GTK 3 zna tylko skale całkowite (DOKUMENTACJA §13.2),
2. nowsza gałąź grafiki i Wayland (GSK, Vulkan/GL, DMA-BUF),
3. odejście od GTK 3 i WebKitGTK 4.1 zanim znikną z dystrybucji; GTK 4 daje też dostęp do nowych funkcji WebKitGTK.

Korzyść 1 **nie jest pewna**: są zgłoszenia rozmytej treści WebKitGTK na GTK 4 przy 125% ([WebKit #271714](https://bugs.webkit.org/show_bug.cgi?id=271714)), a [GTK 4.24 poprawił skalowanie](https://www.phoronix.com/news/GTK-4.24-Released), lecz nie wiadomo, czy WebKit z tego korzysta. Dlatego najpierw etap G0: pomiar, a dopiero potem decyzja o przebudowie.

## Stan wyjścia

- `source/shell/src/main.rs` (1131 wierszy) miesza logikę okien z tao, wry i muda. Pozostałe moduły (`bridge`, `connect`, `instance`, `paths`, `server`) nie zależą od GUI; `desktop.rs` zmieni się tylko w nazwach.
- `wry` 0.57, `tao` 0.37 i `muda` 0.21 wiążą się z GTK 3 (`gtk` 0.18, `webkit2gtk` 2.0 w `Cargo.lock`). Nie znaleziono zapowiedzi GTK 4 w wry ([dyskusja](https://github.com/tauri-apps/wry/discussions/1178) jest stara – potwierdzić przed startem). **Podbicie wersji crate'ów nie wystarczy.**
- Crates.io (2026-11-01): `gtk4` 0.11.5, `webkit6` 0.6.1, `libadwaita` 0.9.2.
- Maszyna robocza: GTK 4.24.1, WebKitGTK 6.0 2.52.6. Brakuje `libwebkitgtk-6.0-dev` (jest w repozytorium).

## Decyzje

| # | Decyzja | Powód |
|---|---|---|
| D1 | Linux: własny kod na `gtk4` + `webkit6`, bez tao, wry, muda i rfd w drzewie zależności Linuksa. Windows/macOS bez zmian | W jednym procesie nie wolno załadować GTK 3 i GTK 4. Fork wry to stały koszt utrzymania |
| D2 | Najpierw wydzielić rdzeń niezależny od GUI, potem dodać backend | Logika zamykania, mostu, przełączania serwera ma pozostać jedna na wszystkie systemy |
| D3 | Zachować `window.ipc.postMessage` przez skrypt startowy w backendzie | `web/native.js` zostaje bez zmian |
| D4 | Bez libadwaita | Główny pulpit to KDE; widżety są w stronach, a mniej zależności to mniej ryzyka. Do `Cargo.toml` dochodzą tylko `gtk4` i `webkit6` |
| D5 | Zostaje klasyczny pasek menu (`PopoverMenuBar` + `GMenu` + `GAction`), bez paska nagłówka | Pełne menu z etapu 1, stanowisko pracy |
| D6 | GTK 3 jako cecha Cargo `gtk3` do końca G5, usunięcie w wydaniu po G6 | Droga odwrotu na antenie; dwa backendy tylko przejściowo |
| D7 | Minimalny system: Ubuntu 24.04 / Debian 13; budowa wydań na 24.04 | Ubuntu 22.04 ma WebKitGTK 6.0, ale stare GTK 4 (do sprawdzenia, oczekiwane 4.6) bez `FileDialog` i `AlertDialog` z 4.10. Starsze systemy: AppImage przechodzi na okno Chromium (jak dziś bez WebKitGTK) |

**Wymaga zgody użytkownika:** zmiana białej listy crate'ów w `CLAUDE.md` i `source/shell/Cargo.toml` (D1) oraz porzucenie Ubuntu 22.04 (D7).

## Etapy

Każdy etap to osobny, zielony commit lub seria; praca na gałęzi od `artist-next`.

### G0. Prototyp i decyzja „idziemy / nie idziemy”

Osobny mały program poza repo: okno GTK 4 + WebKitGTK 6.0 z `/artist` i `/output`. Na KDE Wayland i X11 sprawdzić:

| Pytanie | Warunek przejścia |
|---|---|
| Ostrość tekstu przy 100/125/150% (porównanie zrzutów z GTK 3 i Chromium) | co najmniej nie gorzej niż GTK 3; przy 125% lepiej – inaczej korzyść 1 odpada i decyzję trzeba podjąć od nowa |
| Dekoracje okna na KWin (serwerowe czy rysowane przez GTK) | wygląd do przyjęcia na KDE |
| Płynność Artist i `/output` (animacje, wideo) oraz `HardwareAccelerationPolicy` | bez spadku względem GTK 3 |
| Skróty i pisownia polska | Ctrl+S, Ctrl+Z, F5, F11 działają; `pl_PL` podkreśla błędy |
| Przeciąganie plików z menedżera plików na Artist | strona dostaje `File` |
| Podpięcie wielu monitorów (`fullscreen_on_monitor`) | pełny ekran na wskazanym monitorze, Fill + Key |
| `tools/engines.js` na tym silniku | 0 różnic |

Wynik zapisać w tym pliku. Bez spełnienia warunków zostajemy na GTK 3.

### G1. Wydzielenie rdzenia (bez zmiany zachowania)

Podział `main.rs`: `args.rs` (`Args`, `parse_args`), `menu.rs` (opis menu jako dane: pozycja, identyfikator, skrót, czynna tylko w Artist), `app.rs` (`App`, `on_page`, `on_conn`, `on_menu`, zamykanie, `switch`, `send_file`), `backend/wry.rs`.

Rdzeń rozmawia z oknem przez typ backendu:

```rust
trait Host { fn open(&mut self, spec: WinSpec) -> Result<WinId, String>; fn close(&mut self, id: WinId); fn quit(&mut self); fn set_menu_enabled(&self, id: MenuKey, on: bool); }
trait Win { fn eval(&self, js: &str); fn load(&self, url: &str); fn reload(&self); fn set_title(&self, t: &str); fn set_zoom(&self, z: f64); fn present(&self); fn toggle_fullscreen(&self); fn devtools(&self); fn geometry(&self) -> Geometry; }
enum Ev { Menu(MenuKey), PageLoaded(WinId), Ipc(WinId, String), CloseRequested(WinId), Focused(WinId), /* reszta jak `UserEvent` */ }
```

Bramka: testy `shell` i `tools/engines.js` bez zmian, ręczny przegląd menu. Bez zmian w dokumentacji użytkownika. Ten etap przydaje się też etapowi 2 (Windows), nawet gdyby G2 nie nastąpił.

### G2. Backend GTK 4: okno i strona

`backend/gtk4.rs`, `Cargo.toml`: cecha `gtk4` / `gtk3`, zależności tylko pod `cfg(target_os = "linux")`.

| Dziś | GTK 4 / WebKitGTK 6.0 |
|---|---|
| `EventLoop`, `EventLoopProxy` | `gtk::Application` z `NON_UNIQUE` (jedna instancja dalej przez `instance.rs`, bo zależy od katalogu danych); z wątków `glib::MainContext::default().invoke` |
| `WindowBuilder` | `ApplicationWindow`, `set_default_size`, `maximize`, `fullscreen_on_monitor` |
| `CloseRequested` | `connect_close_request` + `Propagation::Stop` |
| `with_ipc_handler` | `UserContentManager::register_script_message_handler` + skrypt `window.ipc = { postMessage: (s) => webkit.messageHandlers.ipc.postMessage(s) }` |
| `with_initialization_script` | `UserScript` na początku dokumentu, tylko ramka główna (sprawdzić zachowanie wry) |
| `evaluate_script` | `evaluate_javascript` |
| `with_navigation_handler`, nowe okna | `decide-policy` (`NavigationPolicyDecision`, `NewWindow`) |
| `with_on_page_load_handler` | `load-changed` → `Finished` |
| `with_clipboard`, `with_devtools` | `Settings` + `inspector().show()` |
| `with_html` (okno połączenia) | `load_html` |
| `zoom` | `set_zoom_level` |

**Bezpieczeństwo mostu:** nadawca `app:` nadal sprawdzany jak dziś (`page_ok`, tylko `http…`), a `connect` tylko z okna wbudowanego. Test: strona spoza serwera i `data:` nie przechodzą. Żadnych nowych komunikatów.

**Uwaga do skryptu mostu:** wiersz `window.ipc` działa w świecie strony tak jak w wry – nie zwiększa dostępu, ale nie może istnieć w ramkach obcego pochodzenia.

### G3. Menu, dialogi, pobieranie

- Menu: `GMenu`, akcje `app.*`, skróty `set_accels_for_action`. Stan czynny przez `SimpleAction::set_enabled`. Skrótów edycji dalej nie ma w menu (Ctrl+Z i Ctrl+S w polach tekstowych obsługuje strona). Wybrane okno bez paska menu dla wyjścia na monitorze.
- Dialogi: `gtk::FileDialog`, `AlertDialog` – są **asynchroniczne**, a kod jest synchroniczny (`save_file()`). `error_dialog` przed startem aplikacji (zły argument) musi działać bez pętli: mała pętla lokalna albo sam stderr.
- Pobieranie: `NetworkSession` → `download-started`, `decide-destination` z oknem „Zapisz jako” (asynchronicznie; błąd i anulowanie jak dziś).
- Pisownia, polityka GPU (`--gpu`): `WebContext`, `Settings`.

### G4. Integracja z pulpitem

- Identyfikator aplikacji (np. `pl.vizrs.Shell`) to `app_id` na Waylandzie: nazwa wpisu `.desktop` musi się z nim zgadzać. Zmiany: `desktop.rs`, `deploy/install-desktop-linux.sh`, `.deb`, AppImage; stare wpisy `vizrs-shell.desktop` sprzątnąć.
- GTK 4 bierze ikonę z motywu, nie z PNG (`Window::set_default_icon_name`) – `ICON_PNG` tylko do instalacji ikony.
- **Pamięć pozycji odpada:** GTK 4 nie ustawia pozycji okna. Zostają rozmiar, maksymalizacja, zoom, scena. Pole `pos` w `window.json` ignorowane, stare pliki czytane dalej.
- Lista monitorów: `gdk::Display::monitors()`, nazwa z `model()`/`connector()`; menu Wyjście odświeżane przy podłączeniu monitora (dziś nie jest, a tu jest to darmowe – `ListModel`).

### G5. Testy i CI

- CI (`ci.yml`, `release.yml`): `libwebkitgtk-6.0-dev libgtk-4-dev`, Ubuntu 24.04; `shell-e2e` przez Xvfb, sprawdzić, czy zmienne `WEBKIT_DISABLE_*` są nadal potrzebne.
- Strażnik: `cargo tree --target x86_64-unknown-linux-gnu` bez `gtk`, `gdk`, `webkit2gtk`, `tao`, `wry`, `muda`, `rfd` – krok CI (po G6).
- `tests/e2e/shell.js`, `tools/engines.js` (0 różnic, 82/82), `--probe`, dodane: most – odrzucanie obcych stron, zamykanie ze zmianami, przeciąganie.
- Lista ręczna: KDE i GNOME, Wayland i X11, 100/125/150%, Fill + Key, praca z serwerem zdalnym, plik z menedżera plików, kopia robocza po awarii.

### G6. Przełączenie i sprzątanie

1. Wydanie z GTK 4 domyślnie, GTK 3 jako `--features gtk3` (jedna wersja).
2. Pakiety: `.deb` `Depends: libwebkitgtk-6.0-4, libgtk-4-1`; AppRun sprawdza WebKitGTK 6.0, bez niego okno Chromium; `release.yml`.
3. Usunięcie backendu GTK 3, `webkit2gtk`, cechy `gtk3`.
4. Dokumentacja: `CLAUDE.md` (biała lista), DOKUMENTACJA §13.2 i wymagania, RUNBOOK, CHANGELOG, LUKI, STATUS, nagłówek `Cargo.toml`; binarka `bin/linux-x86_64/vizrs-shell` razem z commitem.

## Ryzyka

| Ryzyko | Zabezpieczenie |
|---|---|
| Skalowanie ułamkowe nie poprawi się | warunek w G0 |
| Dekoracje GTK 4 obce na KDE | G0; w razie potrzeby własny styl CSS, inaczej stop |
| `webkit6` nie ma potrzebnego API | prototyp G0 przechodzi przez most, pobieranie i politykę nawigacji; luki przez `*-sys` z `unsafe` |
| `send_file` duże pliki (do 512 MB przez `evaluate_javascript`) | pomiar 500 MB w G2; jest to też istniejący limit, nie gorszy niż teraz |
| Utrata odbiorców na starszych systemach | D7, AppImage → Chromium; przed G6 komunikat w CHANGELOG |
| Dwa backendy się rozjadą | krótki okres (D6), wspólny rdzeń (D2) |
| Sterowniki WebKit/DMA-BUF | `--gpu off`, `VIZRS_GPU` bez zmian |

## Kolejność i punkty decyzji

G0 → **decyzja** → G1 → G2 → G3 → G4 → G5 → G6. G1 można robić niezależnie od wyniku G0.

Następny krok: `sudo apt install libwebkitgtk-6.0-dev` i prototyp G0.
