# PLAN – belka „Za chwilę / Następnie” przed końcem audycji

> Status (2026-11-02): **Z1, Z2, Z3 i Z6 wdrożone** – VizRS CG Playout 2.7.0 (gałąź `za-chwile`), VizRS: pole strony `playout` i **⏱ kanał** w Trio. Czeka: Z4 (odliczanie), Z5 (osobna scena, jeśli KafelProgramu nie wystarczy), weryfikacja na stosie (§5). Zgoda użytkownika na zmiany w obu repozytoriach: 2026-11-02.

## 1. Cel

Na kanale ErsatzTV, na przykład **60 s przed końcem audycji**, ma wejść belka „Za chwilę: *tytuł następnej audycji* o *godzina*”, a kilka sekund przed końcem ma zejść. Ma to działać samo, bez operatora, dla wszystkich audycji albo tylko dla wybranych.

## 2. Stan obecny

| Gdzie | Jak liczone są opóźnienia | Wniosek |
|---|---|---|
| Sidecar, `in_seconds` / `out_seconds` (`config.rs`, `VIZRS_IN_SECONDS`, `VIZRS_OUT_SECONDS`) | Komendy idą `in_seconds` po `MediaItem_Start`, a `OUT` stron z linii `TAKE` idzie `out_seconds` przed `MediaItem_Stop`. Oba czasy są w osi wideo pozycji (`timed()` w `src/vizrs.rs`, z poprawką na `lead_seconds`). | Czasy są **globalne**, a wejście liczy się **tylko od startu**. Nie da się ustawić „TAKE N s przed końcem”. |
| Sidecar, symbole (`template.rs`) | `{next_title}`, `{next_start}`, `{next_start_full}`, `{next_duration}` z EPG ErsatzTV (`epg_entries: 3`) | Treść belki jest już dostępna. |
| Sidecar, reklama | Każda reklama podbija `ad_generation`, więc zaplanowana grafika przepada. Pierwszy zwykły klip przywraca strony zdjęte przez `STATE`. | Belka zaplanowana przed reklamą śródprogramową przepada (por. §4.3). |
| VizRS, scena | `motion.in/out.delay`, stop pointy: opóźniają animację, a nie komendę | Nie nadaje się. |
| VizRS, makra | `+2.5 TAKE 3001`: opóźnienie stałe od uruchomienia makra | Nie nadaje się, bo długości audycji są różne, a `{duration}` to tekst bez arytmetyki. |

Brakuje więc tylko **planowania komendy względem końca**. Silnik VizRS nie potrzebuje zmian.

## 3. Rozwiązanie w skrócie

1. **Z1:** w sidecarze wspólny mechanizm „czasu względem końca” (rozszerzenie `timed()`).
2. **Z2:** szablon `on_end` w konfiguracji, czyli belka „Za chwilę” dla każdej audycji z elementem **VizRS** (także w `shows[]` i `channels{}`). Zmiany w harmonogramach ErsatzTV nie są potrzebne.
3. **Z3:** parametry czasu w adresie elementu strony (`page=1405&at=-60`), żeby belkę można było zaznaczać w ErsatzTV tylko przy wybranych pozycjach.
4. **Z4 (opcjonalnie):** symbol pełnej daty startu następnej audycji do odliczania zegarem VizRS.
5. **Z5:** w VizRS scena i strona belki gotowe dla kanału (renderer natywny, lekkie `/output`).
6. **Z6:** czas strony na kanale ustawiany w Trio (`page.playout`), przepisywany przez synchronizację sidecara do adresu elementu.

Przykład docelowej konfiguracji sidecara:

```json
"on_end": "TAKE 1405 next=Za chwilę|programs={next_title}|times={next_start}",
"end_seconds": 60,
"end_out_seconds": 5,
"end_ref": "programme"
```

## 4. Etapy

### Z1 – planowanie względem końca (sidecar, `src/vizrs.rs`)

- Nowa struktura `Timing { at: f64, out: Option<f64>, end_ref: EndRef }`, gdzie `at ≥ 0` oznacza sekundy po starcie, a `at < 0` sekundy przed końcem. `out` to sekundy przed końcem; `None` oznacza brak `OUT`. Dzisiejsze zachowanie to `Timing { at: in_seconds, out: Some(out_seconds), end_ref: Item }`, więc obecne wektory i testy się nie zmieniają.
- `timed()` dostaje `Timing` zamiast czytać `config.in_seconds` i `config.out_seconds`. `take_at` i `out_at` liczy tak jak dziś przez `made_at`, czyli w osi wideo z `lead_seconds`.
- **Koniec** (`EndRef`):
  - `item` to `MediaItem_Stop`, czyli koniec pozycji;
  - `programme` to `Stop` wpisu EPG, w którym leży start pozycji (pole `Epg` danych ErsatzTV; dziś czytane tylko dla `{next_*}`). Bez EPG obowiązuje `item`.
