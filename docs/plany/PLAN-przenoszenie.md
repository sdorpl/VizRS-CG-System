# PLAN – przenoszenie i edycja elementów grafiki (`.cgpack`, wysyłka, strony)

Stan: **P0–P4 wdrożone (2026-11-14)**, P5 odłożony (decyzje w §6). Dotyczy całego pakietu VizRS:
VizRS CG System (serwer, Trio, zamrożony `/artist`), VizRS CG Artist (praca lokalna, praca na serwerze, wysyłka).
VizRS Live i VizRS CG Playout nie czytają ani nie piszą `.cgpack` i nie zmieniają scen ani stron (sprawdzone w kodzie).

## 1. Stan obecny – kto, czym i jak przenosi

| Droga | Gdzie | Kod | Co przenosi | Tożsamość sceny | Strony |
|---|---|---|---|---|---|
| ⇩ Eksport / ⇩ Paczka | Artist (oba tryby), `/artist` | `cgpack.buildPack` | sceny + pliki (+ strony, gdy zaznaczone) | `id` zapisane w paczce | tak (z serwera albo z `show.json` danych lokalnych) |
| ⇧ Import (Artist, na serwerze) | Artist, `/artist` | `cgpack.importPack` | pliki, sceny, strony | **zawsze nowe `id`** | tak, numer albo nowy |
| ⇧ Import (Artist, lokalnie) | Artist | `cgpack.importPack` (`withPages=false`) | pliki, sceny | zawsze nowe `id` | **pominięte** (magazyn lokalny nie przyjmuje stron) |
| ⇧ Import paczki | Trio | `cgpack.importPack` | pliki, sceny, strony | zawsze nowe `id` | tak, numer albo nowy |
| ⇧ Wyślij na serwer | Artist (lokalnie) | `src/deploy.rs` (`Placer`, `send`) | pliki + sceny | **to samo `id`**, `rev` z `deploy.json`, 409 → „nadpisz” | **nie** |
| ⇩ Kopia / ⇧ Przywróć show | Trio | `PUT /api/show?force=1` | cały show bez plików | bez zmian | tak (zastępuje wszystko) |
| ＋ Utwórz stronę w Trio | Artist (na serwerze) | `POST /api/pages` | nowa strona | – | nowy numer |
| ⧉ Duplikuj stronę | Trio | `POST /api/pages` | kopia strony w tym samym show | – | nowy numer |

Reguły plików (`assetPlacer` w JS i `Placer` w Ruście) są spójne i dobrze opisane (DOKUMENTACJA §8.7): nazwa jak na serwerze,
czcionka o tej samej treści nie jest dublowana, ta sama nazwa i treść – pomijana, nic nie jest nadpisywane (`nazwa_2.ext`).

## 2. Problemy w logice (od najważniejszego)

### 2.1 Dwie niezgodne definicje „tej samej sceny”
- Import paczki zawsze tworzy **nową scenę** (nowe `id`); wysyłka z Artist **aktualizuje po `id`**.
- Skutek A – **duplikaty:** scena trafiła na serwer importem paczki (nowe `id`), potem autor wysyła poprawkę z Artist → status `new`, na serwerze powstaje druga scena o tej samej nazwie.
- Skutek B – **nie da się zaktualizować belki paczką.** Poprawiona oprawa z innego stanowiska (studio → VPS) zawsze ląduje obok starej. Usunięcie starej sceny **kaskadowo usuwa jej strony oraz pozycje playlist i rundownów** (`engine.delete`) – podmiana szablonu paczką jest więc niszcząca.
- Skutek C – nazwy: import przy zajętej nazwie dodaje scenę **pod tą samą nazwą** („dodano jako kopię” tylko w raporcie). Reguły `dock` `scene:<nazwa>` stają się niejednoznaczne.

