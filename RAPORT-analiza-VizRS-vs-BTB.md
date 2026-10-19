# VizRS 1.0.0 — analiza działania i porównanie z BTB CG System (TRIO) oraz BTB CG Editor

Data: 2026-10-19. Zakres: test działania binarki `bin/linux-x86_64/vizrs`, przegląd kodu `source/`,
porównanie z `BTB-CG-System-alpha` (gałąź `TRIO`, odczyt przez `git show`, bez zmiany drzewa roboczego) i `btb-cgeditor`.

## 1. Podsumowanie

VizRS to spójny, bardzo lekki „klon Viz Artist + Trio” w jednym pliku wykonywalnym (≈0,9 MB, zero zależności, ~2,3 tys. linii Rust
+ ~2,6 tys. linii UI). Działa stabilnie, jest szybki i ma świetnie dopracowany model *scena → stop pointy → TAKE/CONTINUE/OUT*
oraz zestaw funkcji newsowych (autofollow, squeeze-lines, tickery, QR, warianty), których BTB nie ma w tej formie.
Jego słabe strony są po stronie **bezpieczeństwa sieciowego, niezawodności danych i narzędzi produkcyjnych** (brak uwierzytelniania,
brak kopii show, brak rundownu, brak testów) — czyli dokładnie tam, gdzie BTB ma plan (M5) albo gotowe rozwiązania.

BTB to przeciwieństwo: bogaty ekosystem (edytor tekstu, pisownia, fonty, paczki, docking, testy ~80 plików), ale ciężki
(Node + SQLite + Socket.io + GSAP + Rust/egui), z otwartymi lukami G1–G3 i ogromnym długiem w synchronizacji całego stanu.

## 2. Wyniki testów VizRS

| Test | Wynik |
|---|---|
| Start, porty 8088 (HTTP/SSE) i 6100 (TCP) | OK, start < 1 s |
| UI: Trio, Artist, Output w Playwright | brak błędów JS; Trio F1 = TAKE, F3 = OUT działają; zrzuty poprawne |
| API: take / continue / out / update / clear / state | OK; update na żywo bez ponownej animacji działa |
| TCP: `STATE`, `TAKE`, `CLEAR` | OK (odpowiedzi `OK {json}`) |
| Wyjście z błędną sceną (zepsute `root`, `tracks`, `stops`) | **odporne** — inna warstwa gra dalej, `update` dalej działa |
| JSON zagnieżdżony 200 000 poziomów | odrzucony (limit głębokości), serwer żyje |
| `PUT /api/show` z błędną strukturą (`scenes:"x"`, `[1,2,null]`) | **przyjęty i zapisany** (200) — brak walidacji schematu |
| Nazwa pliku w `/api/assets` (`../x`, `%2f`, `.hidden`, `\0`) | bezpieczne (zamiana na `_`), brak path traversal |
| Numer strony ujemny / `1e300` | ujemny cicho zamieniony na 1001, `1e300` przyjęty (zaokrąglony do i64) |
| 500 × TAKE przez curl | 3,1 s (≈6 ms / żądanie, w tym start curl) |
| 300 równoległych klientów SSE | OK, ale 301 wątków (wątek na połączenie, bez limitu) |
| Upload 30 MB `.bin` | przyjęty; brak whitelisty typów i limitu łącznego |
| CORS z obcego `Origin` + `GET /api/clear` | **200, wykonane** (`Access-Control-Allow-Origin: *`, brak auth) |

### Znalezione problemy (od najważniejszych)

1. **Brak uwierzytelniania + `CORS: *` + mutujące GET + nasłuch `0.0.0.0`.** Dowolna strona otwarta w przeglądarce operatora może
   wysłać `PUT /api/show` albo `GET /api/clear`/`take` (nawet w sieci studia — ktoś z Wi-Fi). To najpoważniejsza luka przed użyciem na antenie.
2. **Brak ochrony danych show:** jeden `PUT /api/show` (też błędny) nadpisuje `show.json`; jest tylko kopia `show.json.bak` przy
   *uszkodzonym* pliku. Brak rotacji kopii, brak wersji (rev), brak wykrywania dwóch operatorów nadpisujących się nawzajem
   (Trio i Artist zapisują cały show). Błąd zapisu (`fs::write(...).is_ok()` → `rename().ok()`) jest **ignorowany po cichu**.
