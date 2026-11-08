; TaigiKeyboard for Windows — Inno Setup 6.5+ script (roadmap W8; `x64compatible`
; is 6.3 syntax, the official ChineseTraditional.isl 6.5).
;
; Compiled by windows/scripts/release-app.sh:
;   iscc /DAppVersion=<x.y.z> /DDist=<staging dir> /O<output dir> TaigiKeyboard.iss
;
; The staging dir holds what the script ships:
;   TaigiKeyboard.dll              (x64 text service, signed)
;   TaigiKeyboard32.dll            (x86 text service for 32-bit hosts, signed;
;                                   beside the x64 one — windows-release.md
;                                   § The 32-bit service)
;   TaigiKeyboardArm64.dll         (Arm64 text service, signed; installed on
;                                   an Arm64 machine only)
;   TaigiKeyboard64X.dll           (Arm64X forwarder, signed; Arm64 only — the
;                                   InProcServer32 that sends Arm64 hosts to
;                                   the Arm64 service and x64-emulated hosts to
;                                   the x64 one, docs/architecture/
;                                   windows-arm64-roadmap.md)
;   TaigiKeyboardSettings.exe      (x64, signed)
;   arm64\TaigiKeyboardSettings.exe (Arm64, signed; installed under the same
;                                   name on an Arm64 machine)
;   Runtime\*                      (the Windows App Runtime the settings window
;                                   runs on — self-contained, roadmap W17;
;                                   installed BESIDE the exe)
;   RuntimeArm64\*                 (the same set for the Arm64 exe)
;   Dictionaries\*                 (from the repo-root assets/dictionaries)
;   Fonts\*                        (from the repo-root assets/fonts/font)
;   update-check-task.xml          (the scheduled task's definition)
;
; What it does that a plain file copy would not (rakukan `rakukan_installer.iss`,
; PIME `installer.nsi`): registers the DLL with the matching regsvr32 per
; architecture, stops the settings window and unregisters the old DLL before
; an upgrade, MOVES ASIDE every payload file a running host still holds so the
; upgrade lands without a sign-out (see MakeWay in [Code]), creates the
; Start-menu shortcut that carries the toast AUMID, registers the daily
; update-check task, and reverses every step on uninstall — leaving
; %APPDATA%\TaigiKeyboard (the user's learning data) alone.

#ifndef AppVersion
  #error Pass /DAppVersion=<x.y.z> (windows/scripts/release-app.sh does)
#endif
#ifndef Dist
  #error Pass /DDist=<staging dir> (windows/scripts/release-app.sh does)
#endif
#ifndef ProductNameStringId
  #error Pass /DProductNameStringId=<id> (windows/scripts/release-app.sh reads it from build-support/resource.rs)
#endif

#define AppName "TaigiKeyboard"
#define AppPublisher "Soo Bîn-hiân 蘇民弦"
#define AppContactMail "info@taigikeyboard.tw"
#define AppURL "https://taigikeyboard.tw"
#define AppUserModelID "TaigiKeyboard.Settings"
#define TaskName "TaigiKeyboard Update Check"
#define SettingsExe "TaigiKeyboardSettings.exe"
#define ServiceDll "TaigiKeyboard.dll"
#define ServiceDll32 "TaigiKeyboard32.dll"
#define ServiceDllArm64 "TaigiKeyboardArm64.dll"
#define ForwarderDll "TaigiKeyboard64X.dll"
; The one Start-menu entry, spelled once for [Icons] and for the two [Code]
; sites that localize and un-localize its display name.
#define ShortcutName "{autoprograms}\" + AppName

[Setup]
; Stable across versions: what makes a later installer an UPGRADE.
AppId={{6F0D8C2B-4A57-4E3F-9B1E-2D7C3A8E5F61}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppContact={#AppContactMail}
AppUpdatesURL={#AppURL}
; The updater pins these against the running settings exe's own VERSIONINFO
; (taigi-windows-update::verify): ProductName identical, ProductVersion = the
; workspace version.
VersionInfoVersion={#AppVersion}
VersionInfoProductVersion={#AppVersion}
VersionInfoProductName={#AppName}
VersionInfoDescription={#AppName} Setup
VersionInfoCompany={#AppPublisher}
VersionInfoCopyright=Copyright (c) 2025-2026 Soo Bîn-hiân 蘇民弦. Apache License 2.0.
; Machine-wide, under Program Files: a text service is loaded into every
; process of every user, and regsvr32 needs HKLM (W8).
DefaultDirName={autopf}\TaigiKeyboard
DisableDirPage=yes
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
; No "Select Setup Language" page: the system already says which language the
; user reads, and every other page is disabled anyway, so a picker would be
; the only thing standing between the user and the install. `uilanguage` reads
; GetUserDefaultUILanguage() — the UI language, NOT the region — which is the
; same distinction the app itself draws (`system_locale` reads
; GetUserPreferredUILanguages, roadmap parity audit #640). A UI language that
; matches no entry falls back to the FIRST one listed, Hanji, which is also
; what the app falls back to. `/LANG=english` on the command line still wins.
ShowLanguageDialog=no
LanguageDetectionMethod=uilanguage
; One universal installer (docs/architecture/windows-arm64-roadmap.md D3):
; `x64compatible` matches x64 Windows and Arm64 Windows 11. A text service
; loads in the host process's own architecture, so an Arm64 machine gets the
; Arm64 service, the x64 one for x64-emulated hosts and the Arm64X forwarder
; between them ([Files] `Check: IsArm64`). Arm64 Windows 10 has no x64
; emulation and `x64compatible` does not match it, so Inno itself refuses it.
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
OutputBaseFilename=TaigiKeyboard-{#AppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
; [Code] unregisters the old DLL and moves every held payload file aside before
; files are copied, so nothing here has to close a host. Inno's own "close
; applications" dialog would name explorer and the text-input infrastructure —
; processes an input method must never ask a user to kill.
CloseApplications=no
; A log in %TEMP% on every run. It is what turned the first real install
; failure into a one-line diagnosis: the screen showed only a step name,
; while the log had `schtasks /Create -> 1` next to `regsvr32 -> 0`.
SetupLogging=yes

; The macOS bundle's system localizations (App/*.lproj: Hanji, English,
; Japanese), Hanji first = the fallback when the user's UI language is none
; of them (the app's own default).
;
; ChineseTraditional.isl is vendored rather than taken from `compiler:Languages`.
; It is still an UNOFFICIAL translation as of Inno Setup 6.7.3 — it lives in the
; source tree under `Files/Languages/Unofficial/`, and Inno's own installer
; (`setup.iss`: `Source: "files\Languages\*.isl"`) does not descend into that
; directory, so a stock installation does not have the file at all. Depending on
; it meant depending on someone having dropped it into the Inno directory by
; hand, which is exactly the kind of undeclared machine state a release must not
; rest on. Japanese.isl and Default.isl are official and stay where they are.
[Languages]
Name: "chinesetraditional"; MessagesFile: "{#SourcePath}Languages\ChineseTraditional.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

; [CustomMessages] come from i18n/desktop.json (`desktop.installer*`), written
; by the repo-root `make i18n` — never edited here.
#include "Messages.iss"

[Files]
; The x64 service on both machines: on Arm64 the x64-emulated hosts load it
; through the forwarder.
Source: "{#Dist}\{#ServiceDll}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Dist}\{#ServiceDll32}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Dist}\{#ServiceDllArm64}"; DestDir: "{app}"; Flags: ignoreversion; Check: IsArm64
Source: "{#Dist}\{#ForwarderDll}"; DestDir: "{app}"; Flags: ignoreversion; Check: IsArm64
; The settings window and its runtime in the machine's own architecture, under
; one install name, so the shortcut, the uninstall icon, the update task and
; the updater's own-exe check are the same on both.
Source: "{#Dist}\{#SettingsExe}"; DestDir: "{app}"; Flags: ignoreversion; Check: not IsArm64
Source: "{#Dist}\arm64\{#SettingsExe}"; DestDir: "{app}"; Flags: ignoreversion; Check: IsArm64
; W17: the Windows App Runtime files sit beside the exe (their loader looks
; in the exe's directory); Inno records each and removes them on uninstall.
Source: "{#Dist}\Runtime\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Check: not IsArm64
Source: "{#Dist}\RuntimeArm64\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Check: IsArm64
Source: "{#Dist}\Dictionaries\*"; DestDir: "{app}\Dictionaries"; Flags: ignoreversion
Source: "{#Dist}\Fonts\*"; DestDir: "{app}\Fonts"; Flags: ignoreversion
Source: "{#Dist}\update-check-task.xml"; DestDir: "{app}"; Flags: ignoreversion

; Inno's uninstall log is name-keyed, so the pre-rename "Taigi Keyboard"
; shortcut would sit beside the new one on a machine that ran an older build.
[InstallDelete]
Type: files; Name: "{autoprograms}\Taigi Keyboard.lnk"
; The bundled typefaces were renamed when they moved to the repo-root
; assets/fonts/font directory (3.6.8). `ignoreversion` only overwrites files it also
; ships, so an upgrade from 3.6.7 or earlier would leave the four old-name
; copies behind as ~40 MB of dead weight until uninstall.
Type: files; Name: "{app}\Fonts\GenYoGothic2TW-R.otf"
Type: files; Name: "{app}\Fonts\GenYoMin2TW-R.otf"
Type: files; Name: "{app}\Fonts\Iansui-Regular.ttf"
Type: files; Name: "{app}\Fonts\jf-openhuninn-2.1.ttf"

[Icons]
; The AUMID is what lets an unpackaged desktop app post toasts (W9); the
; updater's toast is silently dropped without this shortcut. The shortcut's
; FILE name stays untranslated; its display name is localized afterwards, in
; [Code] — see LocalizeShortcutName.
Name: "{#ShortcutName}"; Filename: "{app}\{#SettingsExe}"; AppUserModelID: "{#AppUserModelID}"

[Registry]
; The name Windows' "Installed apps" list shows, as a resource reference the
; system resolves against the user's UI language (Microsoft, "Using registry
; string redirection"). `UninstallDisplayName`
; above stays the plain name for any reader that does not understand the
; `_Localized` value — the same pair Windows' own Remote Desktop entry carries
; (measured on Windows 11: `DisplayName_Localized =
; @C:\Windows\System32\mstsc.exe,-4000`, REG_EXPAND_SZ). The subkey is Inno's
; own uninstall key, spelled from the AppId so the two cannot drift, and it is
; removed with that key on uninstall.
Root: HKLM; Subkey: "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{#SetupSetting('AppId')}_is1"; ValueType: expandsz; ValueName: "DisplayName_Localized"; ValueData: "@{app}\{#SettingsExe},-{#ProductNameStringId}"

; Registration and the scheduled task are NOT [Run] entries: Inno ignores a
; [Run] entry's exit code, and a DLL that failed to register must be an
; installation FAILURE (PR10 Codex) — see RegisterEverything in [Code], which
; rolls back and raises. The task's own failure is reported, not fatal.

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/IM {#SettingsExe} /F"; Flags: runhidden waituntilterminated; RunOnceId: "StopSettings"
; The uninstaller is elevated and the task sits in the Task Scheduler root
; folder, which only an administrator can write — the same reason the install
; side creates it elevated (RegisterUpdateTask).
Filename: "{sys}\schtasks.exe"; Parameters: "/Delete /TN ""{#TaskName}"" /F"; Flags: runhidden waituntilterminated; RunOnceId: "DeleteTask"
Filename: "{syswow64}\regsvr32.exe"; Parameters: "/s /u ""{app}\{#ServiceDll32}"""; Flags: runhidden waituntilterminated; RunOnceId: "UnregisterX86"
; The native 64-bit service unregisters itself (NativeDll in [Code]). Never
; the forwarder: it exports only the two activation entry points.
Filename: "{sys}\regsvr32.exe"; Parameters: "/s /u ""{app}\{#ServiceDll}"""; Flags: runhidden waituntilterminated; RunOnceId: "UnregisterX64"; Check: not IsArm64
Filename: "{sys}\regsvr32.exe"; Parameters: "/s /u ""{app}\{#ServiceDllArm64}"""; Flags: runhidden waituntilterminated; RunOnceId: "UnregisterArm64"; Check: IsArm64

[UninstallDelete]
Type: filesandordirs; Name: "{app}\Dictionaries"
Type: filesandordirs; Name: "{app}\Fonts"
; Stale copies are NOT listed here: they can sit in any subdirectory the
; payload has, and an [UninstallDelete] pattern does not recurse. They are
; collected in [Code], at usPostUninstall — see DiscardLeftoverTree.
; %APPDATA%\TaigiKeyboard (settings, learning data, custom dictionary) is the
; user's and is deliberately NOT listed here.

[Code]
// The Start-menu shortcut's DISPLAY name, in the user's UI language. The shell
// keeps a file's localized name in its folder's desktop.ini, under
// [LocalizedFileNames], as an `@<file>,-<id>` reference it resolves against the
// user's UI language (Microsoft, "Locating redirected strings"); Windows names
// its own Start-menu entries exactly this way. SHSetLocalizedName writes that
// line and SHRemoveLocalizedName takes it back out — neither touches the .lnk
// itself, so its file name, its target and its AUMID are unaffected. The Mac
// gets the same thing from CFBundleDisplayName in the bundle's .lproj (#613).
function SHSetLocalizedName(pszPath: String; pszResModule: String; idsRes: Integer): Integer;
  external 'SHSetLocalizedName@shell32.dll stdcall';
function SHRemoveLocalizedName(pszPath: String): Integer;
  external 'SHRemoveLocalizedName@shell32.dll stdcall';

function ShortcutPath: String;
begin
  Result := ExpandConstant('{#ShortcutName}.lnk');
end;

// Windows cannot delete a file whose image section a live process still has
// mapped — DeleteFile answers ERROR_ACCESS_DENIED (5) — but it CAN rename one,
// because a rename touches the directory entry and not the section. Every file
// this installer replaces that a host can hold is therefore moved aside rather
// than deleted, and the new payload is written at the now-empty canonical path.
// Hosts that already mapped the old files keep running from the renamed copies
// until they exit; new hosts load the new ones.
//
// WHICH files are moved is defined by EXCLUSION — everything under {app}
// except the two files Setup itself owns. The obvious alternative, listing what
// a host can hold (the DLL, Dictionaries\*, Fonts\*, the runtime), is a second
// copy of the payload list that [Files] already defines 250 lines above, with
// nothing keeping the two in step: `update-check-task.xml` was in [Files] and
// would have been missing from such a list on day one, and `Runtime\*` installs
// `recursesubdirs` into 89 locale directories that a non-recursive list cannot
// reach. Excluding instead of enumerating means a file added to [Files] in a
// later release is covered without anyone remembering this section exists.
//
// Known holders, for the record: the TSF DLL is mapped as an image by every
// host that loaded it; `Dictionaries\*` are mmap'd by engine/lexicon
// (dictionary_reader.rs:160, association_reader.rs:64); `Fonts\*` go through
// DirectWrite AddFontFile (ui/render.rs:296-322); the Windows App Runtime and
// its locale resources belong to the settings window, which StopSettingsWindow
// kills first but could relaunch a moment later.
//
// Not all of those actually refuse a delete, and that is the point of not
// asking. Measured on the box: with the DLL loaded and all eight dictionary and
// font files mapped, only the DLL survived the delete — an IMAGE section has no
// delete-pending state, while a DATA section over a file opened with
// FILE_SHARE_DELETE (which is what Rust's File::open requests) deletes fine and
// frees when the last handle closes. A guard would have had to know that
// distinction per file, and get it right again for every file a later release
// adds. Renaming works on all of them, so nothing here has to know.
const
  StaleExtension = '.stale';

type
  TMovedFile = record
    Original: String;
    Stale: String;
  end;

var
  MovedFiles: array of TMovedFile;
  FinishedNoteAdded: Boolean;
  UpdateTaskFailed: Boolean;
  // Which services the version being replaced had, taken before anything is
  // moved: a restore registers only those (RestorePreviousVersion), so a
  // failed FRESH install registers nothing, and a failed upgrade from a
  // version without the 32-bit service does not register the new one.
  HadNativeDll: Boolean;
  HadX86Dll: Boolean;

// Delete, or fail that and have Windows delete it at the next restart.
// Delete, or fail that and have Windows delete it at the next restart.
//
// Deleting at the next restart is how a copy a live process still has mapped
// finally goes away. It does NOT make a restart part of installing: the new
// version is in place and usable when Setup finishes; this only reclaims the
// old bytes whenever the machine next reboots on its own.
//
// `RestartReplace(Path, '')` is the documented delete form — "if DestFile is ''
// then TempFile will be deleted" — and raises rather than returning a result.
// Inno's DelayDeleteFile is the wrong sibling and cannot substitute: it retries
// the delete a few times, 250 msec apart, and gives up. Retrying cannot outlast
// a live mapping, because the holder will not let go until it exits.
procedure DiscardStale(const Path: String);
begin
  if DeleteFile(Path) then Exit;
  try
    RestartReplace(Path, '');
    Log('stale: ' + Path + ' still held — deleting at next restart');
  except
    Log('stale: could not delete or schedule ' + Path);
  end;
end;

// Where a file goes when it is moved aside. DERIVED from the file itself,
// `TaigiKeyboard.dll` -> `TaigiKeyboard.dll.stale`, so that a leftover names
// the payload file it came from: a run killed between moving a file and putting
// it back leaves the only copy under that name, and the next run can recognise
// it and put it back (RecoverStaleFiles). A random name would make the
// leftovers unreadable, which is exactly the hole RecoverLegacyLeftovers exists
// to patch for the previous installer.
//
// A copy from an earlier upgrade can already own the name when the machine has
// not rebooted since; it is old bytes, so it is collected first, and only if it
// is STILL held does this fall back to a unique name it cannot recover from.
function StaleNameFor(const Path: String): String;
begin
  Result := Path + StaleExtension;
  if not FileExists(Result) then Exit;
  DiscardStale(Result);
  if FileExists(Result) then
    Result := GenerateUniqueName(ExtractFileDir(Path), StaleExtension);
end;

// Move one payload file out of the way, remembering the pair so the move can
// be undone. Always a rename, never a delete: the rename is what makes this
// REVERSIBLE, and a later step failing must be able to put the previous
// version back exactly as it was.
function MakeWay(const Path: String): Boolean;
var
  Stale: String;
  Index: Integer;
begin
  Result := True;
  if not FileExists(Path) then Exit;
  Stale := StaleNameFor(Path);
  if not RenameFile(Path, Stale) then begin
    Log('make way: could not move ' + Path + ' aside');
    Result := False;
    Exit;
  end;
  Index := GetArrayLength(MovedFiles);
  SetArrayLength(MovedFiles, Index + 1);
  MovedFiles[Index].Original := Path;
  MovedFiles[Index].Stale := Stale;
end;

// Every file under Directory, recursively. Skipped: Setup's own uninstaller and
// its log, which this installer does not ship and must not strand, and stale
// copies, which moving would only breed more of.
function MakeWayForTree(const Directory: String): Boolean;
var
  Found: TFindRec;
  Full: String;
begin
  Result := True;
  if not DirExists(Directory) then Exit;
  if not FindFirst(AddBackslash(Directory) + '*', Found) then Exit;
  try
    repeat
      if (Found.Name = '.') or (Found.Name = '..') then
        Continue;
      Full := AddBackslash(Directory) + Found.Name;
      if Found.Attributes and FILE_ATTRIBUTE_DIRECTORY <> 0 then begin
        if not MakeWayForTree(Full) then begin
          Result := False;
          Exit;
        end;
      end else if not SameText(Copy(Found.Name, 1, 5), 'unins') and
                  not SameText(ExtractFileExt(Found.Name), StaleExtension) then begin
        if not MakeWay(Full) then begin
          Result := False;
          Exit;
        end;
      end;
    until not FindNext(Found);
  finally
    FindClose(Found);
  end;
end;

// Undo every move this run made, newest first, and drop whatever the payload
// managed to write at those paths. After this the machine holds exactly the
// files it held before Setup started.
procedure PutBackMovedFiles;
var
  Index: Integer;
  Original: String;
begin
  for Index := GetArrayLength(MovedFiles) - 1 downto 0 do begin
    Original := MovedFiles[Index].Original;
    // The new file may itself have been mapped by now — regsvr32 registered it
    // before the step that failed, so a host could have activated it. Deleting
    // it then answers ERROR_ACCESS_DENIED, and the restore would fail for want
    // of an empty path. Renaming it aside works on a mapped image where
    // deleting does not, which is the whole premise of this file.
    if FileExists(Original) and not DeleteFile(Original) then
      RenameFile(Original, StaleNameFor(Original));
    if not RenameFile(MovedFiles[Index].Stale, Original) then
      Log('put back: could not restore ' + Original +
          ' from ' + MovedFiles[Index].Stale);
  end;
  SetArrayLength(MovedFiles, 0);
end;

// The install stuck: the previous version's files are not coming back, so the
// copies moved aside are now just old bytes.
procedure DiscardMovedFiles;
var
  Index: Integer;
begin
  for Index := 0 to GetArrayLength(MovedFiles) - 1 do
    DiscardStale(MovedFiles[Index].Stale);
  SetArrayLength(MovedFiles, 0);
end;

// One leftover from an interrupted earlier run. When the canonical path is
// EMPTY the leftover is the only copy of that file left on the machine — a run
// was killed after moving it aside and before putting it back — so it goes
// back. Otherwise it is old bytes and is collected. Failure to collect is not
// fatal: a copy a host still holds simply survives another cycle.
procedure RecoverOrDiscard(const Leftover, Original: String);
begin
  if not FileExists(Leftover) then Exit;
  if not FileExists(Original) and RenameFile(Leftover, Original) then
    Log('recovered ' + Original + ' — an earlier run was interrupted after moving it aside')
  else
    DiscardStale(Leftover);
end;

// Everything the uninstaller left under Directory, recursively, then Directory
// itself: deleted now where it can be, otherwise queued for the next restart.
// Used on uninstall, where nothing is coming back.
//
// Two kinds of file survive Inno's own pass. Stale copies: Inno's uninstall log
// does not know these names — they are created after it is written — and an
// [UninstallDelete] pattern cannot reach into the payload's subdirectories.
// And payload files a running host still has mapped (the service DLLs, in any
// app the input method was active in): Inno logs "it may be in use" and moves
// on. Those are renamed to a fresh unique stale name before being queued, never
// queued under their own name: a reinstall before the restart writes the new
// payload at that very path, and a delete queued against the path would take
// the new file at boot. The unique name is also one RecoverStaleFiles can only
// discard — it records no original, so a reinstall never resurrects an
// uninstalled DLL. `uninsrestartdelete` in [Files] would queue the path itself,
// which is why it is not used.
//
// Pending operations run in order at boot, so each directory is queued after
// what it held, children before parents. Every other file left is treated as
// payload, a user's own included: {app} sits under Program Files, which only an
// administrator writes. Setup's own uninstaller files are already gone by
// usPostUninstall; the `unins` skip covers only Inno failing to delete them,
// which Inno handles itself.
procedure DiscardLeftoverTree(const Directory: String);
var
  Found: TFindRec;
  Full, Aside: String;
begin
  if not DirExists(Directory) then Exit;
  if FindFirst(AddBackslash(Directory) + '*', Found) then
    try
      repeat
        if (Found.Name = '.') or (Found.Name = '..') then
          Continue;
        Full := AddBackslash(Directory) + Found.Name;
        if Found.Attributes and FILE_ATTRIBUTE_DIRECTORY <> 0 then
          DiscardLeftoverTree(Full)
        else if SameText(ExtractFileExt(Found.Name), StaleExtension) then
          DiscardStale(Full)
        else if not SameText(Copy(Found.Name, 1, 5), 'unins') and
                not DeleteFile(Full) then begin
          Aside := GenerateUniqueName(Directory, StaleExtension);
          if RenameFile(Full, Aside) then begin
            Log('uninstall: ' + Full + ' is still held — moved aside as ' + Aside);
            DiscardStale(Aside);
          end else
            Log('uninstall: could not move ' + Full + ' aside; left in place');
        end;
      until not FindNext(Found);
    finally
      FindClose(Found);
    end;
  if RemoveDir(Directory) then Exit;
  try
    RestartReplace(Directory, '');
    Log('uninstall: ' + Directory + ' is not empty yet — removing it at next restart');
  except
    Log('uninstall: could not schedule ' + Directory + ' for removal');
  end;
end;

// Every stale copy under Directory, recursively, matching how they are made.
procedure RecoverStaleFiles(const Directory: String);
var
  Found: TFindRec;
  Full, Original: String;
begin
  if not DirExists(Directory) then Exit;
  if not FindFirst(AddBackslash(Directory) + '*', Found) then Exit;
  try
    repeat
      if (Found.Name = '.') or (Found.Name = '..') then
        Continue;
      Full := AddBackslash(Directory) + Found.Name;
      if Found.Attributes and FILE_ATTRIBUTE_DIRECTORY <> 0 then
        RecoverStaleFiles(Full)
      else if SameText(ExtractFileExt(Found.Name), StaleExtension) then begin
        Original := ChangeFileExt(Full, '');
        // A payload file always has an extension of its own (.dll, .bin, .ttf,
        // .xml). A leftover whose name does not resolve to one came from
        // StaleNameFor's unique-name fallback, which records nothing about
        // where it came from — that one can only be discarded.
        if ExtractFileExt(Original) <> '' then
          RecoverOrDiscard(Full, Original)
        else
          DiscardStale(Full);
      end;
    until not FindNext(Found);
  finally
    FindClose(Found);
  end;
end;

// A pre-3.6.8 installer probed for a lock by renaming the DLL to `.setup-probe`
// and renaming it back WITHOUT checking that the second rename worked, and kept
// a `.setup-backup` copy it deleted only on success. Either name can therefore
// hold the only service DLL on a machine that ran windows-v3.6.7.
//
// Delete this procedure once no supported upgrade path starts before 3.6.8 —
// it is a migration shim, and the naming scheme above is what stops the same
// shim being needed again.
procedure RecoverLegacyLeftovers(const Path: String);
begin
  RecoverOrDiscard(Path + '.setup-probe', Path);
  RecoverOrDiscard(Path + '.setup-backup', Path);
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

// In 64-bit install mode `{sys}` is the native System32 on both machines, so
// this is the x64 regsvr32 on x64 Windows and the Arm64 one on Arm64.
function NativeRegsvr32: String;
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
  Result := ExpandConstant('{app}\{#ServiceDll32}');
end;

// The 64-bit service that registers itself. Never the x64 one on Arm64: it
// would overwrite InProcServer32 with a path Arm64 hosts cannot load.
function NativeDll: String;
begin
  if IsArm64 then
    Result := ExpandConstant('{app}\{#ServiceDllArm64}')
  else
    Result := X64Dll;
end;

procedure StopSettingsWindow;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM {#SettingsExe} /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

// Before the files are copied (after Install was pressed): stop the settings
// window, collect what an interrupted earlier run left behind, unregister the
// old service, and move every installed file aside so the payload writes onto
// empty paths. The moved originals ARE the rollback material — there is no
// separate backup copy.
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  AppDir: String;
begin
  Result := '';
  NeedsRestart := False;
  AppDir := ExpandConstant('{app}');
  StopSettingsWindow;

  // Leftovers first, so this run's own moves are the only stale copies in
  // flight when it starts.
  RecoverLegacyLeftovers(X64Dll);
  RecoverStaleFiles(AppDir);
  HadNativeDll := FileExists(NativeDll);
  HadX86Dll := FileExists(X86Dll);

  // Unregister BEFORE moving anything: it stops new activations finding the
  // old DLL, which shrinks the window in which a host maps a file this is
  // about to move. A failure here is logged, not fatal — the move below empties
  // the path anyway, so nothing can be loaded from it, and the new DLL's own
  // registration overwrites the same CLSID.
  if HadNativeDll and not RegisterDll(NativeRegsvr32, NativeDll, True) then
    Log('unregister: regsvr32 /u ' + NativeDll + ' failed');
  if HadX86Dll and not RegisterDll(X86Regsvr32, X86Dll, True) then
    Log('unregister: regsvr32 /u ' + X86Dll + ' failed');

  // Only a file Windows will not even RENAME stops the install now, which is a
  // far rarer thing than one a host holds — and the message names the recipe
  // that frees it. DeinitializeSetup puts back what this managed to move.
  if not MakeWayForTree(AppDir) then
    Result := CustomMessage('installerDllLocked');
end;

// The daily update check (roadmap W9), created from the shipped definition
// with the installed exe's path filled in. The definition stays ASCII: Inno's
// string file functions are ANSI, and the directory is fixed to Program Files
// (no dir page).
//
// Created from Setup's own ELEVATED context: the Task Scheduler ROOT folder
// needs a high-integrity token, and an unelevated create answers `ERROR:
// Access is denied.` The definition names no UserId, so the task binds to the
// account Setup runs as. Both in docs/architecture/windows-release.md.
function RegisterUpdateTask: Boolean;
var
  Definition: AnsiString;
  Text: String;
  XmlPath: String;
  ResultCode: Integer;
begin
  Result := False;
  XmlPath := ExpandConstant('{app}\update-check-task.xml');
  if not LoadStringFromFile(XmlPath, Definition) then begin
    Log('update task: could not read ' + XmlPath);
    Exit;
  end;
  Text := String(Definition);
  StringChangeEx(Text, '@@COMMAND@@', ExpandConstant('{app}\{#SettingsExe}'), True);
  if not SaveStringToFile(XmlPath, AnsiString(Text), False) then begin
    Log('update task: could not write ' + XmlPath);
    Exit;
  end;
  Result := Exec(ExpandConstant('{sys}\schtasks.exe'),
    '/Create /TN "{#TaskName}" /XML "' + XmlPath + '" /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode)
    and (ResultCode = 0);
  Log(Format('schtasks /Create -> %d', [ResultCode]));
end;

// PutBackMovedFiles restores only what it moved, so a payload file the previous
// version did not have stays behind. The 32-bit service is the one such file
// that matters: HadX86Dll keeps this restore from registering it, but left on
// disk it would read as the previous version's on the NEXT run, and a second
// failure would then register the new 32-bit service beside the old 64-bit
// one. Renamed before it is discarded, like every file here, because a 32-bit
// host may already have mapped it.
procedure RemoveAddedX86Dll;
var
  Stale: String;
begin
  if HadX86Dll or not FileExists(X86Dll) then Exit;
  if DeleteFile(X86Dll) then Exit;
  Stale := StaleNameFor(X86Dll);
  if RenameFile(X86Dll, Stale) then
    DiscardStale(Stale)
  else
    Log('restore: could not remove ' + X86Dll + ', which the previous version did not have');
end;

// Put the previous version back — the whole of it, not just the DLL. Every file
// the payload overwrote was moved aside rather than deleted, so restoring is a
// rename per file, and the previous version's code and its dictionaries and
// fonts go back together. That matters: the old DLL opens its dictionaries
// lazily, so a restored old DLL left beside NEW dictionaries would be reading
// files it was never built against.
procedure RestorePreviousVersion;
begin
  PutBackMovedFiles;
  RemoveAddedX86Dll;
  // Only what the previous version had, x86 first as in RegisterEverything.
  // A file merely being THERE is not enough: after a failed fresh install the
  // new payload is still on disk (Inno does not roll back past ssPostInstall),
  // and registering it would leave a half-installed service active.
  if HadX86Dll and not RegisterDll(X86Regsvr32, X86Dll, False) then
    Log('restore: put ' + X86Dll + ' back but could not register it');
  if HadNativeDll and not RegisterDll(NativeRegsvr32, NativeDll, False) then
    Log('restore: put ' + NativeDll + ' back but could not register it');
end;

// The ssPostInstall path into it: this run got as far as registering the new
// DLL, so that registration comes off before its files move away underneath it.
procedure RollBack;
begin
  RegisterDll(X86Regsvr32, X86Dll, True);
  RegisterDll(NativeRegsvr32, NativeDll, True);
  RestorePreviousVersion;
end;

procedure FailStep(const Step: String);
begin
  RollBack;
  RaiseException(FmtMessage(CustomMessage('installerStepFailed'), [Step]));
end;

// After the files are in place: register the service DLL — and treat THAT
// failing as the installation failing. The scheduled task is deliberately not
// in that tier: the input method works without it and the settings window
// checks for updates on demand, so its failure is logged and carried to the
// finished page instead.
//
// Setup finalizes the uninstall log BEFORE ssPostInstall, and its own
// documentation says that after that point "any subsequent errors will not
// cause what was installed before to be rolled back" (Inno Setup, Setup
// installation order) — so Inno itself will not undo the copied payload. This
// installer does it instead: RollBack drops the new files and renames the
// previous version's own files back, because it moved them aside rather than
// deleting them. The exception then makes Setup report the failure, and the
// user is left with the previous input method intact and an installer to run
// again.
//
// The 32-bit service registers FIRST. Each architecture writes its own
// CLSID\InProcServer32 (the 64-bit view and WOW6432Node), but the TSF profile
// and its categories under HKLM\SOFTWARE\Microsoft\CTF\TIP are ONE shared
// record that the last registration overwrites — so the native 64-bit DLL goes
// last and the profile's description and icon name it, the copy every 64-bit
// host and the Settings keyboard list read (docs/architecture/windows-release.md,
// measured 2026-09-01). On Arm64 that is the Arm64 DLL, whose InProcServer32 is
// the forwarder; x64-emulated hosts reach the x64 DLL through it.
procedure RegisterEverything;
begin
  if not RegisterDll(X86Regsvr32, X86Dll, False) then
    FailStep('regsvr32 ' + X86Dll);
  if not RegisterDll(NativeRegsvr32, NativeDll, False) then
    FailStep('regsvr32 ' + NativeDll);
  UpdateTaskFailed := not RegisterUpdateTask;
end;

// The shortcut exists by ssPostInstall ([Icons] runs during the install step).
// Not a failure tier: a shortcut under its untranslated name still starts the
// settings window, so this is logged and carried no further.
procedure LocalizeShortcutName;
var
  Failure: Integer;
begin
  Failure := SHSetLocalizedName(
    ShortcutPath, ExpandConstant('{app}\{#SettingsExe}'), {#ProductNameStringId});
  if Failure <> 0 then
    Log('shortcut: could not localize ' + ShortcutPath + ' -> ' + IntToStr(Failure));
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then begin
    RegisterEverything;
    LocalizeShortcutName;
  end;
  // ssDone is reached only when the whole install succeeded, so the previous
  // version's files are not needed any more. Discarding them also empties
  // MovedFiles, which is what tells DeinitializeSetup there is nothing to undo.
  if CurStep = ssDone then
    DiscardMovedFiles;
end;

// ssDone does not run when Setup fails or the user cancels, and this does.
// Anything still recorded here means the payload was moved aside but the
// install never completed — put the previous version back rather than leave a
// machine with half of each.
procedure DeinitializeSetup;
begin
  if GetArrayLength(MovedFiles) > 0 then begin
    Log('setup did not complete — restoring the previous version''s files');
    RestorePreviousVersion;
  end;
end;

// After the uninstaller has removed everything it logged: collect what it
// could not (DiscardLeftoverTree), so {app} can go too.
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppDir: String;
begin
  // Before the shortcut goes: the localized name lives in the SHARED Start-menu
  // folder's desktop.ini, which the uninstaller does not own and which would
  // otherwise keep a line naming a .lnk that no longer exists.
  if CurUninstallStep = usUninstall then
    if SHRemoveLocalizedName(ShortcutPath) <> 0 then
      Log('shortcut: could not remove the localized name for ' + ShortcutPath);
  if CurUninstallStep <> usPostUninstall then Exit;
  AppDir := ExpandConstant('{app}');
  DiscardLeftoverTree(AppDir);
end;

procedure CurPageChanged(CurPageID: Integer);
var
  Note: String;
begin
  if (CurPageID = wpFinished) and not FinishedNoteAdded then begin
    FinishedNoteAdded := True;
    Note := CustomMessage('installerSignOutNote');
    if UpdateTaskFailed then
      Note := Note + #13#10#13#10 + CustomMessage('installerUpdateTaskSkippedNote');
    WizardForm.FinishedLabel.Caption := WizardForm.FinishedLabel.Caption + #13#10#13#10 + Note;
  end;
end;