### 2.2 Strony nie mają spójnej drogi
- Wysyłka z Artist nie przenosi stron, a import lokalny w Artist je pomija – strony z paczki **giną** na drodze „paczka → Artist lokalnie → wyślij”. Jedyna droga stron na serwer to import w Trio (albo Artist na serwerze).
- Trio **nie umie wyeksportować** wybranych stron (z ich scenami i plikami) – operator bez Artist nie przeniesie stron między serwerami ani między show tego samego serwera (poza kopią całego show).
- Przenumerowanie przy imporcie (zajęty numer → `max+1`) nie poprawia odniesień do numerów: `dock` `page:<numer>`, makra (`TAKE 1001`), CSV rundownów, skróty operatora.
- Pozycje playlist/rundownów nie jadą w paczce (świadomie), ale raport o tym milczy.

### 2.3 Błędy i kruchości w `cgpack.js` / imporcie
1. **`buildPack` zbiera pliki z wartości wszystkich podanych stron**, zanim odfiltruje strony spakowanych scen (`for (const p of pages) sceneAssets(p.values…)` przed `pages.filter(ids.has(p.scene))`). ⇩ Eksport podaje `show.pages` całego show → paczka jednej sceny dostaje grafiki stron innych scen.
2. Strona ze sceną, której zapis się nie udał albo której nie ma w paczce, jest **pomijana po cichu** (`if (!sid) continue`) – brak wiersza w raporcie.
3. Wykrywanie zajętego numeru po tekście komunikatu serwera (`/zajęty/.test(e.message)`) – zmiana treści błędu w `engine.rs` psuje import. Importer i tak zna show – może sam nadać wolne numery przed zapisem.
4. Import nie jest atomowy: pliki są wgrywane przed scenami, błąd w połowie zostawia część scen i osierocone pliki. Brak „planu” przed zapisem – użytkownik widzi skutki dopiero w raporcie.
5. `packKind` rozpakowuje całe archiwum, a `readPack` robi to drugi raz (do 256 MB dwa razy) – wystarczy jedno `readPack` z czytelnym błędem.
6. Reguły plików są zaimplementowane dwa razy (JS `assetPlacer`, Rust `Placer`) z osobnymi testami; drobna różnica: Rust `rename_assets` nie dekoduje `%xx` w `/assets/…` (dziś nieszkodliwe, bo nazwy są bezpieczne – ale to zalążek rozjazdu).

### 2.4 Edycja – stan dobry, do zachowania
Rewizje `rev` na każdym obiekcie, 409 przy rozbieżności, świadome „nadpisz” (Trio: strona; Artist: wysyłka), blokady edycji (`/api/lock`). Nowa logika przenoszenia powinna używać **tych samych** mechanizmów zamiast własnych.

## 3. Docelowe zasady (propozycja)

**Z1. Jedna tożsamość sceny: `id`.** Paczka niesie `id` sceny (już niesie). Import nie wyrzuca go z góry: gdy `id` jest wolne – scena powstaje z tym `id`; gdy zajęte – to ta sama scena (wersja) i użytkownik wybiera czynność. Wysyłka z Artist i import paczki mówią wtedy tym samym językiem.

**Z2. Najpierw plan, potem zapis** (jak dzisiejsze „Sprawdź” w wysyłce). Jedna funkcja `planImport(pk, cel)` zwraca dla każdej pozycji stan i domyślną czynność:

| Obiekt | Stan | Czynność domyślna | Do wyboru |
|---|---|---|---|
| scena | `new` (brak `id`) | dodaj z tym `id` | – |
| scena | `same` (ta sama treść) | pomiń | – |
| scena | `changed` (inne `nodes`/ustawienia) | brak – wybiera użytkownik (§6 pkt 1) | zaktualizuj (`PUT` z `rev` odczytanym przy planie – strony zostają) / dodaj jako kopię (nazwa „… (2)”) / pomiń |
| scena | `name-taken` (inne `id`, ta sama nazwa) | brak – wybiera użytkownik (§6 pkt 1) | dodaj jako kopię „… (2)” / zastąp tamtą (aktualizacja tamtego `id`) / pomiń |
| strona | `new` | dodaj z numerem z paczki | – |
| strona | `number-taken` | nowy numer (pierwszy wolny, raport stary → nowy) | pomiń |
| strona | `exists` (to samo `id`) | pomiń | nadpisz wartości |
| plik | wg `assetPlacer` | jak dziś | – |

