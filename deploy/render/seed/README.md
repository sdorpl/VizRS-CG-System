Show startowy dla Render.com (darmowy plan nie ma trwałego dysku – po każdym restarcie/uśpieniu dane wracają do tego stanu).

Żeby zachować zmiany: pobierz show (`GET /api/show` z tokenem albo eksport w Artist/Trio),
zapisz tutaj jako `show.json` (pliki graficzne do `assets/`), zacommituj i zrób deploy.
Brak `show.json` → VizRS startuje z wbudowanym show demo.
