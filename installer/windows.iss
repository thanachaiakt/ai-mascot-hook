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

; Inno Setup ships no Thai wizard text, so the texts a user reads are overridden here. Anything
; not listed (rare error dialogs) stays English.
[Messages]
SetupAppTitle=ติดตั้ง
SetupWindowTitle=ติดตั้ง - %1
UninstallAppTitle=ถอนการติดตั้ง
UninstallAppFullTitle=ถอนการติดตั้ง %1
ButtonBack=< &ย้อนกลับ
ButtonNext=&ถัดไป >
ButtonInstall=&ติดตั้ง
ButtonOK=ตกลง
ButtonCancel=ยกเลิก
ButtonYes=&ใช่
ButtonNo=&ไม่
ButtonFinish=&เสร็จสิ้น
ButtonBrowse=&เลือก...
ExitSetupTitle=ออกจากการติดตั้ง
ExitSetupMessage=การติดตั้งยังไม่เสร็จ ถ้าออกตอนนี้ โปรแกรมจะไม่ถูกติดตั้ง%n%nต้องการออกหรือไม่?
WelcomeLabel1=ยินดีต้อนรับสู่การติดตั้ง [name]
WelcomeLabel2=[name/ver] คือแมวน้ำตัวเล็กบนเดสก์ท็อป ที่แสดงให้เห็นว่า Claude Code กำลังทำอะไรอยู่: คิด ทำงาน รอคุณอนุญาต หรือเสร็จแล้ว%n%nแนะนำให้ปิดโปรแกรมอื่นก่อนดำเนินการต่อ
ReadyLabel1=พร้อมติดตั้งแล้ว
ReadyLabel2a=กด "ติดตั้ง" เพื่อเริ่มติดตั้ง หรือ "ย้อนกลับ" ถ้าต้องการแก้ไขตัวเลือก
ReadyLabel2b=กด "ติดตั้ง" เพื่อเริ่มติดตั้ง
InstallingLabel=กำลังติดตั้ง [name] กรุณารอสักครู่...
FinishedHeadingLabel=ติดตั้ง [name] เสร็จแล้ว
FinishedLabelNoIcons=ติดตั้ง [name] ในเครื่องของคุณเรียบร้อยแล้ว
FinishedLabel=ติดตั้ง [name] ในเครื่องของคุณเรียบร้อยแล้ว เปิดโปรแกรมได้จากทางลัดในเมนู Start%n%nถ้าเพิ่มฮุกให้ Claude Code ไว้ ให้เปิด Claude Code ใหม่ด้วย เพื่อให้โหลดฮุกนั้น
ConfirmUninstall=ต้องการถอน %1 และส่วนประกอบทั้งหมดออกจากเครื่องหรือไม่?
UninstallStatusLabel=กำลังถอน %1 ออกจากเครื่อง กรุณารอสักครู่...
UninstalledAll=ถอน %1 ออกจากเครื่องเรียบร้อยแล้ว
UninstalledMost=ถอน %1 เรียบร้อยแล้ว%n%nบางรายการลบไม่ได้ ลบเองได้ภายหลัง
UninstallOpenError=เปิดไฟล์ %1 ไม่ได้ จึงถอนการติดตั้งต่อไม่ได้

