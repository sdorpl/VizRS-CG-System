# Paczki grafik

Każda oprawa to jeden plik `.cgpack`: sceny, strony, grafiki i czcionki. Wczytanie: w Trio utwórz nowy show (▾),
potem Artist → **⇧ Import** → plik paczki (sceny i strony dochodzą do bieżącego show, pliki trafiają do assets).
Paczka nie przenosi playlist.

| Paczka | Sceny | Strony | Pliki w paczce |
|---|---|---|---|
| `INFO24.cgpack` | 16 – belka, podpis, autor, program, tytuł, pilne z logo, NA ŻYWO (1 i 2 wiersze), odliczanie, wynik, QR, hashtag, kafel programu, tickery (przewijany, góra-dół, wipe) | 1001–5001 | `tlo_hashtag.jpg` |
| `R7_NEWS.cgpack` | 13 – ten sam zestaw w stylistyce R7, bez tytułu, pilne z logo i wyniku | 7001–7505 | `tlo_hashtag2.jpg` |
| `BTB.cgpack` | 31 – wzorce BTB CG i belka PILNE 1–3-wierszowa | 8001–8402, 8501–8506 | `Bahnschrift.ttf`, logo `.webp` |
| `BTB_ALPHA.cgpack` | 58 – grafiki z gałęzi `alpha` BTB CG System (szablony kodowe): belki Republika, EXP z grafiką, Lw, Modern, OCG (biało-szara, czerwona, gość, imię i nazwisko, WS, norm), plakietki NA ŻYWO / Miasto / tagi OCG, 17 tickerów, zegary, logo, widżet zegar + BTB INFO | 14001–14034 belki, 14101–14117 plakietki, 14201–14217 tickery, 14301–14302 zegary, 14401–14404 logo, 14501 widżet | `Bahnschrift.ttf`, logo i grafiki boczne |
| `BTB_TRIO.cgpack` | 18 – grafiki z gałęzi TRIO BTB CG System z **powiązaniami (dock)**: belki (główna, PILNE, wizytówki), plakietki NA ŻYWO / MIASTO / WYDANIE SPECJALNE, 4 tickery, zegar, ozdobnik; każda na własnej warstwie `z<N>_…` | 15001–15004 belki, 15101–15109 plakietki, 15201–15204 tickery, 15301 zegar, 15401 ozdobnik | `Bahnschrift.ttf`, logo |
| `BTB_INFO24.cgpack` | 16 – zestaw INFO24 w stylistyce BTB (czerwień, żółty akcent, granat, Bahnschrift), generowany z `INFO24.show.json` | 9001–13001 (INFO24 + 8000) | `Bahnschrift.ttf`, `tlo_hashtag.jpg` |

INFO24 to także demo wbudowane w serwer (`source/web/demo_show.json`). Fonty INFO24 i R7 (Barlow, Oswald, …) są wbudowane w serwer.

**BTB.cgpack, BTB_INFO24.cgpack, BTB_ALPHA.cgpack i BTB_TRIO.cgpack zawierają font Microsoftu Bahnschrift – nie dystrybuuj tych paczek.** Opis wzorców i znane różnice względem BTB CG:
[archiwum/BTB_CG/README.md](archiwum/BTB_CG/README.md). Względem 31 wzorców: dołączona belka wielowierszowa,
usunięty „zegarek (kopia)” (8302), tickery 8202/8203 przemianowane na „Ticker tryby (Whip / Poziomy / Pionowy)” i „Ticker tryby – kategorie”.

## Budowa

```bash
cd source && node tools/build-packs.mjs
```

Źródła są w `archiwum/`: dawne `INFO24.show.json`, `R7_NEWS.show.json`, `BTB_PILNE.show.json`, katalog `BTB_CG/`
(wynik `tools/build-btb-pack.mjs`), `BTB_ALPHA/` (wynik `tools/build-btb-alpha.mjs`, opis w [archiwum/BTB_ALPHA/README.md](archiwum/BTB_ALPHA/README.md)), `BTB_TRIO/` (wynik `tools/build-btb-trio.mjs`, [opis](archiwum/BTB_TRIO/README.md)) i `assets/` (tła hashtagów). Testy i narzędzia (`validate.rs`, `gallery.js`, `engines.js`, `golden.js`) czytają te pliki.
