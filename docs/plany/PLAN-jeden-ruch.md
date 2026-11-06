# Plan: jeden model ruchu w Artist – presety jako klipy na osi czasu (J1–J7)

Stan: propozycja (2026-11-07), wariant C wybrany przez użytkownika; czeka na wdrożenie od J1.

## Cel

Belka zaimportowana z BTB i belka zrobiona w Artist mają być **nie do odróżnienia w pracy**: jedna oś czasu, jeden panel,
te same narzędzia. Po imporcie z BTB nic nie wymaga ręcznej poprawki.

## Stan dziś

- Format show jest już jeden (`show.json`, `validate.rs`, jeden `ScenePlayer`). Węzeł może mieć dwa mechanizmy ruchu:
  klucze `anim` (◆, sposób Viz) i presety `node.motion` (`in`, `out`, `textIn`, `textOut`, `loop`, `bg`, `after` – sposób BTB, `web/motion.js`).
- Artist pokazuje je w **dwóch osobnych widokach osi czasu** przełączanych `#seqMode` (`renderTimeline` / `renderSeqTimeline`,
  stan w `localStorage` `vizrs.seq`). Zamiana „Na klucze” / „Z kluczy” (`motionToKeys` / `keysToMotion`) przełącza widok.
- Konflikt: „Na klucze” odmawia, gdy węzeł ma klucze tych samych właściwości; „Z kluczy” – gdy ma preset.
- Import BTB gubi: maski warstw, `layout.dock`, auto-szerokość `follow`, „po warstwie” przy wyjściu (LUKI #8), `{sz=}` w px (#11);
  dock VizRS umie tylko dx/dy (#7).

## Zasada

**Format show się nie zmienia** (poza J6/J7, które dodają pola). `node.motion` zostaje źródłem prawdy dla presetów, `anim` dla kluczy.
Ujednolicenie jest w edytorze: oba mechanizmy widać i edytuje się w jednym miejscu. Dzięki temu nie ruszamy `renderer.js`,
`source/render/` ani zgodności `native-compare.js` w etapach J1–J5.

## Etapy

### J1. Jedna oś czasu
- Usunąć przełącznik `#seqMode`. Każdy węzeł z ruchem ma na osi **wiersz węzła** z klipami presetów, pod nim (rozwijane ▸) wiersze kluczy właściwości.
- Klip = prostokąt od `start` do `start+dur`: `in` (zielony), `out` (czerwony, zaczyna się w ostatnim stopie), `textIn`/`textOut` (niebieski,
  cieńszy), `loop` (kreskowany pas między końcem IN a początkiem OUT), `bg` (cienki pas przez całą scenę). Etykieta = `ANIM_LABELS`.
- Czasy liczone tymi samymi funkcjami co renderer (`inEnd`, `outLen`, `sequenceShifts`, `specDelay`), żadnej nowej logiki czasu.
- Wiersz węzła bez ruchu nie istnieje (jak dziś wiersze kluczy). Kolejność: zaznaczony węzeł na górze.

### J2. Edycja klipów myszą
- Przeciągnięcie klipu = `delay` (dla `in`, z uwzględnieniem przesunięcia `after`); prawa krawędź = `dur`; klip `out` przyklejony do stopu – przeciągnięcie
  pyta „przesuń stop?” albo zmienia `delay` wyjścia.
- Klik klipu = zaznacza węzeł i otwiera w panelu zakładkę **Ruch** (P7 z [PLAN-panel-wlasciwosci.md](PLAN-panel-wlasciwosci.md)) na tej fazie.
- Dwuklik na pustym wierszu węzła = menu „dodaj wejście / wyjście / efekt tekstu / pętlę”. Każda zmiana przez `checkpoint()` (cofnij działa).
- Sekwencja `after` rysowana strzałką od końca klipu poprzednika; przeciągnięcie klipu na inny węzeł ustawia `after`.

### J3. Klucze i presety obok siebie bez konfliktu
- Renderer już składa preset z kluczami (`compose`); w edytorze trzeba to pokazać: gdy klip i klucze dotyczą tej samej właściwości,
  wiersz klucza ma znacznik „+ preset” zamiast błędu.
- „Rozbij na klucze” (menu klipu) = dzisiejsze `motionToKeys` dla jednej fazy; przy kolizji **scala** (klucze presetu dodane względem istniejących, `rest` z wartości ścieżki
  w chwili startu) zamiast odmowy. Typy z `UNKEYABLE` – pozycja menu nieaktywna z podpowiedzią dlaczego.
- „Zrób preset z kluczy” = `keysToMotion` dla zaznaczonych kluczy (ramka), z istniejącym przybliżeniem i raportem.

### J4. Jeden panel ruchu
- Sekcja „Ruch – bez kluczy” → zakładka **Ruch** (P7) z listą faz w kolejności czasu: każda faza = typ, kierunek, czas, opóźnienie, krzywa – ten sam formularz co dziś (`motionForm`).
- Na górze zakładki podgląd kluczy węzła (lista właściwości z ◆, skok do osi). Jedno miejsce, gdzie widać cały ruch węzła.

### J5. Import BTB bez ręcznych poprawek – część bez zmian formatu
- `{sz=}` w px (#11): przy imporcie przeliczać px → % względem `size` węzła (zaokrąglenie do 1%), zamiast pomijać. Wektor w `tests/vectors/`.
- „Po warstwie” przy wyjściu (#8): mapować na `after` wyjścia, jeśli renderer obsłuży `after` dla `out` (sprawdzić `sequenceShifts`); inaczej przesunięcie `delay` liczone przy imporcie.
- Raport importu (`btb-io.js`) dzieli ostrzeżenia na „przeniesione przybliżeniem” i „nieprzeniesione”; cel – druga lista pusta dla paczek `examples/BTB*.cgpack`.

### J6. Brakujące mechanizmy – maski i follow (zmiana formatu)
- **Maska warstwy** (#8): nowa właściwość węzła `mask: '<id węzła>'` – kształt węzła wskazanego wycina zawartość (alfa). Reguła w `validate.rs` + test,
  renderer (`renderer.js`) i ten sam kod w `source/render/` + `node tools/native-compare.js`; do tego czasu `check` zgłasza ją jako nieobsługiwaną.
- **Auto-szerokość `follow`** BTB: sprawdzić, czy daje się wyrazić istniejącym `follow`/`followAxis`/`followPad`/`followMin`; brakujące tryby dopisać jako nowe wartości `followAxis`.

### J7. Dock względem celu (#7, zmiana formatu)
- `scene.dock[]` dostaje opcjonalne `edge` (`top|bottom|left|right`) i `gap`: przesunięcie liczone z rzeczywistego prostokąta celu
  (silnik zna on-air warstwy; wysokość celu z `ScenePlayer` – raport przez wyjście jak dziś przy autofollow). `dx/dy` zostają jako tryb prosty.
- Import `layout.dock` z BTB mapuje osie i krawędzie na `edge/gap`.

## Kolejność i bramki

1. J1 + J2 (jeden commit lub dwa) – największa zmiana odczuwalna; tylko `artist.html`.
2. J3, J4 (razem z P7 planu panelu).
3. J5 – tylko `btb-io.js` + wektory; `node tests/vectors/btb.js`, `tools/btb-visual.js --frames`.
4. J6, J7 – zmiany formatu: `validate.rs`, renderer, renderer natywny, DOKUMENTACJA §3–4, API (dock), CHANGELOG, zamknięcie #7/#8/#11 w LUKI.

Każdy etap: `node tests/e2e/artist.js` (testy używające `#seqMode` przepisane na nową oś), `node tests/motion.js && node tests/e2e/motion.js`,
zrzut Playwright osi czasu z belką BTB i belką VizRS, opis w DOKUMENTACJA (Artist – oś czasu) i INSTRUKCJA-poczatkujacy.

## Miara sukcesu

- W Artist nie ma przełącznika widoków; ruch każdego węzła widać i zmienia się na jednej osi.
- Paczki `examples/BTB*.cgpack` importują się bez pozycji „nieprzeniesione”.
- `native-compare.js` bez nowych różnic.
