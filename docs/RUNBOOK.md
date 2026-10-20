# Runbook awarii i lista kontrolna wdrożenia

## Wyjście milczy (OBS nie pokazuje grafiki)
1. Trio → **⚙ Stan**: czy „Wyjścia” ≥ 1? Jeśli 0 – OBS nie jest połączony: w źródle Przeglądarka kliknij *Odśwież pamięć podręczną bieżącej strony* (po odświeżeniu VizRS odtwarza stan anteny).
2. `GET /health` (status `ok`? `lastEventAgeS` rośnie bez końca = serwer żyje, ale nic się nie dzieje).
3. Czerwony znacznik **„⚠ zapis show nieudany”** → sprawdź miejsce na dysku i prawa do katalogu danych; edycje nie są zapisywane, antena działa dalej.
4. **„⚠ błędy scen: N”** → ⚙ Stan → lista błędów (scena, komunikat). Wadliwy węzeł jest ukryty, reszta gra; popraw scenę w Artist.
5. Zdjęcie wszystkiego: **CLEAR ALL** (Shift+Esc) albo `GET /api/clear`.
6. Restart usługi: `systemctl restart vizrs` (stan anteny odtworzy się po następnym TAKE; show jest na dysku).

## Odtworzenie danych
- ⚙ Stan → „Kopie zapasowe show” → wybierz kopię → **Przywróć** (stan sprzed przywrócenia trafia do najnowszej kopii bieżącej).
- Ręcznie: pliki w `vizrs-data/backups/` (`show-RRRRMMDD-GGMMSS.json` = bieżące, `…-GG.json` = godzinowe, `…RRRRMMDD.json` = dzienne, UTC) – skopiuj jako `vizrs-data/show.json` przy zatrzymanym serwerze.
- Uszkodzony `show.json` przy starcie: serwer zapisuje go jako `show.json.bak` i wczytuje demo.

## Dwóch operatorów
Trio pokazuje „🔒 imię” przy stronie edytowanej przez kogoś innego; zapis cudzej strony wymaga potwierdzenia. Blokada wygasa po 45 s bez aktywności.
Konflikt zapisu (ktoś zapisał wcześniej) → komunikat z wyborem „nadpisz moją wersją / zachowaj zmiany”.

## Rotacja tokenu
Zmień `VIZRS_TOKEN` w usłudze i zrestartuj; w OBS zaktualizuj `?token=` w URL wyjścia; w Companion nagłówek `X-Token`.

## VPS i Docker
Obraz: statyczna binarka na pustym obrazie (`Dockerfile`, ~3 MB, użytkownik 10001, healthcheck `vizrs --healthcheck`). Konfiguracja przez zmienne `VIZRS_*`, dane w wolumenie `/data`.

**Serwer z HTTPS (zalecane na VPS)** – `deploy/docker-compose.yml` (VizRS + Caddy z certyfikatem Let's Encrypt):
1. Rekord DNS `A`/`AAAA` domeny (np. `cg.example.pl`) na adres VPS; porty 80 i 443 otwarte w zaporze.
2. `cd deploy && cp env.example .env` → ustaw `VIZRS_DOMAIN` i `VIZRS_TOKEN` (długi losowy, np. `openssl rand -hex 24`).
3. `docker compose up -d --build` → `docker compose ps` pokazuje `healthy`.
4. Trio: `https://cg.example.pl/trio?token=…` (raz, potem ciasteczko). OBS: `https://cg.example.pl/output?token=…`.

**Sieć lokalna, bez proxy:** `docker run -d --init --name vizrs -p 8088:8088 -e VIZRS_TOKEN=… -v vizrs-data:/data --restart unless-stopped vizrs`.
Bez tokenu kontener nie wystartuje (nasłuch `0.0.0.0`); świadomie bez tokenu: `-e VIZRS_INSECURE=1`.

**Własny nginx zamiast Caddy:** `deploy/nginx-vizrs.conf` – konieczne `proxy_set_header Host $host` (inaczej 403 „niedozwolony Origin”) oraz `proxy_buffering off` dla `/events` (inaczej wyjście dostaje TAKE z opóźnieniem).

**Nie wystawiaj do Internetu** portu komend TCP (6100, w obrazie wyłączony `VIZRS_TCP=0`) ani OSC – tekst jawny. Companion zdalnie: moduł *Generic HTTP* przez HTTPS z nagłówkiem `X-Token`, albo VPN (WireGuard/Tailscale) do VPS.

**Kopie i aktualizacja:**
- Kopia wolumenu: `docker run --rm -v deploy_vizrs-data:/data -v "$PWD":/b alpine tar czf /b/vizrs-data.tgz -C / data` (nazwa wolumenu: `docker volume ls`).
- Aktualizacja: `git pull && docker compose up -d --build` (show i kopie zostają w wolumenie; po starcie sprawdź `/health`).
- Logi: `docker compose logs -f vizrs`; log audytowy komend w `/data/log.jsonl`.

**Katalog danych montowany z hosta** (`-v /srv/vizrs:/data`) musi należeć do UID 10001: `sudo chown -R 10001:10001 /srv/vizrs`.

**Opóźnienie przez Internet:** komenda idzie Trio → VPS → OBS, typowo 30–150 ms – wystarcza dla grafiki CG, ale nie jest to synchronizacja co do klatki. Po zerwaniu łącza grafika na wyjściu zostaje, a po powrocie wyjście odtwarza stan anteny.

**Drugi kanał** = drugi kontener z własnym wolumenem i własną domeną (w `Caddyfile` drugi blok z `reverse_proxy vizrs2:8088`).

## Lista kontrolna przed emisją (OBS / vMix / Tricaster / ATEM)
- [ ] OBS: źródło Przeglądarka `http://host:8088/output?token=…`, 1920×1080, „Odśwież, gdy scena aktywna”.
- [ ] Test TAKE/CONTINUE/OUT każdej używanej strony; test `UPDATE` na antenie.
- [ ] Fonty: ⚙ Stan → „Fonty offline”; własne fonty wgrane do assets (bez Google Fonts na antenie).
- [ ] Key/Fill: dwa źródła – Fill `?mode=fill`, Key `?mode=key`; w mikserze Key = luma/linear key.
- [ ] Companion/OSC: przycisk testowy dla TAKE, OUT i NEXT; `GET /api/variables` zwraca dane.
- [ ] `GET /health` = `ok`; `vizrs-data/backups/` zapełnia się; wolne miejsce na dysku.
- [ ] Preview (`?bus=preview`) w osobnym monitorze operatora; ARM (F6) nie zmienia anteny.
- [ ] Autostart usługi i restart po awarii (`deploy/`); token ustawiony, jeśli `--bind 0.0.0.0` (bez niego serwer nie wystartuje).
- [ ] VPS: HTTPS działa (kłódka w przeglądarce), port 6100/OSC niewystawiony, kopia wolumenu `/data` zaplanowana.
