# VizRS CG System

Lekki klon duetu **Viz Artist** (projektowanie szablonów) + **Viz Trio** (playout stron) dla grafiki telewizyjnej na żywo.
Serwer w Rust bez zależności zewnętrznych; grafika renderuje się w przeglądarce jako przezroczysta strona 1920×1080,
którą OBS (albo vMix, Tricaster, ATEM przez Fill/Key) pobiera jako źródło **Przeglądarka**.
Projektowanie scen to od 2026-11-07 osobny program **VizRS CG Artist** (`../VizRS CG Artist`, Tauri 2) – kopiuje moduły
rysowania z `source/web` (`tools/sync-cg.sh` po jego stronie), więc zmiana w `renderer.js` i spółce trafia tam przez synchronizację.
`/artist` w serwerze jest **zamrożony** (zapas do pracy w przeglądarce, do 3.0): tylko poprawki błędów, nowe funkcje Artist – w VizRS CG Artist.

Język pracy z użytkownikiem i dokumentacji: **polski**. Kod, komentarze i komunikaty UI też są po polsku — pisz zgodnie z otoczeniem.

## Zacznij każdą sesję tutaj

1. Przeczytaj [docs/STATUS.md](docs/STATUS.md) (bieżący etap, następne zadanie), `git log -5` i `git status`.
2. Przeczytaj [docs/LUKI.md](docs/LUKI.md): znane braki, uszeregowane G1 (niezawodność) → G2 (wierność) → G3 (funkcje).
3. Przy zmianie zachowania aplikacji przeczytaj właściwą sekcję [docs/DOKUMENTACJA.md](docs/DOKUMENTACJA.md) — w tym samym commicie ją zaktualizujesz.
4. Plany i analizy są w [docs/plany/](docs/plany/README.md). Przy tickerach: [PLAN-ticker.md](docs/plany/PLAN-ticker.md); przy większych pracach: [PLAN-rozbudowy.md](docs/plany/PLAN-rozbudowy.md); okna aplikacji i serwer VPS/Docker: [PLAN-natywne-i-serwer.md](docs/plany/PLAN-natywne-i-serwer.md); playout kanału (VizRS CG Playout, osobne repo): [PLAN-kanal.md](docs/plany/PLAN-kanal.md), lekkie wyjście: [PLAN-lekkie-output.md](docs/plany/PLAN-lekkie-output.md).

## Niepodważalne zasady

- **Zero zależności w rdzeniu.** `source/Cargo.toml` (serwer `vizrs`) nie ma sekcji zależności — własny HTTP/1.1, SSE, JSON, OSC. Nie dodawaj tam crate'ów. Front-end to czysty JS (moduły ES), bez bundlera i bez bibliotek z CDN (wyjście musi działać offline).
- **Wyjątek: okno natywne `source/shell/`** (`vizrs-shell`, decyzja użytkownika 2026-10-20) — osobny workspace z własnym `Cargo.lock`; dozwolone tylko crate'y wymienione w nagłówku `source/shell/Cargo.toml` (tao, wry, muda, rfd, png, serde_json, webkit2gtk). Powłoka nie wkompilowuje serwera, tylko uruchamia `vizrs serve --idle-exit` albo dołącza do działającego; nie dawaj stronie dostępu do systemu przez IPC (biała lista komunikatów). Zmiana renderera → `node tools/engines.js` (Chromium vs WebView okna, musi być 0 różnic).
- **UI jest wbudowane w binarkę** (`include_str!` w `source/src/main.rs`, tablica `STATIC`). Wniosek:
  - zmiana w `source/web/*` nie działa w uruchomionym programie, dopóki nie przebudujesz binarki albo nie uruchomisz z `--web source/web`;
  - nowy plik w `web/` trzeba dopisać do `STATIC`;
  - `start-linux.sh` uruchamia `bin/linux-x86_64/vizrs` (plik w repo) — po zmianach UI skopiuj tam `source/target/release/vizrs`.
