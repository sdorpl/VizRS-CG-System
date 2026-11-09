# PLAN: Optymalizacja VizRS Live pod emisję FFmpeg na Oracle Ampere A1

> Status: **Projekt techniczny (2026-11-09)**. Dotyczy optymalizacji panelu zarządczego `VizRS Live` (nakładka na ErsatzTV legacy + Next) na instancji Oracle Cloud Infrastructure (OCI Always Free) Ampere A1 (ARM64, 4 OCPU, 24 GB RAM, bez GPU).

## 1. Rola VizRS Live w torze emisyjnym

* **Zadanie:** VizRS Live jest panelem operatora i interfejsem konfiguracyjnym dla całego kanału emisyjnego:
  * Zarządza kanałami, bibliotekami mediów, harmonogramami i playoutami w ErsatzTV (przez lekki most C# .NET hosting startup w procesie legacy).
  * Konfiguruje **profile FFmpeg** (rozdzielczość, klatkaż, kodeki, presety, parametry skalowania i normalizacji audio/wideo).
  * Steruje grafiką VizRS i modułem live ingest (sloty na żywo, przejmowanie kanału).
* **Środowisko:** Instancja Ampere A1 współdzielona z procesami kodowania wideo FFmpeg, silnikiem odtwarzania ErsatzTV Next i serwerem grafiki `vizrs`.

---

## 2. Kluczowe optymalizacje środowiska uruchomieniowego .NET i procesu legacy

ErsatzTV legacy to aplikacja w środowisku .NET / C#. Na maszynie 4 OCPU bez optymalizacji proces .NET potrafi generować skoki zużycia procesora i alokacji pamięci.

### L1. Optymalizacja Garbage Collectora (GC) dla .NET na A1
* **Problem:** Domyślny tryb Server GC alokuje pamięć w dużych segmentach na każdy rdzeń CPU i wykonuje intensywne cykle czyszczenia, rywalizując o zasoby CPU z enkoderem wideo `libx264`.
* **Rozwiązanie:** W kontenerze ErsatzTV / VizRS Live włączyć Workstation GC lub sztywny limit sterty w zmiennych środowiskowych:
  ```bash
  # Użycie lżejszego Workstation GC zamiast agresywnego Server GC
  DOTNET_gcServer=0
  # Sztywny limit sterty GC (np. 1.5 GB z dostępnych 24 GB RAM)
  DOTNET_GCHeapHardLimit=0x60000000
  ```
* **Korzyść:** Brak nagłych zacięć procesora wywoływanych przez zatrzymanie świata (*Stop-the-World*) GC w trakcie kompresji wideo.

### L2. Wyłączenie ciężkiego interfejsu Blazor Server
* **Problem:** Standardowy interfejs webowy legacy oparty o Blazor Server utrzymuje stałe połączenia WebSocket (`/_blazor`) i przechowuje stan komponentów w pamięci serwera, obciążając wątki aplikacyjne.
* **Rozwiązanie:** Cel VizRS Live – zastąpienie web GUI legacy lekkim panelem w Rust + czysty JS (z mostem C# jako ogólnym dyspozytorem komend MediatR). Po wdrożeniu ekranów VizRS Live, web GUI Blazor zostaje wyłączone lub ograniczone wyłącznie do sieci wewnętrznej.

---

### 3. Wymuszanie optymalnych profili FFmpeg z poziomu VizRS Live

To VizRS Live (przez komendy `CreateFFmpegProfile` / `UpdateFFmpegProfile`) definiuje polecenia, z jakimi ErsatzTV Next uruchamia FFmpeg. Na Ampere A1 szablony profili muszą wymuszać:

1. **Format wideo kanału:**
   * Rozdzielczość: **1920×1080** (lub **1280×720** dla kanałów sportowych/dynamicznych).
   * Klatkaż: ściśle **25 fps** (`FrameRate = 25`).
   * Kodek wideo: `libx264`, profil `high`, poziom `4.1`.
   * Preset kodera: `veryfast` (wymuszony w profilu i wspierany przez `VIZRS_FFMPEG_PRESET`).
   * Długość GOP: 50 klatek (dokładnie 2 sekundy dla HLS, `keyint_min = 25`, brak adaptacyjnych B-frame'ów: `-bf 0` lub `-bf 2`).
2. **Format audio:**
   * Kodek: `aac`, próbkowanie: `48000 Hz`, stereo, bitrate: `192 kbps`.
   * Normalizacja głośności EBU R128 wyliczana w locie lub z metadanych skanera.
3. **Akceleracja i przeplot:**
   * Akceleracja sprzętowa wyłączona: `HardwareAcceleration = None` (czyste dekodowanie programowe).
   * Deinterlacing: wyłączony dla materiałów progresywnych (unikanie filtrów bwdif/yadif w torze wideo, jeśli pliki są 1080p).

---

## 4. Ograniczenie narzutu skanera mediów (Media Scanner)

Skaner biblioteki analizuje pliki wideo, generuje miniatury i odczytuje metadane przez `ffprobe`. Na 4 rdzeniach procesora równoległe skanowanie biblioteki podczas emisji grozi utratą płynności transmisji (*stuttering*).

* **Rozwiązania:**
  1. **Niski priorytet procesów skanera:** Uruchamianie procesów pomocniczych z niskim priorytetem CPU i I/O:
     ```bash
     nice -n 19 ionice -c 3
     ```
  2. **Harmonogramowanie skanowania:** W ustawieniach skanera wyłączyć automatyczne skanowanie ciągłe przy każdej zmianie katalogu (*file watcher*); skanowanie uruchamiać wyłącznie w zaplanowanych oknach serwisowych lub ręcznie na żądanie.
  3. **Wstępna normalizacja plików:** Wgrywanie plików z metadanymi i w formacie docelowym eliminuje potrzebę generowania próbkowania transkodowania podczas odtwarzania.

---

## 5. Integracja slotów na żywo i harmonogramu z VizRS CG Playout

1. **Zarządzanie listenerami live (`GET /ingest/listeners`):**
   * VizRS Live konfiguruje listę aktywnych portów dosyłowych (RTMP / SRT) dla `listener.rs`.
   * Dynamiczne włączanie i wyłączanie portów słuchających zapobiega niepotrzebnemu uruchamianiu procesów FFmpeg w tle dla nieużywanych kanałów dosyłowych.
2. **Przełączanie bez retranskodowania:**
   * Po wykryciu sygnału na żywo, harmonogram wykonuje przejęcie kanału (`takeover`), a wideo live jest przekazywane bezpośrednio do kompozytora bez dodatkowej warstwy transkodowania pośredniego.

---

## 6. Zestawienie konfiguracji środowiskowej VizRS Live na A1

W konfiguracji kontenera VizRS Live / ErsatzTV:

```env
# Optymalizacja .NET CLR na 4 OCPU ARM
DOTNET_gcServer=0
DOTNET_GCHeapHardLimit=0x60000000
DOTNET_EnableWriteXorExecute=0

# Konfiguracja integracji z silnikiem
VIZRS_LIVE_PORT=8092
VIZRS_LIVE_BIND=127.0.0.1
VIZRS_ETV_URL=http://127.0.0.1:8409

# Przydział procesora
CPUS_LIMIT=1
```
