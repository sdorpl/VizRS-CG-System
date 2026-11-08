@echo off
rem Uruchamia VizRS Trio w trybie polaczenia ze zdalnym serwerem (VPS / studio).
cd /d "%~dp0"
if exist "vizrs-shell.exe" (
    start "" "vizrs-shell.exe" connect
) else (
    echo Okno polaczenia wymaga vizrs-shell.exe.
    pause
)
