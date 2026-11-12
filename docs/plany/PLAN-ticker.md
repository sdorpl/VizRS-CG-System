# PLAN – tickery w VizRS

Status: **wdrożony** (2026-10-19) – poza opcjonalnym T5.2 (źródło zewnętrzne, `docs/LUKI.md` #16).
Plan respektuje założenia Z1–Z10 z [PLAN-rozbudowy.md](PLAN-rozbudowy.md): wygląd i zachowanie projektuje się w Artist, Trio tylko podaje dane.

## 1. Wymagania

### 1.1 Model danych
- Ticker ma listę wiadomości z kategorią (`{ text, category }`); kategoria wyznacza etykietę i jej kolor.
- Ustawienia tickera: tryb (whip / pionowy / poziomy), prędkość (px/s) albo czas wiadomości (s), styl separatora (skos, kropka, kwadrat, kreska, brak), etykieta domyślna dla wiadomości bez kategorii, wygląd pasa kategorii (kolory, gradient, font, błysk).
- Kolory kategorii jako mapa `"PILNE=#e00000; INFO=#0050c0"`; pas kategorii bierze kolor z tej mapy.
- Edytor pokazuje tylko te ustawienia, których dotyczy wybrany tryb.

### 1.2 Tryby odtwarzania
| Tryb | Zachowanie |
|---|---|
| **whip** | Wiadomość pojawia się (fade), wisi, znika; **pas kategorii przejeżdża w prawo przez cały pasek**, w połowie drogi zmienia się etykieta na kategorię następnej wiadomości, pas wraca na miejsce, start następnej. |
| **pionowy** | Podmiana wiadomości co zadany czas (min. 1,5 s), etykieta zmienia się z krótkim fade'em. |
| **poziomy** | Klasyczny crawl: tor z dwiema kopiami listy, prędkość px/s, separator między wiadomościami. |

Wspólne: etykieta = kategoria wiadomości, a gdy jej brak → etykieta domyślna; brak etykiety ukrywa pas kategorii; tekst wiadomości zaczyna się za pasem kategorii; błysk przejeżdżający po pasie co N s.
Wiadomość, która się nie mieści, jest ściskana, a dopiero potem dostaje mniejszy font.

### 1.3 Praca operatora
- **Szybka edycja:** wiadomości pogrupowane w karty kategorii, nazwa kategorii edytowalna w nagłówku karty, przeciąganie wiadomości między kartami, „+ grupa”, „+ wiadomość”; zapis od razu trafia na antenę.
- **Lista wierszy:** `[treść] [kategoria] ↑ ↓ ✕`, „+ dodaj wiadomość”, kolory kategorii w tej samej sekcji, tryb i prędkość jako listy wyboru.
- Zmiana wiadomości na antenie **nie może restartować tickera**.

## 2. Stan VizRS (2.0.0)

- Węzeł `ticker` w drzewie sceny ([renderer.js:90](../../source/web/renderer.js#L90)) z trybami `crawl`, `roll` (góra–dół), `wipe` (maska z „wycieraczką”).
- Wiadomości = wiersze tekstu pola `text`; kategoria w składni `KAT#kolor|treść` ([parseTickerItem](../../source/web/renderer.js#L439)).
- Właściwości: `hold`, `speed`, `sep`, `sepColor`, `catW`, `catBg`, `catFg` + typografia. Każdą można podpiąć jako pole kontrolne (np. `tickerMode` jako select dla operatora).
- Squeeze per wiadomość w roll/wipe (scaleX do 0,6, potem zmniejszenie fontu).
- Animacja liczona z czasu (`now - wallStart`), więc działa z `tools/render_video.py` (podmieniony `performance.now`).
- Trio: pole tickera to zwykła `textarea` (6 wierszy) – operator wpisuje `KAT#kolor|treść` ręcznie.

### 2.1 Różnice do zamknięcia
| # | Wymaganie | VizRS 2.0.0 | Priorytet |
|---|---|---|---|
| D1 | Edytor listy wiadomości z kategoriami (wiersze, ↑↓, grupy) | textarea ze składnią `KAT#kolor\|` | **P1** |
| D2 | Mapa kolorów kategorii na poziomie sceny | kolor wpisywany w każdej wiadomości albo jeden `catBg` | **P1** |
| D3 | Domyślna etykieta, gdy wiadomość nie ma kategorii | brak etykiety → pusty pas kategorii | P1 |
| D4 | Tryb **whip** (pas kategorii przejeżdża przez pasek) | `wipe` jest podobny, ale odsłania maską, wiper nie przejeżdża całego paska | P2 |
| D5 | Style separatora (skos, kropka, kwadrat, pionowa kreska) | tylko tekst `sep` | P2 |
| D6 | Błysk (gleam) na etykiecie | brak | P3 |
| D7 | Ciągłość tickera przy zmianie wiadomości | `update()` czyści cache – crawl przeskakuje (zmienia się długość cyklu), roll/wipe skaczą do innego indeksu | **P1** |
| D8 | Presety startowe tickerów w edytorze | brak | P2 |

## 3. Plan implementacji

### T1. Model danych i parser (renderer.js, validate.rs) – P1
1. **Zostawiamy format wierszy** `KAT#kolor|treść` jako zapis w show (zgodność z TCP/OSC/HTTP, CSV rundownu, makrami i istniejącymi show). Edytor z T3 tylko go generuje i parsuje.
2. W `renderer.js` dodajemy `serializeTickerItem({cat, color, text})` jako odwrotność `parseTickerItem` i eksportujemy oba (dla Trio i Artist). Ucieczka znaku `|` w treści: `\|`.
3. Nowe właściwości węzła `ticker` (grupa „Ticker”, w PROPS):
   - `catColors` (string) – mapa `PILNE=#e00000; INFO=#0050c0`. Kolejność koloru: kolor z wiadomości > `catColors` > `catBg`.
   - `label` (string) – etykieta domyślna dla wiadomości bez kategorii. Pusta = dzisiejsze zachowanie.
   - `sepStyle` (select: `text` | `dot` | `square` | `pipe` | `skew`) – `text` = dzisiejszy `sep`.
4. `validate.rs`: limity tekstu tickera (np. ≤ 200 wiadomości, ≤ 500 znaków na wiadomość), poprawność `catColors`. Wektory testowe w `tests/vectors` dla parsera i serializera (te same reguły w JS i w Rust, jeśli serwer kiedyś będzie parsował – np. komendy TCP z T5).

### T2. Renderer – P1/P2
1. **Ciągłość przy UPDATE (D7, P1).** Stan tickera trzymamy osobno od `it.cache` (np. `it.tk = { x, idx, phase0 }`), a `update()` go nie zeruje:
   - crawl: pozycja toru `x` przyrasta o `speed·dt` (z `now`, nie z timerów, żeby działał `render_video.py`); po zmianie wiadomości nowy tor dobudowuje się **za bieżącą kopią**, więc to, co jest na ekranie, przewija się dalej bez skoku, a nowe wiadomości wjeżdżają od prawej krawędzi;
   - roll/wipe/whip: bieżąca wiadomość dogrywa swój `hold`, następna jest już z nowej listy (indeks szukany po treści, a gdy jej nie ma – od początku listy).
2. **Etykieta domyślna i mapa kolorów (D2, D3)** w `tickerSlots` i w crawl (plakietki kategorii).
3. **Tryb `whip` (D4, P2)** jako czwarta opcja `tickerMode`: wiadomość in (fade 0,2 s) → hold → out → wiper (pas kategorii) przejeżdża w prawo przez `w`, w połowie zmienia etykietę na kolejną kategorię, wraca (`translateX`), start następnej. Czasy ≈0,4 s out, 1,0 s przejazd, 0,5 s powrót, skalowane przez `hold`. Całość liczona z fazy w `hold`, jak dziś roll/wipe, bez `setTimeout`.
4. **Separatory (D5, P2)**: `sepStyle` rysowany jako `<span>` z CSS (kropka = koło `0.3em`, kwadrat, kreska `2px × 0.9em`, skos = kreska `skewX(-20deg)`), kolor `sepColor` albo kolor tekstu z kryciem 50%.
5. **Błysk (D6, P3)**: opcjonalne `gleam` (bool) + `gleamEvery` (s) na pasie kategorii – gradient w `::after`, pozycja liczona z `now`.
6. Pomiar squeeze per wiadomość wystawiony tak jak dla tekstu (`it.sq`), żeby Trio mogło pokazać, która wiadomość jest ściśnięta (T3.4).

### T3. Trio – edytor wiadomości – P1
1. Dla pola, którego `nodeType === 'ticker'`, zamiast textarea rysujemy **listę wierszy**: `[kategoria ▾] [treść] [licznik] ↑ ↓ ✕`, pod nią „＋ wiadomość” i przełącznik „tekst surowy” (dzisiejsza textarea – dla wklejania i zaawansowanych).
   - Kategoria: pole z podpowiedziami (`<datalist>`) z kluczy `catColors` sceny oraz kategorii już użytych na stronie; kolor widoczny jako kropka obok. Kolor wpisany ręcznie (`#…`) dalej działa.
   - Enter w treści = nowa wiadomość pod spodem; Ctrl+↑/↓ = przesuwanie; Ctrl+Backspace w pustym wierszu = usuń.
   - Wklejenie wielu wierszy do treści = wiele wiadomości (parsowane `parseTickerItem`).
2. **Widok grup (szybka edycja, §1.3)** – przełącznik „grupuj po kategorii”: karty kategorii, zmiana nazwy w nagłówku zmienia kategorię wszystkich wiadomości w karcie, przeciąganie wiadomości między kartami (HTML5 drag & drop, bez Sortable).
3. Każda zmiana serializuje listę do wierszy, więc reszta Trio (draft, `dirtyPage`, Zapisz, UPDATE, blokady, reguły pól) działa bez zmian.
4. Podgląd: klik w ticker w dużym podglądzie przenosi do edytora; najechanie na wiersz wiadomości przełącza podgląd na tę wiadomość (roll/wipe/whip) i pokazuje jej squeeze – operator sprawdza każdą wiadomość bez czekania na rotację.
5. Reguły pola (`fieldMeta`, edytowane w Artist): `maxItems`, `maxChars` (na wiadomość), `cats` (lista dozwolonych kategorii → select zamiast wolnego pola), `upper`.

### T4. Artist (edytor graficzny) – P2
1. Panel właściwości węzła `ticker` pokazuje tylko to, co dotyczy wybranego trybu (crawl: `speed`, `sep*`; roll/wipe/whip: `hold`, `cat*`, `label`).
2. „Wiadomości próbne” w Artist edytowane tym samym komponentem listy co w Trio (T3.1) – jeden moduł `ticker-items.js` w `source/web/`, dołączany przez oba widoki.
3. **Presety startowe (D8)** w menu „＋”: *Ticker pełny ekran* (rect 1920×65 + ticker crawl), *Pasek z wiperem* (rect + ticker whip z `label`), *Crawl z linią akcentu*, *Pionowy słup* (rect + ticker roll, wysoki). Każdy od razu z polem kontrolnym `NEWS` na `text` i opcjonalnie `tickerMode` jako select dla operatora.
4. Edytor reguł pola (fieldMeta) dostaje sekcję „Ticker” dla `maxItems`, `cats`.

### T5. Sterowanie zewnętrzne i API – P3
1. Komendy TCP/HTTP na liście wiadomości strony, z automatycznym UPDATE, gdy strona jest na antenie: `TICKER <nr> ADD KAT|treść`, `TICKER <nr> DEL <i>`, `TICKER <nr> SET <i> KAT|treść`, `TICKER <nr> CLEAR`. Składnia istniejących komend bez zmian (Z10).
2. (Opcja) źródło zewnętrzne: URL z JSON/RSS i interwał odświeżania na poziomie strony; serwer pobiera i robi UPDATE – pozycja z listy życzeń.

### T7. Testy i dokumentacja
- Wektory JS (`tests/vectors/run.js`): parser/serializer (z `\|`, z kolorem, bez kategorii), mapa `catColors`, kolejność koloru.
- Test renderera bez przeglądarki dla ciągłości UPDATE: pozycja toru i indeks wiadomości przed i po `update()` (podmieniony `performance.now`, jak w `render_video.py`).
- E2E (`tests/e2e/run.js`): edytor listy w Trio – dodaj, przesuń, zmień kategorię → poprawny zapis wierszy; UPDATE na antenie nie restartuje tickera (zrzut przed/po).
- `cargo test`: limity w `validate.rs`.
- Dokumentacja: `CHANGELOG.md`, `docs/API.md` (komendy z T5), instrukcja operatora (edytor wiadomości), `docs/LUKI.md` (zamknięte D1–D8).

## 4. Kolejność i szacunek

| Etap | Zakres | Szacunek |
|---|---|---|
| 1 | T1 + T2.1–T2.2 + testy T7 | 2–3 dni |
| 2 | T3 (edytor listy, grupy, podgląd wiadomości) | 2 dni |
| 3 | T2.3–T2.4 (whip, separatory) + T4 (Artist, presety) | 2–3 dni |
| 4 | T2.5, T5 (błysk, komendy, źródło zewnętrzne) | 1–2 dni |

Kryterium ukończenia: ticker ma kategorie, kolory kategorii i tryby z §1.2; operator edytuje wiadomości listą bez znajomości składni; UPDATE na antenie nie przerywa przewijania; testy wektorowe, `cargo test` i E2E zielone.
