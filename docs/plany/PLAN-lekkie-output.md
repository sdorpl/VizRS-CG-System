# PLAN – lekkie `/output` dla bezgłowego Chromium (Ampere A1)

Status: **L1–L4 wdrożone** (2026-10-24): `?fps=N`, `?fx=step` (efekty ruchowe wyłączone), uśpienie pętli bez stron,
wskazówki w DOKUMENTACJA §10 i instrukcji; test klatek screencastu w `tests/e2e/motion.js`. L5 (wskaźnik w Artist) – później.
Decyzje z §6: osobne parametry `fps` i `fx=step` (bez `lite=1`); w L2 efekty ruchowe wyłączone.
Dotyczy wypalania `/output` w obraz kanału przez bezgłowy Chromium bez GPU: VirtRS CG Playout (ErsatzTV)
(wcześniej także VizRS ffPlayout System – od 2026-10-24 archiwum). Docelowo Oracle Cloud Free, Ampere A1 (arm64, do 4 OCPU), gdzie grafika ma się
zmieścić w ok. 1,5 OCPU obok kodowania x264. OBS, vMix i okno `vizrs output` zostają bez zmian. Wszystko,
co niżej, włącza się parametrem adresu.

## 1. Definicja

**Lekkie `/output`** to wyjście, które po wejściu stron, a przed ich wyjściem zmienia obraz tylko wtedy,
gdy zmienia się treść, i nigdy częściej niż wynosi klatkaż kanału.

Miarą jest **liczba zmienionych klatek na sekundę**, a nie wielkość grafiki ani liczba węzłów.
Kryteria w 1080p, Chromium bez GPU:

| # | Sytuacja | Cel | Dziś |
|---|---|---|---|
| K1 | strona statyczna (belka, podpis, logo, QR) po animacji wejścia | 0 kl./s | spełnione |
| K2 | zegar, odliczanie, wynik z zegarem | ≤ 1–2 kl./s | spełnione (~1,2) |
| K3 | ruch ciągły (ticker crawl) | ≤ klatkaż kanału (25 albo 50) | 60 Hz – albo tyle, ile Chromium nadąży |
| K4 | efekty uwagi (`blink`, `motion.loop`, `motion.bg`) | kilka zmian na okres | płynnie, 60 Hz |
| K5 | animacje wejścia i wyjścia (TAKE, OUT) | ≤ klatkaż kanału, krótko | 60 Hz, krótko – nie szkodzi |

## 2. Skąd się bierze koszt

Sidecar odbiera obraz przez DevTools `Page.startScreencast`. Każda klatka kompozytora, w której coś
się zmieniło, oznacza dla Chromium odczyt **całego** obszaru 1920×1080, kodowanie PNG i wysłanie go
w base64. Sidecar potem dekoduje PNG. Statyczna strona nie generuje klatek (K1), więc nic nie kosztuje.

Wniosek z pomiarów: **jedna zmieniona klatka kosztuje prawie tyle samo, niezależnie od tego, co się
zmieniło.** Mrugająca kropka 14 px w „Na żywo” kosztuje tyle co przewijany pasek na całą szerokość
ekranu.

Globalna pętla `requestAnimationFrame` w `renderer.js` wywołuje `apply()` 60 razy na sekundę. Gdy
wartości stylów są takie same jak poprzednio, Chromium niczego nie maluje (K1 działa). Wystarczy jednak
jedna właściwość liczona z czasu, np. krycie `blink` z cosinusa, żeby każda klatka 60 Hz była nowa.

## 3. Pomiary

Środowisko:
- Xeon E3-1270 v3 (x86, 8 wątków), Google Chrome 156 headless, VizRS 2.0.1, `demo_show.json`;
- przechwytywanie `Capture` z VirtRS CG Playout w 1920×1080;
- 6 s pomiaru, 2 s po TAKE.

CPU to suma procesów Chromium oraz proces Node sidecara (dekodowanie PNG i wycinanie prostokątów).

