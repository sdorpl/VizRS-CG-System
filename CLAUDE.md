# VizRS CG System

System grafiki telewizyjnej na żywo: **Artist** (projektowanie szablonów) + **Trio** (playout stron) + **wyjście** (`/output`).
Serwer w Rust bez zależności zewnętrznych; grafika renderuje się w przeglądarce jako przezroczysta strona 1920×1080,
którą OBS (albo vMix, Tricaster, ATEM przez Fill/Key) pobiera jako źródło **Przeglądarka**.

Język pracy z użytkownikiem i dokumentacji: **polski**. Kod, komentarze i komunikaty UI też są po polsku — pisz zgodnie z otoczeniem.

## 1. Początek sesji

1. [docs/STATUS.md](docs/STATUS.md) (bieżący etap, następne zadanie), `git log -5`, `git status`.
2. [docs/LUKI.md](docs/LUKI.md) – znane braki: G1 (niezawodność) → G2 (wierność) → G3 (funkcje).
3. Przy zmianie zachowania aplikacji – właściwa sekcja [docs/DOKUMENTACJA.md](docs/DOKUMENTACJA.md) (aktualizujesz ją w tym samym commicie).
4. Plany: tickery – [PLAN-ticker.md](PLAN-ticker.md); większe prace – [PLAN-rozbudowy.md](PLAN-rozbudowy.md); okna aplikacji i VPS/Docker – [PLAN-natywne-i-serwer.md](PLAN-natywne-i-serwer.md).

## 2. Niepodważalne zasady

**Architektura**

- **Zero zależności w rdzeniu.** `source/Cargo.toml` (serwer `vizrs`) nie ma sekcji zależności — własny HTTP/1.1, SSE, JSON, OSC. Nie dodawaj tam crate'ów. Front-end to czysty JS (moduły ES), bez bundlera i bez bibliotek z CDN (wyjście musi działać offline).
- **Wyjątek: okno natywne `source/shell/`** (`vizrs-shell`, decyzja użytkownika 2026-10-20) — osobny workspace z własnym `Cargo.lock`; dozwolone tylko crate'y z nagłówka `source/shell/Cargo.toml` (tao, wry, muda, rfd, png, serde_json, webkit2gtk).
  - Powłoka nie wkompilowuje serwera – uruchamia `vizrs serve --idle-exit` albo dołącza do działającego.
  - Strona nie dostaje dostępu do systemu przez IPC (biała lista komunikatów).
  - Zmiana renderera → `node tools/engines.js` (Chromium vs WebView okna, musi być 0 różnic).
- **UI jest wbudowane w binarkę** (`include_str!` w `source/src/main.rs`, tablica `STATIC`):
  - zmiana w `source/web/*` nie działa w uruchomionym programie, dopóki nie przebudujesz binarki albo nie uruchomisz z `--web source/web`;
  - nowy plik w `web/` trzeba dopisać do `STATIC`;
  - `start-linux.sh` uruchamia `bin/linux-x86_64/vizrs` (plik w repo) — po zmianach UI skopiuj tam `source/target/release/vizrs`.
- **Jeden renderer.** `web/renderer.js` (`ScenePlayer`) rysuje wyjście OBS, podgląd w Trio i kanwę Artist. Nie duplikuj logiki rysowania w Trio/Artist.

**Dane i niezawodność**

- **Dane z zewnątrz są wrogie.** Każdy zapis show przechodzi przez `validate.rs` (limity, typy). Nowe pole w modelu = reguła w `validate.rs` + test.
- **Nie wywalaj serwera:** błędy przez `Result`, blokady odporne na zatrucie (`lk()`), wolny klient SSE jest odłączany. Antena działa dalej, nawet gdy zapis na dysk się nie uda.
- **Import tylko do VizRS.** Elementy z zewnętrznego edytora CG są importowane (`web/btb-io.js`); eksportu z VizRS do innych formatów nie ma i nie przywracamy go. `.cgpack` to własny format VizRS.
- **Wspólny model tekstu:** `web/text-model.js` to kopia zewnętrznego modułu; zgodność pilnują wektory `source/tests/vectors/*.json` i `source/tools/sync-btb.sh --check`. Nie zmieniaj go tylko po jednej stronie.

