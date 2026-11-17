# VizRS CG System

Lekki klon duetu **Viz Artist** (projektowanie szablonów) + **Viz Trio** (playout stron) dla grafiki telewizyjnej na żywo.
Serwer w Rust bez zależności zewnętrznych; grafika renderuje się w przeglądarce jako przezroczysta strona 1920×1080,
którą OBS (albo vMix, Tricaster, ATEM przez Fill/Key) pobiera jako źródło **Przeglądarka**.
Projektowanie scen to od 2026-11-07 osobny program **VizRS CG Artist** (`../VizRS CG Artist`, Tauri 2) – kopiuje moduły
rysowania z `source/web` (`tools/sync-cg.sh` po jego stronie), więc zmiana w `renderer.js` i spółce trafia tam przez synchronizację.
`/artist` w serwerze jest **zamrożony** (zapas do pracy w przeglądarce, do 3.0): tylko poprawki błędów, nowe funkcje Artist – w VizRS CG Artist.

<!-- pakiet:A – blok wspólny pakietu VizRS; źródło: VizRS CG System/CLAUDE.md, kopie: tools/sync-zasady.sh (nie edytuj w kopii) -->
Język pracy z użytkownikiem, dokumentacji, komentarzy, komunikatów i UI: **polski** – pisz zgodnie z otoczeniem.

## Pakiet VizRS

