@echo off
rem Uruchamia VizRS Trio (okno natywne playoutu + serwer w tle).
rem Dane przechowywane sa w folderze vizrs-data obok tego pliku.
cd /d "%~dp0"
if exist "vizrs-shell.exe" (
    start "" "vizrs-shell.exe" --vizrs "vizrs.exe" --data "vizrs-data" trio
) else if exist "vizrs-app.exe" (
    start "" "vizrs-app.exe" --data "vizrs-data" trio
) else (
    start "" "vizrs.exe" --data "vizrs-data" trio
)
