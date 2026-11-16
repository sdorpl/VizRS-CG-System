# PLAN-ruch-belek – wzajemny ruch belek (dokowanie jak w BTB CG)

> Stan: **wdrożony** (2026-11-16): A0, A0b, A1, A2, B1–B4 zrobione; A3 odłożone, A4 i dopasowanie do BTB – poza zakresem, B5 (nazwany stos) – osobna decyzja (decyzje §7).
> Zastępuje szkic J6 z [PLAN-jeden-ruch.md](PLAN-jeden-ruch.md) i etap P5 planu Artist (`../VizRS CG Artist/docs/plany/PLAN-pozycjonowanie-scen.md`).
> Pakiet: **VizRS CG System** (format, silnik, oba renderery, Trio), **VizRS CG Artist** (edycja, podgląd), **VizRS CG Playout** (wydanie).

## 1. Jak działa dokowanie w BTB CG

Źródło: repozytorium `sdorpl/BTB-CG-System` (panel „CG Control Pro”, wyjście `output.html`): `renderer.js` – `showGraphic`, `recalculateAttachments`;
`app.js` – formularz „Powiązania (Dokowanie)”; dane – `db.json`. Założenie: „BTB CG Master” = ten system (do potwierdzenia, §7 pkt 1).

**Model.** Każda grafika (instancja w panelu operatora, z treścią i układem) ma w `layout`:

| Pole | Znaczenie |
|---|---|
| `attachedToGraphicId` | lista grafik-celów dla osi Y (wybór wielokrotny w panelu) |
| `attachOffsetY` | stałe przesunięcie Y w px, „gdy cel jest na żywo” |
| `attachedToGraphicIdX`, `attachOffsetX` | to samo dla osi X, z osobną listą celów |

**Zachowanie** (`recalculateAttachments`, wołane po każdej zmianie stanu):
- przesunięcie osi = `attachOffset`, gdy **którykolwiek** cel z listy jest widoczny (`visible`), inaczej 0 – jedna wartość na oś, bez sumowania;
- ruch płynny: `transition: transform 0.6s cubic-bezier(0.22, 1, 0.36, 1)` (szybki start, miękkie hamowanie);
- grafika wchodząca, gdy cel już jest – od razu z przesunięciem (`showGraphic`, `initialOffsetY/X`), bez dojeżdżania;
- zejście celu: `visible = false` w stanie → przeliczenie od razu, więc grafika wraca **w chwili OUT celu**, równolegle z jego animacją wyjścia;
- bez łańcucha (patrzy tylko, czy cel jest widoczny), bez pomiaru rozmiaru celu, bez nadpisania operatora poza zmianą pól.

**Konfiguracje w danych** (`db.json`, 44 grafiki, 7 z dokowaniem):

| Grafika | Cele | Przesunięcie |
|---|---|---|
| WYDANIE SPECJALNE (y 930) | Belka główna, Belka pilne (y 855) | Y −130 |
| Z OSTATNIEJ CHWILI (y 930) | Belka główna, Belka pilne | Y −130 |
| Republika Logo (y 880) | Belka główna, Belka pilne | Y −125 |
| MIASTO lewa / prawa (y 105) | NA ŻYWO lewa / prawo (+ „tylko w btb info”) | Y +55 (pod plakietką) |
| tylko w btb info | NA ŻYWO prawo | X −175 (w lewo) |
| NA ŻYWO prawo | tylko w btb info | Y 0 |

## 2. Porównanie z VizRS (stan 26.11.3 / Artist 26.11.4)

VizRS ma `scene.dock = [{ target, dx, dy, dur }]` (DOKUMENTACJA §4.7). Test 2026-11-13 na `/output` (ticker `back`, WS `z2_ws`
z regułą `scene:Wizytowka, dy −140`, wizytówka `main`): WS jedzie w górę przy TAKE wizytówki, wraca przy OUT, wchodzi od razu na miejsce, gdy
wizytówka już jest; ⇅ w Trio wyłącza powiązanie.

