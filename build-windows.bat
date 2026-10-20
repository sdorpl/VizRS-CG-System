@echo off
rem Buduje VizRS dla Windows. Wymaga Rust: https://rustup.rs (jedna instalacja, bez dodatkowych bibliotek).
cd /d "%~dp0source"
cargo build --release || (echo Blad kompilacji & pause & exit /b 1)
if not exist "..\bin\windows-x86_64" mkdir "..\bin\windows-x86_64"
copy /y target\release\vizrs.exe "..\bin\windows-x86_64\vizrs.exe"
copy /y target\release\vizrs-app.exe "..\bin\windows-x86_64\vizrs-app.exe"
rem okno natywne (opcjonalne, WebView2 jest w Windows 10/11): blad budowy nie przerywa
pushd shell
cargo build --release && copy /y target\release\vizrs-shell.exe "..\..\bin\windows-x86_64\vizrs-shell.exe"
if errorlevel 1 echo Okno natywne vizrs-shell nie zbudowane - dzialaja okna Edge (vizrs-app.exe)
popd
echo Gotowe: bin\windows-x86_64\vizrs.exe  -- uruchom start-windows.bat
echo Skroty w menu Start: powershell -ExecutionPolicy Bypass -File deploy\install-windows-shortcuts.ps1
pause
