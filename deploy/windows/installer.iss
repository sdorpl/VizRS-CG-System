; Inno Setup Script dla VizRS Trio (Windows x86_64)
#ifndef AppVersion
  #define AppVersion "2.0.2"
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
Source: "..\..\source\target\release\vizrs-app.exe"; DestDir: "{app}"; Flags: ignoreversion; Tasks: ; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\..\source\shell\target\release\vizrs-shell.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "vizrs.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "README-Windows.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "start-trio.bat"; DestDir: "{app}"; Flags: ignoreversion
Source: "start-trio-zdalnie.bat"; DestDir: "{app}"; Flags: ignoreversion
Source: "start-serwer.bat"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\examples\*.cgpack"; DestDir: "{app}\examples"; Flags: ignoreversion recursesubdirs createallsubdirs optional

[Icons]
Name: "{group}\VizRS Trio"; Filename: "{app}\vizrs-shell.exe"; Parameters: "--vizrs ""{app}\vizrs.exe"" --data ""{userappdata}\VizRS\vizrs-data"" trio"; WorkingDir: "{app}"; IconFilename: "{app}\vizrs.ico"; Comment: "Playout grafiki CG na zywo"
Name: "{group}\VizRS Trio (serwer zdalny)"; Filename: "{app}\vizrs-shell.exe"; Parameters: "connect"; WorkingDir: "{app}"; IconFilename: "{app}\vizrs.ico"; Comment: "Polaczenie ze zdalnym serwerem VizRS"
Name: "{group}\VizRS Serwer"; Filename: "{app}\vizrs.exe"; Parameters: "--data ""{userappdata}\VizRS\vizrs-data"" serve"; WorkingDir: "{app}"; IconFilename: "{app}\vizrs.ico"; Comment: "Serwer VizRS (okno konsoli)"
Name: "{group}\{cm:UninstallProgram,VizRS Trio}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\VizRS Trio"; Filename: "{app}\vizrs-shell.exe"; Parameters: "--vizrs ""{app}\vizrs.exe"" --data ""{userappdata}\VizRS\vizrs-data"" trio"; WorkingDir: "{app}"; IconFilename: "{app}\vizrs.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\vizrs-shell.exe"; Parameters: "--vizrs ""{app}\vizrs.exe"" --data ""{userappdata}\VizRS\vizrs-data"" trio"; Description: "{cm:LaunchProgram,VizRS Trio}"; Flags: nowait postinstall skipifsilent
