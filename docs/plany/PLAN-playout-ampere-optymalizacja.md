# PLAN: Optymalizacja VizRS CG Playout i toru FFmpeg na Oracle Ampere A1

> Status: **Projekt techniczny (2026-11-09)**. Dotyczy optymalizacji procesów transkodowania, nakładania grafiki (`overlay`) oraz wejść sygnału na żywo w repozytorium `VizRS CG Playout` (`VirtRS CG Playout`) z wykorzystaniem wrappera `vizrs-ffmpeg` i silnika ErsatzTV Next na instancji Oracle Cloud Infrastructure (OCI Always Free) Ampere A1.

## 1. Kontekst architektoniczny

* **Środowisko:** Maszyna wirtualna OCI Always Free – 4 OCPU (Ampere Altra / Neoverse-N1 ARM64), 24 GB RAM, bez GPU.
* **Rola modułu:**
  * `vizrs-ffmpeg` podstawia się jako binarka FFmpeg w ErsatzTV, przechwytując transkodowanie strumieni do formatu HLS.
  * Zamiast canvasu HTTP z ErsatzTV, wrapper wstrzykuje przez gniazdo unixowe (`pipe:N`) surowe ramki `yuva420p` renderowane przez sidecar VizRS.
  * Moduły `listener.rs` i `takeover.rs` odbierają sygnały na żywo (RTMP/SRT) i przełączają kanał na audycję live z nałożoną grafiką.

---

## 2. Kluczowe optymalizacje FFmpeg i wrappera `overlay.rs`

### P1. Wymuszenie stałego klatkażu kanału 1080p25
* **Problem:** Materiały w bibliotece emisyjnej posiadają zróżnicowany klatkaż (np. 29.97, 30, 50, 59.94 fps). Przetwarzanie 50/60 fps na CPU bez akceleracji GPU powoduje 100% obciążenia rdzeni i gubienie klatek.
* **Konfiguracja:**
  ```bash
  VIZRS_FFMPEG_FPS=25
  VIZRS_FFMPEG_FPS_BLEND=1
  ```
* **Działanie:** Wrapper wymusza konwersję do 25 fps przed filtrem grafiki, a dla materiałów 30/60 fps wykonuje blendowanie klatek (`framerate=fps=25`) eliminując szarpanie ruchu (*judder*). Zmniejsza to o 50% obciążenie filtru `overlay` oraz enkodera `libx264`.

### P2. Preset enkodera `libx264` i ograniczenie liczby wątków
* **Problem:** Domyślny preset ErsatzTV (`medium`) jest zbyt wolny dla programowego kodowania 1080p w czasie rzeczywistym na rdzeniach ARM Neoverse-N1. Z kolei niekontrolowana liczba wątków zabiera zasoby serwerowi grafiki i systemowi.
* **Konfiguracja:**
  ```bash
  VIZRS_FFMPEG_PRESET=veryfast
  ```
* **Parametry x264 wstrzykiwane przez profil:**
  * `-preset veryfast` (optymalna równowaga jakość/CPU dla Neoverse-N1).
  * `-tune zerolatency` (minimalizacja buforowania i opóźnienia potoku).
  * `-threads 2` (maksymalnie 3, aby zarezerwować co najmniej 1 fizyczny rdzeń dla filtrów, grafiki i systemu).

### P3. Optymalizacja filtru `overlay`
* **Problem:** Standardowe blendowanie pełnej klatki 1920×1080 RGBA/YUVA na klatkę wideo w każdej klatce zużywa cenne cykle CPU.
* **Działania w potoku filtrów:**
  1. Parametr `shortest=0:repeatlast=1` – zapobiega przerywaniu strumienia przy chwilowym braku nowej klatki grafiki.
  2. Użycie `alpha=premultiplied` – eliminuje konieczność dzielenia składowych barwnych w procesie grafiki.
  3. Nakładanie wycinków (`REGION`) – dla grafiki zajmującej stały fragment (np. ticker dolny):
     ```bash
     [0:v][gfx]overlay=x=0:y=960:format=auto:alpha=premultiplied:repeatlast=1
     ```

### P4. Płynność strumieniowania HLS
* **Konfiguracja:**
  ```bash
  VIZRS_FFMPEG_READRATE_CATCHUP=2
  ```
* **Działanie:** W przypadku chwilowego spadku wydajności procesora FFmpeg przyspiesza odczyt wejścia HLS, aby natychmiast dogonić czoło transmisji (*live edge*) zamiast kumulować opóźnienie.

---

## 3. Optymalizacja modułów Live Ingest (`listener.rs`, `ingest.rs`)

1. **Bezwzględne zero-transcode na wejściu:**
   * Sygnały dosyłowe (RTMP porty 1936–1943, SRT porty 9000–9007) są demuxowane i przekazywane do sidecara jako czysty MPEG-TS bez dekodowania (`-c copy`).
2. **Statyczna plansza awaryjna (Standby):**
   * Zdefiniowanie pliku graficznego w `VIZRS_FFMPEG_STANDBY=/assets/standby.png` gwarantuje, że przy zerwaniu dosyłu live FFmpeg nie zużywa mocy CPU na generowanie syntetycznych klatek, a widz widzi estetyczną planszę stacji.

---

## 4. Przydział rdzeni procesora (CPU Affinity) na Ampere A1

Instancja dysponuje 4 fizycznymi rdzeniami bez SMT. W konfiguracji Docker compose przypisujemy zasoby za pomocą `cpuset`:

```yaml
services:
  # Główny proces kodowania i odtwarzania ErsatzTV / FFmpeg
  ersatztv:
    image: ersatztv:latest
    cpuset: "0,1,2"
    environment:
      VIZRS_FFMPEG_FPS: "25"
      VIZRS_FFMPEG_PRESET: "veryfast"
      VIZRS_FFMPEG_FPS_BLEND: "1"
      VIZRS_FFMPEG_READRATE_CATCHUP: "2"

  # Sidecar i silnik grafiki VizRS (serwer + renderer natywny + caddy)
  vizrs:
    image: vizrs:latest
    cpuset: "3"
    environment:
      VIZRS_BIND: "127.0.0.1"
      VIZRS_PORT: "8088"
      VIZRS_TCP: "6100"
```

---

## 5. Podsumowanie zmiennych środowiskowych emisyjnych

W pliku `.env` środowiska emisyjnego na Ampere A1 należy skonfigurować:

```bash
# Playout / FFmpeg
VIZRS_CG_URL=http://127.0.0.1:7700
VIZRS_FFMPEG=/usr/local/bin/ffmpeg
VIZRS_FFMPEG_FPS=25
VIZRS_FFMPEG_PRESET=veryfast
VIZRS_FFMPEG_FPS_BLEND=1
VIZRS_FFMPEG_READRATE_CATCHUP=2
VIZRS_FFMPEG_INGEST=1
VIZRS_FFMPEG_STANDBY=/data/assets/standby.png
VIZRS_FFMPEG_CUDA=

# Architektura
RUSTFLAGS="-C target-cpu=neoverse-n1"
```