3. **Brak walidacji modelu** przy `put_show`/`upsert` (typy pól sceny, `stops`, `root`). Renderer wytrzymuje, ale dane śmieciowe zostają.
4. **Zasoby serwera:** wątek na każde połączenie (także SSE), nieograniczone kolejki `mpsc` dla wolnych klientów SSE, brak limitu
   łącznego rozmiaru assets (limit tylko 64 MB na żądanie), brak heartbeat SSE (martwe połączenia wykryte dopiero przy zapisie).
5. **Mutexy z `unwrap()`:** panika w jednym wątku trzymającym `inner.lock()` zatruje mutex i wywróci wszystkie kolejne żądania
   (brak `catch_unwind`/odporności na poison). Dziś nie udało się tego wywołać, ale to ten sam typ ryzyka co G1 w BTB.
6. **Zero testów:** 1 `#[test]` (json.rs), brak testów silnika, API, renderera. Dla projektu, który ma pilnować anteny — to dług nr 1 jakościowy.
7. **Fonty z Google Fonts** ładowane z internetu (`display=block`) — przy awarii sieci na wizji tekst jest niewidoczny/zamieniony
   (jest ścieżka offline, ale brak paczki fontów i manifestu jak w BTB M3.1).
8. **Mniejsze:** `OUT` usuwa warstwę z silnika natychmiast (animację OUT odgrywa tylko klient — kolejny TAKE w czasie OUT może
   ją ściąć); brak „ack” gotowości wyjścia (serwer nie wie, czy OBS faktycznie wyrenderował); brak audytu komend; `/api/state` nie
   pokazuje błędów elementów; skrót „0–9” zawsze przenosi fokus do pola numeru; binarka w `bin/` nie ma odtwarzalnego powiązania ze źródłami (brak sum kontrolnych/wersji w `--help`).

## 3. Porównanie funkcjonalne

Legenda: ✅ jest, 🟡 częściowo, ❌ brak, ➕ wyróżnik.

| Obszar | VizRS | BTB CG System (TRIO) | BTB CG Editor |
|---|---|---|---|
| Stos / dystrybucja | ✅ jeden plik Rust, offline, ➕ | Node+SQLite+Socket.io, Docker/Electron | Rust/egui (natywny) |
| Model graficzny | drzewo kontenerów w Artist (web) | element = kod HTML/CSS/JS **lub** scena (Studio) | scena + warstwy + klucze |
| Animacje | klucze + 6 krzywych, stop pointy ➕ | CGMotion/GSAP: 15 presetów IN/OUT, efekty tekstu, pętle uwagi ➕ | klucze, graph editor, ścieżka ruchu, Béziery (nie grają na wizji) |
| Stop pointy / CONTINUE | ✅ ➕ (rdzeń modelu) | ❌ (był plan M2.6 Local Stop, WASM porzucony) | ✅ w edytorze, nie na wizji |
| Ściskanie tekstu | ✅ squeeze / squeeze-lines per wiersz ➕ | ✅ Global squash, minSquash, wskaźnik ściśnięcia | ✅ parytet |
| Autofollow tła za tekstem | ✅ ➕ (z punktem X/Y, płynnie po UPDATE) | 🟡 autoW/fitText/dock (powiązania M4) | ✅ autoW + fitText |
| Tickery | ✅ 3 tryby, kategorie z kolorem ➕ | ✅ 3 tryby, wiper | ❌ |
| Kod QR, odliczanie, rotacja wierszy, migotanie | ✅ ➕ | 🟡 (zegar, część w szablonach) | 🟡 zegar |
| Edytor tekstu sformatowanego | 🟡 `[[wyróżnienie]]`, zwykłe pola | ✅ TipTap/WYSIWYG, RichText, style, paleta, Word Replace | ✅ pełny (F2), pisownia sjp.pl |
| Pisownia / zamiana słów | ❌ | ✅ (spell-server, Word Replace) | ✅ |
| Fonty offline + manifest | 🟡 plik w assets | ✅ pakiet + manifest + fonty systemowe | ✅ |
| Strony / numery | ✅ numery 1001…, szukanie, duplikowanie | ✅ Bank (shotbox), kod wywołania | — |
| Playlisty / karuzela | ✅ po stronie serwera ➕ | 🟡 presety zestawów, brak rundownu (M5.3 plan) | — |
| Warstwy on-air | ✅ back/main/front/własne ➕ | 🟡 attachment/dock, brak warstw wg Viz | — |
| Warianty (przyciski w Trio) | ✅ ➕ | 🟡 | — |
| Sterowanie zewn. | ✅ HTTP GET/POST + TCP :6100 ➕ | ❌ (plan M5.1/5.2: Companion/OSC) | — |
| Hotkeys | F1–F4, Ctrl+S, Shift+Esc | ✅ F1/F2/Esc + **per-grafika** | — |
| Preview/Program, key/fill | 🟡 podgląd lokalny w Trio, brak osobnego busa | 🟡 Preview/Program w panelu; M5.5 plan | — |
| Import/eksport | ✅ show JSON, sceny | ✅ `.cgpack`, import z edytora, kontrakt | ✅ `.cgpack` |
| Niezawodność emisji | 🟡 dobra izolacja scen, brak kopii/auth | 🟡 G1 otwarte, ale plan M5.0 i odroczone aktualizacje na wizji ➕ | n/d |
| Testy | ❌ (1 test) | ✅ ~80 plików testów + ci.js | ✅ make check, golden UI |
| Dokumentacja | README, CHANGELOG, SZYBKI_START | rozbudowana (docs/, plany M1–M7) | rozbudowana |
| i18n | PL | PL | PL/EN |

