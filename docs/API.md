# VizRS – API (HTTP, TCP, OSC, webhook)

Wszystkie ścieżki HTTP zaczynają się od `/api`. Metody `GET` działają tam, gdzie wpisano „GET” (Companion / Stream Deck).
Przy `--token`: nagłówek `Authorization: Bearer <token>`, `X-Token: <token>` albo `?token=<token>`. Żądania z obcych witryn są odrzucane (403).
Kody: `400` zły argument/walidacja · `401` brak tokenu · `403` obca witryna · `409` konflikt rewizji (`rev`) lub blokady · `500` błąd zapisu na dysk · `503` za dużo połączeń.

## Show, strony, sceny, playlisty, rundowny, makra
| Metoda i ścieżka | Opis |
|---|---|
| `GET /show`, `PUT /show[?force=1]` | cały show; `PUT` to import (plik z innym `rev` → 409, `force=1` nadpisuje) |
| `POST /<rodzaj>`, `PUT /<rodzaj>/<id>`, `DELETE /<rodzaj>/<id>` | rodzaje: `scenes`, `pages`, `playlists`, `rundowns`, `macros`; `rev` w ciele = wykrywanie konfliktu (409) |
| `GET /state` | stan anteny (warstwy, `pending`, `hasDock`/`dockOff`), preview, rundowny (kursor, odegrane), `errors`, `saveError`, `locks`, `rev` |
| `GET /variables` | płaski słownik zmiennych dla Companion (`layer_main`, `layer_main_name`, `layers_on_air`, `rundown_<id>_next`…) |
| `GET /health`, `GET /api/health` | zdrowie systemu: status, wersja, uptime, wyjścia, błędy scen, ostatnie zdarzenie, kopie |

## Playout
| Ścieżka | Opis |
|---|---|
| `GET\|POST /take/<strony>[?pole=wartość…]` | TAKE; lista po przecinku `1001,3001`; wartości z query lub `{"values":{…}}` |
| `GET\|POST /arm/<strony>` | uzbrojenie na wyjściu Preview (`/output?bus=preview`) |
| `POST /preview/clear` | czyści Preview |
| `GET\|POST /continue/<warstwy\|strony>` | CONTINUE (lista po przecinku) |
| `GET\|POST /out/<warstwy\|strony>` | OUT (lista po przecinku) |
| `GET\|POST /clear` | zdejmuje wszystko |
| `GET\|POST\|PATCH /update/<strona>[?save=0]` | podmiana danych na antenie bez ponownej animacji |
| `GET\|POST /deploy/<warstwa>` | wdraża nową wersję szablonu do warstwy na antenie (bez animacji) |
| `GET\|POST /dock/<warstwa>/off\|on` | wyłącza/włącza powiązanie (dock) sceny – nadpisanie operatora |
| `GET\|POST /carousel/start/<playlista>`, `/carousel/stop[?out=0]` | karuzela playlisty |
| `GET\|POST /rundown/<id>/next`, `/take/<pozycja>`, `/cursor/<pozycja>`, `/reset` | rundown: NEXT, wzięcie pozycji, ustawienie kursora, zerowanie |
| `GET\|POST /macro/<nazwa\|id>`, `/macro/stop` | makro (kroki z opóźnieniami); nowe makro przerywa poprzednie |
| `GET /ticker/<strona[:pole]>` | wiadomości tickera strony: `{"field":"items","items":[{"cat","color","text"}]}` |
| `GET\|POST /ticker/<strona>/add?item=KAT\|treść`, `/set/<n>?item=…`, `/del/<n>`, `/clear` | zmiana listy (n od 1; `item` także w treści `{"item":"…"}`); zapisuje stronę i – gdy jest na antenie – aktualizuje bez restartu przewijania; odpowiedź jak UPDATE + `items` |

## Zasoby i narzędzia
| Ścieżka | Opis |
|---|---|
| `GET /assets`, `POST /assets?name=plik.png`, `DELETE /assets/<nazwa>` | obrazy/wideo/fonty (dozwolone: png jpg jpeg gif webp svg mp4 webm ttf otf woff woff2; limit 2 GB łącznie) |
| `GET /fonts` | rodziny fontów dostępnych offline (wbudowane) |
| `GET\|PUT /animpresets` | biblioteka presetów animacji (Artist) |
| `GET\|PUT /wordreplace` | lista zamian słów stanowiska (`{"@p":["Prezydent"]}`) |
| `GET /backups`, `POST /backups/restore/<nazwa>` | kopie zapasowe show |
| `GET /locks`, `POST /lock/<rodzaj>/<id>?who=…`, `POST /unlock/<rodzaj>/<id>?who=…[&force=1]` | blokady edycji (45 s bez odświeżenia) |
| `POST /report`, `GET\|DELETE /errors` | błędy scen zgłaszane przez wyjście |
| `GET /audit[?n=100]` | ostatnie komendy z logu audytowego |

## TCP (port 6100; przy `--token` pierwsza linia `AUTH <token>`)
`TAKE <strona[,strona…]> [k=v|k2=v2]` · `ARM <strona>` · `CONTINUE <warstwa|strona>` · `OUT <warstwa|strona>` · `CLEAR` · `UPDATE <strona> k=v|…` · `LIVE <strona> k=v|…` (bez zapisu)
· `TICKER <strona[:pole]> LIST|ADD <KAT|treść>|SET <n> <KAT|treść>|DEL <n>|CLEAR`
· `NEXT <rundown>` · `RUNDOWN TAKE <rundown> [pozycja]` · `RUNDOWN RESET <rundown>` · `MACRO <nazwa>` · `MACRO STOP` · `CAROUSEL START <playlista>` · `CAROUSEL STOP` · `STATE` · `PING`.
Odpowiedź: `OK {json}` albo `ERR komunikat`.

## OSC (UDP; `--osc 8000`)
`/vizrs/take 1001` · `/vizrs/arm 1001` · `/vizrs/out main` · `/vizrs/continue main` · `/vizrs/clear` · `/vizrs/next <rundown>` · `/vizrs/macro <nazwa>` · `/vizrs/update 1001 pole wartość [pole wartość…]` · `/vizrs/ticker 3001 ADD SPORT|Gol` (argumenty jak w TCP `TICKER`).
Przy `--token` adres musi mieć postać `/vizrs/<token>/take …`.

## Webhook (`--webhook http://host:port/ścieżka`)
`POST` JSON: `{"event":"onAir","layer":"main","page":{"number":1001,"name":"…"},"ts":…}`, `{"event":"offAir","layer":"main"|"*"}`, `{"event":"error","layer":"<scena>","message":"…"}`. Tylko `http://` (bez TLS).

## Strumień zdarzeń (SSE) `GET /events`
Zdarzenia: `snapshot` (stan startowy; osobny z `bus:"preview"`), `take`, `continue`, `out`, `clear`, `update`, `rescene`, `dock`, `show`, `state`, `error`, `wordreplace`.

## Wyjście `/output`
`?layers=main,front` · `?bg=%2300ff00` · `?w=1280&h=720` · `?checker=1` · `?status=1` · `?bus=preview` · `?mode=fill` (kolor na czerni) · `?mode=key` (alfa jako luma). Fill + Key = dwa źródła przeglądarki dla ATEM/Tricastera.