- **Jeden renderer.** `web/renderer.js` (`ScenePlayer`) renderuje wyjście OBS, podgląd w Trio i kanwę Artist. Nie duplikuj logiki rysowania w Trio/Artist.
- **Wyjątek: renderer natywny `source/render/`** (`vizrs-render`, gałąź `render-native`, plan [PLAN-native.md](docs/plany/PLAN-native.md)) — drugi wykonawca tej samej semantyki tylko dla playoutu kanału bez GPU; osobny workspace z własnym `Cargo.lock`, dozwolone crate'y z nagłówka `source/render/Cargo.toml` (tiny-skia, rustybuzz, wuff, serde_json, png, jpeg-decoder). Czego nie umie, zgłasza (`check`, flaga klatki) zamiast zgadywać. Zmiana w `renderer.js` (rysowanie, tekst, krzywe, ticker) → ta sama w `source/render/` + `node tools/native-compare.js`.
- **Dane z zewnątrz są wrogie.** Każdy zapis show przechodzi przez `validate.rs` (limity, typy). Nowe pole w modelu = reguła w `validate.rs` + test.
- **Nie wywalaj serwera:** błędy przez `Result`, blokady odporne na zatrucie (`lk()`), wolny klient SSE jest odłączany. Antena ma działać dalej, nawet gdy zapis na dysk się nie uda.
- **Model tekstu:** `web/text-model.js` (sanityzacja, RichText, typografia PL, reguły pól, Word Replace); zachowanie pilnują wektory `source/tests/vectors/*.json` (`node tests/vectors/run.js`) – zmiana zachowania = zmiana wektorów w tym samym commicie.
- **Gdzie co opisywać:** README tylko wprowadza i odsyła; opis działania tylko w `docs/DOKUMENTACJA.md` (nie powielaj go w README ani w planach); wdrożenie i awarie – `docs/RUNBOOK.md`; paczki – `examples/README.md`; nowy plan – `docs/plany/` + wiersz w `docs/plany/README.md`.
- **Numer wersji: rok.miesiąc.rewizja** (`26.11.1` = listopad 2026, pierwsze wydanie w miesiącu; kolejne `26.11.2`, w grudniu od `26.12.1`), bez zer wiodących (semver: Cargo, paczki, obrazy). Znacznik `v<wersja>` = `version` w `source/Cargo.toml` i `source/shell/Cargo.toml` (renderer natywny `source/render/` ma własną wersję protokołu). Wydania idą z gałęzi `main`; `main-oldstable` to stan sprzed 26.11.1.
- **Dokumentacja razem z kodem:** commit zmieniający zachowanie (UI, skróty, format show, API, opcje CLI) aktualizuje w tym samym commicie `docs/DOKUMENTACJA.md`, a gdy dotyczy codziennej pracy — także `docs/INSTRUKCJA-poczatkujacy.md`; API → `docs/API.md`; zmiany wydania → `CHANGELOG.md` (sekcja „Niewydane”); zamknięta luka → wiersz z `docs/LUKI.md`.

## Współpraca z Gemini

Szczegóły: [docs/AGENCI.md](docs/AGENCI.md); ten sam podział obowiązuje w całym pakiecie VizRS. W skrócie:

