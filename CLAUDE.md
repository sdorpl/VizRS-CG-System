# VizRS CG System

Lekki klon duetu **Viz Artist** (projektowanie szablonów) + **Viz Trio** (playout stron) dla grafiki telewizyjnej na żywo.
Serwer w Rust bez zależności zewnętrznych; grafika renderuje się w przeglądarce jako przezroczysta strona 1920×1080,
którą OBS (albo vMix, Tricaster, ATEM przez Fill/Key) pobiera jako źródło **Przeglądarka**.

Język pracy z użytkownikiem i dokumentacji: **polski**. Kod, komentarze i komunikaty UI też są po polsku — pisz zgodnie z otoczeniem.

## Zacznij każdą sesję tutaj

1. Przeczytaj [docs/STATUS.md](docs/STATUS.md) (bieżący etap, następne zadanie), `git log -5` i `git status`.
2. Przeczytaj [docs/LUKI.md](docs/LUKI.md): znane braki, uszeregowane G1 (niezawodność) → G2 (wierność) → G3 (funkcje).
3. Przy zmianie zachowania aplikacji przeczytaj właściwą sekcję [docs/DOKUMENTACJA.md](docs/DOKUMENTACJA.md) — w tym samym commicie ją zaktualizujesz.
4. Przy tickerach: [PLAN-ticker.md](PLAN-ticker.md); przy większych pracach: [PLAN-rozbudowy.md](PLAN-rozbudowy.md); okna aplikacji i serwer VPS/Docker: [PLAN-natywne-i-serwer.md](PLAN-natywne-i-serwer.md).

## Niepodważalne zasady

- **Tylko BTB → VizRS.** BTB CG (`../BTB-CG-System-alpha`, `../btb-cgeditor`) to baza pomysłów i źródło grafik do importu. Eksportu z VizRS do BTB nie ma i nie przywracamy go.
- **Zero zależności w rdzeniu.** `source/Cargo.toml` (serwer `vizrs`) nie ma sekcji zależności — własny HTTP/1.1, SSE, JSON, OSC. Nie dodawaj tam crate'ów. Front-end to czysty JS (moduły ES), bez bundlera i bez bibliotek z CDN (wyjście musi działać offline).
- **Wyjątek: okno natywne `source/shell/`** (`vizrs-shell`, decyzja użytkownika 2026-10-20) — osobny workspace z własnym `Cargo.lock`; dozwolone tylko crate'y wymienione w nagłówku `source/shell/Cargo.toml` (tao, wry, muda, rfd, png, serde_json, webkit2gtk). Powłoka nie wkompilowuje serwera, tylko uruchamia `vizrs serve --idle-exit` albo dołącza do działającego; nie dawaj stronie dostępu do systemu przez IPC (biała lista komunikatów). Zmiana renderera → `node tools/engines.js` (Chromium vs WebView okna, musi być 0 różnic).
- **UI jest wbudowane w binarkę** (`include_str!` w `source/src/main.rs`, tablica `STATIC`). Wniosek:
  - zmiana w `source/web/*` nie działa w uruchomionym programie, dopóki nie przebudujesz binarki albo nie uruchomisz z `--web source/web`;
  - nowy plik w `web/` trzeba dopisać do `STATIC`;
  - `start-linux.sh` uruchamia `bin/linux-x86_64/vizrs` (plik w repo) — po zmianach UI skopiuj tam `source/target/release/vizrs`.
- **Jeden renderer.** `web/renderer.js` (`ScenePlayer`) renderuje wyjście OBS, podgląd w Trio i kanwę Artist. Nie duplikuj logiki rysowania w Trio/Artist.
- **Dane z zewnątrz są wrogie.** Każdy zapis show przechodzi przez `validate.rs` (limity, typy). Nowe pole w modelu = reguła w `validate.rs` + test.
- **Nie wywalaj serwera:** błędy przez `Result`, blokady odporne na zatrucie (`lk()`), wolny klient SSE jest odłączany. Antena ma działać dalej, nawet gdy zapis na dysk się nie uda.
- **Wspólny model tekstu z BTB:** `web/text-model.js` to kopia modułu BTB-CG; zgodność pilnują wektory `source/tests/vectors/*.json` i `source/tools/sync-btb.sh --check`. Nie zmieniaj go tylko po jednej stronie.
- **Dokumentacja razem z kodem:** commit zmieniający zachowanie (UI, skróty, format show, API, opcje CLI) aktualizuje w tym samym commicie `docs/DOKUMENTACJA.md`, a gdy dotyczy codziennej pracy — także `docs/INSTRUKCJA-poczatkujacy.md`; API → `docs/API.md`; zmiany wydania → `CHANGELOG.md` (sekcja „Niewydane”); zamknięta luka → wiersz z `docs/LUKI.md`.