„Ta sama treść” = porównanie sceny bez `id`/`rev`/`name` po kanonicznym JSON (po `renameAssets`).

**Z3. Jedno miejsce reguł.** `cgpack.js` zostaje jedynym źródłem reguł planu i plików dla stron WWW (Trio, Artist, `/artist`). `deploy.rs` dostaje **wspólne wektory testowe** (`source/tests/vectors/cgpack-*.json`, uruchamiane przez `node` i `cargo test`, jak wektory modelu tekstu) – nie przepisujemy wysyłki na JS (token zostaje w oknie).

**Z4. Wysyłka = import planem na zdalny serwer.** `deploy check` zwraca te same stany co `planImport` (`new/same/changed` + dziś `foreign` → `changed` z informacją „niewysłana stąd”), opcjonalnie ze stronami sceny (pkt S5).

**Z5. Strony jeżdżą z paczką także z Trio.** Trio dostaje ⇩ **Eksport paczki** zaznaczonych stron (ich sceny + pliki, `buildPack`) – przenoszenie stron między serwerami i między show tego samego serwera bez Artist.

**Z6. Odniesienia po przenumerowaniu.** Plan zgłasza (nie zmienia sam) odniesienia, które przestaną pasować: `dock` `page:N`/`scene:<nazwa>`, makra `TAKE N`, skróty. Opcja „przesuń numery stron o +K” (blok numerów dla oprawy) zamiast numerowania pojedynczo.

**Z7. Bez cichych strat.** Każda pominięta strona/scena/plik ma wiersz w raporcie; Artist lokalnie przy paczce ze stronami mówi wprost: „strony zaimportuje Trio – ta sama paczka”.

## 4. Etapy

| Etap | Zakres | Repo | Ryzyko |
|---|---|---|---|
| **P0** | Poprawki bez zmiany zasad: 2.3.1 (`buildPack` filtruje strony przed zbieraniem plików), 2.3.2 (raport pominiętych stron), 2.3.3 (wolne numery liczone przez importer, regex tylko jako zapas), 2.3.5 (jedno rozpakowanie). Testy w `tests/cgpack.mjs`. | System → sync do Artist | małe |
| **P1** | `planImport` + okno planu w Trio i Artist (wspólny moduł UI w `cgpack.js` albo `pack-ui.js`); import z zachowaniem `id` i aktualizacją po `rev`; nazwy kopii „… (2)”. `validate.rs` bez zmian (pola już są). | System → sync | średnie – zmiana zachowania importu (DOKUMENTACJA §8.7, §9, INSTRUKCJA) |
| **P2** | ⇩ Eksport paczki w Trio (zaznaczone strony / bieżąca strona), także w układzie telefonu (szuflada). E2E w `tests/e2e/pack.js`. | System | małe |
| **P3** | Wektory `cgpack-*.json` wspólne dla `cgpack.mjs` i `deploy.rs`; stany `deploy check` = stany planu; poprawka `%xx` w `rename_assets`. | System + Artist | małe |
| **P4** | Odniesienia (Z6): wykrywanie w planie, opcja przesunięcia numerów. | System | średnie |
| **P5** (opcja) | Wysyłka stron sceny z Artist (gdy strony są w danych lokalnych) albo `POST /api/import` – atomowy import w jednym zamku silnika (wymaga tego samego w `store.rs` Artist). | System + Artist | duże – tylko gdy P1 okaże się za mało |

`_packVersion` zostaje `1`: format się nie zmienia (`id` już jest w paczce), zmienia się interpretacja przy imporcie. Starsze programy czytają nowe paczki jak dziś.

## 5. Testy

