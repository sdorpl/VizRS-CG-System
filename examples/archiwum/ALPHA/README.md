# ALPHA → VizRS

Grafiki z gałęzi `alpha` oryginalnego repozytorium (commit `4ae2366`).
W tej wersji oryginału grafiki to szablony kodowe (HTML + CSS + JS z podstawieniami `{{…}}`), więc nie da się ich wczytać importem elementu.
Sceny powstały z **pomiaru wyjścia oryginału**: każda grafika wchodzi osobno na `output.html` działającego serwera oryginału, po wejściu zapisywane są
prostokąty, gradienty, ramki, teksty (font, rozmiar, grubość, kolor, odstępy liter) i obrazy.

| Plik | Co |
|---|---|
| `zrodlo.json` | 58 grafik z `database.sqlite` + 23 szablony bez grafik (z ustawieniami domyślnymi szablonu) i pomiar DOM każdej z nich |
| `show.json` | 58 scen, 75 stron (wynik `tools/build-alpha.mjs`) |
| `konwersja.json` | pominięte grafiki i mapa strona → grafika oryginału |
| `assets/` | logo i grafiki boczne wyjęte z `data:` URI (font Bahnschrift paczka bierze z `../WZORCE/assets`) |

Odtworzenie: serwer oryginału (alpha) na kopii bazy (`PORT=3999 DATABASE_URL=kopia.sqlite node server.js`), potem w `source/`:
`node tools/measure-alpha.cjs` → `node tools/build-alpha.mjs` → `node tools/build-packs.mjs`.

## Zasady przeniesienia

- Każda grafika oryginału = strona; grafiki o identycznym wyglądzie (inny tylko tekst) dzielą scenę.
- Animacja wejścia/wyjścia grafiki (`animation.in/out`, efekt tekstu) → ruch grupy (`node.motion`) jak przy imporcie elementu.
- Gradient CSS `c1 0% → c2 50% → c1 100%` (belki OCG) → dwie połówki z gradientem dwukolorowym.
- Tickery: tło paska + węzeł `ticker` (tryb z `tickerMode`: whip / przewijany / pionowy → `whip` / `crawl` / `roll`), wiper → etykieta (`label`, `catW`, `catBg`), połysk → `gleam`, separator → `sepStyle`/`sepColor`.
- Belki rysowane na `<canvas>` (OCG: imię i nazwisko, belka WS, belka norm) i widżet zegar + TV INFO – złożone według kodu szablonu (wymiary, kolory, kolejność i czasy wejścia).
- Strony: 14001– belki, 14101– plakietki i tagi, 14201– tickery, 14301– zegary, 14401– logo, 14501 widżet.

## Pominięte i znane różnice

- Pominięte: „Lw (21:17:23)” (szablon `republika-composite`) i 5 grafik „Modern Ticker” / „Republika Ticker (21:02:59)” (szablon `modern-ticker`) – tych szablonów nie ma w bazie alpha, oryginał nic dla nich nie rysuje.
- „TAG: WYDANIE SPECJALNE” i „Republika Ticker (Cycling)” w alpha się nie wyświetlają (brak klasy `.active` / `display: none` w CSS) – odtworzone według reguł CSS szablonu.
- Tagi OCG (Białystok, NA ŻYWO, Powtórka) bez grafik w bazie mają szerokość domyślną szablonu (190 px); animowany gradient tła (`liveGrad`) jest statyczny, z połyskiem.
- Ticker przewijany z wiperem: w oryginale wiper pokazuje kategorię bieżącej wiadomości, w VizRS stała etykieta (kategorie z wiadomości zdjęte).
- Pulsująca kropka NA ŻYWO: pętla „oddychanie” zamiast pulsu skali 1 → 0,9.
