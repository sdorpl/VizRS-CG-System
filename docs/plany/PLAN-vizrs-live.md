# VizRS Live – nakładka na ErsatzTV legacy z funkcjami jego UI

> Stan (2026-11-06): **plan dalekosiężny.** Rozpoznanie obu projektów zrobione (§2–§3, stan repozytoriów z 2026-11-05/06: `ErsatzTV/next` 92e93d7, `ErsatzTV/legacy` 06c008b). **Decyzja użytkownika (2026-11-06): VizRS Live jest nakładką na ErsatzTV legacy** – bez pisania własnego planisty, bazy i skanera („nie wymyślamy koła na nowo”), ale w zakresie funkcji UI legacy (§4). Etapów i terminu jeszcze nie ma – otwarte pytania w §5.

## 1. Pomysł

**VizRS Live** to przyszła aplikacja rodziny VizRS: **nakładka na ErsatzTV legacy** z tymi samymi funkcjami, które daje interfejs WWW legacy, spiętymi z grafiką VizRS. Operator kanału prowadzi w jednym miejscu kanały, ramówkę i grafikę VizRS. Pod spodem pracuje niezmienione ErsatzTV legacy (baza, skaner, planowanie), a nadaje ErsatzTV Next – silnik legacy (§2).

| Repozytorium / aplikacja | Rola |
|---|---|
| VizRS CG System | silnik grafiki: Artist, Trio, `/output`, API TCP / HTTP / SSE |
| VizRS CG Playout | sidecar dla ErsatzTV: komendy na start klipu, reklamy bez grafiki, wtapianie, live ingest |
| **VizRS Live** (przyszłość) | UI kanału nad ErsatzTV legacy: funkcje UI legacy plus spięcie z grafiką i live ingest VizRS |

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

Źródła: [github.com/ErsatzTV/legacy](https://github.com/ErsatzTV/legacy) (`ErsatzTV/Pages/*.razor`, `Shared/MainLayout.razor`, `Controllers/`), [ersatztv.org/docs](https://ersatztv.org/docs/). Aplikacja C#/.NET, UI w Blazor + MudBlazor, baza SQLite albo MySQL, licencja zlib. Legacy **rozwija się aktywnie** (wydania co tydzień, 26.11.x; baner strony mówi o poprawkach bezpieczeństwa „w dającej się przewidzieć przyszłości”). W wersji niewydanej usunięto silnik legacy: **wszystkie kanały nadaje już ErsatzTV Next**. UI to Blazor po stronie serwera; działa pod podścieżką (`ETV_BASE_URL`, np. `/etv`) za odwrotnym proxy, logowanie przez OpenID Connect, API przez klucz API albo JWT.

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


Inne oficjalne wejścia legacy dla narzędzi zewnętrznych: harmonogramy **Sequential** (pliki YAML), playout z **zewnętrznego pliku JSON**, **elementy graficzne** w YAML (z nich korzysta dziś element VizRS w VizRS CG Playout), odczyt `channels.m3u` i `xmltv.xml`.

## 4. Kierunek: nakładka na legacy

Zasada: **legacy liczy, VizRS Live pokazuje i spina z VizRS.** Baza, skaner mediów, budowanie playoutu, EPG i uruchamianie Next zostają w legacy. VizRS Live nie powiela tej logiki, więc nie trzeba odtwarzać ~157 tys. linii legacy ani ręcznie pilnować zgodności ramówek.

Jak uzyskać zakres funkcji UI legacy – warstwy, od najtańszej:

1. **UI legacy wewnątrz VizRS Live.** Legacy działa pod podścieżką (`ETV_BASE_URL=/etv`) za odwrotnym proxy VizRS Live, z jednym logowaniem (OpenID Connect). Od pierwszego dnia są wszystkie funkcje z §3 – nakładka obudowuje je, nie przepisuje.
2. **Oficjalne wejścia legacy** (§3.3): REST API (kanały, reset/continue playoutu, skan, sesje), API Scripted Schedule, pliki Sequential YAML, elementy graficzne YAML, M3U/XMLTV. Na nich powstają **własne widoki VizRS Live** tam, gdzie VizRS daje coś ponad legacy:
   - polski widok operatora kanału: co jest na antenie, co następne, stan kanałów i sesji;
   - ramówka z grafiką VizRS: strony VizRS przypięte do pozycji (dziś element VizRS w CG Playout), „Za chwilę” ([PLAN-za-chwile.md](PLAN-za-chwile.md));
   - sloty na żywo (live ingest z CG Playout);
   - podgląd kanału z grafiką i monitoring.
3. **Brakujące API – do legacy, nie obok.** Gdy własny widok potrzebuje funkcji, której API legacy nie ma (np. edycja kanału, pozycji harmonogramu, bloków), dokładamy endpoint REST nad istniejącymi komendami legacy jako **zmianę zgłaszaną do projektu ErsatzTV** (pull request), a nie we własnym forku. Tak samo **polskie tłumaczenie UI** (dziś pl: tylko menu, wspólne napisy i kanały) – do legacy, korzyść dla wszystkich.
4. **Bez zapisu wprost do bazy legacy** (SQLite/MySQL): omija walidację i przebudowę playoutu, psuje się przy każdej migracji.

Miejsce w rodzinie VizRS: VizRS Live jest **interfejsem** kanału, a VizRS CG Playout zostaje **wykonawcą** obok legacy (komendy na start klipu, reklamy bez grafiki, wtapianie, live ingest). VizRS Live konfiguruje Playout i pokazuje jego stan – nie dubluje go.

Na później, jako opcja: warstwa `canvas` i źródło `dynamic` z Next (§2) mogą zastąpić wtapianie i nakładkę ffmpeg w CG Playout – przez element graficzny legacy (Motion/Script), bez obchodzenia legacy.

## 5. Do ustalenia przed etapami

1. **Pierwszy zakres własnych widoków** (warstwa 2): który z czterech widoków jest najpilniejszy?
2. **Forma VizRS Live**: osobna aplikacja (proxy + widoki) w nowym repozytorium, czy rozszerzenie UI VizRS CG Playout (już stoi obok legacy, ma serwer HTTP i stan kanałów)?
3. **Współpraca z ErsatzTV**: czy zgłaszamy brakujące API i tłumaczenie pl do projektu (warstwa 3)? Do sprawdzenia z opiekunem legacy, czy przyjmie rozszerzenia API.
4. **Osadzenie UI legacy**: proxy pod jedną domeną z jednym logowaniem, czy tylko odnośniki do legacy w osobnej karcie.
