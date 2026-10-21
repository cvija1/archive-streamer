[Setup]
AppId=ArchiveStreamerGUI
AppName=Archive Streamer
AppVersion=1.0.0
DefaultDirName={autopf}\Archive Streamer
DefaultGroupName=Archive Streamer
OutputDir=output
OutputBaseFilename=Archive-Streamer-GUI-Setup
PrivilegesRequired=admin
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\archive-streamer-gui.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "..\target\x86_64-pc-windows-gnu\release\archive-streamer-gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\x86_64-pc-windows-gnu\release\*.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\icon.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Archive Streamer"; Filename: "{app}\archive-streamer-gui.exe"
Name: "{autodesktop}\Archive Streamer"; Filename: "{app}\archive-streamer-gui.exe"; IconFilename: "{app}\icon.ico"; Tasks: desktopicon




