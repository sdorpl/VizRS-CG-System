# PLAN: VizRS Artist jako osobna aplikacja natywna w Ruście (osobne repo)

> Stan (2026-11-07): **propozycja (A0–A9)**, czeka na decyzje z §9. Opis bieżącego działania Artist zostaje
> w [DOKUMENTACJA](../DOKUMENTACJA.md) do czasu przeniesienia; ten plan opisuje tylko kierunek i etapy.

## 1. Cel

Artist (projektowanie scen) staje się osobnym programem **VizRS Artist** w osobnym repozytorium,
napisanym w Ruście w całości tam, gdzie to rozsądne: okno, panele, drzewo, oś czasu, właściwości i **kanwa
rysowana natywnie**. Serwer `vizrs` + Trio + `/output` zostają w tym repo (VizRS CG System) – to odpowiednik
podziału Viz Artist / Viz Trio.

Co zyskujemy:
- niezależne wydania Artist i serwera (Artist nie blokuje wydania anteny i odwrotnie);
- szybki, „desktopowy” edytor (bez WebView, bez `include_str!`, bez mostu `native.js`);
- **drugi realny użytkownik `vizrs-render`** – kanwa Artist wymusza pełną zgodność renderera natywnego,
  co od razu pomaga playoutowi kanału (PLAN-native N3/N4);
- praca bez serwera: otwieranie i zapis `.show.json` / `.cgpack` z dysku.

Czego **nie** robimy: Trio, `/output` i telefon zostają stronami (antena to Chromium w OBS/vMix – PLAN-natywne-i-serwer §3).

## 2. Punkt wyjścia (stan 2.0.19)

| Element | Dziś | Rozmiar |
|---|---|---|
| UI Artist | `web/artist.html` (+ `native.js`, `ui.css`) | ~2,3 tys. wierszy JS/HTML |
| logika współdzielona z Trio/Output | `renderer.js`, `motion.js`, `text-model.js`, `richtext.js`, `ticker-*.js`, `field-rules.js`, `color-picker.js` | ~4 tys. wierszy |
| import BTB, paczki | `btb-io.js`, `cgpack.js` | ~700 wierszy |
| renderer natywny | `source/render/` (`vizrs-render` 0.3, tiny-skia + rustybuzz) | ~7,3 tys. wierszy; N1, N2 zrobione, **N3 (ruch BTB) i N4 (WebP, efekty) czekają** |
| okno natywne | `source/shell/` (tao + wry + muda + rfd) – WebView z Artist/Trio | – |
| API używane przez Artist | `GET/PUT /api/show` (z `rev`, 409), `/api/assets`, `/assets/*`, `/api/lock|unlock/scenes/*`, `/events` (SSE), `/fonts/*`, `/api/backups` | – |

Wniosek: najdroższa część (rysowanie sceny w Ruście) **już istnieje** w `vizrs-render`. Brakuje UI edytora
oraz portu logiki edycyjnej (pola, warianty, import BTB, paczki).

## 3. Wybór techniki

| Wariant | Okno/UI | Kanwa | Ocena |
|---|---|---|---|
| **A. egui + eframe (wgpu)** | natychmiastowe UI, gotowe drzewa, przeciąganie, skróty; wzorzec w `btb-cgeditor` | bufor RGBA z `vizrs-render` jako tekstura, nakładki (uchwyty, ramki, prowadnice) rysowane przez egui | **rekomendowany**: najkrótsza droga do edytora z osią czasu, dojrzały, Linux + Windows |
| B. Slint | deklaratywne UI, ładniejsze kontrolki | to samo (obraz) | dobry, ale słabszy przy niestandardowych widżetach (oś czasu, drzewo z DnD); licencja GPL/komercyjna do sprawdzenia |
| C. GTK 4 (gtk4-rs) | natywny wygląd Linux, pisownia (libspelling) | `gtk::Picture` z bufora | najlepszy na Linuksie, słaby na Windows (priorytet: Linux, potem Windows) |
| D. iced | Elm-owy | obraz/`canvas` | mniej gotowych widżetów edytorskich |

