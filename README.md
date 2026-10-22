# VizRS — grafika CG na żywo dla OBS Studio

System grafiki telewizyjnej na żywo, napisany w Rust:

- **Artist** – projektowanie szablonów (scen),
- **Trio** – playout stron (TAKE / CONTINUE / OUT / UPDATE),
- **wyjście** – przezroczysta strona 1920×1080, którą OBS pobiera jako źródło **Przeglądarka** (vMix, Tricaster, ATEM – przez Fill/Key).

Najważniejsze cechy:

- jeden plik wykonywalny, **zero zależności** (własny HTTP/1.1, SSE, JSON, OSC) – kompiluje się i działa offline;
- interfejs (Artist, Trio, wyjście) wbudowany w binarkę;
- opcjonalne okno natywne `vizrs-shell` (osobny program, z zależnościami);
- show w `vizrs-data/show.json`, pliki graficzne w `vizrs-data/assets/`, automatyczne kopie zapasowe.

| Dokument | Zawartość |
|---|---|
| [Instrukcja dla początkujących](docs/INSTRUKCJA-poczatkujacy.md) | pierwsza grafika na antenie, pierwsza belka, OBS, skróty |
| [Pełna dokumentacja](docs/DOKUMENTACJA.md) | model show, węzły, animacja, pola, tekst, ticker, opcje |
| [API](docs/API.md) · [Companion](docs/COMPANION.md) | HTTP, TCP, OSC, webhook, Stream Deck |
| [Runbook](docs/RUNBOOK.md) | awarie, odtwarzanie danych, VPS i Docker |
| [Spis dokumentów](docs/README.md) · [Zmiany](CHANGELOG.md) | |

---

## 1. Uruchomienie

**Gotowa binarka (Linux):**

```bash
./start-linux.sh                  # bin/linux-x86_64/vizrs --data ./vizrs-data
```

**Budowa ze źródeł:**

```bash
cd source
cargo build --release
./target/release/vizrs            # serwer; Windows: target\release\vizrs.exe
./target/release/vizrs trio       # okno aplikacji Trio (Edge/Chrome/Chromium bez pasków) + serwer
./target/release/vizrs artist     # okno Artist (dołącza do działającego serwera)
./target/release/vizrs trio --connect https://cg.example.pl --token …   # okno dla serwera na VPS
```

Windows: `build-windows.bat`, potem `start-windows.bat`. macOS: `./build-macos-linux.sh`.

| Adres | Co to jest |
|---|---|
| http://localhost:8088/ | strona startowa + dokumentacja API |
| http://localhost:8088/artist | **Artist** – projektowanie scen |
| http://localhost:8088/trio | **Trio** – playout |
| http://localhost:8088/output | **wyjście dla OBS** (przezroczyste) |

**Okna:**

