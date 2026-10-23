# DOCK → VizRS

Grafiki z gałęzi `TRIO` oryginalnego repozytorium (commit `511ec91`, `db.json`: 19 grafik, szablony HTML/CSS/JS) przeniesione razem z **powiązaniami (dokowaniem)**.
Metoda jak w [ALPHA](../ALPHA/README.md): pomiar DOM wyjścia oryginału, z którego powstają sceny.

| Plik | Co |
|---|---|
| `zrodlo.json` | 19 grafik z `db.json` gałęzi TRIO i pomiar DOM każdej z nich |
| `show.json` | 18 scen, 19 stron 15001–15401 (wynik `tools/build-dock.mjs`) |
| `konwersja.json` | mapa strona → grafika oryginału, powiązania, uwagi |
| `assets/` | logo z `uploads/` oryginału (font Bahnschrift paczka bierze z `../WZORCE/assets`) |

Odtworzenie: w katalogu oryginału na gałęzi TRIO `PORT=3998 DATABASE_URL=kopia.sqlite node server.js` (pusta baza wczytuje `db.json`), potem w `source/`:
`CG_ZRODLO="…" CG_SRC_DIR=… node tools/measure-alpha.cjs http://localhost:3998 ../examples/archiwum/DOCK/zrodlo.json` → `CG_SRC_DIR=… node tools/build-dock.mjs` → `node tools/build-packs.mjs`.

## Zasady przeniesienia

- Każda grafika oryginału = strona na **własnej warstwie** `z<N>_<nazwa>` (N = warstwa oryginału: belki i plakietki 1, miasta 2, tickery 5, ozdobnik 9), więc wszystkie mogą być na wizji jednocześnie, a kolejność w osi Z jest jak w oryginale.
- Dokowanie `layout.attachedToGraphicId` / `attachedToGraphicIdX` (tryb „stały odstęp”) → `scene.dock`: cele to strony rodziców (`page:A|page:B` – przesunięcie raz, gdy na wizji jest którykolwiek), przesunięcie `attachOffsetY/X`, czas 0,6 s (jak przejście w rendererze oryginału).
  - MIASTO lewa ↓55 przy NA ŻYWO lewa; MIASTO prawa ↓55 przy NA ŻYWO prawo / „tylko w tv info”; „tylko w tv info” ←175 przy NA ŻYWO prawo;
    WYDANIE SPECJALNE ↑160 przy TICKER PEŁNY EKRAN / Belka – PILNE / Wizytówka – PILNE; „Z ostatniej chwili” ↑130 i Ozdobnik ↑155 przy Belce głównej.
- Animacje elementów `data-cg-anim` plakietek miast (tło wipe, tekst clip) → ruch węzłów; animacja grafiki → ruch grupy.

## Znane różnice

- Powiązania z grafikami, których nie ma w bazie TRIO (usunięte identyfikatory), pominięto – lista w `konwersja.json`.
- „Belka glowna” w oryginale po wejściu zostaje poza ekranem (translateY 1080 po flip-x) – w VizRS stoi tam, gdzie pozostałe belki (y 825).
- „Ticker – zepsuty” przeniesiony tak, jak go oryginał rysuje (tryb pionowy, pierwsza wiadomość).
- Pulsująca kropka NA ŻYWO: pętla „oddychanie” zamiast pulsu skali.
