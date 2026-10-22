# LUKI – znane braki (uszeregowane: niezawodność → wierność → funkcje)

Zamykając pozycję, usuń wiersz i dopisz wersję w `CHANGELOG.md`. Nowe braki dopisuj od razu.

## G1 – niezawodność i bezpieczeństwo
| # | Luka | Uwagi |
|---|---|---|
| 1 | Brak TLS w samej binarce (token jawnie w sieci bez proxy) | zamierzone (bez kryptografii od zera); HTTPS przez reverse proxy: `deploy/docker-compose.yml` (Caddy), `deploy/nginx-vizrs.conf`, RUNBOOK „VPS i Docker” |
| 2 | Wątek na połączenie (limit 512, nie pula) | wystarcza dla studia; pula przy większej skali |
| 3 | `PUT /api/show` (import) nadal zastępuje cały show | chronione `rev`/`force`, kopiami i walidacją; brak scalania |
| 4 | Blokady edycji są miękkie (ostrzeżenie + potwierdzenie) | zamierzone – operator może przejąć |
| 5 | CI macOS/Windows nieuruchamiane lokalnie | pierwszy przebieg pokaże ewentualne różnice |

## G2 – wierność
| # | Luka | Uwagi |
|---|---|---|
| 6 | `OUT` zdejmuje warstwę w silniku od razu; animację OUT gra tylko klient | kolejny TAKE w czasie OUT może ją ściąć |
| 7 | Dock tylko przesuwa (dx, dy); brak reguł względem rozmiaru/wysokości celu | BTB ma osie i krawędzie (`layout.dock`) |
| 8 | Konwerter BTB: maski warstw, `layout.dock`, auto-szerokość `follow`, „po warstwie” przy wyjściu → ostrzeżenia, nie przenoszone | ruch, efekty tekstu, pętle i efekty tła przenoszone 1:1; zgodność klatek wejścia z BTB sprawdza `btb-visual.js --frames` |
| 10 | Pisownia: natywna przeglądarki (lang=pl); brak własnego słownika (sjp.pl/Hunspell) | w CEF (OBS dock) zależy od przeglądarki |
| 11 | Znaczniki tekstu: rozmiar `{sz=}` w % (BTB używa px) – przy imporcie z BTB pomijany | |

## G3 – funkcje
| # | Luka | Uwagi |
|---|---|---|
| 12 | Rundown bez harmonogramu czasowego/automatu (zamierzone: tylko ręczne NEXT) | opcja „AUTO” z widocznym stanem i STOP |
| 13 | Brak dedykowanego modułu Companion (jest generyczny HTTP/OSC + zmienne) | |
| 14 | Parenting przez drzewo (zmiana rodzica); brak „pick whip” jak w AE | |
| 15 | Kształty wektorowe, tryby mieszania, ekspresje (wiggle) – poza zakresem | propozycje G, H, K z btb-cgeditor |
| 17 | Okna aplikacji: lista zapisanych połączeń i ekran wyboru – jest w oknie natywnym (Plik → Połącz z serwerem), brak w oknie Chromium (`vizrs trio`, tam `--connect URL`); brak globalnych skrótów, menu systemowego i zasobnika | PLAN-natywne-i-serwer.md N3/N4; skróty sprzętowe – Companion przez HTTP/OSC |
| 18 | Okna aplikacji i `vizrs-shell` na Windows/macOS sprawdzone tylko kompilacją (Linux: testy E2E, prawdziwy Chrome i WebKitGTK) | uruchomić `vizrs trio`, `vizrs-shell` i skróty z `install-windows-shortcuts.ps1` na Windows; macOS – WKWebView niezmierzony `tools/engines.js` |
| 19 | `vizrs-shell`: brak zasobnika, skrótów globalnych, wyboru monitora i instalatorów (MSI/.deb/.app); na Waylandzie ikona okna tylko przez wpis `vizrs-shell.desktop` | PLAN-natywne-i-serwer.md N4.2–N4.5 |
| 16 | Ticker: brak źródła zewnętrznego (URL z JSON/RSS i odświeżanie) | PLAN-ticker.md T5.2 (opcja); dziś: `TICKER …` / `/api/ticker` z zewnętrznego skryptu |
