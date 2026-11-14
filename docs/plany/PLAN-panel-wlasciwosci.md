# Plan: prostszy panel Właściwości w Artist (P1–P8)

Stan: P1–P8 wdrożone. P8 (edycja wielu zaznaczonych) – w VizRS CG Artist 26.11.5 (plan `../VizRS CG Artist/docs/plany/PLAN-edycja-na-kanwie.md` K1).
Różnica względem propozycji: lista właściwości zaawansowanych (`ADV`) i reguły widoczności (`WHEN`) są w `artist.html`, nie w `PROPS` – renderer zostaje bez zmian.

## Diagnoza

Panel `#props` (`renderProps()` w `source/web/artist.html`) to jedna długa lista budowana z `TYPES[n.type].props`:

- węzeł **Tekst** ma ok. 16 właściwości wspólnych (`COMMON`: transform + maska/efekty) i ok. 20 tekstowych, do tego sekcje
  „Ruch – bez kluczy”, „Wariant – widoczność”, „Presety animacji” i blok pomocy; razem 45+ wierszy;
- każdy wiersz ma ten sam ciężar: etykieta, pole, ◆ i łańcuch — rzadko używane `fillSqueeze`, `clipB` czy `skewX`
  wyglądają tak samo ważnie jak `x`, `text` i `color`;
- pary, które w głowie są jedną rzeczą, zajmują osobne wiersze: x/y, w/h, punkt X/Y, maski L/P/G/D, kolor/kolor 2/kąt;
- nie widać, co różni się od domyślnej, co jest animowane i co jest polem kontrolnym, bez przewijania;
- grupy nie zwijają się, a stan nie przetrwa zmiany zaznaczenia (całość renderuje się od nowa);
- opcje zależne od innych są zawsze widoczne (np. `squeezeMin` przy `textFit: none`, kąt gradientu bez `color2`);
  wyjątek jest tylko dla trybów tickera (`TK_HIDE`).

## Rozwiązania

Kolejność = stosunek zysku do pracy. Każdy etap samodzielny; żaden nie zmienia formatu show, renderera ani `validate.rs`
(to wyłącznie warstwa UI Artist), więc nie dotyczy zgodności z `source/render/`.

### P1. Zwijane sekcje z pamięcią stanu
- Nagłówek `.pgroup` staje się przyciskiem ▸/▾; stan zwinięcia per grupa w `localStorage` (`vizrs.propsFold`), wspólny dla wszystkich węzłów.
- Domyślnie rozwinięte: Transform, Wygląd/Tekst/Obraz/Zegar/Ticker (grupa „główna” typu). Zwinięte: Maska / efekty, Autofollow,
  Ruch, Wariant, Presety animacji, pomoc.
- Na zwiniętym nagłówku licznik zmian: „Maska / efekty · 2 zmienione · ◆1”, żeby nic się nie chowało po cichu.
- Koszt: mały (jedna funkcja nagłówka + filtr wierszy). Zysk: od razu ~połowa wysokości panelu.

### P2. Wiersze złożone (pary i czwórki w jednej linii)
- `x | y`, `w | h` (z kłódką proporcji), `punkt X | Y` jako siatka 3×3 zamiast dwóch selectów,
  `maska L P G D` w jednym wierszu, `kolor → kolor 2 · kąt` jako jeden „wypełnienie” (kąt pokazany tylko, gdy `color2` ≠ pusty).
- Każde pole zachowuje własne ◆ i łańcuch (małe, w rogu pola), scrub działa na krótkiej etykiecie „X”, „Y”.
- Tekst: `font` + `grubość` + `kursywa` w jednej linii; `wyrównanie` i `w pionie` jako przyciski ikon (⯇ ≡ ⯈), `WERSALIKI` jako przełącznik.
- Definicja w jednym miejscu: tablica `ROWS` w artist.html (np. `{ props: ['x','y'], kind: 'pair' }`), reszta jak dziś.

