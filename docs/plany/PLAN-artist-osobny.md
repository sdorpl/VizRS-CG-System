# PLAN: VizRS Artist jako osobna aplikacja (osobne repo), Rust tam, gdzie się opłaca

> Stan (2026-11-07): **propozycja (A0–A8)**. Decyzja użytkownika (2026-11-07): **nie przepisujemy wszystkiego na Rust
> na siłę – kanwa zostaje w JS** (`renderer.js`, `ScenePlayer`), zasada „Jeden renderer” obowiązuje dalej.
> Opis bieżącego działania Artist zostaje w [DOKUMENTACJA](../DOKUMENTACJA.md) do czasu przeniesienia.

## 1. Cel

Artist (projektowanie scen) staje się osobnym programem **VizRS Artist** w osobnym repozytorium, z własnym
cyklem wydań. Serwer `vizrs` + Trio + `/output` + telefon zostają w VizRS CG System (podział Viz Artist / Viz Trio).

Do Rusta przechodzi to, co tam naturalnie pasuje: okno, menu, pliki, import i paczki, walidacja, praca bez serwera.
Zostaje w JS: kanwa (ta sama `renderer.js` co antena – WYSIWYG z OBS) i panele UI, które dziś działają dobrze.

Zyski:
- niezależne wydania Artist i serwera;
- Artist działa **bez serwera** (otwórz/zapisz `.show.json`, `.cgpack`), a z serwerem – wdraża sceny na antenę;
- operacje plikowe i import BTB w Ruście: szybsze, testowalne `cargo test`, bez limitów przeglądarki;
- serwer i jego binarka chudną o UI Artist.

## 2. Punkt wyjścia (stan 2.0.19)

| Element | Dziś |
|---|---|
| UI Artist | `web/artist.html` (~2,3 tys. wierszy) + `native.js` (most z oknem) + `ui.css` |
| współdzielone z Trio/Output | `renderer.js`, `motion.js`, `text-model.js`, `richtext.js`, `ticker-*.js`, `field-rules.js`, `color-picker.js` |
| tylko Artist | `btb-io.js` (import BTB), `cgpack.js` (ZIP), część `qr.js` |
| okno natywne | `source/shell/` (tao + wry + muda + rfd): menu, „Zapisz jako”, pytanie przy zamknięciu, kopia robocza, jedna instancja, skojarzenia |
| API używane przez Artist | `GET/PUT /api/show` (`rev`, 409), `/api/assets`, `/assets/*`, blokady scen, `/events`, `/fonts/*`, `/api/backups` |

## 3. Podział: co w Ruście, co w JS

| Część | Gdzie | Uzasadnienie |
|---|---|---|
| Kanwa, odtwarzanie, podgląd (`renderer.js`, `motion.js`, `richtext.js`, ticker) | **JS** (bez zmian) | jeden renderer z anteną, zero ryzyka rozjazdu |
| Panele (drzewo, oś czasu, Właściwości, presety, kolory) | **JS** | działają, są świeżo przebudowane (PLAN-panel-wlasciwosci); przepisanie nic nie daje użytkownikowi |
| Okno, menu, skróty systemowe, dialogi plików, jedna instancja, skojarzenia, pamięć okna | **Rust** – `vizrs-artist` (z kodu `source/shell/`) | już jest w Ruście |
| Lokalny magazyn show (plik, kopia robocza, kopie zapasowe, assets, fonty) | **Rust** | tryb bez serwera; strona mówi do niego tym samym API co do serwera |
| Walidacja show | **Rust** – wspólny crate `vizrs-model` (wydzielony z `validate.rs`, bez zależności) | ten sam strażnik w serwerze i w Artist |
| Import BTB (`btb-io.js`) i paczki `.cgpack` (`cgpack.js`) | **Rust** – `vizrs-btb` / `vizrs-pack` | parsowanie wrogich danych, ZIP, duże pliki – lepiej natywnie; testy na `tests/fixtures/btb/` |
| Wdrożenie na serwer (deploy sceny/paczki, konflikt `rev`, blokady) | **Rust** (klient HTTP) + okno scalania w JS | token trzymany poza stroną |
| `text-model.js` | **JS** (kopia z BTB) | wspólny z BTB, pilnowany wektorami – nie ruszamy |

## 4. Architektura

```
repo „VizRS Artist”                                   repo „VizRS CG System”
┌────────────────────────────────────────────┐        ┌──────────────────────────────┐
│ vizrs-artist (Rust: tao + wry + muda + rfd)│        │ vizrs (serwer, zero zależn.) │
│  ├─ lokalny serwer API na 127.0.0.1:losowy │  HTTP  │  /api/show rev+409, assets,  │
│  │   (show z pliku, assets, fonty)         │◀──────▶│  locks, /events, /fonts      │
│  ├─ import BTB, .cgpack, walidacja         │ deploy │ Trio, /output, /mobile       │
│  └─ WebView ← web/ (artist.html + kopie    │        │ web/renderer.js … (źródło)   │
│               renderer.js, motion.js …)    │        └──────────────────────────────┘
└────────────────────────────────────────────┘
        crate vizrs-model (validate, typy show) – w repo CG System, zależność git z tagiem
```

