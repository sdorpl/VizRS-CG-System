# PLAN ROZBUDOWY VizRS (v1.0 → v2.0)

Status: **wdrożony w całości** (2026-10-19, wydanie 2.0.0) – szczegóły i odstępstwa w `docs/STATUS.md` i `docs/LUKI.md`. Podstawa: [RAPORT-analiza-VizRS-vs-BTB.md](RAPORT-analiza-VizRS-vs-BTB.md) (numery #1–#26 odnoszą się do jego propozycji).
Wzorce organizacyjne: plany M1–M7 w `BTB-CG-System-alpha` (gałąź `TRIO`: `docs/plans/`, `docs/gaps.md`) i zasady `btb-cgeditor` (`CLAUDE.md`, `docs/LUKI.md`, `PROPOZYCJE-FUNKCJI.md`).

## 1. Cel i kryterium ukończenia

VizRS ma pozostać **wiernym, lekkim odpowiednikiem Viz Artist (projekt scen) + Viz Trio (playout)** dla OBS, a przy tym stać się systemem,
któremu można zaufać na antenie: realizator przeprowadza zmianę prowadzoną z rundownu, sterując z Trio, Stream Decka lub Companion,
a awaria sceny, klienta, sieci albo błędny zapis **nie zatrzymuje wizji i jest widoczna** (nic nie ginie po cichu).

Kryterium v2.0: (a) wszystkie pozycje P1 z raportu zamknięte i pokryte testami; (b) pełna zmiana z rundownu bez myszy;
(c) show z jednego komputera uruchamia się na drugim bez zmian i bez internetu; (d) `cargo test` + E2E zielone w CI na Linux/macOS/Windows.

## 2. Założenia nienaruszalne (wierność Viz Artist / Viz Trio)

Każda zmiana jest oceniana wobec poniższej listy; kolizja = zmiana odrzucona albo zrobiona jako opcja, nie zmiana domyślna.

| # | Założenie z Vizrt | Jak pilnujemy w VizRS |
|---|---|---|
| Z1 | **Rozdział ról**: Artist projektuje szablony (sceny), Trio tylko je wypełnia danymi i emituje. Operator nie zmienia układu sceny. | Trio nie dostaje edycji sceny; nowe funkcje projektowe trafiają do Artist, a Trio widzi je jako pola/warianty. |
| Z2 | **Scena = drzewo kontenerów** z transformacją, właściwościami i hierarchią; render „od wierzchu”. | Parenting (F3.2) rozszerza drzewo, nie zastępuje go; kolejność i dziedziczenie jak w Viz. |
| Z3 | **Director / keyframe'y / stop pointy**: TAKE → 1. stop, CONTINUE → kolejny, OUT → od ostatniego stopu do końca; bez stopów TAKE gra całość, OUT ją „wycina”. | Silnik zostaje źródłem prawdy o stopach; markery (F3.2) mapowane na stop pointy; żadnych zmian semantyki TAKE/CONTINUE/OUT. |
| Z4 | **Control Object / Control Text / Image…**: pole kontrolne na dowolnej właściwości; Trio generuje formularz z pól sceny. | Reguły pól (maxChars, maxLines, tab order…) to właściwości pola kontrolnego w Artist, nie ustawienia Trio. |
| Z5 | **Szablon → strona (page)**: strona = dane + numer; numery stron do wywołania z klawiatury; edycja z podglądem. | Numeracja i wczytywanie numerem bez zmian; rundown jest *listą stron*, nie nowym bytem. |
| Z6 | **Warstwy Viz (back/main/front + własne)**: TAKE na tej samej warstwie zastępuje poprzednią scenę, OUT zdejmuje warstwę. | Bez zmian; dock/powiązania (F3.1) działają *między* warstwami, nie zmieniają ich semantyki. |
| Z7 | **Update**: podmiana danych na antenie bez ponownej animacji. | Zachowane; odroczona aktualizacja *szablonu* (F2.4) dotyczy sceny, nie danych strony. |
| Z8 | **Playlista/Carousel** z czasami i pętlą po stronie serwera; **Show** jako zestaw stron. | Rundown (F2.1) i presety segmentów (F2.6) budowane na Show i Playlist, bez nowej architektury. |
| Z9 | **Preview/Program**: osobny kanał podglądu i antena (Viz Preview vs Program). | Preview bus (F2.3) = osobne wyjście, zasada „uzbrój w preview, wyemituj na program”. |
| Z10 | **Sterowanie zewnętrzne tekstem** (port 6100: `TAKE`, `CONTINUE`, `OUT`, `UPDATE`…). | Rozszerzamy zgodnie (makra, listy), składnia istniejących komend nie zmienia znaczenia. |
| Z11 | **Jeden zestaw danych, wiele kopii klienta**: Trio i Artist to klienty serwera (Engine). | Serwer zostaje jedynym właścicielem stanu on-air; klienty proponują, serwer potwierdza (D3). |
| Z12 | Cechy 3D, kamery, materiały, shadery Vizrt są **poza zakresem** (VizRS = 2D dla OBS). | Brak prób 3D; zakres jak w README („Model (jak w Vizrt)”). |

## 3. Decyzje architektoniczne

- **D1 Zero zależności i jeden plik wykonywalny zostają** (własny HTTP/SSE/JSON). Nowe funkcje serwera piszemy w tym samym duchu; zależność dopuszczalna tylko za zgodą i z uzasadnieniem (np. kompresja kopii).
- **D2 Zdarzenia i komendy przez jeden rejestr** (`engine::command`): HTTP, TCP, UI, makra i harmonogram wołają ten sam kod (wzór: `commands.js` w BTB M5.1, „everything is a command” z EffectCraft). Brak równoległych implementacji.
- **D3 Serwer jest właścicielem stanu on-air i danych show.** Klient wysyła *małe* zmiany z `rev`; serwer potwierdza (`ack`) albo odrzuca (`rejected{reason}`). Pełny `PUT /api/show` tylko dla importu.
- **D4 Bezpieczne domyślne**: nasłuch lokalny, token, brak mutujących GET bez tokenu, wąski CORS (wzór BTB M5 D4). Wymaga opcji `--bind 0.0.0.0` do pracy sieciowej.
- **D5 Wersjonowanie append-only** show i scen (kopie rotacyjne, historia) — spójne z odroczoną aktualizacją szablonów.
- **D6 Jedna reguła tekstu na całą rodzinę**: model RichText i reguły dopasowania zgodne z BTB (`text-model.js`, `scene-model.js`), weryfikowane **wspólnymi wektorami testowymi** (jak `tests/text/*.json` w btb-cgeditor) — bez kopiowania kodu na ślepo.
- **D7 Niezawodność przed funkcjami** (zasada BTB `gaps.md` G1→G2→G3→nowe funkcje): etapy F0–F1 nie mogą zostać wyprzedzone.
- **D8 Nie wywalaj serwera**: nowy kod bez `unwrap()`/`expect()`/panik na ścieżkach z danych; dane z sieci i plików traktowane jako wrogie (limity, NaN/inf, głębokość); każda poprawka awarii ma test regresji.
- **D9 Czas** w sekundach (f64/f32) niezależnie od fps; stan on-air odtwarzalny po odświeżeniu OBS (jak dziś).

## 4. Etapy

Każdy krok = osobny commit z testem i aktualizacją dokumentacji (README, CHANGELOG, SZYBKI_START). Bramka przed commitem (nowa, F0): `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, E2E.

### F0 — Fundament jakości (równolegle z F1)
| Krok | Zakres | Pozycja raportu | Koszt |
|---|---|---|---|
| F0.1 | Układ testów: `cargo test` dla silnika (take/continue/out/update/carousel, warstwy, numery stron), routingu HTTP i parsera JSON (testy losowe na własnym LCG, jak w btb-cgeditor). | #6 | M |
| F0.2 | E2E w Playwright na binarce (wzór `ci.js`/`test-dock-e2e.js` z BTB): Trio F1–F4, Artist zapis sceny, Output po TAKE/UPDATE/OUT, scena błędna. | #6 | M |
| F0.3 | Golden screenshoty Output dla zestawów INFO24 i R7 (`tests/golden/`), `--update` odświeża; tylko jako test lokalny, nie blokuje CI na różnicach fontów. | #6 | S–M |
| F0.4 | CI (GitHub Actions): build + testy Linux/macOS/Windows, `--version` z hashem commita, sumy kontrolne w wydaniu. | #26 | S–M |
| F0.5 | `docs/LUKI.md` i `docs/STATUS.md` jak w BTB; reguła „zaktualizuj STATUS w tym samym commicie”. | proces | S |

### F1 — Niezawodność i bezpieczeństwo (odpowiednik M5.0)
| Krok | Zakres | Pozycja | Koszt |
|---|---|---|---|
| F1.1 | Token + `--bind` (domyślnie 127.0.0.1) + wąski CORS; UI (`trio`, `artist`, `output`) dostaje token przez cookie/meta; mutujące GET wymagają tokenu (`?token=` dla Companion). | #1 | M |
| F1.2 | Kopie show: rotacja (24 godzinne + 14 dzienne w `backups/`), `fsync`, błąd zapisu zwracany jako 500 i widoczny w UI; komenda/przycisk „Przywróć”. | #2 | S–M |
| F1.3 | `rev` show i stron/scen; zapis tylko pojedynczych obiektów w UI (`PUT /api/pages/:id`, `/api/scenes/:id`); nieaktualny `rev` → 409 z komunikatem; `PUT /api/show` tylko jako import z potwierdzeniem. | #3 | M |
| F1.4 | Walidacja schematu show/scen/stron (typy, zakres numeru 1–99999, liczba stopów, głębokość/rozmiar sceny, długości pól); odrzucenie zamiast zapisu. | #4 | M |
| F1.5 | Odporność serwera: odzyskiwanie po zatrutym mutexie, `catch_unwind` wokół handlera, limit połączeń i wątków, ograniczona kolejka SSE (odłączanie wolnych), heartbeat SSE, whitelist typów i limit sumy assets. | #5 | M |
| F1.6 | Izolacja błędów sceny w rendererze: `try/catch` per węzeł i per warstwa, raport `POST /api/report` → lista błędów w Trio („Stan systemu”). | #11 | S–M |
| F1.7 | Reconnect wyjścia: `output` trzyma ostatni stan, po powrocie SSE robi diff bez migotania; wskaźnik w `?status=1`. | G1.3 w BTB | S |
| F1.8 | Paczka fontów offline (Oswald, Inter, Bebas Neue, …) z manifestem; Google Fonts tylko jako opcja; ostrzeżenie w Trio, gdy font nie załadowany. | #7 | S–M |
| F1.9 | `/health` (wersja, rev, outputs, wiek ostatniego zdarzenia, liczba błędów scen). | #11 | S |

### F2 — Narzędzia operatora (Trio)
| Krok | Zakres | Pozycja | Koszt |
|---|---|---|---|
| F2.1 | **Rundown** jako lista stron w Show: kolejność (drag&drop), notatka, stan (gotowy/na wizji/odegrany), `NEXT` (TAKE następnej + zbrojenie kolejnej), import/eksport CSV. Spójnie ze strukturą stron i numerów (Z5, Z8). | #8 | M–L |
| F2.2 | Hotkey per strona (`Ctrl+1`…), rejestrowanie w UI, wykrywanie konfliktów; zapis w stronie. | #9 | S |
| F2.3 | **Preview bus** (`output?bus=preview`, uzbrajanie strony bez wejścia na PGM) i **key/fill** (`?mode=key`). | #10 | M |
| F2.4 | **Odroczona aktualizacja szablonu na wizji**: zapis sceny z Artist, gdy strona jest on-air, wchodzi po OUT; wskaźnik „czeka” z przyciskiem „Wdróż teraz” (wzór: `splitDeferred`/`applyPending` w BTB). | #12 | S–M |
| F2.5 | Makra i komendy zbiorcze: `TAKE 1001,1101`, `OUT main,front`, makro z opóźnieniami; składnia istniejących komend bez zmian (Z10). | #13 | S–M |
| F2.6 | Presety segmentów (Show-section): zestaw stron + warstw wywoływany jednym kliknięciem. | #23 | S–M |
| F2.7 | Log audytowy `log.jsonl` (czas, źródło, komenda) i panel „Stan systemu”. | #14 | S |
| F2.8 | Pasek „zmiany niezapisane / zapis nieudany” w Trio i Artist (po F1.2–F1.3). | #15 | S |
| F2.9 | Integracje: gotowy profil **Bitfocus Companion** (przyciski TAKE/OUT/NEXT/CONTINUE + zmienne „na wizji”), most **OSC**, webhook/WS zdarzeń `onAir`/`offAir`/`error`. | M5.2 | M |

### F3 — Pola, tekst i projekt (Artist ↔ Trio)
| Krok | Zakres | Pozycja | Koszt |
|---|---|---|---|
| F3.1 | **Reguły pól kontrolnych** (Control Text): `maxChars`, `softMax` (żółty licznik), `maxLines`, jedna linia, wielkie litery, polska typografia (twarde spacje), kolejność Tab, podpowiedź, readonly; edytowane w Artist, egzekwowane w Trio. Wzór: `ocgInputs[]` + wektory `operator-fields.json`. | #18 | M |
| F3.2 | **Tekst sformatowany**: pasek B/I/U/kolor/rozmiar fragmentu (RichText v1) obok `[[wyróżnienia]]`; wspólny model i wektory testowe z BTB; wsteczna zgodność ze zwykłym tekstem. | #16 | L |
| F3.3 | **Pisownia PL i Word Replace** (słownik, lista zamian per stanowisko), włączane w polach Trio; wymiana słownika bez zmian w show. | #17 | M |
| F3.4 | **Powiązania/dock** między warstwami (rola/grupa/nazwa): np. belka przesuwa się, gdy ticker jest na wizji; ustawiane w Artist, wykonywane przez serwer/renderer; operator może nadpisać. | #19 | M |
| F3.5 | **Parenting węzłów i markery** na osi czasu Artist (marker ↔ stop point); kopiuj/wklej klucze, zaznaczanie ramką, zoom osi. | #20, #21 | M–L |
| F3.6 | **Biblioteka presetów animacji** (zapis/stosowanie zestawu kluczy między scenami, JSON wymienny z BTB). | #22 | M |
| F3.7 | Edycja w podglądzie wg Trio: Tab/Shift+Tab między polami, klik w pole na podglądzie, zapis wielu pól naraz. | z BTB v0.2.x | M |

### F4 — Współpraca z ekosystemem BTB
| Krok | Zakres | Pozycja | Koszt |
|---|---|---|---|
| F4.1 | Wspólny zestaw **wektorów testowych** (squeeze, autofollow, dopasowanie tekstu, RichText) używany w VizRS, BTB i btb-cgeditor; test rozjazdu w CI. Cel: koniec dryfu trzech implementacji. | #25 | M |
| F4.2 | Konwerter **BTB `.json` v3 / `.cgpack` → scena VizRS** (jednokierunkowy, z raportem „co się nie przeniosło”). BTB CG jest bazą pomysłów – grafik nie przenosimy w drugą stronę. | #25 | L |
| F4.3 | **Wiele operatorów**: blokada edycji strony/sceny, widok „kto edytuje” (wymaga F1.3). | #24 | M |
| F4.4 | Dystrybucja: usługa systemd/launchd, katalog danych per użytkownik, instrukcja wdrożenia i „runbook awarii” (co robić, gdy output milczy), checklista prób w OBS/vMix. | #26 | S–M |

## 5. Kolejność i zależności

```
F0 (testy, CI) ──┬─► F1.1–F1.5 ─► F1.6–F1.9 ─► F2.1–F2.4 ─► F2.5–F2.9
                 │                    │
                 │                    └──────► F4.3 (po F1.3)
                 └─► F3.1 ─► F3.3 ─► F3.2 ─► F3.7
F3.4, F3.5, F3.6 — niezależne od F2 (Artist), po F0
F4.1 — po F3.1/F3.2 (wektory muszą istnieć), F4.2 — na końcu
```

Proponowane wydania:
- **v1.1 „Bezpieczny”**: F0.1–F0.4, F1.1, F1.2, F1.5 (zero zmian funkcjonalnych dla operatora).
- **v1.2 „Spójny”**: F1.3, F1.4, F1.6–F1.9, F2.4, F2.7, F2.8.
- **v1.3 „Operator”**: F2.1–F2.3, F2.5, F2.6, F2.9.
- **v1.4 „Tekst”**: F3.1–F3.3, F3.7.
- **v2.0 „Studio”**: F3.4–F3.6, F4.1–F4.4.

## 6. Ryzyka

| Ryzyko | Skutek | Środek zaradczy |
|---|---|---|
| Token/CORS psuje integracje (Companion, własne skrypty) | utrata sterowania | `--token` opcjonalny przy `--bind 127.0.0.1`; dokumentacja `?token=`; okres przejściowy z ostrzeżeniem w logu |
| `rev` i zapis per obiekt zmieniają zachowanie Artist/Trio | niezgodność klientów | `PUT /api/show` działa dalej (import), nowe ścieżki opcjonalne w 1.2; testy obu ścieżek |
| Rozjazd reguł tekstu z BTB (3 implementacje) | inne łamanie linii na antenie niż w edytorze | F4.1: wspólne wektory, test w CI; zmiana reguły = zmiana we wszystkich miejscach |
| Rundown/harmonogram wchodzi na wizję w złym momencie | błąd na antenie | brak automatu w F2.1 (tylko ręczny NEXT); ewentualny automat jako opcja z widocznym „AUTO” i „STOP automatu” |
| Zakres rośnie (tekst sformatowany, pisownia) | opóźnienia wydań | etapy niezależne; v1.1–v1.3 dają wartość bez F3; F3.2 po F3.1 |
| Zachowanie wierności Viz przy nowych funkcjach (dock, parenting) | operator dostaje „nie-Viz-owe” zachowanie | każde PR sprawdzane wobec tabeli Z1–Z12; domyślnie wyłączone, jeśli zmienia semantykę |
| Fonty/kopie rosną w repo | duże wydania | fonty jako osobny plik/asset; kopie w katalogu danych, nie w repo; limity retencji |
| Regresje po zmianach silnika | awaria na wizji | F0 przed F1; golden screenshoty, E2E; zielone CI jako warunek wydania |

## 7. Proces pracy (wzór BTB/btb-cgeditor)

- `docs/STATUS.md` (bieżący etap, następne zadanie, blokery) i `docs/LUKI.md` (znane braki, uszeregowane G1→G2→G3→funkcje) — aktualizowane w tym samym commicie co zmiana stanu.
- Jeden commit = jedna spójna zmiana, po polsku, tylko zielone stany; `git push` na prośbę.
- Komentarze i identyfikatory w kodzie zgodnie z otoczeniem; teksty UI po polsku, z miejscem na tłumaczenie (klucze w jednym pliku `i18n`, w przyszłości PL/EN).
- Wzorce wizualne i elementy demonstracyjne (INFO24, R7) pozostają referencją do testów; każda zmiana renderera przechodzi przez golden screenshoty.
- Zmiany reguł wspólnych z BTB (tekst, squeeze, autofollow) dokumentowane w obu repo w jednym czasie.
