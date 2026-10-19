# VizRS – historia zmian

## Niewydane – „Ticker” (plan: `PLAN-ticker.md`)
- **Edytor wiadomości tickera** w Trio i Artist (`ticker-items.js`): lista *kategoria + treść* z ↑↓ i ✕, widok grup po kategoriach (przeciąganie między grupami, zmiana nazwy grupy), tekst surowy; Enter = nowa wiadomość, Ctrl+↑↓ = przesuń, wklejenie wielu wierszy = wiele wiadomości; najechanie na wiersz pokazuje tę wiadomość w podglądzie; ściśnięcie każdej wiadomości widoczne od razu.
- **Ciągłość przy UPDATE**: zmiana wiadomości na antenie nie zaczyna przewijania od nowa – crawl płynie dalej, a nowe wiadomości dochodzą za tym, co jest na ekranie; w roll/wipe/whip bieżąca wiadomość dogrywa swój czas.
- **Tryb `whip`**: etykieta (pas kategorii) przejeżdża przez pasek między wiadomościami i w połowie drogi zmienia się na kategorię następnej (jak w BTB).
- Nowe właściwości węzła Ticker: `catColors` (mapa `PILNE=#e00000; INFO=#0050c0`), `label` (etykieta domyślna; w crawl – stała etykieta po lewej), `sepStyle` (tekst, kropka, kwadrat, kreska, skos), `gleam` (błysk etykiety co N s).
- Reguły pola tickera (Artist ⚙): maks. wiadomości, maks. znaków wiadomości, lista dozwolonych kategorii, WERSALIKI, typografia, tylko do odczytu.
- Artist: panel węzła Ticker pokazuje tylko właściwości bieżącego trybu; presety „Ticker pełny ekran”, „Pasek z wiperem”, „Crawl z linią akcentu”, „Ticker boczny” w menu ＋ Dodaj.
- Sterowanie: `TICKER <strona[:pole]> LIST|ADD|SET|DEL|CLEAR` (TCP), `/api/ticker/<strona>[/add|/set/<n>|/del/<n>|/clear]` (HTTP), `/vizrs/ticker …` (OSC) – zmiana zapisuje stronę i aktualizuje antenę.
- BTB: import tickerów z kategoriami, kolorami kategorii, trybem (`horizontal`→crawl, `vertical`→roll, `whip`), etykietą wipera i błyskiem; szablony kodowe przekształcone w scenę (np. `ticker-unified`) odtwarzane jako węzeł ticker zamiast statycznego tekstu; eksport tickera jako element `TICKER` z `items[].category`.
- Walidacja: limity tickera (200 wiadomości × 500 znaków), `catColors`, `fieldMeta.maxItems`/`cats`; wspólne wektory zapisu wiadomości JS ↔ Rust (`tests/vectors/ticker.json`).
- Trio: zoom dużego podglądu (kółko, dwuklik w element, przeciąganie), wybór tła monitora PGM (szachownica, czarne, białe, kluczowanie, własny kolor); jeden monitor PGM zamiast PGM+PRV.

## 2.0.0 (2026-10-19) – „Studio”
- **Powiązania (dock)**: reguły `layer:`/`page:`/`scene:` przesuwają scenę, gdy wskazana grafika jest na antenie; nadpisanie operatora (przycisk ⇅ w Trio); stan odtwarzany po odświeżeniu wyjścia.
- **Artist**: markery na osi (M, dwuklik; → stop point), zaznaczanie kluczy (Shift, ramka), kopiuj/wklej klucze (Ctrl+C/V), zoom Ctrl+kółko i „dopasuj”, przeciąganie w drzewie (zmiana rodzica z zachowaniem położenia), biblioteka presetów animacji (zapis, zastosowanie, import/eksport).
- **BTB CG**: import elementów (`.json` v3, `.cgpack`) jako scena + strona z raportem konwersji; eksport sceny jako element BTB; wspólny model tekstu i wektory testowe (`tools/sync-btb.sh`).
- **Wielu operatorów**: blokady edycji (miękkie, 45 s), znacznik 🔒 w Trio i Artist.
- **Dystrybucja**: `deploy/` (systemd, launchd, Windows/NSSM), `VIZRS_TOKEN`, runbook awarii, `docs/API.md`, `docs/COMPANION.md`.

## 1.4.0 – „Tekst”
- Reguły pól kontrolnych (Artist ⚙): `maxChars`, `softMax`, `maxLines`, jedna linia, WERSALIKI, polska typografia (`CGText.typo`), kolejność Tab, podpowiedź, tylko do odczytu, pasek formatowania, pisownia przeglądarki.
- Tekst sformatowany: `{b} {i} {u} {c=} {sz=}` obok `[[ ]]`; znaczniki przenoszone między wierszami w `squeeze-lines`.
- Word Replace (Ctrl+Spacja, lista w ⚙ Stan, `/api/wordreplace`); klik w element podglądu przenosi do pola.

