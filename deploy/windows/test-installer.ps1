# Instalator z dist\ na prawdziwym Windows – tak, jak użytkownik: tryb „dla wszystkich” (C:\Program Files bez prawa
# zapisu), skrót VizRS Trio bez --data, okno z instalacji (source/tests/e2e/native-smoke.js --installed), deinstalacja.
# Wydanie 2.0.20 nie otwierało tam Trio.
#   pwsh deploy/windows/test-installer.ps1   (z katalogu głównego repo; Node.js w PATH)
# Wspólny dla CI (shell-smoke) i wydania (release.yml – instalator, który trafia do wydania).
$ErrorActionPreference = 'Stop'
$setup = Get-ChildItem dist/VizRS-Trio-*-setup.exe | Select-Object -First 1
if (-not $setup) { throw 'brak dist/VizRS-Trio-*-setup.exe' }
$p = Start-Process -Wait -PassThru $setup.FullName -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/ALLUSERS', '/LOG=install.log'
if ($p.ExitCode -ne 0) { Get-Content install.log -Tail 40; throw "instalator: kod $($p.ExitCode)" }
$app = "$env:ProgramFiles\VizRS"
foreach ($f in 'vizrs.exe', 'vizrs-shell.exe', 'serwer.bat') { if (-not (Test-Path "$app\$f")) { throw "brak $app\$f" } }
$lnk = (New-Object -ComObject WScript.Shell).CreateShortcut("$env:ProgramData\Microsoft\Windows\Start Menu\Programs\VizRS\VizRS Trio.lnk")
"skrót: $($lnk.TargetPath) $($lnk.Arguments)"
if ($lnk.Arguments -ne 'trio') { throw "skrót VizRS Trio ma argumenty: $($lnk.Arguments)" }
node source/tests/e2e/native-smoke.js "$app\vizrs.exe" "$app\vizrs-shell.exe" --installed
if ($LASTEXITCODE -ne 0) { throw "okno z instalacji: kod $LASTEXITCODE" }
$p = Start-Process -Wait -PassThru "$app\unins000.exe" -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART'
Start-Sleep 3
if ($p.ExitCode -ne 0 -or (Test-Path "$app\vizrs-shell.exe")) { throw 'deinstalacja nie usunęła programu' }
'instalator: zainstalowany, okno działa, odinstalowany'