**Dokumentacja**

- Commit zmieniający zachowanie (UI, skróty, format show, API, opcje CLI) aktualizuje w tym samym commicie:
  - `docs/DOKUMENTACJA.md`, a gdy dotyczy codziennej pracy — także `docs/INSTRUKCJA-poczatkujacy.md`;
  - API → `docs/API.md`;
  - zmiany wydania → `CHANGELOG.md` (sekcja „Niewydane”);
  - zamknięta luka → usunięty wiersz z `docs/LUKI.md`.

## 3. Uruchamianie

```bash
./start-linux.sh                                        # bin/linux-x86_64/vizrs --data ./vizrs-data
cd source && cargo build --release                      # budowa
./target/release/vizrs --data ../vizrs-data --web web   # UI z dysku (bez przebudowy przy zmianach web/)
```

| Adres | Co |
|---|---|
| `http://localhost:8088/` | strona startowa |
| `/artist` | Artist — projektowanie scen |
| `/trio` | Trio — playout |
| `/output` | wyjście dla OBS (przezroczyste) |

Do testów ręcznych uruchamiaj osobny serwer na kopii danych i innym porcie (`--port 8099 --tcp 0 --data <kopia>`), żeby nie ruszać `vizrs-data/` użytkownika.

## 4. Testy (bramka przed commitem)

```bash
cd source
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
node tests/vectors/run.js && node tests/vectors/btb.js && node tests/motion.js   # wektory tekstu, import, ruch
node tests/e2e/run.js target/release/vizrs        # Trio/Output (Playwright, PLAYWRIGHT_DIR=…)
node tests/e2e/artist.js target/release/vizrs     # Artist
node tests/e2e/pack.js target/release/vizrs       # paczka .cgpack
node tests/e2e/motion.js target/release/vizrs     # ruch z presetów
node tests/e2e/app.js target/release/vizrs        # okna aplikacji (udawana przeglądarka)
node tests/e2e/golden.js target/release/vizrs     # zrzuty wzorcowe (lokalnie, zależne od fontów)

# okno natywne
(cd shell && cargo clippy --all-targets -- -D warnings && cargo test && cargo build --release)
node tests/e2e/shell.js target/release/vizrs shell/target/release/vizrs-shell   # w działaniu (otwiera okna)
node tools/engines.js target/release/vizrs shell/target/release/vizrs-shell     # zgodność WebView z Chromium
```

- Playwright nie jest zależnością repo: `PLAYWRIGHT_DIR` wskazuje katalog z `node_modules/playwright` (domyślną ścieżkę ustala `tests/e2e/run.js`).
- Przy pracy nad UI **zobacz wynik** — zrzut ekranu z Playwright (pliki PNG trzymaj poza repo).

## 5. Struktura

**Serwer (`source/src/`)**

| Plik | Rola |
|---|---|
| `main.rs` | serwer HTTP, routing `/api/*`, pliki statyczne (`STATIC`), port TCP, tryb okna (`app_window`) |
| `config.rs` | opcje CLI i zmienne `VIZRS_*` (czysty parser), komendy `serve`/`artist`/`trio`/`output`, `--healthcheck` |
| `engine.rs` | show, warstwy on-air, TAKE/CONTINUE/OUT/UPDATE/ARM, deploy, dock, karuzela, rundown, makra, blokady, SSE |
| `validate.rs` | schemat i limity show (sceny, strony, playlisty, rundowny, makra, `fieldMeta`, presety, Word Replace) |
| `ticker.rs` | zapis wiadomości tickera (`KAT|treść`), komendy `TICKER` |
| `guard.rs` | tokeny (operator, wyjście tylko do odczytu), ochrona przed obcymi witrynami (CSRF), spowalnianie złych prób |
| `app.rs` | okna aplikacji: wyszukiwanie Chromium/Edge, argumenty `--app`, profil, `--connect` |
| `datalock.rs`, `client.rs`, `shutdown.rs` | blokada katalogu danych (`.vizrs.lock`), klient HTTP GET, SIGTERM/Ctrl+C |
| `backup.rs`, `audit.rs` | kopie zapasowe show, log audytowy |
| `osc.rs`, `hook.rs` | OSC (UDP), webhook |
| `http.rs`, `json.rs` | własny parser HTTP/1.1 i JSON |
| `fonts.rs`, `fonts_data.rs` | wbudowane fonty offline (`/fonts/fonts.css`) |
| `bin/vizrs-app.rs` | uruchamianie okien bez konsoli (Windows) |

