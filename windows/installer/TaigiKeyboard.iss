; Taigi Keyboard for Windows — Inno Setup 6.5+ script (roadmap W8; `x64compatible`
; is 6.3 syntax, the official ChineseTraditional.isl 6.5).
;
; Compiled by windows/scripts/release-app.sh:
;   iscc /DAppVersion=<x.y.z> /DDist=<staging dir> /O<output dir> TaigiKeyboard.iss
;
; The staging dir holds what the script ships:
;   TaigiKeyboard.dll              (x64 text service, signed)
;   x86\TaigiKeyboard.dll          (optional — WOW64 hosts; roadmap: gated)
;   TaigiKeyboardSettings.exe      (signed)
;   Runtime\*                      (the Windows App Runtime the settings window
;                                   runs on — self-contained, roadmap W17;
;                                   installed BESIDE the exe)
;   Dictionaries\*                 (from ios/Resources/Dictionaries, W2)
;   Fonts\*                        (from ios/Resources/Fonts)
;   update-check-task.xml          (the scheduled task's definition)
;
; What it does that a plain file copy would not (rakukan `rakukan_installer.iss`,
; PIME `installer.nsi`): registers the DLL with the matching regsvr32 per
; architecture, stops the settings window and unregisters the old DLL before
; an upgrade, tells the user to switch input method + sign out when a host
; still holds the DLL, creates the Start-menu shortcut that carries the toast
; AUMID, registers the per-user update-check task, and reverses every step on
; uninstall — leaving %APPDATA%\TaigiKeyboard (the user's learning data) alone.

#ifndef AppVersion
  #error Pass /DAppVersion=<x.y.z> (windows/scripts/release-app.sh does)
#endif
#ifndef Dist
  #error Pass /DDist=<staging dir> (windows/scripts/release-app.sh does)
#endif

#define AppName "Taigi Keyboard"
#define AppPublisher "Taigi Keyboard"
#define AppURL "https://taigikeyboard.tw"
#define AppUserModelID "TaigiKeyboard.Settings"
#define TaskName "TaigiKeyboard Update Check"
#define SettingsExe "TaigiKeyboardSettings.exe"
#define ServiceDll "TaigiKeyboard.dll"

[Setup]
; Stable across versions: what makes a later installer an UPGRADE.
AppId={{6F0D8C2B-4A57-4E3F-9B1E-2D7C3A8E5F61}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}
; The updater pins these against the running settings exe's own VERSIONINFO
; (taigi-windows-update::verify): ProductName identical, ProductVersion = the
; workspace version.
VersionInfoVersion={#AppVersion}
VersionInfoProductVersion={#AppVersion}
VersionInfoProductName={#AppName}
VersionInfoDescription={#AppName} Setup
VersionInfoCompany={#AppPublisher}
; Machine-wide, under Program Files: a text service is loaded into every
; process of every user, and regsvr32 needs HKLM (W8).
DefaultDirName={autopf}\TaigiKeyboard
DisableDirPage=yes
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
; x64 only for v1 (roadmap W8); the 32-bit DLL is shipped once x64 is proven.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Inno's own default is 6.1sp1 (Windows 7 SP1). 17763 is Windows 10 1809,
; the oldest build the Windows App Runtime supports (Microsoft, Windows App
; SDK § supported Windows releases) — the settings window is WinUI 3 over
; that runtime and does not start below it, so this is a hard floor, not a
; preference. The rest degrades rather than fails: the theme comes from
; `AppsUseLightTheme`, the windows declare per-monitor DPI v2, and the UI
; asks DWM for the dark title bar and the accent colour. What is actually
; run is Windows 11; every Windows 10 below 22H2 is out of support.
MinVersion=10.0.17763
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\{#SettingsExe}
SetupIconFile=..\resources\TaigiKeyboard.ico
OutputBaseFilename=TaigiKeyboard-{#AppVersion}-Setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
; The DLL is unregistered and lock-checked by [Code] before files are copied;
; Inno's own "close applications" dialog would name every host process.
CloseApplications=no
SetupLogging=yes

; The macOS bundle's system localizations (App/*.lproj: Hanji, English,
; Japanese), Hanji first = the fallback when the user's UI language is none
; of them (the app's own default). ChineseTraditional.isl is official since
; Inno Setup 6.5.
[Languages]
Name: "chinesetraditional"; MessagesFile: "compiler:Languages\ChineseTraditional.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

; [CustomMessages] come from i18n/desktop.json (`desktop.installer*`), written
; by the repo-root `make i18n` — never edited here.
#include "Messages.iss"

[Files]
Source: "{#Dist}\{#ServiceDll}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Dist}\x86\{#ServiceDll}"; DestDir: "{app}\x86"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#Dist}\{#SettingsExe}"; DestDir: "{app}"; Flags: ignoreversion
; W17: the Windows App Runtime files sit beside the exe (their loader looks
; in the exe's directory); Inno records each and removes them on uninstall.
Source: "{#Dist}\Runtime\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#Dist}\Dictionaries\*"; DestDir: "{app}\Dictionaries"; Flags: ignoreversion
Source: "{#Dist}\Fonts\*"; DestDir: "{app}\Fonts"; Flags: ignoreversion
Source: "{#Dist}\update-check-task.xml"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; The AUMID is what lets an unpackaged desktop app post toasts (W9); the
; updater's toast is silently dropped without this shortcut.
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#SettingsExe}"; AppUserModelID: "{#AppUserModelID}"

; Registration and the scheduled task are NOT [Run] entries: Inno ignores a
; [Run] entry's exit code, and a DLL that failed to register or a task that
; failed to create must be an installation FAILURE (PR10 Codex) — see
; RegisterEverything in [Code], which rolls back and raises.

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/IM {#SettingsExe} /F"; Flags: runhidden waituntilterminated; RunOnceId: "StopSettings"
Filename: "{sys}\schtasks.exe"; Parameters: "/Delete /TN ""{#TaskName}"" /F"; Flags: runhidden waituntilterminated runasoriginaluser; RunOnceId: "DeleteTask"
Filename: "{syswow64}\regsvr32.exe"; Parameters: "/s /u ""{app}\x86\{#ServiceDll}"""; Flags: runhidden waituntilterminated; RunOnceId: "UnregisterX86"; Check: FileExists(ExpandConstant('{app}\x86\{#ServiceDll}'))
Filename: "{sys}\regsvr32.exe"; Parameters: "/s /u ""{app}\{#ServiceDll}"""; Flags: runhidden waituntilterminated; RunOnceId: "UnregisterX64"

[UninstallDelete]
Type: filesandordirs; Name: "{app}\Dictionaries"
Type: filesandordirs; Name: "{app}\Fonts"
Type: filesandordirs; Name: "{app}\x86"
; %APPDATA%\TaigiKeyboard (settings, learning data, custom dictionary) is the
; user's and is deliberately NOT listed here.

[Code]
var
  BackupDll: String;
  FinishedNoteAdded: Boolean;

// The DLL cannot be replaced while a host process holds it. The old DLL is
// unregistered first (so new processes stop loading it), then a rename
// proves whether it is free; when it is not, the user gets the one recipe
// that works — switch input method, sign out, sign in (rakukan).
function IsServiceDllLocked(const Path: String): Boolean;
var
  Temporary: String;
  Attempt: Integer;
begin
  Result := False;
  if not FileExists(Path) then Exit;
  Temporary := Path + '.setup-probe';
  Result := True;
  for Attempt := 1 to 3 do begin
    if RenameFile(Path, Temporary) then begin
      RenameFile(Temporary, Path);
      Result := False;
      Break;
    end;
    Sleep(1000);
  end;
end;

// regsvr32 as a checked call: launch failure and a non-zero exit are both
// failures (a [Run] entry would swallow them).
function RegisterDll(const Regsvr32, Dll: String; Unregister: Boolean): Boolean;
var
  ResultCode: Integer;
  Parameters: String;
begin
  if Unregister then
    Parameters := '/s /u "' + Dll + '"'
  else
    Parameters := '/s "' + Dll + '"';
  Result := Exec(Regsvr32, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode) and (ResultCode = 0);
  Log(Format('regsvr32 %s -> %d', [Parameters, ResultCode]));
end;

function X64Regsvr32: String;
begin
  Result := ExpandConstant('{sys}\regsvr32.exe');
end;

function X86Regsvr32: String;
begin
  Result := ExpandConstant('{syswow64}\regsvr32.exe');
end;

function X64Dll: String;
begin
  Result := ExpandConstant('{app}\{#ServiceDll}');
end;

function X86Dll: String;
begin
  Result := ExpandConstant('{app}\x86\{#ServiceDll}');
end;

procedure StopSettingsWindow;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM {#SettingsExe} /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

// Before the files are copied (after Install was pressed): stop the window,
// keep a copy of the installed DLL for the rollback, unregister it, and
// refuse to go on while a host still holds either DLL.
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  NeedsRestart := False;
  StopSettingsWindow;
  BackupDll := '';
  if FileExists(X64Dll) then begin
    BackupDll := X64Dll + '.setup-backup';
    CopyFile(X64Dll, BackupDll, False);
    RegisterDll(X64Regsvr32, X64Dll, True);
  end;
  if FileExists(X86Dll) then
    RegisterDll(X86Regsvr32, X86Dll, True);
  if IsServiceDllLocked(X64Dll) or IsServiceDllLocked(X86Dll) then begin
    // Put the old registration back: nothing was replaced.
    if BackupDll <> '' then begin
      RegisterDll(X64Regsvr32, X64Dll, False);
      DeleteFile(BackupDll);
      BackupDll := '';
    end;
    if FileExists(X86Dll) then
      RegisterDll(X86Regsvr32, X86Dll, False);
    Result := CustomMessage('installerDllLocked');
  end;
end;

// The scheduled task is the ORIGINAL user's (roadmap W9: per-user), created
// from the shipped definition with the installed exe's path filled in.
// UTF-8 declared, ASCII content: Inno's string file functions are ANSI, and
// the directory is fixed to Program Files (no dir page).
function RegisterUpdateTask: Boolean;
var
  Definition: AnsiString;
  Text: String;
  XmlPath: String;
  ResultCode: Integer;
begin
  Result := False;
  XmlPath := ExpandConstant('{app}\update-check-task.xml');
  if not LoadStringFromFile(XmlPath, Definition) then Exit;
  Text := String(Definition);
  StringChangeEx(Text, '@@COMMAND@@', ExpandConstant('{app}\{#SettingsExe}'), True);
  if not SaveStringToFile(XmlPath, AnsiString(Text), False) then Exit;
  Result := ExecAsOriginalUser(ExpandConstant('{sys}\schtasks.exe'),
    '/Create /TN "{#TaskName}" /XML "' + XmlPath + '" /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode)
    and (ResultCode = 0);
  Log(Format('schtasks /Create -> %d', [ResultCode]));
end;

// Undo what this run did and put the previous version's registration back.
procedure RollBack;
var
  ResultCode: Integer;
begin
  ExecAsOriginalUser(ExpandConstant('{sys}\schtasks.exe'), '/Delete /TN "{#TaskName}" /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  if FileExists(X86Dll) then
    RegisterDll(X86Regsvr32, X86Dll, True);
  RegisterDll(X64Regsvr32, X64Dll, True);
  if BackupDll <> '' then begin
    CopyFile(BackupDll, X64Dll, False);
    RegisterDll(X64Regsvr32, X64Dll, False);
  end;
end;

// After the files are in place: register, create the task — and treat
// either failing as THE installation failing: roll back, then raise, which
// makes Setup report an error and undo its file installation.
procedure RegisterEverything;
var
  Failure: String;
begin
  Failure := '';
  if not RegisterDll(X64Regsvr32, X64Dll, False) then
    Failure := 'regsvr32 ' + X64Dll
  else if FileExists(X86Dll) and not RegisterDll(X86Regsvr32, X86Dll, False) then
    Failure := 'regsvr32 ' + X86Dll
  else if not RegisterUpdateTask then
    Failure := 'schtasks /Create {#TaskName}';
  if Failure <> '' then begin
    RollBack;
    RaiseException(FmtMessage(CustomMessage('installerStepFailed'), [Failure]));
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    RegisterEverything;
  if (CurStep = ssDone) and (BackupDll <> '') then begin
    DeleteFile(BackupDll);
    BackupDll := '';
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if (CurPageID = wpFinished) and not FinishedNoteAdded then begin
    FinishedNoteAdded := True;
    WizardForm.FinishedLabel.Caption := WizardForm.FinishedLabel.Caption + #13#10#13#10 + CustomMessage('installerSignOutNote');
  end;
end;
