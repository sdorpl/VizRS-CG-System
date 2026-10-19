# LUKI – znane braki (uszeregowane: niezawodność → wierność → funkcje)

Zamykając pozycję, usuń wiersz i dopisz wersję w `CHANGELOG.md`. Nowe braki dopisuj od razu.

## G1 – niezawodność i bezpieczeństwo
| # | Luka | Uwagi |
|---|---|---|
| 1 | Brak TLS (token w ciasteczku/nagłówku jawnie w sieci) | dla sieci zaufanej; w sieciach współdzielonych użyj reverse proxy z HTTPS |
| 2 | Wątek na połączenie (limit 512, nie pula) | wystarcza dla studia; pula przy większej skali |
| 3 | `PUT /api/show` (import) nadal zastępuje cały show | chronione `rev`/`force`, kopiami i walidacją; brak scalania |
| 4 | Blokady edycji są miękkie (ostrzeżenie + potwierdzenie) | zamierzone – operator może przejąć |
| 5 | CI macOS/Windows nieuruchamiane lokalnie | pierwszy przebieg pokaże ewentualne różnice |

## G2 – wierność
| # | Luka | Uwagi |
|---|---|---|
| 6 | `OUT` zdejmuje warstwę w silniku od razu; animację OUT gra tylko klient | kolejny TAKE w czasie OUT może ją ściąć |
| 7 | Dock tylko przesuwa (dx, dy); brak reguł względem rozmiaru/wysokości celu | BTB ma osie i krawędzie (`layout.dock`) |
| 8 | Konwerter BTB: efekty tekstu (poza reveal/clip), pętle uwagi, maski, `layout.dock`, auto-szerokość `follow` → ostrzeżenia, nie przenoszone | lista w raporcie importu |
| 9 | Eksport do BTB: klucze animacji i formatowanie tekstu nie są przenoszone | ustawia domyślne wejście/wyjście |
| 10 | Pisownia: natywna przeglądarki (lang=pl); brak własnego słownika (sjp.pl/Hunspell) | w CEF (OBS dock) zależy od przeglądarki |
| 11 | Znaczniki tekstu: rozmiar `{sz=}` w % (BTB używa px) – przy konwersji pomijany | |

## G3 – funkcje
| # | Luka | Uwagi |
|---|---|---|
| 12 | Rundown bez harmonogramu czasowego/automatu (zamierzone: tylko ręczne NEXT) | opcja „AUTO” z widocznym stanem i STOP |
| 13 | Brak dedykowanego modułu Companion (jest generyczny HTTP/OSC + zmienne) | |
| 14 | Parenting przez drzewo (zmiana rodzica); brak „pick whip” jak w AE | |
| 15 | Kształty wektorowe, tryby mieszania, ekspresje (wiggle) – poza zakresem | propozycje G, H, K z btb-cgeditor |
