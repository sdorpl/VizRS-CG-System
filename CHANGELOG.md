# VizRS – historia zmian

## Niewydane

- **Maska warstwy** (`props.mask` – nazwa węzła z tej samej grupy, plan [PLAN-jeden-ruch.md](docs/plany/PLAN-jeden-ruch.md) J5): prostokąt węzła maski (położenie i wymiar w bieżącym czasie, także z kluczy; maska może być niewidoczna) przycina element razem z ruchem presetów – tekst wjeżdża zza krawędzi maski. `renderer.js` i renderer natywny (`scene.rs`) tym samym wzorem (`native-compare.js`: 0,00% różnicy), walidacja `validate.rs` + test. DOKUMENTACJA §4.3.

- **Okno Trio / wyjścia (`vizrs-shell`) – akceleracja sprzętowa na Linuksie z kartą NVIDIA** (sterownik własnościowy): okno wymusza renderer DMA-BUF WebKitGTK (`WEBKIT_FORCE_DMABUF_RENDERER=1`, `__NV_DISABLE_EXPLICIT_SYNC=1`) – dotąd WebKitGTK sam wyłączał akcelerację i strony rysowały się na CPU (animacja 37 → 60 kl./s w oknie połączenia, pełne okno – kilkanaście razy szybciej). `--gpu` / `VIZRS_GPU` przyjmują też `x11` (XWayland) i `system` (bez zmian); zmienne `WEBKIT_*` użytkownika mają pierwszeństwo. Moduł `source/shell/src/gpu.rs` – ten sam w VizRS CG Artist i VizRS Live. DOKUMENTACJA §12.2.

## 26.11.4 (2026-11-13)

- **Powiązania (dock) – wejście przy celu na antenie:** grafika z regułą dock, która wchodzi, gdy cel już jest na antenie (np. „Wydanie specjalne” przy wizytówce), stoi od razu na przesuniętym miejscu, jak w BTB CG. Dotąd w scenach z tekstem albo ruchem dojeżdżała z położenia bazowego (`renderer.js`), a w rendererze natywnym zawsze; to samo przy odświeżeniu wyjścia w rendererze natywnym. Test e2e i test `vizrs-render`. DOKUMENTACJA §4.7, plan [PLAN-ruch-belek.md](docs/plany/PLAN-ruch-belek.md) A0.

## 26.11.3 (2026-11-13)

- **Renderer natywny: miniatury stron** (`vizrs-render thumb`): strona po wejściu, przycięta do miejsca, w którym rysuje, zmniejszona do zadanej szerokości (PNG z przezroczystością) – dla miniatur elementów graficznych w panelu VizRS Live (przez VizRS CG Playout).
- **Okno Trio na Tauri 2** (`vizrs-shell`, zamiast tao + wry + muda + rfd – te same wtyczki co VizRS CG Artist): role i opcje bez zmian (`trio`, `output`, `connect`, `--connect`, `--data`, `--port`, `--token`, `--gpu`, `--probe`), menu, wyjście na monitorze i Fill + Key, pytanie przy zamknięciu, stan anteny w tytule, menu pod prawym przyciskiem, „Zapisz jako”, okno połączenia. Jedna instancja działa teraz także na Windows (wtyczka Tauri; jeden program na użytkownika, niezależnie od `--data`). Błędy argumentów (np. `vizrs-shell artist`) idą tylko na wyjście błędów – bez okienka czekającego na kliknięcie. Strona dostaje most tylko od adresu bieżącego serwera; skróty przeglądarki WebView2 wyłącza `web/native.js`. Nowe: paczki „VizRS Trio” z bundlera Tauri (`.deb`, `.rpm`, AppImage, `.msi`/NSIS, `.dmg`) z serwerem `vizrs` obok okna (DOKUMENTACJA §12.2); dotychczasowe paczki zostają. Wymagany macOS 11+. CI: biblioteki budowy `librsvg2-dev libxdo-dev`.
- **Trio w wyglądzie VizRS Live** (stanowisko, telefon i okno): pasek górny z logo, karty paneli, pigułki stanu z kropką, zakładki podkreślone akcentem, przyciski TAKE / CONTINUE / OUT / UPDATE / ARM w stylu Live (kolory bez zmian), okna pytań i ⚙ Stan jak okna Live, strona logowania w tych samych kolorach. W oknie natywnym Trio pisze czcionką Inter jak w przeglądarce. `ui.css` zawiera już tylko style stanowiska (na rdzeniu `vizrs-ui.css`); zamrożony `/artist` i strona startowa zachowują dawny wygląd (`ui-frozen.css`). Zainstalowana aplikacja Trio (PWA) pobierze nowe pliki sama (nowa wersja pamięci podręcznej).
- **Wspólny rdzeń wyglądu `vizrs-ui.css`** (wzór: panel VizRS Live): tokeny kolorów, promieni i cieni oraz kontrolki (przyciski, pola, pigułki stanu, pasek górny, menu z szufladą, tabele, okna, karty) w jednym pliku `source/web/vizrs-ui.css`, podawanym pod `/vizrs-ui.css`. VizRS Live i VizRS CG Artist trzymają jego identyczne kopie (kontrola w CI). Trio i Artist przejdą na ten wygląd w kolejnych etapach planu `PLAN-wyglad-live-i-tauri.md`.
- **Trio i Trio na telefonie to jedna strona `/trio`:** na wąskim ekranie (do 760 px, także zwężone okno na komputerze) i na telefonie położonym poziomo Trio samo przechodzi w układ dotykowy, wzorowany na panelu VizRS Live – widoki Strony / Edycja / Antena / Kolejka przesuwane palcem, pasek TAKE / CONTINUE / OUT / UPDATE na dole, narzędzia stanowiska (show, kanał, ⚙ Stan, kopie, CLEAR ALL) w szufladzie spod ☰, pytania jako arkusze od dołu. Telefon ma teraz pełne Trio: tworzenie i usuwanie stron, formatowanie, czcionki, edycję playlist, rundownu i makr. Rozwiązania dawnego `/mobile` zostały: blokada podwójnego dotknięcia, wibracja, monitor PGM tylko na widoku Antena (15 kl./s), podgląd tylko na Edycji, klawiatura ekranowa chowa zakładki, niewygaszanie ekranu, ponowne łączenie po uśpieniu, instalacja (PWA) i start bez sieci. `/mobile` przekierowuje na `/trio`, bez tokenu `/trio` pokazuje ekran logowania. Pliki aplikacji: `/trio.webmanifest`, `/trio-sw.js`, `/trio-icon.svg|png` (zamiast `/mobile*`) – aplikację zainstaloną z `/mobile` warto zainstalować od nowa. Także na stanowisku: komenda bez odpowiedzi przez 6 s zgłasza błąd, a usunięty wybrany kanał przełącza na antenę główną. DOKUMENTACJA §9.9.
- **Ticker – kategoria i treść:** po wpisaniu kategorii wiadomości przejście do pola treści (Tab, kliknięcie, dotknięcie) nie gubi już fokusu – edytor przerysowuje się po zmianie fokusu i zostawia kursor w wybranym polu.