| Cecha BTB | VizRS | Ocena |
|---|---|---|
| kilka celów, przesunięcie, gdy którykolwiek widoczny | `target: "scene:Belka glowna\|scene:Belka pilne"` – jedna reguła | **jest** |
| jedna wartość na oś (bez sumowania) | jedna reguła z `a\|b` – tak; osobne reguły – sumują się | **jest**, ale edytor powinien prowadzić do jednej reguły |
| osobne cele dla X i Y | dwie reguły (`dx` i `dy`) | **jest** |
| wejście przy widocznym celu – od razu przesunięte | do 26.11.3 w scenach z tekstem / ruchem belka **dojeżdżała** z miejsca bazowego (§2.1, przypadek E); poprawione w A0 | **jest** (po A0) |
| powrót w chwili OUT celu | tak | **jest** |
| czas 0,6 s | `dur`, domyślnie 0,6 s | **jest** |
| krzywa `cubic-bezier(0.22, 1, 0.36, 1)` | stała `cubic-bezier(.4, 0, .2, 1)` | **inna** – ruch wolniej rusza |
| cele = konkretne grafiki (instancje) | `page:<numer>` (instancja) albo `scene:` (wszystkie strony szablonu), `layer:` | **jest** (i więcej) |
| ustawienie w panelu operatora przy grafice | reguła w szablonie (Artist); Trio – tylko ⇅ wł./wył. | **inaczej** – w Trio nie da się zmienić reguły |
| wybór celów z listy | Artist: pole tekstowe `scene:…` z podpowiedziami | **gorzej** – trzeba znać składnię |
| podgląd ruchu przy ustawianiu | (nie sprawdzone w BTB) · Trio: monitor PGM; Artist – brak podglądu ruchu | **brak** w Artist |

### 2.1 Odwzorowanie na lokalnym stosie (2026-11-13)

BTB: prawdziwe `output.html` + `renderer.js` z BTB-CG-System, stan z `db.json` podany przez podstawione socket.io (Chromium, 1920×1080).
VizRS: lokalny stos (`virtrs-vizrs-1`, `ghcr.io/sdorpl/vizrs:latest`, `/output`), sceny z geometrią zmierzoną w BTB – „BTB Belka glowna” / „BTB Belka pilne”
(`main`, 220–1870 × 859–975, zoom 0,25 s) i „BTB Wydanie specjalne” (`z2_ws`, 220–560 × 930–974) z regułą przepisaną 1:1 z `db.json`:
`{ target: "scene:BTB Belka glowna|scene:BTB Belka pilne", dy: -130, dur: 0.6 }`; strony 7001–7003. Górna krawędź WS (px) co 0,1 s:

| Przypadek | BTB | VizRS |
|---|---|---|
| A. sam WS | 930 | 930 |
| B. TAKE Belki głównej | 930 → 852 → 817 → 804 → 801 → **800** | 926 → 890 → 840 → 812 → 802 → **800** |
| D. Belka pilne zastępuje główną (ta sama warstwa) | – | 800 bez ruchu |
| C. OUT belki | wraca na **930** | 807 → 850 → 896 → 920 → 928 → **930** |
| E. WS wchodzi, gdy belka już jest | **803 → 800** (od razu na miejscu; zmiana = jego zoom) | **924 → 880 → 834 → 810 → 800** (dojeżdża z dołu) |

Położenia końcowe zgodne co do piksela (WS 15 px nad wizytówką). Różnice: krzywa (VizRS w pierwszych 0,1 s przejeżdża 4 px, BTB 78 px – A1)
i przypadek E (A0). Przyczyna E: oba renderery traktują pierwsze ustawienie przesunięcia po TAKE jak zmianę na antenie – przeglądarka animuje od 0,
jeśli zdążyła policzyć style nowego elementu (tekst, pomiary; w scenie z samym prostokątem nie animuje), renderer natywny zawsze od (0,0)
(`Dock::default`, `set_dock` w `scene.rs`).

### 2.2 Obserwacja z testów na serwerze S2 (2026-11-15)

Scenariusz testowy: strona ID 1405 (Kafel programu, warstwa `prog`) posiada regułę dokowania do strony ID 1001 (Belka, warstwa `main`):
`"dock": [{ "target": "page:1001", "dy": -90, "dur": 0.6 }]` – powinna odjechać 90 px w górę po pojawieniu się belki.