- **Claude dowodzi i jako jedyny pisze kod** (także `source/web/`, `source/shell/src/bridge.rs` z białą listą, `validate.rs`, `deploy/`, `.github/`, pliki zasad).
- **Gemini wykonuje lekkie zadania:** redaguje wskazane pliki `.md`, uruchamia kompilację i bramkę, streszcza długie wyjścia, przeszukuje źródła (także `../VizRS CG Artist`). Nie commituje.
- **Kontekst Gemini:** `AGENTS.md` to dowiązanie do tego pliku – `agy` sam wczytuje `AGENTS.md` i `GEMINI.md`, także w `-p` (pomiar agy 1.5.3, 2026-11-08: plik 19 KB w całości); importów `@plik` nie rozwija. Zasady zmieniaj tylko tutaj, `GEMINI.md` opisuje wyłącznie rolę wykonawcy i niczego stąd nie powiela.
- **Wywołanie:** `~/.gemini/bin/agy` (poza `PATH`), przełączniki – AGENCI §5. Wygodniej przez wrapper wtyczki antigravity: `PATH="$HOME/.gemini/bin:$PATH" agy-delegate [--tier flash|flash-lo] [--digest] [--mode plan] --dir . "<zlecenie>"`. Wrapper sam stawia `-p` na końcu i odcina stdin. Zamiast kodu 0 zwraca kod błędu: 10 – limit, 11 – logowanie, 12 – przekroczony czas, 14 – brak modelu, 15 – odmowa uprawnień, 3 – pusta odpowiedź. Na stderr wypisuje wiersz `AGY_USAGE` z tokenami i modelem. Tylko subskrypcja Google (bez kluczy API). `--yolo` / `--dangerously-skip-permissions` – nigdy: daje zgodę na cały komputer, a `--sandbox` jej nie ogranicza.
- **Próg opłacalności:** każde zlecenie ma koszt stały (zlecenie, czekanie, przegląd). Małe, jednorazowe i wymagające decyzji zadania rób sam – ich zlecanie to zmierzona strata. Zlecaj duże porcje tekstu przy niewielu decyzjach: prozę dokumentacji (też w kilku repozytoriach naraz, AGENCI §5), przeszukanie dużych źródeł, nieustrukturyzowane logi. Jedno duże zlecenie zamiast wielu małych. Bierz streszczenie (`--digest`), nie surowe wyjście; plików, które przerobił Gemini, nie czytaj ponownie – przeglądaj diff. Logi (bramka, kompilacja, testy) czytaj sam przez plik i filtr (`tail`, `grep`) – AGENCI §4.
- **Weryfikacja:** przed zleceniem migawka `git status` (i `git diff`, gdy pliki mają już zmiany), po zleceniu przegląd diffu – razem z usuniętymi wierszami. Kod 0 i raport „gotowe” niczego nie dowodzą. Bramkę uruchom sam, w czystym stanie – `agy` potrafi zmienić środowisko (połatać pakiety, podstawić atrapy), żeby test przeszedł. Każdą liczbę, ścieżkę i stan etapu sprawdź z kodem (Gemini dopowiada fakty).

## Uruchamianie

```bash
./start-linux.sh                                   # bin/linux-x86_64/vizrs --data ./vizrs-data
cd source && cargo build --release                 # budowa
./target/release/vizrs --data ../vizrs-data --web web   # UI z dysku (bez przebudowy przy zmianach web/)
```

| Adres | Co |
|---|---|
| `http://localhost:8088/` | strona startowa |
| `/artist` | Artist — projektowanie scen (zamrożony; właściwy edytor: VizRS CG Artist) |
| `/trio` | Trio — playout |
| `/mobile` | Trio na telefonie — sterowanie anteną z dotykowego ekranu |
| `/output` | wyjście dla OBS (przezroczyste) |

Do testów ręcznych uruchamiaj osobny serwer na kopii danych i innym porcie (`--port 8099 --tcp 0 --data <kopia>`), żeby nie ruszać `vizrs-data/` użytkownika.

## Testy (bramka przed commitem)

```bash
cd source
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
node tests/vectors/run.js
node tests/e2e/run.js target/release/vizrs        # Trio/Output (Playwright, PLAYWRIGHT_DIR=…)
node tests/e2e/mobile.js target/release/vizrs     # Trio na telefonie
node tests/e2e/artist.js target/release/vizrs     # Artist (zamrożony /artist)
node tests/e2e/pack.js target/release/vizrs       # paczka .cgpack
node tests/e2e/app.js target/release/vizrs        # okna aplikacji (udawana przeglądarka)
(cd shell && cargo clippy --all-targets -- -D warnings && cargo test && cargo build --release)   # okno natywne
node tests/e2e/shell.js target/release/vizrs shell/target/release/vizrs-shell     # okno natywne w działaniu (otwiera okna)
node tests/e2e/native-smoke.js target/release/vizrs shell/target/release/vizrs-shell   # test dymny okna (CI: Windows z instalatora, macOS)
node tools/engines.js target/release/vizrs shell/target/release/vizrs-shell       # zgodność WebView z Chromium (otwiera okno)
(cd render && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo build --release)   # renderer natywny
node tools/native-compare.js                      # zgodność renderera natywnego z Chromium (każda obsługiwana strona < 2%)
node tests/cgpack.mjs                             # reguły zasobów paczki (nazwy, czcionki bez duplikatów)
node tests/motion.js && node tests/e2e/motion.js target/release/vizrs   # ruch (node.motion)
node tests/e2e/golden.js target/release/vizrs     # zrzuty wzorcowe (lokalnie, zależne od fontów)
```

