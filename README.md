# VizRS — grafika CG na żywo dla OBS Studio (2.0)

Lekki klon duetu **Viz Artist** (projektowanie szablonów) + **Viz Trio** (playout stron),
napisany w Rust. Odpowiedniki pojęć Vizrt: [DOKUMENTACJA §1](docs/DOKUMENTACJA.md#1-czym-jest-vizrs). Grafika renderuje się jako przezroczysta strona 1920×1080, którą
OBS (albo vMix, Tricaster, ATEM przez Fill/Key) pobiera przez źródło **Przeglądarka (Browser Source)**.

- Jeden plik wykonywalny, **zero zależności** (własny HTTP/1.1, SSE, JSON, OSC) — kompiluje się i działa offline, z wbudowanymi fontami.
- UI (Trio, wyjście; zamrożony Artist) jest wbudowane w binarkę. Opcjonalne okno natywne `vizrs-shell` (Trio, wyjście) to osobny program.
- Projektowanie scen: zainstaluj osobny program **[VizRS CG Artist](https://github.com/sdorpl/VizRS-CG-Artist/releases)** (`vizrs-cg-artist`, Tauri 2: Linux .deb/.rpm/AppImage, Windows .msi/instalator, macOS .dmg; działa bez serwera, sceny wysyła na serwer vizrs).
- Show w `vizrs-data/show.json`, pliki graficzne w `vizrs-data/assets/`, kopie zapasowe w `vizrs-data/backups/`.

## Pakiet VizRS

VizRS CG System to jedna z czterech aplikacji pakietu VizRS, które mają być ze sobą **zawsze zgodne**: **VizRS CG System** (to repo; silnik grafiki, Trio, `/output`), [VizRS CG Playout](../VirtRS%20CG%20Playout) (grafika i sloty na żywo na kanałach ErsatzTV), [VizRS Live](../VizRS%20Live) (panel kanału nad ErsatzTV legacy) i [VizRS CG Artist](../VizRS%20CG%20Artist) (edytor grafik i animacji). Styki i zasady zgodności: [DOKUMENTACJA §1](docs/DOKUMENTACJA.md#pakiet-vizrs--zgodność-aplikacji).

## Uruchomienie

```bash
./start-linux.sh                          # gotowa binarka bin/linux-x86_64/vizrs, dane w ./vizrs-data
cd source && cargo build --release        # budowa ze źródeł
./target/release/vizrs                    # serwer; Windows: target\release\vizrs.exe
./target/release/vizrs trio               # okno aplikacji Trio (Edge/Chrome/Chromium bez pasków) + serwer
./target/release/vizrs trio --connect https://cg.example.pl --token …   # okno dla serwera na VPS
```

Paczki `.deb` i AppImage (`VizRS-Trio-…AppImage`), binarki oraz obraz Docker są w [wydaniach na GitHub](https://github.com/sdorpl/VizRS-CG-System/releases).

| Adres | Co to jest |
|---|---|
| http://localhost:8088/ | strona startowa, podłączenie OBS, skrót API |
| http://localhost:8088/artist | **Artist** – zamrożony zapas w przeglądarce (główny edytor: [VizRS CG Artist](https://github.com/sdorpl/VizRS-CG-Artist/releases)) |
| http://localhost:8088/trio | **Trio** – playout |
| http://localhost:8088/mobile | **Trio na telefonie** – sterowanie anteną z ekranu dotykowego |
| http://localhost:8088/output | **wyjście dla OBS** (przezroczyste) |

W OBS: Źródła → **+** → **Przeglądarka**, URL `http://localhost:8088/output`, 1920 × 1080.
Opcje (`--port`, `--bind`, `--token`, `--data`…, zmienne `VIZRS_*`): `vizrs --help` i [DOKUMENTACJA §13](docs/DOKUMENTACJA.md#13-uruchamianie-opcje-bezpieczeństwo).
Domyślnie serwer słucha tylko na `127.0.0.1`; w sieci wymaga tokenu.

## Dokumentacja

| Dokument | Dla kogo |
|---|---|
| [Instrukcja dla początkujących](docs/INSTRUKCJA-poczatkujacy.md) | nowy operator / grafik – pierwsza grafika na antenie w 5 minut |
| [Pełna dokumentacja](docs/DOKUMENTACJA.md) | wszyscy – model, Artist, Trio, wyjście, tekst, ticker, opcje, dane |
| [API](docs/API.md) · [Companion](docs/COMPANION.md) | integratorzy, realizatorzy (HTTP, TCP, OSC, webhook, Stream Deck) |
| [Runbook](docs/RUNBOOK.md) | dyżur techniczny – awarie, VPS/Docker, Render.com, lista kontrolna przed emisją |
| [Paczki grafik](examples/README.md) | gotowe oprawy INFO24, R7 NEWS, BTB (`.cgpack`) |
| [Historia zmian](CHANGELOG.md) | wszyscy |
| [Spis wszystkich dokumentów](docs/README.md) | w tym stan prac, znane braki, plany |

Programiści i agenci: [CLAUDE.md](CLAUDE.md) (zasady, struktura, testy, commity).
