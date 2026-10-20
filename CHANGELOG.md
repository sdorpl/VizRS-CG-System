# VizRS – historia zmian

## Niewydane – „Okna i serwer” (plan: `PLAN-natywne-i-serwer.md`)
- **Poprawka: tekst mierzony przed wczytaniem swojej odmiany fontu** (np. „NA ŻYWO” w Barlow Condensed 700 przy pierwszym TAKE po otwarciu wyjścia) dostawał zbędne ściśnięcie fontu zastępczego, gdy przeglądarka nie zgłosiła `loadingdone` przed końcem animacji (zawsze w WebKit/WebKitGTK, w Chromium przy wolnym wczytaniu). Renderer wczytuje teraz font tekstu jawnie (`document.fonts.load`) i mierzy tekst ponownie. Test w `tests/e2e/run.js` (fonty z opóźnieniem, bez `loadingdone`).
- **Konfiguracja przez zmienne środowiskowe** `VIZRS_PORT`, `VIZRS_TCP`, `VIZRS_OSC`, `VIZRS_BIND`, `VIZRS_DATA`, `VIZRS_WEB`, `VIZRS_WEBHOOK`, `VIZRS_INSECURE` (obok `VIZRS_TOKEN`); flaga ma pierwszeństwo. Komenda `serve` (domyślna) i składnia `--opcja=wartość`.
- **`vizrs --healthcheck`**: sprawdza `/health` działającego serwera (kod 0/1) – dla Dockera bez curla i monitoringu.
- **Jeden serwer na katalog danych** (`<data>/.vizrs.lock`): drugi serwer na tym samym `--data` odmawia startu i podaje adres działającego; martwy plik po awarii jest zastępowany. `/health` zwraca `instance`.
- **Zmiana zachowania:** nasłuch na adresie innym niż lokalny bez tokenu kończy start z błędem (świadomie: `--insecure`); nieznana opcja lub zła wartość portu to błąd (kod 2) zamiast cichego pominięcia.
- **Okna aplikacji** `vizrs artist`, `vizrs trio`, `vizrs output`: Artist/Trio/wyjście we własnym oknie Edge/Chrome/Chromium w trybie `--app` (bez pasków, osobny profil, ten sam silnik co OBS) z serwerem w tym samym procesie; kolejne okno dołącza do działającego serwera na tym samym `--data`; po zamknięciu okna serwer działa, dopóki podłączone jest jakiekolwiek okno lub wyjście (`--linger`); `--connect URL` – okno dla serwera zdalnego (VPS); wyjście na monitor/HDMI: `--fullscreen --position X,Y --query mode=key`; `--browser`/`VIZRS_BROWSER`; bez Chromium – przeglądarka systemowa.
- `vizrs-app` (Windows): uruchamianie okien bez okna konsoli, błąd startu w okienku systemowym. Skróty: `deploy/install-desktop-linux.sh` (menu aplikacji, ikona `deploy/vizrs.svg`), `deploy/install-windows-shortcuts.ps1` (menu Start). `start-windows.bat` otwiera okno Trio (`artist`, `serve` jako argument).
- Artist: wskaźnik **połączono / brak połączenia** z serwerem (jak w Trio).
- Test `tests/e2e/app.js` (udawana przeglądarka): adres i token okna, dołączanie do serwera, `--connect`, serwer żyje przy podłączonym OBS i kończy pracę po jego odłączeniu.
- **Token wyjścia** `--output-token` / `VIZRS_OUTPUT_TOKEN`: tylko odczyt dla OBS/vMix (`/output`, zdarzenia, fonty, grafiki, zgłaszanie błędów scen); TAKE, edycja, Trio i Artist → 403. Wyciek adresu wyjścia nie daje sterowania anteną.
- **Ochrona w sieci publicznej:** spowalnianie odpowiedzi po 5 nieudanych próbach tokenu z adresu (do 3 s, wpisy „odmowa” w logu audytowym), limit 128 połączeń z jednego adresu, termin 15 s na cały nagłówek żądania (slowloris), ostrzeżenie przy krótkim tokenie.
- `--trust-proxy` (adres klienta z `X-Forwarded-For` za Caddy/nginx), `--log-stdout` (log audytowy na standardowe wyjście), obsługa **SIGTERM/SIGINT** i zdarzeń konsoli Windows bez zależności (natychmiastowy `docker stop`, zwolnienie blokady katalogu danych). Compose ustawia token wyjścia, `VIZRS_TRUST_PROXY` i `VIZRS_LOG_STDOUT`.
- **Docker i VPS:** `Dockerfile` (statyczna binarka musl na `scratch`, ~3 MB, użytkownik 10001, `HEALTHCHECK` przez `vizrs --healthcheck`, wolumen `/data`, TCP 6100 wyłączony), `deploy/docker-compose.yml` z Caddy (automatyczny HTTPS), `deploy/nginx-vizrs.conf`, `deploy/env.example`; RUNBOOK „VPS i Docker”; job CI `docker` (budowa, healthy, token, restart z zachowaniem show). `VIZRS_GIT_HASH` dla `--version` przy budowie bez `.git`.

