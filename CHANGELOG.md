# VizRS – historia zmian

## Niewydane

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
- **Paczka `examples/BTB_TRIO.cgpack`:** grafiki z gałęzi `TRIO` BTB CG System (19 grafik, 18 scen, strony 15001–15401) z zachowanymi powiązaniami belek (dokowanie BTB → `scene.dock`) i kolejnością warstw; opis w `archiwum/BTB_TRIO/README.md`, budowa `tools/build-btb-trio.mjs`.
- **Powiązania (dock) z kilkoma celami:** `target` przyjmuje `a|b` – przesunięcie raz, gdy na antenie jest którykolwiek cel (jak dokowanie do wielu rodziców w BTB).
- **Kolejność warstw `z<N>_nazwa`:** warstwy o takiej nazwie leżą nad innymi własnymi warstwami w kolejności N (pod `front`).
- **Paczka `examples/BTB_ALPHA.cgpack`:** grafiki z gałęzi `alpha` BTB CG System (szablony HTML/CSS/JS, baza `database.sqlite`) przepisane na sceny VizRS – 58 scen, 75 stron 14001–14501: belki Republika/EXP/Lw/Modern/OCG (też rysowane na `<canvas>`), plakietki NA ŻYWO, Miasto i tagi OCG, tickery (whip, przewijany, pionowy, Unified V1–V3, Crawl, Cycling), zegary, logo i widżet zegar + BTB INFO. Każda grafika = strona, grafiki o identycznym wyglądzie dzielą scenę. Wygląd z pomiaru DOM wyjścia BTB (`tools/measure-btb-alpha.cjs` → `archiwum/BTB_ALPHA/zrodlo.json`), sceny z `tools/build-btb-alpha.mjs`; galeria `tools/gallery.js` obejmuje zestaw.
- **Paczka `examples/BTB_INFO24.cgpack`:** zestaw INFO24 (16 scen, 22 strony 9001–13001) w stylistyce BTB – kolory BTB i font Bahnschrift; generowana przez `tools/build-packs.mjs` z `INFO24.show.json`.
- **Paczki przykładowe jako `.cgpack`:** `examples/INFO24.cgpack`, `R7_NEWS.cgpack`, `BTB.cgpack` – każda oprawa w jednym pliku ze scenami, stronami, grafikami (tła hashtagów) i czcionkami (Bahnschrift w BTB). BTB łączy 31 wzorców z belką wielowierszową (8501–8506); usunięty duplikat „zegarek (kopia)”, tickery z trybami przemianowane. Dawne `.show.json` i `BTB_CG/` w `examples/archiwum/`; budowa: `tools/build-packs.mjs`.
- **Osobny show na audycję:** biblioteka `vizrs-data/shows/`, lista wyboru w pasku Trio (przełączenie zdejmuje wszystko z anteny), ▾ nowy / kopia bieżącego / usuń; API `/api/shows`.
- Artist: z interfejsu usunięto nazwy BTB (przycisk importu elementu to teraz **⇧ Element**, bez „BTB” w podpowiedziach, raportach i komunikatach).
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
- `source/tools/video.js`: film wszystkich paczek (`media/vizrs_WSZYSTKIE_PACZKI.mp4`, 50 kl./s, czas wirtualny); lista paczek `source/tools/sets.js`; zapisana galeria `media/galeria/`.
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