- **W przeglądarce (`/output`)**: dokowanie działa prawidłowo. Wyjście `/output` bez parametrów renderuje wszystkie warstwy w ramach jednej instancji `LayerHost`, lista `air` zawiera wszystkie aktywne elementy anteny, cel zostaje wykryty i przesunięcie następuje.
- **Na antenie (renderer do ErsatzTV w VizRS CG Playout)**: brak jakiegokolwiek ruchu, kafel 1405 stoi w miejscu.
- **Przyczyna**: w `VirtRS CG Playout` domyślnie aktywny jest tryb `native_layers = true` (jeden proces `vizrs-render stream` na warstwę, z adresem `/output?layers=<nazwa>`). W `host.rs` (oraz analogicznie w `renderer.js`) zdarzenia `take` i `snapshot` są filtrowane przez `accepts(&layer)`:
  proces dedykowany dla warstwy `prog` natychmiast odrzuca zdarzenie `take` warstwy `main`. W rezultacie lista `air` w danym procesie zawiera wyłącznie jego własną warstwę. Proces jest całkowicie „ślepy” na pozostałe warstwy show, cel reguły `dock` nigdy nie jest wykrywany (`hit == false`), a przesunięcie wynosi zawsze 0. Rozwiązanie opisano w §4 etap A0b.

**Wniosek:** semantyka anteny VizRS już odpowiada BTB – wszystkie 7 konfiguracji z `db.json` da się zapisać bez zmiany formatu
(np. WS: `{ "target": "scene:Belka glowna|scene:Belka pilne", "dy": -130, "dur": 0.6 }`). Różnice są w **krzywej ruchu**, w **niezawodności na antenie przy wielu procesach (A0b)** oraz w **wygodzie ustawiania**
(lista celów, podgląd, ustawianie przez operatora). Dopasowanie do faktycznej wysokości celu (zastąpienie sztywnych wartości rzeczywistym obrysem) wykracza poza BTB –
zostaje jako rozszerzenie (część B, B1).

## 3. Zasady

- **Zgodność wstecz:** istniejące reguły `dx/dy` działają jak dziś; nowe pola opcjonalne. Stary serwer ignoruje nieznane pola reguły dock
  (`validate.rs` sprawdza tylko `target`, `dx`, `dy`, `dur`).
- **Liczy renderer, nie serwer** (jak dziś: `LayerHost.recomputeDock`, `host.rs: recompute_dock`) – oba renderery z tą samą semantyką,
  zmiana w `renderer.js` = ta sama w `source/render/` + `node tools/native-compare.js`.
- Bez nowych zależności. Dokumentacja razem z kodem (CG System DOKUMENTACJA §4.7 i API, Artist DOKUMENTACJA §4, CHANGELOG-i).

## 4. Część A – pełna zgodność z BTB (bez dużej zmiany formatu)

### A0. Błąd: wejście przy widocznym celu (CG System: `renderer.js`, `source/render`) – **zrobione** (2026-11-13, CHANGELOG „Niewydane”)
Pierwsze przesunięcie nowego odtwarzacza (po TAKE, `rescene`, `relayer`) – bez animacji: `setDock(dx, dy, dur, { instant: true })` w `LayerHost.take`
(albo znacznik „jeszcze nie ustawiony” w `ScenePlayer`), w `scene.rs` `Dock { from: to }` przy pierwszym `set_dock`. Test e2e: przypadek E z §2.1
na scenie z tekstem i ruchem (oba renderery). Poprawka bez zmiany formatu – może wyjść w najbliższym wydaniu CG System, przed resztą planu.

### A0b. Błąd krytyczny: brak dokowania na antenie w trybie wielu procesów (Playout: `native_layers`, CG System: `host.rs`, `renderer.js`) – **zrobione** (2026-11-16)
- **Problem (§2.2)**: `VirtRS CG Playout` z domyślnym `native_layers: true` uruchamia proces `vizrs-render` na każdą warstwę show (`?layers=<warstwa>`). W `host.rs` i `renderer.js` metody `take` i `snapshot` sprawdzają `if (!self.accepts(name)) return;` i odrzucają zdarzenia innych warstw. Proces warstwy zależnej (np. `prog`) nie rejestruje wejścia celu z innej warstwy (`main`), lista `air` w `recompute_dock()` zawiera tylko warstwę własną, reguła dokowania nigdy się nie aktywuje i na antenie nic się nie przesuwa.
- **Obejście natychmiastowe na emisyjnym S2**: ustawienie `VIZRS_NATIVE_LAYERS=0` w zmiennych środowiskowych sidecara Playout (jeden proces na wszystkie warstwy, który widzi całą antenę naraz).
- **Poprawka w silniku (CG System)**:
  - Rozdzielenie rejestracji stanu anteny (`air`) od tworzenia odtwarzacza (`Player`) i rastrowania (`draw`).
  - W `Host::handle` / `LayerHost.handle`: zdarzenia `take`, `snapshot`, `out`, `clear`, `relayer`, `dock` aktualizują lekką mapę obecności warstw na antenie (`air: (layer, page_number, scene_name)`) dla **wszystkich warstw** show.
  - Warunek `accepts(layer)` decyduje wyłącznie o powołaniu odtwarzacza `Player` (oraz elementów DOM w `renderer.js`) i rysowaniu na płótnie.
  - Dzięki temu procesy warstw w `native_layers` zachowują pełną izolację i wydajność (rysują tylko swoje warstwy), a `recompute_dock()` w każdym procesie ma pełną listę `air` i bezbłędnie przelicza przesunięcia powiązań.

