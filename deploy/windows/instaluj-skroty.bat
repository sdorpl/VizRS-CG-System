@echo off
rem Tworzy skroty do VizRS Trio i Serwera w Menu Start biezacego uzytkownika.
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -Command "$PSScriptRoot='%~dp0'; & '%~dp0install-windows-shortcuts.ps1' -Bin '%~dp0.' -Data '%~dp0vizrs-data'"
pause
