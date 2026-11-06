# VizRS Live – nakładka na ErsatzTV legacy z funkcjami jego UI

> Stan (2026-11-06): **plan dalekosiężny.** Rozpoznanie obu projektów zrobione (§2–§3, stan repozytoriów z 2026-11-05/06: `ErsatzTV/next` 92e93d7, `ErsatzTV/legacy` 06c008b). **Decyzja użytkownika (2026-11-06): VizRS Live jest nakładką na ErsatzTV legacy** – bez pisania własnego planisty, bazy i skanera („nie wymyślamy koła na nowo”), ale w zakresie funkcji UI legacy (§4). Dalsze decyzje (2026-11-06): **osobne repozytorium** (modułowość), **nic nie zgłaszamy do projektu ErsatzTV**, **cel: operator w ogóle nie korzysta z web GUI legacy** (§4.2). Etapów i terminu jeszcze nie ma – otwarte pytania w §5. Wizualizacja architektury i obu wersji dostępu do UI legacy (A/B, §4.1): [claude.ai/artifact/53Wbamc9qDDPJiBmrrSKws](https://claude.ai/artifact/53Wbamc9qDDPJiBmrrSKws).

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

1. **UI legacy dostępne z VizRS Live** (wersja A albo B, §4.1). Od pierwszego dnia są wszystkie funkcje z §3 – nakładka obudowuje je, nie przepisuje.
2. **Oficjalne wejścia legacy** (§3.3): REST API (kanały, reset/continue playoutu, skan, sesje), API Scripted Schedule, pliki Sequential YAML, elementy graficzne YAML, M3U/XMLTV. Na nich powstają **własne widoki VizRS Live** tam, gdzie VizRS daje coś ponad legacy:
   - polski widok operatora kanału: co jest na antenie, co następne, stan kanałów i sesji;
   - ramówka z grafiką VizRS: strony VizRS przypięte do pozycji (dziś element VizRS w CG Playout), „Za chwilę” ([PLAN-za-chwile.md](PLAN-za-chwile.md));
   - sloty na żywo (live ingest z CG Playout);
   - podgląd kanału z grafiką i monitoring.
3. **Bez zmian w ErsatzTV** (decyzja 2026-11-06): nie zgłaszamy API ani tłumaczeń i nie utrzymujemy forka. Granica jest stała – własne widoki robią tylko to, na co pozwalają istniejące API i pliki legacy; resztę (edycja kanałów, harmonogramów, bloków, mediów) operator robi w UI legacy. Po polsku są tylko widoki VizRS Live; UI legacy zostaje po angielsku (z jego częściowym pl).
4. **Bez zapisu wprost do bazy legacy** (SQLite/MySQL): omija walidację i przebudowę playoutu, psuje się przy każdej migracji.

### 4.1 Dostęp do UI legacy – dwie wersje

- **A – jedna domena, legacy za proxy.** Przeglądarka łączy się tylko z VizRS Live (np. `live.kanal.pl`). Ścieżki `/etv/*` VizRS Live przekazuje do legacy w sieci wewnętrznej; legacy ustawione na `ETV_BASE_URL=/etv`. UI legacy to Blazor po stronie serwera – działa na stałym połączeniu WebSocket (`/etv/_blazor`), które proxy też musi przekazywać. Logowanie raz: w VizRS Live (legacy osiągalne tylko przez proxy) albo przez wspólnego dostawcę OpenID Connect (legacy go obsługuje). Operator nie wychodzi z aplikacji: pasek VizRS Live na górze, pod nim ekran legacy (ten sam adres, więc może być w ramce), z widoku VizRS jedno kliknięcie do właściwego ekranu legacy. Koszt: proxy z WebSocketem, sesje, kontrola dostępu; zmiana ścieżek albo logowania w nowej wersji legacy może zepsuć proxy i głębokie odnośniki.
- **B – odnośniki do osobnej karty.** Legacy ma własny adres (np. `etv.kanal.pl` albo `:8409` w sieci lokalnej) i własne logowanie. Widoki VizRS Live mają przyciski „Edytuj w ErsatzTV ↗”, które otwierają właściwy ekran legacy w nowej karcie. Koszt prawie zerowy, nic się nie psuje przy aktualizacji legacy; minusy: dwa adresy, dwa logowania, przy dostępie z internetu dwa serwisy do zabezpieczenia (albo legacy tylko w sieci lokalnej / VPN).

Zalecenie: **start od B**, przejście do A, gdy kanał ma być obsługiwany z internetu pod jednym adresem z jednym logowaniem. A dokłada proxy przed tymi samymi odnośnikami, więc nie wymaga przebudowy.

### 4.2 Cel: bez web GUI legacy (decyzja 2026-11-06)

Docelowo operator robi wszystko w VizRS Live, a UI legacy jest zamknięte (tylko wewnętrzne API). Warstwy 1 i 3 z §4 tego nie dają: oficjalne API legacy obejmuje mało funkcji (§3.3), a bez niego kanałów, profili FFmpeg, bibliotek, harmonogramów, bloków ani playoutów nie da się założyć. Każdy ekran legacy wywołuje jednak w tym samym procesie gotową komendę z `ErsatzTV.Application` (187 komend MediatR, np. `CreateChannel`, `UpdateFFmpegProfile`, `CreateLocalLibrary`, `ReplaceProgramScheduleItems`, `CreateBlockPlayout`, `UpdateDeco`, `CreateWatermark`). Brakuje tylko drogi do nich z zewnątrz.

**Most VizRS ↔ legacy (propozycja, do sprawdzenia prototypem):** mała wtyczka .NET ładowana do procesu legacy bez zmiany jego kodu – mechanizm ASP.NET Core *hosting startup* (`ASPNETCORE_HOSTINGSTARTUPASSEMBLIES`; legacy używa `ConfigureWebHostDefaults` i publikuje się bez przycinania, więc DLL dołożony do `/app` powinien się załadować). Wtyczka dokłada kontrolery REST, które wywołują istniejące komendy i zapytania legacy, z jego uwierzytelnianiem kluczem API. Obraz: `FROM ghcr.io/ersatztv/legacy:<wersja>` + DLL + zmienna środowiskowa.

- **Ile C#, ile Rusta.** Część w procesie legacy musi być w .NET – legacy to aplikacja .NET, a komendy żyją w jego procesie (Rust nie wywoła ich bez pośrednika w .NET). Tę część można jednak zmniejszyć do **ogólnego dyspozytora** (~150–300 linii C#): jeden endpoint `POST /vizrs/command` z nazwą komendy i JSON-em, biała lista dozwolonych komend, deserializacja do rekordu komendy legacy (komendy to zwykłe rekordy, np. `CreateChannel(Name, Number, Group, …)`), wysłanie przez MediatR i zamiana wyniku (`Either<BaseError, T>`, `Option<BaseError>`) na `{ok, wynik | błąd}`. Wszystko inne – mapowanie ekranów na komendy, walidacja, logika VizRS, UI – w Ruście/JS w VizRS Live. Nowa funkcja legacy = wpis na białej liście, bez nowego kodu C#.
- Zgodne z decyzjami: logika zostaje w legacy (nie wymyślamy koła), kod legacy bez zmian, nic nie zgłaszamy, brak zapisu wprost do bazy.
- Cena: VizRS Live musi mieć **własne ekrany dla wszystkich funkcji z §3** (to główna praca – po stronie UI, nie logiki); wtyczka w C# (drugi język w rodzinie VizRS); komendy legacy to kod wewnętrzny, więc każde wydanie legacy (co tydzień) wymaga przebudowy i testu wtyczki – wersja legacy przypięta w compose, aktualizacja dopiero po zielonych testach.
- Nie wszystko da się przenieść 1:1: logowanie do Plex (przepływ w przeglądarce) i podobne – do sprawdzenia, ewentualnie na końcu.
- Dojście etapami: dopóki ekran VizRS Live nie istnieje, dana funkcja zostaje w UI legacy (odnośnik, wersja B z §4.1); po pokryciu wszystkich ekranów UI legacy jest zamykane.

Makieta UI VizRS Live (wszystkie ekrany legacy po polsku + widoki VizRS, z komendami legacy pod każdym ekranem): [claude.ai/artifact/9ajuiXeKEZD6ntYLxtAzbM](https://claude.ai/artifact/9ajuiXeKEZD6ntYLxtAzbM).

Odrzucone drogi do tego celu: **zapis wprost do bazy legacy** (omija walidację i przebudowę playoutu, psuje się przy migracjach – legacy ma ich kilka w miesiącu); **sterowanie UI legacy automatem przeglądarki** (kruche); **planista VizRS Live bezpośrednio dla Next bez legacy** (to pisanie legacy od nowa).

Miejsce w rodzinie VizRS: VizRS Live jest **interfejsem** kanału, a VizRS CG Playout zostaje **wykonawcą** obok legacy (komendy na start klipu, reklamy bez grafiki, wtapianie, live ingest). VizRS Live konfiguruje Playout i pokazuje jego stan – nie dubluje go.

Na później, jako opcja: warstwa `canvas` i źródło `dynamic` z Next (§2) mogą zastąpić wtapianie i nakładkę ffmpeg w CG Playout – przez element graficzny legacy (Motion/Script), bez obchodzenia legacy.

## 5. Do ustalenia przed etapami

1. **Pierwszy zakres własnych widoków** (§4 warstwa 2): który widok jest najpilniejszy – widok operatora, ramówka z grafiką VizRS, sloty na żywo, podgląd i monitoring?
2. **Most w procesie legacy** (§4.2): zgoda na wtyczkę .NET (hosting startup) jako jedyny element w C#? Pierwszy krok: prototyp – wtyczka z jednym endpointem (`CreateChannel`) w obrazie legacy, test na dwóch kolejnych wydaniach.
3. **Przejście**: do czasu pokrycia wszystkich ekranów brakujące funkcje przez odnośniki do UI legacy (§4.1 B).
4. **Technika VizRS Live**: serwer w Ruście + UI w czystym JS jak w VizRS (w osobnym repozytorium zależności dozwolone) – do potwierdzenia przy pierwszym etapie.

Ustalone: osobne repozytorium (2026-11-06), bez zgłoszeń do ErsatzTV (2026-11-06), cel bez web GUI legacy (2026-11-06).
