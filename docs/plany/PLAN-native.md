# PLAN: renderer natywny VizRS (`vizrs-render`, bez Chromium)

> Stan (2026-10-31): **etapy N1 i N2 zrobione** na gałęzi `render-native` (renderer 0.2). Uzasadnienie, odrzucone kierunki
> i decyzje: plan wspólny z playoutem kanału – `VizRS CG Playout`, `docs/plany/native.md`. Opis działania: [DOKUMENTACJA §10.1](../DOKUMENTACJA.md#101-renderer-natywny-vizrs-render).

## Cel

Kanał wypalany (VizRS CG Playout na Oracle Ampere A1, bez GPU) rysuje grafikę za ułamek CPU bezgłowego Chromium,
z tym samym wyglądem i ruchem. OBS, Trio i Artist zostają przy `renderer.js`.

## Zasady (decyzje z planu wspólnego, §8)

1. **Drugi renderer, jedna semantyka.** `source/render/` odtwarza `renderer.js` dla obsłużonego podzbioru; zgodność
   pilnuje `tools/native-compare.js` (strona po stronie z Chromium) i testy jednostkowe (krzywe, `trackValue`,
   dopasowanie tekstu, model tickera, strefy czasowe). Zmiana w `renderer.js` → ta sama zmiana w `source/render/`.
2. **Wyjątek od „zero zależności”** tylko w osobnym workspace `source/render/` (jak `source/shell/`): tiny-skia,
   rustybuzz (z ttf-parser), wuff, serde_json, png, jpeg-decoder – lista w nagłówku `Cargo.toml`. Serwer `vizrs` bez zmian.
3. **Kanał protokołu:** renderer sam czyta `/events`, `/fonts` i `/assets` jak strona `/output` (serwer bez zmian);
   klatki oddaje playoutowi przez stdin/stdout procesu podrzędnego (rekordy RGBA w rozmiarze kanału lub obszaru).
4. **Braki zgłaszane, nie zgadywane:** czego renderer nie umie (`check`, flaga w każdej klatce), to playout rysuje
   Chromium (`renderer: auto`). Grafika nigdy nie znika z powodu renderera natywnego.

## Etapy

| Etap | Zakres | Stan |
|---|---|---|
| N0 | pomiar kosztu | x86 (kontener deweloperski): crawl 1080p25 3,8 ms/klatkę (ok. 9% rdzenia), sam pasek 1920×120 2,2 ms (ok. 5,5%) – `vizrs-render bench`; **pomiar na A1 do zrobienia** |
| N1 | prymitywy, klucze, tekst bez znaczników, zegary, crawl, host warstw, `stream`, `check`, `frame`, `bench`; sidecar: `renderer: chromium\|native\|auto` | **zrobione (0.1)**: INFO24 i R7 – 13/13 stron poniżej 1% różnych pikseli wobec Chromium (poza stronami z `[[ ]]`) |
| N2 | tekst sformatowany (`[[ ]]`, `{b}{i}{u}{c=}{sz=}`), ticker `roll`/`wipe`/`whip`, kod QR | **zrobione (0.2)**: INFO24 16/16 i R7 13/13 scen natywnie, 39 stron (także z tickerami) poniżej 1% różnych pikseli |
| N2+ | wierność: plansze testowe, pomiar szerokości tekstu, błąd klatek ruchu; reguły Blink/Skia (DOKUMENTACJA §10.1) | **zrobione**: oprawy średnio poniżej 0,6/255, ruch – MSE klatek poniżej 5; zostaje wygładzanie krawędzi glifów (FreeType analitycznie, tiny-skia nadpróbkowaniem) |
| N3 | ruch na wzór BTB (`node.motion`: presety IN/OUT, efekty tekstu, pętle, tło) – port `motion.js` | czeka |
| N4 | WebP, efekty specjalne (3D, RGB, glitch), szybsza ścieżka `yuva420p` i prostokąty zmian | czeka |
| N5 | `renderer: auto` domyślnie w `docker-compose.ampere.yml` po tygodniu emisji testowej | czeka |

## Dostosowanie szablonów

Szablony nie muszą się zmieniać – czego renderer nie umie, rysuje Chromium. Żeby kanał szedł natywnie, sceny kanału
dostosowuje się stopniowo (raport: `vizrs-render check --show show.json`):

- **fonty z Google Fonts** – Barlow, Bebas Neue i Roboto Mono (INFO24) są już wbudowane (2026-10-31), bez zmiany
  wyglądu szablonów: INFO24 13/16 scen natywnie (było 9/16). Inny font z Google: wgrać plik do assets i wskazać go
  w węźle albo przejść na font wbudowany (DOKUMENTACJA §4.8);
- **ruch BTB** (`node.motion`, paczki BTB_*) – do etapu N3 te sceny idą przez Chromium; tekst sformatowany, tickery
  `roll`/`wipe`/`whip` i QR są natywne od wersji 0.2.