| Strona demo | Chromium | Node | kl./s |
|---|---|---|---|
| pusto | 5% | 1% | 0 |
| belki, podpisy, tytuł, QR, hashtag, belka programu (statyczne) | 7–10% | 1–2% | 0 |
| 4101 Pilne z logo (zegar HH:mm) | 10% | 2% | 0 |
| 2101 Odliczanie, 5001 Wynik meczu (zegar) | 14–16% | 5% | 1,2–1,3 |
| 3003 Ticker wipe, 3002 Ticker góra–dół | 37–43% | 13–17% | 4–6 |
| 1405 Kafel programu (`cycle`) | 41–47% | 15–16% | 5–6 |
| 3001 / 3004 Ticker crawl | 148–190% | 46–67% | 19–34 |
| **2001 / 1404 Na żywo (`blink` kropki)** | **167–186%** | **47–73%** | **24–30** |

Prototyp zmian z §4 (kopia `web/` poza repo, ten sam pomiar):

| Strona | dziś | `?fps=25` | `?fps=25` + schodkowe miganie |
|---|---|---|---|
| 2001 Na żywo | 186% / 47%, 26,7 kl./s | 105% / 30%, 23,7 kl./s | **12% / 3%**, 1,5 kl./s |
| 3001 Ticker crawl | 182% / 46%, 24,2 kl./s | **100% / 32%**, 23,8 kl./s | 100% / 30% |
| 1405 Kafel (`cycle`) | 47% / 15%, 6,3 kl./s | 31% / 9%, 6,0 kl./s | 32% / 9% |

(Chromium / Node, klatki/s.) Chromium nadal potrzebuje ok. 4% CPU na każdą klatkę na sekundę.
`chromium-headless-shell`, którego używa obraz VirtRS CG, zmniejsza to mniej więcej o połowę.
Arm64 jeszcze nie zmierzony (A1 to Neoverse N1, 1 OCPU = 1 rdzeń bez SMT); spodziewany koszt jest
podobny albo wyższy.

## 4. Proponowane zmiany w VizRS

### L1. `/output?fps=N` – pętla animacji w klatkażu kanału

Globalna pętla w `renderer.js` uruchamia `tick()` najwyżej N razy na sekundę. `dt` dalej liczy się
z zegara, więc tempo animacji się nie zmienia, rzadsze jest tylko próbkowanie. Próg zależny od
poprzedniego wywołania daje przy 60 Hz tylko 20 kl./s. Trzeba liczyć termin następnej klatki:

```js
let last = 0, nextAt = 0;
function loop(now) {
  if (FPS) {
    if (now < nextAt - 1000 / 120) { requestAnimationFrame(loop); return; }
    nextAt = Math.max(nextAt + 1000 / FPS, now);   // przy 60 Hz na zmianę co 2 i 3 klatki, średnio 25/s
  }
  // … dotychczasowa pętla
}
```

- Bez parametru wszystko działa jak dziś (OBS, podgląd w Trio, kanwa Artist).
- Wynik: ticker crawl o ~45% taniej, pozostałe animacje ciągłe podobnie.
- Przy 50 kl./s zysk jest mały (60 → 50 Hz). Kanał 1080p50 z animacjami ciągłymi to na A1 osobne pytanie
  (zob. [PLAN-kanal.md](PLAN-kanal.md) §11, pkt 4).
- Nie dotyczy wideo w węźle `image`: element `<video>` odtwarza się we własnym tempie i każda jego
  klatka to nowa klatka screencastu.

### L2. `/output?fx=step` – efekty uwagi schodkami zamiast płynnie

W trybie „schodkowym” efekty liczone z zegara zmieniają obraz tylko kilka razy na okres:

| Efekt | Dziś | `fx=step` |
|---|---|---|
| `blink` (okres s) | krycie z cosinusa, 60 Hz | 2 stany: pełne i 0,15, po pół okresu |
| `motion.loop` (`pulse`, `breathe`, `glow`, `flash`, `heartbeat`…) | płynnie | stan próbkowany 4 razy na okres |
| `motion.loop` ruchowe (`float`, `bounce`, `wobble`, `shake`), `shine`, `underline-sweep` | płynnie | wyłączone (pozycja spoczynkowa) |
| `motion.bg` (`shimmer`, `pulse`, `glow`, `scanline`) | płynnie | wyłączone albo stan próbkowany 4 razy na okres |

- Wynik dla `blink` zmierzony: z ~180% do ~12%, czyli poziom grafiki statycznej.
- Wygląd na kanale różni się od OBS, więc to świadomy wybór kanału, a nie nowe zachowanie
  domyślne. Projektant widzi różnicę w podglądzie `/output?fx=step&checker=1`.

### L3. Pętla bez stron na antenie

Gdy `players` jest puste, pętla dalej wywołuje `requestAnimationFrame` 60 razy na sekundę.
Bez stron na antenie Chromium i tak zużywa tylko ok. 5% CPU, więc zysk jest mały. Do zrobienia przy
okazji L1: zatrzymać pętlę przy pustym `players` i wznowić przy pierwszym `new ScenePlayer`.

### L4. Wskazówki dla projektanta scen kanału (do `INSTRUKCJA-poczatkujacy.md` i `DOKUMENTACJA.md`)

- **Ticker:** tryby `roll` i `wipe` kosztują ok. 4 razy mniej niż `crawl`, bo między zmianami
  wiadomości obraz stoi.
- **Efekty ciągłe:** `blink`, pętle uwagi i efekty tła na grafikach, które wiszą godzinami (logo,
  „Na żywo”), to stały koszt pełnego klatkażu. Na kanale wypalanym lepiej z nich zrezygnować
  albo użyć `fx=step`.
- **Zegar:** format bez sekund (`HH:mm`) zmienia obraz raz na minutę, z sekundami raz na sekundę.
  Oba są tanie.
- **Rotacja (`cycle`):** tania między przejściami. Koszt to głównie samo przejście.
- **Wideo w grafice:** nie na kanale wypalanym. Każda klatka filmu to pełna klatka screencastu.
- **Rozmiar elementu nie ma znaczenia:** liczy się tylko to, czy obraz się zmienia.

### L5. (później, G3) Wskaźnik kosztu w Artist

Artist mógłby wskazywać sceny z animacją ciągłą w stanie ustalonym (`blink`, `motion.loop`,
`motion.bg`, ticker `crawl`, wideo), np. ikoną „ciągły ruch” na liście scen.

## 5. Spójność z playoutami

- VirtRS CG Playout: `output_url` / `VIZRS_OUTPUT_URL` dostaje `&fps=25` (i ewentualnie
  `&fx=step`). Sidecar nie zmienia adresu sam, bo klatkaż kanału zna dopiero z danych ErsatzTV,
  a jedna strona przechwytywania obsługuje wszystkie kanały danego rozmiaru.
- VizRS ffPlayout System: archiwum od 2026-10-24, bez zmian.
- Po wdrożeniu parametry trafiają do `docs/API.md` (sekcja „Wyjście `/output`”) i do dokumentacji
  VirtRS CG Playout.

## 6. Decyzje do podjęcia

1. Nazwy parametrów: `fps` i `fx=step` (propozycja) albo jeden `?lite=1` (25 kl./s + schodki).
2. Czy w L2 efekty ruchowe wyłączać, czy też próbkować schodkami (np. `float` w 4 pozycjach).
3. Kolejność: L1 i L2 razem z testem e2e (Playwright: liczba klatek screencastu na stronie
   z `blink` i z crawl, przy `?fps=25` około 25 kl./s, przy `fx=step` najwyżej kilka na sekundę).
   Potem L4 w dokumentacji, L3 przy okazji, L5 później.
