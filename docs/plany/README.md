# Plany i analizy

Dokumenty projektowe: decyzje, etapy i ich uzasadnienie. Bieżący stan prac jest w [../STATUS.md](../STATUS.md), znane braki w [../LUKI.md](../LUKI.md).

| Dokument | Stan | Zakres |
|---|---|---|
| [RAPORT-analiza-VizRS-vs-BTB.md](RAPORT-analiza-VizRS-vs-BTB.md) | archiwalny (wersja 1.0.0) | test działania, porównanie z BTB CG System i btb-cgeditor, propozycje #1–#26 |
| [PLAN-rozbudowy.md](PLAN-rozbudowy.md) | wdrożony (2.0.0) | etapy F0–F4, założenia wierności Viz Artist / Trio Z1–Z10 |
| [PLAN-ticker.md](PLAN-ticker.md) | wdrożony (2.0.0), poza T5.2 | tickery na wzór BTB CG (T1–T7) |
| [PLAN-natywne-i-serwer.md](PLAN-natywne-i-serwer.md) | N0–N4.1 wdrożone (2.0.1), reszta czeka | okna aplikacji, okno natywne `vizrs-shell`, VPS/Docker |
| [PLAN-gtk4.md](PLAN-gtk4.md) | propozycja (G0–G6), czeka na prototyp G0 | przebudowa okna natywnego na Linuksie na GTK 4 i WebKitGTK 6.0 |
| [PLAN-kanal.md](PLAN-kanal.md) | realizacja w ffplayout porzucona (2026-10-24, repo archiwalne); kanał: VizRS CG Playout (ErsatzTV) | VizRS Playout – branding kanału sterowany harmonogramem (Viz Multichannel) |
| [PLAN-native.md](PLAN-native.md) | N1 w toku (gałąź `render-native`, renderer 0.1) | renderer natywny `vizrs-render` dla playoutu kanału bez GPU: sceny rysowane bez Chromium, braki do Chromium |
| [PLAN-za-chwile.md](PLAN-za-chwile.md) | Z1–Z3 i Z6 wdrożone (Playout 2.7.0, Trio ⏱ kanał); Z4, Z5 i test na stosie czekają | belka „Za chwilę / Następnie” N s przed końcem audycji: planowanie względem końca w VizRS CG Playout (`on_end`, parametry elementu), scena w VizRS |
| [PLAN-lekkie-output.md](PLAN-lekkie-output.md) | L1–L4 wdrożone (2026-10-24), L5 później | lekkie `/output` dla bezgłowego Chromium bez GPU (Ampere A1): `?fps=N`, schodkowe efekty, wskazówki dla scen kanału |

## Plany dalekosiężne

Pomysły na rozwój pakietu VizRS bez decyzji i terminu – kierunek, nie zobowiązanie.

| Dokument | Stan | Zakres |
|---|---|---|
| [PLAN-vizrs-live.md](PLAN-vizrs-live.md) | prototyp 0.1.0 działa (2026-11-06, repo `VizRS Live`): most C# w legacy + panel w Ruście; dalej: edycja pozostałych ekranów, sloty na żywo | **VizRS Live** – nakładka na ErsatzTV legacy z funkcjami jego UI (kanały, media, harmonogramy, EPG), spięta z grafiką VizRS i VizRS CG Playout |

Założenia Z1–Z10 z planu rozbudowy obowiązują przy każdej nowej funkcji.
