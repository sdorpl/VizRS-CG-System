# VizRS Live – klient ErsatzTV Next z interfejsem na wzór ErsatzTV legacy

> Stan (2026-11-06): **plan dalekosiężny, tylko zapis pomysłu.** Brak decyzji, etapów i terminu. Przed etapami: rozpoznanie ErsatzTV Next (pkt „Do rozpoznania”).

## Pomysł

**VizRS Live** to przyszła aplikacja rodziny VizRS: graficzny klient (GUI) dla **ErsatzTV Next**, z funkcjami, które dziś daje interfejs WWW **ErsatzTV legacy** ([github.com/ErsatzTV/legacy](https://github.com/ErsatzTV/legacy.git)). Operator kanału prowadzi w jednym miejscu kanały, ramówkę i grafikę VizRS, a pod spodem pracuje ErsatzTV Next.

Miejsce w rodzinie VizRS:

| Repozytorium / aplikacja | Rola |
|---|---|
| VizRS CG System | silnik grafiki: Artist, Trio, `/output`, API TCP / HTTP / SSE |
| VizRS CG Playout | sidecar dla ErsatzTV: komendy na start klipu, reklamy bez grafiki, wtapianie, live ingest |
| **VizRS Live** (przyszłość) | GUI kanału nad ErsatzTV Next: to, co w legacy robi interfejs WWW, plus spięcie z grafiką i live ingest VizRS |

## Zakres funkcji (wzór: interfejs ErsatzTV legacy)

- **Kanały**: lista, numer, nazwa, logo, profil kodowania (ffmpeg), strumień i adres M3U/HDHomeRun.
- **Media**: biblioteki lokalne i Remote Streams, skan, kolekcje (zwykłe, inteligentne, wielokolekcje).
- **Harmonogramy i playout**: pozycje harmonogramu (Fixed Start, Playout Mode One/Multiple/Duration/Flood), wypełniacze, przerwy reklamowe, reset i podgląd playoutu.
- **EPG**: podgląd ramówki kanału, XMLTV.
- **Ustawienia**: ffmpeg, akceleracja (VAAPI, NVIDIA, bez GPU – Ampere A1), logi.
- **Ponad legacy (spięcie z VizRS)**: elementy VizRS na pozycjach ramówki (komendy jak w VizRS CG Playout), audycje na żywo jako pozycje ramówki (slot live ingest), podgląd kanału z grafiką, czas strony na kanale (⏱ kanał w Trio, [PLAN-za-chwile.md](PLAN-za-chwile.md)).

## Założenia wstępne

- ErsatzTV Next zostaje **niezmieniony** (jak ErsatzTV w VizRS CG Playout): VizRS Live jest jego klientem, nie forkiem.
- Osobne repozytorium (jak VizRS CG Playout), nie część rdzenia `vizrs` – rdzeń zostaje bez zależności.
- UI w duchu VizRS: polski interfejs, praca offline, opcjonalnie okno natywne jak `vizrs-shell`.
- Dane z ErsatzTV Next traktujemy jak dane z zewnątrz (walidacja, limity).

## Do rozpoznania przed etapami

1. Jak ErsatzTV Next daje się sterować z zewnątrz (API, pliki konfiguracyjne, ich stabilność między wersjami) i czego jeszcze nie umie względem legacy.
2. Które funkcje legacy są potrzebne na kanale (minimum) – lista do wyboru przez użytkownika.
3. Relacja z VizRS CG Playout: czy sidecar zostaje osobno, czy VizRS Live go wchłania lub nim zarządza.
4. Przejście kanałów z ErsatzTV legacy na Next (import konfiguracji, ramówek, bibliotek).
5. Forma aplikacji: serwer WWW + przeglądarka, okno natywne, czy oba.
