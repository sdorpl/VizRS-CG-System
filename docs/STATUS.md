# STATUS – stan prac

> Aktualizuj w tym samym commicie, który zmienia stan. Plan: [../PLAN-rozbudowy.md](../PLAN-rozbudowy.md), raport: [../RAPORT-analiza-VizRS-vs-BTB.md](../RAPORT-analiza-VizRS-vs-BTB.md).
>
> **Ostatnia aktualizacja:** 2026-10-22

## Bieżący etap
**Wydanie 2.0.1 „Okna i serwer”** (2026-10-22; 2.0.0 „Studio” – wdrożone wszystkie etapy planu F0–F4, patrz niżej). Kolejne prace wynikają z [LUKI.md](LUKI.md).

**Okna aplikacji i serwer** – [../PLAN-natywne-i-serwer.md](../PLAN-natywne-i-serwer.md): okna aplikacji Artist/Trio i serwer na VPS/Docker (wydane w 2.0.1, w `main`).

| Etap | Stan |
|---|---|
| N0 fundament CLI (`VIZRS_*`, `serve`, `--healthcheck`, blokada katalogu danych, token wymagany w sieci) | zrobione |
| N1 Docker i VPS (`Dockerfile`, compose + Caddy, nginx, RUNBOOK, job CI `docker`) | zrobione; obraz sprawdza CI (lokalnie brak Dockera – sprawdzona statyczna binarka musl w pustym środowisku) |
| N2 bezpieczeństwo sieciowe (token wyjścia, spowalnianie prób, limit na adres, termin nagłówków, `--trust-proxy`, `--log-stdout`, SIGTERM) | zrobione |
| N3 okna aplikacji (`vizrs artist|trio|output`, `--connect`, `vizrs-app` bez konsoli, skróty Linux/Windows, wskaźnik połączenia w Artist, `tests/e2e/app.js`) | zrobione; Windows/macOS: sprawdzona kompilacja (clippy dla celów), uruchomienie okien – do sprawdzenia na tych systemach |
| N4.1 okno natywne `vizrs-shell` (Artist/Trio/wyjście, menu, „Zapisz jako”, pisownia pl, dołączanie/uruchamianie serwera, skróty, CI, `tools/engines.js` 82/82 zgodnych z Chromium) | zrobione; Linux sprawdzony w działaniu, Windows – kompilacja (clippy), macOS – CI |
| N4.2–N4.5 (zasobnik, skróty globalne, wybór monitora, instalatory), N5 relay i kanały | opcjonalne – czekają na decyzję |

## Zrobione (wg planu)
| Etap | Kroki | Gdzie |
|---|---|---|
| F0 | F0.1 testy (`cargo test`, 72), F0.2 E2E (`tests/e2e/run.js`, `artist.js`), F0.3 zrzuty wzorcowe (`golden.js`, 39 stron), F0.4 CI, F0.5 STATUS/LUKI | `source/tests`, `.github/workflows/ci.yml` |
| F1 | F1.1 token/`--bind`/CSRF, F1.2 kopie show, F1.3 `rev`+409, F1.4 walidacja (`validate.rs`), F1.5 odporność serwera, F1.6 izolacja błędów węzłów + `/api/report`, F1.7 snapshot bez migotania, F1.8 fonty offline, F1.9 `/health` | `engine.rs`, `guard.rs`, `validate.rs`, `fonts.rs`, `renderer.js` |
| F2 | F2.1 rundown, F2.2 skróty stron, F2.3 preview bus + key/fill, F2.4 odroczony szablon + deploy, F2.5 makra/komendy zbiorcze, F2.6 presety segmentów (makra „z anteny”), F2.7 log audytowy, F2.8 znacznik zapisu, F2.9 OSC/webhook/zmienne Companion | `engine.rs`, `main.rs`, `osc.rs`, `hook.rs`, `trio.html` |
| F3 | F3.1 reguły pól, F3.2 tekst sformatowany, F3.3 Word Replace (+ pisownia przeglądarki), F3.4 dock, F3.5 markery/zoom/kopiuj-wklej/ramka/DnD w drzewie, F3.6 presety animacji, F3.7 edycja w podglądzie | `artist.html`, `trio.html`, `richtext.js`, `renderer.js` |
| Ticker | T1 model i parser (`ticker-model.js`, `ticker.rs`, wspólne wektory), T2 renderer (ciągłość przy UPDATE, whip, separatory, błysk, squeeze), T3 edytor wiadomości w Trio, T4 Artist (panel trybu, presety, reguły pola), T5.1 komendy `TICKER`/`/api/ticker`/OSC, T6 import z BTB, T7 testy | [../PLAN-ticker.md](../PLAN-ticker.md), `web/ticker-*.js`, `src/ticker.rs` |
| Tekst | ściskanie jak w BTB (`squeeze` do `squeezeMin` bez zmiany fontu, `squeeze-shrink`), wskaźnik i ostrzeżenie w Trio, dopasowanie wiadomości tickera, import BTB (granice, pola dla warstw bez pola) | `renderer.js`, `trio.html`, `btb-io.js`, `validate.rs` |
| Paczki i ruch | paczka belek `.cgpack` (sceny + strony + grafiki + czcionki, czcionki z serwera na /output); ruch jak w BTB CG bez kluczy (`node.motion`: 22 presety IN/OUT, 31 efektów tekstu, 11 pętli, krzywe GSAP); import BTB przenosi ruch 1:1; prosty tryb sekwencji w Artist (kaskada, po kolei, na klucze) | `web/cgpack.js`, `web/motion.js`, `artist.html`, `renderer.js`, `btb-io.js` |
| F4 | F4.1 wspólne wektory (`tools/sync-btb.sh`), F4.2 konwerter BTB (`btb-io.js`, `.cgpack`), F4.3 blokady edycji, F4.4 `deploy/`, runbook | `tests/vectors`, `web/btb-io.js`, `deploy/`, `docs/` |

## Następne zadanie
Wybierz z [LUKI.md](LUKI.md) pozycję o najwyższym priorytecie (G1 → G2 → G3).

## Blokery / uwagi
- Binarki macOS/Windows buduje CI (`.github/workflows/ci.yml`); lokalnie przetestowano tylko Linux x86_64.
- E2E wymaga Playwright (`PLAYWRIGHT_DIR`); zrzuty wzorcowe zależą od maszyny/fontów (test lokalny).
