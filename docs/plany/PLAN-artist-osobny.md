# PLAN: VizRS CG Artist jako osobna aplikacja (osobne repo), Rust tam, gdzie się opłaca

> Stan (2026-11-07): **A0, A2–A4, A6, A7 wykonane (2026-11-07); A1 (`vizrs-model`) i A5 (`.cgpack` w Ruście) – do zrobienia**, VizRS CG Artist wydany (0.3.0). Decyzja użytkownika (2026-11-07): **nie przepisujemy wszystkiego na Rust
> na siłę – kanwa zostaje w JS** (`renderer.js`, `ScenePlayer`), zasada „Jeden renderer” obowiązuje dalej.
> Opis bieżącego działania Artist w [DOKUMENTACJA](../DOKUMENTACJA.md) dotyczy zamrożonego `/artist`.

## 1. Cel

Artist (projektowanie scen) staje się osobnym programem **VizRS CG Artist** w osobnym repozytorium, z własnym
cyklem wydań. Serwer `vizrs` + Trio + `/output` + telefon zostają w VizRS CG System (podział Viz Artist / Viz Trio).

Do Rusta przechodzi to, co tam naturalnie pasuje: okno, menu, pliki, import i paczki, walidacja, praca bez serwera.
Zostaje w JS: kanwa (ta sama `renderer.js` co antena – WYSIWYG z OBS) i panele UI, które dziś działają dobrze.

Zyski:
- niezależne wydania Artist i serwera;
- Artist działa **bez serwera** (otwórz/zapisz `.show.json`, `.cgpack`), a z serwerem – wdraża sceny na antenę;
- operacje plikowe i import paczek w Ruście: szybsze, testowalne `cargo test`, bez limitów przeglądarki;
- serwer i jego binarka chudną o UI Artist.

## 2. Punkt wyjścia (stan 2.0.19)

| Element | Dziś |
|---|---|
| UI Artist | `web/artist.html` (~2,3 tys. wierszy) + `native.js` (most z oknem) + `ui.css` |
| współdzielone z Trio/Output | `renderer.js`, `motion.js`, `text-model.js`, `richtext.js`, `ticker-*.js`, `field-rules.js`, `color-picker.js` |
| tylko Artist | `cgpack.js` (ZIP), część `qr.js` |
| okno natywne | `source/shell/` (tao + wry + muda + rfd): menu, „Zapisz jako”, pytanie przy zamknięciu, kopia robocza, jedna instancja, skojarzenia |
| API używane przez Artist | `GET/PUT /api/show` (`rev`, 409), `/api/assets`, `/assets/*`, blokady scen, `/events`, `/fonts/*`, `/api/backups` |

## 3. Podział: co w Ruście, co w JS

| Część | Gdzie | Uzasadnienie |
|---|---|---|
| Kanwa, odtwarzanie, podgląd (`renderer.js`, `motion.js`, `richtext.js`, ticker) | **JS** (bez zmian) | jeden renderer z anteną, zero ryzyka rozjazdu |
| Panele (drzewo, oś czasu, Właściwości, presety, kolory) | **JS** | działają, są świeżo przebudowane (PLAN-panel-wlasciwosci); przepisanie nic nie daje użytkownikowi |
| Okno, menu, skróty systemowe, dialogi plików, jedna instancja, skojarzenia, pamięć okna | **Rust** – `vizrs-cg-artist` na **GTK 4 + WebKitGTK 6.0** (`gtk4`, `webkit6`, opcjonalnie `libadwaita`); moduły bez GUI (`bridge`, `connect`, `instance`, `paths`, `server`) przeniesione z `source/shell/` | decyzja 2026-11-07; prace z [PLAN-gtk4.md](PLAN-gtk4.md) (G0–G6) realizowane w nowym repo |
| Lokalny magazyn show (plik, kopia robocza, kopie zapasowe, assets, fonty) | **Rust** | tryb bez serwera; strona mówi do niego tym samym API co do serwera |
| Walidacja show | **Rust** – wspólny crate `vizrs-model` (wydzielony z `validate.rs`, bez zależności) | ten sam strażnik w serwerze i w Artist |
| Paczki `.cgpack` (`cgpack.js`) | **Rust** – `vizrs-pack` | parsowanie wrogich danych, ZIP, duże pliki – lepiej natywnie; testy `tests/cgpack.mjs` |
| Wdrożenie na serwer (deploy sceny/paczki, konflikt `rev`, blokady) | **Rust** (klient HTTP) + okno scalania w JS | token trzymany poza stroną |
| `text-model.js` | **JS** | pilnowany wektorami – nie ruszamy |

## 4. Architektura

```
repo „VizRS CG Artist”                                 repo „VizRS CG System”
┌────────────────────────────────────────────┐        ┌──────────────────────────────┐
│ vizrs-cg-artist (Rust: gtk4 + webkit6)     │        │ vizrs (serwer, zero zależn.) │
│  ├─ lokalny serwer API na 127.0.0.1:losowy │  HTTP  │  /api/show rev+409, assets,  │
│  │   (show z pliku, assets, fonty)         │◀──────▶│  locks, /events, /fonts      │
│  ├─ import .cgpack, walidacja              │ deploy │ Trio, /output, /mobile       │
│  └─ WebView ← web/ (artist.html + kopie    │        │ web/renderer.js … (źródło)   │
│               renderer.js, motion.js …)    │        └──────────────────────────────┘
└────────────────────────────────────────────┘
        crate vizrs-model (validate, typy show) – w repo CG System, zależność git z tagiem
```