[CustomMessages]
TaskStartup=เปิดแมวน้ำให้อัตโนมัติเมื่อเข้าสู่ระบบ Windows
TaskStartupGroup=ตัวเลือกเพิ่มเติม:
RunHooks=กำลังเพิ่มฮุกให้ Claude Code...
RunStart=เปิดแมวน้ำเดี๋ยวนี้
UpdateTitle=อัปเดต ai-mascot-hook
UpdateWelcome=พบ ai-mascot-hook รุ่น %1 ในเครื่องแล้ว ตัวติดตั้งนี้จะอัปเดตเป็นรุ่น %2%n%nการตั้งค่า ตำแหน่งแมวน้ำ และฮุกของ Claude Code จะอยู่ครบ แมวน้ำที่เปิดอยู่จะถูกปิดแล้วเปิดใหม่ให้เอง%n%nกด "ถัดไป" เพื่ออัปเดต
SameVersion=ติดตั้ง ai-mascot-hook รุ่น %1 อยู่แล้ว ตัวติดตั้งนี้จะติดตั้งทับ (ซ่อมแซม) ให้%n%nการตั้งค่าของคุณจะอยู่ครบ
OlderVersion=ในเครื่องมี ai-mascot-hook รุ่น %1 ซึ่งใหม่กว่ารุ่นของตัวติดตั้งนี้ (%2)%n%nต้องการติดตั้งรุ่นเก่ากว่าทับหรือไม่?
HooksTitle=เชื่อมต่อกับ Claude Code
HooksSubtitle=ai-mascot-hook ขออนุญาตแก้ไขไฟล์ตั้งค่าของ Claude Code
HooksAllow=อนุญาตให้เพิ่มฮุกลงใน settings.json
PurgeAsk=ต้องการลบการตั้งค่าของ ai-mascot-hook ออกด้วยหรือไม่?%n%nจะลบ: ตำแหน่งที่วางแมวน้ำ ขนาด ข้อความและสีของฟองสถานะ (status.txt) และไฟล์สถานะ ในโฟลเดอร์%n%1%n%nถ้าตอบ "ไม่" การตั้งค่าจะอยู่ต่อ และกลับมาใช้ได้เมื่อติดตั้งใหม่
HooksBody=Claude Code จะรันคำสั่งเล็ก ๆ ที่เรียกว่า "ฮุก" เมื่อมีเหตุการณ์เกิดขึ้น เช่น คุณส่งข้อความ เครื่องมือเริ่มทำงาน Claude ขออนุญาต หรือจบหนึ่งรอบ เพื่อให้แมวน้ำขยับตาม Claude ตัวติดตั้งจะเพิ่มฮุกหนึ่งตัวต่อหนึ่งเหตุการณ์ ลงในไฟล์:%n    %1\.claude\settings.json%n%nแต่ละฮุกรันคำสั่ง "ai-mascot-hook.exe --set <สถานะ>" ซึ่งเขียนคำเดียวลงไฟล์ในโฟลเดอร์ %%LOCALAPPDATA%%i-mascot-hook แล้วจบทันที ไม่มีการส่งข้อมูลผ่านอินเทอร์เน็ตเลย%n%n- ไฟล์เดิมจะถูกสำรองไว้ก่อน เป็น settings.json.bak-<เวลา>%n- ฮุกและการตั้งค่าอื่นของคุณอยู่ครบ ฮุก ai-mascot-hook รุ่นเก่าจะถูกแทนที่%n- ตอนถอนการติดตั้ง ฮุกเหล่านี้จะถูกเอาออกให้%n- เปิด Claude Code ใหม่หลังติดตั้ง เพื่อให้โหลดฮุก%n%nถ้าไม่อนุญาต แมวน้ำยังทำงานได้ แต่จะไม่ขยับตาม Claude ถ้าอยากเพิ่มฮุกทีหลัง ให้รันตัวติดตั้งนี้อีกครั้ง

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
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\install-hooks.ps1"" -Exe ""{app}\ai-mascot-hook.exe"""; StatusMsg: "{cm:RunHooks}"; Flags: runhidden waituntilterminated; Check: HooksAllowed
; after an update the seal comes back by itself, even in a silent run, since the update closed it
Filename: "{app}\ai-mascot-hook.exe"; Flags: nowait runasoriginaluser; Check: IsUpgrade
Filename: "{app}\ai-mascot-hook.exe"; Description: "{cm:RunStart}"; Flags: nowait postinstall skipifsilent; Check: not IsUpgrade

[UninstallRun]
Filename: "taskkill.exe"; Parameters: "/F /IM ai-mascot-hook.exe"; Flags: runhidden waituntilterminated; RunOnceId: "StopSeal"
Filename: "{app}\ai-mascot-hook.exe"; Parameters: "--remove-hooks"; Flags: runhidden waituntilterminated; RunOnceId: "RemoveHooks"

[Code]
const
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{6B0C7E2A-3F4D-4C1B-9A8E-5D2F1E7C9A41}_is1';

var
  HooksPage: TInputOptionWizardPage;
  PurgeSettings: Boolean;
  OldVersion: String;
  // -1 an older build is installed (an update), 0 the same, 1 a newer one, 2 nothing installed
  Compared: Integer;

function HooksInstalled: Boolean;
var
  Text: AnsiString;
begin
  Result := LoadStringFromFile(ExpandConstant('{%USERPROFILE}\.claude\settings.json'), Text)
    and (Pos('ai-mascot-hook', String(Text)) > 0);
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

// Editing another program's settings needs the user's say-so, so this page spells out
// exactly what changes in settings.json before asking
procedure InitializeWizard;
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
  HooksPage.Add(CustomMessage('HooksAllow'));
  HooksPage.Values[0] := True;
end;

// On an update whose hooks are already in place the question was answered once: refresh them
// without asking again. Anyone who declined (or has none) is asked as on a first install.
function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := (PageID = HooksPage.ID) and IsUpgrade and HooksInstalled;
end;

function HooksAllowed: Boolean;
begin
  Result := (IsUpgrade and HooksInstalled) or HooksPage.Values[0];
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

// The seal's own settings (placement, status.txt, state) live outside the install folder, so
// uninstalling only clears them if the user says so. Silent uninstalls keep them.
function InitializeUninstall: Boolean;
begin
  Result := True;
  PurgeSettings := (not UninstallSilent) and
    (MsgBox(FmtMessage(CustomMessage('PurgeAsk'), [ExpandConstant('{localappdata}i-mascot-hook')]),
      mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES);
end;

// Hosts that redirect %LOCALAPPDATA% writes (the Claude desktop app, from the Store) keep
// their own copy of the folder under Packages\<app>\LocalCache\Local
procedure PurgeStoreCopies;
var
  Packages: String;
  Found: TFindRec;
begin
  Packages := ExpandConstant('{localappdata}\Packages');
  if FindFirst(Packages + '*', Found) then
  try
    repeat
      DelTree(Packages + Found.Name + '\LocalCache\Locali-mascot-hook', True, True, True);
    until not FindNext(Found);
  finally
    FindClose(Found);
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if (CurUninstallStep = usPostUninstall) and PurgeSettings then
  begin
    DelTree(ExpandConstant('{localappdata}i-mascot-hook'), True, True, True);
    PurgeStoreCopies;
  end;
end;
