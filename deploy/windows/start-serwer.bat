@echo off
rem Uruchamia sam serwer VizRS w konsoli (bez otwierania okna GUI).
rem UI dostepne pod adresem: http://localhost:8088/trio
rem Wyjscie dla OBS:          http://localhost:8088/output
cd /d "%~dp0"
"vizrs.exe" --data "vizrs-data" serve
if errorlevel 1 pause