### A1. Krzywa ruchu (CG System: `renderer.js`, `source/render`) – **zrobione** (2026-11-16)
Opcjonalne pole reguły `ease`: `"std"` (domyślna, dotychczasowa `cubic-bezier(.4,0,.2,1)`), `"out"` (`cubic-bezier(.22,1,.36,1)`, szybki start, miękkie hamowanie). Walidacja + testy,
oba renderery. Powrót po zejściu celu z czasem i krzywą reguły (dotąd stałe 0,6 s). Domyślna bez zmian – istniejące sceny ruszają się jak dotąd.

### A2. Wygodne ustawianie w Artist (`web/artist.html`) – **zrobione** (2026-11-16; podgląd 👁 i ✋ były od 26.11.4)
- Tabela „Powiązania” jak formularz BTB: **cele z listy** (sceny i warstwy show, wielokrotny wybór → jedna reguła `a|b`), **oś** (Y / X), **przesunięcie**, **czas**, **krzywa**.
  Reguły z ręcznie wpisanym `page:` i dowolnym `target` – dalej edytowalne w polu tekstowym.
- Ostrzeżenie, gdy dwie reguły tej samej osi mają wspólny cel (suma przesunięć, inaczej niż w BTB).
- **Podgląd ruchu** (P4 z planu Artist): przy scenach odniesienia przełącznik „na antenie” dla celów reguł; bieżąca scena przesuwa się na kanwie
  o wynik reguł (ta sama funkcja co na antenie – eksport z `renderer.js`), ▶ odtwarza ruch z `dur` i krzywą. Ostrzeżenie o nachodzeniu (26.11.4) liczone z przesunięciem.
- „Położenie względem innej sceny → jako powiązanie” (26.11.4) – bez zmian, wpisuje wyliczone `dy`/`dx`.
- `tests/e2e/okno.js`: reguła z listy celów zapisana w scenie, podgląd przesuwa scenę o `dy`.

### A3. Trio – reguły widoczne i (opcjonalnie) ustawiane przez operatora – **odłożone** (decyzja 2026-11-16: na razie tylko Artist)
- Dymek ⇅ na liście „Na antenie” wypisuje reguły warstwy („↑ 130 px, gdy: Belka główna, Belka pilne”) – operator wie, co wyłącza.
- Opcjonalnie (decyzja §7 pkt 3): **nadpisanie na stronie** `page.dock` – jak w BTB, operator ustawia powiązanie przy konkretnej grafice;
  gdy strona ma `page.dock`, zastępuje reguły szablonu. Zmiana formatu (walidacja strony, `engine.rs` przekazuje regułę w zdarzeniu `take`, oba renderery),
  formularz w Trio wzorowany na „Powiązania (Dokowanie)” z BTB.

### A4. Przeniesienie konfiguracji z BTB – **nie będzie** (decyzja 2026-11-16)
Narzędzie `tools/btb-dock.mjs` (CG System): czyta `db.json` BTB i wypisuje reguły `dock` do wklejenia (albo łata scenę po nazwie):
`attachedToGraphicId` + `attachOffsetY` → `{ target: "scene:<nazwa>|…", dy }`, oś X analogicznie, krzywa `btb`. Tylko jeśli grafiki BTB są przenoszone do VizRS.

### A5. Wydania
CG System (A1, A3) → Artist (`tools/sync-cg.sh`, A2) → Playout (`VIZRS_REF`, test kanału: strefy grafiki przy pierwszym odjeździe belki – ślad strony nie zna
położenia po dock; sprawdzić, że poszerzenie strefy przy obrazie spoza niej, S3b, nie ucina belki).

