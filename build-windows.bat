@echo off
rem Buduje VizRS dla Windows. Wymaga Rust: https://rustup.rs (jedna instalacja, bez dodatkowych bibliotek).
cd /d "%~dp0source"
cargo build --release || (echo Blad kompilacji & pause & exit /b 1)
if not exist "..\bin\windows-x86_64" mkdir "..\bin\windows-x86_64"
copy /y target\release\vizrs.exe "..\bin\windows-x86_64\vizrs.exe"
echo Gotowe: bin\windows-x86_64\vizrs.exe  -- uruchom start-windows.bat
pause