## Niewydane – „Ticker” (plan: `PLAN-ticker.md`)
- **Ruch jak w BTB CG – efekty wizualne bez kluczy** (`node.motion`, `web/motion.js`): 22 presety wejścia/wyjścia (m.in. przesłony, maska ukośna, żaluzja, odwrócenia 3D, glitch, rozdzielenie RGB, rosnący pasek, spadanie), 31 efektów tekstu (całość, słowa, znaki, linie, maszyna do pisania z kursorem, szyfrowanie, licznik liczby), 11 pętli uwagi (pulsowanie, bicie serca, połysk, poświata, podkreślenie…), krzywe GSAP (`back.out(1.7)`, `bounce.out`, `elastic.out`, `cubic-bezier`, aliasy `spring`/`overshoot`/`broadcast`). Ten sam renderer w Artist, Trio i OBS; składanie z kluczami jak w BTB.
- Przykład `examples/BTB_PILNE.show.json` przebudowany na nowej scenie z paczki BTB_CG (ruch `node.motion`: zoom elementu z lewego górnego rogu, odsłonięcie tekstu, świecenie paska) z zachowanym układem 1–3 wierszy (`squeeze-lines`, `fill`).
- **Belki BTB dopasowane do VizRS:** paczka `examples/BTB_CG` przebudowana nowym konwerterem (31 scen z `node.motion` zamiast kluczy). Stałe efekty tła kształtów BTB (`motion.bg`: połysk, pulsowanie, świecenie, linie skanowania), punkt odniesienia ruchu (`motion.origin`, ruch elementu z lewego górnego rogu), ruch elementu ze skalą layoutu w wewnętrznej grupie, efekty tekstu z wartościami `cgmotion.js` (maski `inset`, rozmycie 20/14 px, `hide` od lewej, szyfrowanie wg postępu całego tekstu), domyślne czasy efektu tekstu jak w BTB. `btb-visual.js --frames`: klatki 25/50/75% wejścia – 31/31 zgodnych z oryginałem.
- **Prosty tryb sekwencji** (Artist → ≣ Sekwencja, jak „Ruch” w BTB CG Editor): wiersz na element z paskami wejścia, wyjścia, efektu tekstu i pętli; przeciąganie zmienia opóźnienie i czas; ⇶ Kaskada (krok, wyjścia odwrotnie), ↳ Po kolei (`motion.after`), ⧉ Kopiuj ruch na rodzeństwo, ◆ Klucze (presety → klucze, `motionToKeys`), ✕, ⟲ Dopasuj; pole „po węźle” w panelu Ruch.
- **Import BTB przenosi ruch 1:1** (`node.motion` zamiast przybliżenia kluczami): presety IN/OUT warstw i elementu z kierunkiem i krzywą, efekty tekstu (aliasy `blurIn`, `slideReveal`…), pętle uwagi z siłą, „po warstwie” przy wejściu; nieznany typ → `fallback` z BTB. Luka #8 zawężona.
- Artist: grupa **Ruch (jak w BTB CG)** we właściwościach węzła i **⟲ Dopasuj stop i długość**; walidacja `motion` w `validate.rs`; testy `tests/motion.js` i `tests/e2e/motion.js`.
- **Paczka belek `.cgpack`** (Artist → ⇩ Paczka / ⇧ Import, `web/cgpack.js`): wybrane sceny, ich strony z Trio, grafiki i czcionki w jednym pliku ZIP (`manifest.json` + `scenes.json` + `assets/`). Import dodaje sceny jako nowe, pomija pliki już obecne, rozwiązuje kolizje nazw (`nazwa_2.ext` z przepisaniem odnośników) i numerów stron, pokazuje raport. Paczka BTB CG podana do ⇧ Import trafia do importu BTB.
- **Zasoby paczki w scenie:** sekcja w ustawieniach sceny – pliki używane (wykryte) i dołączone ręcznie (`scene.packAssets`, walidacja w `validate.rs`); ＋ Wgraj i dołącz (grafika, wideo, czcionka).
- **Czcionki z serwera na /output:** wyjście ładuje wszystkie czcionki z assets zaraz po otwarciu (i nowe co 30 s) – komputer z OBS nie potrzebuje zainstalowanej czcionki, pierwszy TAKE nie czeka na font.
- **Łamanie wierszy „wypełniaj”** (`lineBreak: fill`, próg `fillSqueeze`, domyślnie 0,9) w `squeeze-lines`/`squeeze-each`: wiersz bierze kolejne słowa do progu ściśnięcia, reszta przechodzi niżej, każdy wiersz ściskany osobno – jak belki TVP. Domyślnie bez zmian (`balance`, wiersze równej szerokości). Walidacja obu właściwości.
- Przykład `examples/BTB_PILNE.show.json` (strony 8501–8506): „Belka - PILNE” z paczki BTB_CG jako belka 1–3-wierszowa z łamaniem `fill`, teksty z belek TVP i TV Republika.
- `source/tools/gallery.js`: galeria wszystkich stron zestawów przykładowych (zrzuty, arkusze, `index.html`, poza repo).
- Poprawka: automatyczne łamanie wierszy nie dzieli tekstu na twardej spacji (polska typografia „i\u00a0w\u00a0z…”) – wcześniej jednoliterowy spójnik mógł zostać sam na końcu wiersza.
- **Przepełnienie tekstu widoczne wszędzie** (zamknięta luka #17): Artist pokazuje ściśnięcie zaznaczonego tekstu pod „min. ściśnięcie” i czerwoną ramkę przepełnionego tekstu na kanwie (`data-over`, jak `data-cg-overflow` w BTB); wyjście programu zgłasza przepełnienie do `/api/errors` („⚠ błędy scen” w Trio).
- **Tryb dopasowania `squeeze-each`:** wiersze wpisuje operator (Enter, do `maxLines`), każdy wiersz ściskany osobno do `squeezeMin` bez zmiany fontu – belka dwuwierszowa z różną szerokością liter i tą samą wysokością; wskaźnik `w1/w2` i ostrzeżenie w Trio.
- Paczka `examples/BTB_CG`: `Belka_-_PILNE.json` i `.cgpack` dostawały to samo id sceny (strony 8003 i 8004 wskazywały jedną scenę) – scena z `.json` ma teraz id `btb_belka_-_pilne_json`; `build-btb-pack.mjs` dopisuje rozszerzenie przy kolizji.
- **Fonty z assets wczytują się ponownie:** font wgrany do assets już po otwarciu Trio/Artist/wyjścia (albo chwilowo niedostępny) jest ponawiany co 2–30 s i teksty przeliczają się po jego wczytaniu; wcześniej zostawał font zastępczy (inne łamanie wierszy, inne ściśnięcie) aż do odświeżenia strony – także na wyjściu OBS.
- **Ściskanie tekstu jak w BTB CG:** `textFit: squeeze` ściska tylko poziomo, do granicy `squeezeMin` (wcześniej na sztywno 62% i potem mniejszy font); font się nie zmienia, za długi tekst wystaje, a Trio ostrzega „⚠ nie mieści się”. Dawne zachowanie – nowy tryb `squeeze-shrink` (w scenach demo dla wąskich pól: „NA ŻYWO”, etykiety, programy). Nowy tekst w Artist ma `squeeze` (60%).
- Trio: wskaźnik `ściśnięcie: N%` także dla trybów `squeeze`/`squeeze-shrink`; edytor wiadomości tickera oznacza ⚠ wiadomości, które zostaną ucięte.
- Ticker (`roll`/`wipe`/`whip`): dopasowanie wiadomości wg `textFit` i `squeezeMin` węzła (domyślnie `squeeze-shrink` 60% – jak dotąd).
- Import BTB: brak granicy ściśnięcia → bez granicy (jak migracja `upgrade-minsquash` w BTB; wcześniej 0,8); BADGE z granicą 0,5; tekst z `autoW` → `squeeze` w `maxW`; warstwy tekstu bez pola → pola `tekst1`, `tekst2`… z szerokością do krawędzi tła (paczka `examples/BTB_CG` przebudowana: belki „Belka glowna”, „Belka pilne”, „Wizytówka” edytowalne w Trio).
- Walidacja: `props.textFit` (lista trybów) i `props.squeezeMin` (0–1); Artist przycina liczby do zakresu właściwości.
- **Dokumentacja:** `docs/DOKUMENTACJA.md` (pełny opis aplikacji), `docs/INSTRUKCJA-poczatkujacy.md` (instrukcja krok po kroku), `docs/README.md` (spis), `CLAUDE.md` (zasady pracy z kodem).
- Trio: duży podgląd strony sam kadruje element – środek na elemencie, mały element powiększony tak, by zmieścił się cały; dwuklik w puste miejsce wraca do tego kadru.
- **Edytor wiadomości tickera** w Trio i Artist (`ticker-items.js`): lista *kategoria + treść* z ↑↓ i ✕, widok grup po kategoriach (przeciąganie między grupami, zmiana nazwy grupy), tekst surowy; Enter = nowa wiadomość, Ctrl+↑↓ = przesuń, wklejenie wielu wierszy = wiele wiadomości; najechanie na wiersz pokazuje tę wiadomość w podglądzie; ściśnięcie każdej wiadomości widoczne od razu.
- **Ciągłość przy UPDATE**: zmiana wiadomości na antenie nie zaczyna przewijania od nowa – crawl płynie dalej, a nowe wiadomości dochodzą za tym, co jest na ekranie; w roll/wipe/whip bieżąca wiadomość dogrywa swój czas.
- **Tryb `whip`**: etykieta (pas kategorii) przejeżdża przez pasek między wiadomościami i w połowie drogi zmienia się na kategorię następnej (jak w BTB).
- Nowe właściwości węzła Ticker: `catColors` (mapa `PILNE=#e00000; INFO=#0050c0`), `label` (etykieta domyślna; w crawl – stała etykieta po lewej), `sepStyle` (tekst, kropka, kwadrat, kreska, skos), `gleam` (błysk etykiety co N s).
- Reguły pola tickera (Artist ⚙): maks. wiadomości, maks. znaków wiadomości, lista dozwolonych kategorii, WERSALIKI, typografia, tylko do odczytu.
- Artist: panel węzła Ticker pokazuje tylko właściwości bieżącego trybu; presety „Ticker pełny ekran”, „Pasek z wiperem”, „Crawl z linią akcentu”, „Ticker boczny” w menu ＋ Dodaj.
- Sterowanie: `TICKER <strona[:pole]> LIST|ADD|SET|DEL|CLEAR` (TCP), `/api/ticker/<strona>[/add|/set/<n>|/del/<n>|/clear]` (HTTP), `/vizrs/ticker …` (OSC) – zmiana zapisuje stronę i aktualizuje antenę.
- BTB: import tickerów z kategoriami, kolorami kategorii, trybem (`horizontal`→crawl, `vertical`→roll, `whip`), etykietą wipera i błyskiem; szablony kodowe przekształcone w scenę (np. `ticker-unified`) odtwarzane jako węzeł ticker zamiast statycznego tekstu.
- Usunięto eksport sceny VizRS do formatu BTB CG (przycisk „⇩ BTB”, `exportScene`, `toBtbHtml`): BTB CG jest bazą pomysłów, grafiki przenosimy tylko z BTB do VizRS.
- **Paczka `examples/BTB_CG`**: 31 wzorców BTB CG (belki, plakietki, tickery, zegary, logo) skonwertowanych osobno, strony 8001–8402 na warstwach `main`/`badge`/`ticker`/`clock`/`logo`, font Bahnschrift i logo w `assets/`; budowa `tools/build-btb-pack.mjs`, porównanie wizualne z wyjściem BTB CG `tests/e2e/btb-visual.js` (31/31 zgodnych, znane różnice w `examples/BTB_CG/README.md`).
- Import BTB: format zegara `HH:MM` (minuty w BTB) → `HH:mm`, formaty rozszerzone `YYYY/DD/MM`; ściskanie tekstu BADGE do szerokości tła; pole tickera tylko dla elementów `TICKER`; obrazy `https://` zachowują pełny adres; warstwa ticker BTB bez kategorii, z odstępem separatora z `gap`; rozmiar i grubość etykiety z tekstu wipera.
- Ticker: `crawlFrom` (`right`/`left` – taśma od razu pełna jak w BTB), `labelSize`, `labelWeight` (etykieta/wiper).
- Walidacja: limity tickera (200 wiadomości × 500 znaków), `catColors`, `fieldMeta.maxItems`/`cats`; wspólne wektory zapisu wiadomości JS ↔ Rust (`tests/vectors/ticker.json`).
- Trio: zoom dużego podglądu (kółko, dwuklik w element, przeciąganie), wybór tła monitora PGM (szachownica, czarne, białe, kluczowanie, własny kolor); jeden monitor PGM zamiast PGM+PRV.

## 2.0.0 (2026-10-19) – „Studio”
- **Powiązania (dock)**: reguły `layer:`/`page:`/`scene:` przesuwają scenę, gdy wskazana grafika jest na antenie; nadpisanie operatora (przycisk ⇅ w Trio); stan odtwarzany po odświeżeniu wyjścia.
- **Artist**: markery na osi (M, dwuklik; → stop point), zaznaczanie kluczy (Shift, ramka), kopiuj/wklej klucze (Ctrl+C/V), zoom Ctrl+kółko i „dopasuj”, przeciąganie w drzewie (zmiana rodzica z zachowaniem położenia), biblioteka presetów animacji (zapis, zastosowanie, import/eksport).
- **BTB CG**: import elementów (`.json` v3, `.cgpack`) jako scena + strona z raportem konwersji; wspólny model tekstu i wektory testowe (`tools/sync-btb.sh`).
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
