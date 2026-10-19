# STATUS – stan prac

> Aktualizuj w tym samym commicie, który zmienia stan. Plan: [../PLAN-rozbudowy.md](../PLAN-rozbudowy.md), raport: [../RAPORT-analiza-VizRS-vs-BTB.md](../RAPORT-analiza-VizRS-vs-BTB.md).
>
> **Ostatnia aktualizacja:** 2026-10-19

## Bieżący etap
**Wydanie 2.0.0 „Studio”** – wdrożone wszystkie etapy planu F0–F4 (patrz niżej). Kolejne prace wynikają z [LUKI.md](LUKI.md).

## Zrobione (wg planu)
| Etap | Kroki | Gdzie |
|---|---|---|
| F0 | F0.1 testy (`cargo test`, 72), F0.2 E2E (`tests/e2e/run.js`, `artist.js`), F0.3 zrzuty wzorcowe (`golden.js`, 39 stron), F0.4 CI, F0.5 STATUS/LUKI | `source/tests`, `.github/workflows/ci.yml` |
| F1 | F1.1 token/`--bind`/CSRF, F1.2 kopie show, F1.3 `rev`+409, F1.4 walidacja (`validate.rs`), F1.5 odporność serwera, F1.6 izolacja błędów węzłów + `/api/report`, F1.7 snapshot bez migotania, F1.8 fonty offline, F1.9 `/health` | `engine.rs`, `guard.rs`, `validate.rs`, `fonts.rs`, `renderer.js` |
| F2 | F2.1 rundown, F2.2 skróty stron, F2.3 preview bus + key/fill, F2.4 odroczony szablon + deploy, F2.5 makra/komendy zbiorcze, F2.6 presety segmentów (makra „z anteny”), F2.7 log audytowy, F2.8 znacznik zapisu, F2.9 OSC/webhook/zmienne Companion | `engine.rs`, `main.rs`, `osc.rs`, `hook.rs`, `trio.html` |
| F3 | F3.1 reguły pól, F3.2 tekst sformatowany, F3.3 Word Replace (+ pisownia przeglądarki), F3.4 dock, F3.5 markery/zoom/kopiuj-wklej/ramka/DnD w drzewie, F3.6 presety animacji, F3.7 edycja w podglądzie | `artist.html`, `trio.html`, `richtext.js`, `renderer.js` |
| Ticker | T1 model i parser (`ticker-model.js`, `ticker.rs`, wspólne wektory), T2 renderer (ciągłość przy UPDATE, whip, separatory, błysk, squeeze), T3 edytor wiadomości w Trio, T4 Artist (panel trybu, presety, reguły pola), T5.1 komendy `TICKER`/`/api/ticker`/OSC, T6 import z BTB, T7 testy | [../PLAN-ticker.md](../PLAN-ticker.md), `web/ticker-*.js`, `src/ticker.rs` |
| Tekst | ściskanie jak w BTB (`squeeze` do `squeezeMin` bez zmiany fontu, `squeeze-shrink`), wskaźnik i ostrzeżenie w Trio, dopasowanie wiadomości tickera, import BTB (granice, pola dla warstw bez pola) | `renderer.js`, `trio.html`, `btb-io.js`, `validate.rs` |
| F4 | F4.1 wspólne wektory (`tools/sync-btb.sh`), F4.2 konwerter BTB (`btb-io.js`, `.cgpack`), F4.3 blokady edycji, F4.4 `deploy/`, runbook | `tests/vectors`, `web/btb-io.js`, `deploy/`, `docs/` |

## Następne zadanie
Wybierz z [LUKI.md](LUKI.md) pozycję o najwyższym priorytecie (G1 → G2 → G3).

## Blokery / uwagi
- Binarki macOS/Windows buduje CI (`.github/workflows/ci.yml`); lokalnie przetestowano tylko Linux x86_64.
- E2E wymaga Playwright (`PLAYWRIGHT_DIR`); zrzuty wzorcowe zależą od maszyny/fontów (test lokalny).