- okna aplikacji przez Chromium, skróty w menu (Linux/Windows), wyjście na drugi monitor – [DOKUMENTACJA §13.1](docs/DOKUMENTACJA.md#131-okna-aplikacji-artist-trio-output);
- okno natywne (WebKitGTK / WebView2 / WKWebView, z menu): `vizrs-shell` (Artist) albo `vizrs-shell trio`; kod w `source/shell/`, gotowy plik `bin/linux-x86_64/vizrs-shell` – [DOKUMENTACJA §13.2](docs/DOKUMENTACJA.md#132-okno-natywne-vizrs-shell).

**Najważniejsze opcje** (każdą można ustawić zmienną `VIZRS_*`, np. `VIZRS_PORT`, `VIZRS_TOKEN`; pełna lista: `vizrs --help` i [DOKUMENTACJA §13](docs/DOKUMENTACJA.md#13-uruchamianie-opcje-bezpieczeństwo)):

| Opcja | Domyślnie | Opis |
|---|---|---|
| `--port` | 8088 | port HTTP |
| `--tcp` | 6100 | port komend tekstowych (0 = wyłączony) |
| `--bind` | 127.0.0.1 | adres nasłuchu (sieć wymaga tokenu) |
| `--token` / `--output-token` | – | token operatora / token tylko do odczytu dla OBS |
| `--osc`, `--webhook` | – | port OSC (UDP), adres webhooka |
| `--data` | `vizrs-data` | katalog danych |
| `--web` | – | interfejs z dysku zamiast wbudowanego (przy zmianach w `web/`) |
| `--healthcheck`, `--version` | | sprawdzenie działającego serwera, wersja |

## 2. OBS

1. Źródła → **+** → **Przeglądarka**.
2. URL `http://localhost:8088/output`, 1920 × 1080.
3. Zaznacz „Odśwież przeglądarkę, gdy scena staje się aktywna” – po odświeżeniu VizRS odtwarza bieżący stan anteny (grafiki stają w swoich stop pointach).
4. Trio jako dock: *Docki → Niestandardowe docki przeglądarki* → `http://localhost:8088/trio`.

| Parametr wyjścia | Działanie |
|---|---|
| `?layers=front` | tylko wybrane warstwy (osobne źródła w OBS) |
| `?bg=%2300ff00` | tło chroma |
| `?w=1280&h=720` | inna rozdzielczość |
| `?checker=1` | podgląd z szachownicą |
| `?bus=preview` | wyjście Preview (strony uzbrojone ARM) |
| `?mode=fill` / `?mode=key` | Fill/Key dla ATEM i Tricastera |

## 3. Model

| Pojęcie | W VizRS |
|---|---|
| scena | drzewo kontenerów: grupa, prostokąt, elipsa, tekst, kod QR, obraz/wideo, zegar, ticker |
| animacja | ścieżki kluczy z krzywymi (`inout`, `out`, `in`, `linear`, `back`, `hold`) albo ruch z presetów |
| stop pointy | TAKE → 1. stop, CONTINUE → kolejny stop, OUT → od ostatniego stopu do końca |
| pole kontrolne | ⛓ na dowolnej właściwości (tekst, kolor, liczba, obraz…) |
| strona | szablon = scena, strona = dane + numer (np. 1001) |
| warstwy | `back` < `main` < `front` + dowolne własne nazwy |
| UPDATE | podmiana danych na antenie bez ponownej animacji |
| karuzela | playlista z czasami, w pętli, odtwarzana przez serwer |

## 4. Funkcje

| Obszar | Funkcje |
|---|---|
| **Operator (Trio)** | **rundown** (kolejność przeciąganiem, notatki, stany, **NEXT** F8, import/eksport CSV), **skrót klawiszowy strony**, **ARM** (F6) na wyjście Preview, **makra** i presety segmentów („⇥ z anteny”), komendy zbiorcze `TAKE 1001,3001`, biblioteka show (osobny show na audycję), ⚙ **Stan systemu** (zdrowie, błędy scen, kopie, log, Word Replace) |
| **Pola i tekst** | reguły pola (maks. znaków, próg ostrzeżenia, linie, WERSALIKI, polska typografia, kolejność Tab, podpowiedź, tylko do odczytu), **tekst sformatowany** `{b} {i} {u} {c=#hex} {sz=120}` + `[[wyróżnienie]]`, **Word Replace** (skrót + Ctrl+Spacja), sprawdzanie pisowni (PL), klik w element podglądu → pole |
| **Artist** | **powiązania (dock)** scen, markery, kopiuj/wklej klucze, zoom Ctrl+kółko, przeciąganie w drzewie, **presety animacji**, ruch z presetów bez kluczy, tryb sekwencji, **paczki `.cgpack`** (sceny + strony + grafiki + czcionki) |
| **Wyjście** | Fill/Key, Preview, wbudowane fonty offline (Oswald, Barlow Condensed, Inter, Roboto, Roboto Condensed, Montserrat), izolacja błędów węzłów, odtworzenie stanu po ponownym połączeniu bez migotania |
| **Sterowanie** | HTTP, TCP, OSC, webhook, `/api/variables` dla Companion |
| **Niezawodność** | `rev` i konflikty 409, walidacja danych, odroczone wdrożenie zmian szablonu („Wdróż teraz”), blokady edycji, `/health`, log audytowy, kopie zapasowe |

### Grafika newsowa

| Funkcja | Jak użyć |
|---|---|
| **Autofollow** | prostokąt ma pole „śledź węzeł” = nazwa tekstu (lub kilka: `name,title`); rozmiar dopasowuje się do treści, płynnie także po UPDATE |
| **Punkt X/Y** | `right` / `bottom` – element rośnie w lewo / w górę (bug „NA ŻYWO” przy prawej krawędzi, belka nad tickerem) |
| **Dopasowanie tekstu** | `squeeze` ściska poziomo do `min. ściśnięcie` bez zmiany fontu; `squeeze-shrink` potem zmniejsza font; `shrink` zmniejsza font; „maks. linii” = 1, 2… |
| **Ściskanie każdego wiersza** | `squeeze-lines`: wiersze z Enter albo automatyczny podział; każdy wiersz ściskany osobno. Trio pokazuje ściśnięcie: `w1: 100%  w2: 72%` (pomarańczowe < 100%, czerwone < 70%) |
| **Warianty** | pole wyboru sceny (np. `styl = info, pilne`) + „widoczny gdy” na węźle; w Trio jako przyciski |
| **Wyróżnienie** | `[[tekst]]` → fragment na kolorowym tle (np. numer telefonu) |
| **Miganie** | „miganie (okres s)” – np. kropka przy NA ŻYWO |
| **Odliczanie** | zegar w trybie `target` + format `Dn HH:mm:ss` → „73 dni 15:16:13” |

### Tickery – cztery tryby

| Tryb | INFO24 | R7 | Działanie |
|---|---|---|---|
| `crawl` | 3001, 3004 | 7301, 7304 | przewijanie poziome; kategorie jako kolorowe plakietki, opcjonalnie stała etykieta po lewej |
| `roll` | 3002 | 7302 | wiadomości zmieniają się góra-dół; kategoria przewija się tylko, gdy się zmienia |
| `wipe` | 3003 | 7303 | wiadomość odsłaniana maską z „wycieraczką”, po czasie zmazywana |
| `whip` | – | – | wiadomość wchodzi i wychodzi; etykieta przejeżdża przez pasek i zmienia się na kategorię następnej |

- Operator edytuje wiadomości w Trio listą (kategoria + treść, ↑↓, grupy po kategoriach, tekst surowy); najechanie na wiersz pokazuje wiadomość w podglądzie.
- UPDATE na antenie nie zaczyna przewijania od nowa – nowe wiadomości dochodzą za tym, co jest na ekranie.
- Zapis w polu (jedna wiadomość w linii): `KATEGORIA|treść`, z kolorem `SPORT#1f8a4c|treść`, bez kategorii – sama treść (`\|` = znak `|`).
- Właściwości węzła: tryb, czas wiadomości, prędkość, separator i jego styl, kolory kategorii (`PILNE=#e00000; INFO=#0050c0`), etykieta domyślna, wygląd kategorii, błysk etykiety. Za długa wiadomość jest ściskana.
- Presety: Artist → ＋ Dodaj… → „Presety tickerów”. Zdalnie: `TICKER 3001 ADD SPORT|Gol` (TCP), `/api/ticker/3001/add?item=…` (HTTP).

### Animacje OUT

Każdy element ma zaprojektowane wyjście: teksty i kody QR gasną z odjazdem (kolejno, od wierzchu), potem tła zmazują się maską w kierunku zgodnym z wejściem; ticker i wynik odjeżdżają w całości.
Wyjście edytujesz w Artist (klucze za ostatnim stop pointem) albo generujesz ponownie dla całego show: `python3 source/tools/out_choreo.py plik.show.json`.

## 5. Grafiki przykładowe

**Demo wbudowane – „Newsroom INFO24”** (marka fikcyjna):

| Strony | Grafika |
|---|---|
| 1001–1003 | belki (info / pilne / z wyróżnieniem) |
| 1101–1102 | podpisy |
| 2001, 2101 | bug NA ŻYWO, odliczanie |
| 3001 | ticker z zegarem |
| 4001, 4101 | tytuł, „pilne” z logo (logo wgrasz w polu „Logo”) i zegarem |
| 5001 | wynik |

| Element dodatkowy | INFO24 | R7 | Opis |
|---|---|---|---|
| kafel z kodem QR (lewy/prawy) | 1401, 1402 | 7501, 7502 | węzeł **Kod QR** – w Trio wpisujesz adres, kod generuje się sam (bez internetu) |
| belka-hashtag z tłem | 1403 | 7503 | obraz lub wideo w tle paska, plakietka dopasowana do tekstu |
| NA ŻYWO z 2-wierszowym miejscem | 1404 | 7504 | Enter w polu „Miejsce” = drugi wiersz, każdy ściskany osobno |
| rotujący kafel programu | 1405 | 7505 | programy i godziny w kolejnych liniach, zmieniane z przenikaniem |

Istniejący `vizrs-data/show.json` nie jest nadpisywany – żeby zobaczyć demo, uruchom z pustym katalogiem (`--data nowy-katalog`).

**Paczki `.cgpack`** w `examples/` (m.in. **R7 NEWS** – petrol + bursztyn, strony 7001–7505): w Trio utwórz nowy show (▾), potem Artist → ⇧ Import → plik paczki. Lista: [examples/README.md](examples/README.md).

**Podgląd:** galeria [media/galeria/index.html](media/galeria/index.html) (odświeżenie: `node source/tools/gallery.js`), film ze wszystkimi paczkami [media/vizrs_WSZYSTKIE_PACZKI.mp4](media/vizrs_WSZYSTKIE_PACZKI.mp4) (nagranie: `node source/tools/video.js [vizrs] [--only R7] [--fps 25]`, wymaga ffmpeg).

**Fonty:** wbudowane działają offline; inne nazwy z Google Fonts ładują się z internetu. Do pracy offline wgraj plik `.ttf/.otf/.woff2` do assets i wpisz jego nazwę w polu „font” (np. `MojFont.ttf`). Paczka `.cgpack` przenosi czcionki razem ze scenami.

## 6. Sterowanie zewnętrzne

HTTP (działa też jako GET – Bitfocus Companion / Stream Deck), pełna lista w [docs/API.md](docs/API.md):

```
/api/take/1001?name=Jan%20Kowalski&title=Reporter
/api/continue/main
/api/out/main
/api/update/5001?homeScore=3          (&save=0 = bez zapisu)
/api/carousel/start/pl1   /api/carousel/stop
/api/clear
/api/state
```

TCP (port 6100, linia = komenda): `TAKE 1001`, `TAKE 1001 name=Anna|title=Ekspertka`, `CONTINUE main`, `OUT main`,
`UPDATE 5001 homeScore=3`, `LIVE 5001 homeScore=4`, `CAROUSEL START <playlista>`, `CAROUSEL STOP`, `CLEAR`, `STATE`,
`TICKER 3001 ADD PILNE|Treść`, `TICKER 3001 DEL 2`, `TICKER 3001 LIST`.

## 7. Bezpieczeństwo

Domyślnie serwer słucha **tylko lokalnie** (`127.0.0.1`). Żeby OBS na innym komputerze albo Companion mogły się łączyć,
uruchom z `--bind 0.0.0.0 --token TWOJE_HASŁO` (znaki `A-Z a-z 0-9 - _ .`).

| Kanał | Jak podać token |
|---|---|
| przeglądarka | raz otwórz `http://host:8088/trio?token=HASŁO` (zapisze się ciasteczko) |
| OBS / vMix (tylko odczyt) | `--output-token DRUGIE_HASŁO`, w OBS `…/output?token=DRUGIE_HASŁO` – bez TAKE i edycji |
| HTTP API | `Authorization: Bearer HASŁO`, `X-Token: HASŁO` albo `?token=HASŁO` |
| TCP :6100 | pierwsza linia `AUTH HASŁO` |

- Serwer odrzuca żądania z obcych witryn (`Sec-Fetch-Site`, `Origin`) – strona otwarta w przeglądarce operatora nie wywoła TAKE ani nie nadpisze show.
- Bez tokenu serwer nie wystartuje na adresie sieciowym, chyba że świadomie dodasz `--insecure` (każdy w sieci steruje anteną – nie w produkcji).
- TLS zapewnia reverse proxy (Caddy, nginx) – patrz niżej.

## 8. Dane i kopie zapasowe

Każdy zapis tworzy w `vizrs-data/backups/` kopie: bieżące (ostatnie 20, nie częściej niż co 30 s), godzinowe (24) i dzienne (14), czas UTC.
Zapis idzie przez plik tymczasowy + `fsync`. Gdy się nie uda, Trio pokazuje czerwony znacznik „⚠ zapis show nieudany”, a API zwraca 500 (`saveError` w `/api/state`).

```
GET  /api/backups                     lista kopii (od najnowszej)
POST /api/backups/restore/<nazwa>     przywróć (stan sprzed przywrócenia trafia do najnowszej kopii bieżącej)
```

## 9. Wdrożenie

- **Usługa systemowa:** `deploy/` (systemd, launchd, Windows/NSSM); token w `VIZRS_TOKEN`.
- **VPS / Docker:** `Dockerfile` (statyczna binarka, obraz ~3 MB) i `deploy/docker-compose.yml` (VizRS + Caddy z HTTPS) – krok po kroku w [docs/RUNBOOK.md](docs/RUNBOOK.md#vps-i-docker). Gotowy obraz: `ghcr.io/sdorpl/vizrs`.

### Render.com (plan darmowy)

W repo są `Dockerfile`, `render.yaml` i `deploy/render/start.sh`.

1. Render → **New → Blueprint** → wybierz to repozytorium (plan `free`).
2. Podaj **`VIZRS_TOKEN`** – bez niego usługa nie wystartuje.
3. OBS: `https://<nazwa>.onrender.com/output?token=TOKEN`, Trio: `https://<nazwa>.onrender.com/trio?token=TOKEN`.

Ograniczenia planu darmowego:

- **brak trwałego dysku** – po restarcie, uśpieniu lub deployu show wraca do `deploy/render/seed/show.json` (brak pliku = demo). Gotowy show pobierz (`GET /api/show`), zapisz tam, zacommituj i zrób deploy;
- **usypianie po 15 min bez ruchu** (budzenie ~1 min) – ustaw zewnętrzny ping co 10 min na `https://<nazwa>.onrender.com/` (wystarczy dowolna odpowiedź, nawet 401);
- 750 h pracy miesięcznie na konto; TCP :6100 i OSC niedostępne – Companion przez HTTP.

Do realnej anteny lepszy plan płatny z dyskiem – patrz komentarz w `render.yaml`.

## 10. Dla programistów

```
source/src/main.rs        serwer HTTP, routing /api, pliki statyczne, port TCP
source/src/engine.rs      show, warstwy, TAKE/CONTINUE/OUT/UPDATE, rundown, makra, karuzela, zapis
source/src/validate.rs    schemat i limity show
source/src/http.rs, json.rs   własny parser HTTP/1.1 i JSON
source/web/renderer.js    silnik renderujący (wspólny dla wyjścia, Trio i Artist)
source/web/artist.html    edytor scen
source/web/trio.html      playout
source/web/output.html    wyjście OBS
source/shell/             okno natywne vizrs-shell
```

Pełna struktura i zasady pracy z kodem: [CLAUDE.md](CLAUDE.md).

```bash
cd source
cargo clippy --all-targets -- -D warnings
cargo test                                               # silnik, API, parser JSON, ochrona, kopie
cargo build --release
node tests/vectors/run.js                                # wektory tekstu i tickera
node tests/e2e/run.js target/release/vizrs               # E2E Trio/Output (Playwright, PLAYWRIGHT_DIR=…)
node tests/e2e/artist.js target/release/vizrs            # E2E Artist
node tests/e2e/golden.js target/release/vizrs [--update] # zrzuty wzorcowe (test lokalny)
```

CI: `.github/workflows/ci.yml` (Linux/macOS/Windows). `vizrs --version` pokazuje wersję i skrót commita.
