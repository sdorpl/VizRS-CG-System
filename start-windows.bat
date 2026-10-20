@echo off
rem VizRS dla Windows: okno Trio (Edge w trybie aplikacji) + serwer.
rem   start-windows.bat          okno Trio
rem   start-windows.bat artist   okno Artist (drugie okno dolacza do dzialajacego serwera)
rem   start-windows.bat serve    sam serwer - UI w dowolnej przegladarce: http://localhost:8088/trio
rem Skroty w menu Start bez okna konsoli: deploy\install-windows-shortcuts.ps1
cd /d "%~dp0"
if not exist "bin\windows-x86_64\vizrs.exe" (echo Najpierw uruchom build-windows.bat & pause & exit /b 1)
set MODE=%1
if "%MODE%"=="" set MODE=trio
"bin\windows-x86_64\vizrs.exe" --data vizrs-data %MODE%
if errorlevel 1 pause