## 26.11.2 (2026-11-13)

- **Zaokrąglenie osobno dla rogów i boków** (`rect`, `qr`, `image`): nowe właściwości `radiusTL`/`radiusTR`/`radiusBR`/`radiusBL` (każdy róg) i `radiusT`/`radiusR`/`radiusB`/`radiusL` (każdy bok – oba rogi przy boku; dwa boki przy rogu dają róg eliptyczny). Wartość `-1` (domyślna) = jak `radius`, więc dawne sceny rysują się bez zmian. Renderer natywny rysuje je tak samo (rogi eliptyczne, zmniejszanie jak w CSS, ramka, cień). DOKUMENTACJA §4.3.
- **Renderer natywny: granice stron** (`vizrs-render bounds`): gdzie strona rysuje przez całe życie (wejście, stop pointy, wyjście), w komórkach 16 px – dla stref grafiki VizRS CG Playout (PLAN-strefy S6). DOKUMENTACJA §10.1.

## 26.11.1 (2026-11-12)

Pierwsze wydanie w numeracji rok.miesiąc.rewizja (`26.11.1` = listopad 2026, wydanie 1). Gałąź `main` = dawna `main` (2.1.1) scalona z `next-v`; poprzednia `main` zachowana jako `main-oldstable`.

- **Import i przykłady uproszczone:** ⇧ Import w Artist przyjmuje tylko paczki `.cgpack` VizRS i dawne `.vizrs.json`; odczyt ZIP jest w `web/cgpack.js`. Przykładowe paczki: `INFO24.cgpack` i `R7_NEWS.cgpack`; galeria i film obejmują te dwa zestawy. `text-model.js` jest własnym modułem VizRS (bez synchronizacji z innym repo), wektory `tests/vectors/` – bez zmian. Rozdziały DOKUMENTACJA od „Uruchamianie, opcje, bezpieczeństwo” mają numery o jeden mniejsze (§12–§15). `PLAYWRIGHT_DIR` nie ma już domyślnego katalogu.

- **Jedne zasady przenoszenia grafiki – tylko paczka `.cgpack`:** ⇩ Eksport w Artist zapisuje bieżącą scenę jako `.cgpack` (z grafikami, czcionkami i stronami) zamiast `.vizrs.json` bez plików. Dawne pliki `.vizrs.json` są tylko czytane – przez ten sam import co paczka, z raportem o brakujących plikach. Trio ma **⇧ Import paczki** (sceny, strony i pliki dodawane do bieżącego show), a przyciski `.show.json` nazywają się **⇩ Kopia show / ⇧ Przywróć show** (kopia zapasowa całego show, nie przenoszenie belek). Import jest jeden dla Artist i Trio (`cgpack.importPack`); ponowny import tej samej paczki nie tworzy kolejnych kopii plików (`nazwa_3.ext`…). DOKUMENTACJA §8.7, §9.

- **Czcionki w paczkach `.cgpack` bez duplikatów:** import paczki nie wgrywa drugi raz czcionki, która jest już w assets pod inną nazwą (ta sama treść i rozszerzenie) – scena dostaje istniejącą, a raport to wymienia. Eksport pakuje czcionkę wgraną pod kilkoma nazwami tylko raz. Nazwy plików są poprawiane tak jak na serwerze przed zapisem (dawniej `Moja Czcionka.ttf` z paczki zapisywała się jako `Moja_Czcionka.ttf`, a scena wskazywała nieistniejący plik), a kolizja nazw nie zależy od wielkości liter (Windows, macOS). Reguły: `cgpack.assetPlacer`, testy `node tests/cgpack.mjs`. DOKUMENTACJA §8.7.

- **Lżejsze renderowanie stron w przeglądarce (OBS, Trio, wyjście kanału):** scena po animacji wejścia nie jest liczona od nowa w każdej klatce, tylko gdy obraz może się zmienić (zegar raz na sekundę, ticker `roll`/`wipe` i rotacja `cycle` w czasie przejścia; miganie, pętle i crawl jak dotąd w każdej klatce). Style węzłów są zapisywane tylko przy zmianie wartości (dawniej co klatkę, a ticker `roll`/`wipe` wymuszał 60 przeliczeń stylu na sekundę), a sceny i tickery mają izolowany układ (`contain: strict`). Główny wątek Chromium na `/output`: statyczne belki ok. 1% zamiast 3–5%, zegary 1,1% zamiast 7,5%, ticker `roll` 1,9% zamiast 6%, crawl 6% zamiast 7%. Obraz bez zmian (zrzuty piksel w piksel z poprzednią wersją). DOKUMENTACJA §10.
- **Renderer natywny (`vizrs-render`) rysuje tylko obszar zmiany:** gdy na antenie zmienia się mały element, na przykład taśma pod dużą planszą, renderer rysuje od nowa tylko jego prostokąt (z marginesem rozmycia) zamiast całego kadru. Rekord klatki łata tylko zmienione wiersze, a od‑premnożenie alfy korzysta z tablicy. Na kanale 1080p50 z planszą i taśmą: nowy obraz w 26 ms zamiast 56 ms, 4,6% zamiast 12,9% klatek po czasie, więc VizRS CG Playout nie spada już do 25 kl./s. Protokół `stream` i piksele bez zmian (test klatka po klatce: rysowanie częściowe = pełne). DOKUMENTACJA §10.1.
- **Łaty w protokole `stream` (rodzaj 6, flaga `p`):** odbiorca, który o nie poprosi (VizRS CG Playout z tego cyklu), dostaje przy niezmienionym prostokącie z grafiką tylko zmienione piksele, a nie cały prostokąt. Przewijany crawl pod planszą: 0,6 MB zamiast 4 MB na klatkę, 50 kl./s na kanale. Starszy odbiorca bez `p` dostaje cały prostokąt, jak dotąd.

## 2.1.1 (2026-11-09) – wydanie serwisowe

- **Adres wyjścia dla OBS w oknie natywnym:** okno Trio nie ma paska adresu i samo wybiera wolny port, więc operator nie miał skąd wziąć adresu `/output`. Przycisk **OBS ⧉** nad monitorem PGM (także w przeglądarce) i menu okna **Wyjście → Adres dla OBS…** pokazują gotowy adres (host, port, kanał anteny, przypomnienie o `?token=`) z przyciskiem Kopiuj i ustawieniami źródła Przeglądarka. `/health` podaje `tokenRequired` i `outputToken` (same flagi).