Rekomendacja: **egui/eframe**, kanwa z `vizrs-render`. Pisownia polska (dziś z WebView) – osobny krok A7
(słownik hunspell przez crate `spellbook` albo pominięcie w polach scen, które i tak są szablonami).

## 4. Architektura docelowa

```
repo „VizRS Artist”                                  repo „VizRS CG System”
┌───────────────────────────────────────────┐        ┌──────────────────────────────┐
│ vizrs-artist (bin, eframe)                │  HTTP  │ vizrs (serwer, zero zależn.) │
│  ├─ ui/     drzewo, oś czasu, właściwości │◀──────▶│  /api/show rev+409, assets,  │
│  ├─ canvas  tekstura + uchwyty            │  SSE   │  locks, /events, /fonts      │
│  ├─ doc     cofanie, schowek, kopia robocza│        │  validate.rs (strażnik)      │
│  └─ io      serwer | plik .show/.cgpack   │        │ Trio, /output (renderer.js)  │
│        │                                  │        │ source/render → vizrs-render │
│        ▼                                  │        └──────────────┬───────────────┘
│ vizrs-scene (lib, nowa)  ◀── git dep ─────┼───────────────────────┘
│  model show, PROPS/TYPES, trackValue,     │   vizrs-render (lib) – rysowanie klatki
│  krzywe, pola/warianty, text-model, motion│
│ vizrs-btb (lib) import BTB, .cgpack       │
└───────────────────────────────────────────┘
```

Zasady:
1. **Serwer zostaje strażnikiem.** Artist zapisuje przez `PUT /api/show` – `validate.rs` dalej odrzuca złe dane.
   Lokalny zapis do pliku też przechodzi walidację (ta sama reguła w `vizrs-scene`, zgodność testami wektorowymi).
2. **Jedna semantyka, dwóch wykonawców** (jak dziś `renderer.js` ↔ `vizrs-render`): kanwa Artist = `vizrs-render`,
   antena = `renderer.js`. Zgodność pilnuje `tools/native-compare.js`, rozszerzony o **wszystkie** sceny paczek
   z `examples/`. To zmienia zasadę „Jeden renderer” w CLAUDE.md – wymaga decyzji (§9 pkt 1).
3. **Rdzeń `vizrs` bez zmian zależności.** Nowe crate'y żyją w repo Artist albo w `source/render/`.
4. **Czego kanwa nie umie, mówi wprost:** węzeł nieobsługiwany przez `vizrs-render` dostaje znacznik
   „podgląd przybliżony” + przycisk „Podgląd w Chromium” (otwiera `/output?bus=preview`). Nigdy cichego zgadywania.
5. **Format show jest kontraktem.** Wersja schematu w show; Artist odmawia zapisu show nowszego niż zna.

## 5. Co przepisać na Rust, a co zostawić

| Część | Decyzja | Uzasadnienie |
|---|---|---|
| UI edytora (drzewo, oś czasu, właściwości, zakładki, szukanie, presety, dock, warianty) | **Rust** (egui) | sedno projektu |
| Kanwa i odtwarzanie (stop pointy, scrub, pętle) | **Rust** – `vizrs-render` | już istnieje; dokończyć N3/N4 |
| Model sceny, `PROPS`/`TYPES`, krzywe, `trackValue`, pola, `showIf` | **Rust** – `vizrs-scene` (wydzielone z `source/render/src/model.rs`, `scene.rs`) | wspólne dla Artist i renderera |
| `text-model.js` (reguły pól, typografia PL, Word Replace) | **Rust** – port + te same wektory `tests/vectors/*.json` | model wspólny z BTB; wektory gwarantują identyczność |
| `richtext.js`, ticker-model | **Rust** – już częściowo w `render/src/richtext.rs`, `ticker.rs` | przenieść do `vizrs-scene` |
| `motion.js` (ruch BTB) | **Rust** – port (to jest PLAN-native N3) | potrzebny i kanwie, i kanałowi |
| `btb-io.js`, `cgpack.js` (import BTB, ZIP) | **Rust** – `vizrs-btb` (ZIP: crate `zip` albo własny „stored”+deflate) | tylko BTB → VizRS, bez eksportu |
| `color-picker.js`, `ticker-items.js` | **Rust** (widżety egui) | UI |
| Trio, `/output`, telefon, `renderer.js` | **zostają JS** w VizRS CG System | antena = Chromium |
| `web/artist.html` | zamrożony po A8, usunięty po A9 (decyzja §9 pkt 3) | – |
| `source/shell/` | zostaje dla Trio; menu Artist z niego znika | – |