**Wnioski z porównania**
- VizRS jest **lepszy** w: prostocie wdrożenia, modelu stop pointów, funkcjach „newsowych” (autofollow, squeeze-lines, tickery,
  QR, warianty), API/TCP gotowym do Stream Decka, warstwach w stylu Viz.
- BTB jest **lepszy** w: tekście (edytor, style, pisownia, fonty), testach, pakowaniu/wymianie elementów, odroczonych aktualizacjach
  na wizji, docelowo w rundownie/wersjonowaniu.
- Oba mają ten sam cel (Viz-podobny playout do OBS) i **duplikują pracę**: tekst, squeeze i autofollow są implementowane 3×
  (VizRS `renderer.js`, BTB `scene-model.js`, btb-cgeditor `compile.rs`) z różnymi regułami.

## 4. Propozycje zmian i nowych funkcji dla VizRS

Priorytety: **P1** = przed użyciem na antenie, **P2** = duży zysk operacyjny, **P3** = rozwój. Koszt orientacyjny: S ≤ 1 dzień, M 2–4 dni, L > 4 dni.

### P1 — bezpieczeństwo i niezawodność
| # | Propozycja | Opis | Koszt |
|---|---|---|---|
| 1 | **Token API i domyślny nasłuch lokalny** | `--bind 127.0.0.1` domyślnie, `--token X` (nagłówek `Authorization` lub `?token=`; Companion obsługuje query). Wyłączyć mutujące GET bez tokenu, zawęzić CORS do własnego origin (UI jest serwowane z tego samego hosta). Uwaga: `output` i `trio` muszą dostać token (cookie/meta). | M |
| 2 | **Kopie show** | Przed każdym zapisem rotacyjny backup (`backups/show-YYYYMMDD-HHMM.json`, np. 24 godzinne + 14 dzienne), komenda/ przycisk „Przywróć”; zapis `fsync` + obsługa błędu (zwrot 500 + czerwony pasek w Trio zamiast ciszy). | S–M |
| 3 | **`rev` i optymistyczna współbieżność** | Numer rewizji show; `PUT` z nieaktualnym `rev` → 409 z komunikatem; Artist/Trio zapisują **pojedyncze** sceny/strony (już jest `PUT /api/pages/:id`), a `PUT /api/show` tylko dla importu (z potwierdzeniem). | M |
| 4 | **Walidacja schematu** | Przy `put_show`/`upsert`: typy pól, liczba stopów, długość nazw, zakres `number` (1–99999), maks. głębokość/rozmiar sceny; odrzucać zamiast zapisywać śmieci. | M |
| 5 | **Odporność serwera** | Zamiana `lock().unwrap()` na odzyskiwanie po poison; `catch_unwind` wokół handlera; pula wątków / limit połączeń (np. 256) i limit kolejki SSE (odłączanie wolnych), heartbeat SSE co 5 s; limit sumy assets i whitelist typów (png/jpg/webp/svg/mp4/webm/ttf/otf/woff2). | M |
| 6 | **Testy** | `cargo test`: silnik (take/continue/out/update/carousel), parser JSON (fuzz na LCG), routing HTTP, regresje dla powyższych luk; test E2E Playwright (jak w BTB `ci.js`) uruchamiany na binarce; golden screenshots Output dla demo show. | M–L |
| 7 | **Paczka fontów offline** | Wbudować/dołączyć kilka krojów (Oswald, Inter, Bebas…) jako assets + `@font-face`; Google Fonts tylko jako opcja. Pasek ostrzeżenia w Trio, gdy font nie załadowany. | S–M |

