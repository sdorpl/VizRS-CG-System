@echo off
cd /d "%~dp0"
if not exist "bin\windows-x86_64\vizrs.exe" (echo Najpierw uruchom build-windows.bat & pause & exit /b 1)
start "" http://localhost:8088/trio
"bin\windows-x86_64\vizrs.exe" --data vizrs-data
