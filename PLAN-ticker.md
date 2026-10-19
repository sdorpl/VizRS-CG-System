# PLAN – tickery w VizRS na wzór BTB CG System

Status: **wdrożony** (2026-10-19) – poza opcjonalnym T5.2 (źródło zewnętrzne, `docs/LUKI.md` #16). Źródło analizy: `BTB-CG-System-alpha` – `app.js` (inspektor, szybka edycja wiadomości),
`inspector-window.html` / `inspector-shared.js` (okno Trio), `scene-model.js` i `scene_designer.js` (edytor scen, warstwa `ticker`),
`renderer.js` (zmienne szablonów), szablony `templates/ticker-unified.json`, `tpl-btb-fullwidth-ticker.json`,
`tpl-urgent-ocg-wiper.json`, `upgrade-templates.js` (kolory kategorii, błysk wipera).
Plan respektuje założenia Z1–Z10 z [PLAN-rozbudowy.md](PLAN-rozbudowy.md): wygląd i zachowanie projektuje się w Artist, Trio tylko podaje dane.

## 1. Jak działa ticker w BTB

### 1.1 Model danych
- Element typu `TICKER` ma `items: [{ text, category }]` (dopuszczalne też same stringi – stare dane).
- Do szablonu trafiają jako `{{ITEMS_B64}}` (JSON w base64, w ukrytym `<span data-b64>`), `{{ITEMS_JSON}}`, `{{ITEMS}}`.
- Ustawienia na poziomie elementu: `tickerMode` (`whip` | `horizontal` | `vertical`), `speed` (px/s, a w trybie pionowym **sekundy na wiadomość** – to samo pole o dwóch znaczeniach), `separatorStyle` (`skewed` | `dot` | `square` | `pipe` | `none` → `{{SEPARATOR_CSS}}`), `introText` (domyślna etykieta wipera), `wiper.{show, bgColor, color2, useGradient, gradientAngle, textColor, fontSize, fontWeight, fontFamily, letterSpacing, gleam*}`.
- Kolory kategorii: zmienna `CATEGORY_COLORS` w postaci `"PILNE=#e00000; INFO=#0050c0"`; wiper i plakietka kategorii biorą kolor z tej mapy.
- Inspektor wykrywa możliwości z kodu szablonu (`capabilities()`): `items`, `itemCategory` (kod używa `category`), `speed`, `tickerMode`, `separator`, `wiperShow`. Pokazuje tylko kontrolki, które szablon obsługuje.

### 1.2 Tryby odtwarzania (szablon `ticker-unified`, kod w JS szablonu)
| Tryb | Zachowanie |
|---|---|
| **whip** | Wiadomość pojawia się (fade), wisi 4 s, znika; **wiper (etykieta) przejeżdża w prawo przez cały pasek**, w połowie drogi zmienia się etykieta na kategorię następnej wiadomości, wiper wraca na miejsce, start następnej. |
| **vertical** | Podmiana wiadomości z czasem `speed` s (min. 1,5 s), etykieta zmienia się z krótkim fade'em. |
| **horizontal** | Klasyczny crawl: tor z dwiema kopiami listy, `translateX` w rAF, prędkość px/s, separator między wiadomościami. |

Wspólne: etykieta = kategoria wiadomości, a gdy jej brak → `introText`; brak etykiety ukrywa wiper; tekst wiadomości wyrównywany za szerokością wipera (`--wiper-w`); błysk (gleam) przejeżdżający po wiperze co N s.
Szablon pilnego paska (`tpl-urgent-ocg-wiper`) dodatkowo **zmniejsza font**, gdy wiadomość się nie mieści.

### 1.3 Edytor graficzny (Artist BTB)
- `scene-model.js`: warstwa `ticker` ma tylko `speed`, `separator`, `gap`, `font`; kompiluje się do toru GSAP (`x: 0 → -half`, `repeat: -1`), start w `onShow`, stop w `onHide`. Obsługuje **tylko crawl**.
- `scene_designer.js`: presety startowe „Paski i tickery” (pełny ekran, wiper, crawl, pionowy słup) budowane z plate/badge/text.
- `scene_convert.js`: konwersja szablonu kodowego na scenę rozpoznaje tor (`track|marquee|scroller`) i zamienia go na warstwę `ticker`.
  **Uwaga:** w `templates/ticker-unified.json` konwersja zgubiła logikę – scena ma statyczny tekst „PRZYKŁADOWA WIADOMOŚĆ NR 1”, a tryby whip/vertical przetrwały tylko w `codeBackup`. Nie wolno powtórzyć tego przy imporcie do VizRS.

### 1.4 Praca operatora
- **Szybka edycja** (`openTickerEditor`): wiadomości pogrupowane w karty kategorii, nazwa kategorii edytowalna w nagłówku karty, przeciąganie wiadomości między kartami i kolejności kart (Sortable), „+ grupa”, „+ wiadomość”. Zapis spłaszcza karty do `items[{text, category}]` i od razu wysyła na serwer (wyjście przerysowuje ticker).
- **Okno Trio** (`inspector-window.html`): lista wierszy `[treść] [kategoria] ↑ ↓ ✕`, „+ dodaj wiadomość”, pole `CATEGORY_COLORS` w tej samej sekcji, tryb i prędkość jako selecty.
- Edycja w Edytorze Tekstu (WYSIWYG) dzieli HTML po `<br>/<div>/<p>` na wiadomości – formatowanie łatwo się gubi.
- Zmiana wiadomości na antenie **restartuje ticker** (szablon jest renderowany od nowa).

## 2. Stan VizRS (2.0.0)

- Węzeł `ticker` w drzewie sceny ([renderer.js:90](source/web/renderer.js#L90)) z trybami `crawl`, `roll` (góra–dół), `wipe` (maska z „wycieraczką”).
- Wiadomości = wiersze tekstu pola `text`; kategoria w składni `KAT#kolor|treść` ([parseTickerItem](source/web/renderer.js#L439)).
- Właściwości: `hold`, `speed`, `sep`, `sepColor`, `catW`, `catBg`, `catFg` + typografia. Każdą można podpiąć jako pole kontrolne (np. `tickerMode` jako select dla operatora).
- Squeeze per wiadomość w roll/wipe (scaleX do 0,6, potem zmniejszenie fontu) – odpowiednik `adjustFontSize` z BTB.
- Animacja liczona z czasu (`now - wallStart`), więc działa z `tools/render_video.py` (podmieniony `performance.now`).
- Trio: pole tickera to zwykła `textarea` (6 wierszy) – operator wpisuje `KAT#kolor|treść` ręcznie.
- Import z BTB ([btb-io.js:261](source/web/btb-io.js#L261)): bierze tylko `items[].text`, **gubi kategorie**, tryb zawsze `crawl`.

### 2.1 Różnice do zamknięcia
| # | BTB | VizRS | Priorytet |
|---|---|---|---|
| D1 | Edytor listy wiadomości z kategoriami (wiersze, ↑↓, grupy) | textarea ze składnią `KAT#kolor\|` | **P1** |
| D2 | Mapa kolorów kategorii `CATEGORY_COLORS` na poziomie szablonu | kolor wpisywany w każdej wiadomości albo jeden `catBg` | **P1** |
| D3 | Domyślna etykieta (`introText`), gdy wiadomość nie ma kategorii | brak etykiety → pusty pas kategorii | P1 |
| D4 | Tryb **whip** (wiper przejeżdża przez pasek) | `wipe` jest podobny, ale odsłania maską, wiper nie przejeżdża całego paska | P2 |
| D5 | Style separatora (skos, kropka, kwadrat, pionowa kreska) | tylko tekst `sep` | P2 |
| D6 | Błysk (gleam) na etykiecie | brak | P3 |
| D7 | Restart tickera przy zmianie wiadomości | `update()` czyści cache – crawl przeskakuje (zmienia się długość cyklu), roll/wipe skaczą do innego indeksu | **P1** – VizRS powinien tu być *lepszy* niż BTB |
| D8 | Presety startowe tickerów w edytorze | brak | P2 |
| D9 | Import BTB z kategoriami, trybem i ustawieniami wipera | tylko tekst | P1 |

## 3. Plan implementacji

### T1. Model danych i parser (renderer.js, validate.rs) – P1
1. **Zostawiamy format wierszy** `KAT#kolor|treść` jako zapis w show (zgodność z TCP/OSC/HTTP, CSV rundownu, makrami i istniejącymi show). Edytor z T3 tylko go generuje i parsuje.
2. W `renderer.js` dodajemy `serializeTickerItem({cat, color, text})` jako odwrotność `parseTickerItem` i eksportujemy oba (dla Trio i btb-io). Ucieczka znaku `|` w treści: `\|`.
3. Nowe właściwości węzła `ticker` (grupa „Ticker”, w PROPS):
   - `catColors` (string) – mapa `PILNE=#e00000; INFO=#0050c0`. Kolejność koloru: kolor z wiadomości > `catColors` > `catBg`.
   - `label` (string) – etykieta domyślna dla wiadomości bez kategorii (BTB `introText`). Pusta = dzisiejsze zachowanie.
   - `sepStyle` (select: `text` | `dot` | `square` | `pipe` | `skew`) – `text` = dzisiejszy `sep`.
4. `validate.rs`: limity tekstu tickera (np. ≤ 200 wiadomości, ≤ 500 znaków na wiadomość), poprawność `catColors`. Wektory testowe w `tests/vectors` dla parsera i serializera (te same reguły w JS i w Rust, jeśli serwer kiedyś będzie parsował – np. komendy TCP z T5).

### T2. Renderer – P1/P2
1. **Ciągłość przy UPDATE (D7, P1).** Stan tickera trzymamy osobno od `it.cache` (np. `it.tk = { x, idx, phase0 }`), a `update()` go nie zeruje:
   - crawl: pozycja toru `x` przyrasta o `speed·dt` (z `now`, nie z timerów, żeby działał `render_video.py`); po zmianie wiadomości nowy tor dobudowuje się **za bieżącą kopią**, więc to, co jest na ekranie, przewija się dalej bez skoku, a nowe wiadomości wjeżdżają od prawej krawędzi;
   - roll/wipe/whip: bieżąca wiadomość dogrywa swój `hold`, następna jest już z nowej listy (indeks szukany po treści, a gdy jej nie ma – od początku listy).
2. **Etykieta domyślna i mapa kolorów (D2, D3)** w `tickerSlots` i w crawl (plakietki kategorii).
3. **Tryb `whip` (D4, P2)** jako czwarta opcja `tickerMode`: wiadomość in (fade 0,2 s) → hold → out → wiper (pas kategorii) przejeżdża w prawo przez `w`, w połowie zmienia etykietę na kolejną kategorię, wraca (`translateX`), start następnej. Czasy jak w BTB (≈0,4 s out, 1,0 s przejazd, 0,5 s powrót) skalowane przez `hold`. Całość liczona z fazy w `hold`, jak dziś roll/wipe, bez `setTimeout`.
4. **Separatory (D5, P2)**: `sepStyle` rysowany jako `<span>` z CSS (kropka = koło `0.3em`, kwadrat, kreska `2px × 0.9em`, skos = kreska `skewX(-20deg)`), kolor `sepColor` albo kolor tekstu z kryciem 50% (jak w `upgradeTickerFw`).
5. **Błysk (D6, P3)**: opcjonalne `gleam` (bool) + `gleamEvery` (s) na pasie kategorii – gradient w `::after`, pozycja liczona z `now`.
6. Pomiar squeeze per wiadomość wystawiony tak jak dla tekstu (`it.sq`), żeby Trio mogło pokazać, która wiadomość jest ściśnięta (T3.4).

### T3. Trio – edytor wiadomości – P1
1. Dla pola, którego `nodeType === 'ticker'`, zamiast textarea rysujemy **listę wierszy**: `[kategoria ▾] [treść] [licznik] ↑ ↓ ✕`, pod nią „＋ wiadomość” i przełącznik „tekst surowy” (dzisiejsza textarea – dla wklejania i zaawansowanych).
   - Kategoria: pole z podpowiedziami (`<datalist>`) z kluczy `catColors` sceny oraz kategorii już użytych na stronie; kolor widoczny jako kropka obok. Kolor wpisany ręcznie (`#…`) dalej działa.
   - Enter w treści = nowa wiadomość pod spodem; Ctrl+↑/↓ = przesuwanie; Ctrl+Backspace w pustym wierszu = usuń.
   - Wklejenie wielu wierszy do treści = wiele wiadomości (parsowane `parseTickerItem`).
2. **Widok grup (jak „Szybka edycja” BTB)** – przełącznik „grupuj po kategorii”: karty kategorii, zmiana nazwy w nagłówku zmienia kategorię wszystkich wiadomości w karcie, przeciąganie wiadomości między kartami (HTML5 drag & drop, bez Sortable).
3. Każda zmiana serializuje listę do wierszy, więc reszta Trio (draft, `dirtyPage`, Zapisz, UPDATE, blokady, reguły pól) działa bez zmian.
4. Podgląd: klik w ticker w dużym podglądzie przenosi do edytora; najechanie na wiersz wiadomości przełącza podgląd na tę wiadomość (roll/wipe/whip) i pokazuje jej squeeze – operator sprawdza każdą wiadomość bez czekania na rotację.
5. Reguły pola (`fieldMeta`, edytowane w Artist): `maxItems`, `maxChars` (na wiadomość), `cats` (lista dozwolonych kategorii → select zamiast wolnego pola), `upper`.

### T4. Artist (edytor graficzny) – P2
1. Panel właściwości węzła `ticker` pokazuje tylko to, co dotyczy wybranego trybu (crawl: `speed`, `sep*`; roll/wipe/whip: `hold`, `cat*`, `label`), jak warunkowe etykiety w inspektorze BTB.
2. „Wiadomości próbne” w Artist edytowane tym samym komponentem listy co w Trio (T3.1) – jeden moduł `ticker-items.js` w `source/web/`, dołączany przez oba widoki.
3. **Presety startowe (D8)** w menu „＋”: *Ticker pełny ekran* (rect 1920×65 + ticker crawl), *Pasek z wiperem* (rect + ticker whip z `label`), *Crawl z linią akcentu*, *Pionowy słup* (rect + ticker roll, wysoki). Każdy od razu z polem kontrolnym `NEWS` na `text` i opcjonalnie `tickerMode` jako select dla operatora.
4. Edytor reguł pola (fieldMeta) dostaje sekcję „Ticker” dla `maxItems`, `cats`.

### T5. Sterowanie zewnętrzne i API – P3
1. Komendy TCP/HTTP na liście wiadomości strony, z automatycznym UPDATE, gdy strona jest na antenie: `TICKER <nr> ADD KAT|treść`, `TICKER <nr> DEL <i>`, `TICKER <nr> SET <i> KAT|treść`, `TICKER <nr> CLEAR`. Składnia istniejących komend bez zmian (Z10).
2. (Opcja) źródło zewnętrzne: URL z JSON/RSS i interwał odświeżania na poziomie strony; serwer pobiera i robi UPDATE – BTB tego nie ma, to pozycja z listy życzeń, nie z parytetu.

### T6. Import / eksport BTB (btb-io.js) – P1
1. `items[{text, category}]` → wiersze `KAT|treść`; `CATEGORY_COLORS` → `catColors`; `introText` / `WIPER_TEXT` → `label`.
2. Tryby: `horizontal` → `crawl`; `vertical` → `roll` (`speed` BTB w sekundach → `hold`); `whip` → `whip` (po T2.3; do tego czasu → `wipe` z ostrzeżeniem w raporcie importu).
3. `separatorStyle` → `sepStyle`; `wiper.bgColor`/`textColor`/`fontSize`/`fontFamily` → `catBg`/`catFg`/rozmiar i font etykiety.
4. Element `TICKER` **bez** warstwy `ticker` w scenie (jak `ticker-unified.json` po konwersji) → budujemy węzeł ticker z `items` i ustawień elementu, zamiast przenosić statyczny tekst, i dopisujemy ostrzeżenie do raportu.
5. Eksport: odwrotnie (`whip`/`roll`/`crawl` → `tickerMode`/`speed` BTB, kategorie → `items[].category`).
6. Wektory testowe w `tests/vectors/btb.js` na podstawie trzech szablonów z `BTB-CG-System-alpha/templates`.

### T7. Testy i dokumentacja
- Wektory JS (`tests/vectors/run.js`): parser/serializer (z `\|`, z kolorem, bez kategorii), mapa `catColors`, kolejność koloru.
- Test renderera bez przeglądarki dla ciągłości UPDATE: pozycja toru i indeks wiadomości przed i po `update()` (podmieniony `performance.now`, jak w `render_video.py`).
- E2E (`tests/e2e/run.js`): edytor listy w Trio – dodaj, przesuń, zmień kategorię → poprawny zapis wierszy; UPDATE na antenie nie restartuje tickera (zrzut przed/po).
- `cargo test`: limity w `validate.rs`.
- Dokumentacja: `CHANGELOG.md`, `docs/API.md` (komendy z T5), instrukcja operatora (edytor wiadomości), `docs/LUKI.md` (zamknięte D1–D9).

## 4. Kolejność i szacunek

| Etap | Zakres | Szacunek |
|---|---|---|
| 1 | T1 + T2.1–T2.2 + T6 + testy T7 | 2–3 dni |
| 2 | T3 (edytor listy, grupy, podgląd wiadomości) | 2 dni |
| 3 | T2.3–T2.4 (whip, separatory) + T4 (Artist, presety) | 2–3 dni |
| 4 | T2.5, T5 (błysk, komendy, źródło zewnętrzne) | 1–2 dni |

Kryterium ukończenia: show z tickerem `ticker-unified` z BTB importuje się z kategoriami, kolorami i trybem; operator edytuje wiadomości listą bez znajomości składni; UPDATE na antenie nie przerywa przewijania; testy wektorowe, `cargo test` i E2E zielone.
