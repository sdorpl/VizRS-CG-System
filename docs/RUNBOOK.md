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

## Lista kontrolna przed emisją (OBS / vMix / Tricaster / ATEM)
- [ ] OBS: źródło Przeglądarka `http://host:8088/output?token=…`, 1920×1080, „Odśwież, gdy scena aktywna”.
- [ ] Test TAKE/CONTINUE/OUT każdej używanej strony; test `UPDATE` na antenie.
- [ ] Fonty: ⚙ Stan → „Fonty offline”; własne fonty wgrane do assets (bez Google Fonts na antenie).
- [ ] Key/Fill: dwa źródła – Fill `?mode=fill`, Key `?mode=key`; w mikserze Key = luma/linear key.
- [ ] Companion/OSC: przycisk testowy dla TAKE, OUT i NEXT; `GET /api/variables` zwraca dane.
- [ ] `GET /health` = `ok`; `vizrs-data/backups/` zapełnia się; wolne miejsce na dysku.
- [ ] Preview (`?bus=preview`) w osobnym monitorze operatora; ARM (F6) nie zmienia anteny.
- [ ] Autostart usługi i restart po awarii (`deploy/`); token ustawiony, jeśli `--bind 0.0.0.0`.