## Uruchamianie

```bash
./start-linux.sh                                   # bin/linux-x86_64/vizrs --data ./vizrs-data
cd source && cargo build --release                 # budowa
./target/release/vizrs --data ../vizrs-data --web web   # UI z dysku (bez przebudowy przy zmianach web/)
```

| Adres | Co |
|---|---|
| `http://localhost:8088/` | strona startowa |
| `/artist` | Artist — projektowanie scen |
| `/trio` | Trio — playout |
| `/output` | wyjście dla OBS (przezroczyste) |

Do testów ręcznych uruchamiaj osobny serwer na kopii danych i innym porcie (`--port 8099 --tcp 0 --data <kopia>`), żeby nie ruszać `vizrs-data/` użytkownika.

## Testy (bramka przed commitem)

```bash
cd source
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
node tests/vectors/run.js && node tests/vectors/btb.js && node tests/motion.js
node tests/e2e/run.js target/release/vizrs        # Trio/Output (Playwright, PLAYWRIGHT_DIR=…)
node tests/e2e/artist.js target/release/vizrs     # Artist
node tests/e2e/pack.js target/release/vizrs       # paczka .cgpack
node tests/e2e/app.js target/release/vizrs        # okna aplikacji (udawana przeglądarka)
(cd shell && cargo clippy --all-targets -- -D warnings && cargo test && cargo build --release)   # okno natywne
node tests/e2e/shell.js target/release/vizrs shell/target/release/vizrs-shell     # okno natywne w działaniu (otwiera okna)
node tools/engines.js target/release/vizrs shell/target/release/vizrs-shell       # zgodność WebView z Chromium (otwiera okno)
node tests/motion.js && node tests/e2e/motion.js target/release/vizrs   # ruch jak w BTB
node tests/e2e/golden.js target/release/vizrs     # zrzuty wzorcowe (lokalnie, zależne od fontów)
```

Playwright nie jest zależnością repo: `PLAYWRIGHT_DIR` wskazuje katalog z `node_modules/playwright` (domyślnie szukany w `../BTB-CG-System-alpha`).
Przy pracy nad UI **zobacz wynik** — zrzut ekranu z Playwright (pliki PNG trzymaj poza repo).

## Struktura

