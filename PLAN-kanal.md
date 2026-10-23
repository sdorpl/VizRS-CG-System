# PLAN: VizRS Playout – emisja kanału z brandingiem sterowanym harmonogramem (na wzór Viz Multichannel)

> Status (2026-10-23): **decyzje podjęte (§11)** – osobne repozytorium „VizRS Playout”, własny playout wideo na bazie ffplayout, XMLTV później. Tam, gdzie §0–§10 mówią o `vizrs-channel`, Tunarr/ErsatzTV i binarce w tym samym crate, obowiązuje §11–§12. Wzór: [Viz Multichannel 4.2 – User Interface](https://docs.vizrt.com/viz-multichannel-guide/4.2/User_Interface.html), [Playlists](https://docs.vizrt.com/viz-multichannel-guide/4.2/Playlists.html), [Rules](https://docs.vizrt.com/viz-multichannel-guide/4.2/Rules.html), [Actions](https://docs.vizrt.com/viz-multichannel-guide/4.2/Actions.html), [Templates, Pages and Pilot Data Elements](https://docs.vizrt.com/viz-multichannel-guide/4.2/Templates,_Pages_and_Pilot_Data_Elements.html).
> Powiązane: [docs/LUKI.md](docs/LUKI.md) #12 (rundown bez automatu), [docs/API.md](docs/API.md), [PLAN-natywne-i-serwer.md](PLAN-natywne-i-serwer.md).

## 0. Streszczenie

W telewizji (np. TVP z Vizrt) grafika idzie dwoma torami:

| Tor | Miejsce | Elementy | Sterowanie | W VizRS |
|---|---|---|---|---|
| Studio | reżyseria studia | belki, plansze, ozdobniki | operator | **Trio** (jest) |
| Kanał (branding) | reżyseria emisji (MCR) | logo, zegar, ticker, oznaczenia wiekowe, „NA ŻYWO”, „Za chwilę/Następnie”, zapowiedzi | harmonogram emisji | **brak** → ten plan |

Proponujemy **osobną aplikację `vizrs-channel`** w roli Viz Multichannel razem z Media Sequencerem. Aplikacja:
- wczytuje harmonogram kanału (XMLTV z Tunarr/ErsatzTV, plik z działu planowania, wpisy ręczne);
- według **reguł** dokłada do programów **zdarzenia wtórne** (strony i akcje VizRS);
- w odpowiednim czasie wysyła komendy do zwykłego serwera `vizrs` (rola „silnik”, odpowiednik Viz Engine) przez istniejący port TCP 6100.

Wyjście `/output` tego serwera (Fill/Key albo przezroczystość) nakłada się na sygnał kanału w torze emisji. VizRS nie wymaga zmian w rdzeniu poza drobnymi dodatkami (§9).

## 1. Odwzorowanie pojęć Viz Multichannel → VizRS Channel

| Viz Multichannel | VizRS Channel | Uwagi |
|---|---|---|
| Channel (lista kanałów w lewym panelu) | **Kanał**: połączenie z jednym serwerem `vizrs` (URL HTTP, port TCP, token), źródło harmonogramu, strefa czasowa, przesunięcie | wiele kanałów w jednej aplikacji |
| Playlist: *broadcast day* / *continuous* | **Playlista dnia emisji** (doba od godziny granicznej, np. 06:00) / **ciągła** (przewijane okno, np. 48 h z XMLTV) | |
| Primary event (grupa lub klip) | **Zdarzenie główne** = program z harmonogramu: `eventId`, start, długość, tytuł, opis, kategoria, kategoria wiekowa, flaga „na żywo”, `mediaId`, atrybuty dowolne | VizRS nie odtwarza wideo, robi to Tunarr/ErsatzTV. Zdarzenie główne niesie tylko czas i dane. |
| Secondary event (strona/akcja z offsetem) | **Zdarzenie wtórne**: strona VizRS (TAKE/CONTINUE/OUT) albo akcja, z kotwicą i offsetem | |
| Begin / Duration / End / Offset, LOCK Start Time | to samo; `lock` blokuje nadpisanie czasu przy aktualizacji | |
| Rules (Conditions, Take Page/Action, Relative To Start/End/Program Duration %, Offset ±, Add Continue, Duration / Duration relative to attribute) | **Reguły**: 1:1 (§4) | sedno modułu |
| Actions (Viz Engine, Viz Ticker, GPO) | **Akcje**: komendy VizRS (`OUT warstwa`, `CLEAR`, `CONTINUE`, `MACRO`, `TICKER …`, `CAROUSEL …`) oraz wyjście „GPO” jako webhook HTTP | |
| Templates / Pages | **Sceny i strony** z show serwera VizRS (odczyt `/api/show`), szablony tworzy Artist | Channel nie ma własnego edytora scen |
| Pilot Data Elements, Database Connection (SQL) | **Wypełnianie pól** z danych zdarzenia: `{{tytul}}`, `{{nastepny.tytul}}`, `{{nastepny.start:HH:mm}}`, `{{attr.X}}`, plus odczyt danych z URL JSON/CSV (bez SQL) | |
| Channel Pool (logo animowane, pilne strony do ręcznego/zewnętrznego wywołania) | **Pula kanału**: gotowe elementy poza harmonogramem, wywoływane przyciskiem albo API `/pool/<id>/take` | |
| Full Update / Full Update Without Event Deletion / Update Times (klucz: unikalny Event ID) | trzy tryby aktualizacji playlisty przy ponownym imporcie, klucz `eventId` | |
| Add Live Event, Run Rules On Element, Enable/Disable, Direct Take/Continue/Out, External Preview, Initialize | to samo w menu kontekstowym; podgląd przez `ARM` → `/output?bus=preview` | |
| Kolumny Available / Loaded, element czerwony/szary, playlista running/stopped, active | **Walidacja**: strona istnieje w show, wymagane pola wypełnione, silnik połączony. Czerwony = błąd, szary = wyłączony. Playlista: uruchomiona/zatrzymana. | |
| Filtry (wszystkie / dowolny / żaden warunek), wyszukiwanie, kolumny przeciągane i ukrywane | to samo, układ kolumn zapamiętywany w przeglądarce | |
| Export XML, raport HTML, Send to MCR | eksport JSON playlisty, raport HTML dnia, **log emisji (as-run)** | |
| Local Engine Status, ikona czasu | stan połączenia z `vizrs` (TCP `PING`/`STATE`, SSE `/events`), zegar z sekundami i klatkami | |

## 2. Architektura

```
 Tunarr / ErsatzTV ──XMLTV──┐        plik planowania (JSON/CSV) ──┐
                            ▼                                     ▼
                    ┌────────────────── vizrs-channel (port 8090) ─────────────────┐
                    │ import → playlista → reguły → zdarzenia wtórne → harmonogram │
                    │ UI /channel (przeglądarka / okno), API, log emisji           │
                    └───────────────┬───────────────────────────────┬──────────────┘
                         TCP 6100 (TAKE/OUT/UPDATE/…)         SSE /events (stan)
                                    ▼                               │
                    ┌──── vizrs serve (rola: silnik brandingu, port 8089) ─────────┐
                    │ show „Branding”: logo, zegar, ticker, wiek, za chwilę         │
                    └───────────────┬──────────────────────────────────────────────┘
                                    ▼ /output (przezroczyste)
 sygnał kanału (Tunarr/ErsatzTV) ─► nakładanie grafiki (MCR) ─► emisja (SRT/RTMP/IPTV)
```

- **Osobny proces i osobny katalog danych** (`--data ./vizrs-channel-data`). Awaria Channel nie zdejmuje grafiki z anteny, a restart silnika VizRS nie gubi harmonogramu.
- **Ta sama skrzynka narzędzi, zero zależności**: druga binarka w tym samym crate (`source/src/bin/vizrs-channel.rs` + moduły `source/src/channel/`). Wspólne są `http.rs`, `json.rs`, `guard.rs`, `datalock.rs`, `shutdown.rs` i `client.rs`. UI `source/web/channel.html` jest wbudowane jak pozostałe (tablica `STATIC` dla binarki channel). Okno: `vizrs channel` / `vizrs-shell --channel` (jak Trio).
- **Jeden kanał = jeden serwer `vizrs`**. Kilka kanałów oznacza kilka serwerów silnika, co pasuje do „kanał = kontener” z PLAN-natywne-i-serwer.
- Channel nie renderuje grafiki. Podgląd strony robi silnik (`ARM` + `/output?bus=preview` w ramce).

## 3. Model danych (`channel.json` w katalogu danych Channel)

```jsonc
{
  "channels": [{
    "id": "ch1", "name": "INFO24",
    "engine": { "http": "http://127.0.0.1:8089", "tcp": "127.0.0.1:6100", "token": "…" },
    "source": { "kind": "xmltv", "url": "http://tunarr:8000/api/xmltv.xml", "channel": "1", "refreshMin": 5 },
    "tz": "Europe/Warsaw", "dayStart": "06:00", "delayMs": 4000, "fps": 25,
    "playlistKind": "continuous",           // | "day"
    "rules": [ /* §4 */ ], "actions": [ /* §5 */ ], "pool": [ /* §5 */ ],
    "attrMap": { "category": "kategoria", "rating": "wiek" }   // mapowanie pól źródła na atrybuty
  }]
}
```
Playlisty: `playlists/<kanał>/<data>.json`:
```jsonc
{ "rev": 7, "events": [{
  "eventId": "xmltv:1:20261023T200000", "begin": "2026-10-23T20:00:00+02:00", "dur": 3600000,
  "title": "Wiadomości", "attrs": { "kategoria": "news", "wiek": "0", "live": true },
  "lock": false, "enabled": true, "origin": "import",          // | "live" | "manual"
  "secondary": [{ "id": "s1", "rule": "r-logo", "kind": "page", "page": 9001, "values": {…},
                   "anchor": "start", "offsetMs": 0, "durMs": null, "continueMs": null,
                   "enabled": true, "manual": false }]
}]}
```
Każdy zapis przechodzi przez walidator `channel/validate.rs` (limity długości, liczby zdarzeń, zakresy czasów, typy), tak jak `validate.rs` w VizRS. Każde nowe pole dostaje regułę i test.

## 4. Reguły (wg Viz Multichannel → Rules)

| Pole | Znaczenie |
|---|---|
| `description` | nazwa (kilka reguł może dotyczyć tej samej strony) |
| `conditions[]` + `match: all\|any\|none` | `{attr, op: eq\|ne\|in\|contains\|re\|gt\|lt\|exists, value}` na atrybutach zdarzenia głównego (kategoria, wiek, live, tytuł, długość) |
| `take` | `{page, values}` albo `{action: id}` |
| `relativeTo` | `start` \| `end` \| `percent` (pozycja w programie, 0–100) |
| `offset` | `hh:mm:ss:ff` + `positive` (ujemny offset może cofnąć zdarzenie do poprzedniego programu, np. odliczanie przed startem; zdarzenie wtórne zostaje przypięte do programu, do którego się odnosi) |
| `addContinue` | dla stron ze stop pointem: CONTINUE po zadanym offsecie |
| `duration` | stała długość albo `durationAttr` (długość z atrybutu zdarzenia); `whole` = do końca programu (logo, oznaczenie wiekowe przez cały program) |
| `layerPolicy` | `exclusive` (domyślnie: TAKE zastępuje warstwę, jak w VizRS) |

Reguły uruchamiają się przy imporcie, przy aktualizacji i na żądanie („Uruchom reguły na elemencie”). Zdarzeń wtórnych dodanych lub zmienionych ręcznie (`manual: true`) automat nie nadpisuje.

**Przykładowe reguły brandingu (paczka startowa):**
1. Logo: każdy program, `start`, do końca. Wyjątek: kategoria `reklama`/`zapowiedz`, wtedy OUT.
2. Oznaczenie wiekowe: `wiek ∈ {7,12,16,18}`, `start`, przez cały program (zgodnie z wymaganiami KRRiT dla oznaczeń; sposób prezentacji ustala nadawca).
3. „NA ŻYWO”: `live = true`, `start + 00:00:05:00`, 10 s.
4. „Za chwilę: {{nastepny.tytul}}”: `end − 00:01:00:00`, 15 s, z `addContinue`.
5. Ticker: kategoria `news`, `start`, do końca. Wiadomości ładowane akcją `TICKER`.
6. Zegar: stale (element puli albo reguła „każdy program, do końca”).

## 5. Akcje i pula
- **Akcje**: `OUT <warstwa>`, `CLEAR`, `CONTINUE <warstwa>`, `MACRO <nazwa>`, `TICKER <strona> SET|ADD|CLEAR …`, `CAROUSEL START|STOP`, `UPDATE <strona> k=v` oraz **GPO** = `POST` na zadany URL (webhook do Companion/miksera). Akcje to zdarzenia wtórne jak strony.
- **Pula kanału**: elementy przygotowane poza harmonogramem (pilna wiadomość, „przerwa techniczna”, logo okolicznościowe). Przycisk w UI oraz API `POST /api/pool/<id>/take|out`. Ręczne wywołanie zapisuje się w logu emisji z flagą `manual`.

## 6. Silnik harmonogramu (sedno niezawodności)

1. **Pętla co 40 ms** (1 klatka przy 25 fps) wylicza listę zdarzeń do wykonania w oknie `[teraz, teraz+40ms)`, przesuniętą o `delayMs`. `delayMs` kompensuje opóźnienie sygnału kanału (HLS, odtwarzacz) względem EPG.
2. **Wyrównywanie stanu, nie tylko wyzwalanie.** Dla zdarzeń o czasie trwania (logo, wiek, ticker) silnik zna *stan pożądany* każdej warstwy w danej chwili i porównuje go ze *stanem rzeczywistym* z SSE `/events`. Po restarcie Channel, restarcie silnika albo przeładowaniu strony `/output` stan wraca bez czekania na następny program. Zdarzenia jednorazowe (np. „NA ŻYWO”) są pomijane, jeśli od ich startu minęło więcej niż `catchUpMs` (domyślnie 2 s).
3. **Ręczne nadpisanie**: operator MCR może zatrzymać kanał (STOP), wstrzymać jedną warstwę („ukryj logo do odwołania”) albo wziąć element ręcznie. Automat nie walczy z operatorem, a stan „HOLD” na warstwie jest widoczny w UI.
4. **Odporność**: brak połączenia z silnikiem → ponawianie z narastającym odstępem i czerwony status; zaległe komendy nie są wysyłane po powrocie (zamiast tego wyrównanie stanu z pkt 2). Błąd zapisu na dysk nie zatrzymuje harmonogramu. Blokady są odporne na zatrucie (`lk()`), błędy idą przez `Result`.
5. **Zegar**: czas systemowy z NTP (zalecenie w RUNBOOK). Czasy liczone w UTC, wyświetlane w `tz`; zmiana czasu letni/zimowy jest obsłużona w dobie emisji 23/25 h.

## 7. Import i aktualizacja harmonogramu
- **XMLTV** (Tunarr, ErsatzTV, każdy generator EPG): minimalny parser XML bez zależności, tylko potrzebne elementy: `programme@start/stop/channel`, `title`, `desc`, `category`, `rating/value`, `episode-num` oraz znaczniki `live`/`premiere`. Limit rozmiaru, ignorowanie DTD i encji zewnętrznych, bo dane z zewnątrz są wrogie. `eventId` = źródło + kanał + start.
- **JSON/CSV z działu planowania**: zdefiniowany format (kolumny: eventId, begin, dur, title, attrs…).
- **Wpisy ręczne**: „Dodaj program”, „Dodaj zdarzenie na żywo”.
- Tryby aktualizacji jak w Viz Multichannel: **pełna**, **pełna bez usuwania**, **tylko czasy**. Klucz `eventId`; zdarzenia z `lock` zachowują start.
- Odświeżanie co `refreshMin`; różnice trafiają do logu („program 20:00 przesunięty o +2 min”).
- **Ograniczenia źródeł**: Tunarr/ErsatzTV nie zawsze pokazują w EPG wypełniacze i przerwy jako osobne pozycje. Reguły zależne od reklam wymagają kategorii w źródle albo ręcznego oznaczenia. Do sprawdzenia na rzeczywistym kanale (M1).

## 8. Interfejs `/channel` (układ jak w Viz Multichannel)
1. **Menu i pasek narzędzi**: Plik / Narzędzia / Akcje / Pomoc. Zegar (hh:mm:ss:ff), **stan silnika** (połączony / opóźnienie / błąd), konfiguracja. Przyciski: Uruchom/Zatrzymaj playlistę, Take, Continue, Out, Podgląd, Odśwież dane, Szukaj, Filtr.
2. **Panel wyboru (lewy)**: wybór kanału oraz zakładki Playlisty · Strony (z show silnika) · Akcje · Pula · Statystyki.
3. **Obszar roboczy**: drzewo playlisty (program → zdarzenia wtórne, rozwijane strzałką), kolumny: Stan, Start, Długość, Koniec, Offset, Warstwa, Opis, Strona, Media ID, Event ID, Miniatura. Kolumny można przestawiać i ukrywać. Pasek „teraz” przewija się z czasem, a „Przewiń do bieżącego programu” wraca do niego. Przeciąganie zdarzeń wtórnych między programami (Alt+↑/↓), F2 zmiana opisu, Del usuwanie, Ctrl+F szukanie.
4. **Selektor (prawy/dolny)**: Playlisty · Reguły · Import · Informacje (log emisji, błędy, różnice z ostatniego importu).
- Edytor reguły w formularzu z podglądem, które programy dnia spełniają warunki.
- Podgląd strony: ramka `/output?bus=preview` silnika po `ARM`.
- Kolory: zielony = w antenie, żółty = następne, szary = wyłączone, czerwony = błąd (brak strony, puste wymagane pole, brak połączenia), z opisem po najechaniu.

## 9. Zmiany w rdzeniu VizRS (minimalne)
- `STATE` przez TCP i SSE `/events` już są. Do dodania: **pole `source`** w komendach i logu audytowym (`channel:ch1`), żeby log emisji i audyt silnika dało się zestawić.
- Opcjonalnie `GET /api/pages?scene=` z listą pól i miniaturą do kolumny „Miniatura”. Zamiennikiem jest `/api/show`.
- Paczka `examples/BRANDING.cgpack`: logo, zegar, ticker, oznaczenia wiekowe 0/7/12/16/18, „NA ŻYWO”, „Za chwilę/Następnie”, plansza „Przerwa techniczna”.

## 10. Etapy

| Etap | Zakres | Testy / kryterium |
|---|---|---|
| **M0** fundament | binarka `vizrs-channel` (CLI `--port --data --token --bind`, blokada danych, SIGTERM, `/health`), model `channel.json` + walidator, wspólne moduły HTTP/JSON | `cargo test` (walidator, parser CLI), clippy |
| **M1** import | parser XMLTV (bez zależności, limity), import JSON/CSV, playlista dnia/ciągła, trzy tryby aktualizacji, strefy czasowe i zmiana czasu | wektory XMLTV z Tunarr i ErsatzTV (`tests/vectors/xmltv/*`), testy doby 23/25 h |
| **M2** reguły | warunki, kotwice start/end/%, offset ±, addContinue, duration/durationAttr/whole, ochrona wpisów ręcznych | testy jednostkowe na stałym harmonogramie (oczekiwane zdarzenia wtórne) |
| **M3** silnik | pętla klatkowa, `delayMs`, klient TCP do `vizrs`, wyrównywanie stanu z SSE, catch-up, STOP/HOLD, log emisji `asrun/<data>.jsonl` | test z udawanym zegarem; E2E: Channel + `vizrs` + Playwright na `/output` – logo wchodzi o czasie, wraca po restarcie silnika |
| **M4** UI | `web/channel.html` (4 strefy, drzewo, kolumny, filtry, edytor reguł, pula, podgląd), okno `vizrs channel` | `tests/e2e/channel.js`, zrzuty ekranu |
| **M5** branding | `BRANDING.cgpack`, reguły startowe (§4), dokumentacja (`DOKUMENTACJA.md`, `INSTRUKCJA-poczatkujacy.md`, `API.md`, RUNBOOK: Tunarr/ErsatzTV + NTP) | próba 24 h na kanale testowym Tunarr/ErsatzTV, porównanie logu emisji z EPG |
| **M6** opcje | eksport/raport HTML, statystyki, GPO, wiele kanałów w jednym UI, Docker compose (Tunarr/ErsatzTV + vizrs + channel) | według potrzeb |

Zamknięcie M3 zamyka też LUKI #12 dla brandingu. Rundown studyjny w Trio zostaje ręczny.

## 11. Decyzje (2026-10-23)
1. **Osobne repozytorium** (nie binarka w crate `vizrs`, nie workspace w `source/`). Zasada „zero zależności” dotyczy rdzenia VizRS, w nowym repo zależności ustala się osobno (ffplayout używa wielu crate'ów). Z VizRS rozmawia tylko przez protokół: TCP 6100, `/api/*`, SSE `/events`.
2. **Nazwa: „VizRS Playout”** (binarka/repo `vizrs-playout`).
3. **Własny playout na podstawie [ffplayout](https://github.com/ffplayout/ffplayout)** zamiast Tunarr/ErsatzTV jako źródła sygnału. Harmonogram = playlisty playoutu (zdarzenia główne znamy dokładnie, z klatką, bo to my odtwarzamy klip). **XMLTV później** – najpierw jako eksport EPG z playlisty, import zewnętrzny jako opcja (dawne M1).
4. **Klatkaż i zapis czasu** – do potwierdzenia, wyjaśnienie niżej. Rekomendacja: 25 fps, czas w UI jako `hh:mm:ss:ff`, wewnętrznie w klatkach.

> Wyjaśnienie pkt 4: telewizja w Polsce nadaje 25 klatek na sekundę. Vizrt pokazuje czasy i przesunięcia nie w milisekundach, lecz w formacie *godziny:minuty:sekundy:klatki*, np. `00:00:05:12` = 5 s i 12 klatek (12 × 40 ms). Pytanie brzmi: czy w VizRS Playout liczymy i pokazujemy czas w klatkach (dokładnie jak w emisji, każde zdarzenie trafia w konkretną klatkę), czy w zwykłych sekundach/ms. Przy własnym playoucie klatki są naturalne, bo ffmpeg i tak koduje klatka po klatce.

## 12. Kierunek po decyzjach: playout na bazie ffplayout

- **ffplayout** (Rust + ffmpeg): playlisty dzienne JSON, tryb ciągły, wypełniacze, wyjście HLS/RTMP/SRT/dekklink, panel WWW. **Licencja GPL-3.0** – jeśli kopiujemy lub forkujemy kod, repo „VizRS Playout” musi być na GPL-3.0. Do ustalenia: fork czy własna implementacja inspirowana ffplayout.
- **Grafika VizRS na sygnale**: playout zna czas każdej klatki, więc reguły (§4) liczy względem zdarzeń z własnej playlisty, a komendy wysyła do `vizrs` (TCP 6100). Do rozstrzygnięcia w P0: jak nałożyć `/output` (strona przeglądarki) na wideo w ffmpeg – (a) zewnętrzny mikser (OBS/vMix) bierze wideo z playoutu i `/output`, (b) renderowanie `/output` bezgłowym Chromium do strumienia z kanałem alfa i filtr `overlay` w ffmpeg, (c) zrzut klatek przez SSE/WebSocket. Rekomendacja na start: (a), docelowo (b).
- **Etapy (zamiast M0–M6)**: P0 repo, licencja, prototyp nakładki; P1 odtwarzanie playlisty dnia (ffmpeg, wypełniacz, wyjście SRT/HLS); P2 reguły i zdarzenia wtórne → `vizrs`; P3 UI (układ jak §8); P4 log emisji, EPG/XMLTV (eksport, potem import); P5 branding (`BRANDING.cgpack`), Docker compose `vizrs` + `vizrs-playout`.
- **W tym repo** zostają tylko drobne dodatki do rdzenia z §9 i dokumentacja współpracy.