- **Reguła okna:** moment TAKE musi leżeć w `[start pozycji, stop pozycji)`. Jeśli wypada w innej części tej samej audycji (np. odcinek pocięty reklamami śródprogramowymi), ta sesja nic nie wysyła, bo wyśle ją część, w której moment leży. Widz dołączający w trakcie okna `[take, out)` wywołuje TAKE od razu, a po `out` nie wywołuje nic (jak dziś przy `in_seconds`).
- **Za krótko:** gdy pozycja kończy się przed TAKE albo `take ≥ out`, belka nie wchodzi (log `debug`, jak dziś).
- **Wymuszone `OUT`:** grafika planowana od końca musi mieć `OUT`. Inaczej belka zostałaby na antenie, przy reklamie zdjęłoby ją `STATE`, a pierwszy klip następnej audycji przywróciłby ją (`restore`). Bez jawnego `out` stosujemy `end_out_seconds`, a wartość ujemna jest odrzucana z ostrzeżeniem.
- **Klucz powtórzeń** (`state.sent`) obejmuje też `Timing`, żeby `on_clip` i `on_end` tej samej pozycji nie blokowały się nawzajem.
- Testy jednostkowe w `vizrs.rs` (udawany VizRS, krótkie czasy, jak `in_and_out_seconds`): TAKE przed końcem, wejście w trakcie okna, pozycja za krótka, `programme` w drugiej części audycji, reklama anulująca plan, brak EPG.

### Z2 – `on_end` w konfiguracji (sidecar)

- Nowe opcje `on_end` (szablon, domyślnie pusty, czyli wyłączony), `end_seconds` (60), `end_out_seconds` (5) i `end_ref` (`item` | `programme`, domyślnie `programme`), ze zmiennymi `VIZRS_ON_END`, `VIZRS_END_SECONDS`, `VIZRS_END_OUT_SECONDS`, `VIZRS_END_REF`.
- `on_end` można nadpisać w `shows[]` (np. tylko „Wiadomości”) i w `channels{}`. Pusty `on_end` w regule wyłącza belkę dla tej audycji.
- `clip()` przy zwykłej pozycji z ogólnym elementem **VizRS** planuje `on_clip` (dzisiejszy `Timing`) i niezależnie `on_end` (`at = -end_seconds`). Reklama nie dostaje `on_end`, a elementy `page=` / `cmd=` też go nie dostają (tam służy Z3).
- Zgodnie z zasadą z CLAUDE.md sidecara obejmuje to `config.rs` z testem, `config.example.json`, `docker-compose.yml` (zmienne) oraz DOKUMENTACJA §3 (nowa podsekcja „Belka przed końcem”) i §5.

### Z3 – czas w adresie elementu strony (sidecar, `server.rs`, `sync.rs`)

- Parametry adresu `at` (liczba ze znakiem), `out` i `ref`, np. `http://cg:7700/etv?channel=1&page=1405&fields=next=Za chwilę&at=-60&out=5&ref=programme`. Błędna wartość daje ostrzeżenie w logu i domyślne czasy, a wartości są ograniczone do ±86 400 s, bo dane z zewnątrz są wrogie.
- `server.rs` czyta je obok `page`/`take`/`cmd` i przekazuje `Timing` do `events.session()`.
- `sync.rs`: `at`, `out` i `ref` dopisane do `ELEMENT_PARAMS`, żeby `channel_url()` nie przenosił ich na wszystkie generowane `page-<ID>.yml`. Pliki generowane nadal dostają tylko `page=`; czas dopisuje się ręcznie w osobnym pliku albo w adresie elementu (do ustalenia, §6 p. 3). Test w `sync.rs`.
- W ErsatzTV element „Za chwilę (1405)” zaznacza się przy wybranych pozycjach harmonogramu, obok **VizRS**. Sesja niesie przezroczyste klatki, a komendy wysyła sama (mechanizm z DOKUMENTACJA §3 sidecara).
- Testy kontraktu (`tests/contract/`, `npm test`): parametry adresu dochodzą do komend w odpowiednim czasie.

### Z4 – odliczanie do następnej audycji (opcjonalnie)

- Symbol `{next_start_iso}` (`2026-11-02T20:00:00+01:00`) do pola `target` zegara VizRS w trybie `target` (format `mm:ss`). `zoneTime()` w `renderer.js` przyjmuje już przesunięcie strefy.
- Zegar `/output` liczy według `Date.now()`, a klatki powstają z wyprzedzeniem. Przesunięcie wyrównuje już `clock_offset_seconds` sidecara. Trzeba tylko sprawdzić odliczanie w nagraniu kanału.
- Renderer natywny `vizrs-render` musi umieć tryb `target`. Sprawdzi to `vizrs-render check` dla sceny z Z5; brak jest zgłaszany, a nie zgadywany.

### Z5 – scena i strona w VizRS CG System

