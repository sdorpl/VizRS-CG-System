# VizRS Live – klient ErsatzTV Next z interfejsem na wzór ErsatzTV legacy

> Stan (2026-11-06): **plan dalekosiężny.** Rozpoznanie obu projektów zrobione (§2–§3, stan repozytoriów z 2026-11-05/06: `ErsatzTV/next` 92e93d7, `ErsatzTV/legacy` 06c008b). Brak decyzji, etapów i terminu – pytania w §5.

## 1. Pomysł

**VizRS Live** to przyszła aplikacja rodziny VizRS: graficzny klient (GUI) dla **ErsatzTV Next**, z funkcjami, które dziś daje interfejs WWW **ErsatzTV legacy**. Operator kanału prowadzi w jednym miejscu kanały, ramówkę i grafikę VizRS, a pod spodem pracuje ErsatzTV Next.

| Repozytorium / aplikacja | Rola |
|---|---|
| VizRS CG System | silnik grafiki: Artist, Trio, `/output`, API TCP / HTTP / SSE |
| VizRS CG Playout | sidecar dla ErsatzTV: komendy na start klipu, reklamy bez grafiki, wtapianie, live ingest |
| **VizRS Live** (przyszłość) | GUI kanału nad ErsatzTV Next: to, co w legacy robi interfejs WWW, plus spięcie z grafiką i live ingest VizRS |

## 2. ErsatzTV Next – co to jest

