# Plany i analizy

Dokumenty projektowe: decyzje, etapy i ich uzasadnienie. Bieżący stan prac jest w [../STATUS.md](../STATUS.md), znane braki w [../LUKI.md](../LUKI.md).

| Dokument | Stan | Zakres |
|---|---|---|
| [RAPORT-analiza-VizRS-vs-BTB.md](RAPORT-analiza-VizRS-vs-BTB.md) | archiwalny (wersja 1.0.0) | test działania, porównanie z BTB CG System i btb-cgeditor, propozycje #1–#26 |
| [PLAN-rozbudowy.md](PLAN-rozbudowy.md) | wdrożony (2.0.0) | etapy F0–F4, założenia wierności Viz Artist / Trio Z1–Z10 |
| [PLAN-ticker.md](PLAN-ticker.md) | wdrożony (2.0.0), poza T5.2 | tickery na wzór BTB CG (T1–T7) |
| [PLAN-natywne-i-serwer.md](PLAN-natywne-i-serwer.md) | N0–N4.1 wdrożone (2.0.1), reszta czeka | okna aplikacji, okno natywne `vizrs-shell`, VPS/Docker |
| [PLAN-kanal.md](PLAN-kanal.md) | realizacja w ffplayout porzucona (2026-10-24, repo archiwalne); kanał: VizRS CG Playout (ErsatzTV) | VizRS Playout – branding kanału sterowany harmonogramem (Viz Multichannel) |
| [PLAN-native.md](PLAN-native.md) | N1 w toku (gałąź `render-native`, renderer 0.1) | renderer natywny `vizrs-render` dla playoutu kanału bez GPU: sceny rysowane bez Chromium, braki do Chromium |
| [PLAN-lekkie-output.md](PLAN-lekkie-output.md) | L1–L4 wdrożone (2026-10-24), L5 później | lekkie `/output` dla bezgłowego Chromium bez GPU (Ampere A1): `?fps=N`, schodkowe efekty, wskazówki dla scen kanału |

Założenia Z1–Z10 z planu rozbudowy obowiązują przy każdej nowej funkcji.