- Punkt wyjścia: istniejąca scena **KafelProgramu** (`ix_rot`, strona **1405** w INFO24; `rx_rot` / 7505 w R7), warstwa `prog`, pola `next`, `programs`, `times`. Działa bez zmian kodu: `TAKE 1405 next=Za chwilę|programs={next_title}|times={next_start}`.
- Warstwa jest **osobna** od głównej belki (`on_clip`), więc „Za chwilę” nie zdejmuje belki programu. Jeśli ma ją zastępować, wystarczy ta sama warstwa. `OUT 4001` strony już zastąpionej daje tylko ostrzeżenie w logu sidecara i nie jest błędem emisji.
- Długość animacji wyjścia sceny musi być ≤ `end_out_seconds`, żeby belka zeszła przed cięciem.
- Opcjonalnie: wariant sceny z dwoma wierszami („Za chwilę” i „Następnie”: `{next_title}` oraz kolejny wpis EPG) wymaga symbolu `{after_next_*}` w sidecarze (EPG ma 3 wpisy). Decyzja w §6 p. 4.
- Sprawdzenie: `vizrs-render check` (renderer natywny) i `/output?fps=…` (lekkie wyjście, [PLAN-lekkie-output.md](PLAN-lekkie-output.md)). Jeśli powstanie nowa scena, trafia do paczki w `examples/` (`tools/build-packs.mjs`) i do danych startowych sidecara (`seed/`).

### Z6 – czas strony na kanale w Trio (oba repozytoria)

- VizRS: strona ma opcjonalne `playout: { at, out, ref }` (`at` −86 400…86 400 s, ujemne = przed końcem; `out` 0…86 400 s; `ref` `item`/`programme`), reguła w `validate.rs` z testem. Silnik tylko je przechowuje.
- Trio: w statusie edytora rozwijane **⏱ kanał** (wejście: domyślnie / po starcie / przed końcem + sekundy, zejście, koniec), zapis razem ze stroną.
- Sidecar: `parse_pages` czyta `playout` (wartości spoza zakresu pomija), `/etv/pages` je przekazuje, a synchronizacja wpisuje `at`/`out`/`ref` do URL wygenerowanych `page-<ID>.yml` – także istniejących (reszta URL zostaje). Pliki ręczne bez zmian.

### Decyzje (2026-11-02)

Przyjęte propozycje z §6: koniec domyślnie `programme`; pierwsze wydanie Z1+Z2+Z3, a do tego Z6 (prośba o ustawianie czasu w Trio); treść jednowierszowa (KafelProgramu); Z4 później. Pliki elementów (§6 p. 3): wariant „przed końcem” dostaje się, ustawiając czas strony w Trio (Z6), bez osobnych plików `-end`.

## 5. Weryfikacja na stosie

1. `docker compose up -d` (sidecar z gałęzi), kanał testowy z dwiema krótkimi audycjami (np. 3 min), `on_end` z `end_seconds: 30`.
2. Nagranie strumienia kanału: belka wchodzi 30 s przed końcem pozycji, schodzi 5 s przed nim, a tytuł i godzina następnej audycji się zgadzają.
3. Odcinek z reklamą śródprogramową i `end_ref: programme`: belka pojawia się tylko w ostatniej części.
4. Dołączenie widza 20 s przed końcem: belka wchodzi od razu. Dołączenie 3 s przed końcem: brak belki.
5. Sprzątanie po teście bez ruszania danych działających instancji (DOKUMENTACJA §7 sidecara).

## 6. Decyzje do podjęcia

1. **Domyślny koniec:** `programme` (koniec audycji z EPG, odporny na reklamy śródprogramowe) czy `item` (koniec pozycji, prostszy)? Propozycja: `programme`.
2. **Zakres pierwszego wydania:** Z1+Z2 (belka globalnie i per audycja z konfiguracji) wystarcza do celu z §1. Z3 tylko wtedy, gdy belka ma być zaznaczana ręcznie w harmonogramie ErsatzTV.
3. **Z3, plik elementu:** czy `vizrs-cg sync` ma umieć tworzyć wariant „przed końcem” strony (np. `page-1405-end.yml`), czy wystarczy ręczny plik?
4. **Treść:** jeden wiersz („Za chwilę: X o 20:00”) czy dwa („Za chwilę” i „Następnie”), który wymaga `{after_next_*}`?
5. **Odliczanie (Z4):** potrzebne teraz czy później?

## 7. Zmiany i dokumentacja

| Repozytorium | Pliki | Dokumentacja |
|---|---|---|
| VizRS CG Playout | `src/vizrs.rs`, `src/config.rs`, `src/server.rs`, `src/sync.rs`, `src/template.rs` (Z4), `config.example.json`, `docker-compose*.yml`, testy jednostkowe i `tests/contract/` | DOKUMENTACJA §3 (nowa podsekcja), §5 (opcje i zmienne); wersja **2.7.0** |
| VizRS CG System | ewentualna nowa scena w `examples/archiwum/*.show.json` i paczkach (`tools/build-packs.mjs`) | ten plan (status), wiersz w `docs/plany/README.md`; kodu silnika nie trzeba zmieniać |

Bramki: w sidecarze `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo build --release && npm test`. W VizRS przy nowej scenie: `node tools/native-compare.js` i `node tests/e2e/pack.js`.
