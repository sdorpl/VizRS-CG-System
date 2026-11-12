# LUKI – znane braki (uszeregowane: niezawodność → wierność → funkcje)

Zamykając pozycję, usuń wiersz i dopisz wersję w `CHANGELOG.md`. Nowe braki dopisuj od razu, z kolejnym wolnym numerem (numery nie są używane ponownie; wyjątek z przeszłości: #17 w CHANGELOG przy 2.0.1 to zamknięta luka o przepełnieniu tekstu, nie obecna pozycja).

## G1 – niezawodność i bezpieczeństwo
| # | Luka | Uwagi |
|---|---|---|
| 1 | Brak TLS w samej binarce (token jawnie w sieci bez proxy) | zamierzone (bez kryptografii od zera); HTTPS przez reverse proxy: `deploy/docker-compose.yml` (Caddy), `deploy/nginx-vizrs.conf`, RUNBOOK „VPS i Docker” |
| 2 | Wątek na połączenie (limit 512, nie pula) | wystarcza dla studia; pula przy większej skali |
| 3 | `PUT /api/show` (import) nadal zastępuje cały show | chronione `rev`/`force`, kopiami i walidacją; brak scalania |
| 4 | Blokady edycji są miękkie (ostrzeżenie + potwierdzenie) | zamierzone – operator może przejąć |
| 5 | Testy macOS/Windows działają tylko w CI, nie lokalnie | CI przechodzi na trzech systemach; działanie okien: #18 |

## G2 – wierność
| # | Luka | Uwagi |
|---|---|---|
| 6 | `OUT` zdejmuje warstwę w silniku od razu; animację OUT gra tylko klient | kolejny TAKE w czasie OUT może ją ściąć |
| 7 | Dock tylko przesuwa (dx, dy); brak reguł względem rozmiaru/wysokości celu | brak osi i krawędzi celu |
| 10 | Pisownia: natywna przeglądarki (lang=pl); brak własnego słownika (sjp.pl/Hunspell) | w CEF (OBS dock) zależy od przeglądarki |

## G3 – funkcje
| # | Luka | Uwagi |
|---|---|---|
| 12 | Rundown bez harmonogramu czasowego/automatu (zamierzone: tylko ręczne NEXT) | opcja „AUTO” z widocznym stanem i STOP |
| 13 | Brak dedykowanego modułu Companion (jest generyczny HTTP/OSC + zmienne) | |
| 14 | Parenting przez drzewo (zmiana rodzica); brak „pick whip” jak w AE | |
| 15 | Kształty wektorowe, tryby mieszania, ekspresje (wiggle) – poza zakresem | |
| 16 | Ticker: brak źródła zewnętrznego (URL z JSON/RSS i odświeżanie) | plany/PLAN-ticker.md T5.2 (opcja); dziś: `TICKER …` / `/api/ticker` z zewnętrznego skryptu |
| 17 | Okna aplikacji: lista zapisanych połączeń i ekran wyboru – jest w oknie natywnym (Plik → Połącz z serwerem), brak w oknie Chromium (`vizrs trio`, tam `--connect URL`); brak globalnych skrótów, menu systemowego i zasobnika | plany/PLAN-natywne-i-serwer.md N3/N4; skróty sprzętowe – Companion przez HTTP/OSC |
| 18 | Okna aplikacji `vizrs trio` (Chromium/Edge) na Windows/macOS sprawdzone tylko kompilacją; `vizrs-shell` – test dymny w CI (`shell-smoke`: Windows z instalatora w `Program Files`, macOS), bez pełnego `shell.js` i bez ręcznego sprawdzenia menu i pytań | uruchomić `vizrs trio` i skróty z `install-windows-shortcuts.ps1` na Windows; macOS – WKWebView niezmierzony `tools/engines.js` |
| 19 | `vizrs-shell`: brak zasobnika, skrótów globalnych i paczki macOS (`.app`/`.dmg`, podpis); Windows ma instalator Inno Setup | plany/PLAN-natywne-i-serwer.md N4.2, N4.3, N4.5 |
| 20 | Artist nie wskazuje scen z ciągłym ruchem w stanie ustalonym (`blink`, `motion.loop`, `motion.bg`, ticker `crawl`, wideo) – kosztownych na kanale wypalanym bezgłowym Chromium | plany/PLAN-lekkie-output.md L5; np. ikona „ciągły ruch” na liście scen |
