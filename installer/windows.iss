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
AppPublisherURL=https://github.com/thanachaiakt/ai-mascot-status
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
UninstallDisplayIcon={app}\ai-mascot-hook.exe

[Tasks]
Name: "startup"; Description: "Start the seal when I sign in to Windows"

[Files]
Source: "..\target\release\ai-mascot-hook.exe"; DestDir: "{app}"; Flags: ignoreversion
; The script turns a path with spaces (a user name like "Jane Doe") into its 8.3 short form,
; which --install-hooks refuses to guess
Source: "..\tools\install-hooks.ps1"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\ai-mascot-hook"; Filename: "{app}\ai-mascot-hook.exe"
Name: "{userstartup}\ai-mascot-hook"; Filename: "{app}\ai-mascot-hook.exe"; Tasks: startup

[Run]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\install-hooks.ps1"" -Exe ""{app}\ai-mascot-hook.exe"""; StatusMsg: "Installing the Claude Code hooks..."; Flags: runhidden waituntilterminated; Check: HooksAllowed
Filename: "{app}\ai-mascot-hook.exe"; Description: "Start the seal now"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill.exe"; Parameters: "/F /IM ai-mascot-hook.exe"; Flags: runhidden waituntilterminated; RunOnceId: "StopSeal"
Filename: "{app}\ai-mascot-hook.exe"; Parameters: "--remove-hooks"; Flags: runhidden waituntilterminated; RunOnceId: "RemoveHooks"

[Code]
var
  HooksPage: TInputOptionWizardPage;

// Editing another program's settings needs the user's say-so, so this page spells out
// exactly what changes in settings.json before asking
procedure InitializeWizard;
begin
  HooksPage := CreateInputOptionPage(wpWelcome,
    'Connect to Claude Code',
    'ai-mascot-hook needs your permission to edit Claude Code''s settings.',
    'Claude Code runs small commands, called hooks, when something happens: you send a ' +
    'prompt, a tool runs, Claude asks for permission, or a turn finishes. To make the seal ' +
    'follow Claude, setup adds one hook per event to:' + #13#10 +
    '    ' + ExpandConstant('{%USERPROFILE}') + '\.claude\settings.json' + #13#10#13#10 +
    'Each hook runs "ai-mascot-hook.exe --set <state>", which writes one word to a file in ' +
    '%LOCALAPPDATA%\ai-mascot-hook and exits. Nothing is sent over the network.' + #13#10#13#10 +
    '- The current file is backed up first as settings.json.bak-<time>.' + #13#10 +
    '- Your other hooks and settings are kept; older ai-mascot-hook hooks are replaced.' + #13#10 +
    '- Uninstalling removes these hooks again.' + #13#10 +
    '- Restart Claude Code afterwards so it loads them.' + #13#10#13#10 +
    'Without this the seal still runs, but it will not react to Claude. To add the hooks later, ' +
    'run this setup again.',
    False, False);
  HooksPage.Add('Allow setup to add the hooks to settings.json');
  HooksPage.Values[0] := True;
end;

function HooksAllowed: Boolean;
begin
  Result := HooksPage.Values[0];
end;

// A running seal locks its exe, so an upgrade over it would fail to replace the file
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
begin
  Exec('taskkill.exe', '/F /IM ai-mascot-hook.exe', '', SW_HIDE, ewWaitUntilTerminated, Code);
  Result := '';
end;
