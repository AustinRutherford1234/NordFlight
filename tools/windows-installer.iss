#ifndef SourceDir
  #error SourceDir must point to the packaged Windows game.
#endif
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

[Setup]
AppId={{2DCAC2A0-2F3A-4B14-B8C4-B774A753DA10}
AppName=NordFlight
AppVersion={#AppVersion}
AppPublisher=NordFlight
DefaultDirName={localappdata}\Programs\NordFlight
DefaultGroupName=NordFlight
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
SetupIconFile={#SourceDir}\NordFlight.ico
UninstallDisplayIcon={app}\NordFlight.ico
OutputBaseFilename=NordFlight-Setup
Compression=lzma2/fast
SolidCompression=yes
WizardStyle=modern
CloseApplications=no
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a Desktop shortcut"; Flags: checkedonce

[Files]
Source: "{#SourceDir}\NordFlight.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\NordFlight.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\build-version.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#SourceDir}\docs\*"; DestDir: "{app}\docs"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\NordFlight"; Filename: "{app}\NordFlight.exe"; WorkingDir: "{app}"; IconFilename: "{app}\NordFlight.ico"
Name: "{userdesktop}\NordFlight"; Filename: "{app}\NordFlight.exe"; WorkingDir: "{app}"; IconFilename: "{app}\NordFlight.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\NordFlight.exe"; Description: "Launch NordFlight"; WorkingDir: "{app}"; Flags: nowait postinstall skipifsilent