## 1.3.0 – „Operator”
- Rundown (NEXT/F8, kursor, odegrane, przeciąganie, notatki, CSV), skróty klawiszowe stron, ARM/Preview (`?bus=preview`), Fill/Key (`?mode=fill|key`), makra i presety segmentów, komendy zbiorcze (`TAKE 1001,3001`), OSC, webhook, `/api/variables`.

## 1.2.0 – „Spójny”
- `rev` i konflikty (409), walidacja schematu, odroczone wdrożenie szablonu („Wdróż teraz”), izolacja błędów węzłów i raport do serwera, odtwarzanie po ponownym połączeniu bez migotania, wbudowane fonty offline, `/health`, log audytowy, okno ⚙ Stan, zrzuty wzorcowe Outputu.

## 1.1.0 (2026-10-19) – „Bezpieczny”
Wydanie bezpieczeństwa i niezawodności (plan: `PLAN-rozbudowy.md`, etapy F0 i F1; szczegóły w `docs/STATUS.md`).
- **Domyślnie nasłuch lokalny** (`--bind`, domyślnie 127.0.0.1) – **zmiana zachowania**: dla dostępu z sieci użyj `--bind 0.0.0.0 --token …`.
- `--token`: token dla UI/SSE/API/TCP (`Authorization: Bearer`, `X-Token`, `?token=`, ciasteczko, `AUTH` w TCP).
- Blokada żądań z obcych witryn (CSRF: `Sec-Fetch-Site`, `Origin`); usunięte nagłówki `Access-Control-Allow-*: *`.
- Rotacyjne kopie show (bieżące/godzinowe/dzienne), zapis z `fsync`, błąd zapisu zgłaszany (HTTP 500, znacznik w Trio), `GET /api/backups`, `POST /api/backups/restore/<nazwa>`.
- Odporność serwera: blokady odporne na zatrucie, `catch_unwind` wokół żądań, limit 512 połączeń, ograniczone kolejki SSE (zawieszony klient jest odłączany), limit długości linii nagłówków/TCP.
- Wgrywanie zasobów: lista dozwolonych rozszerzeń, limit 2 GB na katalog assets, nagłówek CSP dla plików.
- Testy: 29 testów jednostkowych/integracyjnych (silnik, API, JSON z testami losowymi, ochrona, kopie), E2E Playwright, CI GitHub Actions.
- `vizrs --version` (wersja + skrót commita).

## 1.0.0 (2026-10-19)
Pierwsze pełne wydanie.

**Silnik i serwer (Rust, zero zależności)**
- HTTP/1.1, Server-Sent Events, JSON – jeden plik wykonywalny, kompilacja offline.
- Warstwy on-air, TAKE / CONTINUE / OUT / UPDATE, stop pointy, karuzela playlist po stronie serwera.
- Odtwarzanie stanu anteny po odświeżeniu OBS, HTTP API (także GET dla Companion/Stream Deck), port komend TCP 6100.

**Artist**
- Drzewo kontenerów: grupa, prostokąt, elipsa, tekst, obraz/wideo, zegar, ticker, kod QR.
- Timeline z keyframe'ami (6 krzywych), stop pointy, podgląd TAKE/CONTINUE/OUT, cofanie/ponawianie, import/eksport scen.
- Pola kontrolne (Control Object), warianty (pola wyboru + „widoczny gdy”), obszary bezpieczne.

**Trio**
- Strony z numerami, wczytywanie numerem, edycja z podglądem, F1–F4, monitor programu, warstwy, playlisty, import/eksport show.
- Wskaźnik ściśnięcia każdego wiersza, warianty jako przyciski.

**Grafika**
- Autofollow, punkty zaczepienia, dopasowanie tekstu: shrink / squeeze / squeeze-lines (każdy wiersz osobno).
- Wyróżnienia `[[…]]`, miganie, rotacja wierszy, odliczanie z dniami, własny generator kodów QR (bez internetu).
- Tickery: crawl (z plakietkami kategorii), roll (góra-dół), wipe (z wycieraczką); kategorie `KAT#kolor|treść`.
- Zaprojektowane animacje OUT dla wszystkich elementów.

**Zestawy**: INFO24 (22 strony) i R7 NEWS (17 stron) – fikcyjne marki.
**Narzędzia**: `tools/out_choreo.py` (generator wyjść), `tools/render_video.py` (render wideo klatka po klatce).
