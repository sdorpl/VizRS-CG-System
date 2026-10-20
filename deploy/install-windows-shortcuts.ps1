# Skróty VizRS w menu Start (Windows): „VizRS Artist” i „VizRS Trio” z serwerem na wskazanym katalogu danych.
# Gdy obok vizrs.exe jest vizrs-shell.exe – okno natywne (WebView2, menu, „Zapisz jako”), inaczej okno aplikacji
# Edge/Chrome bez okna konsoli (vizrs-app.exe). Przełącznik -Chromium wymusza to drugie.
#
# Użycie (PowerShell, w katalogu repozytorium, po build-windows.bat):
#   powershell -ExecutionPolicy Bypass -File deploy\install-windows-shortcuts.ps1 [-Bin katalog] [-Data katalog]
# Usunięcie: usuń folder „VizRS” z menu Start (Win+R → shell:programs).
param(
    [string]$Bin = (Join-Path $PSScriptRoot '..\bin\windows-x86_64'),
    [string]$Data = (Join-Path $PSScriptRoot '..\vizrs-data'),
    [switch]$Chromium
)
$ErrorActionPreference = 'Stop'
$Bin = (Resolve-Path $Bin).Path
$app = Join-Path $Bin 'vizrs-app.exe'
$shellExe = Join-Path $Bin 'vizrs-shell.exe'
$useShell = (-not $Chromium) -and (Test-Path $shellExe)
if (-not $useShell -and -not (Test-Path $app)) { Write-Error "Brak $app – uruchom najpierw build-windows.bat"; exit 1 }
New-Item -ItemType Directory -Force -Path $Data | Out-Null
$Data = (Resolve-Path $Data).Path

$dir = Join-Path ([Environment]::GetFolderPath('Programs')) 'VizRS'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$shell = New-Object -ComObject WScript.Shell
$items = @(
    @{ Name = 'VizRS Artist'; Mode = 'artist'; Desc = 'Projektowanie szablonów grafiki CG' },
    @{ Name = 'VizRS Trio'; Mode = 'trio'; Desc = 'Playout grafiki CG na żywo' }
)
foreach ($it in $items) {
    $lnk = $shell.CreateShortcut((Join-Path $dir ($it.Name + '.lnk')))
    if ($useShell) {
        $lnk.TargetPath = $shellExe
        $lnk.Arguments = '--vizrs "' + (Join-Path $Bin 'vizrs.exe') + '" --data "' + $Data + '" ' + $it.Mode
    } else {
        $lnk.TargetPath = $app
        $lnk.Arguments = '--data "' + $Data + '" ' + $it.Mode
    }
    $lnk.WorkingDirectory = $Bin
    $lnk.Description = $it.Desc
    $lnk.Save()
    Write-Host "Utworzono: $($it.Name)"
}
# serwer bez okna aplikacji (konsola z adresami) – np. gdy UI otwierasz w przeglądarce na innym komputerze
$lnk = $shell.CreateShortcut((Join-Path $dir 'VizRS Serwer.lnk'))
$lnk.TargetPath = Join-Path $Bin 'vizrs.exe'
$lnk.Arguments = '--data "' + $Data + '" serve'
$lnk.WorkingDirectory = $Bin
$lnk.Description = 'Serwer VizRS (konsola)'
$lnk.Save()
Write-Host "Skróty w menu Start → VizRS ($dir). Dane: $Data"
if ($useShell) { Write-Host "Okno natywne: $shellExe" }