**Interfejs (`source/web/`)**

| Plik | Rola |
|---|---|
| `renderer.js` | `ScenePlayer`, `PROPS`, `TYPES`, krzywe, pola, autofollow, tekst, ticker, zegar |
| `artist.html` | edytor scen |
| `trio.html` | playout |
| `output.html` | wyjście OBS (`?layers`, `?bus=preview`, `?mode=fill|key`, `?bg`, `?checker`) |
| `text-model.js`, `richtext.js` | reguły pól, typografia PL, Word Replace; znaczniki `{b}{i}{u}{c=}{sz=}` i `[[ ]]` |
| `ticker-model.js`, `ticker-items.js` | model wiadomości tickera; edytor listy wiadomości (Trio i Artist) |
| `motion.js` | ruch z presetów (`node.motion`): krzywe GSAP, presety IN/OUT, efekty tekstu, pętle uwagi, sekwencja `after` – czyste funkcje, rysuje `renderer.js` |
| `cgpack.js` | paczka `.cgpack` (sceny + strony + grafiki + czcionki): zapis ZIP, odczyt, instalacja plików |
| `btb-io.js` | import elementów z zewnętrznego edytora (`.json` v3, `.cgpack`) |
| `demo_show.json` | show startowy (INFO24) |

**Pozostałe**

| Ścieżka | Rola |
|---|---|
| `source/shell/` | `vizrs-shell` – okno natywne Artist/Trio (tao + wry: WebKitGTK / WebView2 / WKWebView), menu, „Zapisz jako”, pisownia pl; osobny workspace z zależnościami |
| `source/tests/` | `vectors/` (wektory tekstu i tickera), `e2e/` (Playwright), `golden/` (zrzuty wzorcowe) |
| `source/tools/` | `build-packs.mjs` (paczki przykładowe), `gallery.js` (galeria zrzutów), `engines.js`, `out_choreo.py`, `render_video.py`, `sync-btb.sh` |
| `examples/` | paczki przykładowe `.cgpack` (lista: `examples/README.md`); `archiwum/` – źródła paczek i pliki używane przez testy |
| `deploy/` | systemd, launchd, Windows (NSSM), Docker compose + Caddy, nginx, skróty okien (Linux `.desktop`, Windows `.ps1`), AppImage |
| `Dockerfile` | obraz `scratch` ze statyczną binarką musl |
| `bin/linux-x86_64/` | gotowe binarki Linux `vizrs` i `vizrs-shell` (w repo, używa ich `start-linux.sh`) |
| `vizrs-data/` | dane użytkownika (`show.json`, `assets/`, `backups/`, `log.jsonl`) — w `.gitignore`; przy pustym katalogu serwer wczytuje demo |

## 6. Model w skrócie

Show = `scenes[]` (szablony z Artist) + `pages[]` (strona = numer + scena + `values`) + `playlists[]` + `rundowns[]` + `macros[]`.
Scena ma warstwę (`layer`), długość, stop pointy, drzewo węzłów (`nodes`) z `props`, `anim` (klucze `{t,v,e}`), `motion` (ruch z presetów),
`fields` (właściwość → pole kontrolne) i `showIf` (wariant). Szczegóły: [docs/DOKUMENTACJA.md](docs/DOKUMENTACJA.md) §3–4.

## 7. Commity

- Jedna spójna zmiana = jeden commit, po polsku, opisowo (np. „Tickery: ciągłość przewijania przy UPDATE”). Tylko zielone stany.
- Pliki dodawaj jawnie (`git add ścieżka`), nie `git add -A`. Nie commituj `source/target/`, zrzutów i plików tymczasowych.
- Binarkę `bin/linux-x86_64/vizrs` aktualizuj w commicie, który zmienia UI lub serwer; `bin/linux-x86_64/vizrs-shell` – w commicie zmieniającym `source/shell/`.
- Commit i push tylko na prośbę użytkownika.
