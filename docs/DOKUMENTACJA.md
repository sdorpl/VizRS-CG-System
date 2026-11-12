# VizRS – dokumentacja aplikacji

> Wersja 2.0.19 + zmiany „Niewydane” z [CHANGELOG.md](../CHANGELOG.md). Dokument opisuje stan z kodu (`source/`).
> Instrukcja krok po kroku dla nowych osób: [INSTRUKCJA-poczatkujacy.md](INSTRUKCJA-poczatkujacy.md).
> API: [API.md](API.md) · Companion: [COMPANION.md](COMPANION.md) · awarie: [RUNBOOK.md](RUNBOOK.md) · braki: [LUKI.md](LUKI.md).

## Spis treści

1. [Czym jest VizRS](#1-czym-jest-vizrs) – w tym [pakiet VizRS i zgodność](#pakiet-vizrs--zgodność-aplikacji)
2. [Architektura](#2-architektura)
3. [Model danych (show)](#3-model-danych-show)
4. [Scena: węzły, właściwości, animacja](#4-scena-węzły-właściwości-animacja)
5. [Pola kontrolne i reguły pól](#5-pola-kontrolne-i-reguły-pól)
6. [Tekst](#6-tekst)
7. [Ticker](#7-ticker)
8. [Artist – projektowanie scen (VizRS CG Artist, zamrożony /artist)](#8-artist--projektowanie-scen)
9. [Trio – playout](#9-trio--playout)
10. [Wyjście dla OBS i mikserów](#10-wyjście-dla-obs-i-mikserów)
11. [Sterowanie zewnętrzne](#11-sterowanie-zewnętrzne)
12. [Uruchamianie, opcje, bezpieczeństwo](#12-uruchamianie-opcje-bezpieczeństwo)
13. [Dane na dysku i kopie zapasowe](#13-dane-na-dysku-i-kopie-zapasowe)
14. [Testy i narzędzia](#14-testy-i-narzędzia)
15. [Znane ograniczenia](#15-znane-ograniczenia)

---

## 1. Czym jest VizRS

VizRS to system grafiki telewizyjnej (CG) na żywo, wzorowany na duecie Vizrt:

| Vizrt | VizRS | Do czego |
|---|---|---|
| Viz Artist | **Artist** (osobny program [VizRS CG Artist](https://github.com/sdorpl/VizRS-CG-Artist); w serwerze zamrożony `/artist`) | projektowanie szablonów: drzewo kontenerów, klucze animacji, stop pointy, pola kontrolne |
| Viz Trio | **Trio** (`/trio`) | playout: strony z danymi, TAKE / CONTINUE / OUT / UPDATE, rundown, playlisty, makra |
| Viz Engine | **wyjście** (`/output`) | przezroczysta strona 1920×1080 w źródle Przeglądarka (OBS) albo Fill/Key dla mikserów |

Typowe grafiki: belki (lower third), podpisy, plakietki NA ŻYWO / miasto, tickery, zegary i odliczania, logo, kody QR, wyniki.

Odpowiedniki pojęć Viz:

| Vizrt | VizRS |
|---|---|
| scena Viz Artist | scena (drzewo kontenerów: grupa, prostokąt, elipsa, tekst, kod QR, obraz/wideo, zegar, ticker) |
| keyframe'y / director | ścieżki animacji z krzywymi `inout`, `out`, `in`, `linear`, `back`, `hold` |
| stop points | TAKE → 1. stop, CONTINUE → kolejny stop, OUT → od ostatniego stopu do końca |
| Control Object / Control Text | ⛓ pole kontrolne na dowolnej właściwości |
| template / page | szablon = scena, strona = dane + numer (np. 1001) |
| layers back/main/front | warstwy `back` < `main` < `front` + własne nazwy |
| Update | podmiana danych na antenie bez ponownej animacji |
| Variants / Concepts | warianty sceny (pole wyboru + „widoczny gdy” na węźle) |
| Carousel | playlista z czasami, w pętli, odtwarzana przez serwer |

### Pakiet VizRS – zgodność aplikacji

VizRS CG System to jedna z czterech aplikacji pakietu VizRS, które mają być **zawsze ze sobą zgodne** (bieżące wersje działają razem).

| Repozytorium | Rola | Styk z tą aplikacją |
|---|---|---|
| **VizRS CG System** (to repo) | silnik grafiki (Rust): Trio (playout stron), wyjście `/output` (też anteny kanałów `bus=ch-N`), API TCP 6100 / HTTP / SSE (opis: `docs/API.md`), serwer `vizrs`; zamrożony zapasowy Artist w przeglądarce | – |
| [VizRS CG Playout](../../VirtRS%20CG%20Playout) (katalog `VirtRS CG Playout`) | grafika VizRS i sloty na żywo na kanałach ErsatzTV (sidecar `cg`, bez zmian w ErsatzTV) | protokół VizRS: komendy TCP 6100 (`AUTH`, `STATE`, `TAKE`, `OUT`…), strona `/output`, renderer `vizrs-render stream` (dostawca styku: VizRS CG System, `docs/API.md`) |
| [VizRS Live](../../VizRS%20Live) | panel kanału nad ErsatzTV legacy (sterowanie kanałami, ramówką, playoutami bez web GUI legacy) | tylko z przeglądarki operatora: ramka `/output` (token wyjścia) i odnośniki do Trio |
| [VizRS CG Artist](../../VizRS%20CG%20Artist) | samodzielny edytor grafik i animacji (Tauri 2), projektuje sceny; działa też bez serwera | wysyłanie scen na serwer `vizrs` (dostawca styku: VizRS CG System) |

Zasady zgodności:

- **Styk zmienia się w obu repozytoriach naraz.** VizRS CG System jest dostawcą protokołu TCP 6100, API HTTP/SSE, wyjścia `/output` i Trio dla pozostałych aplikacji pakietu (opis: `docs/API.md`). Zmiana formatu komend, parametrów, struktury scen lub stron albo zachowania wyjścia po stronie serwera wymaga dostosowania klientów (VizRS CG Playout, VizRS CG Artist, VizRS Live) i ich dokumentacji, zanim wejdzie do wydania.
- **Starsza wersja sąsiada nie psuje serwera ani wyjścia.** Serwer i wyjście zachowują kompatybilność wsteczną – brak nowszych pól, obecność starszych wariantów zapytań czy nieznane parametry nie mogą przerywać pracy silnika ani zakłócać emisji grafiki na żywo.
- **Wydania razem.** Przed wydaniem VizRS CG System sprawdź z bieżącymi wersjami pozostałych aplikacji pakietu; zmianę wymagającą nowszego sąsiada odnotuj w `CHANGELOG.md` z minimalną wersją.

## 2. Architektura

```
            ┌───────────── przeglądarka ─────────────┐
 Artist ──┐ │  renderer.js (ScenePlayer)             │
 Trio  ───┼─┤  ten sam silnik w Artist, Trio, Output │
 Output ──┘ └────────────────────────────────────────┘
      │  HTTP /api/*  ·  SSE /events
      ▼
 vizrs (Rust, jeden plik, zero zależności)
   engine.rs  show + warstwy on-air + karuzela/rundown/makra
   validate.rs  schemat i limity  ·  backup.rs  kopie  ·  audit.rs  log
   TCP :6100  ·  OSC (UDP)  ·  webhook
      │
      ▼
 vizrs-data/ show.json · assets/ · backups/ · log.jsonl
```

- **Serwer** (`source/src/`) trzyma show i stan anteny. Każda komenda (TAKE, OUT…) zmienia stan i jest rozgłaszana zdarzeniem SSE do wszystkich wyjść i paneli. Serwer nie renderuje grafiki.
- **Renderer** (`source/web/renderer.js`, klasa `ScenePlayer`) odtwarza scenę w DOM: liczy wartości właściwości w czasie `t` (klucze + pola strony), układa tekst, autofollow, ticker, zegar. Ten sam kod działa w Artist (kanwa), Trio (podgląd strony) i na wyjściu, więc podgląd = antena.
- **UI jest wbudowane w binarkę** (`include_str!`). Opcja `--web katalog` serwuje pliki z dysku (do modyfikacji bez przebudowy).
- Wyjście po ponownym połączeniu dostaje `snapshot` i odtwarza stan anteny bez migotania: grafiki stają w swoich stop pointach.
- Błąd w jednym węźle sceny nie psuje reszty: węzeł jest ukrywany, a wyjście zgłasza błąd do serwera (`/api/report`, znacznik „⚠ błędy scen” w Trio).

Kolejność warstw na wyjściu: `back` < `main` < inne nazwy < `front`. Warstwy `z<N>_nazwa` (N 0–99, np. `z5_ticker`) układają się nad innymi nazwami wg N (większe wyżej); zawsze pod `front`.

## 3. Model danych (show)

Show to jeden plik JSON (`vizrs-data/show.json`):

```json
{
  "name": "Newsroom INFO24",
  "rev": 42,
  "scenes":    [ { "id": "s_belka", "name": "Belka", "layer": "main", "...": "..." } ],
  "pages":     [ { "id": "p1", "number": 1001, "name": "Belka – budżet", "scene": "s_belka", "values": { "headline": "…" } } ],
  "playlists": [ { "id": "pl1", "name": "Belki", "loop": true, "items": [ { "page": "p1", "duration": 7 } ] } ],
  "rundowns":  [ { "id": "…", "name": "…", "items": [ { "id": "…", "page": "p1", "note": "…" } ] } ],
  "macros":    [ { "id": "…", "name": "…", "steps": [ { "cmd": "TAKE 1001", "delay": 0 } ] } ]
}
```

| Obiekt | Pola | Uwagi |
|---|---|---|
| **scena** | `id`, `name`, `layer`, `width`, `height`, `x`, `y` (położenie całej sceny w px, −16384…16384, domyślnie 0), `duration`, `stops[]`, `markers[]`, `dock[]`, `packAssets[]` (pliki dołączone do paczki .cgpack), `fieldMeta{}`, `nodes[]`, `rev` | warstwa: litery, cyfry, `-`, `_` (maks. 50) |
| **strona** | `id`, `number` (całkowita), `name`, `scene` (id sceny), `values{}`, `hotkey`, `note`, `playout{at, out, ref}`, `rev` | `values` tylko wartości proste (tekst, liczba, tak/nie); `playout` – czas strony na kanale (niżej) |
| **playlista** | `id`, `name`, `loop`, `items[{page, duration}]` | karuzela odtwarzana przez serwer |
| **rundown** | `id`, `name`, `items[{id, page, note}]` | kursor i „odegrane” żyją w pamięci serwera (do restartu) |
| **makro** | `id`, `name`, `steps[{cmd, delay}]` | `cmd` w składni portu TCP; makro nie może wywołać `MACRO` |

**Rewizje i konflikty.** Show i każdy obiekt mają `rev`. Zapis z nieaktualnym `rev` kończy się błędem 409 (Trio pyta: nadpisać czy zachować zmiany). Import całego show (`PUT /api/show`) z innym `rev` wymaga `force=1`.

**Walidacja** (`validate.rs`) odrzuca m.in.: zły typ pola, numer strony niecałkowity, za głębokie drzewo lub za dużo węzłów, czasy kluczy poza 0–3600 s, ticker ponad 200 wiadomości × 500 znaków, `softMax > maxChars`, makro z `MACRO`.

## 4. Scena: węzły, właściwości, animacja

### 4.1 Węzeł

```json
{ "id": "b_info", "type": "rect", "name": "bar_info",
  "props":  { "x": 60, "y": 998, "w": 1800, "h": 90, "anchorY": "bottom", "color": "#141418e8" },
  "anim":   { "clipR": [ { "t": 0, "v": 100, "e": "out" }, { "t": 0.35, "v": 0 } ] },
  "fields": { "text": "headline" },
  "showIf": { "field": "styl", "value": "info" },
  "children": [] }
```

- `props` – wartości stałe; `anim` – ścieżki kluczy (`t` czas w s, `v` wartość, `e` krzywa **do następnego klucza**); ścieżka ma pierwszeństwo przed `props`.
- `fields` – mapa właściwość → nazwa pola kontrolnego; wartość ze strony (`values`) ma pierwszeństwo przed animacją.
- `showIf` – węzeł widoczny tylko, gdy pole wariantu ma jedną z wartości (`"info"` albo `"info,pilne"`).
- `motion` – ruch opisany presetami, bez kluczy (§4.5).
- Kolejność w drzewie: niżej na liście = wyżej na ekranie.

### 4.2 Typy węzłów

| Typ | Etykieta w Artist | Właściwości poza wspólnymi | Dzieci |
|---|---|---|---|
| `group` | Grupa | autofollow | tak |
| `rect` | Prostokąt | kolor, gradient, zaokrąglenie, ramka, cień, autofollow | tak |
| `ellipse` | Elipsa | kolor, gradient, ramka, cień, autofollow | nie |
| `text` | Tekst | tekst, styl tekstu, rotacja wierszy, kolory `[[wyróżnienia]]` | nie |
| `qr` | Kod QR | tekst (adres), kolor, tło kodu, zaokrąglenie, cień | nie |
| `image` | Obraz/wideo | plik, dopasowanie, zaokrąglenie, cień, zapętl wideo, autofollow | nie |
| `clock` | Zegar | tryb, format, start, cel, strefa czasowa, styl tekstu | nie |
| `ticker` | Ticker | wiadomości, tryb, czasy, separator, kategorie, etykieta, styl tekstu | nie |

### 4.3 Właściwości

**Wspólne (Transform, Maska / efekty)** – każdy węzeł:

| Właściwość | Domyślnie | Animowalna | Opis |
|---|---|---|---|
| `x`, `y`, `w`, `h` | 0, 0, 400, 100 | tak | położenie i rozmiar w px sceny |
| `anchorX` / `anchorY` (punkt X/Y) | `left` / `top` | – | `right`/`bottom`: element rośnie w lewo/w górę (bug przy prawej krawędzi, belka nad tickerem) |
| `rotation`, `scale`, `opacity` | 0, 1, 1 | tak | rotacja°, skala, krycie 0–1 |
| `clipL/R/T/B` (maska L/P/G/D %) | 0 | tak | przycięcie z każdej strony – wejścia „wipe” |
| `blur`, `skewX` | 0 | tak | rozmycie, pochylenie° |
| `blink` | 0 | – | miganie z okresem w s (kropka NA ŻYWO) |

**Autofollow** (`rect`, `ellipse`, `group`, `image`): `follow` = nazwa węzła tekstu (lub kilka: `name,title`), `followAxis` `w`/`h`/`wh`, `followPad` (margines, domyślnie 40), `followMin` (minimum). Tło dopasowuje się do treści, płynnie także po UPDATE na antenie.

**Wygląd:** `color`, `color2` + `gradAngle` (gradient), `radius`, `borderColor`, `borderWidth`, `shadow`.

**Zaokrąglenie** (`rect`, `qr`, `image`): `radius` zaokrągla wszystkie rogi. Nadpisania (wartość ujemna, domyślnie `-1` = „jak `radius`”, wszystkie animowalne):
- rogi `radiusTL`, `radiusTR`, `radiusBR`, `radiusBL` (↖ ↗ ↘ ↙) – róg okrągły o tym promieniu; mają pierwszeństwo,
- boki `radiusT`, `radiusR`, `radiusB`, `radiusL` – oba rogi przy boku (`radiusT: 20` = zaokrąglona tylko góra, „zakładka”). Gdy róg ma ustawione oba boki, jest eliptyczny: poziomo wartość boku górnego/dolnego, pionowo lewego/prawego (`radiusT: 80, radiusL: 20` → lewy górny róg 80×20).

Rogi większe niż bok maleją razem jak w CSS (`border-radius`). Renderer natywny (§10.1) stosuje tę samą regułę (`cornerRadii` w `renderer.js`, `radius_of` w `render/src/scene.rs`). Animacja z `-1` przechodzi przez „jak `radius`” – do animacji ustaw na kluczach jawne wartości.

**Tekst** (`text`, `clock`; częściowo `ticker`): `font`, `size`, `weight` (300–900), `italic`, `upper`, `spacing`, `lineHeight`, `align`, `valign`, `textFit`, `maxLines`, `squeezeMin`, `lineBreak`, `fillSqueeze`, `padX`, `hlColor`/`hlText` (kolory `[[wyróżnienia]]`), `cycle` (rotacja wierszy co N s z przenikaniem).

**Kod QR:** `text` (adres), `color`, `qrBg` (tło kodu), `radius`, `shadow`.

**Obraz:** `src` (plik z assets albo adres `http(s)://`), `fitMode` `contain`/`cover`/`fill`, `loop` (wideo `.mp4`/`.webm`/`.mov`).

**Zegar:** `mode` – `time` (godzina), `up` (stoper od `seconds`), `down` (odliczanie od `seconds`), `target` (do daty `target`); `format`:
- tryb `time`: `yyyy MM dd HH H mm ss`;
- tryby licznikowe: `Dn` („3 dni”), `D`, `HH`, `H`, `mm`, `ss`, `S` (dziesiąte);
- tekst w apostrofach jest dosłowny: `D 'dni' HH:mm:ss`.
- `tz` – strefa czasowa (nazwa IANA), domyślnie **`Europe/Warsaw`**, niezależnie od strefy komputera z OBS czy serwera (np. kontener w UTC); `local` = strefa komputera, nieznana nazwa → domyślna. Dotyczy godziny w trybie `time` i daty `target` (czytanej jako czas w tej strefie; jawne przesunięcie `Z`/`+02:00` ma pierwszeństwo).

**Ticker:** patrz [§7](#7-ticker).

### 4.4 Animacja i stop pointy

- Scena ma `duration` i listę `stops`. **TAKE** gra od 0 do pierwszego stopu, **CONTINUE** do kolejnego, **OUT** od ostatniego stopu do końca sceny i zdejmuje grafikę.
- Bez stop pointów: TAKE gra całość i grafika zostaje; OUT ją wycina.
- Krzywe: `inout` (domyślna), `out`, `in`, `linear`, `back` (z przestrzeleniem), `hold` (skok na następnym kluczu).
- Animować można liczby i kolory (kolory mieszane w RGBA); pozostałe właściwości przełączają się w czasie klucza.
- **Markery** (`markers[{t, name}]`) – znaczniki na osi; z markera można utworzyć stop point.
- **Presety animacji** – biblioteka zestawów kluczy (czasy względne) wspólna dla stanowiska (`/api/animpresets`): zapis z węzła, wstawienie od bieżącego czasu, import/eksport pliku.
- **Wyjście (OUT)** jest projektowane w scenie (klucze za ostatnim stop pointem). Dla gotowych zestawów `tools/out_choreo.py` generuje wyjścia całego show: teksty gasną z odjazdem, tła zmazują się maską zgodnie z wejściem.

### 4.5 Ruch bez kluczy (presety)

Obok kluczy węzeł może mieć **ruch opisany presetami** (`node.motion`, moduł `web/motion.js`). Renderer liczy go z czasu sceny, więc działa jednakowo w Artist (przewijanie osi), w podglądzie Trio i na wyjściu OBS.

| Sekcja | Pola | Czas |
|---|---|---|
| `in` – wejście | `type`, `dir` (left/right/top/bottom), `dur`, `delay`, `ease` | od początku sceny (TAKE) |
| `out` – wyjście | jak `in` | od **ostatniego stop pointu** (OUT) |
| `textIn` / `textOut` – efekt tekstu | `type`, `dur`, `delay`, `stagger` (odstęp jednostek, puste = auto: znaki 0,04 s, słowa 0,09 s), `from` (start/end/center/edges) | jak wejście / wyjście |
| `loop` – pętla uwagi | `type`, `dur` (okres), `amp` (siła), `color` | zegar ścienny, gdy grafika stoi (po wejściu, przed wyjściem) |
| `after` | id węzła | wejście zaczyna się po wejściu wskazanego węzła (sekwencja) |
| `origin` | `[x, y]` (0–1) | punkt odniesienia skali/obrotu ruchu, gdy preset go nie ustala (Artist: „punkt ruchu”) |
| `bg` | `shimmer`, `pulse`, `glow`, `scanline` | stały efekt tła kształtu (Artist: „efekt tła”) – gra od wejścia na wizję, także w trakcie IN/OUT |

- **Presety IN/OUT (21):** wjazd, przenikanie, zoom, zoom z bliska, wyskok, przesłony pozioma i pionowa, przycięcie, rozmycie, odwrócenia X/Y (3D), obrót, skos, spadanie, glitch, wjazd z przeskokiem, rozsunięcie, maska ukośna, żaluzja, rosnący pasek, rozdzielenie RGB.
- **Efekty tekstu (30 na wejściu, 26 na wyjściu):** (maski `inset`, rozmycie 20 px całości i 14 px jednostek, przesunięcia 50/40 px): całość (odsłonięcie, maszyna do pisania, maszyna z kursorem, z maski, przenikanie, rozmycie, skala, przesunięcia), **słowa / znaki / linie** (w górę, w dół, przenikanie, rozmycie, skala, z maski), szyfrowanie (scramble), licznik (pierwsza liczba w tekście liczy od 0 z zachowaniem formatu, np. „1 250,5”). Wyjście ma te same grupy bez odsłonięcia, maszyny do pisania, szyfrowania i licznika; zamiast `clip-up` są `clip-down` i `hide`. Tekst jest dzielony tylko na czas efektu – potem wraca dokładny układ (ściskanie, łamanie wierszy).
- **Pętle uwagi (11):** pulsowanie, oddychanie, bicie serca, unoszenie, podskakiwanie, chwianie, drżenie, błyski, poświata, połysk, przesuwane podkreślenie. Wchodzą płynnie po wejściu i schodzą przed wyjściem; siła: pulsowanie/bicie serca – % skali, chwianie – stopnie, oddychanie – % krycia, unoszenie/podskakiwanie/drżenie – px.
- **Krzywe (nazwy GSAP):** `power2.out` (domyślna), `power2.in`, `power2.inOut`, `power3.out`, `power4.out`, `expo.out`, `sine.inOut`, `linear`, `back.out`, `back.in`, `bounce.out`, `elastic.out` oraz aliasy `spring`, `overshoot`, `broadcast`; przyjmowane są też `back.out(2)` i `cubic-bezier(…)`.
- **Składanie z kluczami:** przesunięcie, obrót, pochylenie i rozmycie się sumują, skala i krycie mnożą, maski dają część wspólną.
- Wyjście gra tylko, gdy scena ma stop point i czas po nim. Artist podpowiada **⟲ Dopasuj stop i długość** (stop po najdłuższym wejściu, długość = stop + najdłuższe wyjście).
- Walidacja (`validate.rs`): typy z list, kierunki, czasy 0–60 s, krzywa tylko z liter, cyfr i `.(),-`, nieznane pola odrzucane.

### 4.6 Warianty

Pole wyboru na poziomie sceny (`fieldMeta.<pole> = {type: "select", options: [...], default}`), np. `styl = info, pilne`. Węzły mają „widoczny gdy” (`showIf`). W Trio wariant to rząd przycisków.

### 4.7 Powiązania (dock)

`scene.dock = [{ target, dx, dy, dur }]` – scena przesuwa się o (`dx`, `dy`) w czasie `dur` s, gdy na antenie jest wskazana grafika:
`layer:<warstwa>`, `page:<numer>` albo `scene:<nazwa>`; kilka celów rozdzielonych `|` (np. `page:15001|page:15002`) – przesunięcie raz, gdy na antenie jest którykolwiek. Przykład: belka unosi się o 90 px, gdy na warstwie `back` jest ticker.
Operator może wyłączyć powiązanie na antenie (⇅ w Trio, `/api/dock/<warstwa>/off`). Stan jest odtwarzany po odświeżeniu wyjścia.

### 4.8 Fonty

- Wbudowane offline: **Oswald, Barlow Condensed, Barlow, Bebas Neue, Inter, Roboto, Roboto Condensed, Roboto Mono, Montserrat** (`/fonts/fonts.css`). Barlow, Bebas Neue i Roboto Mono (używane w INFO24) doszły w gałęzi `render-native`: szablony z nimi nie potrzebują już internetu, a renderer natywny (§10.1) rysuje je sam.
- Fonty systemowe (Arial, Verdana…) – bez ładowania.
- Inne nazwy z Google Fonts ładują się z internetu (na antenie lepiej tego unikać).
- Własny font: wgraj `.ttf/.otf/.woff/.woff2` do assets i wpisz w polu „font” nazwę pliku, np. `MojFont.ttf`. Font wgrany, gdy Trio lub wyjście są już otwarte, wczyta się sam w ciągu kilkunastu sekund (bez odświeżania strony).
- **Font jedzie z serwerem, nie z komputerem OBS:** wyjście `/output` pobiera wszystkie fonty z assets z serwera VizRS zaraz po otwarciu (i sprawdza nowe co 30 s), więc komputer renderujący nie musi mieć fontu zainstalowanego, a pierwszy TAKE nie czeka na pobranie. Fonty przenosi między stanowiskami paczka belek `.cgpack` (§8.7).

## 5. Pola kontrolne i reguły pól

**Pole kontrolne** (⛓ przy właściwości w Artist) wystawia właściwość węzła operatorowi Trio. Wiele węzłów może używać tego samego pola (jedna wartość zasila kilka miejsc).
Kolejność pól w Trio: najpierw warianty, potem w kolejności drzewa (albo wg `tab`).

`fieldMeta.<pole>` – etykieta i reguły (Artist → ustawienia sceny → Pola kontrolne → ⚙):

| Reguła | Klucz | Działanie w Trio |
|---|---|---|
| etykieta | `label` | nazwa pola w edytorze strony |
| podpowiedź | `help` | tekst pod etykietą |
| maks. znaków | `maxChars` | twardy limit (znaczniki formatowania się nie liczą) |
| próg ostrzeżenia | `softMax` | licznik pomarańczowy (domyślnie 85% limitu) |
| maks. linii | `maxLines` | Enter tworzy nowy wiersz do limitu |
| jedna linia | `oneLine` | pole jednowierszowe |
| WERSALIKI | `upper` | zamiana na wielkie litery |
| polska typografia | `typo` | twarda spacja po jednoliterowych spójnikach i przyimkach, cudzysłowy „”, `...` → `…`, ` - ` → półpauza (przy zatwierdzeniu pola) |
| kolejność Tab | `tab` | jak „Field identifier” w Viz Trio |
| tylko do odczytu | `readonly` | operator nie zmienia wartości |
| pasek formatowania | `rich` (domyślnie tak) | przyciski B / I / U / kolor / rozmiar / `[[ ]]` |
| sprawdzanie pisowni | `spell` (domyślnie tak) | pisownia przeglądarki (`lang=pl`) |
| ticker: maks. wiadomości, dozwolone kategorie | `maxItems`, `cats` | pilnuje edytor listy wiadomości |

## 6. Tekst

### 6.1 Dopasowanie (`textFit`)

| Tryb | Działanie |
|---|---|
| `squeeze` | ściska tekst poziomo, najwyżej do `squeezeMin` (domyślnie 60%); **font się nie zmienia**. Gdy to za mało, tekst wystaje poza ramkę, a Trio ostrzega. Domyślny tryb nowego tekstu w Artist |
| `squeeze-shrink` | ściska do `squeezeMin`, a dopiero potem zmniejsza font (dla wąskich, stałych pól: „NA ŻYWO”, etykiety) |
| `squeeze-lines` | każdy wiersz osobno (z Enter albo automatyczny, wyrównany podział): ściskany do `squeezeMin`, poniżej zmniejsza się font tylko tego wiersza |
| `squeeze-each` | wiersze wpisuje operator (Enter, do `maxLines`); **każdy wiersz ściskany osobno** do `squeezeMin`, font i wysokość wierszy bez zmian (belka dwuwierszowa jak w TV: różna szerokość liter, ta sama wysokość). Wiersz za długi nawet przy granicy jest dzielony, gdy zostało miejsce na kolejny; inaczej wystaje, a Trio ostrzega |
| `shrink` | zmniejsza font, aż tekst się zmieści (`maxLines` linii); domyślny dla tekstów bez ustawionego trybu (starsze sceny) |
| `none` | bez dopasowania |

`squeezeMin` (0,3–1) to najmocniejsze dozwolone ściśnięcie: 0,6 = litery zwężone najwyżej do 60% szerokości.

**Łamanie wierszy** (`lineBreak`, tryby `squeeze-lines` i `squeeze-each`) – jak tekst dzieli się sam, gdy nie mieści się w wierszu:

| Wartość | Działanie |
|---|---|
| `balance` (domyślnie) | wiersze o możliwie równej szerokości |
| `fill` („wypełniaj”, jak belki TVP) | wiersz bierze kolejne słowa, dopóki mieści się po ściśnięciu najwyżej do `fillSqueeze` (domyślnie 0,9); reszta przechodzi niżej, każdy wiersz ma własne ściśnięcie. Gdy tekst nie mieści się w `maxLines` wierszach, próg spada stopniowo do `squeezeMin` |

Twarda spacja (reguła „polska typografia”: „i w z…”) nigdy nie jest miejscem podziału – jednoliterowy spójnik nie zostaje sam na końcu wiersza.

Trio pokazuje pod polem ściśnięcie tekstu: `ściśnięcie: 72%` (tryby `squeeze`, `squeeze-shrink`) albo każdego wiersza `w1: 100%  w2: 72%` (`squeeze-lines`, `squeeze-each`); pomarańczowe < 100%, czerwone < 70%. W trybach `squeeze` i `squeeze-each` po przekroczeniu granicy pojawia się czerwone **⚠ nie mieści się – skróć tekst**.

To samo widać w Artist: pod „min. ściśnięcie” zaznaczonego tekstu (`ściśnięcie: N%` / `w1 … w2 …`, ⚠ przy przepełnieniu), a przepełniony tekst ma na kanwie czerwoną przerywaną ramkę. Wyjście programu (`/output`, nie `?bus=preview`) zgłasza przepełnienie grafiki na antenie do `/api/errors` – w Trio jako „⚠ błędy scen” z treścią tekstu.

### 6.2 Tekst sformatowany

Wartość pola pozostaje zwykłym tekstem ze znacznikami (`web/richtext.js`):

| Znacznik | Efekt |
|---|---|
| `[[tekst]]` | wyróżnienie – kolorowe tło (`hlColor`) i tekst (`hlText`), np. numer telefonu |
| `{b}…{/b}`, `{i}…{/i}`, `{u}…{/u}` | pogrubienie, kursywa, podkreślenie |
| `{c=#e30613}…{/c}` | kolor |
| `{sz=120}…{/sz}` | rozmiar w % (30–300) |

Znaczniki można zagnieżdżać, niedomknięte zamykają się na końcu; w `squeeze-lines` przechodzą między wierszami.

### 6.3 Word Replace i pisownia

Lista zamian słów stanowiska (`{"@p": ["Prezydent"]}`, klucz 1–8 znaków): w polu tekstowym wpisz skrót i naciśnij **Ctrl+Spacja**. Lista: Trio → ⚙ Stan → Word Replace albo `/api/wordreplace`.
Sprawdzanie pisowni to natywna funkcja przeglądarki (w doku OBS zależy od CEF).

## 7. Ticker

Węzeł `ticker` – jedna wiadomość w linii pola tekstowego:

```
KATEGORIA|treść
SPORT#1f8a4c|treść z własnym kolorem kategorii
sama treść (bez kategorii)        \|  = znak |
```

Kolor plakietki: kolor wiadomości > kolor tej kategorii na liście (pierwszy podany; wiadomości kategorii bez `#kolor` go przejmują, także te dodane przez `TICKER ADD`) > mapa sceny `catColors` > `catBg`. Wiadomość bez kategorii ma kolor etykiety `label` z tych samych źródeł, a jej `#kolor` (`#c00|treść`) koloruje też stałą etykietę crawla. Kolor: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`.

W edytorze wiadomości (Trio, Artist) kwadrat przy kategorii otwiera wybór koloru: pole barwy, odcień, `#hex`, próbki (w użyciu, paleta, ostatnie), podgląd plakietki z ostrzeżeniem o słabym kontraście i **ze sceny** (usuwa własny kolor). Wybór obowiązuje wszystkie wiadomości kategorii na stronie; kolor równy scenowemu nie jest zapisywany. Zmiana nazwy kategorii przejmuje kolor kategorii docelowej. Wiadomość bez kategorii ustawia kolor etykiety `label`; gdy etykiety też nie ma, plakietki nie ma, a kwadrat przenosi do pola kategorii.

| Tryb (`tickerMode`) | Działanie |
|---|---|
| `crawl` | klasyczne przewijanie poziome (`speed` px/s, `crawlFrom` `right`/`left`); kategorie jako plakietki, opcjonalnie stała etykieta po lewej |
| `roll` | wiadomości zmieniają się góra–dół co `hold` s; kategoria w osobnym polu |
| `wipe` | wiadomość odsłaniana maską z „wycieraczką”, po czasie zmazywana |
| `whip` | wiadomość wchodzi i wychodzi; etykieta przejeżdża przez pasek i w połowie zmienia się na kategorię następnej |

Pozostałe właściwości: `sep` + `sepStyle` (tekst, kropka, kwadrat, kreska, skos) + `sepColor`, `catColors` (`PILNE=#e00000; INFO=#0050c0`), `label` (etykieta domyślna), `catW`, `catBg`, `catFg`, `labelSize`, `labelWeight`, `gleam` (błysk etykiety co N s).

Za długa wiadomość w `roll`/`wipe`/`whip` jest dopasowywana wg `textFit` i `squeezeMin` węzła: domyślnie (`squeeze-shrink`, 60%) ściskana, a poniżej granicy dostaje mniejszy font; `squeeze` – tylko ściskanie, reszta ucięta na krawędzi paska; `shrink` – mniejszy font; `none` – bez dopasowania. Edytor wiadomości pokazuje przy każdej wiadomości potrzebne ściśnięcie, a ⚠ na czerwono, gdy wiadomość zostanie ucięta.

**Ciągłość przy UPDATE:** zmiana wiadomości na antenie nie zaczyna przewijania od nowa – crawl płynie dalej, nowe wiadomości dochodzą za tym, co jest na ekranie; w `roll`/`wipe`/`whip` bieżąca wiadomość dogrywa swój czas.

**Edytor wiadomości** (Trio i Artist): lista *kategoria + treść* z ↑↓ i ✕, widok grup po kategoriach (przeciąganie między grupami), tekst surowy. Enter = nowa wiadomość, Ctrl+↑↓ = przesuń, wklejenie wielu wierszy = wiele wiadomości. Najechanie na wiersz pokazuje tę wiadomość w podglądzie.

**Presety:** Artist → ＋ Dodaj… → „Presety tickerów” (pełny ekran, pasek z wiperem, crawl z linią akcentu, boczny).

**Zdalnie:** `TICKER 3001 ADD SPORT|Gol` (TCP), `/api/ticker/3001/add?item=…` (HTTP), `/vizrs/ticker …` (OSC). Zmiana zapisuje stronę i – gdy jest na antenie – aktualizuje ją bez restartu przewijania.

## 8. Artist – projektowanie scen

> **Projektowanie scen:** od wersji 0.3.0 (2026-11-07) do projektowania i edycji szablonów służy osobny program **[VizRS CG Artist](https://github.com/sdorpl/VizRS-CG-Artist)** (`vizrs-cg-artist`, Tauri 2 dla systemów Linux, Windows i macOS; repozytorium: https://github.com/sdorpl/VizRS-CG-Artist). Aplikacja działa bez serwera (zapis/odczyt plików `.cgpack` i `.show.json`), a gotowe sceny wdraża bezpośrednio na serwer vizrs.
>
> Poniższy opis dotyczy **zamrożonego edytora `/artist`** serwowanego przez serwer VizRS jako zapas do pracy w przeglądarce (np. na zdalnym VPS bez instalacji aplikacji, zachowany do wydania 3.0). Nowe funkcje projektowania scen powstają wyłącznie w VizRS CG Artist.

### 8.1 Układ

| Obszar | Zawartość |
|---|---|
| Pasek górny | wybór sceny (nazwa · warstwa), ＋ Nowa, ⧉ duplikuj, kosz – usuń, ⇩ Eksport (bieżąca scena jako paczka `.cgpack`), ⇩ Paczka (wybrane sceny w paczce `.cgpack`, §8.7), ⇧ Import (`.cgpack`, dawny `.vizrs.json`), ↶ ↷, ▢ Safe (obszary bezpieczne 5% / 10%), **Zapisz**. W przeglądarce widoczne są zakładki i pasek górny; okno natywne `vizrs-shell` służy do pracy w Trio i podglądu wyjścia, a Artist do projektowania scen to VizRS CG Artist |
| Położenie sceny | przesuwa całą scenę naraz (`scene.x`, `scene.y`): pola „Położenie sceny” w ustawieniach sceny (⟲ zeruje), przeciągnięcie pustego miejsca kanwy albo strzałki (Shift = 10 px), gdy zaznaczona jest scena. Nie zmienia kluczy animacji ani `props` węzłów – animacje grają względem nowego położenia; to, co wychodzi poza kadr, jest przycinane. Działa na wyjściu, w podglądzie Trio i w rendererze natywnym; `dock` dodaje się do niego |
| Drzewo sceny (lewo) | ＋ Dodaj… (typy węzłów i presety tickerów; przy zaznaczonej **grupie** nowy element trafia do niej, przy elemencie w grupie – obok niego, przy elemencie na najwyższym poziomie albo bez zaznaczenia – na najwyższy poziom), ↑ ↓ (kolejność), ⇤ wyjmij z grupy, ⇥ włóż do poprzedniej grupy, ⧉, ⤢ dopasuj wymiar grupy do zawartości, kosz; przeciąganie zmienia rodzica z zachowaniem położenia. 🔒/🔓 w wierszu blokuje element (`locked`, tak/nie) – zablokowanego, ani elementów wewnątrz zablokowanej grupy, nie da się chwycić na kanwie (klik trafia w element pod spodem), przesunąć strzałkami, przeciągnąć w drzewie ani usunąć; właściwości nadal edytuje się w panelu. Blokada działa tylko w Artist, nie wpływa na wyjście. **Wiersz sceny** (pierwszy, nadrzędny) pokazuje położenie całej sceny (X, Y), gdy różni się od 0. **Wymiar grupy** ustawia się sam, gdy grupa dostaje element (dodanie, ⇥, przeciągnięcie): szerokość i wysokość = zasięg zawartości, czyli największy element z jego przesunięciem w grupie |
| Kanwa (środek) | podgląd sceny w czasie bieżącym; zaznaczanie i przeciąganie elementów; kliknięcie pustego miejsca = ustawienia sceny. **Zoom podglądu** (10–800%): kółko wokół kursora, −/＋, ⤢ dopasuj (Shift+1), 1:1 (Shift+2), ⌖ kadr na zaznaczony element (Shift+3); środkowy przycisk przesuwa widok. Zmiana rozmiaru panelu zachowuje zoom, zmiana sceny lub jej rozdzielczości wraca do dopasowania |
| Właściwości (prawo) | trzy zakładki: **Właściwości** (grupy Transform, Maska / efekty, Autofollow, Wygląd, Tekst, Obraz, Zegar, Ticker), **Ruch** (ruch bez kluczy, presety animacji, lista kluczy węzła), **Dane** („widoczny gdy”, pola kontrolne węzła); kropka na zakładce = coś jest ustawione. Szczegóły panelu: poniżej |
| Oś czasu (dół) | pasek: czas, ⏮, ▶ Take, ⏭ Continue, ⏏ Out, ▶/⏸, ＋ Stop, Dł. (długość sceny), ≣ Sekwencja, ▾ marker (M), ⧉ / ⎘ kopiuj / wklej klucze, suwak powiększenia, ↔ dopasuj. Pod nim stały **pasek klucza** (czas, wartość, krzywa i usunięcie zaznaczonego klucza), a w trybie sekwencji w tym miejscu **pasek sekwencji** (§8.4) – oś czasu nie przeskakuje |
| Pasek stanu (dół okna) | podpowiedź ze skrótami panelu pod kursorem; wskaźnik **połączono / brak połączenia** z serwerem |

Panele są zadokowane jak w programie desktopowym: szerokość drzewa i właściwości oraz wysokość osi czasu zmienia się przeciągnięciem rozdzielacza (dwuklik = rozmiar domyślny); rozmiary są zapamiętane w przeglądarce / oknie. Pasek osi czasu mieści się w jednym wierszu od szerokości strony 1280 px (np. ekran Full HD przy skali 150%, 3K przy 200%). Interfejs nie zaznacza tekstu poza polami edycji; w oknie natywnym prawy przycisk nie otwiera menu przeglądarki (w polach tekstowych zostaje: kopiuj, wklej, pisownia).

### 8.2 Ustawienia sceny (kliknij puste tło kanwy)

Nazwa, **warstwa**, rozdzielczość, długość, stop pointy (× usuwa), markery (⏹ = stop point z markera), **powiązania (dock)**, **warianty** (＋ Wariant), **pola kontrolne** (etykieta w Trio + ⚙ reguły), **zasoby paczki** (grafiki i czcionki, §8.7) i **＋ Utwórz stronę w Trio**.

**Panel Właściwości** (zaznaczony węzeł):

- **Grupy zwijane** – klik w nagłówek; stan zapamiętany w przeglądarce (`vizrs.propsUI`), wspólny dla wszystkich węzłów. Domyślnie zwinięte: Maska / efekty, Autofollow. Nagłówek pokazuje licznik: `N zm.` (wartości inne niż domyślne), `◆N` (animowane), `⛓N` (pola kontrolne).
- **Podstawowe / Wszystko** – w trybie podstawowym rzadkie właściwości (pochylenie, miganie, rozmycie, maski, cień, ściśnięcia, łamanie wierszy, wyróżnienia `[[ ]]`, rotacja wierszy, część tickera i zegara) są ukryte, chyba że mają wartość inną niż domyślna, klucze albo pole kontrolne. Przycisk „＋ N rzadziej używanych” na końcu listy przełącza na wszystko.
- **Właściwości zależne** są ukryte, gdy nic nie robią: min. ściśnięcie (tylko przy dopasowaniu `squeeze…`), ściśnięcie przy wypełnianiu (łamanie `fill`), kąt gradientu (gdy jest kolor 2), kolor ramki (ramka > 0), oś / margines / minimum autofollow (gdy jest `follow`), start i cel zegara (wg trybu), zapętlenie (plik wideo). Użyta, a nieaktywna właściwość zostaje widoczna, przygaszona.
- **Wiersze złożone** – X/Y, szer./wys. (⛓ = zachowaj proporcje), obrót/skala/krycie, maska L P G D, kolor/kolor 2/kąt, font/grubość, rozmiar/odstęp/interlinia w jednej linii; **punkt** zaczepienia jako siatka 3×3; **wyrównanie** tekstu jako przyciski; kursywa i WERSALIKI obok siebie. Każde pole ma własny ◆ i łańcuch.
- **Stan wiersza** – pogrubiona etykieta z kropką = wartość inna niż domyślna; pomarańczowy pasek z lewej = animowana; niebieski = pole kontrolne. **Prawy klik na etykiecie** przywraca wartość domyślną (gdy właściwość nie ma kluczy). ◆ i łańcuch są przygaszone, dopóki wiersz nie jest wskazany albo aktywny.
- **Szukaj** (`/`) – filtruje właściwości po nazwie polskiej i kluczu (np. „odstęp”, `spacing`), rozwija zwinięte grupy; Esc czyści.

Zakładka **Ruch** zawiera grupę **Ruch – bez kluczy**: wejście, wyjście, tekst IN/OUT (tekst i zegar), pętla – z czasem, opóźnieniem, kierunkiem, krzywą, odstępem i kolejnością jednostek (§4.5).

### 8.3 Klucze animacji

- **◆** przy właściwości = klucz w bieżącym czasie. Gdy właściwość ma już animację, zmiana wartości tworzy lub aktualizuje klucz automatycznie.
- Dwuklik na ścieżce = nowy klucz. Klik w klucz → pasek klucza: czas, wartość, krzywa do następnego klucza, usunięcie.
- Zaznaczanie wielu kluczy: Shift+klik albo Shift+przeciągnięcie ramką; Ctrl+C / Ctrl+V kopiuje je na zaznaczony węzeł od bieżącego czasu.
- Stop point: ＋ Stop w bieżącym czasie; ▼ na linijce przeciągasz, dwuklik usuwa.
- Marker: **M** albo dwuklik na linijce; klik = skok, dwuklik = zmiana nazwy / usunięcie.
- Zoom osi: suwak, Ctrl+kółko, ↔ dopasuj.

### 8.4 Tryb sekwencji (prosty)

Przycisk **≣ Sekwencja** na osi czasu przełącza widok kluczy na widok ruchu bez kluczy (§4.5). Wybór jest pamiętany na stanowisku.

- **Wiersz na element** (z wcięciem wg drzewa): niebieski pasek – wejście, pomarańczowy – wyjście (za ostatnim stop pointem), cienkie paski pod nimi – efekt tekstu IN/OUT, kreskowane tło – pętla uwagi (od końca wejścia do wyjścia). Etykieta pokazuje „↳ węzeł” (wchodzi po nim) i ◆N (węzeł ma też klucze).
- **Przeciągnij pasek** – zmienia opóźnienie; **przeciągnij krawędź** – czas trwania (Shift = co 0,05 s). Klik w pasek zaznacza element i ustawia głowicę na jego starcie; klik w puste miejsce przewija czas.
- Narzędzia – pasek **SEKWENCJA** pod paskiem osi czasu (działają na dzieciach zaznaczonej grupy, rodzeństwie zaznaczonego elementu albo całej scenie – przy jednej grupie głównej na jej dzieciach):
  - **⇶ Kaskada** – wejścia jedno po drugim co **krok** s (wyjścia w odwrotnej kolejności); elementy bez ruchu dostają przenikanie.
  - **↳ Po kolei** – każdy element wchodzi po zakończeniu wejścia poprzedniego (`motion.after`); przesunięcie liczy się samo, także gdy zmienisz czas poprzednika.
  - **⧉ Kopiuj ruch** – ruch zaznaczonego elementu na jego rodzeństwo (efekty tekstu tylko na tekstach).
  - **◆ Na klucze** – presety wejścia/wyjścia zaznaczonego elementu zamienione na edytowalne klucze: typy liniowe z krzywą `power2.*`/`linear`/`back.out` dają dwa klucze, pozostałe – klucze co 1/30 s; odwrócenia 3D, przesłony, maska ukośna, żaluzja, rosnący pasek, RGB i glitch zostają presetami (raport). Pętle i efekty tekstu zostają. Węzeł z kluczami na tych samych właściwościach – odmowa.
  - **◇ Z kluczy** – odwrotność: klucze wejścia (do ostatniego stop pointu) i wyjścia (od niego) zaznaczonego elementu – położenie, krycie, skala, skos, rozmycie, przycięcie – dopasowane do presetu z kierunkiem, krzywą, czasem i opóźnieniem (przenikanie, zoom, wjazd, przycięcie, rozmycie, wyskok, spadanie…). Stan końcowy wejścia trafia do właściwości elementu, klucze tej fazy znikają; pozostałe klucze (np. obrót) zostają. Gdy żaden preset nie pasuje dokładnie (np. wjazd o 200 px zamiast spoza ekranu), Artist proponuje zamianę w przybliżeniu – inaczej faza zostaje kluczami (raport). Element z presetem wejścia/wyjścia – odmowa.
  - **✕ Usuń ruch** – usuwa ruch z zaznaczonego elementu (bez zaznaczenia – z całej grupy).
  - **⟲ Dopasuj** – stop point po najdłuższym wejściu, długość = stop + najdłuższe wyjście.
- W panelu **Ruch** węzła pole **po węźle** ustawia sekwencję ręcznie.

### 8.5 Skróty Artist

| Skrót | Działanie |
|---|---|
| Ctrl+S | zapisz scenę |
| Ctrl+Z / Ctrl+Y (Ctrl+Shift+Z) | cofnij / ponów |
| Ctrl+D | duplikuj węzeł |
| Delete / Backspace | usuń węzeł (lub zaznaczony klucz, gdy aktywna jest oś czasu) |
| Spacja | odtwórz / pauza |
| Home / End | początek / koniec sceny |
| M | marker w bieżącym czasie |
| / | szukaj właściwości (panel Właściwości, przy zaznaczonym węźle) |
| Ctrl+C / Ctrl+V (na osi czasu) | kopiuj / wklej klucze |
| strzałki (Shift = ×10) | przesuń zaznaczony węzeł o 1 px (10 px) |

### 8.6 Zmiana szablonu, który jest na antenie

Zapis sceny nie zmienia grafiki na antenie: warstwa dostaje znacznik **⏳ nowa wersja** w Trio, a operator wybiera moment **Wdróż teraz** (`/api/deploy/<warstwa>` – podmiana bez ponownej animacji). Kolejny TAKE używa już nowej wersji.

### 8.7 Paczka belek (.cgpack)

Paczka przenosi belki między stanowiskami i serwerami jednym plikiem – razem z grafikami i czcionkami.

**Zasady przenoszenia** (jedne dla VizRS CG Artist, `/artist` i Trio; kod: `web/cgpack.js`):
- Elementy grafiki (belki, tickery, plansze) przenosi się **tylko paczką `.cgpack`**. Eksport jednej sceny (**⇩ Eksport**) i kilku (**⇩ Paczka**) zapisuje `.cgpack` – sceny, ich strony, grafiki i czcionki.
- Dawny eksport sceny `.vizrs.json` (bez plików) jest tylko **czytany**: przechodzi przez ten sam import co paczka (scena zapisana jako nowa), a raport wymienia pliki, których brak w assets. Nowych plików `.vizrs.json` program nie zapisuje.
- Import paczki jest jeden (`cgpack.importPack`) – w Artist (**⇧ Import**) i w Trio (**⇧ Import paczki**, §9): pliki wg reguł poniżej, sceny zawsze jako nowe (istniejące nie są nadpisywane), strony z numerem albo nowym, gdy numer jest zajęty; bieżący show i antena zostają.
- Wysyłka scen z VizRS CG Artist na serwer stosuje te same reguły plików (`src/deploy.rs` w Artist).
- `.show.json` w Trio (**⇩ Kopia show / ⇧ Przywróć show**) to kopia zapasowa całego show (strony, playlisty, rundowny, makra; bez plików), nie sposób przenoszenia belek.

- **⇩ Paczka** otwiera listę scen (bieżąca zaznaczona), opcję „dołącz strony z Trio” i podsumowanie (ile scen, grafik, czcionek). **Pobierz .cgpack** zapisuje plik. Niezapisane zmiany bieżącej sceny trafiają do paczki w wersji z edytora.
- Do paczki trafiają pliki z assets, których scena używa (obrazy i wideo w `src`, fonty w `font`, odnośniki `/assets/…` w tekstach, wartości stron) oraz **dołączone** ręcznie.
- **Własna czcionka elementu:** **⤒** przy polu **font** (tekst, zegar, ticker) wgrywa plik TTF/OTF/WOFF/WOFF2 do assets i ustawia go w tym elemencie; scena używa go wprost, więc plik trafia do paczki bez dołączania.
- **Zasoby paczki** w ustawieniach sceny: lista „Używane” (wykryte) i „Dołączone” (`scene.packAssets`, × odłącza). **＋ Wgraj i dołącz** wgrywa nowy plik (grafika, wideo, czcionka TTF/OTF/WOFF/WOFF2) do assets i dołącza go do sceny; lista „dołącz z assets…” dołącza plik już wgrany. Dołączaj pliki, których scena nie wskazuje wprost – np. czcionkę dla znaczników tekstu albo grafiki, które operator wybiera w Trio. „brak!” = pliku nie ma w assets.
- **Reguły plików** (import paczki, a w VizRS CG Artist także ⤒, „Wgraj i dołącz” i przeciągnięcie pliku; `cgpack.assetPlacer`):
  1. nazwa jak na serwerze – znaki spoza `A–Z a–z 0–9 . _ -` → `_` (`Moja Czcionka.ttf` → `Moja_Czcionka.ttf`), odnośniki w scenie wskazują plik, który naprawdę powstał;
  2. czcionka o treści identycznej z czcionką już wgraną (pod dowolną nazwą, z tym samym rozszerzeniem) nie jest wgrywana drugi raz – scena dostaje istniejącą (font z assets to rodzina o nazwie pliku, więc kopia byłaby drugim, osobno ładowanym krojem);
  3. plik o tej samej nazwie i treści jest pomijany;
  4. nic nie jest nadpisywane: ta sama nazwa (bez względu na wielkość liter – Windows i macOS ich nie rozróżniają) z inną treścią → `nazwa_2.ext`, `nazwa_3.ext`…; gdy któraś z tych kopii ma już tę treść (ponowny import, ponowna wysyłka), używana jest ona zamiast kolejnej.
- **Eksport:** ta sama czcionka wgrana pod kilkoma nazwami trafia do paczki raz; odnośniki w scenach i `packAssets` paczki wskazują jedną nazwę (grafiki – bez zmian, każda nazwa osobno).
- **⇧ Import** przyjmuje `.cgpack`: pliki trafiają do assets wg reguł wyżej (raport wymienia pliki zapisane pod nową nazwą i czcionki zastąpione istniejącymi), sceny są dodawane jako nowe (istniejące nie są nadpisywane), strony dostają swój numer albo nowy, gdy numer jest zajęty. Raport pokazuje, co się zmieniło.
- Czcionki z paczki serwuje serwer (`/assets/<plik>`); wyjście `/output` ładuje je zawczasu (§4.8).

Format (ZIP): `manifest.json` (`_type: "vizrs_pack"`, `_packVersion: 1`, lista scen i plików z sumą CRC-32), `scenes.json` (`{ scenes, pages }` jak w show), `assets/<nazwa>`. Odczyt sprawdza ścieżki, typy plików, sumy i limity (200 plików, 256 MB).

## 9. Trio – playout

### 9.1 Układ

| Kolumna | Zawartość |
|---|---|
| Lewa | **Strony** (pole `nr` + Enter = wczytaj, szukaj, ⧉ Duplikuj, 🗑) i **Szablony** (＋ = nowa strona z wartościami domyślnymi) |
| Środkowa | edytor strony: numer, nazwa, skrót klawiszowy, Zapisz, Przywróć zapisane; status (szablon, **wybór warstwy**, ● ON AIR, 🔒, **⏱ kanał**); **pola**; duży **podgląd** strony; przyciski TAKE / CONTINUE / OUT / UPDATE / ARM |
| Prawa | monitor **PGM** (rzeczywiste wyjście, wybór tła monitora, **OBS ⧉** – adres wyjścia do wklejenia w OBS z przyciskiem Kopiuj: host i port serwera, kanał wybranej anteny, przypomnienie o `?token=`, gdy serwer go wymaga), **Na antenie – warstwy** (⏭ continue, ⏏ out, ⇅ dock, Wdróż teraz), zakładki **Playlista / Rundown / Makra** |
| Pasek górny | lista show (paczka grafiki audycji) z ▾ (nowy / kopia / usuń), stan połączenia, „⚠ zapis show nieudany”, „⚠ błędy scen”, ⚙ Stan, ⇧ Import paczki (`.cgpack` – dodaje sceny, strony i pliki, §8.7), ⇩ Kopia show, ⇧ Przywróć show (`.show.json`), ✖ CLEAR ALL |

Lista **Antena** na górnym pasku wybiera antenę główną albo kanał playoutu (`+ nowy kanał…` dopisuje numer z ErsatzTV; adres `/trio?channel=2` otwiera go od razu). Na wybranej antenie działają TAKE, CONTINUE, OUT, CLEAR, lista „Na antenie”, monitor PGM, karuzela, rundown (każda antena ma własny kursor), makra i dock. Dane stron są wspólne, więc UPDATE dociera na każdą antenę z tą stroną, a Preview (ARM) jest jeden. Lista kanałów przeżywa restart; `− usuń kanał` zdejmuje jego grafikę.

### 9.2 Praca ze stroną

1. Wczytaj stronę: klik na liście, numer + Enter albo ↑ ↓.
2. Zmień pola. Podgląd odświeża się na bieżąco; klik w element podglądu przenosi kursor do jego pola.
3. **TAKE** (F1 / Ctrl+Enter / dwuklik na liście) wysyła na antenę **bieżące wartości z edytora**, także niezapisane. Zapis strony to osobna czynność (Ctrl+S).
4. **CONTINUE** (F2) – następny stop point; **OUT** (F3) – wyjście.
5. **UPDATE** (F4) podmienia dane strony na antenie bez ponownej animacji (bez zapisu strony).
6. **ARM** (F6) wysyła stronę na wyjście Preview (`/output?bus=preview`), antena się nie zmienia.

TAKE strony z warstwy, na której już coś jest, zastępuje poprzednią grafikę tej warstwy. Różne warstwy grają jednocześnie.

**Zmiana warstwy** – lista **warstwa** w statusie strony (`/api/layer/<strona>`). Warstwa należy do szablonu, więc zmiana od razu obejmuje wszystkie jego strony i jest zapisywana. Lista jest ułożona jak stos na wyjściu (od dołu); `●` i numer oznaczają warstwę zajętą na antenie, a **＋ nowa warstwa…** pyta o nazwę. Grafika tego szablonu na antenie i na Preview przechodzi na nową warstwę bez ponownej animacji: ticker i zegar grają dalej, stop point i wyłączony dock zostają, a znacznik ⏳ się nie pojawia. Gdy warstwa docelowa jest zajęta na antenie, Trio pyta o potwierdzenie; po **Zastąp** tamta grafika schodzi z anteny. Otwarta w Artist scena przejmuje nową warstwę. Odwołania do starej nazwy (reguły dock `layer:…`, `?layers=…` w OBS, makra) trzeba poprawić ręcznie.

F2 i F3 działają na warstwie, na której strona faktycznie gra, także gdy szablon dostał w tym czasie inną warstwę w Artist.

### 9.3 Podgląd strony

- Podgląd automatycznie kadruje element strony: środek podglądu wypada na elemencie, a mały element (plakietka, zegar) jest powiększany tak, by zmieścił się cały. Kadr dopasowuje się przy zmianie treści i rozmiaru panelu.
- Kółko myszy – zoom wokół kursora; przeciąganie (lewy przycisk przy powiększeniu albo środkowy) – przesunięcie. Ręczny zoom wyłącza automatyczny kadr do zmiany strony.
- Dwuklik w element – powiększenie na element; dwuklik w puste miejsce – powrót do automatycznego kadru.
- ⌖ – element edytowanego pola; ⤢ – cały kadr 1920×1080; −/＋ – zoom.
- Podgląd lokalny: ▶ In, ⏭, ⏏, ■ (statycznie w pierwszym stopie) – bez wpływu na antenę.

### 9.4 Skróty Trio

| Skrót | Działanie |
|---|---|
| F1 / Ctrl+Enter | TAKE |
| F2 | CONTINUE |
| F3 | OUT |
| F4 | UPDATE |
| F6 | ARM (Preview) |
| F8 | NEXT w rundownie |
| cyfry + Enter | wczytaj stronę o numerze |
| ↑ / ↓ | poprzednia / następna strona |
| Ctrl+S | zapisz stronę |
| Shift+Esc | CLEAR ALL |
| Ctrl+Spacja (w polu) | Word Replace |
| skrót strony | TAKE tej strony, a gdy jest na antenie – OUT |

**Czas na kanale (⏱ kanał):** pod nagłówkiem edytora rozwiń **⏱ kanał** i ustaw, kiedy VizRS CG Playout (sidecar ErsatzTV) bierze stronę na antenę kanału: **wejście** – domyślnie, *po starcie* (sekundy po starcie pozycji) albo *przed końcem* (np. 60 s – belka „Za chwilę”; puste = 60), **zejście** – sekundy przed końcem (puste = domyślne sidecara), **koniec** – audycji z EPG albo pozycji. Zapis trafia do strony jako `playout` (`at` – ujemne = przed końcem, 0–86 400 s; `out` 0–86 400 s; `ref` – `programme`/`item`). VizRS tylko go przechowuje; sidecar przepisuje go do adresu elementu strony w ErsatzTV przy synchronizacji (dokumentacja VizRS CG Playout, §3 „Grafika przed końcem audycji”). Bez sidecara pole nic nie zmienia.

**Skrót strony:** kliknij pole „skrót” w nagłówku edytora i naciśnij kombinację (Ctrl lub Alt + klawisz albo F7, F9–F12, także z Shift); Backspace lub Delete usuwa skrót.

### 9.5 Playlista, rundown, makra

- **Playlista (karuzela):** strony z czasami trwania, opcjonalnie w pętli; ▶ Start / ■ Stop. Odtwarza ją serwer – działa także po zamknięciu Trio.
- **Rundown:** kolejka stron programu. Przeciąganie zmienia kolejność, notatki przy pozycjach, stany: ● na antenie, ▶ następna, ✓ odegrana. **NEXT** (F8) bierze następną pozycję; ↺ zeruje stany; import/eksport CSV (`numer;notatka`). Bez automatu czasowego – zawsze ręczne NEXT.
- **Makra:** jedna komenda w wierszu, opcjonalnie z opóźnieniem: `CLEAR`, `TAKE 1001`, `+2.5 TAKE 3001`, `+6 OUT main,back`. **⇥ z anteny** tworzy makro z bieżącego stanu anteny (preset segmentu). Nowe makro przerywa poprzednie.

### 9.6 Show na audycję

Lista w pasku górnym przełącza aktywny show (sceny + strony + playlisty + rundowny + makra) – każda audycja może mieć własną paczkę grafiki. Przełączenie zdejmuje wszystko z anteny i podglądu, zatrzymuje karuzelę i makro. **▾** tworzy nowy pusty show, kopię bieżącego (te same szablony pod inną nazwą) albo usuwa nieaktywny show z dysku. ⇧ Przywróć show zastępuje show aktywny kopią `.show.json` – żeby wczytać kopię jako osobną audycję, utwórz najpierw nowy show i dopiero potem przywróć. Belki z innego stanowiska dodaje **⇧ Import paczki** (`.cgpack`) – bez zastępowania show.

### 9.7 ⚙ Stan systemu

Zdrowie serwera i wyjść, błędy scen (z przyciskiem Wyczyść), lista Word Replace, kopie zapasowe show (Przywróć), ostatnie komendy z logu audytowego.

### 9.8 Wielu operatorów

Edytowana strona jest blokowana miękko: inni widzą 🔒 z imieniem; zapis cudzej strony wymaga potwierdzenia. Blokada wygasa po 45 s bez aktywności. Konflikt zapisu → wybór „Nadpisz moją wersją / Nie zapisuj” (zmiany zostają w edytorze).

**Pytania w Trio** (usunięcie, CLEAR ALL, zmiana show, nazwy playlist, rundownów, makr i warstw) zadaje okno aplikacji, nie okienko przeglądarki: przyciski nazywają czynność (np. **Zdejmij wszystko**, czerwony przy czynności nieodwracalnej lub zdejmującej z anteny), **Enter** wybiera wyróżniony przycisk, **Esc** albo kliknięcie obok – Anuluj. Gdy okno jest otwarte, skróty Trio (F1–F8, Ctrl+Enter) nie działają.

### 9.9 Trio na telefonie (`/mobile`)

Osobna lekka strona do sterowania anteną z telefonu albo tabletu, także w poziomie. Korzysta z tego samego API co Trio, więc telefon i stanowisko widzą tę samą antenę, a komendy z telefonu trafiają do logu audytowego. Adres z tokenem: `http://<adres-serwera>:8088/mobile?token=<token>`. Telefon sięga do serwera tylko wtedy, gdy nasłuchuje on w sieci (`--bind 0.0.0.0` z `--token`, §12); token wyjścia nie otwiera tej strony (403).

**Instalacja na telefonie (PWA):** strona ma manifest, ikonę i service workera, więc uruchamia się z ikony we własnym oknie, bez paska przeglądarki.

| Telefon | Jak zainstalować | Co działa |
|---|---|---|
| Android, HTTPS | Chrome → menu → **Zainstaluj aplikację** | własne okno, start bez sieci, niewygaszanie ekranu |
| iPhone | Safari → **Udostępnij** → **Do ekranu początkowego** | własne okno; start bez sieci i niewygaszanie tylko przez HTTPS |
| Android, zwykłe HTTP | tylko skrót na ekranie | Chrome nie instaluje aplikacji bez HTTPS, skrót otwiera kartę przeglądarki |

HTTPS musi mieć certyfikat zaufany przez telefon; certyfikat samopodpisany z ostrzeżeniem nie wystarczy. VizRS sam nie obsługuje TLS, więc potrzebne jest reverse proxy (§12, [RUNBOOK](RUNBOOK.md#vps-i-docker)).

Bez tokenu w adresie `/mobile` pokazuje ekran logowania; po podaniu tokenu ciasteczko ważne jest rok, a token znika z adresu. `/mobile.webmanifest` i ikony są dostępne bez tokenu, bo przeglądarka pobiera je bez ciasteczka. Service worker przechowuje wyłącznie pliki interfejsu i zawsze najpierw pyta serwer, więc po aktualizacji telefon dostaje nową wersję. Bez sieci aplikacja startuje i pokazuje „brak połączenia”.

Na dole ekranu stale są wybrana strona oraz **TAKE**, **CONTINUE**, **OUT** i **UPDATE**. Działają jak w Trio (§9.2): TAKE i UPDATE wysyłają wartości z edycji, także niezapisane, a CONTINUE i OUT są aktywne, gdy strona gra na wybranej antenie.

| Zakładka | Zawartość |
|---|---|
| **Strony** | lista z wyszukiwaniem po numerze i nazwie, ● = na antenie, 🔒 = edytuje ktoś inny; przytrzymanie otwiera menu strony (Edytuj pola, ARM, Wyczyść Preview) |
| **Edycja** | podgląd strony w pierwszym stopie, pola, **⋯** (menu strony), **↺** (przywróć zapisane), **Zapisz** |
| **Antena** | monitor PGM (`/output?fps=15`, działa tylko na tej zakładce), warstwy z ⏭, ⏏ i **Wdróż**, gdy czeka nowa wersja szablonu, **CLEAR ALL** z potwierdzeniem |
| **Kolejka** | rundown: dotknięcie pozycji ustawia kursor, **NEXT ▶** bierze ją na antenę; karuzela: ▶ / ■; makra: ▶ uruchamia, **■ Przerwij makro** |

Zakładki zmienia pasek na dole albo przesunięcie palcem w bok. Pytania i menu to arkusze wysuwane od dołu; zamyka je **Anuluj**, dotknięcie tła albo przycisk Wstecz w Androidzie. Gdy coś jest na antenie, nagłówek i pasek systemowy telefonu są czerwone, a przy zakładce Antena widać liczbę warstw. Komendy potwierdza krótka wibracja, a błąd podwójna (tylko Android, iPhone nie udostępnia wibracji stronom). **⋯** w nagłówku: odświeżenie danych, instalacja, pełne Trio.

- **Ochrona anteny:** dotknięcie strony lub pozycji rundownu niczego nie wysyła. Kolejna komenda czeka na odpowiedź serwera i krótką przerwę, więc podwójne dotknięcie TAKE nie gra grafiki dwa razy.
- **Pola:** tekst, liczby, przełączniki, listy, warianty, kolor, obraz (lista grafik, **⤒** wgrywa zdjęcie z telefonu) i wiadomości tickera (wiersz: kategoria i treść, **＋ Wiadomość**, kolor wiadomości zostaje). Reguły pól działają jak w Trio. Czcionkę zmienia się w Trio; strona podaje, ile takich pól pominięto, a ich wartości zostają bez zmian.
- **⏱ kanał** pod polami strony: czas strony na kanale dla VizRS CG Playout – te same ustawienia co w Trio (§9, „Czas na kanale”): wejście domyślne / po starcie / przed końcem (puste = 60 s), zejście w sekundach przed końcem, koniec audycji (EPG) albo pozycji. Zapisuje się ze stroną (**Zapisz**) jako `playout`.
- **Blokada edycji** (§9.8) zakłada się przy pierwszej zmianie pola, nie przy wybraniu strony. Nazwa operatora to `vizrs.who`, domyślnie `Telefon NN`.
- **Antena:** lista pojawia się, gdy są kanały playoutu; `?channel=<kanał>` i wybór są wspólne z Trio w tej samej przeglądarce. Po usunięciu wybranego kanału strona wraca na antenę główną i o tym informuje.
- **Sieć:** komenda bez odpowiedzi przez 6 s zgłasza błąd i nie jest ponawiana. Po takim błędzie sprawdź antenę, bo komenda mogła dojść. Zanik połączenia widać na pasku; po powrocie sieci albo telefonu z uśpienia strona sama się łączy i odświeża dane.
- **Wygaszanie ekranu:** strona prosi o niewygaszanie, ale przeglądarki pozwalają na to tylko przez HTTPS; po zwykłym HTTP wydłuż czas wygaszania w telefonie.

Na telefonie nie ma tworzenia i usuwania stron, przełączania show, formatowania tekstu, grania animacji w podglądzie ani edycji playlist, rundownu i makr.

## 10. Wyjście dla OBS i mikserów

OBS: Źródła → **+** → **Przeglądarka** → `http://localhost:8088/output`, 1920×1080, zaznacz „Odśwież przeglądarkę, gdy scena staje się aktywna”. Trio można dodać jako dock (*Docki → Niestandardowe docki przeglądarki*).

| Parametr | Działanie |
|---|---|
| `?layers=main,front` | tylko wybrane warstwy (osobne źródła w OBS) |
| `?bus=preview` | wyjście Preview (ARM) |
| `?mode=fill` / `?mode=key` | Fill (kolor na czerni) / Key (alfa jako luma) – dwa źródła dla ATEM/Tricastera |
| `?bg=%2300ff00` | tło (chroma) |
| `?w=1280&h=720` | inna rozdzielczość |
| `?checker=1` | szachownica (podgląd); bez `bg` tło jak monitor PGM w Trio |
| `?status=1` | pasek stanu połączenia |
| `?token=…` | token przy `--token` |
| `?fps=25` | lekkie wyjście: animacje najwyżej N klatek/s (klatkaż kanału) |
| `?fx=step` | lekkie wyjście: efekty uwagi schodkami (tabela niżej) |

**Lekkie wyjście (bezgłowy Chromium bez GPU).** Playout kanału przechwytuje `/output` przez DevTools (`Page.startScreencast`): każda klatka, w której cokolwiek się zmieniło, to odczyt, kodowanie i dekodowanie całego obrazu 1920×1080 (ok. 4% CPU na każdą klatkę na sekundę). Koszt zależy więc od liczby zmienionych klatek, a nie od wielkości grafiki – mrugająca kropka kosztuje tyle co przewijany pasek. Strona statyczna (belka, podpis, logo, zegar bez sekund) po wejściu nie zmienia obrazu i nie kosztuje nic.

- `?fps=N` – pętla animacji rysuje najwyżej N razy na sekundę (1–120). Tempo animacji się nie zmienia, rzadsze jest tylko próbkowanie. Ticker crawl przy `fps=25` kosztuje ok. 45% mniej niż przy 60 Hz. Nie dotyczy wideo w węźle `image` – film gra we własnym tempie.
- `?fx=step` – efekty liczone z zegara zmieniają obraz tylko kilka razy na okres:

| Efekt | Domyślnie | `fx=step` |
|---|---|---|
| `blink` | płynne krycie | 2 stany (pełne i 0,15) |
| `motion.loop`: `pulse`, `breathe`, `glow`, `flash`, `heartbeat` | płynnie | 4 próbki na okres |
| `motion.loop`: `float`, `bounce`, `wobble`, `shake`, `shine`, `underline-sweep` | płynnie | wyłączone (pozycja spoczynkowa) |
| `motion.bg`: `pulse`, `glow` | płynnie | 4 próbki na okres |
| `motion.bg`: `shimmer`, `scanline` | płynnie | wyłączone |

Wygląd z `fx=step` różni się od OBS, dlatego to świadomy wybór kanału; podgląd: `/output?fx=step&checker=1`. Adres dla playoutu kanału 25 kl./s: `/output?fps=25&fx=step`. Bez stron na antenie pętla animacji zasypia (w każdym trybie).

**Scena, która stoi, nie jest liczona od nowa (w każdym trybie: OBS, Trio, wyjście kanału).** Po animacji wejścia odtwarzacz sceny (`ScenePlayer`) nie przelicza i nie przypisuje stylów węzłów w każdej klatce, tylko wtedy, gdy obraz może się zmienić: zegar – na pełną sekundę (dziesiątą przy `S` w liczniku), rotacja wierszy `cycle` i ticker `roll`/`wipe`/`whip` – w czasie przejścia, błysk etykiety – w czasie przebiegu; `blink`, pętle uwagi, efekty tła i crawl grają w każdej klatce, a z `fx=step` sprawdzane są co 40 ms (`blink` – na zmianę stanu). Budzą ją: UPDATE, nowa wersja szablonu, CONTINUE/OUT, podgląd wiadomości w Trio i wczytany font. Styl węzła jest zapisywany tylko przy zmianie wartości, a pola sceny i tickera mają izolowany układ (`contain: strict`), więc pomiar tekstu nie przelicza reszty strony (Trio). Obraz jest taki sam jak wcześniej (zrzuty piksel w piksel); kanwa Artist liczy scenę w każdej klatce jak dotąd. Zmierzony czas głównego wątku Chromium (`/output`, 1080p, bez GPU): statyczne belki ok. 1% zamiast 3–5%, zegary 1,1% zamiast 7,5%, ticker `roll` 1,9% zamiast 6% (przeliczenia stylu 9/s zamiast 60/s).

**Taśma crawla przesuwana przez playout (`window.vizrsTape`).** Przewijany crawl zmienia obraz w każdej klatce, choć taśma tylko się przesuwa. Playout kanału (VizRS CG Playout, opcja `tape`) może ją przesuwać sam: wtedy Chromium rysuje taśmę tylko raz na jej okno („film”), a nie w każdej klatce. `/output` wystawia do tego interfejs dla DevTools (`Runtime.evaluate`); bez wywołań nic nie zmienia i nic nie kosztuje (OBS, Trio, Artist):

| Wywołanie | Działanie |
|---|---|
| `vizrsTape.state()` | crawle na antenie po ostatniej klatce: `{crawls:[{id, x, y, w, h, s, hidden, key}]}` – okno crawla w px strony, przesunięcie taśmy `s` w px ekranu (kolumna X okna pokazuje punkt X + s taśmy), `hidden` = w tej klatce taśmę rysuje playout, `key` zmienia się ze zmianą wyglądu taśmy (styl, kolor, font, rozmiar) |
| `vizrsTape.external(id, on)` | `true`: strona ukrywa taśmę crawla `id`, gdy ten stoi; `false`: rysuje ją sama |
| `vizrsTape.film(id, s)` | następna klatka to film: tylko taśma crawla `id` przy przesunięciu `s`, reszta strony ukryta; kolejna klatka wraca do zwykłej |

Strona ukrywa taśmę tylko wtedy, gdy crawl stoi: okno w tym samym miejscu co w poprzedniej klatce, przodkowie z pełnym kryciem, bez filtra, maski, `clip-path` i trybu mieszania, a żaden inny widoczny węzeł (wyższa warstwa albo dalej w scenie) nie maluje nad oknem: liczy się tło, ramka i obraz węzła oraz linie jego tekstu, nie całe pole (napis nad belką, którego pole sięga okna, nie przeszkadza); cienie sąsiadów (etykieta, zegar przy oknie) się nie liczą. W przeciwnym razie (wejście i wyjście sceny, `dock`, element na tickerze, podgląd wiadomości `pin`) rysuje ją sama w tej samej klatce, więc ruch nie przerywa się. Identyfikator `id` zmienia się, gdy taśma powstaje od nowa (nowy TAKE, zmiana trybu tickera). Interfejs ma `version: 1`.

Wskazówki dla scen kanału wypalanego: ticker w trybie `roll` albo `wipe` kosztuje ok. 4 razy mniej niż `crawl` (między zmianami wiadomości obraz stoi); `blink`, pętle uwagi i efekty tła na grafikach wiszących godzinami (logo, „Na żywo”) to stały koszt pełnego klatkażu – lepiej z nich zrezygnować albo użyć `fx=step`; zegar `HH:mm` zmienia obraz raz na minutę, z sekundami raz na sekundę (oba tanie); rotacja `cycle` kosztuje głównie w czasie przejścia; wideo w grafice nie nadaje się na kanał wypalany.

### 10.1 Renderer natywny (`vizrs-render`)

`vizrs-render` (katalog `source/render/`) rysuje antenę bez przeglądarki: czyta te same zdarzenia co `/output` (`/events`, fonty `/fonts`, pliki `/assets`), odtwarza sceny tą samą logiką co `renderer.js` (stop pointy, klucze i krzywe, pola strony, warianty, autofollow, `dock`, ticker `crawl`, zegary) i rastruje je na CPU (tiny-skia, tekst kształtowany jak w Chromium – rustybuzz). Służy playoutowi kanału (VizRS CG Playout, opcja `renderer`) na maszynach bez GPU: przewijany crawl 1080p25 kosztuje kilka procent jednego rdzenia zamiast ok. 2 rdzeni bezgłowego Chromium. OBS, Trio i Artist dalej używają `renderer.js`; renderer natywny to drugi wykonawca tej samej semantyki, nie nowy format.

Czego jeszcze nie rysuje, zgłasza jako brak: playout przechodzi wtedy na Chromium (`renderer: auto`), więc grafika nie znika i nie zmienia wyglądu. Stan (wersja 0.2; INFO24 16/16 i R7 13/13 scen natywnie):

| Obsługiwane | Jeszcze nie (scena idzie do Chromium) |
|---|---|
| `group`, `rect`, `ellipse` (kolor, gradient, zaokrąglenie, ramka, cień), `image` PNG/JPEG, `qr` (te same moduły co `qr.js`) | obraz WebP/GIF/SVG, wideo |
| `text`, `clock`: `textFit` (wszystkie tryby), `lineBreak`, `maxLines`, `padX`, `upper`, `spacing`, `lineHeight`, cień, `cycle`; tekst sformatowany (`[[wyróżnienie]]` z `hlColor`/`hlText`, `{b}`, `{i}`, `{u}`, `{c=}`, `{sz=}`); zegar `time`/`up`/`down`/`target`, strefy IANA | `{sz=}` nie powiększa wysokości wiersza (w przeglądarce większy fragment ją podnosi) |
| `ticker` `crawl`, `roll`, `wipe`, `whip`: etykieta z `gleam`, plakietki kategorii i kolory, separatory (`text`, `dot`, `square`, `pipe`, `skew`), `crawlFrom`, `hold`, dopasowanie wiadomości, ciągłość po UPDATE | podgląd jednej wiadomości (`pin`, tylko Trio) |
| klucze wszystkich animowanych właściwości, położenie sceny `x`/`y`, `opacity`, maski `clip*`, `blur`, `rotation`, `scale`, `skewX`, `blink` (też `fx=step`), `showIf`, `follow`, `dock`, warstwy i `?layers`/`?bus`/`?w`/`?h` | ruch (`node.motion`: presety IN/OUT, efekty tekstu, pętle, tło) |
| fonty wbudowane (§4.8) i z assets (`.ttf`, `.otf`, `.woff`, `.woff2`) | fonty z Google Fonts (spoza wbudowanych) |

```bash
cd source/render && cargo build --release      # osobny workspace z zależnościami (rdzeń vizrs bez zmian)
vizrs-render check --show show.json            # które sceny pójdą natywnie, a które do Chromium i dlaczego
vizrs-render frame --show show.json --page 1001 --stop 0 -o 1001.png   # jedna klatka (--time S: S sekund od TAKE)
vizrs-render bench --show show.json --page 3001 --seconds 10 [--area 0,960,1920,120]   # koszt klatki
vizrs-render bounds --show show.json --all 1 --size 1600x900 [--url <adres /output>]   # granice stron (VizRS CG Playout, strefy grafiki)
vizrs-render stream --url 'http://vizrs:8088/output?token=…' --size 1920x1080 [--area x,y,w,h]…   # dla playoutu; --area można powtórzyć
```

`frame`, `check` i `bench` biorą fonty z `--fonts` (domyślnie `web/fonts` obok) i pliki z `--assets`; `stream` – z serwera.

**Granice stron (`bounds`)**: odtwarza każdą stronę jak na antenie: TAKE, dojście do kolejnych stop pointów (`stops` sceny) i na każdym spoczynek `--hold` sekund (domyślnie 4; pętle, tickery, zegary), CONTINUE do następnego, po ostatnim OUT i wyjście (najwyżej 10 s); scena bez stop pointów: `duration` + spoczynek, potem OUT. W każdej klatce (`--fps`, domyślnie 25) zaznacza komórki siatki `--cell` px (domyślnie 16), w których jest choć jeden piksel z alfą > 0; domyślnie rysuje w połowie rozmiaru (`--half 0` wyłącza) – te same komórki w pikselach `--size`, ok. 4× mniej rysowania. Strony: `--page N` (można powtórzyć) albo `--all 1`; z `--url` fonty i pliki `/assets` z serwera jak na antenie (token wyjścia wystarcza), bez niego `--fonts`/`--assets` jak w `frame`; rozmiar sceny wyjścia z `w`/`h` w adresie, inaczej 1920×1080. Wynik: jeden wiersz JSON na stronę: `id`, `number`, `size`, `cell`, `rects` (`[x, y, w, h]` w px `--size`, poziome serie komórek sklejone pionowo) i `issues` (czego renderer natywny nie umie – granice wtedy niepełne); strona bez sceny: `error`. Pomiar: przykładowe INFO24 (22 strony) przy 1600x900 – 3,7 s na komputerze x86 (jeden proces). Używa tego VizRS CG Playout przy `VIZRS_AREA=auto` (PLAN-strefy S6 w tamtym repozytorium).

**Protokół `stream`** (proces podrzędny playoutu): na stdin wiersze `F <czas_ms> <ścienny_ms> [cfp]` – klatka w chwili `czas_ms` (zegar odtwarzania, monotoniczny; animacje liczą się od niego, nie od zegara procesu) przy czasie ściennym `ścienny_ms` (zegary `time` i `target`); `c` = rysuj tylko, gdy całą antenę da się narysować natywnie, `f` = wyślij obraz (albo „pusto”) także bez zmian – odbiorca wraca po przerwie; `p` = odbiorca przyjmuje łaty (rodzaj 6); `Q` kończy. Na stdout na każdą klatkę rekord: 16 bajtów nagłówka (`VZR1`, rodzaj u8, flagi u8, 2 bajty zera, szerokość i wysokość u32 LE), a przy rodzaju 1 piksele RGBA bez przemnożenia alfy, wiersz po wierszu, w rozmiarze `--area` (domyślnie całe `--size`). Rodzaje: 0 – bez zmian, 1 – nowy obraz (cały), 2 – pusto, 3 – nie narysowano (`c` przy niepełnej antenie), 4 – nowy obraz, ale tylko prostokąt z grafiką: po nagłówku jego x, y, szerokość i wysokość (u32 LE), potem jego piksele RGBA, reszta przezroczysta. Renderer wysyła nowe obrazy jako rodzaj 4 (pasek tickera to ułamek przesyłu całego obrazu); odbiorca czyta też rodzaj 1. Przy kilku `--area` jeden proces rysuje każdy obszar osobno (tylko ich piksele, nie obwiednię), a nowy obraz to rodzaj 5: szerokość i wysokość nagłówka to rozmiar kanału, po nagłówku liczba prostokątów (u32 LE), potem każdy z nich jak w rodzaju 4, x i y w px kanału; pusty obszar nie ma prostokąta, a wszystkie puste to rodzaj 2. Rodzaj 6 – **łata** poprzedniego obrazu, tylko przy `p` i jednym obszarze: po nagłówku liczba prostokątów (u32 LE), potem każdy jak w rodzaju 4 (x, y, szerokość, wysokość w px obrazu, RGBA) – ich piksele zastępują piksele ostatniego obrazu (rodzaj 1, 4 albo poprzednia łata), reszta bez zmian; zero prostokątów = zmiana poza grafiką. Renderer wysyła łatę, gdy prostokąt z grafiką się nie zmienił, a odbiorca ma jego poprzedni obraz; po „pusto”, „nie narysowano”, przy `f` i przy zmianie prostokąta – cały rodzaj 4. Bez `p` (starszy odbiorca) – jak dotąd. Flagi: bit 0 – wszystko na antenie natywnie, bit 1 – połączenie z VizRS działa. Zmiana listy braków trafia na stderr. Adres `--url` to adres wyjścia (`/output`) – token, `layers`, `bus`, `w`, `h` i `fx=step` działają jak na stronie.

**Rysowanie tylko obszaru zmiany** (`stream` i `bench`; od 2026-11-11). Przy każdej klatce renderer liczy podpis drzewa rysowania (skrót węzła, skrót poddrzewa i obwiednię w px kanału razem z cieniami i rozmyciem) i porównuje go z poprzednim. Od nowa rysuje tylko prostokąt węzłów, które się zmieniły (stare i nowe położenie), na płótnie pomocniczym powiększonym o zasięg największego rozmycia w scenie. Dzięki temu rozmycie i cień przy krawędzi wychodzą tak samo jak przy pełnym rysowaniu. Do obrazu trafia tylko ten prostokąt. Całość rysuje się przy pierwszej klatce, przy zmianie liczby grafik, zasobów (fonty, obrazy), rozmiaru albo przekształcenia sceny, gdy obszar zmiany przekracza połowę obrazu oraz co 50. klatkę rysowaną częściowo (przy 50 kl./s – raz na sekundę; zabezpieczenie: gdyby obwiednia czegoś nie objęła, ślad zniknie najpóźniej po tylu klatkach). Rekord rodzaju 4 powstaje z zapamiętanego poprzedniego: zasięg alfy liczy się na nowo tylko w wierszach obszaru zmiany, a gdy prostokąt z grafiką się nie zmienił, w gotowym rekordzie podmienia się tylko ten obszar (bez skanu całego obrazu). Odbiorca z `p` dostaje przy tym sam zmieniony prostokąt (rodzaj 6): przewijany crawl pod planszą to ok. 0,6 MB na klatkę zamiast ok. 4 MB, odpowiedź renderera 6,9 ms zamiast 21 ms (mediana, ten sam host pod dużym obciążeniem), a VizRS CG Playout utrzymuje crawl w 50 kl./s (46–50 zmian paska na sekundę w nagraniu kanału). Piksele pozostają bez zmian: test `partial_redraw_matches_full_redraw` porównuje klatka po klatce rysowanie częściowe z pełnym (skala ¼ i ⅙, dwa obszary, rozmyta belka pod ruchomym elementem) – bajt w bajt, razem z rekordem. Pomiar (2026-11-11, Xeon E3-1270 v3, plansza z belką i taśmą na antenie, kanał 1080p50, w tle kodowanie kanału): nowy obraz z taśmą w 26 ms zamiast 56 ms (mediana), 4,6% zamiast 12,9% klatek po czasie, więc bezpiecznik płynności CG Playout (próg 10%) nie obniża już klatkażu do 25. Bez obciążenia: rysowanie zmiany 2,5 ms zamiast 14,5 ms.

**Wierność:** `node tools/native-compare.js [show.json] [--out katalog] [--tickers] [--assets katalog]` porównuje każdą obsługiwaną stronę z Chromium (TAKE, zakończone wejście, zegary ukryte; tickery ukryte, z `--tickers` widoczne – obie strony 2,6 s po TAKE) i podaje odsetek pikseli różnych o więcej niż 24/255 oraz średni błąd: INFO24 i R7 poniżej 1% pikseli i 0,6/255 średnio. Plansze testowe `node tools/native-cases.js <katalog>` (pola, gradienty, cienie, przekształcenia, maski, wszystkie tryby dopasowania, fonty, znaczniki, obrazy, QR, autofollow) – 11 z 12 poniżej 2% (ściśnięty tekst wielowierszowy 3%: tylko wygładzanie krawędzi, bez przesunięć). Szerokości tekstu sprawdza `node tools/native-widths.js` (średnio 0,05 px różnicy, najwięcej 0,04%), moduły QR – test `render/tests/qr_vectors.rs`. Ruch: film `node tools/native-video.js --stats plik.csv` (§14) – błąd każdej klatki; w nagraniu `requestAnimationFrame` strony biegnie raz na klatkę, jak `beginFrame` na kanale. INFO24: średni błąd klatek (MSE) każdej strony poniżej 5; krótkie piki do ok. 30 zostają w ruchu (przesuw `roll`, wejścia) – linia bazowa przesuwanego tekstu wypada czasem na sąsiedni piksel.

Reguły Blink i Skia, które renderer natywny odtwarza (zmiana którejkolwiek po stronie przeglądarki = test planszami):

| Reguła | Skutek, gdy jej brak |
|---|---|
| wysokość wiersza `line-height` × rozmiar w dół do 1/64 px; połowa interlinii nad wierszem w dół do piksela | tekst 0,5–1 px niżej, przy wielu wierszach rozjazd |
| linia bazowa tekstu bez obrotu na pełnym pikselu (w pionie), w poziomie podpikselowo | tekst rozmyty w pionie |
| wyśrodkowanie elementu tekstu wg dokładnej wysokości (nie `offsetHeight`) | wiersz co któryś o piksel niżej |
| szerokość elementu wielowierszowego = min(tekst w jednym wierszu, `max-width`) w jednostkach 1/64 px | wyrównanie do środka i prawej przesunięte |
| element z przekształceniem innym niż przesunięcie (`scaleX` ściśnięcia) – położenie do pełnego piksela | ściśnięty tekst do prawej/środka ±0,33 px |
| fragment `{sz=}` podnosi wiersz (własna interlinia fragmentu) | zła wysokość i wyśrodkowanie |
| przy łamaniu wiersz mierzony ze znacznikami otwartymi wcześniej (`{b}` z poprzedniego wiersza) | inne łamanie i ściśnięcie |
| rozmiar fontu do FreeType w formacie 26.6 (w dół do 1/64 px) | tekst o ułamkowym rozmiarze (`shrink`) szerszy o 0,05% |
| maska wygładzania glifów przez tablicę `SkMaskGamma` (gamma 1,2, kontrast 0,2, luminancja koloru w 3 bitach) | tekst cieńszy o 1–2,5% jasności |
| gradient z alfą mieszany z przemnożeniem (przezroczysty biały nie rozjaśnia) | jaśniejsze przejścia do przezroczystości |
| pierwsza klatka po TAKE (pętla uśpiona) ma dt = 0 | animacja wejścia o klatkę do przodu |
| zdarzenie z `/events` obsłużone z czasem poprzedniej klatki (`performance.now()` przed następną) | zegar `up`/`down` o klatkę inny |
| `margin-right: .5em` plakietki kategorii od jej własnego rozmiaru fontu | taśma crawla z kategoriami rozjeżdża się o ~4 px na odcinek |

Zmiana w `renderer.js` (rysowanie, dopasowanie tekstu, krzywe, ticker) wymaga tej samej zmiany w `source/render/` i przebiegu narzędzi zgodności.

## 11. Sterowanie zewnętrzne

Pełna lista komend, adresów i zdarzeń: [API.md](API.md); przyciski Stream Deck: [COMPANION.md](COMPANION.md).

| Kanał | Do czego | Przykład |
|---|---|---|
| HTTP (także GET) | automatyzacja, Companion, Stream Deck | `/api/take/1001?name=Jan` |
| TCP :6100 | komendy tekstowe, jedna w linii | `TAKE 1001 name=Anna\|title=Ekspertka` |
| OSC (`--osc 8000`) | konsole i sterowniki | `/vizrs/take 1001` |
| webhook (`--webhook http://…`) | powiadomienia na zewnątrz: `onAir`, `offAir`, `error` | |
| SSE `/events` | strumień zdarzeń stanu anteny | |

## 12. Uruchamianie, opcje, bezpieczeństwo

```bash
./start-linux.sh                       # Linux: gotowa binarka bin/linux-x86_64/vizrs
build-windows.bat → start-windows.bat  # Windows (wymaga Rust)
./build-macos-linux.sh                 # macOS / inne: bin/native/vizrs
```

Komendy: `vizrs` albo `vizrs serve` – serwer bez GUI (UI w przeglądarce); `vizrs artist`, `vizrs trio`, `vizrs output` – okno aplikacji (§12.1); `vizrs --healthcheck` – sprawdza `/health` działającego serwera (kod wyjścia 0 = ok, 1 = błąd; dla Dockera i monitoringu).

| Opcja | Zmienna | Domyślnie | Opis |
|---|---|---|---|
| `--port` | `VIZRS_PORT` | 8088 | port HTTP (UI + wyjście). Gdy domyślny 8088 jest zajęty (np. drugi serwer na innym katalogu danych), serwer bierze następny wolny (do 8107) i pisze o tym na starcie – adres jest w ramce; port podany jawnie (`--port`, `VIZRS_PORT`, Docker) zajęty = błąd, jak dotąd |
| `--tcp` | `VIZRS_TCP` | 6100 | port komend tekstowych (0 = wyłączony) |
| `--bind` | `VIZRS_BIND` | 127.0.0.1 | adres nasłuchu; adres inny niż lokalny (`0.0.0.0`, IP w sieci) **wymaga tokenu** |
| `--token` | `VIZRS_TOKEN` | – | wymagany token (znaki `A-Z a-z 0-9 - _ .`) |
| `--output-token` | `VIZRS_OUTPUT_TOKEN` | – | drugi token, **tylko do odczytu** – dla OBS/vMix (patrz niżej) |
| `--trust-proxy` | `VIZRS_TRUST_PROXY=1` | – | serwer za reverse proxy: adres klienta z `X-Forwarded-For` (log, spowalnianie prób), bez limitu połączeń na adres |
| `--log-stdout` | `VIZRS_LOG_STDOUT=1` | – | log audytowy także na standardowe wyjście (JSON w linii; `docker logs`) |
| `--insecure` | `VIZRS_INSECURE=1` | – | świadoma zgoda na nasłuch w sieci bez tokenu (tylko sieć zaufana) |
| `--osc` | `VIZRS_OSC` | 0 | port UDP OSC |
| `--webhook` | `VIZRS_WEBHOOK` | – | adres `http://` dla zdarzeń |
| `--data` | `VIZRS_DATA` | `vizrs-data` | katalog z `show.json` i `assets/` |
| `--web` | `VIZRS_WEB` | – | serwuj UI z dysku zamiast wbudowanego |
| `--idle-exit N` | | – | `serve`: zakończ po N s bez podłączonych okien i wyjść (serwer uruchamiany przez okno natywne); okno, które sprawdza serwer przed dołączeniem (`/health?attach=1`), zeruje odliczanie |
| `--healthcheck` | | | sprawdź `/health` serwera pod `--bind`/`--port` (z `--token`) i zakończ |
| `--version` | | | wersja i skrót commita |
| `--help` | | | lista opcji |

Flaga ma pierwszeństwo przed zmienną; pusta zmienna = nieustawiona. Opcje można też pisać jako `--port=8099`. Nieznana opcja albo zła wartość (np. port spoza 0–65535) kończy start z błędem (kod 2) – literówka nie przejdzie po cichu.

**Jeden serwer na katalog danych:** serwer zakłada `<data>/.vizrs.lock` (pid, adres, identyfikator instancji – ten sam co pole `instance` w `/health`). Drugi serwer na tym samym `--data` nie wystartuje (kod 3) i poda adres działającego. Plik po awarii (`kill -9`, zanik zasilania) jest rozpoznawany jako martwy i zastępowany przy następnym starcie. Gdy pliku nie da się zapisać, serwer działa dalej bez blokady (ostrzeżenie).

Token: przeglądarka – raz `?token=…` (zapisuje ciasteczko); API – `Authorization: Bearer`, `X-Token` albo `?token=`; TCP – pierwsza linia `AUTH …`.
**Token wyjścia** (`--output-token`, wymaga `--token`): pozwala tylko na to, czego potrzebuje strona wyjścia – `/output`, skrypty i style, `/fonts/*`, `/assets/*`, zdarzenia `/events`, lista grafik i zgłaszanie błędów scen. TAKE, zmiany show, Trio i Artist z tym tokenem → 403. W OBS/vMix wpisuj `…/output?token=<token wyjścia>` – wyciek adresu (kolekcja scen, zrzut ekranu) nie daje sterowania anteną.

**Ochrona w sieci publicznej:** po 5 nieudanych próbach tokenu z jednego adresu każda kolejna odpowiedź 401 jest opóźniana (do 3 s; licznik zeruje się po 10 min; poprawny token działa od razu), próby trafiają do logu audytowego; najwyżej 128 jednoczesnych połączeń z jednego adresu (poza `127.0.0.1` i trybem `--trust-proxy`); cały nagłówek żądania musi dojść w 15 s. Przy tokenie krótszym niż 16 znaków i nasłuchu w sieci serwer ostrzega na starcie.
**Zatrzymanie:** SIGTERM/SIGINT (Docker, systemd, Ctrl+C) kończy pracę od razu i zwalnia blokadę katalogu danych (zapisy show są atomowe).

Niezależnie od tokenu serwer odrzuca żądania z obcych witryn (`Sec-Fetch-Site`, `Origin`). VizRS sam nie obsługuje TLS – w sieciach współdzielonych i na VPS stawiaj go za reverse proxy z HTTPS (Caddy w `deploy/docker-compose.yml`).
Usługa systemowa: `deploy/` (systemd, launchd, Windows/NSSM).

**Docker / VPS:** `docker build -t vizrs .` (statyczna binarka musl na pustym obrazie, ~3 MB, użytkownik 10001, healthcheck `vizrs --healthcheck`, dane w wolumenie `/data`, port komend TCP domyślnie wyłączony). Serwer z HTTPS: `deploy/docker-compose.yml` (VizRS + Caddy, Let's Encrypt), przykład nginx: `deploy/nginx-vizrs.conf`. Instrukcja krok po kroku: [RUNBOOK.md](RUNBOOK.md#vps-i-docker). Gotowy obraz z każdego wydania: `ghcr.io/sdorpl/vizrs:<wersja>` / `:latest` (amd64, arm64).

### 12.1 Okna aplikacji (Trio, Output, zamrożony Artist)

Głównym narzędziem do projektowania scen jest osobny program **[VizRS CG Artist](https://github.com/sdorpl/VizRS-CG-Artist)** (`vizrs-cg-artist`, Tauri 2 dla systemów Linux, Windows i macOS; działa bez serwera i wysyła sceny na serwer).

`vizrs trio` otwiera Trio w **osobnym oknie aplikacji** – bez pasków adresu i kart, z własną pozycją w pasku zadań – i uruchamia serwer w tym samym procesie. `vizrs artist` otwiera w analogicznym oknie zamrożony edytor `/artist` serwera (zapas do pracy lokalnej lub w przeglądarce). Okno to przeglądarka z silnikiem **Chromium** (Edge, Chrome, Chromium, Brave, Vivaldi) w trybie `--app` z osobnym profilem (`~/.config/vizrs/browser`, Windows `%APPDATA%\VizRS\browser`, macOS `~/Library/Application Support/VizRS/browser`). Chromium dlatego, że wyjście w OBS i vMix też rysuje Chromium – podgląd pokazuje dokładnie to, co antena (pomiar tekstu, ściskanie, łamanie wierszy). Bez Chromium okno otwiera się w przeglądarce systemowej (z paskami).

| Sytuacja | Co się dzieje |
|---|---|
| pierwsze okno na danym `--data` | start serwera (port z `--port`, domyślnie 8088 – adres wyjścia dla OBS bez zmian) + okno |
| kolejne okno (np. `vizrs trio` przy otwartym innym oknie) | wykrywa działający serwer na tym katalogu danych (`.vizrs.lock`), otwiera tylko okno i kończy się |
| zamknięcie okna, z którym wystartował serwer | serwer **działa dalej, dopóki podłączone jest jakiekolwiek okno lub wyjście (OBS)** – zamknięcie okna Trio nie zdejmuje grafiki z anteny; po `--linger` s (domyślnie 10) bez nikogo serwer kończy pracę. Ctrl+C / SIGTERM kończy od razu |
| `--connect https://cg.example.pl` | tylko okno dla serwera zdalnego (VPS), bez serwera lokalnego; token z `--token`/`VIZRS_TOKEN` trafia do adresu okna |

`vizrs output` – okno wyjścia 1920×1080, np. na drugi monitor albo wyjście HDMI do miksera: `vizrs output --fullscreen --position 1920,0` (pełny ekran bez ramek, zamknięcie: Alt+F4); Key/Fill przez `--query mode=key` / `--query mode=fill`. Wyjście ma osobny profil przeglądarki (pełny ekran i pozycja działają niezależnie od okien Trio). Wszystkie okna mają wyłączone usypianie animacji w tle i autoodtwarzanie wideo bez kliknięcia.

| Opcja okna | Zmienna | Opis |
|---|---|---|
| `--connect URL` | `VIZRS_CONNECT` | serwer zdalny (`http://` / `https://`) |
| `--browser CMD` | `VIZRS_BROWSER` | przeglądarka: ścieżka (`/usr/bin/chromium`, `C:\…\msedge.exe`) albo polecenie z argumentami (`flatpak run org.chromium.Chromium`) |
| `--fullscreen` | | pełny ekran bez ramek (`--kiosk`) |
| `--position X,Y` | | pozycja okna (drugi monitor: `1920,0`) |
| `--query PARAM` | | parametry adresu okna, np. `mode=key`, `bg=black` |
| `--linger N` | | sekundy bez okien i wyjść przed zakończeniem serwera okna (domyślnie 10) |

Wyszukiwanie przeglądarki: Linux – `chromium`, `chromium-browser`, `google-chrome(-stable)`, `microsoft-edge(-stable)`, `brave-browser`, `vivaldi` w `PATH`, potem Flatpak i Snap; Windows – Edge, Chrome, Brave, Chromium w `Program Files` i `%LOCALAPPDATA%` (Edge jest w każdym Windows 10/11); macOS – `/Applications` i `~/Applications`.

**Skróty systemowe:**
- Debian / Ubuntu 22.04+: `sudo apt install ./vizrs_<wersja>_amd64.deb` z wydania na GitHub (budowa: `deploy/deb/build.sh vizrs vizrs-shell wersja [wyjście]`). W menu: **VizRS Trio**, **VizRS Trio – serwer zdalny** (od razu okno „Połącz z serwerem”, §12.2); pakiet bez wpisu „VizRS Artist” i bez typów MIME (usunięto `deploy/vizrs-mime.xml` – skojarzenia `.cgpack`/`.vizrs.json` należą do paczki VizRS CG Artist). Dane okien w `~/.local/share/vizrs` (`VIZRS_DATA`). Pakiet zawiera usługę systemd `vizrs` (serwer bez GUI, dane `/var/lib/vizrs`, użytkownik `vizrs`) – domyślnie wyłączoną: `sudo systemctl edit vizrs` (token), `sudo systemctl enable --now vizrs`. Pakiet zbudowany lokalnie działa tylko na systemach z glibc co najmniej taką jak u budującego – wydanie budowane jest na Ubuntu 22.04.
- Linux bez instalacji: `VizRS-Trio-…AppImage` z wydania na GitHub (budowa: `deploy/appimage/build.sh vizrs vizrs-shell wersja`; od 2.0.19 AppImage budowane są lokalnie i dołączane do wydania `gh release upload` razem z `SHA256SUMS-AppImage.txt` – akcja Release ich nie buduje; paczka AppImage dla Artist wydawana jest w repozytorium VizRS CG Artist). Okno natywne wymaga systemowego WebKitGTK 4.1 (`libwebkit2gtk-4.1-0`), bez niego otwiera się okno Chromium. Dane: `$VIZRS_DATA`, domyślnie `~/.local/share/vizrs` – drugie okno dołącza do serwera; binarka serwera kopiowana do `~/.cache/vizrs/<wersja>/` (serwer działa dłużej niż zamontowany obraz).
- Linux: `deploy/install-desktop-linux.sh [vizrs] [katalog-danych]` – „VizRS Trio” w menu aplikacji (pliki `.desktop`, ikona `deploy/vizrs.svg`; skrót Artist tworzy instalator VizRS CG Artist).
- Windows: instalator `VizRS-Trio-*-windows-x86_64-setup.exe` z wydania na GitHub (budowa przez Inno Setup na podstawie `deploy/windows/installer.iss`) – instaluje Trio w Menu Start (VizRS Trio, VizRS Trio – serwer zdalny, VizRS Serwer) z deinstalatorem. Do wyboru instalacja dla bieżącego użytkownika (`%LOCALAPPDATA%\Programs\VizRS`, bez uprawnień administratora) albo dla wszystkich (`C:\Program Files\VizRS`). Skróty nie mają wpisanego katalogu danych: każdy użytkownik pracuje na swoim `%LOCALAPPDATA%\VizRS` (okno – domyślny `--data`, „VizRS Serwer” – `serwer.bat` z tym samym wyborem). Gdy w systemie brak Microsoft Edge WebView2 Runtime (okrojone Windows 10, Windows Server), instalator po cichu uruchamia dołączony instalator Microsoftu (bez uprawnień administratora – dla bieżącego użytkownika); gdy to się nie uda, wyświetla komunikat, a do tego czasu działa „VizRS Serwer” i Trio w przeglądarce. W archiwum `.zip` znajdują się skrypty uruchomieniowe (`start-trio.bat`, `start-trio-zdalnie.bat`, `start-serwer.bat`, `instaluj-skroty.bat`). Lokalne tworzenie skrótów: `build-windows.bat`, potem `powershell -ExecutionPolicy Bypass -File deploy\install-windows-shortcuts.ps1`.

Trio pokazuje w pasku górnym wskaźnik **połączono / brak połączenia** z serwerem – przy pracy zdalnej (`--connect`) widać zerwanie łącza.

### 12.2 Okno natywne (`vizrs-shell`)

`vizrs-shell` to okno **tylko Trio i wyjścia (podglądu)** z systemowym silnikiem stron: **WebKitGTK** (Linux), **WebView2** (Windows, to Chromium), **WKWebView** (macOS). Domyślnym rodzajem okna jest Trio. Projektowanie scen to osobny program **[VizRS CG Artist](https://github.com/sdorpl/VizRS-CG-Artist)** (`vizrs-cg-artist`), a `/artist` w serwerze pozostaje jako zamrożony zapas w przeglądarce.

```bash
vizrs-shell                 # okno Trio (domyślnie)
vizrs-shell trio            # okno Trio
vizrs-shell output          # podgląd wyjścia 960×540
vizrs-shell trio --connect https://cg.example.pl --token …   # serwer zdalny
vizrs-shell connect         # najpierw okno „Połącz z serwerem”
```

Wywołanie `vizrs-shell artist` oraz podanie pliku (`.cgpack`, `.vizrs.json`) kończą się komunikatem odsyłającym do VizRS CG Artist i kodem wyjścia 2.

| Opcja | Zmienna | Opis |
|---|---|---|
| `--data KAT` | `VIZRS_DATA` | katalog danych (domyślnie `./vizrs-data`, jeśli zawiera `show.json`; inaczej Linux `~/.local/share/vizrs`, Windows `%LOCALAPPDATA%\VizRS` – a gdy tam nie ma jeszcze show, a jest w `%APPDATA%\VizRS\vizrs-data` (skróty instalatora 2.0.20), to ten drugi; danych nic nie przenosi) |
| `--port N` | `VIZRS_PORT` | port serwera uruchamianego przez okno (domyślnie 8088; gdy zajęty, np. przez serwer na innym katalogu danych – następny wolny, adres w Pomoc → O programie; port podany jawnie bez zmian) |
| `--token T` | `VIZRS_TOKEN` | token serwera |
| `--connect URL` | `VIZRS_CONNECT` | tylko okno, serwer zdalny |
| `--vizrs PLIK` | `VIZRS_BIN` | program `vizrs` (domyślnie obok `vizrs-shell` albo w `PATH`) |
| `--gpu TRYB` | `VIZRS_GPU` | akceleracja GPU stron na Linuksie: `on`, `off`, `auto` (domyślnie: decyduje WebKitGTK); do diagnozy problemów ze sterownikiem |
| `--local` | | serwer lokalny, nawet gdy zapisano łączenie z serwerem zdalnym przy uruchomieniu |
| | `VIZRS_CONFIG` | katalog listy serwerów (domyślnie niżej) |

- **Serwer:** jeśli na danym katalogu danych działa już serwer VizRS, okno dołącza do niego. Jeśli nie, powłoka uruchamia `vizrs serve --idle-exit 10` jako **osobny proces**. Awaria okna nie zdejmuje anteny, a serwer kończy pracę sam, gdy przez 10 s nie ma żadnego okna ani wyjścia (OBS). Okno dołączające do serwera w trakcie tego odliczania zeruje je (`/health?attach=1`), więc nie trafi na serwer, który właśnie się wyłącza. Komunikaty serwera trafiają do `~/.local/state/vizrs/serwer.log` (poprzedni przebieg: `serwer.log.1`; Pomoc → Pokaż logi). Błąd startu (np. zajęty port) okno pokazuje w okienku systemowym.
- **Połącz z serwerem** (Plik → Połącz z serwerem…, Ctrl+Shift+K): okno z adresem serwera (`https://cg.example.pl`, sam host = `https://`) i tokenem operatora. Dla serwera `http://` okno najpierw sprawdza token (`/api/state`: zły token, token wyjścia OBS i brak serwera – komunikat w oknie); `https://` sprawdza wskaźnik połączenia w Trio (powłoka nie ma TLS). Po połączeniu **wszystkie okna** przechodzą na ten serwer (tytuł z nazwą hosta), a gdy żadne nie było otwarte – otwiera się Trio. Lista zapisanych serwerów (klik – wypełnij, dwuklik – połącz, × – usuń) leży w `connections.json`: Linux `~/.config/vizrs/`, Windows `%APPDATA%\VizRS\`, macOS `~/Library/Application Support/VizRS/` (plik z prawami tylko dla właściciela); token zapisuje się tylko przy „zapamiętaj token” – puste pole tokenu = token zapamiętany. „Łącz z tym serwerem przy uruchomieniu” – kolejne starty okna idą od razu do tego serwera, bez serwera lokalnego (`--local` – mimo to lokalny). **Serwer lokalny** wraca do serwera na katalogu danych (w razie potrzeby uruchamia go). Przełączenie serwera nie zmienia niczego na antenie żadnego z serwerów.
- **Menu:** Plik – Połącz z serwerem… (Ctrl+Shift+K), Nowe okno Trio, Okno wyjścia (podgląd), Zamknij okno (Ctrl+W), Zakończ (Ctrl+Q); Widok – Odśwież (F5), Pełny ekran (F11), Powiększ / Pomniejsz / 100% (Ctrl+= / Ctrl+− / Ctrl+0), Narzędzia deweloperskie (Ctrl+Shift+I); Wyjście – Adres dla OBS… (to samo okno co **OBS ⧉** w Trio; okno natywne nie ma paska adresu, a port wybiera samo), wyjście na wybranym monitorze, Fill + Key na monitorach 1 i 2 (przy co najmniej dwóch ekranach); Pomoc – Strona startowa i API, Pokaż logi, Otwórz katalog danych, Sprawdź aktualizacje (strona wydań w przeglądarce), O programie. **macOS:** zgodnie ze zwyczajem systemu pierwsze jest menu **VizRS** (O programie, Usługi, Ukryj VizRS, Ukryj pozostałe, Pokaż wszystko, Zakończ ⌘Q), po Pliku jest **Edycja** z pozycjami systemowymi (Cofnij, Ponów, Wytnij, Kopiuj, Wklej, Zaznacz wszystko – bez nich ⌘C/⌘V nie działają w polach tekstowych WKWebView), a przed Pomocą **Okno** (Zminimalizuj, Powiększ okno i lista okien). Z menu usunięto pozycje projektowania scen (Nowe okno Artist, Nowa scena, Zapisz, Importuj…, Eksportuj scenę…, Paczka .cgpack…, całe menu Edycja [Cofnij, Ponów, Duplikuj, Usuń], pozycje kanwy w Widoku [Dopasuj kanwę, Kanwa 100%, Kanwa do zaznaczenia] oraz menu Scena) – te zadania realizuje VizRS CG Artist. Pełnoekranowe wyjście nie ma paska menu: zamknięcie Ctrl+W.
- **Zamykanie:** okno ze zmianami pyta „Zapisz / Nie zapisuj / Anuluj” (zamknięcie okna, Ctrl+W, Zakończ; Trio). Gdy strona nie odpowiada, ponowne zamknięcie po 5 s zamyka okno bez pytania.
- **Tytuł okna:** tytuł okna Trio lub wyjścia oraz `●` przy niezapisanych zmianach. Trio dopisuje stan widoczny w pasku zadań i Docku bez przełączania okien: `NA ANTENIE (N)` – liczba warstw na antenie, `⚠ BRAK POŁĄCZENIA` – utracone połączenie z serwerem. Przy utracie połączenia okno w tle prosi system o uwagę (miga w pasku zadań Windows, podskakuje w Docku macOS, wyróżnienie w pasku zadań Linuksa) do czasu przełączenia na nie albo powrotu połączenia.
- **Menu pod prawym przyciskiem (Trio):** systemowe menu na liście stron (wybiera stronę; TAKE, ARM – podgląd, CONTINUE / UPDATE / OUT – aktywne, gdy strona jest na antenie, Duplikuj, Usuń…) i na liście warstw na antenie (CONTINUE, OUT, powiązanie dock, Wdróż nową wersję szablonu). Polecenie dotyczy strony lub warstwy, dla której otwarto menu. W polach tekstowych zostaje menu WebView (kopiuj, wklej, pisownia); w przeglądarce – zwykłe menu przeglądarki. Pozycje podaje strona (komunikat `menu` mostu, najwyżej 24 pozycje, etykiety do 80 znaków).
- **Pamięć okna:** rozmiar, pozycja (tylko X11), maksymalizacja i powiększenie w `window.json` obok listy serwerów.
- **Jedna instancja (Linux, macOS):** kolejne uruchomienie na tym samym katalogu danych przekazuje prośbę działającemu programowi: okno na wierzch albo nowe okno Trio/wyjścia. Nie dotyczy `--connect`.
- **Otwieranie plików:** `vizrs-shell artist` i próba otwarcia pliku (.cgpack/.vizrs.json) kończą się komunikatem odsyłającym do VizRS CG Artist (kod 2). Skojarzenia plików `.cgpack` i `.vizrs.json` należą do paczki VizRS CG Artist. Przekazywanie plików do strony zostało usunięte z powłoki.
- **Skalowanie ułamkowe (Linux):** GTK3 zna tylko skale całkowite. Przy 125% strona rysuje się w skali 2, a kompozytor zmniejsza obraz, więc tekst jest ostry, ale nie co do piksela. Dokładne piksele przy skali ułamkowej daje okno Chromium (`vizrs trio`).
- **Ikona na Waylandzie:** bez instalacji okno zakłada ukryty wpis `~/.local/share/applications/vizrs-shell.desktop` i ikonę; istniejącego wpisu nie zmienia.
- **Pliki:** systemowe okno „Zapisz jako” obsługuje pobieranie plików zainicjowane przez stronę.
- **Wygląd:** strony wiedzą, że są w oknie natywnym (`window.__vizrsShell`) – chowają logo i zakładki Trio / Wyjście (są w menu Plik), piszą czcionką systemu (Segoe UI, SF Pro, Noto/Cantarell zamiast Inter; grafika i wyjście bez zmian) i nie pokazują menu przeglądarki pod prawym przyciskiem. Okno ma ciemny motyw (pasek tytułu na Windows, wygląd na macOS, ciemny wariant motywu GTK, o ile motyw go ma) i pokazuje się dopiero z narysowaną stroną (najpóźniej po 2,5 s) – bez białego prostokąta przy starcie; na Windows i macOS tło WebView przed stroną jest ciemne. Obrazków i linków nie da się wyciągnąć z okna, a strona nie „odbija” przy przewijaniu.
- **Windows (WebView2):** wyłączone skróty przeglądarki (Ctrl+F, Ctrl+P, Ctrl+R, Alt+←), więc F5 i inne klawisze mogą być skrótami stron Trio; Odśwież i Narzędzia deweloperskie są w menu Widok. Wyłączone podpowiedzi wcześniej wpisanych wartości w polach; listy rozwijane i paski przewijania WebView2 są ciemne niezależnie od motywu Windows. Dane silnika: `%LOCALAPPDATA%\VizRS\WebView2`.
- **Odnośniki:** strony serwera (np. „Wyjście ↗”) otwierają się w nowym oknie powłoki, adresy spoza serwera w przeglądarce systemowej. Strona nie ma dostępu do systemu – przez most okna przechodzi wynik skryptu testowego `--probe`, wybór serwera wyłącznie z wbudowanego okna połączenia (strony serwera nie mogą przełączyć okien), a ze strony Trio serwera biała lista komunikatów (`shell/src/bridge.rs`: stan okna, decyzja o zamknięciu, kopia robocza pod nazwą ze skrótu). Most w `source/web/native.js` woła `window.__vizrsPost` (okno Tauri VizRS CG Artist), inaczej `window.ipc.postMessage` (wry).
- **Pisownia:** na Linuksie polska (słownik hunspell `pl_PL` z systemu, pakiet `hunspell-pl`); na Windows i macOS według ustawień systemu.
- **Wierność:** WebKitGTK zmierzony z Chromium (OBS) na wszystkich stronach zestawów INFO24 i R7 – wszystkie pola tekstowe z identycznym ściśnięciem, szerokością i wysokością (`node tools/engines.js`). WebView2 to Chromium; WKWebView (macOS) – niezmierzony.
- **Wymagania:** Linux – biblioteki `libwebkit2gtk-4.1` i GTK 3 (są w typowym pulpicie; do budowy pakiety `-dev`); Windows – WebView2 Runtime (w Windows 10/11; instalator doinstalowuje go, gdy brak), bez pakietu Visual C++ Redistributable (biblioteka C wkompilowana statycznie, `source/.cargo/config.toml`); macOS 10.15+. Dane silnika WebView2 (pamięć podręczna, `localStorage`) okno trzyma w `%LOCALAPPDATA%\VizRS\WebView2`, nie obok programu – instalacja w `C:\Program Files` nie ma tam prawa zapisu.
- **Budowa:** `cd source/shell && cargo build --release` (osobny workspace z zależnościami – rdzeń `vizrs` dalej ich nie ma). Linux: `bin/linux-x86_64/vizrs-shell`; Windows: `build-windows.bat` buduje też `vizrs-shell.exe`.
- **Skróty:** `deploy/install-desktop-linux.sh` i `deploy\install-windows-shortcuts.ps1` używają okna natywnego dla Trio, gdy `vizrs-shell` leży obok `vizrs` (wymuszenie okna Chromium: `--chromium` / `-Chromium`). Skróty dotyczą Trio i serwera (skojarzenia plików i Artist należą do VizRS CG Artist).

## 13. Dane na dysku i kopie zapasowe

| Ścieżka | Zawartość |
|---|---|
| `vizrs-data/show.json` | aktywny show (zapis przez plik tymczasowy + `fsync`) |
| `vizrs-data/shows/` | biblioteka show: `<id>.json` na audycję (każdy zapis aktywnego show trafia też tutaj), `.active` = id aktywnego |
| `vizrs-data/assets/` | obrazy, wideo, fonty (png jpg jpeg gif webp svg mp4 webm ttf otf woff woff2; plik do 64 MB, łącznie do 2 GB) |
| `vizrs-data/backups/` | kopie: bieżące (20, nie częściej niż co 30 s), godzinowe (24), dzienne (14); czas warszawski (Europe/Warsaw); w godzinie cofnięcia zegara jesienią pliki z tej godziny nadpisują się |
| `vizrs-data/log.jsonl` | log audytowy komend |

- Brak `show.json` → serwer wczytuje demo INFO24. Uszkodzony `show.json` → zapisany jako `show.json.bak`, wczytane demo.
- Przywrócenie: Trio → ⚙ Stan → Kopie zapasowe → Przywróć (stan sprzed przywrócenia trafia do najnowszej kopii) albo `POST /api/backups/restore/<nazwa>`.
- Nieudany zapis: czerwony znacznik w Trio, API zwraca 500; antena działa dalej.

Gotowe oprawy (`.cgpack`) i sposób ich wczytania: [examples/README.md](../examples/README.md).

## 14. Testy i narzędzia

```bash
cd source
cargo clippy --all-targets -- -D warnings
cargo test                                           # silnik, API, JSON, walidacja, kopie
node tests/vectors/run.js                            # wektory modelu tekstu
node tests/cgpack.mjs                                # reguły paczki .cgpack (pliki, import)
node tests/e2e/run.js target/release/vizrs           # E2E Trio/Output (Playwright)
node tests/e2e/mobile.js target/release/vizrs [katalog]   # E2E Trio na telefonie (emulacja dotyku; katalog = zrzuty)
node tests/e2e/artist.js target/release/vizrs        # E2E Artist
node tests/e2e/pack.js target/release/vizrs          # E2E paczki .cgpack (eksport, import, font na /output)
node tests/motion.js                                 # ruch: krzywe, presety, efekty tekstu, pętle, sekwencja
node tests/e2e/motion.js target/release/vizrs        # E2E ruchu (panel Artist, /output, klatki lekkiego wyjścia ?fps/?fx=step)
node tests/e2e/golden.js target/release/vizrs [--update]   # zrzuty wzorcowe
node tests/e2e/app.js target/release/vizrs          # okna aplikacji (udawana przeglądarka)
node tests/e2e/shell.js target/release/vizrs shell/target/release/vizrs-shell   # okno natywne w działaniu
node tests/e2e/native-smoke.js target/release/vizrs shell/target/release/vizrs-shell   # test dymny okna (CI: Windows z instalatora --installed, macOS)
node tools/engines.js target/release/vizrs shell/target/release/vizrs-shell     # zgodność WebView okna z Chromium (0 różnic)
(cd render && cargo clippy --all-targets -- -D warnings && cargo test && cargo build --release)   # renderer natywny (§10.1)
node tools/native-compare.js                         # zgodność renderera natywnego z Chromium (strony INFO24)
```

| Narzędzie | Do czego |
|---|---|
| `tools/build-packs.mjs [katalog]` | budowa paczek `examples/INFO24.cgpack`, `R7_NEWS.cgpack` z `examples/archiwum/` |
| `tools/gallery.js [vizrs] [--out katalog]` | galeria wszystkich stron zestawów przykładowych (zrzut każdej strony po wejściu, arkusze, `index.html`); wynik poza repo, domyślnie `<tmp>/vizrs-galeria` |
| `tools/video.js [vizrs] [--only INFO24,R7] [--fps 25]` | film ze wszystkimi paczkami (każda strona TAKE → CONTINUE → OUT, 1920×1080), nagrywany klatka po klatce w czasie wirtualnym; wymaga ffmpeg. Lista paczek: `tools/sets.js` |
| `tools/native-cases.js <katalog>` | plansze testowe renderera natywnego (12 stron wariantów: pola, gradienty, cienie, przekształcenia, maski, dopasowanie tekstu, fonty, znaczniki, obrazy, QR, autofollow) – do `native-compare.js <katalog>/show.json --assets <katalog>/assets` |
| `tools/native-widths.js [vizrs]` | szerokości tekstu Chromium i renderera natywnego (`render/examples/measure`) dla fontów wbudowanych, grubości, rozmiarów ułamkowych, odstępów i kursywy |
| `tools/native-video.js [vizrs] [--only INFO24,R7] [--out plik.mp4] [--stats plik.csv]` | film porównawczy renderera natywnego (§10.1) z Chromium: każda strona TAKE → CONTINUE → OUT, oba renderery w tych samych chwilach (czas wirtualny), kadr 2×2 – Chromium, natywnie, różnica ×4, opis strony (czy cała natywnie, czas klatki); `--stats`: błąd (MSE) każdej klatki do CSV i podsumowanie stron (zrzuty PNG); wymaga ffmpeg |
| `tools/out_choreo.py plik.show.json` | wygenerowanie animacji OUT całego show |
| `tools/render_video.py` + `steps_*.py` | filmy demonstracyjne (np. IN, potem OUT zwolnione 2,5×) |
| `tools/gen_fonts.py` | wbudowane fonty (`fonts_data.rs`) |

Pełna bramka przed commitem: [../CLAUDE.md](../CLAUDE.md#testy-bramka-przed-commitem). Zapisana galeria: `media/galeria/index.html`, film: `media/vizrs_WSZYSTKIE_PACZKI.mp4`.

CI: `.github/workflows/ci.yml` (Linux, macOS, Windows).

## 15. Znane ograniczenia

Aktualna lista, uszeregowana od niezawodności do funkcji: [LUKI.md](LUKI.md).
