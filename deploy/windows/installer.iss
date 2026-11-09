; Inno Setup Script dla VizRS Trio (Windows x86_64). Plik w UTF-8 z BOM (polskie znaki w komunikatach).
; Budowa: iscc /DAppVersion=2.1.0 deploy\windows\installer.iss (wynik w dist\).
; WebView2: gdy obok skryptu leży MicrosoftEdgeWebview2Setup.exe (instalator Evergreen Microsoftu, pobiera go akcja Release),
; instalator dołącza go i uruchamia, jeśli w systemie nie ma WebView2 Runtime (okrojone Windows 10, Windows Server).
#ifndef AppVersion
  #define AppVersion "2.0.2"
#endif
#define WebView2Setup AddBackslash(SourcePath) + "MicrosoftEdgeWebview2Setup.exe"
#if FileExists(WebView2Setup)
  #define HaveWebView2Setup
#endif

[Setup]
AppId={{5E6A3B91-7C84-4E4D-9B1D-665A5A4A76E8}
AppName=VizRS Trio
AppVersion={#AppVersion}
AppPublisher=VizRS
AppPublisherURL=https://github.com/sdorpl/VizRS-CG-System
AppSupportURL=https://github.com/sdorpl/VizRS-CG-System
AppUpdatesURL=https://github.com/sdorpl/VizRS-CG-System/releases
DefaultDirName={autopf}\VizRS
DefaultGroupName=VizRS
AllowNoIcons=yes
OutputDir=..\..\dist
OutputBaseFilename=VizRS-Trio-{#AppVersion}-windows-x86_64-setup
SetupIconFile=vizrs.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "polish"; MessagesFile: "compiler:Languages\Polish.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; Binarki i skrypty
Source: "..\..\source\target\release\vizrs.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\source\target\release\vizrs-app.exe"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\..\source\shell\target\release\vizrs-shell.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "vizrs.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "README-Windows.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "start-trio.bat"; DestDir: "{app}"; Flags: ignoreversion
Source: "start-trio-zdalnie.bat"; DestDir: "{app}"; Flags: ignoreversion
Source: "start-serwer.bat"; DestDir: "{app}"; Flags: ignoreversion
Source: "serwer.bat"; DestDir: "{app}"; Flags: ignoreversion
#ifdef HaveWebView2Setup
Source: "MicrosoftEdgeWebview2Setup.exe"; Flags: dontcopy
#endif
Source: "..\..\examples\*.cgpack"; DestDir: "{app}\examples"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

; Skróty bez --data: okno wybiera katalog danych w chwili uruchomienia, osobno dla każdego użytkownika
; (%LOCALAPPDATA%\VizRS; dane z instalatora 2.0.20 w %APPDATA%\VizRS\vizrs-data zostają w użyciu). Ścieżka wpisana
; przy instalacji wskazywałaby katalog konta, które instalowało (instalacja „dla wszystkich” z innego konta administratora).
[Icons]
Name: "{group}\VizRS Trio"; Filename: "{app}\vizrs-shell.exe"; Parameters: "trio"; WorkingDir: "{app}"; IconFilename: "{app}\vizrs.ico"; Comment: "Playout grafiki CG na żywo"
Name: "{group}\VizRS Trio (serwer zdalny)"; Filename: "{app}\vizrs-shell.exe"; Parameters: "connect"; WorkingDir: "{app}"; IconFilename: "{app}\vizrs.ico"; Comment: "Połączenie ze zdalnym serwerem VizRS"
Name: "{group}\VizRS Serwer"; Filename: "{app}\serwer.bat"; WorkingDir: "{app}"; IconFilename: "{app}\vizrs.ico"; Comment: "Serwer VizRS (okno konsoli)"
Name: "{group}\{cm:UninstallProgram,VizRS Trio}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\VizRS Trio"; Filename: "{app}\vizrs-shell.exe"; Parameters: "trio"; WorkingDir: "{app}"; IconFilename: "{app}\vizrs.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\vizrs-shell.exe"; Parameters: "trio"; WorkingDir: "{app}"; Description: "{cm:LaunchProgram,VizRS Trio}"; Flags: nowait postinstall skipifsilent

[Code]
const
  WebView2Key = 'Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';

function HasWebView2At(Root: Integer; Key: String): Boolean;
var
  V: String;
begin
  Result := RegQueryStringValue(Root, Key, 'pv', V) and (V <> '') and (V <> '0.0.0.0');
end;

// WebView2 Runtime: instalacja dla komputera (64-bit Windows: gałąź WOW6432Node) albo dla użytkownika
function WebView2Installed: Boolean;
begin
  Result := HasWebView2At(HKLM, 'SOFTWARE\WOW6432Node\' + WebView2Key)
    or HasWebView2At(HKLM, 'SOFTWARE\' + WebView2Key)
    or HasWebView2At(HKCU, 'Software\' + WebView2Key);
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Code: Integer;
begin
  if (CurStep <> ssPostInstall) or WebView2Installed then
    Exit;
#ifdef HaveWebView2Setup
  WizardForm.StatusLabel.Caption := 'Instalowanie Microsoft Edge WebView2 Runtime (silnik okna Trio)…';
  ExtractTemporaryFile('MicrosoftEdgeWebview2Setup.exe');
  // bez uprawnień administratora instalator Microsoftu instaluje WebView2 dla bieżącego użytkownika
  Exec(ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe'), '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, Code);
  if WebView2Installed then
    Exit;
#endif
  // wiersz nie może zaczynać się od # (preprocesor ISPP wziąłby go za dyrektywę) – znaki nowej linii na końcu wierszy
  SuppressibleMsgBox('Okno VizRS Trio potrzebuje Microsoft Edge WebView2 Runtime, którego nie ma w tym systemie' + #13#10 +
    'i nie udało się go zainstalować. Pobierz go ze strony Microsoftu (szukaj: „WebView2 Runtime”)' + #13#10 +
    'i zainstaluj. Do tego czasu działa skrót „VizRS Serwer” i Trio w przeglądarce: http://localhost:8088/trio',
    mbError, MB_OK, IDOK);
end;
