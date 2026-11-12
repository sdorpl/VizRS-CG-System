# Plan: jeden model ruchu w Artist – presety jako klipy na osi czasu (J1–J6)

Stan: propozycja (2026-11-07), wariant C wybrany przez użytkownika; czeka na wdrożenie od J1.

## Cel

Belka z ruchem opisanym presetami i belka z kluczami mają być **nie do odróżnienia w pracy**: jedna oś czasu, jeden panel,
te same narzędzia.

## Stan dziś

- Format show jest już jeden (`show.json`, `validate.rs`, jeden `ScenePlayer`). Węzeł może mieć dwa mechanizmy ruchu:
  klucze `anim` (◆, sposób Viz) i presety `node.motion` (`in`, `out`, `textIn`, `textOut`, `loop`, `bg`, `after`, `web/motion.js`).
- Artist pokazuje je w **dwóch osobnych widokach osi czasu** przełączanych `#seqMode` (`renderTimeline` / `renderSeqTimeline`,
  stan w `localStorage` `vizrs.seq`). Zamiana „Na klucze” / „Z kluczy” (`motionToKeys` / `keysToMotion`) przełącza widok.
- Konflikt: „Na klucze” odmawia, gdy węzeł ma klucze tych samych właściwości; „Z kluczy” – gdy ma preset.
- Brakuje masek warstw i auto-szerokości tła względem tekstu; dock VizRS umie tylko dx/dy (LUKI #7).

## Zasada

**Format show się nie zmienia** (poza J5/J6, które dodają pola). `node.motion` zostaje źródłem prawdy dla presetów, `anim` dla kluczy.
Ujednolicenie jest w edytorze: oba mechanizmy widać i edytuje się w jednym miejscu. Dzięki temu nie ruszamy `renderer.js`,
`source/render/` ani zgodności `native-compare.js` w etapach J1–J4.

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

### J5. Brakujące mechanizmy – maski i follow (zmiana formatu)
- **Maska warstwy**: nowa właściwość węzła `mask: '<id węzła>'` – kształt węzła wskazanego wycina zawartość (alfa). Reguła w `validate.rs` + test,
  renderer (`renderer.js`) i ten sam kod w `source/render/` + `node tools/native-compare.js`; do tego czasu `check` zgłasza ją jako nieobsługiwaną.
- **Auto-szerokość tła względem tekstu**: sprawdzić, czy daje się wyrazić istniejącym `follow`/`followAxis`/`followPad`/`followMin`; brakujące tryby dopisać jako nowe wartości `followAxis`.

### J6. Dock względem celu (#7, zmiana formatu)
- `scene.dock[]` dostaje opcjonalne `edge` (`top|bottom|left|right`) i `gap`: przesunięcie liczone z rzeczywistego prostokąta celu
  (silnik zna on-air warstwy; wysokość celu z `ScenePlayer` – raport przez wyjście jak dziś przy autofollow). `dx/dy` zostają jako tryb prosty.

## Kolejność i bramki

1. J1 + J2 (jeden commit lub dwa) – największa zmiana odczuwalna; tylko `artist.html`.
2. J3, J4 (razem z P7 planu panelu).
3. J5, J6 – zmiany formatu: `validate.rs`, renderer, renderer natywny, DOKUMENTACJA §3–4, API (dock), CHANGELOG, zamknięcie #7 w LUKI.

Każdy etap: `node tests/e2e/artist.js` (testy używające `#seqMode` przepisane na nową oś), `node tests/motion.js && node tests/e2e/motion.js`,
zrzut Playwright osi czasu z belką z presetami i belką z kluczami, opis w DOKUMENTACJA (Artist – oś czasu) i INSTRUKCJA-poczatkujacy.

## Miara sukcesu

- W Artist nie ma przełącznika widoków; ruch każdego węzła widać i zmienia się na jednej osi.
- `native-compare.js` bez nowych różnic.
