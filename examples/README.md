# Paczki grafik

Każda oprawa to jeden plik `.cgpack`: sceny, strony, grafiki i czcionki. Wczytanie: w Trio utwórz nowy show (▾),
potem Trio → **⇧ Import paczki** (albo Artist → **⇧ Import**) → plik paczki (sceny i strony dochodzą do bieżącego show, pliki trafiają do assets).
Paczka nie przenosi playlist.

| Paczka | Sceny | Strony | Pliki w paczce |
|---|---|---|---|
| `INFO24.cgpack` | 16 – belka, podpis, autor, program, tytuł, pilne z logo, NA ŻYWO (1 i 2 wiersze), odliczanie, wynik, QR, hashtag, kafel programu, tickery (przewijany, góra-dół, wipe) | 1001–5001 | `tlo_hashtag.jpg` |
| `R7_NEWS.cgpack` | 13 – ten sam zestaw w stylistyce R7, bez tytułu, pilne z logo i wyniku | 7001–7505 | `tlo_hashtag2.jpg` |

INFO24 to także demo wbudowane w serwer (`source/web/demo_show.json`): 1001–1003 belki (info / pilne / z wyróżnieniem), 1101–1102 podpisy,
2001 bug NA ŻYWO, 2101 odliczanie, 3001–3004 tickery, 4101 „pilne” z logo i zegarem, 4001 tytuł, 5001 wynik. Marka INFO24 jest fikcyjna.
Istniejący `vizrs-data/show.json` nie jest nadpisywany – demo zobaczysz z pustym katalogiem (`--data nowy-katalog`).

### Elementy dodatkowe INFO24 i R7

| Element | INFO24 | R7 | Opis |
|---|---|---|---|
| Kafel z kodem QR (lewy/prawy) | 1401, 1402 | 7501, 7502 | węzeł **Kod QR** – w Trio wpisujesz adres, kod generuje się sam (bez internetu) |
| Belka-hashtag z tłem | 1403 | 7503 | obraz lub wideo w tle paska, plakietka dopasowana do tekstu |
| NA ŻYWO z 2-wierszowym miejscem | 1404 | 7504 | Enter w polu „Miejsce” = drugi wiersz, każdy ściskany osobno |
| Rotujący kafel programu | 1405 | 7505 | programy i godziny w kolejnych liniach, „rotacja wierszy (s)” |
| Tickery `crawl` / `roll` / `wipe` | 3001 i 3004 / 3002 / 3003 | 7301 i 7304 / 7302 / 7303 | tryby opisane w [DOKUMENTACJA §7](../docs/DOKUMENTACJA.md#7-ticker) |

Podgląd wszystkich stron: `node source/tools/gallery.js`; zapisana galeria: [../media/galeria/index.html](../media/galeria/index.html). Fonty INFO24 i R7 (Barlow, Oswald, …) są wbudowane w serwer.


## Budowa

```bash
cd source && node tools/build-packs.mjs
```

Źródła są w `archiwum/`: `INFO24.show.json`, `R7_NEWS.show.json` i `assets/` (tła hashtagów). Testy i narzędzia (`validate.rs`, `gallery.js`, `engines.js`, `golden.js`) czytają te pliki.