- `tests/cgpack.mjs`: P0 (pliki tylko stron spakowanych scen, raport pominiętych, wolne numery), P1 (stany planu: new/same/changed/name-taken, aktualizacja zachowuje strony, kopia dostaje „(2)”).
- `tests/e2e/pack.js`: eksport z Trio → import na drugim serwerze → ponowny import poprawionej paczki aktualizuje scenę, strony i rundown zostają.
- Artist: `deploy.rs` czyta wektory P3; E2E Artist: paczka → lokalnie → wyślij → brak duplikatu na serwerze, na który wcześniej zaimportowano tę paczkę.

## 6. Decyzje (2026-11-14)

1. **Scena `changed` (i `name-taken`): zawsze decyduje użytkownik.** Bez czynności domyślnej – „Importuj” działa po wyborze dla każdej takiej sceny; przyciski „wszystkie konflikty” ustawiają wszystkie naraz. Bez okna planu (zamrożony `/artist`) – kopia, jak dotąd.
2. **Kopia dostaje „ (2)”, „ (3)”…** – nazwy scen w show się nie powtarzają.
3. **Trio importuje i eksportuje paczki (strony)** – P2.
4. **Strony w Artist lokalnie – zostaje „Artist tworzy tylko sceny”.** Wyjaśnienie: praca lokalna Artist to magazyn szablonów (`store.rs` przyjmuje tylko sceny), a strony należą do Trio – mają numery, wartości, miejsce w playlistach i rundownach konkretnego show na konkretnym serwerze. Gdyby magazyn lokalny trzymał strony z paczki, musiałby też pilnować ich numerów wobec serwera, na który kiedyś trafią, i wysyłka z Artist stałaby się drugim importem stron (z własnymi konfliktami numerów). Zamiast tego: paczkę ze stronami importuje Trio na docelowym serwerze, a Artist lokalnie mówi to wprost w oknie planu i w raporcie. Działa to bez strat, bo sceny i strony łączy `id` – sceny wysłane z Artist i strony zaimportowane w Trio z tej samej paczki wskazują te same sceny.
5. **Atomowy import po stronie serwera – odłożony (P5).** Wyjaśnienie: dziś import to seria osobnych zapisów (pliki, potem sceny, potem strony); błąd w połowie zostawia to, co już zapisano. Atomowy `POST /api/import` robiłby wszystko pod jedną blokadą silnika – wszystko albo nic – ale wymaga drugiej implementacji reguł planu w Ruście (serwer `engine.rs` i magazyn Artist `store.rs`). Plan przed zapisem usuwa większość przyczyn częściowego importu (konflikty nazw, numerów i wersji są rozstrzygnięte zawczasu, a rewizje `rev` chronią przed zmianą w międzyczasie), a raport mówi dokładnie, co się nie zapisało. Wracamy do P5, gdy w praktyce pojawią się częściowe importy.

## 7. Wykonanie (2026-11-14)

| Etap | Stan | Gdzie |
|---|---|---|
| P0 | wdrożony | `buildPack` – pliki tylko ze stron pakowanych scen; raport pominiętych stron; wolne numery liczy import (treść błędu serwera tylko jako zapas); jedno rozpakowanie w Trio i Artist |
| P1 | wdrożony | `planImport`, `planPages`, `importPack(…, { plan })`; okno `pack-ui.js` (`planDialog`) w Trio i VizRS CG Artist; `id` z paczki zachowane, gdy wolne (bez znanego show import sam czyta `/api/show`) |
| P2 | wdrożony | Trio **⇩ Eksport paczki** (`pickPages`), także w szufladzie na telefonie; e2e `tests/e2e/pack.js` |
| P3 | wdrożony | wektory `source/tests/vectors/cgpack-placer.json` (node + `cargo test` w Artist, kopia przez `tools/sync-cg.sh`); stany `deploy check` (`same` po treści, `name-taken`); `%xx` w `rename_assets` |
| P4 | wdrożony | odniesienia dock `page:N` / `scene:<nazwa>` w planie i poprawka przy imporcie; przesunięcie numerów o stałą |
| P5 | odłożony | §6 pkt 5 |
