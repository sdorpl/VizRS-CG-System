# PLAN: Optymalizacja VizRS CG System pod emisję FFmpeg na Oracle Ampere A1

> Status: **Projekt techniczny (2026-11-09)**. Dotyczy optymalizacji rdzenia VizRS CG System oraz natywnego renderera `vizrs-render` na bezpłatnej instancji Oracle Cloud Infrastructure (OCI Always Free) Ampere A1. Powiązany plan dla toru playoutu: [PLAN-playout-ampere-optymalizacja.md](PLAN-playout-ampere-optymalizacja.md).

## 1. Cel i uwarunkowania sprzętowe

* **Sprzęt:** Oracle Cloud Infrastructure Ampere A1 (VM.Standard.A1.Flex) – procesor Ampere Altra (architektura ARMv8.2-A, rdzenie Neoverse-N1, aarch64), do 4 OCPU i 24 GB RAM.
* **Kluczowe ograniczenie:** Brak dedykowanego GPU (brak NVENC/VAAPI/QuickSync, brak sprzętowego rasteryzatora OpenGL/Vulkan). Całość zadań – rasteryzacja grafiki, kompozycja kadrów i kodowanie wideo H.264/H.265 – spoczywa na 4 fizycznych rdzeniach procesora ARM.
* **Cel:** Utrzymanie stabilnej emisji 1080p25 z grafiką telewizyjną na żywo przy zużyciu poniżej 0,5 OCPU na cały podsystem grafiki (serwer VizRS + renderer), pozostawiając pozostałe ~3 OCPU na transkodowanie i nakładanie w FFmpeg.

---

## 2. Podział ról w ekosystemie

1. **Aplikacje emisyjne na Ampere A1:**
   * `vizrs serve`: serwer HTTP/SSE/TCP (lekki proces w Rust, zużycie CPU < 0,5%).
   * `vizrs-render`: natywny renderer w Rust (`tiny-skia` + `rustybuzz`) uruchamiany jako proces podrzędny sidecara (`vizrs-render stream`).
2. **Aplikacje stacji roboczej (uruchamiane lokalnie, poza instancją A1):**
   * `VizRS CG Artist` (Tauri 2): projektowanie scen przez grafika.
   * `vizrs-shell`: okno powłoki desktopowej z WebKitGTK/WebView2 (nieuruchamiane na bezgłowym serwerze).
   * Panele operatora Trio (`/trio`) i Mobile (`/mobile`): operatorzy łączą się zdalnie przez przeglądarkę pod adresem publicznym serwera.

---

## 3. Plan optymalizacji renderera natywnego (`vizrs-render`)

Bezgłowy Chromium w trybie CPU zużywa na Ampere A1 100–180% CPU (1–2 rdzenie). Zastąpienie go przez `vizrs-render` redukuje ten koszt do kilku procent rdzenia.

### R1. Kompilacja pod mikroarchitekturę Neoverse-N1
* **Działanie:** Włączenie dedykowanych flag kompilacji dla architektury ARMv8.2-A w procesie budowania binarki `vizrs-render`.
* **Konfiguracja:**
  ```bash
  RUSTFLAGS="-C target-cpu=neoverse-n1 -C opt-level=3" cargo build --release
  ```
* **Korzyść:** Aktywacja wektoryzacji NEON w pętlach przetwarzania `tiny-skia` i rustybuzz, lepsze planowanie potoków instrukcji procesora Ampere Altra.

### R2. Wykorzystanie wycinków `REGION` (protokół strumieniowy `VZR1`)
* **Działanie:** Pełna klatka 1080p w formacie RGBA to 8,3 MB danych. Renderer w `source/render/src/stream.rs` ma wbudowaną obsługę rekordu `REGION` (rodzaj 4) oraz `REGIONS` (rodzaj 5).
* **Konfiguracja:** Uruchamianie renderera z parametrem `--area` dla scen o stałym położeniu (np. ticker dolny `--area 0,960,1920,120`).
* **Korzyść:** Redukcja kopiowania pamięci i przepustowości IPC z 8,3 MB do ok. 0,9 MB na klatkę (oszczędność ~89% transferu pamięci RAM).

### R3. Eliminacja podwójnej konwersji alfy (`premultiplied`)
* **Działanie:** `tiny-skia` przechowuje piksele w formacie Premultiplied RGBA. Obecnie w `stream.rs` funkcja `unpremultiply` w pętli dzieli wartości kanałów przez alfę. FFmpeg filtr `overlay` wspiera wejście z premultiplied alpha (`alpha=premultiplied`).
* **Krok:** Wprowadzenie flagi lub opcji w `stream.rs`, która pomija `unpremultiply` przy bezpośrednim potoku do FFmpeg.

### R4. Bezpośrednie wyjście `yuva420p` (etap N4)
* **Działanie:** Renderowanie lub konwersja wycinków grafiki bezpośrednio do formatu `yuva420p` w pamięci renderera natywnego przed przekazaniem do FFmpeg.
* **Korzyść:** Wyeliminowanie kosztownego filtru konwersji przestrzeni barw wewnątrz grafu filtrów FFmpeg.

---

## 4. Plan optymalizacji serwera `vizrs`

1. **Konfiguracja bez zbędnych usług:**
   * Wyłączenie portu UDP OSC (`--osc 0`).
   * Port komend TCP 6100 ograniczony do nasłuchu lokalnego (`127.0.0.1` lub gniazdo unixowe).
2. **Udostępnianie wyjścia bez odpytywania Chromium:**
   * Po wdrożeniu `vizrs-render stream` serwer nie musi obsługiwać połączeń DevTools screencast. Wyjście `/output` pozostaje dostępne wyłącznie na żądanie dla podglądu operatora w Trio.
3. **Wieloarchitekturowy obraz Docker (`linux/arm64`):**
   * Statyczna binarka `vizrs` budowana na bazie `rust:1-alpine` (`aarch64-unknown-linux-musl`), pakowana do pustego obrazu `scratch` (obraz o rozmiarze ~3 MB).

---

## 5. Weryfikacja szablonów i paczek graficznych

1. **Badanie zgodności scen:**
   * Przed wdrożeniem na emisję, każda paczka `.cgpack` i show musi przejść weryfikację:
     ```bash
     vizrs-render check --show show.json
     ```
   * Sceny z brakami (np. nieobsługiwane filtry, zewnętrzne fonty) muszą zostać dostosowane, aby nie powodowały przełączania na ciężki fallback Chromium (`renderer: auto`).
2. **Czcionki offline:**
   * Używanie wyłącznie fontów wbudowanych (Barlow, Bebas Neue, Inter, Roboto, Montserrat) lub dołączonych bezpośrednio do katalogu `assets/` sceny.

---

## 6. Etapy wdrożenia

| Krok | Zakres | Kryterium sukcesu |
|---|---|---|
| **A1** | Benchmark `vizrs-render bench` na instancji Ampere A1 | Czas klatki crawla 1080p25 poniżej 4 ms na 1 rdzeniu A1 |
| **A2** | Kompilacja `linux/arm64` z flagami `-C target-cpu=neoverse-n1` | Poprawny start statycznej binarki w kontenerze Docker na A1 |
| **A3** | Tryb `alpha=premultiplied` w `stream.rs` bez pętli `unpremultiply` | Zmniejszenie czasu przygotowania klatki o min. 15% |
| **A4** | Test obciążeniowy 24h na A1 z pełnym show INFO24 | Zużycie procesora przez procesy grafiki poniżej 0,5 OCPU |
