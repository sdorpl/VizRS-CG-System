# Instalator Windows (Inno Setup, z instalatorem WebView2) z binarek source/target/release i source/shell/target/release.
#   pwsh deploy/windows/build-installer.ps1 -Version 26.11.13   (z katalogu głównego repo; wynik w dist\)
# Wspólny dla CI (shell-smoke, wersja 0.0.0) i wydania (release.yml).
param([Parameter(Mandatory)][string]$Version)
$ErrorActionPreference = 'Stop'
# bez VCRUNTIME140.dll (statyczny CRT, source/.cargo/config.toml): czysty Windows 10 nie ma Visual C++ Redistributable
foreach ($f in 'source/target/release/vizrs.exe', 'source/target/release/vizrs-app.exe', 'source/shell/target/release/vizrs-shell.exe') {
  $t = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($f))
  if ($t -match '(?i)vcruntime\d+\.dll|msvcp\d+\.dll') { throw "$f wymaga $($Matches[0]) (Visual C++ Redistributable)" }
  "$f – bez bibliotek Visual C++"
}
# instalator Evergreen WebView2 Runtime (Microsoft) – instalator VizRS uruchamia go, gdy w systemie brak WebView2
Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile deploy/windows/MicrosoftEdgeWebview2Setup.exe
iscc "/DAppVersion=$Version" deploy/windows/installer.iss
if ($LASTEXITCODE -ne 0) { throw "iscc: kod $LASTEXITCODE" }
Get-ChildItem dist
