# VizRS — grafika CG na żywo dla OBS Studio (2.0)

Lekki klon duetu **Viz Artist** (projektowanie szablonów) + **Viz Trio** (playout stron),
napisany w Rust. Grafika renderuje się jako przezroczysta strona 1920×1080, którą
OBS (albo vMix, Tricaster, ATEM przez Fill/Key) pobiera przez źródło **Przeglądarka (Browser Source)**.

- Jeden plik wykonywalny, **zero zależności** (własny HTTP/1.1, SSE, JSON, OSC) — kompiluje się i działa offline, z wbudowanymi fontami.
- UI (Artist, Trio, wyjście) jest wbudowane w binarkę. Opcjonalne okno natywne `vizrs-shell` to osobny program.
- Show w `vizrs-data/show.json`, pliki graficzne w `vizrs-data/assets/`, kopie zapasowe w `vizrs-data/backups/`.

## Uruchomienie

```bash
./start-linux.sh                          # gotowa binarka bin/linux-x86_64/vizrs, dane w ./vizrs-data
cd source && cargo build --release        # budowa ze źródeł
./target/release/vizrs                    # serwer; Windows: target\release\vizrs.exe
./target/release/vizrs trio               # okno aplikacji Trio (Edge/Chrome/Chromium bez pasków) + serwer
./target/release/vizrs trio --connect https://cg.example.pl --token …   # okno dla serwera na VPS
```

| Adres | Co to jest |
|---|---|
| http://localhost:8088/ | strona startowa + dokumentacja API |
| http://localhost:8088/artist | **Artist** – projektowanie scen |
| http://localhost:8088/trio | **Trio** – playout |
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

## Model (jak w Vizrt)

| Vizrt | VizRS |
|---|---|
| Scena Viz Artist | scena (drzewo kontenerów: grupa, prostokąt, elipsa, tekst, obraz/wideo, zegar, ticker, kod QR) |
| Keyframe'y / director | ścieżki animacji z krzywymi; ruch bez kluczy jak w BTB CG |
| Stop points | TAKE → 1. stop, CONTINUE → kolejny stop, OUT → od ostatniego stopu do końca |
| Control Object / Control Text… | ⛓ pole kontrolne na dowolnej właściwości (tekst, kolor, liczba, obraz…) |
| Template / Page (Trio) | szablon = scena, strona = dane + numer (np. 1001) |
| Viz layers (back/main/front) | warstwy `back` < `main` < `front` + dowolne własne nazwy |
| Update | podmiana danych na antenie bez ponownej animacji |
| Carousel | playlista z czasami, w pętli, odtwarzana przez serwer |