## 6. Etapy

| Etap | Zakres | Gotowe gdy | Wielkość |
|---|---|---|---|
| **A0** | Decyzje §9; prototyp: okno eframe, wczytanie show z serwera, scena z `vizrs-render` jako tekstura 60 kl./s przy 1920×1080 na Linuksie i Windows | pomiar: klatka < 8 ms CPU na typowej belce; kanwa skaluje się do okna | S |
| **A1** | Wydzielenie `vizrs-scene` z `source/render/` (model, krzywe, `trackValue`, pola, richtext, ticker) w tym repo; `vizrs-render` od niego zależy; testy wektorowe `text-model` w Ruście | `cargo test` w obu; `native-compare.js` bez zmian wyników | M |
| **A2** | Nowe repo **VizRS Artist**: workspace, CI (Linux, Windows), zależność git od `vizrs-scene`/`vizrs-render` przypięta tagiem; `CLAUDE.md`, `docs/STATUS.md` | zielony CI, pusta aplikacja startuje | S |
| **A3** | Warstwa danych: klient HTTP (show z `rev`, 409 → okno scalania, assets, blokady scen, SSE `/events`, token), tryb pliku (`.show.json`), cofanie/ponów, kopia robocza, walidacja lokalna | edycja jednej właściwości i zapis zgodny z web Artist (porównanie JSON) | M |
| **A4** | UI podstawowe: lista scen, drzewo węzłów (DnD, kopiuj/wklej), panel Właściwości (zakładki, grupy, tryb podstawowy – jak PLAN-panel-wlasciwosci), kanwa z zaznaczaniem, przesuwaniem, skalą, prowadnicami | da się zbudować belkę INFO24 od zera bez przeglądarki | L |
| **A5** | Oś czasu: klucze `{t,v,e}`, krzywe, stop pointy, markery, zoom, scrub; presety animacji; jeden model ruchu (PLAN-jeden-ruch – klipy BTB na osi) | sceny z `examples/` odtwarzają się identycznie jak w web Artist | L |
| **A6** | Ruch BTB i braki renderera: port `motion.js` (= PLAN-native N3), WebP, efekty (N4); znacznik „podgląd przybliżony” dla reszty | `native-compare.js`: wszystkie sceny paczek < 2% różnic, ruch MSE < 5 | L |
| **A7** | Pola i warianty (`fields`, `fieldMeta`, `showIf`, reguły pól), ticker (tryby, presety, edytor wiadomości), kolory, fonty (wbudowane + z serwera), pisownia | wszystkie typy pól z DOKUMENTACJI §4 edytowalne | M |
| **A8** | Import BTB (`.json` v3, `.cgpack`) i paczki VizRS (zapis/odczyt); deploy sceny do serwera; skojarzenia plików, jedna instancja, pamięć okna (przeniesione z `source/shell/`) | testy na `tests/fixtures/btb/` dają to samo show co `btb-io.js` | M |
| **A9** | Przełączenie: instalatory (AppImage/.deb, Windows), dokumentacja w nowym repo, w tym repo `/artist` → strona „Artist jest osobną aplikacją” albo zostaje jako zapas (§9 pkt 3); usunięcie testów `artist.js` | tydzień pracy projektowej tylko w natywnym Artist bez powrotu do web | S |