## 2.1.0 (2026-11-09) – „Natywnie”

Naprawiony instalator Windows z 2.0.20 i okno Trio, które mniej przypomina przeglądarkę: pytania aplikacji, menu systemowe, stan anteny w pasku zadań. Okno natywne sprawdzane w CI na prawdziwym Windows (z instalatora) i macOS.

- **Windows: programy bez `VCRUNTIME140.dll`.** `vizrs.exe`, `vizrs-app.exe` i `vizrs-shell.exe` 2.0.20 nie uruchamiały się na Windows 10 bez pakietu Microsoft Visual C++ Redistributable. Teraz biblioteka C jest wkompilowana statycznie (`source/.cargo/config.toml`, `+crt-static`), a CI i akcja Release odrzucają plik `.exe`, który importuje bibliotekę Visual C++.
- **Instalator Windows:** skróty bez wpisanej ścieżki danych – każdy użytkownik pracuje na swoim `%LOCALAPPDATA%\VizRS` (wcześniej `{userappdata}` z chwili instalacji, czyli katalog konta, które instalowało); dane z instalatora 2.0.20 (`%APPDATA%\VizRS\vizrs-data`) zostają w użyciu, dopóki w nowym miejscu nie ma show. „VizRS Serwer” przez `serwer.bat` z tym samym wyborem. Brak WebView2 Runtime → instalator po cichu instaluje go z dołączonego instalatora Microsoftu (pobieranego przez akcję Release), a gdy się nie uda – komunikat.
- **CI – okno natywne na prawdziwym Windows i macOS** (`shell-smoke`, `tests/e2e/native-smoke.js`): Windows buduje instalator, instaluje „dla wszystkich” do `Program Files`, uruchamia Trio jak skrót (bez `--data`), sprawdza połączenie z serwerem, most, okno pytania, położenie danych WebView2, skrót i deinstalację; macOS – okno z budowy.
- **Okno natywne – menu pod prawym przyciskiem:** systemowe menu (Windows, macOS, GTK) na liście stron Trio (TAKE, ARM, CONTINUE / UPDATE / OUT, Duplikuj, Usuń) i na liście warstw anteny (CONTINUE, OUT, dock, Wdróż); komunikat `menu` mostu z limitami w `bridge.rs`.
- **Okno natywne – stan w pasku zadań:** tytuł okna Trio z `NA ANTENIE (N)` i `⚠ BRAK POŁĄCZENIA`; przy utracie połączenia okno w tle miga w pasku zadań / podskakuje w Docku.
- **Poprawka: okno dołączało do serwera, który właśnie się wyłączał** (`--idle-exit`, ponowne otwarcie okna kilka sekund po zamknięciu poprzedniego → „Connection refused”). Okno pyta teraz `/health?attach=1`, co zeruje odliczanie; zwykłe `/health` (monitoring) serwera nie podtrzymuje.
- **Trio – okna pytań aplikacji zamiast okienek przeglądarki:** 24 pytania (usuwanie, CLEAR ALL, zmiana i lista show, warstwa, konflikt zapisu, nazwy playlist, rundownów, makr, kanałów) zadaje nowy moduł `web/dialogs.js` zamiast `confirm()` / `prompt()`, które w oknie natywnym miały w tytule adres serwera. Przyciski nazywają czynność, czerwony oznacza zdjęcie z anteny lub usunięcie, Enter / Esc, skróty Trio wyłączone pod otwartym oknem; menu show to przyciski zamiast wpisywania numeru. Podwójny klik strony z niezapisanymi zmianami robi TAKE dopiero po wyborze „Porzuć zmiany” (wcześniej po Anuluj szła na antenę poprzednia strona).
- **Okno natywne – mniej przeglądarki, więcej aplikacji:** okno pokazuje się z narysowaną stroną (bez białego błysku), ciemny motyw okna, czcionka systemu w interfejsie, bez przeciągania obrazków i „odbijania” strony; Windows: bez skrótów przeglądarki (Ctrl+F/P/R, Alt+←) i podpowiedzi w polach, ciemne kontrolki WebView2; macOS: menu aplikacji VizRS (Usługi, Ukryj, Zakończ), Edycja z systemowymi Kopiuj/Wklej (⌘C/⌘V w polach tekstowych) i menu Okno.
- **Windows – instalator 2.0.20 nie uruchamiał okna Trio:** `vizrs-shell` nie wskazywał WebView2 katalogu danych, więc silnik zakładał go obok programu (`vizrs-shell.exe.WebView2`). Przy instalacji „dla wszystkich użytkowników” (`C:\Program Files\VizRS`) brak prawa zapisu kończył się błędem tworzenia WebView i okno się nie otwierało. Dane WebView2 trafiają teraz do `%LOCALAPPDATA%\VizRS\WebView2`.

## 2.0.20 (2026-11-09) – „Ampere A1 & Trio”

- **Optymalizacja obrazu Docker pod Oracle Ampere A1 (ARM64, Neoverse-N1):**
  - Wieloplatformowy `Dockerfile` kompiluje statyczną binarkę `vizrs` na platformie budującej za pomocą `rust-lld` z dedykowanymi flagami mikroarchitektury: `RUSTFLAGS="-C target-cpu=neoverse-n1 -C opt-level=3"` dla `linux/arm64` (szybki build bez narzutu emulacji QEMU).
  - Wdrożenie planów technicznych optymalizacji toru FFmpeg i renderera natywnego pod bezpłatne instancje OCI Ampere A1 (`PLAN-ampere-optymalizacja.md`, `PLAN-playout-ampere-optymalizacja.md`, `PLAN-live-ampere-optymalizacja.md`).
