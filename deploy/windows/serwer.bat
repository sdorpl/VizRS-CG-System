@echo off
rem Skrot "VizRS Serwer" z instalatora: sam serwer w konsoli, dane uzytkownika jak w oknie Trio.
rem Dane: %LOCALAPPDATA%\VizRS; dane z instalatora 2.0.20 (%APPDATA%\VizRS\vizrs-data) zostaja w uzyciu,
rem dopoki w nowym miejscu nie ma show (to samo wybiera vizrs-shell.exe bez --data).
set "VIZRS_DIR=%LOCALAPPDATA%\VizRS"
if not exist "%VIZRS_DIR%\show.json" if exist "%APPDATA%\VizRS\vizrs-data\show.json" set "VIZRS_DIR=%APPDATA%\VizRS\vizrs-data"
echo Dane: %VIZRS_DIR%
"%~dp0vizrs.exe" --data "%VIZRS_DIR%" serve
if errorlevel 1 pause