### P3. Tryb „Podstawowe / Wszystko”
- Przełącznik w nagłówku panelu. Pole `adv: 1` w `PROPS` (renderer.js — tylko metadane, renderer ich nie czyta) dla rzadkich:
  `skewX`, `blink`, `clip*`, `blur`, `fillSqueeze`, `squeezeMin`, `lineBreak`, `hlColor`, `hlText`, `cycle`, `labelSize`, `labelWeight`, `gleam`, `followMin`…
- W trybie podstawowym właściwość „zaawansowana” i tak się pokazuje, gdy ma wartość ≠ domyślnej, klucz albo pole kontrolne — nic nie znika z oczu, jeśli jest użyte.

### P4. Widoczność warunkowa (rozszerzenie `TK_HIDE` na wszystkie typy)
- Reguły `showWhen` obok definicji: `squeezeMin` tylko przy `textFit` zaczynającym się od `squeeze`; `fillSqueeze` tylko przy `lineBreak: fill`;
  `gradAngle` tylko z `color2`; `borderColor` tylko gdy `borderWidth > 0`; `followAxis/Pad/Min` tylko gdy `follow` ustawione;
  pola zegara wg `mode` (`seconds` dla up/down, `target` dla target); `loop` obrazu tylko dla wideo.
- Ukryta właściwość z kluczami/polem kontrolnym nadal się pokazuje (z ostrzeżeniem „nie działa przy obecnych ustawieniach”).

### P5. Czytelne stany wiersza
- Etykieta pogrubiona + kropka, gdy wartość ≠ domyślna; klik prawym / ⟲ przy kropce = przywróć domyślną (z `checkpoint()`).
- Wiersz animowany: pasek akcentu z lewej; wiersz będący polem kontrolnym: niebieski pasek i nazwa pola (`f_title`) zamiast samej ikony.
- ◆ i łańcuch widoczne słabo, wyraźne na najechaniu wiersza i gdy aktywne — mniej szumu przy 45 wierszach.

### P6. Szukaj właściwości
- Pole „Szukaj…” na górze panelu (skrót `/` gdy fokus nie jest w polu tekstowym): filtruje po etykiecie i kluczu (`spacing`, „odstęp”),
  rozwija pasujące grupy. Przy dużych typach (ticker ~30 właściwości) najszybsza droga.

### P7. Zakładki zamiast jednej listy
- Trzy zakładki u góry panelu: **Właściwości** (transform + wygląd/tekst), **Ruch** (motion IN/OUT, pętla, presety animacji),
  **Dane** (pola kontrolne węzła, wariant/showIf). Odpowiada podziałowi Viz Artist (Container / Animation / Control).
- Blok pomocy przeniesiony do `data-hint` i podpowiedzi na najechanie (już istniejący mechanizm stopki), nie zajmuje miejsca.

### P8. Edycja wielu zaznaczonych (później)
- Gdy zaznaczonych jest kilka węzłów: wspólne właściwości, wartość „—” przy różnych, zmiana ustawia wszystkim jednym `checkpoint()`.
  Wymaga zaznaczenia wielokrotnego w drzewie — osobna praca, dlatego na końcu.

## Kolejność wdrożenia

1. P1 + P5 (jeden commit) — najmniej kodu, największa ulga wizualna.
2. P2 — przebudowa wierszy, trzeba przejrzeć `syncProps`, scrub i obsługę `data-p` / `data-key` / `data-field`.
3. P4 + P3.
4. P6, P7. 5. P8.

Przy każdym etapie: zrzut Playwright panelu dla Tekstu, Prostokąta i Tickera (przed/po), `node tests/e2e/artist.js` musi przejść
(selektory `data-prop`, `data-key`, `data-field` zostają bez zmian, żeby testy i skróty działały), opis w DOKUMENTACJA (sekcja Artist)
i w INSTRUKCJA-poczatkujacy.

## Miara sukcesu

- Tekst z domyślnymi ustawieniami: z ~45 wierszy do ≤ 15 widocznych bez przewijania (P1–P3).
- Każda zmieniona/animowana/podpięta właściwość widoczna bez rozwijania (licznik na grupie albo wiersz).
