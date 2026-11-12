# Plany i analizy

Dokumenty projektowe: decyzje, etapy i ich uzasadnienie. Bieżący stan prac jest w [../STATUS.md](../STATUS.md), znane braki w [../LUKI.md](../LUKI.md).

| Dokument | Stan | Zakres |
|---|---|---|
| [PLAN-rozbudowy.md](PLAN-rozbudowy.md) | wdrożony (2.0.0) | etapy F0–F4, założenia wierności Viz Artist / Trio Z1–Z10 |
| [PLAN-ticker.md](PLAN-ticker.md) | wdrożony (2.0.0), poza T5.2 | tickery (T1–T7) |
| [PLAN-natywne-i-serwer.md](PLAN-natywne-i-serwer.md) | N0–N4.1 wdrożone (2.0.1), reszta czeka | okna aplikacji, okno natywne `vizrs-shell`, VPS/Docker |
| [PLAN-gtk4.md](PLAN-gtk4.md) | propozycja (G0–G6), czeka na prototyp G0 | przebudowa okna natywnego na Linuksie na GTK 4 i WebKitGTK 6.0 |
| [PLAN-kanal.md](PLAN-kanal.md) | realizacja w ffplayout porzucona (2026-10-24, repo archiwalne); kanał: VizRS CG Playout (ErsatzTV) | VizRS Playout – branding kanału sterowany harmonogramem (Viz Multichannel) |
| [PLAN-native.md](PLAN-native.md) | N1 w toku (gałąź `render-native`, renderer 0.1) | renderer natywny `vizrs-render` dla playoutu kanału bez GPU: sceny rysowane bez Chromium, braki do Chromium |
| [PLAN-za-chwile.md](PLAN-za-chwile.md) | Z1–Z3 i Z6 wdrożone (Playout 2.7.0, Trio ⏱ kanał); Z4, Z5 i test na stosie czekają | belka „Za chwilę / Następnie” N s przed końcem audycji: planowanie względem końca w VizRS CG Playout (`on_end`, parametry elementu), scena w VizRS |
| [PLAN-lekkie-output.md](PLAN-lekkie-output.md) | L1–L4 wdrożone (2026-10-24), L5 później | lekkie `/output` dla bezgłowego Chromium bez GPU (Ampere A1): `?fps=N`, schodkowe efekty, wskazówki dla scen kanału |
| [PLAN-panel-wlasciwosci.md](PLAN-panel-wlasciwosci.md) | P1–P7 wdrożone (2026-11-07), P8 czeka | prostszy panel Właściwości w Artist: zwijane sekcje, wiersze złożone, tryb podstawowy, widoczność warunkowa, szukanie, zakładki |
| [PLAN-jeden-ruch.md](PLAN-jeden-ruch.md) | propozycja (J1–J6), wariant C wybrany | jeden model ruchu w Artist: presety jako klipy na wspólnej osi czasu z kluczami (LUKI #7) |
| [PLAN-artist-osobny.md](PLAN-artist-osobny.md) | A0, A2–A4, A6, A7 wdrożone (2026-11-07); A1, A5 – do zrobienia | **VizRS CG Artist** – osobne repo i aplikacja (Tauri 2: Linux, Windows, macOS, wydanie 0.3.0 z 2026-11-07): pliki i okno w Ruście, kanwa i panele w JS (walidacja `vizrs-model` i paczki `.cgpack` w Ruście – A1, A5); w VizRS CG System: `vizrs-shell` tylko Trio/wyjście, `/artist` w serwerze zamrożony do 3.0 |
| [PLAN-wyglad-live-i-tauri.md](PLAN-wyglad-live-i-tauri.md) | W0 wdrożony (2026-11-12), W1–W3, T1–T2 do zrobienia | wygląd VizRS Live w Trio i Artist (wspólny rdzeń stylu), jeden frontend www + okno, okno Trio i aplikacja VizRS Live na Tauri 2 |
| [PLAN-ampere-optymalizacja.md](PLAN-ampere-optymalizacja.md) | projekt techniczny (2026-11-09) | optymalizacja VizRS CG System i renderera natywnego `vizrs-render` na Oracle Ampere A1 (ARM64, Neoverse-N1) |
| [PLAN-playout-ampere-optymalizacja.md](PLAN-playout-ampere-optymalizacja.md) | projekt techniczny (2026-11-09) | optymalizacja VizRS CG Playout, wrappera `vizrs-ffmpeg`, filtru `overlay` i koderów pod Ampere A1 |
| [PLAN-live-ampere-optymalizacja.md](PLAN-live-ampere-optymalizacja.md) | projekt techniczny (2026-11-09) | optymalizacja VizRS Live (ErsatzTV legacy + Next), środowiska .NET CLR, profili FFmpeg i skanera mediów na Ampere A1 |

## Plany dalekosiężne

Pomysły na rozwój pakietu VizRS bez decyzji i terminu – kierunek, nie zobowiązanie.

| Dokument | Stan | Zakres |
|---|---|---|
| [PLAN-vizrs-live.md](PLAN-vizrs-live.md) | prototyp 0.1.0 działa (2026-11-06, repo `VizRS Live`): most C# w legacy + panel w Ruście; dalej: edycja pozostałych ekranów, sloty na żywo | **VizRS Live** – nakładka na ErsatzTV legacy z funkcjami jego UI (kanały, media, harmonogramy, EPG), spięta z grafiką VizRS i VizRS CG Playout |

Założenia Z1–Z10 z planu rozbudowy obowiązują przy każdej nowej funkcji.