Zasady:
1. **Strona mówi jednym API.** `artist.html` woła `/api/show`, `/api/assets` itd.; odpowiada lokalny serwer w procesie
   Artist (tryb pliku) albo – przez pośrednika w Artist – zdalny `vizrs` (tryb anteny). Strona nie zna różnicy poza
   wskaźnikiem połączenia.
2. **Wspólne moduły JS są kopią ze źródłem w CG System** (jak `text-model.js` z BTB): `tools/sync-cg.sh --check`
   w CI Artist pilnuje identyczności `renderer.js`, `motion.js`, `richtext.js`, `ticker-*.js`, `field-rules.js`, `color-picker.js`.
   Zmiana rysowania zawsze najpierw w CG System.
3. **Serwer zostaje strażnikiem anteny** – deploy przechodzi przez `validate.rs`; lokalnie ta sama walidacja z `vizrs-model`.
4. **Rdzeń `vizrs` bez nowych zależności.** `vizrs-model` też bez zależności (wydzielony kod `validate.rs` + `json.rs`).
5. **IPC tylko z białej listy** (jak dziś w `source/shell/`); strona nie dostaje dostępu do systemu.
6. **Format show jest kontraktem**: wersja schematu, Artist nie zapisze show nowszego niż zna.

## 5. Etapy

| Etap | Zakres | Gotowe gdy | Wielkość |
|---|---|---|---|
| **A0** | Decyzje §7; spis, czego `artist.html` używa z serwera i z `native.js` | lista API i komunikatów IPC zatwierdzona | S |
| **A1** | Wydzielenie `vizrs-model` (typy show, `validate.rs`, JSON) w tym repo jako crate bez zależności; serwer z niego korzysta | `cargo test`, E2E bez zmian | M |
| **A2** | Nowe repo **VizRS Artist**: workspace, CI (Linux, potem Windows), kopia `web/` Artist + `sync-cg.sh --check`, okno z kodu `source/shell/` (tylko Artist) | okno otwiera Artist z działającym zdalnym serwerem | S |
| **A3** | Lokalny serwer API w procesie Artist: show z pliku, assets, fonty, kopia robocza, kopie zapasowe, walidacja | Artist działa bez `vizrs`; `tests/e2e/artist.js` przechodzi na trybie pliku | M |
| **A4** | Tryb anteny: połączenie z `vizrs` (token, `rev`/409, blokady, `/events`), deploy sceny / zestawu scen, okno scalania | edycja lokalna → deploy → strona w Trio na antenie | M |
| **A5** | Import BTB i `.cgpack` w Ruście (`vizrs-btb`, `vizrs-pack`); strona woła IPC/API zamiast `btb-io.js`/`cgpack.js` | dla `tests/fixtures/btb/` i `examples/*.cgpack` wynik identyczny z JS (test porównawczy), potem usunięcie JS | M |
| **A6** | Integracja z systemem: AppImage/.deb, Windows, skojarzenia `.show.json`/`.cgpack`, ostatnie pliki | instalacja na czystym Linuksie i Windows | S |
| **A7** | Przełączenie w CG System: `/artist` z serwera zamrożony albo usunięty (§7 pkt 2), menu Artist znika z `source/shell/`, CLAUDE.md i DOKUMENTACJA odsyłają do nowego repo | tydzień pracy projektowej tylko w nowym Artist | S |
| **A8** (opcja) | Kolejne części do Rusta tylko z konkretnym powodem (np. eksport filmu/miniatur przez `vizrs-render`, wyszukiwanie w dużych show) | – | – |

## 6. Ryzyka

| Ryzyko | Ograniczenie |
|---|---|
| Rozjazd kopii `renderer.js` między repo → Artist nie jest WYSIWYG | `sync-cg.sh --check` w CI; Artist wydawany z wersją modułów oznaczoną tagiem CG System |
| Rozjazd formatu show | `vizrs-model` jako jedna implementacja walidacji, wersja schematu, test kontraktu w obu repo |
| Import w Ruście daje inny wynik niż `btb-io.js` | test porównawczy na wszystkich fixture'ach przed usunięciem JS |
| Utrata Artist w przeglądarce na VPS | decyzja §7 pkt 2 – rekomendacja: zostawić zamrożony do 3.0 |

## 7. Decyzje do podjęcia

1. **Toolkit okna:** zostajemy przy tao + wry (jak `source/shell/`) – rekomendacja, bo kanwa i panele to WebView.
2. **Los `/artist` w serwerze:** zamrożony zapas (praca zdalna na VPS bez instalacji) czy usunięty po A7?
3. **Gdzie `vizrs-model`:** w tym repo (rekomendacja, serwer jest właścicielem formatu) czy w osobnym „VizRS Core”.
4. **Nazwa i licencja** nowego repo.

## 8. Zmiany w tym repo wynikające z planu

- A1: crate `vizrs-model` (struktura w CLAUDE.md, DOKUMENTACJA – walidacja).
- A2: źródło prawdy dla modułów JS współdzielonych z Artist (opis w CLAUDE.md jak przy `text-model.js`).
- A7: CLAUDE.md, DOKUMENTACJA, `tests/e2e/artist.js`, `source/shell/` – odesłania do nowego repo; `docs/plany/README.md` – stan.
