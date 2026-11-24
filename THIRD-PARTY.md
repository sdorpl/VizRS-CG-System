# Składniki zewnętrzne

Kod VizRS CG System: MIT ([LICENSE](LICENSE)). Serwer `vizrs` nie ma zależności zewnętrznych. Rozpowszechniane są:

| Składnik | Gdzie | Licencja |
|---|---|---|
| Fonty Barlow, Barlow Condensed, Bebas Neue, Inter, Montserrat, Oswald, Roboto, Roboto Condensed, Roboto Mono | `source/web/fonts/` (wbudowane w `vizrs` i `vizrs-render`) | SIL Open Font License 1.1 – tekst w `source/web/fonts/<font>/LICENSE.txt` |
| crate'y okna `vizrs-shell`: tauri, tauri-plugin-dialog, tauri-plugin-single-instance, serde_json, webkit2gtk (+ zależności przechodnie) | `source/shell/` (paczki okna) | MIT / Apache-2.0 (wersje: `source/shell/Cargo.lock`) |
| crate'y renderera `vizrs-render`: tiny-skia, rustybuzz, wuff, serde_json, png, jpeg-decoder, image-webp | `source/render/` | BSD-3-Clause (tiny-skia), MIT / Apache-2.0 (pozostałe; wersje: `source/render/Cargo.lock`) |
| WebView2 Runtime (instalator Windows) | instalator Inno Setup | licencja Microsoft – pobierany przez instalator, nie dołączany |

Nazwy krzywych ruchu (`power2.out`, `back.out(1.7)` …) pochodzą z konwencji GSAP; implementacja (`source/web/motion.js`) jest własna.