### P2 — narzędzia operatora
| # | Propozycja | Opis | Koszt |
|---|---|---|---|
| 8 | **Rundown w Trio** | Lista prowadzona (kolejność, notatka, stan: gotowy/na wizji/odegrany), przycisk NEXT = TAKE następnej pozycji, import CSV (numer, pola). Wzór: plan M5.3 w BTB. Playlisty już są — rundown to ich wersja „ręczna”. | M–L |
| 9 | **Hotkey per strona** | Pole `hotkey` na stronie (np. `Ctrl+1`), rejestrowanie w UI jak w BTB; konflikt-detektor. | S |
| 10 | **Preview bus i key/fill** | `output?bus=preview` (podgląd zbrojonej strony bez wejścia na PGM) oraz `?mode=key` (maska luma) dla ATEM/Tricaster. `?layers=` już jest, więc to naturalne rozszerzenie. | M |
| 11 | **Stan wyjść i healthcheck** | `/health` (outputs, wiek ostatniego eventu, wersja, rev), panel „Stan systemu” w Trio z liczbą podłączonych `output`, ostatnim heartbeatem i błędami scen zgłaszanymi przez renderer (`POST /api/report`). Wzór: M5.0 w BTB. | M |
| 12 | **Odroczona aktualizacja szablonu na wizji** | Zapis sceny z Artist, gdy strona jest on-air, nie zmienia wizji, tylko „czeka” i wchodzi po OUT (jak `splitDeferred` w BTB). Dziś zmiana sceny prawdopodobnie widoczna dopiero po następnym TAKE — warto uczynić to jawnym. | S–M |
| 13 | **Komendy zbiorcze i makra** | `TAKE 1001,1101` / `OUT main,front`, makra (sekwencja z opóźnieniami) wywoływane z TCP/HTTP; `GET /api/take/…` zwraca czas. | S–M |
| 14 | **Log audytowy** | Plik `vizrs-data/log.jsonl`: czas, źródło (IP/HTTP/TCP), komenda — przydatny po incydentach i przy wielu operatorach. | S |
| 15 | **Pasek „zmiany niezapisane / zapis nieudany”** w Trio i Artist (po #2/#3). | S |

### P3 — rozwój edycji i grafiki
| # | Propozycja | Opis | Koszt |
|---|---|---|---|
| 16 | **Edytor tekstu sformatowanego** w polach Trio | Obecnie `[[…]]` i czysty tekst. Prosta pasek B/I/kolor/rozmiar fragmentu z modelem znaczników jak RichText v1 z BTB (wspólne wektory testowe). Najbardziej zauważalna różnica dla operatora. | L |
| 17 | **Pisownia PL i Word Replace** | Słownik (sjp.pl, jak `spellbook` w BTB) + lista zamian (np. „%” → „proc.”) w polach Trio. | M |
| 18 | **Pola z regułami** | Odpowiednik `ocgInputs`: `maxChars`, `softMax` (żółty licznik), `maxLines`, jedna linia, wielkie litery, polska typografia (twarde spacje po jednoliterowych spójnikach), kolejność Tab, podpowiedź, readonly. VizRS ma już wskaźnik ściśnięcia — dopełnić go regułami. | M |
| 19 | **Powiązania/dock** | Element A przesuwa się, gdy element B jest na wizji (belka nad tickerem, bug przy grafice). Dziś rozwiązywane przez warstwy i autofollow; `dock` dawałby ogólny mechanizm (role/grupa/nazwa). | M |
| 20 | **Parenting i markery w Artist** | Rodzic–dziecko węzłów (transformacje), markery na osi czasu jako nazwane punkty (można mapować na stop pointy). Z listy propozycji btb-cgeditor (D, E). | M–L |
| 21 | **Kopiuj/wklej klucze, zoom osi, zaznaczanie ramką** | Timeline ma tylko suwak Zoom; brak graph editora. Warto dodać wielokrotne zaznaczenie i kopiowanie kluczy (B, C z btb-cgeditor). | M |
| 22 | **Biblioteka presetów animacji** | Zapis/zastosowanie zestawu kluczy między scenami (i wymiana z BTB jako JSON). | M |
| 23 | **Presety show / segmenty** | „Zestaw na segment programu” (lista stron + warstw) jednym kliknięciem — odpowiednik presetów BTB; ładuje i wywołuje grupę stron. | S–M |
| 24 | **Wiele operatorów** | Blokada edycji strony przez innego operatora, widok „kto edytuje” (po #3). | M |
| 25 | **Narzędzia wspólne z BTB** | Wspólny kontrakt elementu/sceny (konwerter VizRS ↔ BTB v3 `.json`) albo przynajmniej wspólny zestaw wektorów testowych dla squeeze/autofollow, by trzy implementacje nie rozjeżdżały się. Import `.cgpack` z BTB do VizRS jako sceny. | L |
| 26 | **Dystrybucja** | Skrypty buildów mają się (`build-*.sh/bat`); dodać CI (GitHub Actions) z buildem Linux/macOS/Windows, sumami kontrolnymi i `--version`; usługa systemd/launchd; opcjonalnie tray. | S–M |

### Rekomendowana kolejność
1. **#1, #2, #5** (bezpieczeństwo, kopie, odporność) — małe zmiany, największe ryzyko zmniejszone.
2. **#6** (testy) równolegle — bez nich kolejne funkcje zwiększają ryzyko regresji.
3. **#3, #4, #11, #12** (spójność danych i obserwowalność).
4. **#8, #9, #10, #13** (rundown, hotkeys, preview/key-fill, makra) — to, co operator odczuje w pracy.
5. **#16–#18** (tekst) — największa przewaga BTB nad VizRS; wspólny kontrakt tekstu (#25) ogranicza podwójną pracę.

## 5. Co warto przenieść w drugą stronę (z VizRS do BTB)
- **Stop pointy + CONTINUE na wizji** (BTB porzuciło WASM, a bez niego Local Stop nie gra na wizji).
- **Autofollow** i **squeeze-lines** z jednym, jasnym modelem (VizRS ma go w ~1 pliku renderera) — kandydat na wspólny moduł.
- **TCP :6100 i GET-owe API** dla Companion (BTB M5.1/M5.2) — gotowy wzorzec poleceń.
- **Warstwy jak w Viz** (back/main/front/własne) i **warianty jako przyciski**.
- Filozofia **zero zależności + jeden plik** jako wariant dystrybucji „lite”.

## 6. Ograniczenia analizy
- Testy UI były automatyczne (Playwright, headless) — nie oceniałem ergonomii ani działania w samym OBS.
- Niezmierzone: zużycie CPU renderera przy wielu tickerach, wydajność na słabym sprzęcie, zachowanie po wielogodzinnej pracy (wyciek pamięci SSE).
- BTB TRIO oceniane z dokumentacji (`README`, `ROADMAP`, `docs/funkcje.md`, `docs/gaps.md`, plan M5, `STATUS`) i listy plików gałęzi; nie uruchamiałem go.
  Ocena „część ✅” w tabeli porównawczej wynika z deklaracji dokumentacji i obecności testów.
- Instancja testowa używała osobnych katalogów danych (`scratchpad/d2`, `d3`); Twoja instancja na porcie 8088 i `vizrs-data` nie były modyfikowane
  poza kilkoma testowymi TAKE/OUT na początku sesji (stan on-air).
