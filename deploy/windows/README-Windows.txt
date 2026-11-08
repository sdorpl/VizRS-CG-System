VizRS Trio dla Windows (x86_64)
================================

W paczce znajduja sie:
- start-trio.bat          - Uruchamia okno Trio z lokalnym serwerem grafiki
- start-trio-zdalnie.bat  - Okno wyboru polaczenia ze zdalnym serwerem (np. VPS w reżyserce)
- start-serwer.bat        - Serwer w oknie konsoli (dla konfiguracji sieciowych lub OBS)
- instaluj-skroty.bat     - Dodaje skroty do Menu Start biezacego uzytkownika

Pliki wykonywalne:
- vizrs-shell.exe  - Okno natywne (WebView2 / Microsoft Edge Runtime)
- vizrs-app.exe    - Okno w trybie aplikacji Edge (bez konsoli)
- vizrs.exe        - Glowny serwer grafiki CG i API

Wyjscie grafiki do OBS / vMix:
  W zrodle Browser (Przegladarka) podaj URL:
  http://localhost:8088/output
  Szerokosc: 1920, Wysokosc: 1080

Wymagania:
  Windows 10 lub 11 (zawiera wymagany Microsoft Edge WebView2).

Projektowanie scen:
  Osobny program VizRS CG Artist (https://github.com/sdorpl/VizRS-CG-Artist).
