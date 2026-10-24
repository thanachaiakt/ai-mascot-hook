; Inno Setup has no Thai wizard text of its own, so the texts a user reads are written here.
; Anything not listed (rare error dialogs) falls back to English.
[LangOptions]
LanguageName=<0e44><0e17><0e22>
LanguageID=$041E
LanguageCodePage=874

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
SelectLanguageTitle=เลือกภาษา
SelectLanguageLabel=เลือกภาษาที่ใช้ในการติดตั้ง
ExitSetupTitle=ออกจากการติดตั้ง
ExitSetupMessage=การติดตั้งยังไม่เสร็จ ถ้าออกตอนนี้ โปรแกรมจะไม่ถูกติดตั้ง%n%nต้องการออกหรือไม่?
WelcomeLabel1=ยินดีต้อนรับสู่การติดตั้ง [name]
WelcomeLabel2=[name/ver] คือแมวน้ำตัวเล็กบนเดสก์ท็อป ที่แสดงให้เห็นว่า AI เขียนโค้ดของคุณ (Claude Code, Cursor, Copilot, Gemini, Codex ฯลฯ) กำลังทำอะไรอยู่: คิด ทำงาน รอคุณอนุญาต หรือเสร็จแล้ว%n%nแนะนำให้ปิดโปรแกรมอื่นก่อนดำเนินการต่อ
ReadyLabel1=พร้อมติดตั้งแล้ว
ReadyLabel2a=กด "ติดตั้ง" เพื่อเริ่มติดตั้ง หรือ "ย้อนกลับ" ถ้าต้องการแก้ไขตัวเลือก
ReadyLabel2b=กด "ติดตั้ง" เพื่อเริ่มติดตั้ง
InstallingLabel=กำลังติดตั้ง [name] กรุณารอสักครู่...
FinishedHeadingLabel=ติดตั้ง [name] เสร็จแล้ว
FinishedLabelNoIcons=ติดตั้ง [name] ในเครื่องของคุณเรียบร้อยแล้ว
FinishedLabel=ติดตั้ง [name] ในเครื่องของคุณเรียบร้อยแล้ว เปิดโปรแกรมได้จากทางลัดในเมนู Start%n%nถ้าเพิ่มฮุกให้ AI ตัวใดไว้ ให้เปิด AI ตัวนั้นใหม่ด้วย เพื่อให้โหลดฮุก
ConfirmUninstall=ต้องการถอน %1 และส่วนประกอบทั้งหมดออกจากเครื่องหรือไม่?%n%nฮุกที่เพิ่มไว้ให้ AI ทุกตัว และการตั้งค่าของ %1 จะถูกลบออกทั้งหมดด้วย
UninstallStatusLabel=กำลังถอน %1 ออกจากเครื่อง กรุณารอสักครู่...
UninstalledAll=ถอน %1 ออกจากเครื่องเรียบร้อยแล้ว
UninstalledMost=ถอน %1 เรียบร้อยแล้ว%n%nบางรายการลบไม่ได้ ลบเองได้ภายหลัง
UninstallOpenError=เปิดไฟล์ %1 ไม่ได้ จึงถอนการติดตั้งต่อไม่ได้

[CustomMessages]
TaskStartup=เปิดแมวน้ำให้อัตโนมัติเมื่อเข้าสู่ระบบ Windows
TaskStartupGroup=ตัวเลือกเพิ่มเติม:
RunHooks=กำลังเพิ่มฮุกให้ AI ที่เลือก...
RunStart=เปิดแมวน้ำเดี๋ยวนี้
UpdateTitle=อัปเดต ai-mascot-hook
UpdateWelcome=พบ ai-mascot-hook รุ่น %1 ในเครื่องแล้ว ตัวติดตั้งนี้จะอัปเดตเป็นรุ่น %2%n%nการตั้งค่า ตำแหน่งแมวน้ำ และฮุกที่มีอยู่จะอยู่ครบ แมวน้ำที่เปิดอยู่จะถูกปิดแล้วเปิดใหม่ให้เอง%n%nกด "ถัดไป" เพื่ออัปเดต
SameVersion=ติดตั้ง ai-mascot-hook รุ่น %1 อยู่แล้ว ตัวติดตั้งนี้จะติดตั้งทับ (ซ่อมแซม) ให้%n%nการตั้งค่าของคุณจะอยู่ครบ
OlderVersion=ในเครื่องมี ai-mascot-hook รุ่น %1 ซึ่งใหม่กว่ารุ่นของตัวติดตั้งนี้ (%2)%n%nต้องการติดตั้งรุ่นเก่ากว่าทับหรือไม่?
HooksTitle=เชื่อมต่อกับ AI ของคุณ
HooksSubtitle=เลือก AI ที่จะให้แมวน้ำขยับตาม
HooksBody=AI เหล่านี้รันคำสั่งเล็ก ๆ ที่เรียกว่า "ฮุก" เมื่อมีเหตุการณ์เกิดขึ้น เช่น คุณส่งข้อความ เครื่องมือเริ่มทำงาน หรือจบหนึ่งรอบ ติ๊กเลือก AI ที่คุณใช้ ตัวติดตั้งจะเพิ่มฮุกลงในไฟล์ตั้งค่าของ AI ตัวนั้น (ใต้ %1):%n    Claude Code: .claude\settings.json%n    Cursor: .cursor\hooks.json%n    GitHub Copilot CLI: .copilot\hooks\ai-mascot-hook.json%n    Gemini CLI: .gemini\settings.json%n    OpenAI Codex: .codex\hooks.json%n    Qwen Code: .qwen\settings.json%n    Windsurf: .codeium\windsurf\hooks.json%n    Kiro: .kiro\hooks\ai-mascot-hook.json%n%nแต่ละฮุกรันคำสั่ง "ai-mascot-hook.exe --set <สถานะ>" ซึ่งเขียนคำเดียวลงไฟล์ในโฟลเดอร์ %%LOCALAPPDATA%%\ai-mascot-hook แล้วจบทันที ไม่มีการส่งข้อมูลผ่านอินเทอร์เน็ตเลย%n%n- ไฟล์เดิมจะถูกสำรองไว้ก่อน เป็น <ชื่อไฟล์>.bak-<เวลา>%n- ฮุกและการตั้งค่าอื่นของคุณอยู่ครบ ฮุก ai-mascot-hook รุ่นเก่าจะถูกแทนที่%n- ตอนถอนการติดตั้ง ฮุกเหล่านี้จะถูกเอาออกให้ทั้งหมด%n- เปิด AI ตัวนั้นใหม่หลังติดตั้ง เพื่อให้โหลดฮุก (Codex ให้ตรวจฮุกใหม่ด้วย /hooks หนึ่งครั้ง)%n%nถ้าไม่เลือกเลย แมวน้ำยังทำงานได้ แต่จะไม่ขยับตาม AI ถ้าอยากเพิ่มฮุกทีหลัง ให้รันตัวติดตั้งนี้อีกครั้ง
