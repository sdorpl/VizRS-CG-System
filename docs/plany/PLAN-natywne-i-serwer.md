# PLAN: VizRS w oknie systemowym (Artist, Trio) i jako serwer bez GUI (VPS, Docker)

> Status: **N0–N3 i N4.1 (okno natywne `vizrs-shell`) wdrożone** (wydanie 2.0.1, 2026-10-22, gałąź `main`); N4.2–N4.5 i N5 czekają na decyzję (LUKI #17–#19). Stan: [docs/STATUS.md](../STATUS.md).

### Realizacja i odstępstwa od planu
- Decyzje z §9 przyjęte wg rekomendacji: wariant A (Chromium `--app`, zero zależności), najpierw serwer (N0–N2), potem okna (N3), obraz `scratch`, Caddy w compose, dwie role tokenów (operator / wyjście), kanał = kontener.
- §9 pkt 2 (zamknięcie okna) rozstrzygnięty bez pytania w UI: **serwer działa dalej, dopóki podłączone jest jakiekolwiek okno lub wyjście (OBS)**, potem kończy pracę po `--linger` s. Zamknięcie Artist nigdy nie zdejmuje anteny.
- Okno lokalne **nie dostaje losowego tokenu sesji** (§4 pkt 3): serwer okna słucha na `127.0.0.1` jak dotąd bez tokenu (chroni go kontrola Origin/Sec-Fetch); token działa, jeśli ustawiony (`--token`/`VIZRS_TOKEN`). Dzięki temu drugie okno i OBS łączą się bez dodatkowej konfiguracji.
- Lista połączeń z ekranem wyboru (§4 pkt 6) – niezrobiona (jest `--connect` / `VIZRS_CONNECT`); wpis w LUKI #17.
- Windows/macOS: kod okien, sygnałów i `vizrs-app` sprawdzony kompilacją (`cargo clippy --target …`); uruchomienie – do sprawdzenia na tych systemach (LUKI #18). Obraz Docker sprawdza job CI `docker` (lokalnie brak Dockera).
- Dodatkowo: Artist subskrybuje `/events` (wskaźnik połączenia) – serwer okna wie, że Artist jest otwarty.
- **N4.1 (decyzja użytkownika: Artist jako natywne okno powłoki, zależności dozwolone):** `source/shell/` – tao + wry + muda + rfd. Obawa z §3 o wierność WebKitGTK sprawdzona pomiarem (`tools/engines.js`): jedyne różnice (3/82 pól, zbędne ściśnięcie do 90%) wynikały z **błędu renderera** (pomiar tekstu przed wczytaniem odmiany fontu, bez ponownego pomiaru, gdy przeglądarka nie zgłasza `loadingdone`) – poprawionego w rendererze, co pomaga też OBS. Po poprawce 82/82 zgodnych, więc Artist na Linuksie może działać w WebKitGTK. Powłoka uruchamia serwer jako osobny proces z `--idle-exit` (zamiast wkompilowania).
> Powiązane: [PLAN-rozbudowy.md](PLAN-rozbudowy.md), [docs/LUKI.md](../LUKI.md) (G1 #1 TLS, #2 wątki), [deploy/](../../deploy/), [docs/RUNBOOK.md](../RUNBOOK.md).

## 0. Streszczenie

| Postulat | Rekomendacja | Koszt |
|---|---|---|
| Artist jako natywna aplikacja w oknie systemowym | **Etap 1:** `vizrs artist` otwiera Artist w osobnym oknie aplikacji silnika Chromium obecnego w systemie (Edge na Windows, Chrome/Chromium na Linux), z własnym profilem, bez pasków przeglądarki, z serwerem w tym samym procesie. Zero nowych zależności, ten sam renderer co OBS. **Etap 2 (opcjonalny):** osobna „powłoka” w Rust (`wry`/`tao`) jako oddzielny crate, jeśli potrzebne będą menu systemowe, zasobnik, globalne skróty. | E1: mały; E2: średni + wyjątek od zasady zero zależności |
| Trio jako natywna aplikacja | To samo co Artist (`vizrs trio`) + **tryb klienta**: `vizrs trio --connect https://cg.example.pl` — okno Trio sterujące serwerem na VPS. | mały (po Artist) |
| Trio jako serwer bez GUI na VPS / w Dockerze | VizRS **już jest** serwerem bez GUI (UI to strony serwowane z binarki). Brakuje: konfiguracji przez zmienne środowiskowe, obrazu Docker (statyczna binarka musl, `FROM scratch`, ~3–5 MB), healthchecku bez curla, HTTPS przez reverse proxy (Caddy), osobnego tokenu tylko do wyjścia, utwardzenia na publiczny Internet. Webowe GUI zostaje. | mały–średni |

**Nie rekomenduję** przepisywania Artist/Trio na natywny toolkit (egui jak btb-cgeditor) ani Electrona (jak BTB CG System) — uzasadnienie w §3.

## 1. Stan dziś (z kodu)

- Jedna binarka `vizrs` bez zależności: własny HTTP/1.1, SSE, JSON, OSC; UI wbudowane (`include_str!`, tablica `STATIC` w `source/src/main.rs`).
- Rust nie ma żadnego GUI. Artist, Trio i Output to strony w przeglądarce; wszystkie trzy rysuje **jeden** renderer (`web/renderer.js`, `ScenePlayer`). Wyjście do OBS to strona `/output` w CEF (Chromium) wbudowanym w OBS.
- Opcje serwera: `--port`, `--tcp`, `--osc`, `--bind` (domyślnie `127.0.0.1`), `--token` / `VIZRS_TOKEN`, `--webhook`, `--data`, `--web`. Tylko token ma zmienną środowiskową.
- Bezpieczeństwo: token (nagłówek / `?token=` → ciasteczko), ochrona CSRF przez `Sec-Fetch-Site` i porównanie `Origin` z `Host` (`guard.rs`). Brak TLS (LUKI G1 #1).
- Niezawodność: zapis show tmp + fsync + rename (`engine.rs`), kopie rotacyjne (`backup.rs`), `/health`, odłączanie wolnych klientów SSE, limit 512 połączeń (wątek na połączenie), timeout odczytu żądania 30 s.
- Wdrożenie: `deploy/vizrs.service` (systemd z utwardzeniem), launchd, NSSM. CI buduje binarki Linux/macOS/Windows.
- Front-end używa ścieżek bezwzględnych (`fetch('/api/…')`, `EventSource('/events')`) — hosting pod podścieżką (`https://host/kanal1/`) wymagałby zmian; subdomeny działają od ręki.
- Wszystkie trzy strony same łączą się ponownie po zerwaniu SSE i dostają snapshot stanu (F1.7) — to dobra podstawa pod pracę przez Internet.

Wniosek: architektura „serwer + cienki klient webowy” to dokładnie to, czego potrzeba do VPS. Do „okna natywnego” brakuje tylko **opakowania**, nie przebudowy.

## 2. Lekcje z BTB

| Projekt | Wybór | Co z tego wynikło | Wniosek dla VizRS |
|---|---|---|---|
| **BTB CG System** (`../BTB-CG-System-alpha`) | Electron; `main.js` robi `require('./server.js')` (Express + Socket.io + SQLite w procesie okna), `nodeIntegration: true`, `contextIsolation: false`; osobno Dockerfile `node:20-slim` z kompilacją sqlite3 | okno „natywne” to i tak strona z `localhost:3000`; paczka ~150–250 MB; pełny dostęp Node z poziomu strony (ryzyko przy każdym obcym HTML/SVG); obraz Docker ciężki i zależny od natywnych modułów | ten sam wzorzec (serwer + strona w oknie) da się uzyskać bez Electrona i bez Node |
| **btb-cgeditor** (`../btb-cgeditor`) | natywny Rust + egui/eframe (wgpu), ~15 crate'ów | edytor rysuje własnym silnikiem, a wyjście to `output.html` w OBS → trzeba było budować i pilnować **parytetu dwóch silników** (`btb-motion` vs. `anim.rs`/`playback.rs` vs. JS) testami zgodności | VizRS ma zasadę „jeden renderer” właśnie po to, by tego uniknąć. Natywny edytor = drugi renderer = powrót problemu |

Najważniejszy argument techniczny: **antena to Chromium** (CEF w OBS; vMix też używa Chromium). Artist pokazuje WYSIWYG tylko wtedy, gdy renderuje tym samym silnikiem: pomiar tekstu, `squeeze`, łamanie „wypełniaj”, kerning fontów, filtry canvas. Dlatego okno Artist powinno być Chromium również na Linuksie.

## 3. Warianty okna natywnego

| Wariant | Jak | Zależności | Wierność z OBS | Rozmiar | Ocena |
|---|---|---|---|---|---|
| **A. Okno aplikacji Chromium** | binarka uruchamia `msedge`/`chrome`/`chromium` z `--app=http://127.0.0.1:PORT/artist --user-data-dir=<profil VizRS> --window-size=…`; brak pasków, własna ikona w pasku zadań, osobny proces | **żadne** (Edge jest w każdym Windows 10/11; na Linuksie Chrome/Chromium/Brave/Vivaldi — fallback: domyślna przeglądarka) | pełna (Chromium) | +0 | **rekomendowany na start** |
| **B. Powłoka `wry` + `tao`** | osobny crate `vizrs-shell` w workspace: prawdziwe okno systemowe z WebView, menu, zasobnik, skróty globalne, wiele okien | ~40–80 crate'ów pośrednio; Linux wymaga WebKitGTK 4.1 | Windows: WebView2 = Chromium ✔; **Linux: WebKitGTK ≠ Chromium ✘** (inne metryki tekstu, inne wsparcie canvas/filters) | +3–8 MB | opcja na później, osobno od rdzenia |
| C. Tauri | B + framework, IPC, auto-update, instalatory | dużo więcej niż B | jak B | +5–10 MB | nadmiar; przejmuje strukturę projektu |
| D. Electron (jak BTB) | Chromium w paczce | npm, Node | pełna | +150–250 MB | sprzeczne z „zero zależności”, ciężkie |
| E. Natywne UI (egui jak btb-cgeditor) | przepisanie Artist i Trio | ~15+ crate'ów, GPU | **drugi renderer** | +10–20 MB | odradzam: miesiące pracy, łamie zasadę „jeden renderer” |

Uwaga do B na Linuksie: dałoby się w powłoce na Linuksie i tak delegować do wariantu A (Chromium `--app`), a WebKitGTK używać tylko dla Trio (Trio nie projektuje, więc różnice metryk mniej bolą — podgląd PVW/PGM byłby jednak wtedy nie 1:1 z anteną).

### Czego wariant A nie daje (i czy to problem)
- **Globalnych skrótów** (TAKE, gdy okno Trio nie ma fokusu) — w studiu robi to Companion/Stream Deck przez OSC/TCP/HTTP, co już działa (`docs/COMPANION.md`).
- **Menu systemowego / zasobnika** — UI ma własne menu; zasobnik nie jest potrzebny, gdy serwer żyje razem z oknem.
- **Natywnych okien dialogowych plików** — `<input type=file>` i pobieranie w Chromium otwierają systemowe okna dialogowe, więc w praktyce są natywne.
Jeśli któraś z tych rzeczy okaże się konieczna, to sygnał do etapu B.

## 4. Architektura docelowa

```
                ┌─────────────────────────── jedna binarka vizrs ───────────────────────────┐
                │  vizrs serve      – serwer bez GUI (dzisiejsze zachowanie, domyślne)      │
                │  vizrs artist     – serwer lokalny w tle + okno Artist                     │
                │  vizrs trio       – serwer lokalny w tle + okno Trio                       │
                │  vizrs trio  --connect https://cg.vps.pl  – tylko okno, serwer zdalny      │
                │  vizrs --healthcheck – dla Dockera (bez curla)                             │
                └────────────────────────────────────────────────────────────────────────────┘

  Laptop grafika            Studio / reżyserka                    VPS (Docker)
  ┌──────────────┐          ┌────────────────────┐               ┌──────────────────────────┐
  │ vizrs artist │──HTTPS──▶│                    │               │ Caddy :443 (Let's Encrypt)│
  │ (okno)       │          │ vizrs trio         │──HTTPS──────▶ │   └▶ vizrs serve :8088    │
  └──────────────┘          │  --connect (okno)  │               │       wolumen /data       │
                            │ Companion ─ HTTP ──┼──HTTPS──────▶ │                          │
                            │ OBS /output?token= ┼──HTTPS (SSE)─▶│                          │
                            └────────────────────┘               └──────────────────────────┘
```

Zasady:
1. **Bez podkomendy** binarka działa jak dziś (zgodność ze `start-linux.sh`, systemd, NSSM, dokumentacją).
2. **Jeden serwer na katalog danych.** `vizrs artist` i `vizrs trio` uruchomione lokalnie muszą dzielić serwer. Plik blokady `vizrs-data/.vizrs.lock` (PID, port, token sesji): druga instancja na tym samym katalogu nie startuje drugiego silnika, tylko otwiera okno na istniejący. To przy okazji chroni przed dwoma serwerami piszącymi do jednego `show.json` (dziś możliwe).
3. **Serwer lokalny okna** nasłuchuje na `127.0.0.1`, port wolny (lub stały z ustawień — OBS musi znać adres `/output`, więc domyślnie 8088), token sesji losowy, przekazany oknu przez `?token=` (ciasteczko w profilu).
4. **Cykl życia:** dzięki osobnemu `--user-data-dir` Chromium nie przekazuje okna do już działającej przeglądarki, więc proces okna żyje do jego zamknięcia. Zamknięcie ostatniego okna → serwer kończy pracę **po potwierdzeniu, jeśli coś jest na antenie** (pytanie w UI przez `beforeunload` + flaga w API), albo zostaje w tle (opcja „serwer działa dalej”). To decyzja dla użytkownika (§9).
5. **Windows bez czarnej konsoli:** drugi `[[bin]]` w tym samym crate, np. `vizrs-app.exe` z `#![windows_subsystem = "windows"]`, wspólny kod w `lib.rs`. Konsolowe `vizrs.exe` zostaje dla serwera i skryptów. Zero nowych zależności.
6. **Połączenia zdalne (Trio):** `--connect URL [--token …]` albo lista w `~/.config/vizrs/connections.json` (Windows: `%APPDATA%\VizRS\`) i prosty ekran wyboru na stronie startowej okna. Token zapisany w profilu okna, nie w pliku tekstowym, jeśli to możliwe.

## 5. Tryb serwera: VPS i Docker

### 5.1 Konfiguracja (12-factor)
Zmienne środowiskowe dla każdej opcji: `VIZRS_PORT`, `VIZRS_BIND`, `VIZRS_DATA`, `VIZRS_TCP`, `VIZRS_OSC`, `VIZRS_WEBHOOK`, `VIZRS_TOKEN` (jest), nowe `VIZRS_OUTPUT_TOKEN`. Flagi CLI mają pierwszeństwo. W kontenerze domyślnie `--bind 0.0.0.0 --data /data`, a start **bez tokenu przy bind ≠ 127.0.0.1 kończy się błędem** (lub wymaga jawnego `--insecure`).

### 5.2 Obraz
```dockerfile
FROM rust:1-alpine AS build          # musl → statyczna binarka, zero zależności = zero problemów
WORKDIR /src
COPY source/ .
RUN cargo build --release --locked

FROM scratch
COPY --from=build /src/target/release/vizrs /vizrs
USER 10001:10001
VOLUME /data
EXPOSE 8088
ENV VIZRS_BIND=0.0.0.0 VIZRS_DATA=/data VIZRS_TCP=0
HEALTHCHECK --interval=15s --timeout=3s CMD ["/vizrs", "--healthcheck"]
ENTRYPOINT ["/vizrs"]
```
- Obraz ~3–5 MB, bez powłoki i bez pakietów → minimalna powierzchnia ataku.
- Multi-arch `linux/amd64` + `linux/arm64` (tanie VPS ARM, np. Hetzner CAX, Oracle Ampere, Raspberry Pi w studiu) — przy zero zależnościach kompilacja krzyżowa jest trywialna.
- `--healthcheck`: binarka sama robi `GET http://127.0.0.1:$PORT/health` (własny klient HTTP to kilkadziesiąt linii) — w `scratch` nie ma curla.
- Fonty są wbudowane (`fonts_data.rs`) — obraz nie potrzebuje fontów systemowych.

### 5.3 Sygnały i zamykanie
W kontenerze proces ma PID 1, a PID 1 bez własnego handlera **ignoruje SIGTERM** → `docker stop` czeka 10 s i zabija. Zapis show jest atomowy przy każdej zmianie, więc danych nie stracimy, ale:
- krótko: w `docker-compose.yml` `init: true` (tini) — zero kodu;
- docelowo: obsługa SIGTERM/SIGINT bez crate'ów (`extern "C" fn signal` na unix, `SetConsoleCtrlHandler` na Windows) → domknięcie logu audytowego, komunikat „serwer kończy pracę” do klientów SSE (Trio pokaże stan zamiast „zerwane połączenie”).

### 5.4 HTTPS i reverse proxy (zamyka praktycznie LUKI G1 #1)
`docker-compose.yml` z dwiema usługami: `vizrs` (bez publikowanego portu) + **Caddy** (automatyczny certyfikat Let's Encrypt, SSE działa bez dodatkowej konfiguracji).
```
cg.example.pl {
    reverse_proxy vizrs:8088
}
```
Dla nginx w dokumentacji: `proxy_buffering off; proxy_read_timeout 1h; proxy_set_header Host $host;` — bez zachowania `Host` odrzuci żądania kontrola `Origin` w `guard.rs`. Opcja `--trust-proxy` (adres IP klienta z `X-Forwarded-For` do logu audytowego i limitów).
Własny TLS w binarce (bez crate'ów) **odradzam** — implementacja kryptografii od zera to ryzyko większe niż korzyść.

### 5.5 Uprawnienia: osobny token wyjścia
Dziś jeden token = pełne sterowanie. Na VPS adres `/output?token=…` trafia do kolekcji scen OBS, vMixa, do realizatorów zewnętrznych — wyciek oznacza, że ktoś może zrobić TAKE/CLEAR albo nadpisać show.
- `--output-token` / `VIZRS_OUTPUT_TOKEN`: tylko odczyt (`/output`, `/events`, `/fonts/*`, `GET /assets/*`, `GET /api/show` w zakresie potrzebnym wyjściu).
- Token operatora: wszystko. Opcjonalnie później trzeci poziom „Trio bez edycji scen” (operator playoutu nie może zmienić szablonu), co odpowiada podziałowi Viz Artist / Viz Trio.
- Rotacja tokenów bez restartu (sygnał/API) — opisana dziś w RUNBOOK jako restart.

### 5.6 Utwardzenie na publiczny Internet
| Problem | Dziś | Propozycja |
|---|---|---|
| Zgadywanie tokenu | brak limitu prób | opóźnienie narastające per IP po błędnym tokenie |
| Wyczerpanie połączeń (slowloris) | 512 globalnie, timeout nagłówków 30 s | limit połączeń per IP (np. 64), timeout nagłówków 10 s; proxy (Caddy) i tak to łagodzi |
| Porty TCP 6100 / OSC (UDP) | tekst jawny, token w linii/adresie | **nie publikować** w Dockerze (domyślnie `VIZRS_TCP=0`); Companion przez HTTPS (generic HTTP) albo przez VPN (WireGuard/Tailscale) do VPS |
| Duże uploady | limity w `validate.rs` / 413 | limit łącznego rozmiaru `assets/` (kwota) |
| Logi | `log.jsonl` + stdout | opcja `--log-stdout` (audyt w formacie JSON na stdout → `docker logs`, Loki itp.) |

### 5.7 Antena przez Internet — opóźnienia i odporność
- Ścieżka komendy: Trio → VPS → OBS. Typowo 30–150 ms. Dla grafiki CG akceptowalne, ale **nie jest to synchronizacja co do klatki**; trzeba to jasno napisać w dokumentacji.
- Zerwanie łącza: grafika na wyjściu zostaje (stan jest w przeglądarce OBS), po powrocie snapshot. Do sprawdzenia testem E2E z przerwaniem SSE w trakcie animacji.
- Grafiki i fonty: ładowanie zawczasu (jest dla czcionek paczek) — warto rozszerzyć o obrazy wszystkich stron playlisty (prefetch), żeby TAKE nie czekał na pobranie przez Internet.
- **Propozycja na później: `vizrs relay --upstream https://cg.vps.pl`** — lokalny proces w reżyserce, który subskrybuje stan z VPS, trzyma kopię zasobów i serwuje `/output` lokalnie. OBS nie zależy wtedy od Internetu przy wczytywaniu zasobów, a przy zerwaniu łącza lokalny operator może przejąć (tryb awaryjny). To naturalne rozwinięcie modelu Viz (Trio ↔ wiele silników).

### 5.8 Wiele kanałów / klientów
- Najprościej: **jeden kontener = jeden show/kanał**, osobne wolumeny, Caddy kieruje po subdomenach (`kanal1.cg.pl`, `kanal2.cg.pl`). Bez zmian w kodzie.
- Hosting pod podścieżką (`/kanal1/`) wymaga zamiany ścieżek bezwzględnych w front-endzie na względne lub `<base href>` — osobne zadanie, tylko jeśli subdomeny nie wchodzą w grę.
- Wiele show w jednym procesie — odradzam na tym etapie (silnik, SSE, blokady i log są per show).

### 5.9 Kopie i aktualizacje
- Wolumen `/data` zawiera `show.json`, `assets/`, `backups/`, `log.jsonl`. Kopia: `docker run --rm -v vizrs-data:/data -v $PWD:/b alpine tar czf /b/vizrs.tgz /data`.
- Tagowanie obrazów wersją (`2.1.0`) + `latest`; przed aktualizacją automatyczna kopia show (już robi `backup.rs` przy zmianie — dodać „kopię przy starcie nowej wersji”).
- Publikacja obrazu do GHCR przez CI tylko na tag wydania (push — na Twoją prośbę).

## 6. Wpływ na zasady projektu

| Zasada (CLAUDE.md) | Wariant A + serwer | Wariant B (wry) |
|---|---|---|
| Zero zależności | zachowana | **złamana** — tylko w osobnym crate `source/shell/`; rdzeń `vizrs` dalej bez zależności. Wymaga Twojej zgody i zapisu w CLAUDE.md |
| UI wbudowane w binarkę | bez zmian; okno ładuje te same strony | bez zmian (powłoka ładuje URL serwera) |
| Jeden renderer | zachowana | Windows ✔, Linux WebKitGTK — ryzyko rozjazdu z OBS |
| Dane z zewnątrz wrogie | bez zmian; dochodzą nowe zmienne środowiskowe i plik połączeń → walidacja | jak A |
| Nie wywalaj serwera | dochodzi zarządzanie procesem okna — błędy startu Chromium = komunikat + fallback do przeglądarki, nigdy panika | jak A |
| Dokumentacja razem z kodem | DOKUMENTACJA (CLI, okna), INSTRUKCJA-poczatkujacy (uruchamianie z ikony), RUNBOOK (VPS, Docker, proxy), API (token wyjścia), LUKI (#1 częściowo zamknięta) | jak A |

## 7. Etapy

Każdy etap = osobne commity, bramka testów jak w CLAUDE.md, dokumentacja w tym samym commicie.

### N0 — Fundament CLI (mały)
- Podkomendy `serve` (domyślna), `artist`, `trio`, `--healthcheck`; zmienne `VIZRS_*`; odmowa startu bez tokenu przy `--bind` innym niż loopback.
- Plik blokady katalogu danych (`.vizrs.lock`), wykrywanie żywego serwera (`/health` + token sesji).
- Rozdzielenie `main.rs` na `lib.rs` (serwer) + cienkie `main.rs` — pod drugi `[[bin]]`.
- Testy: `cargo test` dla parsera opcji/zmiennych, blokady (dwie instancje), `--healthcheck` przeciw serwerowi testowemu.
- **Gotowe gdy:** `vizrs` bez argumentów zachowuje się jak dziś; `VIZRS_PORT=8099 vizrs` działa; druga instancja na tym samym `--data` odmawia z czytelnym komunikatem.

### N1 — Docker i VPS (mały)
- `Dockerfile` (musl, `scratch`, użytkownik bez uprawnień, healthcheck), `.dockerignore`, `deploy/docker-compose.yml` (vizrs + Caddy, `init: true`), `deploy/Caddyfile`, przykład nginx.
- RUNBOOK: sekcja „VPS krok po kroku” (DNS → compose up → token → OBS `/output?token=`), kopie, aktualizacja.
- CI: budowa obrazu amd64/arm64 + test dymny (`docker run` → `--healthcheck` → `GET /health` → E2E `run.js` przeciw kontenerowi).
- **Gotowe gdy:** `docker compose up -d` na czystym VPS daje działające Trio/Artist/Output po HTTPS; restart kontenera nie gubi show; `docker stop` kończy się < 2 s.

### N2 — Bezpieczeństwo sieciowe (średni)
- `--output-token` (odczyt), limit prób tokenu, limit połączeń per IP, krótszy timeout nagłówków, `--trust-proxy`, `--log-stdout`, obsługa SIGTERM bez crate'ów.
- Testy jednostkowe `guard.rs` dla ról tokenów; E2E: OBS z tokenem wyjścia nie może `POST /api/take`.
- **Gotowe gdy:** testy ról zielone; LUKI G1 #1 przeredagowana („TLS przez reverse proxy, opisane w RUNBOOK”).

### N3 — Okna aplikacji Artist i Trio (mały–średni)
- Wyszukiwanie Chromium: Windows (Edge z rejestru / `Program Files`), Linux (`chromium`, `chromium-browser`, `google-chrome`, `microsoft-edge`, `brave`, Flatpak/Snap), macOS (`/Applications/...`); zmienna `VIZRS_BROWSER` na nadpisanie; fallback do domyślnej przeglądarki z komunikatem.
- Profil okna w katalogu konfiguracji użytkownika, rozmiar i pozycja okna zapamiętane przez Chromium.
- `vizrs trio --connect URL`, lista połączeń, ekran wyboru.
- Cykl życia procesu (zamknięcie okna, coś na antenie → pytanie).
- Integracja z systemem: `packaging/vizrs-artist.desktop`, `vizrs-trio.desktop` + ikony; Windows: `vizrs-app.exe` (bez konsoli), skróty w menu Start (prosty skrypt instalacyjny PowerShell zamiast instalatora MSI na start); `start-linux.sh`/`start-windows.bat` z wyborem Artist/Trio/serwer.
- Testy: E2E uruchamia `vizrs trio` z `VIZRS_BROWSER` wskazującym Chromium z Playwright i sprawdza, że okno ładuje `/trio` z tokenem; test blokady (Artist + Trio = jeden serwer).
- **Gotowe gdy:** dwuklik w ikonę „VizRS Artist” otwiera okno bez pasków przeglądarki; równoległe „VizRS Trio” korzysta z tego samego serwera; OBS dalej czyta `http://localhost:8088/output`.

### N4 — (opcjonalnie) Powłoka natywna `vizrs-shell` (średni, wymaga decyzji)
- Osobny crate w workspace, `wry` + `tao` (+ `muda` dla menu, `global-hotkey` dla skrótów).
- Funkcje uzasadniające ten etap: globalne skróty Trio (TAKE/CONTINUE/OUT bez fokusu), zasobnik z listą kanałów, wiele okien (podgląd PGM na drugim monitorze, pełny ekran jako wyjście HDMI/SDI przez kartę — „poor man's Viz Engine”), menu systemowe.
- Na Linuksie: Artist nadal przez Chromium `--app` (wierność), Trio może w WebKitGTK.
- **Gotowe gdy:** ta sama funkcjonalność co N3 + wymienione funkcje natywne; rdzeń `vizrs` dalej bez sekcji `[dependencies]`.

### N5 — (opcjonalnie) Relay i kanały
- `vizrs relay --upstream` (§5.7), hosting pod podścieżką (§5.8), prefetch zasobów playlisty.

Kolejność: **N0 → N1 → N2** (serwer na VPS gotowy produkcyjnie) **→ N3** (okna). N0 jest wspólny dla obu ścieżek. N4/N5 po praktyce z N1–N3.

## 8. Ryzyka

| Ryzyko | Skutek | Przeciwdziałanie |
|---|---|---|
| Brak Chromium na Linuksie użytkownika | `vizrs artist` nie otworzy okna aplikacji | fallback do domyślnej przeglądarki + komunikat; dokumentacja (pakiet `chromium`) |
| Polityki firmowe Edge na Windows (GPO blokuje `--app`/profil) | okno nie wstaje | fallback; `VIZRS_BROWSER` |
| Kolizja portu 8088 przy oknie lokalnym | OBS nie trafia w wyjście | stały port z czytelnym błędem; port wypisany w oknie (pasek stanu) |
| Wyciek tokenu z URL wyjścia | obcy steruje anteną | N2: token tylko do odczytu |
| Internet między VPS a OBS | opóźnienie/zerwanie | dokumentacja oczekiwań; relay (N5); snapshot po powrocie |
| Zależności w N4 | rozmycie zasady „zero zależności”, aktualizacje bezpieczeństwa WebView | osobny crate, rdzeń nietknięty, N4 tylko gdy N3 nie wystarcza |
| Dwa serwery na jednym `vizrs-data` (już dziś możliwe) | nadpisywanie show | N0: plik blokady |

## 9. Decyzje dla Ciebie

1. **Wariant okna:** A (Chromium `--app`, zero zależności) teraz, B (wry) później tylko jeśli potrzebne — czy od razu chcesz B (godzisz się na zależności w osobnym crate)?
2. **Zamknięcie okna lokalnego:** kończyć serwer (z pytaniem, gdy coś na antenie) czy zostawiać go w tle?
3. **Kolejność:** najpierw serwer na VPS (N0–N2), potem okna (N3) — czy odwrotnie?
4. **Docker:** obraz `scratch` (najmniejszy, bez powłoki) czy `alpine` (powłoka do diagnostyki w kontenerze)?
5. **Reverse proxy w compose:** Caddy (auto HTTPS, rekomendacja) czy zakładasz własny (nginx/Traefik)?
6. **Role tokenów:** dwie (wyjście / operator) czy od razu trzy (wyjście / Trio / Artist)?
7. **Wiele kanałów:** wystarczy „kontener na kanał” z subdomenami?

## 10. Dodatkowe sugestie

- **Podział ról jak Viz:** na VPS Artist jest „narzędziem projektanta”, a Trio „operatora”. Trzeci poziom tokenu (Trio bez zapisu scen) to tani sposób na to, by realizator w nocnej zmianie nie zmienił przypadkiem szablonu belki.
- **Wyjście bez OBS:** okno `vizrs output --fullscreen --screen 2` (Chromium `--app --kiosk` na wybranym monitorze) daje proste wyjście HDMI do miksera bez OBS — to samo „okno aplikacji” z N3, jedna flaga więcej. Klucz/wypełnienie (`?mode=fill|key`) już istnieje → dwa monitory = Fill + Key do ATEM.
- **Import z BTB na serwerze:** paczki `.cgpack` / `.json` v3 wgrywane przez przeglądarkę działają już dziś przez HTTP — nic nie trzeba zmieniać pod VPS; warto tylko dodać limit rozmiaru paczki w N2.
- **Nie powtarzać `nodeIntegration` z BTB:** w żadnym wariancie okno nie powinno dawać stronie dostępu do systemu plików poza API serwera — cała logika zostaje w Rust, a strona to klient jak każdy inny (to też ułatwia `--connect`).
- **Companion zdalnie:** zamiast wystawiać TCP/OSC na VPS, opisać w `docs/COMPANION.md` konfigurację „Generic HTTP” z tokenem przez HTTPS — działa bez zmian w kodzie.
