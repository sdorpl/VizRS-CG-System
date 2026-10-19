# Paczka BTB CG → VizRS

31 elementów z katalogu wzorców BTB CG (`btb-cgeditor/CG elements converted/`), każdy plik skonwertowany osobno
(`source/web/btb-io.js`) i porównany wizualnie z oryginałem renderowanym przez wyjście BTB CG.

| Plik | Zawartość |
|---|---|
| `show.json` | 31 scen + 31 stron |
| `assets/` | `Bahnschrift.ttf` (font z BTB CG), logo `f0b33a…webp` (z `BTB-CG-System-alpha/uploads`) |
| `konwersja.json` | plik źródłowy → scena, strona, warstwa, ostrzeżenia konwertera |

## Wczytanie

1. Skopiuj `assets/*` do `vizrs-data/assets/` (albo wgraj w Artist → Zasoby).
2. Trio → **⇧ Import show** → `show.json` (**zastępuje** bieżący show); albo uruchom `vizrs --data <kopia tego katalogu>`.

## Numeracja i warstwy

| Strony | Rodzaj BTB | Warstwa VizRS |
|---|---|---|
| 8001–8014 | LOWER_THIRD (belki, wizytówki) | `main` |
| 8101–8107 | BADGE (plakietki) | `badge` |
| 8201–8206 | TICKER | `ticker` |
| 8301–8302 | CLOCK | `clock` |
| 8401–8402 | IMAGE (logo) | `logo` |

Osobne warstwy pozwalają mieć na antenie jednocześnie belkę, plakietkę, ticker, zegar i logo.
Warianty o tej samej nazwie (4 × „Belka - PILNE”) mają w nazwie dopisek z nazwą pliku.

## Test wizualny

```bash
cd source
node tools/build-btb-pack.mjs                                   # odbudowa paczki ze wzorców
node tests/e2e/btb-visual.js target/release/vizrs /tmp/zrzuty   # porównanie z BTB CG + zestawienia PNG
```

Wynik: 31/31 zgodnych (stan po wejściu, to samo tło i godzina). Większość belek i plakietek różni się o 0–3% pikseli,
reszta to przesunięcia krawędzi tekstu o 1–2 px (zaokrąglenia silników).

## Znane różnice

- **Belka - PILNE (8001–8004):** pulsująca poświata `glow` belki nie jest przenoszona (VizRS ma tylko czarny cień).
- **Tickery whip (8201, 8203, 8204, 8206):** wiper w VizRS ma proste rogi i nie rzuca cienia.
- **Crawl (8202, 8205):** odstęp wokół separatora przybliżony spacjami (różnica kilkunastu px).
- **MIASTO prawa — Warszawa (8102):** BTB ściska tekst plakietki wg długości (25/len, min. 50%), VizRS do szerokości tła z tą samą granicą 50% – ten sam efekt dla tekstu domyślnego.
- **Belka glowna, Belka pilne, Wizytówki (8006, 8008, 8012, 8013):** w BTB teksty bez pola (edycja w WYSIWYG); w VizRS pola `tekst1`, `tekst2` („Tekst 1”, „Tekst 2”), a pole tekstu sięga do krawędzi tła – dłuższy tekst nie jest ściskany do szerokości przykładu.
- **Plakietki z auto-szerokością (np. Plakietka NA ŻYWO):** w BTB napis dłuższy niż `maxW` wychodził poza plakietkę; w VizRS jest ściskany w `maxW` (do 50%).
- **Grafika - theme (8401):** obraz z adresu Wikimedii zwraca HTTP 400 – pusty także w BTB; podmień obraz w Artist.
- **Bahnschrift** to font Microsoftu (skopiowany z BTB CG) – nie publikuj go w publicznym repozytorium.
