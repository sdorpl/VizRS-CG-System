# VizRS — grafika CG na żywo dla OBS Studio (2.0)

Lekki klon duetu **Viz Artist** (projektowanie szablonów) + **Viz Trio** (playout stron),
napisany w Rust. Grafika renderuje się jako przezroczysta strona 1920×1080, którą
OBS pobiera przez źródło **Przeglądarka (Browser Source)**.

- Jeden plik wykonywalny, **zero zależności** (własny HTTP/1.1, SSE i JSON) — kompiluje się offline. Opcjonalne okno natywne `vizrs-shell` to osobny program (z zależnościami).
- UI (Artist, Trio, wyjście) jest wbudowane w binarkę.
- Show zapisywany w `vizrs-data/show.json`, pliki graficzne w `vizrs-data/assets/`.

**Dokumentacja:** [instrukcja dla początkujących](docs/INSTRUKCJA-poczatkujacy.md) · [pełna dokumentacja](docs/DOKUMENTACJA.md) · [spis dokumentów](docs/README.md)

## Uruchomienie

```bash
cargo build --release
./target/release/vizrs            # serwer; Windows: target\release\vizrs.exe
./target/release/vizrs trio       # okno aplikacji Trio (Edge/Chrome/Chromium bez pasków) + serwer
./target/release/vizrs artist     # okno Artist (dołącza do działającego serwera)
./target/release/vizrs trio --connect https://cg.example.pl --token …   # okno dla serwera na VPS
```
**Okno natywne** (prawdziwe okno systemowe z menu, WebKitGTK / WebView2 / WKWebView): `vizrs-shell` (Artist) albo `vizrs-shell trio` – osobny program w `source/shell/` (jedyne miejsce z zależnościami; serwer `vizrs` dalej bez nich), gotowy plik `bin/linux-x86_64/vizrs-shell`. [DOKUMENTACJA §13.2](docs/DOKUMENTACJA.md#132-okno-natywne-vizrs-shell).
Okna aplikacji przez Chromium, skróty w menu (Linux/Windows), wyjście na drugi monitor: [DOKUMENTACJA §13.1](docs/DOKUMENTACJA.md#131-okna-aplikacji-artist-trio-output).

| Adres | Co to jest |
|---|---|
| http://localhost:8088/ | strona startowa + dokumentacja API |
| http://localhost:8088/artist | **Artist** – projektowanie scen |
| http://localhost:8088/trio | **Trio** – playout |
| http://localhost:8088/output | **wyjście dla OBS** (przezroczyste) |

Opcje: `--port 8088`, `--tcp 6100` (0 = wyłącz), `--bind 127.0.0.1`, `--token HASŁO`, `--insecure`, `--osc PORT`, `--webhook URL`, `--data katalog`, `--web katalog`, `--healthcheck`, `--version` – każdą można też ustawić zmienną `VIZRS_*` (np. `VIZRS_PORT`, `VIZRS_TOKEN`; pełna lista: `vizrs --help`)
(serwuje pliki UI z dysku zamiast wbudowanych – wygodne przy własnych modyfikacjach `web/`).


## Co nowego w 1.2 – 2.0

| Obszar | Funkcje |
|---|---|
| **Operator (Trio)** | **Rundown** (kolejność przeciąganiem, notatki, stany, **NEXT** F8, import/eksport CSV), **skrót klawiszowy strony** (TAKE/OUT), **ARM** (F6) na wyjście **Preview** (`/output?bus=preview`), **makra** i presety segmentów („⇥ z anteny”), komendy zbiorcze `TAKE 1001,3001`, ⚙ **Stan systemu** (zdrowie, błędy scen, kopie, log, Word Replace) |
| **Pola i tekst** | reguły pola kontrolnego z Artist (maks. znaków, próg ostrzeżenia, linie, jedna linia, WERSALIKI, polska typografia, Tab, podpowiedź, tylko do odczytu), **tekst sformatowany** `{b} {i} {u} {c=#hex} {sz=120}` + `[[wyróżnienie]]` z paskiem formatowania, **Word Replace** (skrót + Ctrl+Spacja), sprawdzanie pisowni przeglądarki (PL), klik w element podglądu → pole |
| **Wyjście** | `?mode=fill` / `?mode=key` (ATEM/Tricaster), `?bus=preview`, wbudowane fonty offline (Oswald, Barlow Condensed, Inter, Roboto, Roboto Condensed, Montserrat), izolacja błędów węzłów, odtwarzanie stanu po ponownym połączeniu bez migotania |
| **Artist** | **powiązania (dock)** scen, markery (→ stop point), zaznaczanie kluczy Shift/ramką, **kopiuj/wklej klucze**, zoom Ctrl+kółko, przeciąganie w drzewie (zmiana rodzica), **biblioteka presetów animacji**, **import z BTB CG** (`.json` v3, `.cgpack`) |
| **Sterowanie** | OSC (`--osc`), webhook (`--webhook`), `/api/variables` dla Companion – patrz [docs/COMPANION.md](docs/COMPANION.md), pełne API: [docs/API.md](docs/API.md) |
| **Niezawodność** | `rev` i konflikty 409, walidacja danych, odroczone wdrożenie zmian szablonu („Wdróż teraz”), blokady edycji wielu operatorów, `/health`, log audytowy – runbook: [docs/RUNBOOK.md](docs/RUNBOOK.md) |

### Wspólny model tekstu z BTB CG
`web/text-model.js` to kopia modułu BTB-CG (typografia PL, reguły pola, Word Replace); jej zgodność pilnują wspólne wektory testowe `source/tests/vectors/*.json`.
Odświeżanie i wykrywanie rozjazdu: `source/tools/sync-btb.sh [--check]`.

### Usługa systemowa i Docker
Pliki w `deploy/` (systemd, launchd, Windows/NSSM). Token w `VIZRS_TOKEN`.
VPS / Docker: `Dockerfile` (statyczna binarka, obraz ~3 MB) i `deploy/docker-compose.yml` (VizRS + Caddy z HTTPS) – krok po kroku w [docs/RUNBOOK.md](docs/RUNBOOK.md#vps-i-docker).

## Bezpieczeństwo (od 1.1)

Domyślnie serwer słucha **tylko lokalnie** (`127.0.0.1`). Żeby OBS na innym komputerze lub telefon z Companion mogły się łączyć,
uruchom z `--bind 0.0.0.0 --token TWOJE_HASŁO` (dozwolone znaki: `A-Z a-z 0-9 - _ .`).

| Kanał | Jak podać token |
|---|---|
| Przeglądarka / OBS | raz otwórz `http://host:8088/trio?token=HASŁO` (zapisze się ciasteczko; w OBS wpisz URL wyjścia z `?token=`) |
| OBS / vMix (tylko odczyt) | `--output-token DRUGIE_HASŁO` i w OBS `…/output?token=DRUGIE_HASŁO` – ten token nie pozwala na TAKE ani edycję |
| HTTP API (Companion, skrypty) | nagłówek `Authorization: Bearer HASŁO`, `X-Token: HASŁO` albo `?token=HASŁO` w URL |
| TCP :6100 | pierwsza linia `AUTH HASŁO` |

Niezależnie od tokenu serwer odrzuca żądania z obcych witryn (`Sec-Fetch-Site`, `Origin`) – strona otwarta w przeglądarce
operatora nie może wywołać TAKE/CLEAR ani nadpisać show. Nagłówki CORS zostały usunięte (UI jest serwowany z tego samego adresu).
Bez tokenu serwer nie wystartuje na adresie sieciowym (`--bind 0.0.0.0`) – chyba że świadomie dodasz `--insecure` (wtedy każdy w sieci może sterować anteną; nie rób tak w produkcji).

## Kopie zapasowe show

Każdy zapis tworzy w `vizrs-data/backups/` kopie: bieżące (ostatnie 20, nie częściej niż co 30 s), godzinowe (24) i dzienne (14), czas UTC.
Zapis idzie przez plik tymczasowy + `fsync`. Gdy zapis się nie uda, Trio pokazuje czerwony znacznik „⚠ zapis show nieudany”,
a API zwraca 500 (`saveError` w `/api/state`).

```
GET  /api/backups                     lista kopii (od najnowszej)
POST /api/backups/restore/<nazwa>     przywróć (stan sprzed przywrócenia trafia do najnowszej kopii bieżącej)
```

## Testy

```bash
cd source
cargo clippy --all-targets -- -D warnings
cargo test                       # silnik, API, parser JSON (w tym testy losowe), ochrona, kopie zapasowe
cargo build --release
node tests/vectors/run.js && node tests/vectors/btb.js           # wspólne wektory tekstu i tickera, konwersja BTB
node tests/e2e/run.js target/release/vizrs                       # E2E Trio/Output (Playwright; PLAYWRIGHT_DIR=katalog z node_modules)
node tests/e2e/artist.js target/release/vizrs                    # E2E Artist
node tests/e2e/golden.js target/release/vizrs [--update]         # zrzuty wzorcowe Outputu (INFO24, R7) – test lokalny
```
CI: `.github/workflows/ci.yml` (Linux/macOS/Windows). `vizrs --version` pokazuje wersję i skrót commita.

## OBS

1. Źródła → **+** → **Przeglądarka**.
2. URL `http://localhost:8088/output`, 1920 × 1080.
3. Zaznacz „Odśwież przeglądarkę, gdy scena staje się aktywna” – po odświeżeniu VizRS
   odtwarza bieżący stan anteny (grafiki stają w swoich stop pointach).
4. Trio możesz dodać jako dock: *Docki → Niestandardowe docki przeglądarki* → `http://localhost:8088/trio`.

Parametry wyjścia: `?layers=front` (tylko wybrane warstwy – osobne źródła w OBS),
`?bg=%2300ff00` (tło chroma), `?w=1280&h=720`, `?checker=1` (podgląd z szachownicą).

## Model (jak w Vizrt)

| Vizrt | VizRS |
|---|---|
| Scena Viz Artist | scena (drzewo kontenerów: grupa, prostokąt, elipsa, tekst, obraz/wideo, zegar, ticker) |
| Keyframe'y / director | ścieżki animacji z krzywymi (inout, out, in, linear, back, hold) |
| Stop points | TAKE → 1. stop, CONTINUE → kolejny stop, OUT → od ostatniego stopu do końca |
| Control Object / Control Text… | ⛓ pole kontrolne na dowolnej właściwości (tekst, kolor, liczba, obraz…) |
| Template / Page (Trio) | szablon = scena, strona = dane + numer (np. 1001) |
| Viz layers (back/main/front) | warstwy `back` < `main` < `front` + dowolne własne nazwy |
| Update | podmiana danych na antenie bez ponownej animacji |
| Carousel | playlista z czasami, w pętli, odtwarzana przez serwer |

## Funkcje dla grafiki newsowej

| Funkcja | Jak użyć |
|---|---|
| **Autofollow** | prostokąt ma pole „śledź węzeł” = nazwa tekstu (lub kilka: `name,title`); szerokość/wysokość dopasowuje się do treści, płynnie także po UPDATE na antenie |
| **Punkt X/Y** | `right` / `bottom` – element rośnie w lewo / w górę (np. bug „NA ŻYWO” przy prawej krawędzi, belka nad tickerem) |
| **Dopasowanie tekstu** | `squeeze` ściska poziomo (do 62%), potem zmniejsza font; `shrink` zmniejsza font; „maks. linii” = 1, 2… |
| **Ściskanie każdego wiersza** | `squeeze-lines` (domyślne w belce): wiersze z Enter albo automatyczny, wyrównany podział; każdy wiersz ściskany osobno do „min. ściśnięcie” (domyślnie 60%), poniżej tego zmniejsza się font tylko tego wiersza. Trio pokazuje ściśnięcie: `w1: 100%  w2: 72%` (pomarańczowe < 100%, czerwone < 70%) |
| **Warianty** | pole wyboru sceny (np. `styl = info, pilne`) + „widoczny gdy” na węźle; w Trio jako przyciski |
| **Wyróżnienie** | `[[tekst]]` w polu tekstowym → fragment na kolorowym tle (np. numer telefonu) |
| **Miganie** | „miganie (okres s)” – np. kropka przy NA ŻYWO |
| **Odliczanie** | zegar w trybie `target` + format `Dn HH:mm:ss` → „73 dni 15:16:13” |
| **Ticker** | wiadomości w osobnych liniach, separator z własnym kolorem |

Demo show „Newsroom INFO24”: 1001–1003 belki (info / pilne / z wyróżnieniem), 1101–1102 podpisy,
2001 bug NA ŻYWO, 2101 odliczanie, 3001 ticker z zegarem, 4101 „pilne” z logo i zegarem, 4001 tytuł, 5001 wynik.
Marka INFO24 jest fikcyjna – logo wgrasz jako plik w polu „Logo” strony 4101.

Istniejący show w `vizrs-data/show.json` nie jest nadpisywany; żeby zobaczyć nowe demo,
uruchom z pustym katalogiem (`--data nowy-katalog`) albo usuń `show.json`.

## Drugi zestaw: R7 NEWS (`examples/R7_NEWS.show.json`)

Kompletna oprawa w innej stylistyce (petrol + bursztyn, ścięte krawędzie): belka info/pilne 1–2 wiersze,
belka z wyróżnieniem, podpis, belka z autorem, ticker z logo i zegarem, bug NA ŻYWO, odliczanie, belka programu.
Wczytaj w Trio: „⇧ Import show” albo uruchom `vizrs --data katalog` z tym plikiem jako `show.json`. Strony 7001–7401.

## Belka BTB wielowierszowa (`examples/BTB_PILNE.show.json`)

„Belka - PILNE” z paczki `examples/BTB_CG` przerobiona na 1–3 wiersze w układzie belek TVP: `squeeze-lines` z łamaniem
**„wypełniaj”** (`lineBreak: fill`) – wiersz bierze kolejne słowa, dopóki mieści się po ściśnięciu najwyżej do 90% (`fillSqueeze`),
reszta przechodzi niżej, każdy wiersz ma własne ściśnięcie, belka rośnie w górę. Strony 8501–8506. Font: skopiuj `examples/BTB_CG/assets/*` do `vizrs-data/assets/`.

Podgląd wszystkich grafik przykładowych: `node source/tools/gallery.js` (zrzuty i `index.html` w `<tmp>/vizrs-galeria`).

## Elementy dodatkowe (w obu zestawach)

| Element | INFO24 | R7 | Opis |
|---|---|---|---|
| Kafel z kodem QR (lewy/prawy) | 1401, 1402 | 7501, 7502 | węzeł typu **Kod QR** – w Trio wpisujesz adres, kod generuje się sam (bez internetu); podpis 1–2 wiersze |
| Belka-hashtag z tłem | 1403 | 7503 | obraz lub wideo w tle paska (pole „Tło”), plakietka dopasowana do tekstu |
| NA ŻYWO z 2-wierszowym miejscem | 1404 | 7504 | Enter w polu „Miejsce” = drugi wiersz, każdy ściskany osobno |
| Rotujący kafel programu | 1405 | 7505 | programy i godziny w kolejnych liniach; właściwość tekstu „rotacja wierszy (s)” zmienia je z przenikaniem |

## Tickery – cztery tryby

| Tryb | INFO24 | R7 | Działanie |
|---|---|---|---|
| `crawl` | 3001, 3004 | 7301, 7304 | klasyczne przewijanie poziome; kategorie jako kolorowe plakietki, opcjonalnie stała etykieta po lewej |
| `roll` | 3002 | 7302 | wiadomości zmieniają się góra-dół; kategoria w osobnym polu, przewija się tylko gdy się zmienia |
| `wipe` | 3003 | 7303 | wiadomość odsłaniana maską z „wycieraczką”, po czasie zmazywana; kategoria zmienia się pionowo |
| `whip` | – | – | wiadomość wchodzi i wychodzi; etykieta przejeżdża przez pasek i w połowie drogi zmienia się na kategorię następnej |

Operator edytuje wiadomości w Trio listą (kategoria + treść, ↑↓, grupy po kategoriach, tekst surowy); najechanie na wiersz pokazuje tę wiadomość w podglądzie.
UPDATE na antenie nie zaczyna przewijania od nowa – nowe wiadomości dochodzą za tym, co jest na ekranie.
Zapis w polu (jedna wiadomość w linii): `KATEGORIA|treść`, z własnym kolorem: `SPORT#1f8a4c|treść`, bez kategorii: sama treść (`\|` = znak `|`).
Właściwości węzła Ticker: tryb, czas wiadomości (s), prędkość (crawl), separator i jego styl (kropka, kwadrat, kreska, skos), kolory kategorii (`PILNE=#e00000; INFO=#0050c0`),
etykieta domyślna (gdy wiadomość nie ma kategorii), szer./tło/tekst kategorii, błysk etykiety. Za długa wiadomość jest ściskana.
Presety tickerów: Artist → ＋ Dodaj… → „Presety tickerów”. Zdalnie: `TICKER 3001 ADD SPORT|Gol` (TCP), `/api/ticker/3001/add?item=…` (HTTP).

## Animacje OUT

Każdy element ma zaprojektowane wyjście: teksty i kody QR gasną z odjazdem (kolejno, od wierzchu),
potem tła zmazują się maską w kierunku zgodnym z wejściem. Ticker i wynik meczu odjeżdżają w całości.
Wyjście możesz edytować w Artist (klucze za ostatnim stop pointem) albo wygenerować ponownie dla całego show:
`python3 tools/out_choreo.py plik.show.json`.

Filmy demonstracyjne: `tools/render_video.py` + `tools/steps_out_info24.py` / `steps_out_r7.py` (IN, potem OUT zwolnione 2,5×).

## Sterowanie zewnętrzne

HTTP (działa też jako GET – np. Bitfocus Companion / Stream Deck):

```
/api/take/1001?name=Jan%20Kowalski&title=Reporter
/api/continue/main
/api/out/main
/api/update/5001?homeScore=3          (&save=0 = bez zapisu)
/api/carousel/start/pl1   /api/carousel/stop
/api/clear
/api/state
```

TCP (port 6100, linia = komenda): `TAKE 1001`, `TAKE 1001 name=Anna|title=Ekspertka`,
`CONTINUE main`, `OUT main`, `UPDATE 5001 homeScore=3`, `LIVE 5001 homeScore=4`,
`CAROUSEL START <playlista>`, `CAROUSEL STOP`, `CLEAR`, `STATE`,
`TICKER 3001 ADD PILNE|Treść`, `TICKER 3001 DEL 2`, `TICKER 3001 LIST`.

## Fonty

Fonty o nazwach z Google Fonts (Oswald, Bebas Neue, Inter…) ładują się automatycznie z internetu.
Do pracy offline wgraj plik `.ttf/.otf/.woff2` w Artist (⤒ przy obrazie lub do assets) i wpisz
w polu „font” jego nazwę pliku, np. `MojFont.ttf`.

## Struktura

```
src/main.rs     serwer HTTP, routing, port TCP
src/engine.rs   show, warstwy, TAKE/CONTINUE/OUT/UPDATE, karuzela, zapis
src/http.rs     parser HTTP/1.1
src/json.rs     parser/serializer JSON
web/renderer.js silnik renderujący (wspólny dla OBS, Trio i Artist)
web/artist.html edytor scen
web/trio.html   playout
web/output.html wyjście OBS
web/demo_show.json  przykładowy show (pasek, bug z zegarem, ticker, tytuł, wynik meczu)
```
