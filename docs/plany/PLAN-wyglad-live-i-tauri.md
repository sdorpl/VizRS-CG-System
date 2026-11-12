# PLAN – wygląd VizRS Live w całym pakiecie i aplikacje na Tauri 2

> Decyzje użytkownika z 2026-11-12. Jeden etap = jedna sesja = jeden commit (w repo, którego dotyczy). Stan etapów: kolumna „Stan”.

## Decyzje

1. **Wzorem wyglądu jest panel VizRS Live** (`../VizRS Live/server/web/style.css`, „Wygląd 1.3”): tokeny powierzchni (`--bg-deep` … `--panel3`), promienie, cienie, bursztynowy akcent, czerwień = antena, pigułki stanu z kropką, menu z ikonami zwijane do paska ikon, a na wąskim ekranie (≤ 760 px) szuflada spod ☰, okna na całą szerokość na telefonie. Ten wygląd dostają **VizRS CG Trio** (strona www i aplikacja) i **VizRS CG Artist** (aplikacja; Artist w przeglądarce – `/artist` w serwerze – porzucamy docelowo, zgodnie z planem A7 do 3.0).
2. **Jeden frontend.** Strona jest tym samym kodem w przeglądarce i w oknie Tauri: czysty JS (moduły ES), bez bundlera, npm i `@tauri-apps/api`; zasada „zero zależności” serwerów `vizrs` i `vizrs-live` zostaje. Okno rozmawia ze stroną jednym mostem jak w Artist: `window.__vizrsPost('app:' + JSON)` → komenda Tauri `ipc` z białą listą komunikatów (`bridge.rs`), odpowiedzi przez `window.__vizrs*`. `web/native.js` w CG System już to obsługuje.
3. **Model „strona www + aplikacja natywna (klient)”** zostaje w CG System i przechodzi do Live:
   - **VizRS CG Trio:** `/trio` w przeglądarce (stanowisko i telefon – zrobione) oraz okno natywne, które uruchamia `vizrs serve --idle-exit` albo dołącza do serwera (lokalnego lub zdalnego). Okno przechodzi z `vizrs-shell` (tao + wry + muda + rfd) na **Tauri 2** z tymi samymi wtyczkami co Artist.
   - **VizRS Live:** panel w przeglądarce (bez zmian) oraz **aplikacja Tauri 2** w repo Live: tryb panelu lokalnego (uruchamia `vizrs-live` z konfiguracją) albo **klient** zdalnego panelu – okno wyboru serwera, token trzymany w oknie i dodawany przez pośrednika w serwerze strony okna (jak `local::start_remote` w Artist), strona nie zna tokenu.

## Etapy

| Etap | Repo | Zakres | Gotowe, gdy | Stan |
|---|---|---|---|---|
| W0 | CG System | Trio i Trio na telefonie jako jedna strona `/trio`, układ dotykowy jak w Live (≤ 760 px, szuflada ☰, pasek TAKE, widoki przesuwane palcem), `/mobile` → `/trio`, PWA `trio-*` | `tests/e2e/mobile.js` i `run.js` zielone | zrobione (2026-11-12) |
| W1 | CG System → Artist, Live | **Wspólny rdzeń stylu** `vizrs-ui.css` (tokeny i kontrolki Live: przyciski, pola, pigułki, tabele, okna, szuflada, karty) w `source/web/`, kopiowany do Live (`server/web/`) i do Artist (`tools/sync-cg.sh`) z kontrolą identyczności (`--check` w obu repo, CI). `ui.css` i `style.css` zostają tylko z tym, co swoje dla aplikacji | Live wygląda jak dziś (zrzuty przed/po), `sync --check` zielone w obu repo | do zrobienia |
| W2 | CG System | **Trio w wyglądzie Live** (www i okno): pasek górny jak `topbar` Live, panele jako karty, pigułki stanu z kropką (na antenie pulsuje), przyciski komend w stylu Live z zachowaniem kolorów TAKE/CONTINUE/OUT/UPDATE, okna pytań (`dialogs.js`) jak `.dialog` Live; ⚙ Stan jako okno Live | `run.js`, `mobile.js`, `app.js` zielone, zrzuty stanowiska i telefonu przejrzane | do zrobienia |
| W3 | Artist | **Artist w wyglądzie Live** (aplikacja): `web/artist.html` i `web/native.css` na rdzeniu `vizrs-ui.css` (z W1); `/artist` w CG System bez zmian (zamrożony) | testy Artist zielone, zrzuty | do zrobienia |
| T1 | CG System | **Okno Trio na Tauri 2** zamiast `vizrs-shell`: `source/shell/` przepisany na Tauri (tauri, tauri-plugin-dialog, tauri-plugin-single-instance, serde_json, webkit2gtk – jak Artist), ta sama rola i opcje CLI (`trio`, `output`, `--connect`, okno połączenia), most `bridge.rs` z białą listą, menu, pytanie przy zamknięciu, stan anteny w tytule; paczki `.deb`/AppImage/MSI/`.dmg` z bundlera Tauri zamiast `deploy/` dla okna | `tests/e2e/shell.js`, `native-smoke.js`, `tools/engines.js` zielone na nowym oknie; CI Windows/macOS | do zrobienia |
| T2 | Live | **Aplikacja VizRS Live na Tauri 2** (`desktop/` w repo Live, osobny workspace – wyjątek od „zero zależności” tylko tam, jak `source/shell/` w CG System): panel lokalny albo klient zdalnego panelu, okno wyboru serwera, pośrednik z tokenem, most strony (wspólny `native.js`), menu, jedna instancja; paczki z bundlera Tauri | test okna (wzór: `shell.js`), panel w przeglądarce bez zmian (`tests/deploy.sh`) | do zrobienia |
| T3 | CG System | Usunięcie `/artist` z serwera (3.0, plan A7) | – | później |

Kolejność: W1 → W2 → W3 → T1 → T2 (decyzja użytkownika: najpierw wygląd). T1 i T2 mogą korzystać z kodu okna Artist (połączenie, most, pośrednik) – wspólne części warto przenieść do jednego wzorca, ale bez wspólnego crate'a między repo, dopóki użytkownik o tym nie zdecyduje.

## Uwagi

- `ticker-items.js` i `ui.css` zmieniły się w W0 – Artist musi uruchomić `tools/sync-cg.sh` (inaczej `sync-cg.sh --check` w jego CI nie przejdzie).
- Nowe zależności: Tauri w `source/shell/` (T1) i w `desktop/` Live (T2) zastępują dopuszczone dziś crate'y `vizrs-shell`; nagłówki `Cargo.toml` i zasady w `CLAUDE.md` obu repo trzeba zaktualizować w tych etapach.