- **VizRS CG Artist jako osobna aplikacja (krok A7 planu):** projektowanie scen to od teraz osobny program **VizRS CG Artist** (`vizrs-cg-artist`, repozytorium https://github.com/sdorpl/VizRS-CG-Artist, wydanie 0.3.0 z 2026-11-07, Tauri 2: Linux .deb/.rpm/AppImage, Windows .msi/instalator, macOS .dmg). Działa bez serwera (zapis/odczyt plików `.cgpack` i `.show.json`), a gotowe sceny wysyła na serwer vizrs.
- **Okno natywne `vizrs-shell` tylko dla Trio i wyjścia:**
  - Domyślny rodzaj okna to `trio`; `vizrs-shell artist` oraz przekazanie pliku (`.cgpack`, `.vizrs.json`) kończą się komunikatem odsyłającym do VizRS CG Artist i kodem wyjścia 2.
  - Z menu okna usunięto pozycje projektowania scen: Nowe okno Artist, Nowa scena, Zapisz, Importuj…, Eksportuj scenę…, Paczka .cgpack…, całe menu Edycja (Cofnij, Ponów, Duplikuj, Usuń), w menu Widok pozycje kanwy (Dopasuj kanwę, Kanwa 100%, Kanwa do zaznaczenia) oraz całe menu Scena.
  - Zostają menu: Plik (Połącz z serwerem…, Nowe okno Trio, Okno wyjścia (podgląd), Zamknij okno, Zakończ), Widok (Odśwież, Pełny ekran, Powiększ/Pomniejsz/100%, Narzędzia deweloperskie), Wyjście (monitory, Fill+Key) i Pomoc.
  - Most strony (`native.js`): obsługa `state`, `close`, `stay`, `draft` bez zmian; przekazywanie plików do strony z powłoki zostało usunięte. Most woła `window.__vizrsPost`, gdy jest obecny (okno Tauri VizRS CG Artist), inaczej `window.ipc.postMessage` (wry).
  - Test E2E `source/tests/e2e/shell.js` sprawdza okno Trio oraz odrzucenie `artist` (15/15 zaliczone 2026-11-07).
- **`/artist` w serwerze jako zamrożony zapas:** edytor `/artist` serwowany przez serwer vizrs pozostaje jako zamrożony zapas do pracy w przeglądarce (np. na zdalnym VPS bez instalacji) do wydania 3.0; komendy `vizrs artist` (okno aplikacji Chromium) i `start-*.bat/sh artist` otwierają ten zamrożony `/artist`. Nowe funkcje edytora powstają wyłącznie w VizRS CG Artist. Testy `tests/e2e/artist.js` zostają w repozytorium.
- **Paczki instalacyjne i skróty:**
  - **Pakowanie Windows (Inno Setup & launchery):** dodano skrypt Inno Setup `deploy/windows/installer.iss` generujący instalator `VizRS-Trio-<wersja>-windows-x86_64-setup.exe` w GitHub Actions oraz zestaw skryptów startowych `.bat` (Trio, Trio zdalne, Serwer) z ikoną `vizrs.ico` i dokumentacją dołączany do archiwum `.zip`.
  - Pakiet `.deb` (`deploy/deb/build.sh`) bez wpisu „VizRS Artist” i bez typów MIME (usunięto `deploy/vizrs-mime.xml` – skojarzenia `.cgpack`/`.vizrs.json` należą do paczki VizRS CG Artist).
  - AppImage budowany wyłącznie jako `VizRS-Trio-…AppImage` (`deploy/appimage/build.sh`).
  - Skróty na Linuksie (`deploy/install-desktop-linux.sh`) i Windowsie (`deploy/install-windows-shortcuts.ps1`) tworzą tylko wpisy „VizRS Trio” (+ „VizRS Serwer” na Windows).

## 2.0.19 (2026-11-07) – „Porządek”

- **Wydanie:** AppImage budowane lokalnie i dołączane do wydania (akcja Release buduje binarki, `.deb` i obraz Docker).
- **Artist – prostszy panel Właściwości:** zakładki Właściwości / Ruch / Dane, zwijane grupy z licznikiem zmian, tryb Podstawowe / Wszystko, ukrywanie właściwości, które przy obecnych ustawieniach nic nie robią, wiersze złożone (X/Y, szer./wys. z blokadą proporcji, siatka punktu zaczepienia, przyciski wyrównania), oznaczenie zmienionych, animowanych i podpiętych pól, prawy klik = wartość domyślna, szukanie właściwości (`/`). Format show bez zmian.
- **Artist – położenie całej sceny:** pola X/Y sceny (`scene.x`, `scene.y`, sprawdzane w `validate.rs`), przeciąganie pustego miejsca kanwy i strzałki przy zaznaczonej scenie przesuwają wszystko naraz, bez zmiany kluczy animacji i właściwości elementów. Obsługuje to też renderer natywny.

## 2.0.18 (2026-11-06) – „Kłódka”

- **Artist – drzewo sceny:** blokada elementów i grup (🔒, pole `locked`, sprawdzane w `validate.rs`); wymiar grupy dopasowuje się do zawartości (automatycznie po dodaniu elementu, ręcznie ⤢). Poprawka: nowy element nie ginie już wewnątrz zaznaczonego prostokąta (do środka trafia tylko przy zaznaczonej grupie), Kod QR dostaje domyślny rozmiar, nieznany typ węzła (import) nie blokuje dodawania.

## 2.0.17 (2026-11-04) – „Za chwilę w telefonie”

- **Trio na telefonie – czas strony na kanale (⏱ kanał):** w Edycji, pod polami strony, te same ustawienia co w Trio: wejście po starcie albo przed końcem audycji, zejście i koniec (audycji z EPG / pozycji). Zapis w `playout` strony dla VizRS CG Playout.

## 2.0.16 (2026-11-02) – „Za chwilę”

- **Trio – czas strony na kanale (⏱ kanał):** wejście po starcie albo przed końcem audycji, zejście i koniec (audycji z EPG / pozycji), zapisywane w stronie jako `playout`. Odczytuje go VizRS CG Playout 2.7.0 (belka „Za chwilę” przed końcem audycji w ErsatzTV).

## 2.0.15 (2026-11-01) – „Telefon”

- **Trio na telefonie, wygląd i obsługa aplikacji:** pasek zakładek na dole, przesuwanie zakładek palcem, arkusze zamiast okienek przeglądarki, przycisk Wstecz, wibracje, czerwony nagłówek przy zajętej antenie. Nowe: podgląd strony, wiersze wiadomości tickera, pola koloru i obrazu (ze zdjęciem z telefonu), karuzela, ARM, wdrażanie szablonu.
- **Trio na telefonie jako aplikacja (PWA):** manifest, ikona, start bez sieci przez service workera, ekran logowania tokenem; token znika z adresu po wejściu.
- **Trio na telefonie (`/mobile`):** osobna strona na ekran dotykowy: lista stron, zmiana tekstu, TAKE / CONTINUE / OUT / UPDATE, warstwy na antenie z monitorem PGM, CLEAR ALL, rundown z NEXT, makra, wybór anteny. Dotknięcie strony tylko ją wybiera, a podwójne dotknięcie TAKE gra raz. Reguły pól są wspólne z Trio (`field-rules.js`).
- **Okno natywne (Linux):** pytanie o zapis przy zamykaniu (Artist, Trio), tytuł ze sceną i znacznikiem zmian, pełne menu (Plik, Edycja, Widok, Scena, Wyjście, Pomoc), pamięć okna i ostatniej sceny, jedna instancja, kopia robocza po awarii, skojarzenia `.cgpack` i `.vizrs.json`, przeciąganie plików na Artist, wyjście i Fill + Key na wybranym monitorze, logi w `~/.local/state/vizrs`, domyślne dane w `~/.local/share/vizrs`.
- **Okno natywne:** opcja `--gpu on|off|auto` (`VIZRS_GPU`).
- **Artist:** ostrzeżenie przy zapisie sceny, która jest na antenie.
- **Artist – zoom podglądu:** kółko wokół kursora, przesuwanie środkowym przyciskiem, przyciski −/＋/⤢/1:1/⌖ i skróty Shift+1/2/3. Edycja na kanwie działa w każdym powiększeniu.
- **CI:** okno natywne kompiluje się bez ostrzeżeń na Windowsie i macOS (pole `gpu` używane tylko na Linuksie).

## 2.0.13 (2026-11-01) – „Nazwy kanałów”

- **Nazwy kanałów:** `POST /channels/<kanał>` z `name`, nazwa na liście Antena i monitorze PGM w Trio.

## 2.0.12 (2026-11-01) – „Anteny kanałów”

- **Anteny kanałów playoutu:** jeden serwer ma osobną antenę na kanał: TCP `@<kanał> <komenda>`, HTTP `?channel=<kanał>`, wyjście `/output?bus=ch-<kanał>` (także `vizrs-render`). Każda antena ma własną karuzelę, kursor rundownu, makro i dock; lista kanałów przeżywa restart.
- **Trio:** lista Antena przełącza sterowanie i monitor PGM między anteną główną a kanałami.

## 2.0.11 (2026-11-01) – „Porządkowe”

- Wydanie techniczne razem z VizRS CG Playout 2.3.1. Bez zmian w działaniu aplikacji.

## 2.0.10 (2026-11-01) – „Poprawki”

- **Renderer natywny 0.3:** `vizrs-render stream` przyjmuje kilka `--area` i w jednym procesie rysuje każdy obszar osobno (rekord rodzaju 5), zamiast ich obwiedni.
- **CI:** renderer natywny zgodny z clippy nowszego Rusta (`as_chunks`); test okna natywnego czeka na listę stron po połączeniu z serwerem. Bez zmian w działaniu aplikacji.

## 2.0.9 (2026-11-01) – „Czcionki”

- **Artist, własna czcionka belki:** przycisk **⤒** przy polu **font** wgrywa plik TTF/OTF/WOFF/WOFF2 i od razu ustawia go w elemencie; czcionka jedzie z paczką sceny.
- **Fonty wbudowane: Barlow, Bebas Neue, Roboto Mono** (OFL, @fontsource; 400–700, Bebas Neue 400). Szablony INFO24 (Podpis, Title, Scoreboard, BelkaProgram) nie pobierają już fontów z Google Fonts – działają bez internetu i w rendererze natywnym. Lista fontów offline w Artist bierze się z `OFFLINE_FONTS` renderera.

- **Renderer natywny `vizrs-render` (0.1, gałąź `render-native`):** osobny program w `source/render/` rysuje antenę bez przeglądarki dla playoutu kanału (VizRS CG Playout): prymitywy, klucze i krzywe, tekst z dopasowaniem jak w `renderer.js`, zegary ze strefami, ticker `crawl`, warstwy, `dock`. Polecenia `stream` (klatki dla playoutu), `frame`, `check` (które sceny pójdą natywnie) i `bench`. Czego jeszcze nie umie, zgłasza – playout rysuje to Chromium. Zgodność z Chromium: `tools/native-compare.js` (INFO24 i R7 poniżej 1% różnych pikseli). DOKUMENTACJA §10.1, plan `docs/plany/PLAN-native.md`.
- **Renderer natywny – wierność (badanie różnic z Chromium):** plansze testowe (`tools/native-cases.js`), pomiar szerokości tekstu (`tools/native-widths.js`), błąd każdej klatki ruchu (`native-video.js --stats`) i średni błąd w `native-compare.js`. Odtworzone reguły Blink/Skia: interlinia w 1/64 px i połowa interlinii w dół do piksela, linia bazowa na pełnym pikselu, dokładna wysokość przy wyśrodkowaniu, szerokość elementu wielowierszowego, przyciąganie do piksela elementu ze `scaleX`, wiersz podnoszony przez `{sz=}`, łamanie ze znacznikami z poprzedniego wiersza, rozmiar fontu w 26.6, tablica gamma maski tekstu (`SkMaskGamma`), gradient z przemnożoną alfą, dt = 0 w pierwszej klatce po TAKE, odstęp plakietki kategorii crawla. Poprawki: rodzina fontu z nazwą odmiany w podzbiorach @fontsource (Montserrat 600 jako 700 w trybie offline), pionowe położenie rastra tekstu kwantowane do 1/64 px (tekst bywał o piksel niżej). INFO24 i R7: średni błąd stron poniżej 0,6/255, ruch – MSE klatek poniżej 5.
- **Renderer natywny 0.2:** tekst sformatowany (`[[wyróżnienie]]`, `{b}{i}{u}{c=}{sz=}`), kod QR (moduły jak `qr.js`) i tickery `roll`, `wipe`, `whip` – wszystkie sceny INFO24 i R7 rysują się bez Chromium.

- **Kolor etykiety tickera w Trio:** wiadomości bez kategorii ustawiają kolor etykiety (np. PILNE), także stałej etykiety crawla; bez kategorii i etykiety kwadrat koloru przenosi do pola kategorii zamiast być zablokowany.
- **Dokumentacja uporządkowana i sprawdzona z kodem:** poprawione liczby presetów ruchu (21 IN/OUT, 30 + 26 efektów tekstu), nazwa grupy **Ruch – bez kluczy**, zasady skrótów stron, limit 64 MB na plik i kod 413; sekcje `4.4a` i `8.3a` w DOKUMENTACJA dostały zwykłe numery, a „Show na audycję” własną sekcję 9.6; powtórzenia usunięte z README i DOKUMENTACJA §11, §14, §16; `SZYBKI_START.txt` zaktualizowany (paczki `.cgpack`, okno natywne).
- **Trio, zmiana warstwy:** lista **warstwa** w statusie strony zmienia warstwę szablonu (wszystkie jego strony) i od razu ją zapisuje; `●` pokazuje warstwy zajęte na antenie, a **＋ nowa warstwa…** dodaje własną nazwę. Grafika na antenie przechodzi na nową warstwę bez ponownej animacji, a zajęta warstwa wymaga potwierdzenia. Nowe API `POST /api/layer/<strona>`, zdarzenie `relayer`. Otwarta w Artist scena przejmuje zmianę.
- **Trio, F2 i F3 po zmianie warstwy:** działają na warstwie, na której strona faktycznie gra, także po zmianie warstwy szablonu w Artist.
- **Strona startowa:** skrót API obejmuje rundowny, makra i ticker oraz poprawne metody zapisu; dopisany kod QR, usunięty zbędny krok o „Własnym CSS” w OBS.

## 2.0.8 (2026-10-30) – „Kolory”

- **Trio, kolory kategorii tickera:** wybór koloru w okienku (pole barwy, odcień, `#hex`, próbki, podgląd plakietki z ostrzeżeniem o kontraście, powrót do koloru sceny) zamiast dopisywania `#kolor` do nazwy; także w edytorze wiadomości w Artist.
- **Logika kolorów:** kolor należy do kategorii – wybór obejmuje jej wszystkie wiadomości na stronie, a wiadomości bez `#kolor` (też z `TICKER ADD`) przejmują kolor kategorii; wiadomość bez kategorii w `roll`/`wipe`/`whip` bierze kolor etykiety z mapy `catColors` (tak jak `crawl`); zmiana koloru na taśmie crawla działa od razu; zmiana nazwy kategorii nie gubi koloru. Kolory o długości nieznanej CSS (`#12345`) nie są już uznawane za kolor.

## 2.0.7 (2026-10-30) – „Taśma”

- **Taśma crawla przesuwana przez playout kanału:** `/output` wystawia `window.vizrsTape` (`state`, `external`, `film`) – VirtRS CG Playout (opcja `tape`) przesuwa stojący crawl sam, a Chromium rysuje taśmę raz na jej okno zamiast w każdej klatce. Strona oddaje taśmę tylko, gdy crawl stoi i nic nie maluje nad jego oknem (tekst według liter, nie pola); bez wywołań nic się nie zmienia (DOKUMENTACJA §10).

## 2.0.6 (2026-10-25) – „Czas warszawski”

- **Zegar w strefie Europe/Warsaw:** nowa właściwość zegara `tz` (strefa IANA, domyślnie `Europe/Warsaw`, `local` = strefa komputera) – godzina i odliczanie do daty nie zależą już od strefy komputera z OBS ani serwera (np. kontener w UTC).
- **Czas warszawski wszędzie:** czasy w Trio (⚙ Stan: błędy scen, log audytowy; domyślna nazwa presetu) i nazwy kopii zapasowych (`show-RRRRMMDD-GG….json`, dotąd UTC) liczone w strefie Europe/Warsaw z regułą zmiany czasu UE.

## 2.0.5 (2026-10-24) – „Lekkie wyjście”

- **Jeden numer wersji dla pakietu VizRS:** serwer z Trio i Artist (`vizrs`), okno natywne `vizrs-shell` (dotąd 0.1.0) i VirtRS CG Playout mają wersję 2.0.5. Wydanie zawiera zmiany 2.0.2 (niżej – 2.0.2 nie miało osobnego wydania) oraz:
- **CI:** suma kontrolna samej binarki w testach Windows, nazwa kontenera testu dymnego Dockera `vizrs-ci`.
- **Testy okna natywnego (E2E):** wyjście podłączone przed Trio i czekanie na połączenie `/events` – bez wyścigów z `--idle-exit`.

## 2.0.2 (2026-10-24) – „Lekkie wyjście” (wydane w 2.0.5)

- **Lekkie `/output` dla bezgłowego Chromium** (playout kanału na serwerze bez GPU, plan `docs/plany/PLAN-lekkie-output.md`): `?fps=N` – pętla animacji w klatkażu kanału (ticker crawl przy 25 kl./s ok. 45% taniej), `?fx=step` – miganie i pętle uwagi schodkami, efekty ruchowe wyłączone (kropka „Na żywo” z ~180% CPU do poziomu grafiki statycznej). Bez stron na antenie pętla animacji zasypia. Wskazówki dla scen kanału w DOKUMENTACJA §10; test klatek screencastu w `tests/e2e/motion.js`.
- **Dokumentacja uporządkowana:** plany i raport przeniesione do `docs/plany/` (spis z ich stanem), README skrócony do wprowadzenia i spisu, opis Render.com w RUNBOOK, elementy dodatkowe i strony demo w `examples/README.md`, brakujące testy i narzędzia (`app.js`, `shell.js`, `engines.js`, `video.js`) w DOKUMENTACJA §15.
- **Artist, tryb sekwencji – ◇ Z kluczy:** zamiana kluczy wejścia/wyjścia elementu na presety ruchu (odwrotność „◆ Na klucze”), z propozycją przybliżenia, gdy klucze nie pasują dokładnie.
- **Zajęty port – fallback:** gdy domyślny port 8088 jest zajęty (np. serwer `./start-linux.sh` na innym katalogu danych), serwer `vizrs`, okna `vizrs artist|trio` i okno natywne / AppImage używają następnego wolnego portu (komunikat na starcie) zamiast kończyć się błędem; jawny `--port` / `VIZRS_PORT` bez zmian.
- **Zatrzymanie serwera (SIGTERM, Ctrl+C) przy zamkniętym wyjściu konsoli:** komunikat „zatrzymanie” nie blokuje już zakończenia procesu.
- **Pakiet Debian/Ubuntu** `vizrs_<wersja>_amd64.deb` (`deploy/deb/build.sh`, w wydaniu na GitHub obok AppImage): `vizrs` i `vizrs-shell` w `/usr/bin`, w menu VizRS Artist, VizRS Trio i **VizRS Trio – serwer zdalny** (okno „Połącz z serwerem”), ikona, usługa systemd `vizrs` (wyłączona; użytkownik `vizrs`, dane `/var/lib/vizrs`). Zależności: WebKitGTK 4.1, GTK 3.

## 2.0.1 (2026-10-22) – „Okna i serwer”

### Okna i serwer (plan: `docs/plany/PLAN-natywne-i-serwer.md`)

- **Okno natywne: Plik → Połącz z serwerem…** (Ctrl+Shift+K, `vizrs-shell connect`): adres i token serwera zdalnego (VPS) wpisywane w oknie; przed przełączeniem sprawdzenie tokenu (serwery `http://`), potem wszystkie okna przechodzą na ten serwer. Lista zapisanych serwerów (`connections.json` w katalogu konfiguracji, token tylko na życzenie, plik tylko dla właściciela), „łącz przy uruchomieniu” (`--local` – mimo to serwer lokalny), powrót do serwera lokalnego. Strony serwera nie mogą przełączyć okien (komunikat przyjmuje tylko wbudowane okno połączenia).
- **Wydania na GitHub** (`.github/workflows/release.yml`, znacznik `v*` albo ręcznie): archiwa Windows/macOS/Linux (`vizrs`, `vizrs-app`, `vizrs-shell`, paczki `.cgpack`), **AppImage VizRS Artist i VizRS Trio** (`deploy/appimage/`), **obraz Docker serwera** `ghcr.io/sdorpl/vizrs` (amd64, arm64), `SHA256SUMS.txt`.
- **Poprawka CI Windows:** testy Node importowały moduły po ścieżce `D:\…` (`ERR_UNSUPPORTED_ESM_URL_SCHEME`) – teraz `pathToFileURL`.
- **Powiązania (dock) z kilkoma celami:** `target` przyjmuje `a|b` – przesunięcie raz, gdy na antenie jest którykolwiek cel.
- **Kolejność warstw `z<N>_nazwa`:** warstwy o takiej nazwie leżą nad innymi własnymi warstwami w kolejności N (pod `front`).
- **Paczki przykładowe jako `.cgpack`:** `examples/INFO24.cgpack`, `R7_NEWS.cgpack` – każda oprawa w jednym pliku ze scenami, stronami i grafikami (tła hashtagów). Dawne `.show.json` w `examples/archiwum/`; budowa: `tools/build-packs.mjs`.
- **Osobny show na audycję:** biblioteka `vizrs-data/shows/`, lista wyboru w pasku Trio (przełączenie zdejmuje wszystko z anteny), ▾ nowy / kopia bieżącego / usuń; API `/api/shows`.
- **Artist jak program desktopowy:** panele zadokowane z rozdzielaczami (szerokość drzewa i właściwości, wysokość osi czasu; dwuklik = domyślnie; zapamiętane), płaskie paski narzędzi, pasek stanu z podpowiedziami skrótów panelu pod kursorem i wskaźnikiem połączenia, monochromatyczne ikony zamiast emoji (Artist i Trio), interfejs bez zaznaczania tekstu i kursora „rączki”. W oknie natywnym bez logo i zakładek stron (są w menu) i bez menu przeglądarki pod prawym przyciskiem (także Trio).
- **Poprawka: pasek osi czasu Artist nie mieścił się na ekranie** (Full HD/3K ze skalowaniem – zoom i „Dopasuj” znikały za krawędzią): narzędzia jako ikony z podpowiedzią, edytor klucza w stałym pasku pod spodem, narzędzia sekwencji w pasku w tym samym miejscu; jeden wiersz od 1280 px szerokości.
- **Okno natywne `vizrs-shell`** (`source/shell/`, osobny workspace – jedyne miejsce z zależnościami: tao, wry, muda, rfd, png, serde_json, webkit2gtk): Artist (domyślnie), Trio i podgląd wyjścia w prawdziwym oknie systemowym z WebKitGTK / WebView2 / WKWebView; menu Plik/Widok/Pomoc, nowe okna, „Zapisz jako” dla eksportu i paczek, odnośniki spoza serwera w przeglądarce, pisownia polska (Linux), start zmaksymalizowany. Serwer: dołącza do działającego albo uruchamia `vizrs serve --idle-exit 10` jako osobny proces (awaria okna nie zdejmuje anteny). `--connect` dla serwera zdalnego. Skróty Linux/Windows wybierają okno natywne, gdy jest (`--chromium` / `-Chromium` – okno Chromium). Gotowa binarka `bin/linux-x86_64/vizrs-shell`.
- `vizrs serve --idle-exit N`: serwer kończy pracę po N s bez okien i wyjść.
- `tools/engines.js`: zgodność WebView okna natywnego z Chromium (OBS) – ściśnięcie, szerokość i wysokość każdego pola tekstowego na wszystkich stronach zestawów; WebKitGTK: 82/82 zgodnych. Testy `tests/e2e/shell.js` i joby CI `shell` (3 systemy) oraz `shell-e2e` (Xvfb).
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

### Grafika: tickery, ruch, paczki, tekst (plan tickerów: `docs/plany/PLAN-ticker.md`)
- **Ruch – efekty wizualne bez kluczy** (`node.motion`, `web/motion.js`): 22 presety wejścia/wyjścia (m.in. przesłony, maska ukośna, żaluzja, odwrócenia 3D, glitch, rozdzielenie RGB, rosnący pasek, spadanie), 31 efektów tekstu (całość, słowa, znaki, linie, maszyna do pisania z kursorem, szyfrowanie, licznik liczby), 11 pętli uwagi (pulsowanie, bicie serca, połysk, poświata, podkreślenie…), krzywe GSAP (`back.out(1.7)`, `bounce.out`, `elastic.out`, `cubic-bezier`, aliasy `spring`/`overshoot`/`broadcast`). Ten sam renderer w Artist, Trio i OBS; składanie z kluczami.
- **Ruch – stałe efekty tła i punkt odniesienia:** stałe efekty tła kształtów (`motion.bg`: połysk, pulsowanie, świecenie, linie skanowania), punkt odniesienia ruchu (`motion.origin`), efekty tekstu z maskami `inset`, rozmyciem 20/14 px, `hide` od lewej i szyfrowaniem wg postępu całego tekstu.
- **Prosty tryb sekwencji** (Artist → ≣ Sekwencja): wiersz na element z paskami wejścia, wyjścia, efektu tekstu i pętli; przeciąganie zmienia opóźnienie i czas; ⇶ Kaskada (krok, wyjścia odwrotnie), ↳ Po kolei (`motion.after`), ⧉ Kopiuj ruch na rodzeństwo, ◆ Klucze (presety → klucze, `motionToKeys`), ✕, ⟲ Dopasuj; pole „po węźle” w panelu Ruch.
- Artist: grupa **Ruch** we właściwościach węzła i **⟲ Dopasuj stop i długość**; walidacja `motion` w `validate.rs`; testy `tests/motion.js` i `tests/e2e/motion.js`.
- **Paczka belek `.cgpack`** (Artist → ⇩ Paczka / ⇧ Import, `web/cgpack.js`): wybrane sceny, ich strony z Trio, grafiki i czcionki w jednym pliku ZIP (`manifest.json` + `scenes.json` + `assets/`). Import dodaje sceny jako nowe, pomija pliki już obecne, rozwiązuje kolizje nazw (`nazwa_2.ext` z przepisaniem odnośników) i numerów stron, pokazuje raport.
- **Zasoby paczki w scenie:** sekcja w ustawieniach sceny – pliki używane (wykryte) i dołączone ręcznie (`scene.packAssets`, walidacja w `validate.rs`); ＋ Wgraj i dołącz (grafika, wideo, czcionka).
- **Czcionki z serwera na /output:** wyjście ładuje wszystkie czcionki z assets zaraz po otwarciu (i nowe co 30 s) – komputer z OBS nie potrzebuje zainstalowanej czcionki, pierwszy TAKE nie czeka na font.
- **Łamanie wierszy „wypełniaj”** (`lineBreak: fill`, próg `fillSqueeze`, domyślnie 0,9) w `squeeze-lines`/`squeeze-each`: wiersz bierze kolejne słowa do progu ściśnięcia, reszta przechodzi niżej, każdy wiersz ściskany osobno – jak belki TVP. Domyślnie bez zmian (`balance`, wiersze równej szerokości). Walidacja obu właściwości.
- `source/tools/video.js`: film wszystkich paczek (`media/vizrs_WSZYSTKIE_PACZKI.mp4`, 50 kl./s, czas wirtualny); lista paczek `source/tools/sets.js`; zapisana galeria `media/galeria/`.
- `source/tools/gallery.js`: galeria wszystkich stron zestawów przykładowych (zrzuty, arkusze, `index.html`, poza repo).
- Poprawka: automatyczne łamanie wierszy nie dzieli tekstu na twardej spacji (polska typografia „i\u00a0w\u00a0z…”) – wcześniej jednoliterowy spójnik mógł zostać sam na końcu wiersza.
- **Przepełnienie tekstu widoczne wszędzie** (zamknięta luka #17): Artist pokazuje ściśnięcie zaznaczonego tekstu pod „min. ściśnięcie” i czerwoną ramkę przepełnionego tekstu na kanwie (`data-over`); wyjście programu zgłasza przepełnienie do `/api/errors` („⚠ błędy scen” w Trio).
- **Tryb dopasowania `squeeze-each`:** wiersze wpisuje operator (Enter, do `maxLines`), każdy wiersz ściskany osobno do `squeezeMin` bez zmiany fontu – belka dwuwierszowa z różną szerokością liter i tą samą wysokością; wskaźnik `w1/w2` i ostrzeżenie w Trio.
- **Fonty z assets wczytują się ponownie:** font wgrany do assets już po otwarciu Trio/Artist/wyjścia (albo chwilowo niedostępny) jest ponawiany co 2–30 s i teksty przeliczają się po jego wczytaniu; wcześniej zostawał font zastępczy (inne łamanie wierszy, inne ściśnięcie) aż do odświeżenia strony – także na wyjściu OBS.
- **Ściskanie tekstu:** `textFit: squeeze` ściska tylko poziomo, do granicy `squeezeMin` (wcześniej na sztywno 62% i potem mniejszy font); font się nie zmienia, za długi tekst wystaje, a Trio ostrzega „⚠ nie mieści się”. Dawne zachowanie – nowy tryb `squeeze-shrink` (w scenach demo dla wąskich pól: „NA ŻYWO”, etykiety, programy). Nowy tekst w Artist ma `squeeze` (60%).
- Trio: wskaźnik `ściśnięcie: N%` także dla trybów `squeeze`/`squeeze-shrink`; edytor wiadomości tickera oznacza ⚠ wiadomości, które zostaną ucięte.
- Ticker (`roll`/`wipe`/`whip`): dopasowanie wiadomości wg `textFit` i `squeezeMin` węzła (domyślnie `squeeze-shrink` 60% – jak dotąd).
- Walidacja: `props.textFit` (lista trybów) i `props.squeezeMin` (0–1); Artist przycina liczby do zakresu właściwości.
- **Dokumentacja:** `docs/DOKUMENTACJA.md` (pełny opis aplikacji), `docs/INSTRUKCJA-poczatkujacy.md` (instrukcja krok po kroku), `docs/README.md` (spis), `CLAUDE.md` (zasady pracy z kodem).
- Trio: duży podgląd strony sam kadruje element – środek na elemencie, mały element powiększony tak, by zmieścił się cały; dwuklik w puste miejsce wraca do tego kadru.
- **Edytor wiadomości tickera** w Trio i Artist (`ticker-items.js`): lista *kategoria + treść* z ↑↓ i ✕, widok grup po kategoriach (przeciąganie między grupami, zmiana nazwy grupy), tekst surowy; Enter = nowa wiadomość, Ctrl+↑↓ = przesuń, wklejenie wielu wierszy = wiele wiadomości; najechanie na wiersz pokazuje tę wiadomość w podglądzie; ściśnięcie każdej wiadomości widoczne od razu.
- **Ciągłość przy UPDATE**: zmiana wiadomości na antenie nie zaczyna przewijania od nowa – crawl płynie dalej, a nowe wiadomości dochodzą za tym, co jest na ekranie; w roll/wipe/whip bieżąca wiadomość dogrywa swój czas.
- **Tryb `whip`**: etykieta (pas kategorii) przejeżdża przez pasek między wiadomościami i w połowie drogi zmienia się na kategorię następnej.
- Nowe właściwości węzła Ticker: `catColors` (mapa `PILNE=#e00000; INFO=#0050c0`), `label` (etykieta domyślna; w crawl – stała etykieta po lewej), `sepStyle` (tekst, kropka, kwadrat, kreska, skos), `gleam` (błysk etykiety co N s).
- Reguły pola tickera (Artist ⚙): maks. wiadomości, maks. znaków wiadomości, lista dozwolonych kategorii, WERSALIKI, typografia, tylko do odczytu.
- Artist: panel węzła Ticker pokazuje tylko właściwości bieżącego trybu; presety „Ticker pełny ekran”, „Pasek z wiperem”, „Crawl z linią akcentu”, „Ticker boczny” w menu ＋ Dodaj.
- Sterowanie: `TICKER <strona[:pole]> LIST|ADD|SET|DEL|CLEAR` (TCP), `/api/ticker/<strona>[/add|/set/<n>|/del/<n>|/clear]` (HTTP), `/vizrs/ticker …` (OSC) – zmiana zapisuje stronę i aktualizuje antenę.
- Ticker: `crawlFrom` (`right`/`left` – taśma od razu pełna), `labelSize`, `labelWeight` (etykieta/wiper).
- Walidacja: limity tickera (200 wiadomości × 500 znaków), `catColors`, `fieldMeta.maxItems`/`cats`; wspólne wektory zapisu wiadomości JS ↔ Rust (`tests/vectors/ticker.json`).
- Trio: zoom dużego podglądu (kółko, dwuklik w element, przeciąganie), wybór tła monitora PGM (szachownica, czarne, białe, kluczowanie, własny kolor); jeden monitor PGM zamiast PGM+PRV.

## 2.0.0 (2026-10-19) – „Studio”
- **Powiązania (dock)**: reguły `layer:`/`page:`/`scene:` przesuwają scenę, gdy wskazana grafika jest na antenie; nadpisanie operatora (przycisk ⇅ w Trio); stan odtwarzany po odświeżeniu wyjścia.
- **Artist**: markery na osi (M, dwuklik; → stop point), zaznaczanie kluczy (Shift, ramka), kopiuj/wklej klucze (Ctrl+C/V), zoom Ctrl+kółko i „dopasuj”, przeciąganie w drzewie (zmiana rodzica z zachowaniem położenia), biblioteka presetów animacji (zapis, zastosowanie, import/eksport).
- **Model tekstu i wektory testowe** (`web/text-model.js`, `tests/vectors/`).
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
Wydanie bezpieczeństwa i niezawodności (plan: `docs/plany/PLAN-rozbudowy.md`, etapy F0 i F1; szczegóły w `docs/STATUS.md`).
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