Kolejność A1 → A6 jest celowa: **najpierw zgodność renderera**, bo bez niej natywna kanwa nie jest WYSIWYG
dla anteny, a cały plan traci sens.

## 7. Testy i zgodność

- `tests/vectors/*.json` – wspólne z BTB i z JS; ten sam zestaw uruchamia `cargo test` w `vizrs-scene`.
- `tools/native-compare.js` rozszerzony o paczki `examples/*.cgpack` i klatki pośrednie ruchu – bramka każdej zmiany
  w `renderer.js` **i** w `vizrs-scene`/`vizrs-render`.
- Test zgodności zapisu: ta sama sekwencja edycji w web Artist (Playwright) i natywnym (tryb skryptowy `--script`)
  → identyczny `show.json` (do A9).
- E2E natywnego UI: egui_kittest (zrzuty i symulacja wejścia) w CI nowego repo.

## 8. Ryzyka

| Ryzyko | Skutek | Ograniczenie |
|---|---|---|
| Rozjazd `vizrs-render` ↔ Chromium (wygładzanie glifów, filtry, 3D) | Artist przestaje być WYSIWYG | bramka `native-compare.js`; znacznik „przybliżony”; podgląd Chromium jednym kliknięciem |
| Podwójna praca przy każdej funkcji renderera (JS + Rust) | wolniejszy rozwój | dziś już obowiązuje (PLAN-native); `vizrs-scene` zmniejsza powierzchnię |
| Zgubione funkcje web Artist przy przepisaniu | regres dla projektanta | lista kontrolna z DOKUMENTACJI §5 (Artist) jako kryterium A9; web Artist działa równolegle do A9 |
| Rozjazd wersji formatu show między repo | uszkodzone show | wersja schematu, `validate.rs` po stronie serwera, test kontraktu w CI obu repo |
| egui: wygląd mniej „systemowy”, brak pisowni z systemu | odbiór UI | motyw VizRS; pisownia z hunspell w A7 |
| Koszt | szacunkowo kilka miesięcy (A4–A6 największe) | etapy dają wartość osobno: A1 i A6 pomagają kanałowi nawet bez Artist |

## 9. Decyzje do podjęcia

1. **Zasada „Jeden renderer”** – zgoda, że kanwa Artist rysuje `vizrs-render` (druga implementacja pilnowana testami)?
   Alternatywa: kanwa jako osadzony WebView z `renderer.js` w natywnym oknie (wierność 1:1, ale nie „natywny Rust”).
2. **Toolkit:** egui (rekomendacja) / Slint / GTK 4.
3. **Los `web/artist.html`:** zamrożony zapas na VPS (Artist przez przeglądarkę bez instalacji) czy usunięty po A9?
   Rekomendacja: zostawić zamrożony do wydania 3.0 – praca zdalna na VPS bez instalacji jest wartościowa.
4. **Gdzie żyje `vizrs-scene` + `vizrs-render`:** w tym repo (`source/render/`, Artist zależy przez git) – rekomendacja,
   bo playout kanału już z niego korzysta – czy w trzecim repo „VizRS Core”?
5. **Nazwa i licencja** nowego repo; czy Artist ma umieć pracować w pełni bez serwera (tryb pliku), czy tylko z serwerem.

## 10. Zmiany w tym repo wynikające z planu

- A1: `source/render/` dzieli się na `vizrs-scene` (lib) + `vizrs-render`; DOKUMENTACJA §10.1, CLAUDE.md (struktura).
- A9: CLAUDE.md – sekcje o Artist (`artist.html`, `tests/e2e/artist.js`, menu w `source/shell/`) odsyłają do nowego repo;
  zasada „Jeden renderer” przeformułowana wg decyzji §9 pkt 1; `docs/plany/README.md` – stan planu.
