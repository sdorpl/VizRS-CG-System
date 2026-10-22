# VizRS – instrukcja dla początkujących

> Krótka ścieżka od uruchomienia do pierwszej grafiki na antenie. Pełny opis: [DOKUMENTACJA.md](DOKUMENTACJA.md).

## Najważniejsze pojęcia (w 1 minutę)

| Pojęcie | Co to jest |
|---|---|
| **Scena** (szablon) | wygląd i animacja grafiki, projektowana w **Artist** – np. „Belka” |
| **Strona** | scena + konkretne dane + numer, np. *1001 Belka – budżet*; operator pracuje na stronach w **Trio** |
| **Pole kontrolne** ⛓ | właściwość sceny, którą operator zmienia na stronie (tekst, kolor, obraz…) |
| **Warstwa** | `back`, `main`, `front` lub własna; grafiki na tej samej warstwie się zastępują, na różnych grają razem |
| **Stop point** | miejsce, w którym animacja się zatrzymuje: TAKE gra do 1. stopu, CONTINUE do kolejnego, OUT do końca |
| **Wyjście** | strona `/output`, którą OBS pokazuje jako przezroczystą grafikę |

## 1. Uruchomienie

| System | Polecenie |
|---|---|
| Debian / Ubuntu | `sudo apt install ./vizrs_2.0.1_amd64.deb` (plik z wydania na GitHub), potem w menu aplikacji **VizRS Trio** / **VizRS Artist** / **VizRS Trio – serwer zdalny** |
| inny Linux | `VizRS-Trio-…-x86_64.AppImage` / `VizRS-Artist-…AppImage` z wydania na GitHub – nadaj prawo uruchamiania i kliknij |
| Linux | `./start-linux.sh trio` (okno Trio) albo `./start-linux.sh artist` (okno Artist); `./start-linux.sh` = sam serwer |
| Windows | raz `build-windows.bat` (wymaga Rust z https://rustup.rs), potem `start-windows.bat` (okno Trio) albo `start-windows.bat artist` |
| macOS | `./build-macos-linux.sh`, potem `./bin/native/vizrs --data ./vizrs-data trio` |

**Okna aplikacji:** Artist i Trio otwierają się we własnych oknach (Edge/Chrome/Chromium bez pasków przeglądarki). Drugie okno dołącza do już działającego serwera. Zamknięcie okna nie zdejmuje grafiki z anteny – serwer pracuje, dopóki OBS albo inne okno jest podłączone.
Panele Artist można poszerzać i zwężać, przeciągając cienkie rozdzielacze między nimi (dwuklik przywraca rozmiar); podpowiedzi skrótów dla panelu pod kursorem są w pasku stanu na dole okna.
**Okno natywne (zalecane):** `vizrs-shell` – Artist w prawdziwym oknie systemowym z menu i oknem „Zapisz jako” (Linux: `bin/linux-x86_64/vizrs-shell`; `vizrs-shell trio` – Trio). Zamknięcie okna nie zdejmuje grafiki z anteny.
Ikony w menu: Linux `deploy/install-desktop-linux.sh`, Windows `deploy\install-windows-shortcuts.ps1` (bez czarnego okna konsoli) – same wybiorą okno natywne, jeśli jest. Wyjście na drugi monitor zamiast OBS: `vizrs output --fullscreen --position 1920,0`.

Sterowanie serwerem na VPS z własnego komputera: w oknie natywnym **Plik → Połącz z serwerem…** (Ctrl+Shift+K) – wpisz adres serwera i token, kliknij **Połącz** (zaznacz „zapamiętaj token”, by następnym razem wybrać serwer z listy; **Serwer lokalny** wraca do pracy na swoim komputerze). Z wiersza poleceń: `vizrs trio --connect https://twoj-serwer.pl --token TOKEN`.

Możesz też otworzyć strony w zwykłej przeglądarce:

- `http://localhost:8088/trio` – playout (operator),
- `http://localhost:8088/artist` – projektowanie szablonów,
- `http://localhost:8088/output?checker=1` – podgląd wyjścia.

Za pierwszym razem startuje przykładowy zestaw **INFO24** (belki, ticker, zegar, NA ŻYWO…).

## 2. Pierwsza grafika na antenie (5 minut, Trio)

1. Otwórz **Trio**. Na liście **Strony** kliknij *1001* (albo wpisz `1001` i Enter).
2. W środkowej kolumnie zmień tekst w polu „Treść belki”. Podgląd pod polami pokazuje wynik od razu.
3. Naciśnij **F1** (TAKE). Grafika wchodzi na antenę – widać ją w monitorze **PGM** po prawej i na liście **Na antenie**.
4. Popraw tekst i naciśnij **F4** (UPDATE) – dane zmieniają się bez ponownej animacji.
5. Naciśnij **F3** (OUT) – grafika schodzi.
6. **Ctrl+S** zapisuje zmiany na stronie (TAKE wysyła to, co jest w polach, także bez zapisu).

## 3. Własna strona z szablonu

1. W Trio, panel **Szablony**: kliknij **＋** przy wybranym szablonie.
2. Powstaje nowa strona z kolejnym numerem i wartościami domyślnymi. Zmień numer i nazwę w nagłówku edytora.
3. Wypełnij pola i zapisz (**Ctrl+S**).
4. Przycisk **⧉ Duplikuj** pod listą stron tworzy kopię bieżącej strony – wygodne przy serii podpisów.

## 4. Pierwsza belka w Artist (10 minut)

1. Otwórz **Artist** → **＋ Nowa**, podaj nazwę sceny.
2. Kliknij puste tło kanwy – po prawej pojawią się **ustawienia sceny**. Ustaw **warstwę** (np. `main`) i **długość** (np. 1,5 s).
3. **＋ Dodaj… → Prostokąt**. W **Właściwościach** ustaw `x`, `y`, `w`, `h` i kolor tła belki (możesz też przeciągać element na kanwie; strzałki przesuwają o 1 px, z Shift o 10 px).
4. **＋ Dodaj… → Tekst**. Ustaw położenie, font, rozmiar, kolor. Nowy tekst ma dopasowanie `squeeze` (jak w BTB CG): długi tekst zostanie ściśnięty, najwyżej do `min. ściśnięcie` (60%). Dla belki dwuwierszowej ustaw `maks. linii` = 2 i wybierz `squeeze-each` (operator łamie wiersze Enterem, każdy wiersz ściska się osobno, wysokość liter ta sama) albo `squeeze-lines` (wiersze dzielą się same; „łamanie wierszy” = `fill` wypełnia najpierw pierwszy wiersz, jak belki TVP), dla wąskiej plakietki o stałej szerokości – `squeeze-shrink` (po ściśnięciu zmniejsza font).
5. Przy właściwości **tekst** kliknij **⛓** – powstaje pole kontrolne. W ustawieniach sceny (puste tło) nadaj mu etykietę, np. „Treść belki”.
6. **Zapisz** (Ctrl+S), potem **＋ Utwórz stronę w Trio** w ustawieniach sceny.

### Własna czcionka i grafiki – paczka belek
1. W ustawieniach sceny (puste tło) w sekcji **Zasoby paczki** kliknij **＋ Wgraj i dołącz** i wybierz plik czcionki (`.ttf`, `.otf`, `.woff2`) albo grafikę.
2. Czcionka pojawi się na liście fontów pod nazwą pliku (np. `MojFont.ttf`) – wybierz ją w polu **font** tekstu.
3. **⇩ Paczka** → zaznacz sceny → **Pobierz .cgpack**. Na innym stanowisku: **⇧ Import** i wskaż plik – sceny, strony, grafiki i czcionki wgrają się razem.
4. Komputer z OBS nie musi mieć czcionki – wyjście pobiera ją z serwera VizRS.

### Tło dopasowane do tekstu (autofollow)

Zaznacz prostokąt i w grupie **Autofollow** wpisz w polu „śledź węzeł(y)” nazwę węzła tekstu. Prostokąt dopasuje szerokość (oś `w`) albo wysokość (`h`) do treści, z marginesem `margines`.
Element przy prawej krawędzi ekranu ustaw z **punktem X = right** – będzie rósł w lewo.

## 5. Animacja wejścia i wyjścia

1. Ustaw głowicę osi czasu na **0** (Home). Zaznacz prostokąt, ustaw **maska P%** = 100 i kliknij **◆** przy tej właściwości.
2. Przesuń głowicę na **0,4 s**, ustaw **maska P%** = 0 – klucz powstaje sam (właściwość ma już animację). Belka „wyjeżdża” z lewej.
3. W **0,6 s** kliknij **＋ Stop** – tu grafika stanie po TAKE.
4. Za stop pointem (np. 1,0–1,4 s) zrób odwrotną animację – to będzie wyjście (OUT).
5. Sprawdź: **▶ Take**, **⏏ Out** na osi czasu. Klik w klucz pozwala zmienić krzywą (`out` – hamowanie, `back` – lekkie przestrzelenie).

Gotowy zestaw kluczy możesz zapisać jako **preset animacji** (Właściwości → Presety animacji → ＋ zapisz) i stosować na innych węzłach.

### Szybciej: ruch bez kluczy (jak w BTB CG)
1. Zaznacz węzeł i przewiń Właściwości do grupy **Ruch (jak w BTB CG)**.
2. **wejście** – wybierz efekt (np. *przesłona pozioma*, *wjazd*, *wyskok*), kierunek, czas i opóźnienie. **wyjście** – tak samo.
3. Przy tekście: **tekst IN** (np. *znaki: w górę*, *maszyna do pisania*, *licznik liczby*) i **tekst OUT**.
4. **pętla** – np. *pulsowanie* plakietki „NA ŻYWO” albo *połysk* paska; gra, gdy grafika stoi na wizji.
5. Kliknij **⟲ Dopasuj stop i długość** – stop point stanie po wejściu, a scena wydłuży się o wyjście. Sprawdź **▶ Take** i **⏏ Out**.

### Sekwencja wielu elementów
1. Na osi czasu kliknij **≣ Sekwencja** – zamiast kluczy zobaczysz paski wejścia (niebieskie) i wyjścia (pomarańczowe) każdego elementu.
2. Zaznacz grupę belki i kliknij **⇶ Kaskada** – elementy wejdą jeden po drugim co „krok” (np. 0,1 s), a zejdą w odwrotnej kolejności. **↳ Po kolei** – każdy wchodzi, gdy skończy się poprzedni.
3. Przeciągnij pasek, aby przesunąć start; przeciągnij jego prawą krawędź, aby wydłużyć ruch.
4. Chcesz dopracować ruch ręcznie? Zaznacz element i kliknij **◆ Na klucze** – preset zamieni się na zwykłe klucze. W drugą stronę działa **◇ Z kluczy**: klucze wejścia/wyjścia zamienią się na preset (gdy nie pasują dokładnie, Artist zapyta o przybliżenie).

## 6. Wyświetlenie w OBS

1. OBS → Źródła → **+** → **Przeglądarka**.
2. URL `http://localhost:8088/output`, szerokość 1920, wysokość 1080.
3. Zaznacz **„Odśwież przeglądarkę, gdy scena staje się aktywna”** – po odświeżeniu VizRS odtwarza bieżący stan anteny.
4. Opcjonalnie: Trio jako dock w OBS (*Docki → Niestandardowe docki przeglądarki* → `http://localhost:8088/trio`).

Mikser z kluczowaniem (ATEM, Tricaster): dwa źródła – `/output?mode=fill` i `/output?mode=key`.

## 7. Ticker

1. Wczytaj stronę z tickerem (w INFO24: *3001*).
2. Pole wiadomości to lista: kategoria + treść. **Enter** dodaje wiadomość, **Ctrl+↑↓** przesuwa, ✕ usuwa. Wklejenie kilku wierszy tworzy kilka wiadomości.
3. Najechanie myszą na wiadomość pokazuje ją w podglądzie.
4. Na antenie **UPDATE** (F4) dokłada zmiany bez restartu przewijania.

Zapis tekstowy wiadomości: `PILNE|Treść`, z kolorem kategorii `SPORT#1f8a4c|Treść`, bez kategorii – sama treść.

## 8. Program na żywo: rundown i makra

- **Rundown** (prawa kolumna): **＋ strona** dodaje zaznaczoną stronę, kolejność zmieniasz przeciąganiem, przy pozycji możesz dopisać notatkę. **F8 (NEXT)** bierze następną pozycję na antenę (▶ = następna, ● = na antenie, ✓ = odegrana).
- **Makra**: jedna komenda w wierszu, np.
  ```
  CLEAR
  TAKE 1001
  +2.5 TAKE 3001
  ```
  **⇥ z anteny** zapisuje bieżący zestaw grafik jako makro (preset segmentu).
- **Skrót strony**: kliknij pole „skrót” w nagłówku edytora i naciśnij np. Ctrl+1 – ten skrót robi TAKE strony, a gdy jest na antenie, OUT.

## 9. Ściąga skrótów

**Trio**

| Skrót | Działanie |
|---|---|
| F1 / Ctrl+Enter | TAKE |
| F2 | CONTINUE |
| F3 | OUT |
| F4 | UPDATE |
| F6 | ARM (Preview) |
| F8 | NEXT (rundown) |
| cyfry + Enter | wczytaj stronę |
| ↑ / ↓ | poprzednia / następna strona |
| Ctrl+S | zapisz stronę |
| Ctrl+Spacja | Word Replace (skrót → słowo) |
| Shift+Esc | CLEAR ALL |

**Artist**

| Skrót | Działanie |
|---|---|
| Ctrl+S | zapisz |
| Ctrl+Z / Ctrl+Y | cofnij / ponów |
| Ctrl+D | duplikuj węzeł |
| Delete | usuń węzeł / klucz |
| Spacja | odtwórz / pauza |
| Home / End | początek / koniec |
| M | marker |
| strzałki (+Shift) | przesuń o 1 (10) px |

**Podgląd w Trio:** kółko = zoom, przeciąganie = przesunięcie, dwuklik w element = powiększ, dwuklik w tło = kadr na element, ⤢ = cały ekran.

## 10. Najczęstsze problemy

| Problem | Co zrobić |
|---|---|
| OBS nic nie pokazuje | Trio → **⚙ Stan**: czy „Wyjścia” ≥ 1? W OBS kliknij *Odśwież pamięć podręczną bieżącej strony* w źródle Przeglądarka. Sprawdź URL i token. |
| Zmiana szablonu nie widać na antenie | to celowe: warstwa ma znacznik **⏳ nowa wersja** – kliknij **Wdróż teraz** albo zrób ponowny TAKE. |
| Czerwone „⚠ zapis show nieudany” | brak miejsca lub praw zapisu do `vizrs-data/`; antena działa, edycje nie są zapisywane. |
| „⚠ błędy scen: N” | ⚙ Stan → lista błędów; wadliwy element jest ukryty, reszta gra. Popraw scenę w Artist. „tekst nie mieści się” = tekst na antenie wystaje poza belkę – skróć go i daj UPDATE. |
| Tekst za długi / ściśnięty | pod polem widać `ściśnięcie: 72%` albo `w1: 72%` – pomarańczowe = ściśnięte, czerwone = bardzo ściśnięte; **⚠ nie mieści się** = tekst wystaje poza belkę: skróć go albo podziel na wiersze (Enter). |
| Inny font na antenie niż w Artist | używaj fontów wbudowanych (Oswald, Barlow Condensed, Inter, Roboto, Roboto Condensed, Montserrat) albo wgraj plik fontu do assets (Zasoby paczki → ＋ Wgraj i dołącz) i wybierz go w polu font; OBS pobiera go z serwera. |
| „🔒 edytuje …” przy stronie | stronę edytuje inny operator; zapis wymaga potwierdzenia, blokada wygasa po 45 s. |
| Każda audycja ma mieć własną grafikę | Trio → lista show w pasku górnym: wybierz audycję; **▾** → nowy show albo kopia bieżącego. |
| Trzeba wrócić do wcześniejszej wersji show | ⚙ Stan → **Kopie zapasowe show** → Przywróć. |
| Zdjąć wszystko natychmiast | **Shift+Esc** (CLEAR ALL). |

Więcej: [RUNBOOK.md](RUNBOOK.md) (awarie i lista kontrolna przed emisją).