Playwright nie jest zależnością repo: `PLAYWRIGHT_DIR` wskazuje katalog z `node_modules/playwright` (bez niej – zwykłe `require('playwright')`).
Przy pracy nad UI **zobacz wynik** — zrzut ekranu z Playwright (pliki PNG trzymaj poza repo).

## Struktura

| Ścieżka | Rola |
|---|---|
| `source/src/main.rs` | serwer HTTP, routing `/api/*`, pliki statyczne (`STATIC`), port TCP, tryb okna (`app_window`) |
| `source/src/config.rs` | opcje CLI i zmienne `VIZRS_*` (czysty parser), komendy `serve`/`artist`/`trio`/`output`, `--healthcheck` |
| `source/src/app.rs` | okna aplikacji: wyszukiwanie Chromium/Edge, argumenty `--app`, profil, `--connect` |
| `source/src/datalock.rs`, `client.rs`, `shutdown.rs` | blokada katalogu danych (`.vizrs.lock`), klient HTTP GET, SIGTERM/Ctrl+C bez zależności |
| `source/src/bin/vizrs-app.rs` | uruchamianie okien bez konsoli (Windows) |
| `source/render/` | `vizrs-render` – renderer natywny dla playoutu kanału (tiny-skia, rustybuzz): `stream` (klatki przez stdin/stdout), `frame`, `check`, `bench`, `bounds` (granice stron dla stref VizRS CG Playout); osobny workspace z zależnościami (DOKUMENTACJA §10.1) |
| `source/shell/` | `vizrs-shell` – okno natywne Trio i wyjścia (tao + wry: WebKitGTK / WebView2 / WKWebView), menu, „Zapisz jako”, pisownia pl; bez Artist (`artist` odsyła do VizRS CG Artist); osobny workspace z zależnościami |
| `source/src/engine.rs` | show, warstwy on-air, TAKE/CONTINUE/OUT/UPDATE/ARM, deploy, dock, karuzela, rundown, makra, blokady, SSE |
| `source/src/validate.rs` | schemat i limity show (sceny, strony, playlisty, rundowny, makra, `fieldMeta`, presety, Word Replace) |
| `source/src/ticker.rs` | zapis wiadomości tickera (`KAT|treść`), komendy `TICKER` |
| `source/src/guard.rs` | tokeny (operator, wyjście tylko do odczytu), ochrona przed obcymi witrynami (CSRF), spowalnianie złych prób |
| `source/src/backup.rs`, `audit.rs` | kopie zapasowe show, log audytowy |
| `source/src/osc.rs`, `hook.rs` | OSC (UDP), webhook |
| `source/src/http.rs`, `json.rs` | własny parser HTTP/1.1 i JSON |
| `source/src/fonts.rs`, `fonts_data.rs` | wbudowane fonty offline (`/fonts/fonts.css`) |
| `source/web/renderer.js` | `ScenePlayer`, `PROPS`, `TYPES`, krzywe, pola, autofollow, tekst, ticker, zegar |
| `source/web/artist.html` | edytor scen (zamrożony – zapas w przeglądarce) |
| `source/web/trio.html` | playout |
| `source/web/mobile.html` | Trio na telefonie: osobna lekka strona, to samo API, bez `ScenePlayer` (monitor PGM to `/output` w ramce) |
| `source/web/mobile-sw.js`, `mobile.webmanifest`, `mobile-icon.*`, `mobile-login.html` | aplikacja na telefon (PWA): service worker, manifest, ikony, ekran logowania |
| `source/web/dialogs.js`, `native.js` | okna pytań aplikacji (`ask`, `askText`, `choose`) zamiast `confirm()`/`prompt()`; most strony z oknem natywnym |
| `source/web/field-rules.js` | reguły pól (limity, wersaliki, typografia), wspólne dla Trio i telefonu |
| `source/web/output.html` | wyjście OBS (`?layers`, `?bus=preview`, `?mode=fill|key`, `?bg`, `?checker`) |
| `source/web/index.html`, `ui.css` | strona startowa, wspólne style UI |
| `source/web/text-model.js`, `richtext.js` | reguły pól, typografia PL, Word Replace; znaczniki `{b}{i}{u}{c=}{sz=}` i `[[ ]]` |
| `source/web/ticker-model.js`, `ticker-items.js`, `color-picker.js` | model wiadomości tickera; edytor listy wiadomości i wybór koloru (Trio i Artist) |
| `source/web/qr.js` | generator kodów QR offline |
| `source/web/motion.js` | ruch (`node.motion`): krzywe GSAP, presety IN/OUT, efekty tekstu, pętle uwagi, sekwencja `after` – czyste funkcje, rysuje `renderer.js` |
| `source/web/cgpack.js` | paczka belek VizRS `.cgpack` (sceny + strony + grafiki + czcionki): zapis ZIP, odczyt, instalacja plików |
| `source/web/demo_show.json` | show startowy (INFO24) |
| `source/tests/` | `vectors/` (model tekstu), `cgpack.mjs` (reguły paczki), `e2e/` (Playwright), `golden/` (zrzuty wzorcowe) |
| `source/tools/` | narzędzia pakietów, galerii, filmów i zgodności (`build-packs.mjs`, `gallery.js`, `engines.js`, `video.js`…); pełna lista: DOKUMENTACJA §14 |
| `examples/` | paczki `INFO24.cgpack`, `R7_NEWS.cgpack` (budowa: `tools/build-packs.mjs`); `archiwum/` – źródłowe `.show.json` i `assets/` (używane przez testy) |
| `deploy/`, `render.yaml` | systemd, launchd, Windows (NSSM), Docker compose + Caddy, nginx, Render.com, AppImage i `.deb`, skróty okien (Linux `.desktop`, Windows `.ps1`) |
| `docs/` | dokumentacja (spis: `docs/README.md`); `docs/plany/` – plany i raport (dokumenty projektowe, nie opis bieżącego działania) |
| `Dockerfile` | obraz `scratch` ze statyczną binarką musl; wieloarchitekturowy (amd64, arm64 z optymalizacją pod Ampere A1 Neoverse-N1: `-C target-cpu=neoverse-n1 -C opt-level=3` przez `rust-lld` bez emulacji QEMU) |
| `bin/linux-x86_64/vizrs` | gotowa binarka Linux (w repo, używa jej `start-linux.sh`) |
| `vizrs-data/` | dane użytkownika (`show.json`, `assets/`, `backups/`, `log.jsonl`) — w `.gitignore`; przy pustym katalogu serwer wczytuje demo |

## Model w skrócie

Show = `scenes[]` (szablony z Artist) + `pages[]` (strona = numer + scena + `values`) + `playlists[]` + `rundowns[]` + `macros[]`.
Scena ma warstwę (`layer`), długość, stop pointy, drzewo węzłów (`nodes`) z `props`, `anim` (klucze `{t,v,e}`), `fields` (właściwość → pole kontrolne)
i `showIf` (wariant). Szczegóły: [docs/DOKUMENTACJA.md](docs/DOKUMENTACJA.md) §3–4.

## Commity

- Jedna spójna zmiana = jeden commit, po polsku, opisowo (styl historii: „Tickery: …”). Tylko zielone stany.
- Pliki dodawaj jawnie (`git add ścieżka`), nie `git add -A`. Nie commituj `source/target/`, zrzutów i plików tymczasowych.
- Binarkę `bin/linux-x86_64/vizrs` aktualizuj w commicie, który zmienia UI lub serwer (inaczej `start-linux.sh` uruchamia starą wersję); `bin/linux-x86_64/vizrs-shell` – w commicie zmieniającym `source/shell/`.
- Commit i push tylko na prośbę użytkownika.