Źródła: [github.com/ErsatzTV/next](https://github.com/ErsatzTV/next) (README, `schema/`, `examples/`, `CHANGELOG.md`, kod `crates/ersatztv`), [ersatztv.org/next-docs](https://ersatztv.org/next-docs/).

- **Przepisany w Ruście silnik strumieniowania** ErsatzTV (MIT, wersja 0.2.x, przed 1.0: zmiana `B` w `0.B.C` = zmiana niezgodna). Zasada „jedna rzecz dobrze”: transkodowanie i nadawanie.
- **Poza zakresem z założenia:** biblioteki i metadane, harmonogramy, tworzenie playoutów. **Nie ma UI ani API do zarządzania.** Dołączony `ersatztv-playout-generator` służy tylko do pokazu („feature requests will not be accepted”).
- Dwa tryby: **jako domyślny silnik legacy** (kanał: *Streaming Engine = Next*; legacy pisze konfigurację i playouty), albo **samodzielnie** z własnym planistą, który pisze pliki JSON zgodne ze schematami.
- **Kontrakt = pliki JSON z wersją** (`schema/*.json`; niezgodna wersja = błąd, nie zgadywanie):
  - `lineup.json`: serwer (`bind_address`, `port`), katalog HLS, katalog XMLTV, lista kanałów (`number`, `name`, `config`, `tvg_id`, `logo`, `group`, `overlays`).
  - `channel.json`: katalog playoutu, ffmpeg, normalizacja audio (format, bitrate, kanały, głośność EBU: integrated/range/true peak) i wideo (kodek, rozdzielczość, bitrate, akceleracja, przeplot, tonemapping HDR), napisy (`burn`…), tryb kopiowania strumieni, **fallback** (obraz tła zamiast czerni, czas, `show_error` – wypalony powód awarii).
  - **Nakładki konfiguracji** (`overlays`): kolejne pliki nadpisują `channel.json` (`null` usuwa klucz) – legacy tak dokłada ustawienia domyślne i per kanał.
  - **Playout**: katalog plików `{start}_{finish}.json`, każdy z listą `items` (`id`, `start`, `finish` w RFC 3339, `source` albo `tracks`, `watermark`, `graphics[]`). Pliki są wczytywane ponownie po zmianie (pamięć podręczna wg czasu modyfikacji). Zły element pokazuje fallback tylko na swój czas.
- **Źródła elementu:** `local`, `lavfi`, `http` (z `in/out_point_ms`, `is_live`), `rtsp`, `script` (komenda piszącą MPEG-TS na stdout), **`dynamic`** – element rozwiązywany **w chwili emisji** zapytaniem HTTP, które zwraca `PlayoutItem` JSON (szablon URI z `{{SEKRET}}`, ponawianie z budżetem czasu, nagłówek `x-etv-playout-schema`).
- **Grafika w silniku:**
  - warstwy medialne (obraz/wideo): kotwica, margines i szerokość w %, krycie, harmonogram okresowy (co N ms, `fade_ms`, `hold_ms`, wyłączenie po czasie, zegar ściany);
  - **warstwa `canvas`**: wideo z kanałem alfa (np. NUT), położone pełnoklatkowo albo w regionie, **przewijane razem z treścią**; źródło HTTP dostaje nagłówki `x-etv-channel`, `x-etv-offset-ms`, `x-etv-duration-ms`, `x-etv-frame-rate`. Tak legacy podaje do Next swoje elementy graficzne.
- **Wyjście HTTP** (port 8409): `/channel/{nr}.m3u8` (HLS, sesja startuje na żądanie), `/channels.m3u` (z `tvg-chno`), `/xmltv.xml` (scalony z plików `{tvg_id}.xml`, które **pisze planista**). Brak uwierzytelniania, CORS otwarty.
- **Akceleracja:** AMF, CUDA, QSV, RKMPP, VAAPI, VideoToolbox, Vulkan; sprawdzana w działaniu, w razie braku – programowo. `ersatztv-channel describe --probe` zwraca JSON (`schema/describe.json`) z wersją, akceptowanymi wersjami schematów i możliwościami sprzętu – gotowe źródło dla formularza ustawień w UI.
- Wymaga własnej kompilacji **ErsatzTV-ffmpeg** (zwykły ffmpeg nie jest wspierany). Obraz `ersatztv/next` (amd64, arm64), binarki Linux/Windows/macOS.
- Legacy uruchamia kanał jako `ersatztv-channel run --output-folder … --number N - [default.json] [N.json]` z konfiguracją na stdin, a przed startem zapisuje playout do plików (`SyncNextPlayout`).

## 3. ErsatzTV legacy – funkcje UI

Źródła: [github.com/ErsatzTV/legacy](https://github.com/ErsatzTV/legacy) (`ErsatzTV/Pages/*.razor`, `Shared/MainLayout.razor`, `Controllers/`), [ersatztv.org/docs](https://ersatztv.org/docs/). Aplikacja C#/.NET, UI w Blazor + MudBlazor, baza SQLite albo MySQL, licencja zlib. Strona projektu: legacy dostaje poprawki bezpieczeństwa „w dającej się przewidzieć przyszłości”.

### 3.1 Menu (układ nawigacji)

| Grupa | Strony |
|---|---|
| (góra) | **Channels**, **FFmpeg Profiles**, **Watermarks** |
| Media Sources | Local, Emby, Jellyfin, Plex (połączenie, wybór bibliotek, zamiana ścieżek) |
| Media | Libraries, Trash, TV Shows, Movies, Music Artists, Other Videos, Songs, Images, Remote Streams; wyszukiwarka (zapytania, „zapisz jako smart collection”) |
| Lists | Collections, Smart Collections, Multi Collections, Rerun Collections, Playlists, Trakt Lists, Filler Presets |
| Scheduling | Schedules, Blocks, Templates, Decos, Deco Templates, Playouts |
| Settings | FFmpeg, Logging, HDHomeRun, Scanner, Playout, UI, XMLTV |
| System | Health, Logs, Troubleshooting (podgląd odtwarzania Legacy/Next, logi, walidator YAML, historia i diagnostyka block playoutów) |
| pasek górny | linki do `channels.m3u`, `xmltv.xml`, dokumentacji, Swagger API |

Tłumaczenia: en, pt-br i **częściowe pl** (tylko `MainLayout`, `Common`, `Channels`).

### 3.2 Najważniejsze edytory

- **Kanał**: numer, nazwa, grupa, kategorie, logo (plik/URL), włączony / w EPG, profil FFmpeg, **silnik (Legacy/Next)**, tryb strumienia i transkodowania, źródło playoutu i **kanał lustrzany** (z przesunięciem), tryb playoutu, zachowanie bezczynne, wypełniacz awaryjny, znak wodny, preferowany język/ścieżka audio i napisów, selektor strumieni, tryby teledysków i piosenek, „slug seconds”.
- **Profil FFmpeg**: wideo/audio, rozdzielczość, akceleracja, skalowanie (scale+pad, crop, stretch), przeplot.
- **Znak wodny**: obraz albo logo kanału, 9 pozycji, rozmiar (rzeczywisty/skalowany), stały lub przerywany (co N min, czas), krycie (także wyrażeniem).
- **Playouty** – pięć rodzajów: **Classic** (harmonogram), **Block** (bloki + szablony w kalendarzu + deco), **Sequential** (plik YAML), **Scripted** (zewnętrzny skrypt przez API), **JSON (dizqueTV)**; dla każdego reset, podgląd, harmonogramy alternatywne (dni tygodnia/miesiąca).
- **Harmonogram (Classic)**: start elastyczny/ścisły; pozycje: kolekcja/smart/multi/playlist/serial/sezon/artysta/wyszukiwanie/marathon, kolejność (chronologiczna, losowa, shuffle, shuffle in order, rerun, grupy), **tryb: One / Multiple (liczba) / Duration / Flood**, **Fixed start** (godzina), wypełniacze per pozycja (pre/mid/post-roll, tail, fallback), nadpisania (np. tytuł EPG), elementy graficzne.
- **Bloki, szablony, deco**: blok = lista pozycji o łącznej długości z podglądem; szablon = rozkład bloków w ciągu dnia przypisany do dni kalendarza; **deco** = „wystrój” bloku: znak wodny, **elementy graficzne**, treść przerw (między pozycjami bloku, na znacznikach rozdziałów, na początku/końcu), domyślny wypełniacz, zapas na „martwe powietrze”; tryby dziedzicz/scal/zastąp/wyłącz.
- **Filler Presets**: pre-roll, mid-roll, post-roll, tail, fallback; liczba / losowa liczba / czas / dopełnienie do pełnych N minut.
- **Elementy graficzne** (pliki YAML): **Image**, **Text** (fonty, style, obwódka/halo, dopasowanie, zmienne z EPG – tytuł, następny program), **Subtitle**, **Motion** (wideo z alfą, zachowanie na końcu), **Script** (komenda generująca klatki). Legacy renderuje je w strumień `canvas` dla Next (`/graphics/{kanał}/{element}`, podpisany URL).
- **Ustawienia XMLTV** (czas lokalny/UTC, rzeczywiste czasy albo równy podział), **HDHomeRun** (emulacja tunera dla Plex/Jellyfin/Emby), skaner, logi.

### 3.3 API legacy (REST, Swagger)

Niewielkie: lista kanałów, **reset/continue playoutu**, skan biblioteki, sesje (lista, zamknięcie), diagnostyka, oraz **API Scripted Schedule** – zewnętrzny skrypt buduje playout komendami: `add_collection`, `add_count`, `add_duration`, `add_all`, `pad_to_next`, `pad_until(_exact)`, `wait_until(_exact)`, `start/stop_epg_group`, **`graphics_on/off`**, `watermark_on/off`, `pre_roll_on/off`, `skip_items`, `peek_next`. Większości funkcji UI (kanały, harmonogramy, bloki) przez API nie da się ustawić.

## 4. Wnioski dla VizRS Live

1. **„Klient Next” = planista, nie pilot.** Next nie ma API sterowania ani UI – przyjmuje tylko pliki (`lineup.json`, `channel.json` + nakładki, `playout/*.json`, `xmltv/*.xml`) i odpytuje adresy HTTP (`dynamic`, `canvas`). VizRS Live musiałby więc przejąć rolę legacy: bazę kanałów, mediów i ramówek, budowanie playoutu i EPG, uruchamianie serwera Next. To duży zakres – legacy to wieloletni projekt z bazą danych, skanerem i pięcioma rodzajami planowania.
2. **Nie kopiować legacy 1:1, tylko jego model pracy dla kanału nadawczego.** Legacy celuje w domowe IPTV z bibliotek Plex/Jellyfin/Emby (seriale, sezony, teledyski, Trakt). Dla VizRS ważniejsze są: kanały, profile kodowania, ramówka dnia z godzinami stałymi, bloki z szablonami w kalendarzu, wypełniacze i przerwy (deco, filler presets), EPG, podgląd i diagnostyka. Integracje z serwerami mediów – na końcu albo wcale.
3. **Grafika VizRS bez obejść.** Warstwa `canvas` Next przyjmuje strumień z alfą z adresu HTTP i podaje mu kanał, pozycję i klatkaż. VizRS (`vizrs-render` z [PLAN-native.md](PLAN-native.md) albo Chromium) mógłby podawać taki strumień wprost – bez nakładki ffmpeg i wtapiania z VizRS CG Playout. Do sprawdzenia: synchronizacja z `x-etv-offset-ms` (canvas jest przewijany z treścią, a grafika VizRS sterowana komendami żyje w czasie rzeczywistym).
4. **Live ingest przez `dynamic`.** Slot audycji na żywo może być elementem `dynamic`: w chwili emisji VizRS Live odpowiada `PlayoutItem` ze źródłem studia (`http`/`rtsp`, `is_live`) albo planszą. Zastępuje to dzisiejsze rozpoznawanie slotu w nakładce ffmpeg (CG Playout, DOKUMENTACJA §1).
5. **Komendy grafiki na start klipu** – dziś CG Playout zgaduje start pozycji z procesu ffmpeg. Gdy playout pisze VizRS Live, zna dokładne `start`/`finish` każdego elementu i może sam wysyłać `TAKE`/`OUT`/`UPDATE` (jak Viz Multichannel, [PLAN-kanal.md](PLAN-kanal.md)) – także „Za chwilę” N s przed końcem ([PLAN-za-chwile.md](PLAN-za-chwile.md)).
6. **Ryzyka:** Next jest przed 1.0 (zmiany niezgodne w wersjach `0.B`) – trzeba przypiąć wersje schematów i testować na `describe`; wymagany ErsatzTV-ffmpeg; serwer Next bez uwierzytelniania (tylko w sieci wewnętrznej / za proxy); duży zakres pracy przy samodzielnym planiście.
7. **Licencje:** Next MIT, legacy zlib – pomysły i ewentualnie fragmenty wolno przenosić z zachowaniem informacji o autorach.

## 5. Decyzje do podjęcia przed etapami

1. **Wariant:**
   - **A – pełny zamiennik legacy**: VizRS Live ma własną bazę, media, harmonogramy i UI, pisze pliki dla Next. Największa praca, pełna kontrola.
   - **B – UI nad legacy**: VizRS Live korzysta z legacy (baza, planowanie) i dokłada swoje widoki. Ograniczone, bo API legacy obejmuje mało funkcji UI (§3.3).
   - **C – planista kanału nadawczego (zalecany start)**: wariant A w wąskim zakresie z §4 pkt 2 – kanały, profile, ramówka z blokami i godzinami stałymi, przerwy, EPG, grafika VizRS przez `canvas` i live przez `dynamic`. Biblioteki: katalogi lokalne i źródła zdalne, bez Plex/Jellyfin/Emby.
2. **Relacja z VizRS CG Playout**: przy C sidecar przestaje być potrzebny dla kanałów VizRS Live (komendy, live i grafika idą wprost), a zostaje dla kanałów legacy. Czy go utrzymywać równolegle?
3. **Przejście z legacy**: import kanałów, profili i harmonogramów z bazy legacy (SQLite) – potrzebny, czy kanały zakładamy od nowa?
4. **Forma aplikacji**: serwer WWW + przeglądarka (jak VizRS), okno natywne (`vizrs-shell`), czy oba. Osobne repozytorium z własnymi zależnościami; rdzeń `vizrs` bez zmian.
5. **Pierwszy prototyp**: najmniejszy krok, który sprawdza pkt 3 i 4 z §4 – jeden kanał Next z playoutem pisanym ręcznie, warstwą `canvas` z VizRS i slotem `dynamic`.
