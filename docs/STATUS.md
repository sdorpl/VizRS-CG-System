# STATUS – stan prac

> Aktualizuj w tym samym commicie, który zmienia stan. Plan: [plany/PLAN-rozbudowy.md](plany/PLAN-rozbudowy.md).
>
> **Ostatnia aktualizacja:** 2026-11-07

## Bieżący etap
**Wydanie 2.1.1** (2026-11-09, serwisowe: adres wyjścia dla OBS w oknie Trio) po 2.1.0 „Natywnie”: naprawiony instalator Windows, okno Trio z pytaniami aplikacji, menu systemowym i stanem anteny w pasku zadań, test okna w CI na Windows i macOS. Plan F0–F4 wdrożony w całości; zawartość wydań: [CHANGELOG.md](../CHANGELOG.md). Kolejne prace wynikają z [LUKI.md](LUKI.md).

**Osobny Artist (VizRS CG Artist)** – krok A7 planu [plany/PLAN-artist-osobny.md](plany/PLAN-artist-osobny.md) wdrożony (2026-11-07): projektowanie scen to od teraz osobny program **VizRS CG Artist** (`vizrs-cg-artist`, repo https://github.com/sdorpl/VizRS-CG-Artist, wydanie 0.3.0 z 2026-11-07, Tauri 2). W `vizrs-shell` okno Artist usunięte – powłoka obsługuje tylko Trio i wyjście (podgląd); `vizrs-shell artist` i podanie pliku kończą się błędem (kod 2) z odesłaniem do VizRS CG Artist. Paczki `.deb` i AppImage bez wpisów Artist (tylko Trio; brak skojarzeń MIME `.cgpack`/`.vizrs.json`), a `/artist` serwera zostaje jako zamrożony zapas w przeglądarce do 3.0.

**Okna aplikacji i serwer** – [plany/PLAN-natywne-i-serwer.md](plany/PLAN-natywne-i-serwer.md): okna aplikacji Trio i serwer na VPS/Docker (wydane w 2.0.1, w `main`).

**Playout kanału** – [plany/PLAN-kanal.md](plany/PLAN-kanal.md): własny playout na bazie ffplayout („VizRS Playout”, repo `VizRS ffPlayout System`) **porzucony 2026-10-24** – repozytorium zostaje jako archiwum i wzorzec. Kanał z grafiką VizRS: **VizRS CG Playout** (sidecar dla ErsatzTV, `../VizRS CG Playout`) – komendy na start klipu, reklamy bez grafiki, wtapianie `/output` bezgłowym Chromium.

**Renderer natywny** – [plany/PLAN-native.md](plany/PLAN-native.md): `vizrs-render` (`source/render/`, gałąź `render-native`) rysuje antenę bez Chromium dla playoutu kanału; etapy N1 i N2 (0.2) zrobione – INFO24 i R7 w całości natywnie; następne: ruch `node.motion` (N3), pomiar na Ampere A1.

| Etap | Stan |
|---|---|
| N0 fundament CLI (`VIZRS_*`, `serve`, `--healthcheck`, blokada katalogu danych, token wymagany w sieci) | zrobione |
| N1 Docker i VPS (`Dockerfile`, compose + Caddy, nginx, RUNBOOK, job CI `docker`) | zrobione; obraz sprawdza CI (lokalnie brak Dockera – sprawdzona statyczna binarka musl w pustym środowisku) |
| N2 bezpieczeństwo sieciowe (token wyjścia, spowalnianie prób, limit na adres, termin nagłówków, `--trust-proxy`, `--log-stdout`, SIGTERM) | zrobione |
| N3 okna aplikacji (`vizrs artist|trio|output`, `--connect`, `vizrs-app` bez konsoli, skróty Linux/Windows, wskaźnik połączenia, `tests/e2e/app.js`) | zrobione; Windows/macOS: sprawdzona kompilacja (clippy dla celów), uruchomienie okien – do sprawdzenia na tych systemach |
| N4.1 okno natywne `vizrs-shell` (Trio i wyjście; menu, pisownia pl, dołączanie/uruchamianie serwera, skróty, CI, `tools/engines.js`; Artist to osobny program VizRS CG Artist) | zrobione; Linux sprawdzony w działaniu, Windows – kompilacja (clippy), macOS – CI |
| Krok A7 planu PLAN-artist-osobny (wydzielenie Artist do osobnego programu VizRS CG Artist 0.3.0 na Tauri 2; w `vizrs-shell` tylko Trio i wyjście, `/artist` jako zamrożony zapas w przeglądarce) | zrobione (2026-11-07) |
| N4.2, N4.3, instalatory Windows/macOS, N5 relay i kanały | opcjonalne – czekają na decyzję |

## Zrobione (wg planu)
| Etap | Kroki | Gdzie |
|---|---|---|
| F0 | F0.1 testy (`cargo test`, 72), F0.2 E2E (`tests/e2e/run.js`, `artist.js`), F0.3 zrzuty wzorcowe (`golden.js`, 39 stron), F0.4 CI, F0.5 STATUS/LUKI | `source/tests`, `.github/workflows/ci.yml` |
| F1 | F1.1 token/`--bind`/CSRF, F1.2 kopie show, F1.3 `rev`+409, F1.4 walidacja (`validate.rs`), F1.5 odporność serwera, F1.6 izolacja błędów węzłów + `/api/report`, F1.7 snapshot bez migotania, F1.8 fonty offline, F1.9 `/health` | `engine.rs`, `guard.rs`, `validate.rs`, `fonts.rs`, `renderer.js` |
| F2 | F2.1 rundown, F2.2 skróty stron, F2.3 preview bus + key/fill, F2.4 odroczony szablon + deploy, F2.5 makra/komendy zbiorcze, F2.6 presety segmentów (makra „z anteny”), F2.7 log audytowy, F2.8 znacznik zapisu, F2.9 OSC/webhook/zmienne Companion | `engine.rs`, `main.rs`, `osc.rs`, `hook.rs`, `trio.html` |
| F3 | F3.1 reguły pól, F3.2 tekst sformatowany, F3.3 Word Replace (+ pisownia przeglądarki), F3.4 dock, F3.5 markery/zoom/kopiuj-wklej/ramka/DnD w drzewie, F3.6 presety animacji, F3.7 edycja w podglądzie | `artist.html`, `trio.html`, `richtext.js`, `renderer.js` |
| Ticker | T1 model i parser (`ticker-model.js`, `ticker.rs`, wspólne wektory), T2 renderer (ciągłość przy UPDATE, whip, separatory, błysk, squeeze), T3 edytor wiadomości w Trio, T4 Artist (panel trybu, presety, reguły pola), T5.1 komendy `TICKER`/`/api/ticker`/OSC, T7 testy | [plany/PLAN-ticker.md](plany/PLAN-ticker.md), `web/ticker-*.js`, `src/ticker.rs` |
| Tekst | ściskanie (`squeeze` do `squeezeMin` bez zmiany fontu, `squeeze-shrink`), wskaźnik i ostrzeżenie w Trio, dopasowanie wiadomości tickera | `renderer.js`, `trio.html`, `validate.rs` |
| Paczki i ruch | paczka belek `.cgpack` (sceny + strony + grafiki + czcionki, czcionki z serwera na /output); ruch bez kluczy (`node.motion`: 21 presetów IN/OUT, 30 efektów tekstu na wejściu i 26 na wyjściu, 11 pętli, krzywe GSAP); prosty tryb sekwencji w Artist (kaskada, po kolei, na klucze i z kluczy) | `web/cgpack.js`, `web/motion.js`, `artist.html`, `renderer.js` |
| Lekkie wyjście | `/output?fps=N` (pętla w klatkażu kanału), `?fx=step` (efekty uwagi schodkami), uśpienie pętli bez stron, wskazówki dla scen kanału; L5 (wskaźnik w Artist) – LUKI #20 | [plany/PLAN-lekkie-output.md](plany/PLAN-lekkie-output.md), `renderer.js`, `motion.js`, `output.html` |
| F4 | F4.1 wektory modelu tekstu, F4.3 blokady edycji, F4.4 `deploy/`, runbook | `tests/vectors`, `deploy/`, `docs/` |

## Następne zadanie
Wybierz z [LUKI.md](LUKI.md) pozycję o najwyższym priorytecie (G1 → G2 → G3).

## Blokery / uwagi
- Binarki macOS/Windows buduje CI (`.github/workflows/ci.yml`); lokalnie przetestowano tylko Linux x86_64.
- E2E wymaga Playwright (`PLAYWRIGHT_DIR`); zrzuty wzorcowe zależą od maszyny/fontów (test lokalny).
