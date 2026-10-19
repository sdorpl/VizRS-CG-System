# Bitfocus Companion / Stream Deck z VizRS

VizRS nie wymaga wtyczki: używaj modułu **Generic: HTTP Requests** (albo OSC). Przy włączonym `--token` dodaj nagłówek `X-Token: <token>`
(w module HTTP: „Headers”) lub dopisz `?token=<token>` do adresu.

| Przycisk | Typ | URL (baza `http://host:8088`) |
|---|---|---|
| TAKE strony | GET | `/api/take/1001` |
| TAKE z danymi | GET | `/api/take/5001?homeScore=3&awayScore=1` |
| CONTINUE | GET | `/api/continue/main` |
| OUT warstwy | GET | `/api/out/main` |
| CLEAR ALL | GET | `/api/clear` |
| NEXT (rundown) | GET | `/api/rundown/<id-lub-nazwa>/next` |
| Makro (preset segmentu) | GET | `/api/macro/Wiadomosci` |
| ARM w Preview | GET | `/api/arm/1001` |
| Dock off/on | GET | `/api/dock/main/off` · `/api/dock/main/on` |
| Wiele stron naraz | GET | `/api/take/1001,3001` · `/api/out/main,back` |

## Zmienne i podświetlenie przycisków
`GET /api/variables` zwraca płaski JSON (`layer_main` = numer strony na warstwie, `layer_main_name`, `layers_on_air`, `rundown_<id>_next`…).
W Companion użyj modułu odpytującego JSON (np. „Generic: HTTP” z odczytem JSON) co 1 s, albo webhooka (`--webhook`), który wysyła `onAir`/`offAir`/`error`.

## OSC zamiast HTTP
Uruchom `vizrs --osc 8000`, w Companion dodaj moduł **Generic: OSC** i wysyłaj `/vizrs/take` (arg. int 1001), `/vizrs/next` (arg. tekst – nazwa rundownu), `/vizrs/macro` itd.
Przy `--token`: `/vizrs/<token>/take`.

## Port TCP
Moduł **Generic: TCP/UDP** → host:6100, komenda np. `TAKE 1001\n` (przy tokenie najpierw `AUTH <token>\n`).
