# VizRS – dokumentacja aplikacji

> Wersja 2.0.0 + zmiany „Niewydane” z [CHANGELOG.md](../CHANGELOG.md). Dokument opisuje stan z kodu (`source/`).
> Instrukcja krok po kroku dla nowych osób: [INSTRUKCJA-poczatkujacy.md](INSTRUKCJA-poczatkujacy.md).
> API: [API.md](API.md) · Companion: [COMPANION.md](COMPANION.md) · awarie: [RUNBOOK.md](RUNBOOK.md) · braki: [LUKI.md](LUKI.md).

## Spis treści

1. [Czym jest VizRS](#1-czym-jest-vizrs)
2. [Architektura](#2-architektura)
3. [Model danych (show)](#3-model-danych-show)
4. [Scena: węzły, właściwości, animacja](#4-scena-węzły-właściwości-animacja)
5. [Pola kontrolne i reguły pól](#5-pola-kontrolne-i-reguły-pól)
6. [Tekst](#6-tekst)
7. [Ticker](#7-ticker)
8. [Artist – projektowanie scen](#8-artist--projektowanie-scen)
9. [Trio – playout](#9-trio--playout)
10. [Wyjście dla OBS i mikserów](#10-wyjście-dla-obs-i-mikserów)
11. [Sterowanie zewnętrzne](#11-sterowanie-zewnętrzne)
12. [Import z BTB CG](#12-import-z-btb-cg)
13. [Uruchamianie, opcje, bezpieczeństwo](#13-uruchamianie-opcje-bezpieczeństwo)
14. [Dane na dysku i kopie zapasowe](#14-dane-na-dysku-i-kopie-zapasowe)
15. [Testy i narzędzia](#15-testy-i-narzędzia)
16. [Znane ograniczenia](#16-znane-ograniczenia)

---

## 1. Czym jest VizRS

VizRS to system grafiki telewizyjnej (CG) na żywo, wzorowany na duecie Vizrt:

| Vizrt | VizRS | Do czego |
|---|---|---|
| Viz Artist | **Artist** (`/artist`) | projektowanie szablonów: drzewo kontenerów, klucze animacji, stop pointy, pola kontrolne |
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

Kolejność warstw na wyjściu: `back` (10) < `main` (20) < `front` (30); inne nazwy mają poziom 25 (między `main` a `front`).

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
| **scena** | `id`, `name`, `layer`, `width`, `height`, `duration`, `stops[]`, `markers[]`, `dock[]`, `packAssets[]` (pliki dołączone do paczki .cgpack), `fieldMeta{}`, `nodes[]`, `rev` | warstwa: litery, cyfry, `-`, `_` (maks. 50) |
| **strona** | `id`, `number` (całkowita), `name`, `scene` (id sceny), `values{}`, `hotkey`, `note`, `rev` | `values` tylko wartości proste (tekst, liczba, tak/nie) |
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
| `clock` | Zegar | tryb, format, start, cel, styl tekstu | nie |
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

**Tekst** (`text`, `clock`; częściowo `ticker`): `font`, `size`, `weight` (300–900), `italic`, `upper`, `spacing`, `lineHeight`, `align`, `valign`, `textFit`, `maxLines`, `squeezeMin`, `lineBreak`, `fillSqueeze`, `padX`, `hlColor`/`hlText` (kolory `[[wyróżnienia]]`), `cycle` (rotacja wierszy co N s z przenikaniem).

**Obraz:** `src` (plik z assets albo adres `http(s)://`), `fitMode` `contain`/`cover`/`fill`, `loop` (wideo `.mp4`/`.webm`/`.mov`).

**Zegar:** `mode` – `time` (godzina), `up` (stoper od `seconds`), `down` (odliczanie od `seconds`), `target` (do daty `target`); `format`:
- tryb `time`: `yyyy MM dd HH H mm ss`;
- tryby licznikowe: `Dn` („3 dni”), `D`, `HH`, `H`, `mm`, `ss`, `S` (dziesiąte);
- tekst w apostrofach jest dosłowny: `D 'dni' HH:mm:ss`.

**Ticker:** patrz [§7](#7-ticker).

### 4.4 Animacja i stop pointy

- Scena ma `duration` i listę `stops`. **TAKE** gra od 0 do pierwszego stopu, **CONTINUE** do kolejnego, **OUT** od ostatniego stopu do końca sceny i zdejmuje grafikę.
- Bez stop pointów: TAKE gra całość i grafika zostaje; OUT ją wycina.
- Krzywe: `inout` (domyślna), `out`, `in`, `linear`, `back` (z przestrzeleniem), `hold` (skok na następnym kluczu).
- Animować można liczby i kolory (kolory mieszane w RGBA); pozostałe właściwości przełączają się w czasie klucza.
- **Markery** (`markers[{t, name}]`) – znaczniki na osi; z markera można utworzyć stop point.
- **Presety animacji** – biblioteka zestawów kluczy (czasy względne) wspólna dla stanowiska (`/api/animpresets`): zapis z węzła, wstawienie od bieżącego czasu, import/eksport pliku.
- **Wyjście (OUT)** jest projektowane w scenie (klucze za ostatnim stop pointem). Dla gotowych zestawów `tools/out_choreo.py` generuje wyjścia całego show: teksty gasną z odjazdem, tła zmazują się maską zgodnie z wejściem.

### 4.4a Ruch jak w BTB CG (presety bez kluczy)

Obok kluczy węzeł może mieć **ruch opisany presetami** – tak jak zakładka „Ruch” w BTB CG Editor (`node.motion`, moduł `web/motion.js`, ta sama semantyka co `cgmotion.js` i btb-cgeditor `anim.rs`). Renderer liczy go z czasu sceny, więc działa jednakowo w Artist (przewijanie osi), w podglądzie Trio i na wyjściu OBS.

| Sekcja | Pola | Czas |
|---|---|---|
| `in` – wejście | `type`, `dir` (left/right/top/bottom), `dur`, `delay`, `ease` | od początku sceny (TAKE) |
| `out` – wyjście | jak `in` | od **ostatniego stop pointu** (OUT) |
| `textIn` / `textOut` – efekt tekstu | `type`, `dur`, `delay`, `stagger` (odstęp jednostek, puste = auto: znaki 0,04 s, słowa 0,09 s), `from` (start/end/center/edges) | jak wejście / wyjście |
| `loop` – pętla uwagi | `type`, `dur` (okres), `amp` (siła), `color` | zegar ścienny, gdy grafika stoi (po wejściu, przed wyjściem) |
| `after` | id węzła | wejście zaczyna się po wejściu wskazanego węzła (sekwencja) |
| `origin` | `[x, y]` (0–1) | punkt odniesienia skali/obrotu ruchu, gdy preset go nie ustala (Artist: „punkt ruchu”); import BTB: `[0, 0]` dla ruchu całego elementu |
| `bg` | `shimmer`, `pulse`, `glow`, `scanline` | stały efekt tła kształtu jak `effect` warstwy BTB (Artist: „efekt tła”) – gra od wejścia na wizję, także w trakcie IN/OUT |

- **Presety IN/OUT (22):** wjazd, przenikanie, zoom, zoom z bliska, wyskok, przesłony pozioma i pionowa, przycięcie, rozmycie, odwrócenia X/Y (3D), obrót, skos, spadanie, glitch, wjazd z przeskokiem, rozsunięcie, maska ukośna, żaluzja, rosnący pasek, rozdzielenie RGB.
- **Efekty tekstu (31):** wartości jak `cgmotion.js` na antenie BTB (maski `inset`, rozmycie 20 px całości i 14 px jednostek, przesunięcia 50/40 px): całość (odsłonięcie, maszyna do pisania, maszyna z kursorem, z maski, przenikanie, rozmycie, skala, przesunięcia), **słowa / znaki / linie** (w górę, w dół, przenikanie, rozmycie, skala, z maski), szyfrowanie (scramble), licznik (pierwsza liczba w tekście liczy od 0 z zachowaniem formatu, np. „1 250,5”). Tekst jest dzielony tylko na czas efektu – potem wraca dokładny układ (ściskanie, łamanie wierszy).
- **Pętle uwagi (11):** pulsowanie, oddychanie, bicie serca, unoszenie, podskakiwanie, chwianie, drżenie, błyski, poświata, połysk, przesuwane podkreślenie. Wchodzą płynnie po wejściu i schodzą przed wyjściem; siła: pulsowanie/bicie serca – % skali, chwianie – stopnie, oddychanie – % krycia, unoszenie/podskakiwanie/drżenie – px.
- **Krzywe (nazwy GSAP):** `power2.out` (domyślna), `power2.in`, `power2.inOut`, `power3.out`, `power4.out`, `expo.out`, `sine.inOut`, `linear`, `back.out`, `back.in`, `bounce.out`, `elastic.out` oraz aliasy BTB `spring`, `overshoot`, `broadcast`; przyjmowane są też `back.out(2)` i `cubic-bezier(…)`.
- **Składanie z kluczami** (jak w BTB): przesunięcie, obrót, pochylenie i rozmycie się sumują, skala i krycie mnożą, maski dają część wspólną.
- Wyjście gra tylko, gdy scena ma stop point i czas po nim. Artist podpowiada **⟲ Dopasuj stop i długość** (stop po najdłuższym wejściu, długość = stop + najdłuższe wyjście).
- Walidacja (`validate.rs`): typy z list, kierunki, czasy 0–60 s, krzywa tylko z liter, cyfr i `.(),-`, nieznane pola odrzucane.

### 4.5 Warianty

Pole wyboru na poziomie sceny (`fieldMeta.<pole> = {type: "select", options: [...], default}`), np. `styl = info, pilne`. Węzły mają „widoczny gdy” (`showIf`). W Trio wariant to rząd przycisków.

### 4.6 Powiązania (dock)

`scene.dock = [{ target, dx, dy, dur }]` – scena przesuwa się o (`dx`, `dy`) w czasie `dur` s, gdy na antenie jest wskazana grafika:
`layer:<warstwa>`, `page:<numer>` albo `scene:<nazwa>`. Przykład: belka unosi się o 90 px, gdy na warstwie `back` jest ticker.
Operator może wyłączyć powiązanie na antenie (⇅ w Trio, `/api/dock/<warstwa>/off`). Stan jest odtwarzany po odświeżeniu wyjścia.

### 4.7 Fonty

- Wbudowane offline: **Oswald, Barlow Condensed, Inter, Roboto, Roboto Condensed, Montserrat** (`/fonts/fonts.css`).
- Fonty systemowe (Arial, Verdana…) – bez ładowania.
- Inne nazwy z Google Fonts ładują się z internetu (na antenie lepiej tego unikać).
- Własny font: wgraj `.ttf/.otf/.woff/.woff2` do assets i wpisz w polu „font” nazwę pliku, np. `MojFont.ttf`. Font wgrany, gdy Trio lub wyjście są już otwarte, wczyta się sam w ciągu kilkunastu sekund (bez odświeżania strony).
- **Font jedzie z serwerem, nie z komputerem OBS:** wyjście `/output` pobiera wszystkie fonty z assets z serwera VizRS zaraz po otwarciu (i sprawdza nowe co 30 s), więc komputer renderujący nie musi mieć fontu zainstalowanego, a pierwszy TAKE nie czeka na pobranie. Fonty przenosi między stanowiskami paczka belek `.cgpack` (§8.6).

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
| `squeeze` | jak w BTB CG: ściska tekst poziomo, najwyżej do `squeezeMin` (domyślnie 60%); **font się nie zmienia**. Gdy to za mało, tekst wystaje poza ramkę, a Trio ostrzega. Domyślny tryb nowego tekstu w Artist |
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

### 8.1 Układ

| Obszar | Zawartość |
|---|---|
| Pasek górny | wybór sceny (nazwa · warstwa), ＋ Nowa, ⧉ duplikuj, 🗑 usuń, ⇩ Eksport (JSON sceny), ⇩ Paczka (paczka belek `.cgpack`, §8.6), ⇧ Import (JSON sceny albo `.cgpack`), ⇧ BTB (import z BTB CG), ↶ ↷, ▢ Safe (obszary bezpieczne 5% / 10%), **Zapisz** |
| Drzewo sceny (lewo) | ＋ Dodaj… (typy węzłów i presety tickerów), ↑ ↓ (kolejność), ⇤ wyjmij z grupy, ⇥ włóż do poprzedniej grupy, ⧉, 🗑; przeciąganie zmienia rodzica z zachowaniem położenia |
| Kanwa (środek) | podgląd sceny w czasie bieżącym; zaznaczanie i przeciąganie elementów; kliknięcie pustego miejsca = ustawienia sceny |
| Właściwości (prawo) | grupy: Transform, Maska / efekty, Autofollow, Wygląd, Tekst, Obraz, Zegar, Ticker; „widoczny gdy”; presety animacji |
| Oś czasu (dół) | czas, ⏮, ▶ Take, ⏭ Continue, ⏏ Out, ▶/⏸, ＋ Stop, długość, edytor klucza, ◆ Marker, ⧉ Kopiuj / ⎘ Wklej klucze, zoom |

### 8.2 Ustawienia sceny (kliknij puste tło kanwy)

Nazwa, **warstwa**, rozdzielczość, długość, stop pointy (× usuwa), markery (⏹ = stop point z markera), **powiązania (dock)**, **warianty** (＋ Wariant), **pola kontrolne** (etykieta w Trio + ⚙ reguły), **zasoby paczki** (grafiki i czcionki, §8.6) i **＋ Utwórz stronę w Trio**.

Właściwości węzła mają grupę **Ruch (jak w BTB CG)**: wejście, wyjście, tekst IN/OUT (tekst i zegar), pętla – z czasem, opóźnieniem, kierunkiem, krzywą, odstępem i kolejnością jednostek (§4.4a).

### 8.3 Klucze animacji

- **◆** przy właściwości = klucz w bieżącym czasie. Gdy właściwość ma już animację, zmiana wartości tworzy lub aktualizuje klucz automatycznie.
- Dwuklik na ścieżce = nowy klucz. Klik w klucz → edytor: czas, wartość, krzywa do następnego klucza, 🗑.
- Zaznaczanie wielu kluczy: Shift+klik albo Shift+przeciągnięcie ramką; Ctrl+C / Ctrl+V kopiuje je na zaznaczony węzeł od bieżącego czasu.
- Stop point: ＋ Stop w bieżącym czasie; ▼ na linijce przeciągasz, dwuklik usuwa.
- Marker: **M** albo dwuklik na linijce; klik = skok, dwuklik = zmiana nazwy / usunięcie.
- Zoom osi: suwak, Ctrl+kółko, ↔ dopasuj.

### 8.3a Tryb sekwencji (prosty, jak „Ruch” w BTB CG Editor)

Przycisk **≣ Sekwencja** na osi czasu przełącza widok kluczy na widok ruchu bez kluczy (§4.4a). Wybór jest pamiętany na stanowisku.

- **Wiersz na element** (z wcięciem wg drzewa): niebieski pasek – wejście, pomarańczowy – wyjście (za ostatnim stop pointem), cienkie paski pod nimi – efekt tekstu IN/OUT, kreskowane tło – pętla uwagi (od końca wejścia do wyjścia). Etykieta pokazuje „↳ węzeł” (wchodzi po nim) i ◆N (węzeł ma też klucze).
- **Przeciągnij pasek** – zmienia opóźnienie; **przeciągnij krawędź** – czas trwania (Shift = co 0,05 s). Klik w pasek zaznacza element i ustawia głowicę na jego starcie; klik w puste miejsce przewija czas.
- Narzędzia (działają na dzieciach zaznaczonej grupy, rodzeństwie zaznaczonego elementu albo całej scenie – przy jednej grupie głównej na jej dzieciach):
  - **⇶ Kaskada** – wejścia jedno po drugim co **krok** s (wyjścia w odwrotnej kolejności); elementy bez ruchu dostają przenikanie.
  - **↳ Po kolei** – każdy element wchodzi po zakończeniu wejścia poprzedniego (`motion.after`); przesunięcie liczy się samo, także gdy zmienisz czas poprzednika.
  - **⧉ Kopiuj** – ruch zaznaczonego elementu na jego rodzeństwo (efekty tekstu tylko na tekstach).
  - **◆ Klucze** – presety wejścia/wyjścia zaznaczonego elementu zamienione na edytowalne klucze (jak „Presety na klucze” w BTB): typy liniowe z krzywą `power2.*`/`linear`/`back.out` dają dwa klucze, pozostałe – klucze co 1/30 s; odwrócenia 3D, przesłony, maska ukośna, żaluzja, rosnący pasek, RGB i glitch zostają presetami (raport). Pętle i efekty tekstu zostają. Węzeł z kluczami na tych samych właściwościach – odmowa.
  - **✕** – usuwa ruch z zaznaczonego elementu (bez zaznaczenia – z całej grupy).
  - **⟲ Dopasuj** – stop point po najdłuższym wejściu, długość = stop + najdłuższe wyjście.
- W panelu **Ruch** węzła pole **po węźle** ustawia sekwencję ręcznie.

### 8.4 Skróty Artist

| Skrót | Działanie |
|---|---|
| Ctrl+S | zapisz scenę |
| Ctrl+Z / Ctrl+Y (Ctrl+Shift+Z) | cofnij / ponów |
| Ctrl+D | duplikuj węzeł |
| Delete / Backspace | usuń węzeł (lub zaznaczony klucz, gdy aktywna jest oś czasu) |
| Spacja | odtwórz / pauza |
| Home / End | początek / koniec sceny |
| M | marker w bieżącym czasie |
| Ctrl+C / Ctrl+V (na osi czasu) | kopiuj / wklej klucze |
| strzałki (Shift = ×10) | przesuń zaznaczony węzeł o 1 px (10 px) |

### 8.5 Zmiana szablonu, który jest na antenie

Zapis sceny nie zmienia grafiki na antenie: warstwa dostaje znacznik **⏳ nowa wersja** w Trio, a operator wybiera moment **Wdróż teraz** (`/api/deploy/<warstwa>` – podmiana bez ponownej animacji). Kolejny TAKE używa już nowej wersji.

### 8.6 Paczka belek (.cgpack)

Paczka przenosi belki między stanowiskami i serwerami jednym plikiem – razem z grafikami i czcionkami.

- **⇩ Paczka** otwiera listę scen (bieżąca zaznaczona), opcję „dołącz strony z Trio” i podsumowanie (ile scen, grafik, czcionek). **Pobierz .cgpack** zapisuje plik. Niezapisane zmiany bieżącej sceny trafiają do paczki w wersji z edytora.
- Do paczki trafiają pliki z assets, których scena używa (obrazy i wideo w `src`, fonty w `font`, odnośniki `/assets/…` w tekstach, wartości stron) oraz **dołączone** ręcznie.
- **Zasoby paczki** w ustawieniach sceny: lista „Używane” (wykryte) i „Dołączone” (`scene.packAssets`, × odłącza). **＋ Wgraj i dołącz** wgrywa nowy plik (grafika, wideo, czcionka TTF/OTF/WOFF/WOFF2) do assets i dołącza go do sceny; lista „dołącz z assets…” dołącza plik już wgrany. Dołączaj pliki, których scena nie wskazuje wprost – np. czcionkę dla znaczników tekstu albo grafiki, które operator wybiera w Trio. „brak!” = pliku nie ma w assets.
- **⇧ Import** przyjmuje `.cgpack`: pliki trafiają do assets (ten sam plik o tej samej treści jest pomijany, inna treść pod tą samą nazwą → `nazwa_2.ext` i przepisane odnośniki), sceny są dodawane jako nowe (istniejące nie są nadpisywane), strony dostają swój numer albo nowy, gdy numer jest zajęty. Raport pokazuje, co się zmieniło. Paczka elementu BTB CG wczytana przez ⇧ Import trafia do importu BTB (§12).
- Czcionki z paczki serwuje serwer (`/assets/<plik>`); wyjście `/output` ładuje je zawczasu (§4.7).

Format (ZIP): `manifest.json` (`_type: "vizrs_pack"`, `_packVersion: 1`, lista scen i plików z sumą CRC-32), `scenes.json` (`{ scenes, pages }` jak w show), `assets/<nazwa>`. Odczyt sprawdza ścieżki, typy plików, sumy i limity (200 plików, 256 MB). To format VizRS – nie jest eksportem do BTB CG.

## 9. Trio – playout

### 9.1 Układ

| Kolumna | Zawartość |
|---|---|
| Lewa | **Strony** (pole `nr` + Enter = wczytaj, szukaj, ⧉ Duplikuj, 🗑) i **Szablony** (＋ = nowa strona z wartościami domyślnymi) |
| Środkowa | edytor strony: numer, nazwa, skrót klawiszowy, Zapisz, Przywróć zapisane; status (szablon, warstwa, ● ON AIR, 🔒); **pola**; duży **podgląd** strony; przyciski TAKE / CONTINUE / OUT / UPDATE / ARM |
| Prawa | monitor **PGM** (rzeczywiste wyjście, wybór tła monitora), **Na antenie – warstwy** (⏭ continue, ⏏ out, ⇅ dock, Wdróż teraz), zakładki **Playlista / Rundown / Makra** |
| Pasek górny | nazwa show, stan połączenia, „⚠ zapis show nieudany”, „⚠ błędy scen”, ⚙ Stan, ⇩ Eksport show, ⇧ Import show, ✖ CLEAR ALL |

### 9.2 Praca ze stroną

1. Wczytaj stronę: klik na liście, numer + Enter albo ↑ ↓.
2. Zmień pola. Podgląd odświeża się na bieżąco; klik w element podglądu przenosi kursor do jego pola.
3. **TAKE** (F1 / Ctrl+Enter / dwuklik na liście) wysyła na antenę **bieżące wartości z edytora**, także niezapisane. Zapis strony to osobna czynność (Ctrl+S).
4. **CONTINUE** (F2) – następny stop point; **OUT** (F3) – wyjście.
5. **UPDATE** (F4) podmienia dane strony na antenie bez ponownej animacji (bez zapisu strony).
6. **ARM** (F6) wysyła stronę na wyjście Preview (`/output?bus=preview`), antena się nie zmienia.

TAKE strony z warstwy, na której już coś jest, zastępuje poprzednią grafikę tej warstwy. Różne warstwy grają jednocześnie.

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

**Skrót strony:** kliknij pole „skrót” w nagłówku edytora i naciśnij kombinację (Ctrl/Alt/Shift + klawisz albo F7, F9–F12); Backspace usuwa skrót.

### 9.5 Playlista, rundown, makra

- **Playlista (karuzela):** strony z czasami trwania, opcjonalnie w pętli; ▶ Start / ■ Stop. Odtwarza ją serwer – działa także po zamknięciu Trio.
- **Rundown:** kolejka stron programu. Przeciąganie zmienia kolejność, notatki przy pozycjach, stany: ● na antenie, ▶ następna, ✓ odegrana. **NEXT** (F8) bierze następną pozycję; ↺ zeruje stany; import/eksport CSV (`numer;notatka`). Bez automatu czasowego – zawsze ręczne NEXT.
- **Makra:** jedna komenda w wierszu, opcjonalnie z opóźnieniem: `CLEAR`, `TAKE 1001`, `+2.5 TAKE 3001`, `+6 OUT main,back`. **⇥ z anteny** tworzy makro z bieżącego stanu anteny (preset segmentu). Nowe makro przerywa poprzednie.

### 9.6 ⚙ Stan systemu

Zdrowie serwera i wyjść, błędy scen (z przyciskiem Wyczyść), lista Word Replace, kopie zapasowe show (Przywróć), ostatnie komendy z logu audytowego.

### 9.7 Wielu operatorów

Edytowana strona jest blokowana miękko: inni widzą 🔒 z imieniem; zapis cudzej strony wymaga potwierdzenia. Blokada wygasa po 45 s bez aktywności. Konflikt zapisu → wybór „nadpisz moją wersją / zachowaj zmiany”.

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

## 11. Sterowanie zewnętrzne

Pełna lista: [API.md](API.md). W skrócie:

- **HTTP** (także GET – Companion/Stream Deck): `/api/take/1001?name=Jan`, `/api/continue/main`, `/api/out/main`, `/api/update/5001?homeScore=3`, `/api/clear`, `/api/rundown/<id>/next`, `/api/macro/<nazwa>`, `/api/state`, `/api/variables`.
- **TCP :6100** (linia = komenda): `TAKE 1001`, `TAKE 1001 name=Anna|title=Ekspertka`, `TAKE 1001,3001`, `CONTINUE main`, `OUT main`, `UPDATE 5001 homeScore=3`, `LIVE 5001 homeScore=4` (bez zapisu), `TICKER …`, `NEXT <rundown>`, `MACRO <nazwa>`, `CAROUSEL START <playlista>`, `CLEAR`, `STATE`, `PING`.
- **OSC** (`--osc 8000`): `/vizrs/take 1001`, `/vizrs/out main`, `/vizrs/next <rundown>`… (przy tokenie `/vizrs/<token>/…`).
- **Webhook** (`--webhook http://…`): `onAir`, `offAir`, `error`.
- **SSE** `/events`: strumień zdarzeń (`snapshot`, `take`, `continue`, `out`, `update`, `rescene`, `dock`, `show`, `state`, `error`…).
- Companion: [COMPANION.md](COMPANION.md).

## 12. Import z BTB CG

VizRS **importuje** grafiki z BTB CG; eksportu w drugą stronę nie ma.

- Artist → **⇧ BTB**: element `.json` (eksport v3) albo `.cgpack` → scena + strona, z raportem konwersji (ostrzeżenia o tym, czego nie przeniesiono).
- Przenoszone: układ, teksty z regułami, **ruch 1:1** (presety IN/OUT warstw i elementu z kierunkiem, czasem, opóźnieniem i krzywą, efekty tekstu, pętle uwagi, „po warstwie” przy wejściu → `node.motion`, §4.4a; stop point po wejściu, długość = stop + wyjście), zegary (`HH:MM` → `HH:mm`), tickery (tryby `horizontal`→crawl, `vertical`→roll, `whip`, kategorie, kolory, etykieta, błysk), obrazy.
- Ściskanie: warstwa z `squash` → `textFit: squeeze` z granicą `minSquash` (0 i brak granicy → bez granicy, w VizRS 0,3 – jak migracja BTB `upgrade-minsquash`); BADGE (BTB ściska wg długości, min. 50%) → ściskanie do wnętrza tła z granicą 0,5; tekst z auto-szerokością (`autoW`) → `squeeze` w szerokości `maxW` z granicą 0,5.
- Warstwy tekstu bez pola (w BTB edytowane w WYSIWYG) dostają pola `tekst1`, `tekst2`… (etykiety „Tekst 1”, „Tekst 2”), a przy wyrównaniu do lewej szerokość do krawędzi tła.
- Ruch całego elementu jak w BTB: punkt odniesienia w lewym górnym rogu (`motion.origin`), a przy skali layoutu ≠ 1 ruch w wewnętrznej grupie „ruch elementu” (przesunięcia skalowane jak `.cg-scene` w BTB). Efekty tła kształtów (`shimmer`, `pulse`, `glow`, `scanline`) → `motion.bg`. Efekt tekstu bez czasu/opóźnienia dostaje domyślne BTB (IN 1 s po 0,2 s, OUT 0,5 s).
- Nieprzenoszone (ostrzeżenia): maski warstw, `layout.dock`, auto-szerokość `follow`, rozmiar `{sz=}` w px, „po warstwie” przy wyjściu (start równoczesny). Typ ruchu spoza listy → `fallback` z BTB albo przenikanie.
- Zgodność sprawdza `tests/e2e/btb-visual.js` (stan po wejściu), z `--frames` także klatki 25/50/75% wejścia przy wspólnym zegarze obu stron.
- Paczka wzorców: [examples/BTB_CG](../examples/BTB_CG/README.md) – 31 elementów, strony 8001–8402, warstwy `main`/`badge`/`ticker`/`clock`/`logo`.

## 13. Uruchamianie, opcje, bezpieczeństwo

```bash
./start-linux.sh                       # Linux: gotowa binarka bin/linux-x86_64/vizrs
build-windows.bat → start-windows.bat  # Windows (wymaga Rust)
./build-macos-linux.sh                 # macOS / inne: bin/native/vizrs
```

Komendy: `vizrs` albo `vizrs serve` – serwer bez GUI (UI w przeglądarce); `vizrs --healthcheck` – sprawdza `/health` działającego serwera (kod wyjścia 0 = ok, 1 = błąd; dla Dockera i monitoringu).

| Opcja | Zmienna | Domyślnie | Opis |
|---|---|---|---|
| `--port` | `VIZRS_PORT` | 8088 | port HTTP (UI + wyjście) |
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
| `--healthcheck` | | | sprawdź `/health` serwera pod `--bind`/`--port` (z `--token`) i zakończ |
| `--version` | | | wersja i skrót commita |

Flaga ma pierwszeństwo przed zmienną; pusta zmienna = nieustawiona. Opcje można też pisać jako `--port=8099`. Nieznana opcja albo zła wartość (np. port spoza 0–65535) kończy start z błędem (kod 2) – literówka nie przejdzie po cichu.

**Jeden serwer na katalog danych:** serwer zakłada `<data>/.vizrs.lock` (pid, adres, identyfikator instancji – ten sam co pole `instance` w `/health`). Drugi serwer na tym samym `--data` nie wystartuje (kod 3) i poda adres działającego. Plik po awarii (`kill -9`, zanik zasilania) jest rozpoznawany jako martwy i zastępowany przy następnym starcie. Gdy pliku nie da się zapisać, serwer działa dalej bez blokady (ostrzeżenie).

Token: przeglądarka – raz `?token=…` (zapisuje ciasteczko); API – `Authorization: Bearer`, `X-Token` albo `?token=`; TCP – pierwsza linia `AUTH …`.
**Token wyjścia** (`--output-token`, wymaga `--token`): pozwala tylko na to, czego potrzebuje strona wyjścia – `/output`, skrypty i style, `/fonts/*`, `/assets/*`, zdarzenia `/events`, lista grafik i zgłaszanie błędów scen. TAKE, zmiany show, Trio i Artist z tym tokenem → 403. W OBS/vMix wpisuj `…/output?token=<token wyjścia>` – wyciek adresu (kolekcja scen, zrzut ekranu) nie daje sterowania anteną.

**Ochrona w sieci publicznej:** po 5 nieudanych próbach tokenu z jednego adresu każda kolejna odpowiedź 401 jest opóźniana (do 3 s; licznik zeruje się po 10 min; poprawny token działa od razu), próby trafiają do logu audytowego; najwyżej 128 jednoczesnych połączeń z jednego adresu (poza `127.0.0.1` i trybem `--trust-proxy`); cały nagłówek żądania musi dojść w 15 s. Przy tokenie krótszym niż 16 znaków i nasłuchu w sieci serwer ostrzega na starcie.
**Zatrzymanie:** SIGTERM/SIGINT (Docker, systemd, Ctrl+C) kończy pracę od razu i zwalnia blokadę katalogu danych (zapisy show są atomowe).

Niezależnie od tokenu serwer odrzuca żądania z obcych witryn (`Sec-Fetch-Site`, `Origin`). VizRS sam nie obsługuje TLS – w sieciach współdzielonych i na VPS stawiaj go za reverse proxy z HTTPS (Caddy w `deploy/docker-compose.yml`).
Usługa systemowa: `deploy/` (systemd, launchd, Windows/NSSM).

**Docker / VPS:** `docker build -t vizrs .` (statyczna binarka musl na pustym obrazie, ~3 MB, użytkownik 10001, healthcheck `vizrs --healthcheck`, dane w wolumenie `/data`, port komend TCP domyślnie wyłączony). Serwer z HTTPS: `deploy/docker-compose.yml` (VizRS + Caddy, Let's Encrypt), przykład nginx: `deploy/nginx-vizrs.conf`. Instrukcja krok po kroku: [RUNBOOK.md](RUNBOOK.md#vps-i-docker).

## 14. Dane na dysku i kopie zapasowe

| Ścieżka | Zawartość |
|---|---|
| `vizrs-data/show.json` | show (zapis przez plik tymczasowy + `fsync`) |
| `vizrs-data/assets/` | obrazy, wideo, fonty (png jpg jpeg gif webp svg mp4 webm ttf otf woff woff2; łącznie do 2 GB) |
| `vizrs-data/backups/` | kopie: bieżące (20, nie częściej niż co 30 s), godzinowe (24), dzienne (14); czas UTC |
| `vizrs-data/log.jsonl` | log audytowy komend |

- Brak `show.json` → serwer wczytuje demo INFO24. Uszkodzony `show.json` → zapisany jako `show.json.bak`, wczytane demo.
- Przywrócenie: Trio → ⚙ Stan → Kopie zapasowe → Przywróć (stan sprzed przywrócenia trafia do najnowszej kopii) albo `POST /api/backups/restore/<nazwa>`.
- Nieudany zapis: czerwony znacznik w Trio, API zwraca 500; antena działa dalej.

Zestawy przykładowe: demo **INFO24** (wbudowane), **R7 NEWS** (`examples/R7_NEWS.show.json`, strony 7001–7505), **BTB_CG** (`examples/BTB_CG/`), **belka BTB wielowierszowa** (`examples/BTB_PILNE.show.json`, strony 8501–8506: „Belka - PILNE” z paczki BTB_CG jako 1–3 wiersze z łamaniem `fill`, teksty z belek TVP i TV Republika; font `Bahnschrift.ttf` z `examples/BTB_CG/assets`). Wczytanie: Trio → ⇧ Import show (zastępuje bieżący show).

## 15. Testy i narzędzia

```bash
cd source
cargo clippy --all-targets -- -D warnings
cargo test                                           # silnik, API, JSON, walidacja, kopie
node tests/vectors/run.js && node tests/vectors/btb.js   # wspólne wektory z BTB, konwersja
node tests/e2e/run.js target/release/vizrs           # E2E Trio/Output (Playwright)
node tests/e2e/artist.js target/release/vizrs        # E2E Artist
node tests/e2e/pack.js target/release/vizrs          # E2E paczki .cgpack (eksport, import, font na /output)
node tests/motion.js                                 # ruch: krzywe, presety, efekty tekstu, pętle, sekwencja
node tests/e2e/motion.js target/release/vizrs        # E2E ruchu (panel Artist, /output)
node tests/e2e/golden.js target/release/vizrs [--update]   # zrzuty wzorcowe
node tests/e2e/btb-visual.js target/release/vizrs <katalog> [--frames]  # porównanie z BTB CG (--frames: klatki wejścia)
```

| Narzędzie | Do czego |
|---|---|
| `tools/sync-btb.sh [--check]` | odświeżenie `text-model.js` i wektorów z BTB-CG, wykrycie rozjazdu |
| `tools/build-btb-pack.mjs` | odbudowa paczki `examples/BTB_CG` ze wzorców |
| `tools/gallery.js [vizrs] [--out katalog]` | galeria wszystkich stron zestawów przykładowych (zrzut każdej strony po wejściu, arkusze, `index.html`); wynik poza repo, domyślnie `<tmp>/vizrs-galeria` |
| `tools/out_choreo.py plik.show.json` | wygenerowanie animacji OUT całego show |
| `tools/render_video.py` + `steps_*.py` | filmy demonstracyjne |
| `tools/gen_fonts.py` | wbudowane fonty (`fonts_data.rs`) |

CI: `.github/workflows/ci.yml` (Linux, macOS, Windows).

## 16. Znane ograniczenia

Aktualna lista: [LUKI.md](LUKI.md). Najważniejsze:

- OUT zdejmuje warstwę w serwerze od razu; animację wyjścia gra wyjście – TAKE w trakcie OUT może ją ściąć.
- Dock tylko przesuwa (dx, dy), bez reguł względem rozmiaru celu.
- Rundown bez automatu czasowego; brak źródła tickera z RSS/URL (tylko komendy `TICKER`).
- Brak TLS; blokady edycji są miękkie.
- Kształty wektorowe, tryby mieszania i ekspresje – poza zakresem.
