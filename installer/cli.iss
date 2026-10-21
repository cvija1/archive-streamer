[Setup]
AppId=ArchiveStreamerCLI
AppName=Archive Streamer CLI
AppVersion=1.0.0
DefaultDirName={localappdata}\Programs\Archive Streamer CLI
DefaultGroupName=Archive Streamer CLI
OutputDir=output
OutputBaseFilename=Archive-Streamer-CLI-Setup
PrivilegesRequired=lowest
ChangesEnvironment=yes
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Tasks]
Name: "addtopath"; Description: "Add Archive Streamer CLI to PATH"; Flags: checkedonce

[Files]
Source: "..\target\x86_64-pc-windows-gnu\release\archive-streamer-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\x86_64-pc-windows-gnu\release\*.dll"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Uninstall Archive Streamer CLI"; Filename: "{uninstallexe}"

[Code]
const
  EnvironmentKey = 'Environment';
  PathName = 'Path';

function NormalizePathEntry(Value: string): string;
begin
  Result := RemoveBackslashUnlessRoot(Trim(Value));
  Result := Lowercase(Result);
end;

procedure AddToUserPath(Directory: string);
var
  ExistingPath, NewPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, PathName, ExistingPath) then
    ExistingPath := '';

  if ExistingPath = '' then
    NewPath := Directory
  else if Pos(';' + NormalizePathEntry(Directory) + ';',
              ';' + NormalizePathEntry(ExistingPath) + ';') > 0 then
    Exit
  else
    NewPath := ExistingPath + ';' + Directory;

  if RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, PathName, NewPath) then
    RegWriteStringValue(HKEY_CURRENT_USER, 'Software\ArchiveStreamerCLI', 'PathEntry', Directory);
end;

procedure RemoveFromUserPath;
var
  ExistingPath, Directory, Remaining, Entry: string;
  StartPos, SeparatorPos: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Software\ArchiveStreamerCLI',
                             'PathEntry', Directory) then
    Directory := ExpandConstant('{app}');

  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, PathName, ExistingPath) then
    Exit;

  Remaining := '';
  StartPos := 1;

  while StartPos <= Length(ExistingPath) do
  begin
    SeparatorPos := Pos(';', Copy(ExistingPath, StartPos, MaxInt));
    if SeparatorPos = 0 then
    begin
      Entry := Trim(Copy(ExistingPath, StartPos, MaxInt));
      StartPos := Length(ExistingPath) + 1;
    end
    else
    begin
      Entry := Trim(Copy(ExistingPath, StartPos, SeparatorPos - 1));
      StartPos := StartPos + SeparatorPos;
    end;

    if (Entry <> '') and
       (NormalizePathEntry(Entry) <> NormalizePathEntry(Directory)) then
    begin
      if Remaining <> '' then
        Remaining := Remaining + ';';
      Remaining := Remaining + Entry;
    end;
  end;

  if Remaining = '' then
    RegDeleteValue(HKEY_CURRENT_USER, EnvironmentKey, PathName)
  else
    RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, PathName, Remaining);

  RegDeleteKeyIfEmpty(HKEY_CURRENT_USER, 'Software\ArchiveStreamerCLI');
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    AddToUserPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    RemoveFromUserPath;
end;