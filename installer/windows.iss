; Inno Setup script for the Windows installer. The release workflow builds it with
;   iscc /DAppVersion=<version> installer\windows.iss
; after `cargo build --release`, and writes target\installer\ai-mascot-hook-<version>-setup.exe.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6B0C7E2A-3F4D-4C1B-9A8E-5D2F1E7C9A41}
AppName=ai-mascot-hook
AppVersion={#AppVersion}
AppPublisher=Thanachai Aktananan
AppPublisherURL=https://github.com/thanachaiakt/ai-mascot-hook
; Per user, so no admin prompt: the hooks and settings it touches are per user anyway
PrivilegesRequired=lowest
DefaultDirName={localappdata}\Programs\ai-mascot-hook
DisableDirPage=yes
DisableProgramGroupPage=yes
DefaultGroupName=ai-mascot-hook
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\target\installer
OutputBaseFilename=ai-mascot-hook-{#AppVersion}-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; An upgrade runs over the same folder: the seal is stopped by PrepareToInstall, not by Restart Manager
CloseApplications=no
VersionInfoVersion={#AppVersion}
UninstallDisplayIcon={app}\ai-mascot-hook.exe
; The first screen asks English or Thai, and the uninstaller speaks the language chosen then
ShowLanguageDialog=yes

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "th"; MessagesFile: "Thai.isl"

; English wording for the texts this program changes; the Thai ones are in Thai.isl
[Messages]
en.WelcomeLabel2=[name/ver] is a small seal on your desktop that shows what your AI coding assistant (Claude Code, Cursor, Copilot, Gemini, Codex and others) is doing: thinking, working, waiting for your OK, or done.%n%nIt is recommended that you close all other applications before continuing.
en.FinishedLabel=Setup has finished installing [name] on your computer. Start it from the shortcut in the Start menu.%n%nIf you added hooks for an AI assistant, restart it so it loads them.
en.ConfirmUninstall=Are you sure you want to completely remove %1 and all of its components?%n%nThe hooks it added to your AI assistants and all of its settings are removed too.

[CustomMessages]
en.TaskStartup=Start the seal automatically when I sign in to Windows
en.TaskStartupGroup=Additional options:
en.RunHooks=Adding hooks for the selected AI assistants...
en.RunStart=Start the seal now
en.UpdateTitle=Update ai-mascot-hook
en.UpdateWelcome=Version %1 of ai-mascot-hook is installed. This setup updates it to version %2.%n%nYour settings, the seal's position and the existing hooks are kept. A running seal is closed and started again for you.%n%nClick Next to update.
en.SameVersion=Version %1 of ai-mascot-hook is already installed. This setup installs over it (a repair).%n%nYour settings are kept.
en.OlderVersion=Version %1 of ai-mascot-hook is installed, which is newer than this setup (%2).%n%nInstall the older version over it?
en.HooksTitle=Connect to your AI assistants
en.HooksSubtitle=Choose the ones the seal should follow
en.HooksBody=These assistants run small commands called "hooks" when something happens, such as you sending a message, a tool starting, or a turn ending. Tick the ones you use and setup adds hooks to that assistant's settings file (under %1):%n    Claude Code: .claude\settings.json%n    Cursor: .cursor\hooks.json%n    GitHub Copilot CLI: .copilot\hooks\ai-mascot-hook.json%n    Gemini CLI: .gemini\settings.json%n    OpenAI Codex: .codex\hooks.json%n    Qwen Code: .qwen\settings.json%n    Windsurf: .codeium\windsurf\hooks.json%n    Kiro: .kiro\hooks\ai-mascot-hook.json%n%nEach hook runs "ai-mascot-hook.exe --set <state>", which writes one word to a file in %%LOCALAPPDATA%%\ai-mascot-hook and exits. Nothing is sent over the internet.%n%n- An existing file is backed up first, as <name>.bak-<time>%n- Your other hooks and settings are kept; older ai-mascot-hook hooks are replaced%n- Uninstalling removes these hooks from every assistant%n- Restart the assistant after setup so it loads the hooks (Codex asks you to review new hooks once with /hooks)%n%nIf you tick none, the seal still runs but does not follow any assistant. To add hooks later, run this setup again.

[Tasks]
Name: "startup"; Description: "{cm:TaskStartup}"; GroupDescription: "{cm:TaskStartupGroup}"

[Files]
Source: "..\target\release\ai-mascot-hook.exe"; DestDir: "{app}"; Flags: ignoreversion
; The script turns a path with spaces (a user name like "Jane Doe") into its 8.3 short form,
; which --install-hooks refuses to guess
Source: "..\tools\install-hooks.ps1"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\ai-mascot-hook"; Filename: "{app}\ai-mascot-hook.exe"
Name: "{userstartup}\ai-mascot-hook"; Filename: "{app}\ai-mascot-hook.exe"; Tasks: startup

[Run]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\install-hooks.ps1"" -Exe ""{app}\ai-mascot-hook.exe"" -Agent ""{code:HookAgents}"""; StatusMsg: "{cm:RunHooks}"; Flags: runhidden waituntilterminated; Check: HooksWanted
; after an update the seal comes back by itself, even in a silent run, since the update closed it
Filename: "{app}\ai-mascot-hook.exe"; Flags: nowait runasoriginaluser; Check: IsUpgrade
Filename: "{app}\ai-mascot-hook.exe"; Description: "{cm:RunStart}"; Flags: nowait postinstall skipifsilent; Check: not IsUpgrade

[UninstallRun]
Filename: "taskkill.exe"; Parameters: "/F /IM ai-mascot-hook.exe"; Flags: runhidden waituntilterminated; RunOnceId: "StopSeal"
; every assistant that has our hooks, not only the ones chosen at install time
Filename: "{app}\ai-mascot-hook.exe"; Parameters: "--remove-hooks --agent all"; Flags: runhidden waituntilterminated; RunOnceId: "RemoveHooks"

[Code]
const
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{6B0C7E2A-3F4D-4C1B-9A8E-5D2F1E7C9A41}_is1';
  AgentCount = 8;

var
  HooksPage: TInputOptionWizardPage;
  OldVersion: String;
  // -1 an older build is installed (an update), 0 the same, 1 a newer one, 2 nothing installed
  Compared: Integer;

// The assistants, in the order of the page's boxes. The id is what --agent takes.
function AgentId(I: Integer): String;
begin
  case I of
    0: Result := 'claude';
    1: Result := 'cursor';
    2: Result := 'copilot';
    3: Result := 'gemini';
    4: Result := 'codex';
    5: Result := 'qwen';
    6: Result := 'windsurf';
  else
    Result := 'kiro';
  end;
end;

function AgentTitle(I: Integer): String;
begin
  case I of
    0: Result := 'Claude Code';
    1: Result := 'Cursor';
    2: Result := 'GitHub Copilot CLI';
    3: Result := 'Gemini CLI';
    4: Result := 'OpenAI Codex';
    5: Result := 'Qwen Code';
    6: Result := 'Windsurf';
  else
    Result := 'Kiro';
  end;
end;

// The assistant's own folder in the user's home
function AgentDir(I: Integer): String;
var
  Home: String;
begin
  Home := ExpandConstant('{%USERPROFILE}');
  case I of
    0: Result := Home + '\.claude';
    1: Result := Home + '\.cursor';
    2:
      begin
        Result := GetEnv('COPILOT_HOME');
        if Result = '' then Result := Home + '\.copilot';
      end;
    3: Result := Home + '\.gemini';
    4: Result := Home + '\.codex';
    5: Result := Home + '\.qwen';
    6: Result := Home + '\.codeium\windsurf';
  else
    Result := Home + '\.kiro';
  end;
end;

// The file that holds our hooks for this assistant
function AgentFile(I: Integer): String;
begin
  case I of
    0, 3, 5: Result := AgentDir(I) + '\settings.json';
    1, 4, 6: Result := AgentDir(I) + '\hooks.json';
  else
    Result := AgentDir(I) + '\hooks\ai-mascot-hook.json';
  end;
end;

function AgentHooked(I: Integer): Boolean;
var
  Text: AnsiString;
begin
  Result := LoadStringFromFile(AgentFile(I), Text) and (Pos('ai-mascot-hook', String(Text)) > 0);
end;

function IsUpgrade: Boolean;
begin
  Result := Compared < 2;
end;

procedure Compare;
var
  Old, New: Int64;
begin
  Compared := 2;
  if not RegQueryStringValue(HKCU, UninstallKey, 'DisplayVersion', OldVersion) then Exit;
  if StrToVersion(OldVersion, Old) and StrToVersion('{#AppVersion}', New) then
    Compared := -ComparePackedVersion(New, Old)
  else
    Compared := 0;
end;

// Editing other programs' settings needs the user's say-so, so this page spells out exactly
// what changes before asking. The boxes start ticked for the assistants already hooked (an
// update keeps what was agreed to), or on a first install for Claude Code and every assistant
// found on this computer.
procedure InitializeWizard;
var
  I: Integer;
  Hooked: array[0..AgentCount - 1] of Boolean;
  AnyHooked: Boolean;
begin
  Compare;
  if Compared = -1 then
  begin
    WizardForm.WelcomeLabel1.Caption := CustomMessage('UpdateTitle');
    WizardForm.WelcomeLabel2.Caption := FmtMessage(CustomMessage('UpdateWelcome'), [OldVersion, '{#AppVersion}']);
  end
  else if Compared = 0 then
    WizardForm.WelcomeLabel2.Caption := FmtMessage(CustomMessage('SameVersion'), [OldVersion]);
  HooksPage := CreateInputOptionPage(wpWelcome,
    CustomMessage('HooksTitle'), CustomMessage('HooksSubtitle'),
    FmtMessage(CustomMessage('HooksBody'), [ExpandConstant('{%USERPROFILE}')]),
    False, False);
  AnyHooked := False;
  for I := 0 to AgentCount - 1 do
  begin
    Hooked[I] := AgentHooked(I);
    if Hooked[I] then AnyHooked := True;
  end;
  for I := 0 to AgentCount - 1 do
  begin
    HooksPage.Add(AgentTitle(I));
    if AnyHooked then
      HooksPage.Values[I] := Hooked[I]
    else
      HooksPage.Values[I] := (I = 0) or DirExists(AgentDir(I));
  end;
end;

// The ticked assistants, as --agent takes them
function HookAgents(Param: String): String;
var
  I: Integer;
begin
  Result := '';
  for I := 0 to AgentCount - 1 do
    if HooksPage.Values[I] then
    begin
      if Result <> '' then Result := Result + ',';
      Result := Result + AgentId(I);
    end;
end;

function HooksWanted: Boolean;
begin
  Result := HookAgents('') <> '';
end;

// Installing an older build over a newer one is allowed, but only on purpose
function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if (CurPageID = wpWelcome) and (Compared = 1) then
    Result := MsgBox(FmtMessage(CustomMessage('OlderVersion'), [OldVersion, '{#AppVersion}']),
      mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES;
end;

// A running seal locks its exe, so an upgrade over it would fail to replace the file
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
begin
  Exec('taskkill.exe', '/F /IM ai-mascot-hook.exe', '', SW_HIDE, ewWaitUntilTerminated, Code);
  Result := '';
end;

// Hosts that redirect %LOCALAPPDATA% writes (the Claude desktop app, from the Store) keep
// their own copy of the folder under Packages\<app>\LocalCache\Local
procedure PurgeStoreCopies;
var
  Packages: String;
  Found: TFindRec;
begin
  Packages := ExpandConstant('{localappdata}\Packages\');
  if FindFirst(Packages + '*', Found) then
  try
    repeat
      DelTree(Packages + Found.Name + '\LocalCache\Local\ai-mascot-hook', True, True, True);
    until not FindNext(Found);
  finally
    FindClose(Found);
  end;
end;

// Uninstalling leaves nothing behind: the hooks come out in [UninstallRun], and the seal's own
// settings (placement, status.txt, state), which live outside the install folder, go here.
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    DelTree(ExpandConstant('{localappdata}\ai-mascot-hook'), True, True, True);
    PurgeStoreCopies;
  end;
end;