Zasady:
1. **Strona mówi jednym API.** `artist.html` woła `/api/show`, `/api/assets` itd.; odpowiada lokalny serwer w procesie
   Artist (tryb pliku) albo – przez pośrednika w Artist – zdalny `vizrs` (tryb anteny). Strona nie zna różnicy poza
   wskaźnikiem połączenia.
2. **Wspólne moduły JS są kopią ze źródłem w CG System**: `tools/sync-cg.sh --check`
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
| **A2** | Nowe repo **VizRS CG Artist**: workspace, CI (Linux), kopia `web/` Artist + `sync-cg.sh --check`, okno GTK 4 + WebKitGTK 6.0 (prototyp G0 z PLAN-gtk4: skalowanie ułamkowe, pisownia pl, most IPC), moduły bez GUI z `source/shell/` | okno otwiera Artist z działającym zdalnym serwerem; `tools/engines.js` – 0 różnic WebKitGTK 6.0 wobec Chromium | M |
| **A3** | Lokalny serwer API w procesie Artist: show z pliku, assets, fonty, kopia robocza, kopie zapasowe, walidacja | Artist działa bez `vizrs`; `tests/e2e/artist.js` przechodzi na trybie pliku | M |
| **A4** | Tryb anteny: połączenie z `vizrs` (token, `rev`/409, blokady, `/events`), deploy sceny / zestawu scen, okno scalania | edycja lokalna → deploy → strona w Trio na antenie | M |
| **A5** | `.cgpack` w Ruście (`vizrs-pack`); strona woła IPC/API zamiast `cgpack.js` | dla `examples/*.cgpack` wynik identyczny z JS (test porównawczy), potem usunięcie JS | M |
| **A6** | Integracja z systemem: AppImage/.deb (Linux), potem Windows (§7 pkt 1), skojarzenia `.show.json`/`.cgpack`, ostatnie pliki | instalacja na czystym Linuksie i Windows | S |
| **A7** | Przełączenie w CG System: `/artist` z serwera zamrożony (§7 pkt 2), menu Artist znika z `source/shell/`, CLAUDE.md i DOKUMENTACJA odsyłają do nowego repo | wykonane (2026-11-07): `vizrs-shell` tylko Trio/wyjście, `/artist` zamrożony zapas w przeglądarce, testy E2E zaliczone | S |
| **A8** (opcja) | Kolejne części do Rusta tylko z konkretnym powodem (np. eksport filmu/miniatur przez `vizrs-render`, wyszukiwanie w dużych show) | – | – |

## 6. Ryzyka

| Ryzyko | Ograniczenie |
|---|---|
| Rozjazd kopii `renderer.js` między repo → Artist nie jest WYSIWYG | `sync-cg.sh --check` w CI; Artist wydawany z wersją modułów oznaczoną tagiem CG System |
| Rozjazd formatu show | `vizrs-model` jako jedna implementacja walidacji, wersja schematu, test kontraktu w obu repo |
| Import w Ruście daje inny wynik niż `cgpack.js` | test porównawczy na wszystkich fixture'ach przed usunięciem JS |
| Utrata Artist w przeglądarce na VPS | `/artist` zamrożony do 3.0 (§7 pkt 2) |
| WebKitGTK 6.0 renderuje inaczej niż Chromium (tekst, skalowanie ułamkowe – WebKit #271714) | `tools/engines.js` jako bramka w A2; zapas: okno wry/GTK 3 do czasu poprawki |

## 7. Decyzje (2026-11-07)

1. **Okno: Tauri 2** (Linux, Windows, macOS) od wersji 0.3.0 (zmiana decyzji z 2026-11-07, wcześniej planowano GTK 4).
   Paczki: Linux (.deb, .rpm, AppImage), Windows (.msi, instalator), macOS (.dmg). Program działa bez serwera
   (magazyn lokalny: `show.json`, `assets/` w katalogu danych), a sceny wysyła na serwer `vizrs`.
2. **`/artist` w serwerze:** zostaje **zamrożony** jako zapas do pracy zdalnej na VPS (bez instalacji) do wydania 3.0;
   nowe funkcje Artist powstają tylko w VizRS CG Artist. Opcja na przyszłość: podawanie przez serwer pod `/artist`
   strony z wydań VizRS CG Artist zamiast zamrożonej (decyzja użytkownika – jeszcze nie podjęta).
3. **`vizrs-model` zostaje w tym repo** (serwer jest właścicielem formatu show); VizRS CG Artist zależy od niego przez git z tagiem.
4. **Nazwa:** repo i aplikacja **VizRS CG Artist** (binarka `vizrs-cg-artist`). Licencja – do ustalenia przy zakładaniu repo (A2).

## 8. Zmiany w tym repo wynikające z planu

- A1: crate `vizrs-model` (struktura w CLAUDE.md, DOKUMENTACJA – walidacja).
- A2: źródło prawdy dla modułów JS współdzielonych z Artist (opis w CLAUDE.md jak przy `text-model.js`).
- A7: CLAUDE.md, DOKUMENTACJA, `source/shell/` – odesłania do VizRS CG Artist; `/artist` i `tests/e2e/artist.js` zostają jako zamrożony zapas; `docs/plany/README.md` – stan.