| Ścieżka | Rola |
|---|---|
| `source/src/main.rs` | serwer HTTP, routing `/api/*`, pliki statyczne (`STATIC`), port TCP, tryb okna (`app_window`) |
| `source/src/config.rs` | opcje CLI i zmienne `VIZRS_*` (czysty parser), komendy `serve`/`artist`/`trio`/`output`, `--healthcheck` |
| `source/src/app.rs` | okna aplikacji: wyszukiwanie Chromium/Edge, argumenty `--app`, profil, `--connect` |
| `source/src/datalock.rs`, `client.rs`, `shutdown.rs` | blokada katalogu danych (`.vizrs.lock`), klient HTTP GET, SIGTERM/Ctrl+C bez zależności |
| `source/src/bin/vizrs-app.rs` | uruchamianie okien bez konsoli (Windows) |
| `source/shell/` | `vizrs-shell` – okno natywne Artist/Trio (tao + wry: WebKitGTK / WebView2 / WKWebView), menu, „Zapisz jako”, pisownia pl; osobny workspace z zależnościami |
| `source/src/engine.rs` | show, warstwy on-air, TAKE/CONTINUE/OUT/UPDATE/ARM, deploy, dock, karuzela, rundown, makra, blokady, SSE |
| `source/src/validate.rs` | schemat i limity show (sceny, strony, playlisty, rundowny, makra, `fieldMeta`, presety, Word Replace) |
| `source/src/ticker.rs` | zapis wiadomości tickera (`KAT|treść`), komendy `TICKER` |
| `source/src/guard.rs` | tokeny (operator, wyjście tylko do odczytu), ochrona przed obcymi witrynami (CSRF), spowalnianie złych prób |
| `source/src/backup.rs`, `audit.rs` | kopie zapasowe show, log audytowy |
| `source/src/osc.rs`, `hook.rs` | OSC (UDP), webhook |
| `source/src/http.rs`, `json.rs` | własny parser HTTP/1.1 i JSON |
| `source/src/fonts.rs`, `fonts_data.rs` | wbudowane fonty offline (`/fonts/fonts.css`) |
| `source/web/renderer.js` | `ScenePlayer`, `PROPS`, `TYPES`, krzywe, pola, autofollow, tekst, ticker, zegar |
| `source/web/artist.html` | edytor scen |
| `source/web/trio.html` | playout |
| `source/web/output.html` | wyjście OBS (`?layers`, `?bus=preview`, `?mode=fill|key`, `?bg`, `?checker`) |
| `source/web/text-model.js`, `richtext.js` | reguły pól, typografia PL, Word Replace; znaczniki `{b}{i}{u}{c=}{sz=}` i `[[ ]]` |
| `source/web/ticker-model.js`, `ticker-items.js` | model wiadomości tickera; edytor listy wiadomości (Trio i Artist) |
| `source/web/btb-io.js` | import elementów BTB CG (`.json` v3, `.cgpack`) |
| `source/web/motion.js` | ruch jak w BTB CG (`node.motion`): krzywe GSAP, presety IN/OUT, efekty tekstu, pętle uwagi, sekwencja `after` – czyste funkcje, rysuje `renderer.js` |
| `source/web/cgpack.js` | paczka belek VizRS `.cgpack` (sceny + strony + grafiki + czcionki): zapis ZIP, odczyt, instalacja plików |
| `source/web/demo_show.json` | show startowy (INFO24) |
| `source/tests/` | `vectors/` (wspólne z BTB), `e2e/` (Playwright), `golden/` (zrzuty wzorcowe) |
| `source/tools/` | `sync-btb.sh`, `build-btb-pack.mjs`, `out_choreo.py`, `render_video.py`, `gallery.js` (galeria zrzutów stron) |
| `examples/` | `R7_NEWS.show.json`, `INFO24.show.json`, paczka `BTB_CG/` (31 wzorców z BTB), `BTB_PILNE.show.json` (belka BTB wielowierszowa) |
| `deploy/` | systemd, launchd, Windows (NSSM), Docker compose + Caddy, nginx, skróty okien (Linux `.desktop`, Windows `.ps1`) |
| `Dockerfile` | obraz `scratch` ze statyczną binarką musl |
| `bin/linux-x86_64/vizrs` | gotowa binarka Linux (w repo, używa jej `start-linux.sh`) |
| `vizrs-data/` | dane użytkownika (`show.json`, `assets/`, `backups/`, `log.jsonl`) — w `.gitignore`; przy pustym katalogu serwer wczytuje demo |

## Model w skrócie

Show = `scenes[]` (szablony z Artist) + `pages[]` (strona = numer + scena + `values`) + `playlists[]` + `rundowns[]` + `macros[]`.
Scena ma warstwę (`layer`), długość, stop pointy, drzewo węzłów (`nodes`) z `props`, `anim` (klucze `{t,v,e}`), `fields` (właściwość → pole kontrolne)
i `showIf` (wariant). Szczegóły: [docs/DOKUMENTACJA.md](docs/DOKUMENTACJA.md) §3–4.

## Commity

- Jedna spójna zmiana = jeden commit, po polsku, opisowo (styl historii: „Tickery na wzór BTB CG: …”). Tylko zielone stany.
- Pliki dodawaj jawnie (`git add ścieżka`), nie `git add -A`. Nie commituj `source/target/`, zrzutów i plików tymczasowych.
- Binarkę `bin/linux-x86_64/vizrs` aktualizuj w commicie, który zmienia UI lub serwer (inaczej `start-linux.sh` uruchamia starą wersję); `bin/linux-x86_64/vizrs-shell` – w commicie zmieniającym `source/shell/`.
- Commit i push tylko na prośbę użytkownika.