To repo jest częścią pakietu **VizRS**. Opis całego pakietu (mapa aplikacji, styki, stos, porty, plan porządków): [sdorpl/VizRS](https://github.com/sdorpl/VizRS) (lokalnie `../VizRS`).

| Aplikacja (katalog obok, `../`) | Rola | Dostawca styku |
|---|---|---|
| **VizRS CG System** (`VizRS CG System`) | silnik grafiki: Trio, wyjście `/output`, TCP 6100, HTTP/SSE, `vizrs-render`; źródło wspólnego kodu i zasad pakietu | protokół VizRS, `/output`, `vizrs-render`, format show i `.cgpack` (`docs/API.md`) |
| **VizRS CG Artist** (`VizRS CG Artist`) | edytor scen i animacji (Tauri 2) | – (klient CG System) |
| **VizRS CG Playout** (`VizRS CG Playout`) | sidecar grafiki i live ingest obok ErsatzTV; panel kanału nad ErsatzTV legacy (`panel/`, dawny VizRS Live): VizRS Bridge, natywny klient panelu; stos compose | Live API `:7700`, API panelu i most `/vizrs-bridge/*` (`docs/API.md`) |

VizRS Live (panel, most, okno) połączono 2026-11-15 z VizRS CG Playout w jeden produkt **„VizRS CG Playout”** z jedną wersją i jednym wydaniem (decyzja D1, plan: sdorpl/VizRS §3); repo `sdorpl/VizRS-Live-Playout` przechodzi do archiwum.

Zgodność – wszystkie aplikacje w bieżących wersjach działają razem:
- **Styk zmienia się po obu stronach w tym samym cyklu.** Opis styku jest u dostawcy; klient opisuje tylko, jak z niego korzysta.
- **Starsza wersja sąsiada nie psuje aplikacji ani emisji** – brak nowego pola lub ścieżki to „niedostępne w tej wersji”, nie błąd.
- **Wydania razem:** przed wydaniem sprawdź z bieżącymi wersjami sąsiadów; zmianę wymagającą nowszego sąsiada odnotuj w `CHANGELOG.md` z minimalną wersją.
<!-- /pakiet:A -->

## Zacznij każdą sesję tutaj

<!-- pakiet:S -->
- Przeczytaj [docs/STATUS.md](docs/STATUS.md) (bieżący etap, następne zadanie), `git log -5` i `git status`.
- Przeczytaj [docs/LUKI.md](docs/LUKI.md): znane braki, uszeregowane G1 (niezawodność i bezpieczeństwo) → G2 (wierność) → G3 (funkcje).
- Przy zmianie zachowania przeczytaj właściwą sekcję [docs/DOKUMENTACJA.md](docs/DOKUMENTACJA.md) – w tym samym commicie ją zaktualizujesz.
- Zmiana dotyka styku z inną aplikacją pakietu – przeczytaj opis styku u dostawcy (tabela „Pakiet VizRS”) i sprawdź drugą stronę.
<!-- /pakiet:S -->
- Plany projektu: [docs/plany/](docs/plany/README.md). Przy tickerach: [PLAN-ticker.md](docs/plany/PLAN-ticker.md); przy większych pracach: [PLAN-rozbudowy.md](docs/plany/PLAN-rozbudowy.md); okna aplikacji i serwer VPS/Docker: [PLAN-natywne-i-serwer.md](docs/plany/PLAN-natywne-i-serwer.md); playout kanału (VizRS CG Playout, osobne repo): [PLAN-kanal.md](docs/plany/PLAN-kanal.md), lekkie wyjście: [PLAN-lekkie-output.md](docs/plany/PLAN-lekkie-output.md).

## Niepodważalne zasady

### Wspólne dla pakietu

<!-- pakiet:B -->
- **Emisja nie pada przez nasz błąd.** Błędy przez `Result`, limity czasu na połączeniach, blokady odporne na zatrucie; awaria grafiki, panelu albo zapisu na dysk nie zatrzymuje anteny ani kanału.
- **Dane z zewnątrz są obce.** Wejście z sieci, plików, ErsatzTV i sąsiednich aplikacji – walidacja i limity; tekst do DOM tylko przez `textContent` (bez `innerHTML`/`eval`); sekrety (tokeny) nie trafiają do przeglądarki ani do logów.
- **Zależności tylko z listy.** Serwery pakietu (`vizrs`, `vizrs-live`) nie mają zależności zewnętrznych; front-end to czysty JS (moduły ES) bez bundlera, bibliotek i CDN. Wyjątki (okna Tauri, renderer natywny, sidecar) – tylko crate'y wymienione w nagłówku ich `Cargo.toml`; nowa zależność to decyzja użytkownika.
- **Kod wspólny tylko przez synchronizację.** Kod, wygląd (`vizrs-ui.css`), moduły okna (`gpu.rs`, `scale.rs`, `screen.rs`) i zasady pakietu mają źródło w VizRS CG System; kopie aktualizują skrypty (`tools/sync-cg.sh`, `scripts/sync-ui.sh`, `tools/sync-zasady.sh`) ze sprawdzaniem `--check` w CI – kopii nie edytuje się ręcznie. Cudzy kod tylko na licencji zgodnej z MIT i z wpisem w `THIRD-PARTY.md`. ErsatzTV – wyłącznie przez jego interfejsy (element „script”, ścieżka FFmpeg, startup hook mostu, komendy MediatR), bez kopiowania i bez zmian w jego kodzie.
- **Licencja: MIT** (`LICENSE`, `Copyright (c) 2026 sdorpl`; pole `license` w manifestach); składniki zewnętrzne – `THIRD-PARTY.md`.
- **Numer wersji: rok.miesiąc.rewizja** (`26.11.1` = listopad 2026, pierwsze wydanie w miesiącu; kolejne `26.11.2`, w grudniu od `26.12.1`), bez zer wiodących (semver: Cargo, paczki, obrazy). Znacznik `v<wersja>` = wersja we wszystkich manifestach projektu (lista – „Tego projektu”). Wydania z gałęzi `main`; `main-oldstable` to stan sprzed 26.11.1.
- **Gdzie co opisywać:** README tylko wprowadza i odsyła; opis działania, opcje i zmienne tylko w `docs/DOKUMENTACJA.md` (nie powielaj w README, planach ani STATUS); API, którego aplikacja jest dostawcą – `docs/API.md` (albo sekcja wskazana w „Tego projektu”); praca użytkownika – `docs/INSTRUKCJA*.md`; wdrożenie i awarie – `docs/RUNBOOK.md`; plany – `docs/plany/` + wiersz w `docs/plany/README.md`; opis całego pakietu – sdorpl/VizRS.
- **Dokumentacja razem z kodem:** commit zmieniający zachowanie (UI, API, format, opcje, obrazy) aktualizuje w tym samym commicie `docs/DOKUMENTACJA.md`, a gdy dotyczy codziennej pracy – także instrukcję; API – `docs/API.md`; wdrożenie – `docs/RUNBOOK.md`; zmiany wydania – `CHANGELOG.md` (sekcja „Niewydane”); zamknięta luka – usuń wiersz z `docs/LUKI.md`; zmiana etapu – `docs/STATUS.md`.
- **Produkcji i danych użytkownika nie ruszaj bez prośby.** Testy na kopii danych, innym porcie albo stosie testowym; działających serwerów, stosów i kontenerów nie zatrzymuj (mogą nadawać).
- **Zobacz wynik.** Przy pracy nad UI zrzut ekranu (Playwright; pliki PNG poza repo) i obejrzenie go przed commitem.
<!-- /pakiet:B -->

### Tego projektu

- **Zero zależności w rdzeniu.** `source/Cargo.toml` (serwer `vizrs`) nie ma sekcji zależności — własny HTTP/1.1, SSE, JSON, OSC. Nie dodawaj tam crate'ów. Front-end to czysty JS (moduły ES), bez bundlera i bez bibliotek z CDN (wyjście musi działać offline).
- **Wyjątek: okno natywne `source/shell/`** (`vizrs-shell`, Tauri 2 od 2026-11-12 – decyzja użytkownika, plan [PLAN-wyglad-live-i-tauri.md](docs/plany/PLAN-wyglad-live-i-tauri.md) T1) — osobny workspace z własnym `Cargo.lock`; dozwolone tylko crate'y wymienione w nagłówku `source/shell/Cargo.toml` (tauri, tauri-plugin-dialog, tauri-plugin-single-instance, serde_json, webkit2gtk; tauri-build) – te same co VizRS CG Artist. Okno nie wkompilowuje serwera, tylko uruchamia `vizrs serve --idle-exit` albo dołącza do działającego; nie dawaj stronie dostępu do systemu: komenda `ipc` z białą listą (`bridge.rs`), uprawnienie tylko dla adresu bieżącego serwera (`allow_server`). Zmiana renderera → `node tools/engines.js` (Chromium vs WebView okna, musi być 0 różnic).
- **UI jest wbudowane w binarkę** (`include_str!` w `source/src/static_files.rs`, tablica `STATIC`). Wniosek:
  - zmiana w `source/web/*` nie działa w uruchomionym programie, dopóki nie przebudujesz binarki albo nie uruchomisz z `--web source/web`;
  - nowy plik w `web/` trzeba dopisać do `STATIC`;
  - `start-linux.sh` uruchamia `bin/linux-x86_64/vizrs` (plik w repo) — po zmianach UI skopiuj tam `source/target/release/vizrs`.
- **Jeden renderer.** `web/renderer.js` (`ScenePlayer`) renderuje wyjście OBS, podgląd w Trio i kanwę Artist. Nie duplikuj logiki rysowania w Trio/Artist.
- **Wyjątek: renderer natywny `source/render/`** (`vizrs-render`, gałąź `render-native`, plan [PLAN-native.md](docs/plany/PLAN-native.md)) — drugi wykonawca tej samej semantyki tylko dla playoutu kanału bez GPU; osobny workspace z własnym `Cargo.lock`, dozwolone crate'y z nagłówka `source/render/Cargo.toml` (tiny-skia, rustybuzz, wuff, serde_json, png, jpeg-decoder). Czego nie umie, zgłasza (`check`, flaga klatki) zamiast zgadywać. Zmiana w `renderer.js` (rysowanie, tekst, krzywe, ticker) → ta sama w `source/render/` + `node tools/native-compare.js`.
- **`validate.rs` – granica danych.** Każdy zapis show przechodzi przez `validate.rs` (limity, typy). Nowe pole w modelu = reguła w `validate.rs` + test.
- **Serwer nie pada:** blokady `lk()` odporne na zatrucie, wolny klient SSE jest odłączany. Antena ma działać dalej, nawet gdy zapis na dysk się nie uda.
- **Model tekstu:** `web/text-model.js` (sanityzacja, RichText, typografia PL, reguły pól, Word Replace); zachowanie pilnują wektory `source/tests/vectors/*.json` (`node tests/vectors/run.js`) – zmiana zachowania = zmiana wektorów w tym samym commicie.
- **Gdzie co opisywać (uzupełnienie):** paczki – `examples/README.md`; instrukcja operatora – `docs/INSTRUKCJA-poczatkujacy.md`; API (dostawca styków pakietu: TCP 6100, HTTP/SSE, `/output`, `vizrs-render`) – `docs/API.md`.
- **Manifesty wersji:** znacznik `v<wersja>` = `version` w `source/Cargo.toml`, `source/shell/Cargo.toml` i `source/shell/tauri.conf.json` (renderer natywny `source/render/` ma własną wersję protokołu).

## Współpraca z Gemini

<!-- pakiet:C -->
Szczegóły: [docs/AGENCI.md](docs/AGENCI.md) – wspólny dla pakietu. W skrócie:

- **Claude dowodzi i jako jedyny pisze kod** (lista „Kod” niżej oraz pliki zasad, CI, Docker, testy, skrypty, manifesty, licencje, białe listy, decyzje w planach).
- **Gemini wykonuje lekkie zadania:** redaguje wskazane pliki `.md` z listy „Dokumentacja”, uruchamia bramkę z listy „Bramka” i streszcza wynik, streszcza długie wyjścia, przeszukuje źródła (także sąsiednich repozytoriów pakietu – tylko do czytania). Nie commituje.
- **Kontekst Gemini:** `AGENTS.md` to **kopia** tego pliku (nie dowiązanie: Portainer GitOps odrzuca repozytorium z symlinkiem, a Windows bywa z nimi zawodny) – zmienia się razem z `CLAUDE.md` (`tools/sync-zasady.sh` kopiuje, `--check` w CI porównuje). `agy` sam wczytuje `AGENTS.md` i `GEMINI.md`, także w `-p`; importów `@plik` nie rozwija. `GEMINI.md` i `docs/AGENCI.md` są wspólne dla pakietu (źródło: VizRS CG System) i niczego stąd nie powielają.
- **Wywołanie:** `~/.gemini/bin/agy` (poza `PATH`), przełączniki – AGENCI §5. Wygodniej przez wrapper wtyczki antigravity: `PATH="$HOME/.gemini/bin:$PATH" agy-delegate [-m "Gemini 3.8 Flash (High)"] [--digest] [--mode plan] --dir . "<zlecenie>"` (poziomu `--tier pro` w tej subskrypcji nie ma). Wrapper sam stawia `-p` na końcu i odcina stdin. Zamiast kodu 0 zwraca kod błędu: 10 – limit, 11 – logowanie, 12 – przekroczony czas, 14 – brak modelu, 15 – odmowa uprawnień, 3 – pusta odpowiedź. Na stderr wypisuje wiersz `AGY_USAGE` z tokenami i modelem. Tylko subskrypcja Google (bez kluczy API). `--yolo` / `--dangerously-skip-permissions` – nigdy: daje zgodę na cały komputer, a `--sandbox` jej nie ogranicza.
- **Próg opłacalności:** każde zlecenie ma koszt stały (zlecenie, czekanie, przegląd). Małe, jednorazowe i wymagające decyzji zadania rób sam – ich zlecanie to zmierzona strata. Zlecaj duże porcje tekstu przy niewielu decyzjach: prozę dokumentacji (też w kilku repozytoriach naraz, AGENCI §5), przeszukanie dużych źródeł, nieustrukturyzowane logi. Jedno duże zlecenie zamiast wielu małych. Bierz streszczenie (`--digest`), nie surowe wyjście; plików, które przerobił Gemini, nie czytaj ponownie – przeglądaj diff. Logi (bramka, kompilacja, testy) czytaj sam przez plik i filtr (`tail`, `grep`) – AGENCI §4.
- **Weryfikacja:** przed zleceniem migawka `git status` (i `git diff`, gdy pliki mają już zmiany), po zleceniu przegląd diffu – razem z usuniętymi wierszami. Kod 0 i raport „gotowe” niczego nie dowodzą. Bramkę uruchom sam, w czystym stanie – `agy` potrafi zmienić środowisko (połatać pakiety, podstawić atrapy), żeby test przeszedł. Każdą liczbę, ścieżkę i stan etapu sprawdź z kodem (Gemini dopowiada fakty).
<!-- /pakiet:C -->

### Ten projekt

- **Kod (tylko Claude):** `source/**` – serwer (`source/src/*.rs`; `validate.rs` – granica danych, `guard.rs` – tokeny i CSRF), strony i moduły `source/web/*` (wkompilowane w binarkę), okno `source/shell/` (`src/bridge.rs` – biała lista), renderer `source/render/`, testy i narzędzia; `deploy/**`, `start-*`, `build-*`, `bin/`, `examples/*.cgpack`, `examples/**/*.json`, `render.yaml`, `Dockerfile`, `tools/sync-zasady.sh`.
- **Dokumentacja (Gemini – tylko pliki wskazane w zleceniu):** `README.md`, `CHANGELOG.md`, `SZYBKI_START.txt`, `examples/README.md`, `docs/DOKUMENTACJA.md`, `docs/INSTRUKCJA-poczatkujacy.md`, `docs/API.md`, `docs/COMPANION.md`, `docs/RUNBOOK.md`, `docs/STATUS.md`, `docs/LUKI.md`, `docs/README.md`.
- **Bramka (Gemini może uruchomić i streścić):** `cargo clippy --manifest-path source/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path source/Cargo.toml`, `cargo build --release --manifest-path source/Cargo.toml` (okno i renderer: `source/shell/Cargo.toml`, `source/render/Cargo.toml`); `cargo fmt` tylko z `--check`.
- **Zawsze:** serwer i okna tylko na kopii danych i innym porcie (`--data <kopia> --port 8099 --tcp 0`); nie zatrzymuj działającego `vizrs` (może nadawać grafikę na antenę).

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
| `/trio` | Trio — playout; na telefonie i w wąskim oknie układ dotykowy (`/mobile` przekierowuje tutaj) |
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
node tests/e2e/mobile.js target/release/vizrs     # Trio w układzie telefonu
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
../tools/sync-zasady.sh --check                  # AGENTS.md = CLAUDE.md (zasady pakietu)
```

Playwright nie jest zależnością repo: `PLAYWRIGHT_DIR` wskazuje katalog z `node_modules/playwright` (bez niej – zwykłe `require('playwright')`).
Przy pracy nad UI **zobacz wynik** — zrzut ekranu z Playwright (pliki PNG trzymaj poza repo).

## Struktura

| Ścieżka | Rola |
|---|---|
| `source/src/main.rs` | serwer HTTP (połączenia, limity, SSE), tryb okna (`app_window`) |
| `source/src/routes.rs` | routing `/health` i `/api/*` |
| `source/src/commands.rs` | komendy tekstowe (`exec_command`, `@kanał`): port TCP 6100, OSC, makra, komendy zbiorcze |
| `source/src/static_files.rs` | pliki statyczne: UI wbudowane w binarkę (`STATIC`, `include_str!`), fonty, `/assets/*`, `mime`, `safe_name` |
| `source/src/config.rs` | opcje CLI i zmienne `VIZRS_*` (czysty parser), komendy `serve`/`artist`/`trio`/`output`, `--healthcheck` |
| `source/src/app.rs` | okna aplikacji: wyszukiwanie Chromium/Edge, argumenty `--app`, profil, `--connect` |
| `source/src/datalock.rs`, `client.rs`, `shutdown.rs` | blokada katalogu danych (`.vizrs.lock`), klient HTTP GET, SIGTERM/Ctrl+C bez zależności |
| `source/src/bin/vizrs-app.rs` | uruchamianie okien bez konsoli (Windows) |
| `source/render/` | `vizrs-render` – renderer natywny dla playoutu kanału (tiny-skia, rustybuzz): `stream` (klatki przez stdin/stdout), `frame`, `check`, `bench`, `bounds` (granice stron dla stref VizRS CG Playout), `thumb` (miniatury stron dla panelu VizRS Live); osobny workspace z zależnościami (DOKUMENTACJA §10.1) |
| `source/shell/` | `vizrs-shell` – okno natywne Trio i wyjścia na Tauri 2 (WebKitGTK / WebView2 / WKWebView): menu, „Zapisz jako”, pytanie przy zamknięciu, okno połączenia (`ui/connect.html`), jedna instancja, pisownia pl; `tauri.conf.json` + `tauri.bundle.json` (paczki z serwerem `vizrs` obok okna); bez Artist (`artist` odsyła do VizRS CG Artist); osobny workspace z zależnościami |
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
| `source/web/trio.html` | playout – jedna strona na stanowisko i telefon (układ dotykowy ≤ 760 px, jak panel VizRS CG Playout) |
| `source/web/trio-sw.js`, `trio.webmanifest`, `trio-icon.*`, `login.html` | Trio jako aplikacja na telefon (PWA): service worker, manifest, ikony, ekran logowania |
| `source/web/dialogs.js`, `native.js` | okna pytań aplikacji (`ask`, `askText`, `choose`) zamiast `confirm()`/`prompt()`; most strony z oknem natywnym |
| `source/web/field-rules.js` | reguły pól (limity, wersaliki, typografia) |
| `source/web/output.html` | wyjście OBS (`?layers`, `?bus=preview`, `?mode=fill|key`, `?bg`, `?checker`) |
| `source/web/index.html`, `ui.css`, `ui-frozen.css` | strona startowa; `ui.css` – style własne Trio i Artist (ładowane po `vizrs-ui.css`); `ui-frozen.css` – zamrożony dawny wygląd dla `/artist` i strony startowej |
| `source/web/vizrs-ui.css` | wspólny rdzeń wyglądu rodziny VizRS (tokeny i kontrolki panelu VizRS CG Playout); źródło kopii w VizRS CG Playout (`panel/scripts/sync-ui.sh`) i VizRS CG Artist (`tools/sync-cg.sh`), obie sprawdzane w CI (`--check`) – zmieniaj tylko tutaj |
| `source/web/text-model.js`, `richtext.js` | reguły pól, typografia PL, Word Replace; znaczniki `{b}{i}{u}{c=}{sz=}` i `[[ ]]` |
| `source/web/ticker-model.js`, `ticker-items.js`, `color-picker.js` | model wiadomości tickera; edytor listy wiadomości i wybór koloru (Trio i Artist) |
| `source/web/qr.js` | generator kodów QR offline |
| `source/web/motion.js` | ruch (`node.motion`): krzywe GSAP, presety IN/OUT, efekty tekstu, pętle uwagi, sekwencja `after` – czyste funkcje, rysuje `renderer.js` |
| `source/web/cgpack.js` | paczka belek VizRS `.cgpack` (sceny + strony + grafiki + czcionki): zapis ZIP, odczyt, instalacja plików, plan importu (`planImport`) i import wg planu |
| `source/web/pack-ui.js` | okna paczki wspólne dla Trio i VizRS CG Artist: plan importu (`planDialog`), wybór stron do eksportu (`pickPages`) |
| `source/web/demo_show.json` | show startowy (INFO24) |
| `source/tests/` | `vectors/` (model tekstu; `cgpack-placer.json` – reguły plików paczki, wspólne z VizRS CG Artist), `cgpack.mjs` (reguły paczki), `e2e/` (Playwright), `golden/` (zrzuty wzorcowe) |
| `source/tools/` | narzędzia pakietów, galerii, filmów i zgodności (`build-packs.mjs`, `gallery.js`, `engines.js`, `video.js`…); pełna lista: DOKUMENTACJA §14 |
| `examples/` | paczki `INFO24.cgpack`, `R7_NEWS.cgpack` (budowa: `tools/build-packs.mjs`); `archiwum/` – źródłowe `.show.json` i `assets/` (używane przez testy) |
| `deploy/`, `render.yaml` | systemd, launchd, Windows (NSSM), Docker compose + Caddy, nginx, Render.com, AppImage i `.deb`, skróty okien (Linux `.desktop`, Windows `.ps1`) |
| `tools/sync-zasady.sh` | zasady pakietu VizRS (bloki `pakiet:*` w `CLAUDE.md`, `GEMINI.md`, `docs/AGENCI.md`): to repo jest źródłem; pozostałe repo kopiują je tym skryptem (`--check` w CI) |
| `LICENSE`, `THIRD-PARTY.md` | licencja MIT; składniki zewnętrzne (fonty SIL OFL, crate'y okna i renderera) |
| `docs/` | dokumentacja (spis: `docs/README.md`); `docs/plany/` – plany i raport (dokumenty projektowe, nie opis bieżącego działania) |
| `Dockerfile` | obraz `scratch` ze statyczną binarką musl; wieloarchitekturowy (amd64, arm64 z optymalizacją pod Ampere A1 Neoverse-N1: `-C target-cpu=neoverse-n1 -C opt-level=3` przez `rust-lld` bez emulacji QEMU) |
| `bin/linux-x86_64/vizrs` | gotowa binarka Linux (w repo, używa jej `start-linux.sh`) |
| `vizrs-data/` | dane użytkownika (`show.json`, `assets/`, `backups/`, `log.jsonl`) — w `.gitignore`; przy pustym katalogu serwer wczytuje demo |

## Model w skrócie

Show = `scenes[]` (szablony z Artist) + `pages[]` (strona = numer + scena + `values`) + `playlists[]` + `rundowns[]` + `macros[]`.
Scena ma warstwę (`layer`), długość, stop pointy, drzewo węzłów (`nodes`) z `props`, `anim` (klucze `{t,v,e}`), `fields` (właściwość → pole kontrolne)
i `showIf` (wariant). Szczegóły: [docs/DOKUMENTACJA.md](docs/DOKUMENTACJA.md) §3–4.

## Commity

<!-- pakiet:D -->
- Jedna spójna zmiana = jeden commit, po polsku, opisowo (styl historii repo). Tylko zielone stany (bramka z „Testy”).
- Pliki dodawaj jawnie (`git add ścieżka`), nie `git add -A`. Nie commituj katalogów `target/`, zrzutów, logów, plików tymczasowych, `.env` ani sekretów.
- Commit i push tylko na prośbę użytkownika.
<!-- /pakiet:D -->
- Binarkę `bin/linux-x86_64/vizrs` aktualizuj w commicie, który zmienia UI lub serwer (inaczej `start-linux.sh` uruchamia starą wersję); `bin/linux-x86_64/vizrs-shell` – w commicie zmieniającym `source/shell/`.