## 5. Część B – ponad BTB (dokowanie dynamiczne o zmiennej geometrii)

Rozszerzenia, których BTB nie posiadał – kluczowe, gdy belki i elementy oprawy mają zmienną wysokość zależną od treści (np. belka informacyjna 1001 z 1 vs 2 wierszami tekstu i tłem z `autofollow`):

| # | Rozszerzenie | Format |
|---|---|---|
| B1 | **reguła krawędzi (dynamiczna wysokość)**: `edge` (`top\|bottom\|left\|right`) + `gap` – przesunięcie z rzeczywistego obrysu celu w stanie pierwszego stop pointu (`restRect`), z wartościami strony; tylko odsuwanie; zapasowe `dx/dy` dla starszych silników | tak |
| B2 | **łańcuch**: obrys celu z jego własnym przesunięciem (belka nad WS jedzie z nią); pętle reguł pomijane i zgłaszane w `/api/state` | razem z B1 |
| B3 | **kilka celów przy B1**: najdalsze przesunięcie zamiast sumy (odsuwanie na maksymalną kolizję) | razem z B1 |
| B4 | **powrót po animacji OUT celu**: `back: "end"` (BTB i dziś VizRS – w chwili rozpoczęcia OUT) | tak |
| B5 | **stos belek**: nazwany stos w show zamiast reguł w każdej scenie | tak, duża |

**Stan (2026-11-16):** B1–B4 wdrożone w obu rendererach (`dockShift`, `edgeShift`, `restRect` w `renderer.js`; `dock_shift`, `rest_rect`, `drawn_rect` w `source/render`), walidacja, Artist (zachowanie / odstęp / powrót, podgląd z danymi strony celu). Różnice względem projektu niżej: kolizja wymaga nakładania zakresów na drugiej osi; obrys liczy też całe pola tickera, obrazu i QR; serwer nie dołącza `restRect` do zdarzeń – każdy proces mierzy sam z treści zdarzenia (scena + wartości). B5: układ stosu daje już B1 + B2 + B3 (każda belka „nad” niższymi); osobny format `stacks` w show – tylko po decyzji.

### 5.1 Szczegóły techniczne B1: Dynamiczna wysokość celu i reguła krawędzi

1. **Format reguły (`scene.dock[]`)**:
   Zamiast sztywnego offsetu w px (`dy: -90`), reguła definiuje krawędź odniesienia i margines:
   ```json
   {
     "target": "page:1001",
     "edge": "top",
     "gap": 15,
     "dy": -90,
     "dur": 0.6,
     "ease": "btb"
   }
   ```
   - `edge`: krawędź celu, do której dosuwana jest nasza przeciwna krawędź (`"top"` $\rightarrow$ nasz dół do góry celu; `"bottom"` $\rightarrow$ nasza góra do dołu celu; `"left"` $\rightarrow$ nasza prawa do lewej celu; `"right"` $\rightarrow$ nasza lewa do prawej celu).
   - `gap`: odstęp w pikselach (np. `15`), domyślnie 0.
   - `dy` / `dx`: opcjonalny fallback stałego przesunięcia (gdy silnik nie zna `edge` lub obrys celu nie jest jeszcze dostępny).
   - `dur` i `ease`: czas trwania i krzywa animacji przejścia do nowej pozycji.

2. **Pomiar obrysu w spoczynku (`restRect`)**:
   - Elementy grafiki w chwili wejścia (TAKE) animują się (wipe, zoom, fade), dlatego dynamiczny offset **nie może** być mierzony klatka po klatce w trakcie animacji wejściowej celu, gdyż powodowałoby to szarpanie i niepotrzebne wibracje elementu dokowanego.
   - Renderer mierzy geometrię celu w jego **stanie spoczynku** (pierwszy stop-point, po odliczeniu animacji `in`), po podstawieniu realnych wartości pól tekstowych (`values`).
   - Obrys (`restRect: [min_x, min_y, max_x, max_y]`) wyznaczany jest jako suma prostokątów widocznych węzłów rysujących (tekst po ukształtowaniu przez rustybuzz/DOM, prostokąty tła z autofollow, obrazy), z pominięciem pustych grup i cieni.

3. **Wyliczanie przesunięcia osi**:
   Gdy cel pojawia się na antenie:
   - Dla `edge: "top"`:
     `cel_top = target.y + target.restRect.min_y`
     `wlasny_bottom = self.y + self.restRect.max_y`
     `potrzebne_dy = (cel_top - gap) - wlasny_bottom`
     Jeśli `potrzebne_dy < 0` (nachodzą na siebie): aplikowane jest `dy = potrzebne_dy` (odjechanie w górę o realną wysokość celu + gap). Jeśli brak kolizji: `dy = 0`.
   - Analogicznie dla `edge: "bottom"` (`dy = (target.bottom + gap) - self.top`).

4. **Współpraca z `native_layers` (wiele procesów)**:
   - Dzięki poprawce A0b każdy proces warstwy zna zdarzenie `take` celu (`scene` + `values`).
   - Proces warstwy zależnej może wyliczyć `restRect` celu bez jego rastrowania (używając struktury sceny celu i `values` otrzymanych w SSE), albo serwer VizRS może dołączać wyliczony statyczny `restRect` celu do zdarzenia `take` w SSE.

5. **Edycja w VizRS CG Artist**:
   - W panelu „Powiązania (dokowanie)” dodany przełącznik trybu: *Stały offset (px)* vs *Dynamicznie do krawędzi celu*.
   - Przy wyborze krawędzi: wybór relacji (Nad celem / Pod celem / Z lewej / Z prawej) oraz pole marginesu *Odstęp (gap)* w px.
   - Podgląd na kanwie uwzględnia wariant tekstu celu (np. podgląd z 1 vs 2 wierszami tekstu belki).

Obrys = suma prostokątów węzłów rysujących (bez grup i pustych prostokątów, tekst – obrys napisu), jak chwytanie i pomiar w Artist 26.11.4.
Etapy B: rozpoznanie (pomiar obrysu w rendererze natywnym bez rastrowania, koszt w lekkim `/output`), `validate.rs` + `renderer.js` (`restRect`, `dockShift`
eksportowane dla Artist), renderer natywny + `native-compare.js`, Artist (kolumny „zachowanie / odstęp / powrót”, podgląd z wariantem danych celu), Playout.
Szacunek B1–B4: ok. 5 dni łącznie w trzech repozytoriach; B5 – osobny plan po decyzji.

## 6. Szacunek i testy części A

| Etap | Repo | Format | Szacunek |
|---|---|---|---|
| A0 | CG System (oba renderery) | nie | 0,25 dnia |
| A1 | CG System | pole opcjonalne `ease` | 0,5 dnia |
| A2 | Artist | nie | 1,5 dnia |
| A3 | CG System (Trio) | dymek – nie; `page.dock` – tak | 0,25 dnia / +1,5 dnia |
| A4 | CG System (narzędzie) | nie | 0,5 dnia |
| A5 | wydania, Playout | nie | 0,5 dnia + test na stosie |

Scenariusze (e2e na `/output`, pomiar położenia co 100 ms jak w teście z 2026-11-13; te same w `native-compare.js` i podglądzie Artist):
1. WS (y 930) z celami „Belka główna | Belka pilne”, `dy −130`: TAKE jednej z belek – WS w górę; druga belka dochodzi – WS bez zmian (bez sumy); obie schodzą – WS wraca.
2. WS wchodzi, gdy belka główna już jest – od razu na miejscu (A0; scena z tekstem i ruchem – na niej błąd jest widoczny).
3. MIASTO pod plakietką NA ŻYWO (`dy +55`), „tylko w btb info” w lewo (`dx −175`) – osie niezależne.
4. Krzywa `btb` – położenie w połowie ruchu zgodne z `cubic-bezier(0.22, 1, 0.36, 1)` (± 1 px), w obu rendererach.
5. ⇅ operatora – powrót na miejsce; (A3) `page.dock` zastępuje regułę szablonu.

## 7. Decyzje (2026-11-16)

1. „BTB CG Master” = BTB-CG-System – tak; zgodność z BTB i porównania nie są jednak celem (ruch belek rozwijany na własnych zasadach).
2. Krzywa: domyślna bez zmian (`std`), `out` do wyboru w regule (Artist: *zwykła* / *szybka*).
3. Ustawianie powiązań: na razie tylko w Artist; Trio (A3, `page.dock`) – do rozważenia później.
4. Import konfiguracji z BTB (A4) – nie.
5. Część B – wdrożyć razem z częścią A i wydać razem (B1–B4); B5 – osobna decyzja.
