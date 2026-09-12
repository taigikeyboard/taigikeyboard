# Windows Desktop IME — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `windows`, `TSF`, `Text Services Framework`, `fourth platform`, `engine reuse`, `macOS parity`
> **Status**: Phase 0 approved 2026-08-29; PR1–PR10 + parity audit merged 2026-08-29 (authored without a Windows machine). **W17 (2026-08-30/31)**: the settings window moved from egui to WinUI 3 via `windows-reactor` — USER decision, Codex GO WITH CHANGES; a Windows box (`ssh win`) gates it. **W17 complete 2026-08-31** (A0 #647 / A #648 / B1 #649 / B #650 / C #651): WinUI 3 is the settings window, eframe/egui is gone, and nothing of it has been SEEN yet — every pane is a dogfood item on the box. Phase status in memory.
> **Session memory**: project memory `project_windows_ime.md` (Claude auto-memory) (phase status + active pointer)
> **Sibling**: `docs/architecture/macos-roadmap.md` — the platform this one mirrors

---

## Goal

Add Windows as the fourth platform. USER 2026-08-29 (verbatim):

> 我目前還沒有windows電腦，但我想請你實作輸入法 for windows，等待我之後有的時候再來測試和build。視窗介面、設定選單、自訂詞庫、辭典管理、功能、候選窗、快捷鍵、release流程都與macOS一致。使用者在macOS和windows轉換的時候，使用體驗一致。

Two consequences shape every decision below:

1. **macOS is the behaviour oracle.** Every observable macOS behaviour — key table,
   modes, auto-space, full-width punctuation, candidate geometry, settings keys and
   defaults, storage schemas, CSV, update flow, release pipeline — is ported from the
   `file:line` specs extracted in Phase 0. The Codex pre-impl sharpened this into a
   five-way classification, used throughout: **identical semantics** · **platform-adapted
   presentation** · **unsupported host capability** · **intentionally deferred** ·
   **unverified until Windows dogfood**. Each delta is named per
   `.claude/rules/cross-platform-alignment.md` §3.
2. **PR1–PR10 were authored without a Windows machine** and verified on the macOS host
   (§ W13). Since 2026-08-30 a Windows box exists (`ssh win`, MSVC Rust 1.98, no .NET):
   W17 and everything after it is built and smoke-run there; the macOS host keeps the
   fast type-check. First-run breakage on the box is the dogfood's job — see § Dogfood
   run-book.

**Hard constraints**: release / tag / store actions stay user-gated. TL + POJ only, no
TPS (same as macOS). Shared-surface changes (engine proto, i18n, release tooling) are
allowed on their merits and listed in § Shared-surface register.

## Architecture (approved, revised per Codex 2026-08-29)

```
Host app (Notepad / Word / Chrome / …) — one process each, possibly several TSF contexts
        ▲ ITfRange::SetText / EndComposition           │ ITfKeyEventSink::OnKeyDown
┌───────┴───────────────────────────────────────────────▼───────────────────────────┐
│ TaigiKeyboard.dll  (crate taigi-windows-tsf, cdylib, loaded IN-PROC by every host) │
│  TextService : ITfTextInputProcessorEx + ITfKeyEventSink + ITfCompositionSink +    │
│    ITfDisplayAttributeProvider + ITfThreadMgrEventSink + ITfThreadFocusSink +      │
│    ITfLangBarItemButton (+ITfLangBarItem, ITfSource)                               │
│  EditSession closure wrapper · composition per CONTEXT · preserved keys            │
│  CandidateWindow (Win32 popup, Direct2D + DirectWrite, 3 layouts) + UI-less        │
│    ITfCandidateListUIElement contract · ModeFlash                                  │
├────────────────────────────────────────────────────────────────────────────────────┤
│ taigi-windows-core   (pure, host-testable, unsafe_code = forbid, NO C deps)        │
│  ComposingSessionCoordinator (per process, keyed by context token) →               │
│  ComposingManager port (3-phase apply) · ComposingKeyIntent (7-tier table) ·       │
│  candidate models (layouts / metrics<TextMeasurer> / positioning) · AutoSpace ·    │
│  FullWidthPunctuation · shortcuts model · settings MODEL + revision · NextWord      │
│  learner · engine bridge (prost envelope → dispatch::process_request) · i18n       │
│ taigi-windows-storage (rusqlite: freq v2 / assoc v6 / custom v3 · settings.json   │
│  file store · CSV)          taigi-windows-update (manifest · download · Authenticode)│
├────────────────────────────────────────────────────────────────────────────────────┤
│ engine/ crates by path (dispatch, protos, composing, lexicon, phonetics, nextword)  │
└────────────────────────────────────────────────────────────────────────────────────┘
        │ ShellExecuteW("TaigiKeyboardSettings.exe")            ▲ settings.json (revision)
        ▼                                                       │
┌────────────────────────────────────────────────────────────────┴───────────────────┐
│ TaigiKeyboardSettings.exe (crate taigi-windows-settings, WinUI 3 via windows-reactor│
│  — WinUI 3 since the W17-C cutover) · self-contained Windows App Runtime beside it  │
│  NavigationView: 一般 / 外觀 / 快捷鍵 / 自訂詞庫 / 詞庫來源 (+ unlisted 辭典搜尋)    │
│  custom-dict CRUD + CSV · shortcut recorder (WH_KEYBOARD thread hook) · update      │
│  check/download/verify/install · `--check-updates` headless (per-user scheduled task)│
│  The TSF DLL never links or loads WinUI (artifact-inspected, W17).                  │
└────────────────────────────────────────────────────────────────────────────────────┘
Runtime data: %APPDATA%\TaigiKeyboard\{settings.json, user_frequency.db,
user_association.db, custom_dictionary.db}; install dir %ProgramFiles%\TaigiKeyboard\
{TaigiKeyboard.dll (x64), x86\TaigiKeyboard.dll, arm64\TaigiKeyboard.dll,
TaigiKeyboardSettings.exe + the Windows App Runtime files (W17), Dictionaries\*, Fonts\*}.
```

## Design decisions (W1–W16, grounded in code; Codex verdict per item)

Citations refer to the Phase-0 spec extracts (macOS `file:line`) and the reference
IMEs under `references/`. "Codex:" records the ANALYSIS-ONLY verdict and what changed.

- **W1 Language + crate layout** — Rust throughout, a NEW Cargo workspace at
  `windows/` (not a member of `engine/`: that workspace forbids `unsafe_code` and pins
  Apple/Android targets; `windows/` references `../engine/*` crates by path). Zero
  FFI: the TIP calls `dispatch::process_request` in-process with a prost-encoded
  `Request` — the identical envelope contract `engine/swift-ffi/src/lib.rs:41-54` and
  `macos/.../Engine/RustEngineBridge.swift:97-142` speak. **Codex: CONFIRM WITH
  CHANGES — split the shell-independent code by dependency class**, so the host-native
  test and MSVC-check promises hold: `taigi-windows-core` (pure models, composing
  orchestration, proto bridge, candidate geometry, shortcut semantics, settings model;
  `unsafe_code = forbid`, no C deps), `taigi-windows-storage` (rusqlite stores,
  settings file store, CSV), `taigi-windows-update` (manifest, download, Authenticode),
  `taigi-windows-tsf` (cdylib `TaigiKeyboard.dll`, `unsafe` with `// SAFETY:` per
  `rust-ffi-safety.md` §3), `taigi-windows-settings` (bin `TaigiKeyboardSettings.exe`).
  Every ABI entry (COM method, exported fn) is a `catch_unwind` boundary; no `RefCell`
  borrow or COM-wrapper drop panic may cross it.
- **W2 Data locations** — `%APPDATA%\TaigiKeyboard\` holds `settings.json` and the
  three user DBs (macOS: `~/Library/Application Support/<bundle id>/`,
  `Storage/UserDataDirectory.swift:29-32`). Dictionary artefacts are copied at
  release-build time from the repo-root `dictionaries/` and fonts from
  the repo-root `fonts/font/` — the one copy every platform packages —
  into the install dir, resolved from the DLL's **own `HMODULE` saved in `DllMain`**
  (Codex: never from the current exe). **Codex: CONFIRM WITH CHANGES** — AppContainer
  (UWP) hosts cannot read `%APPDATA%`: the TIP holds an explicit per-process
  `DataCapability { settings, custom_dictionary, learning }` state, probed once and
  logged once, so it never re-opens files or re-logs per keystroke; classified
  **unsupported host capability**. **CORRECTED 2026-09-05**: that degradation was
  originally read as a reason to withhold `GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT`. It is
  the opposite — the TIP degrades *on purpose* there, so it FUNCTIONS in an immersive
  host, and the category is now declared (see W6). Atomic settings replace = write temp + `rename`
  (Rust's Windows `rename` is `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`) with a bounded
  retry, because a reader holding the file open without `FILE_SHARE_DELETE` blocks it.
- **W3 Threading, ownership, lifetime** — TSF is STA. **Codex: REFUTE AS WRITTEN — one
  process may host several thread managers / document managers / contexts (Office,
  browsers).** Ownership is therefore keyed by **context**, not process: a
  `ContextToken` (the `ITfContext` COM identity + a monotonically increasing session
  id) owns a composition; the per-process engine singleton
  (`engine/composing/src/handle.rs`) is serialised behind a mutex and is only ever
  driven by the context that currently owns it; any other context's key is a handover
  (commit the owner's composition under its own edit session if reachable, else reset +
  fresh generation). Stale focus/async messages are rejected by token + generation.
  Coding rule: **never hold a `RefCell` borrow across a COM call** — take the data,
  release, call, re-borrow and re-validate by generation (COM re-enters synchronously).
  All engine work happens inside `RequestEditSession(TF_ES_SYNC | TF_ES_READWRITE)`
  from the key sink (SampleIME; khiin `tip/edit_session.rs:11-34`): the engine
  mutation runs INSIDE the session closure, so a failed session (`TF_E_SYNCHRONOUS`,
  read-only context, teardown — all normal paths, checked on both the call `HRESULT`
  and `phrSession`) leaves engine + document untouched and the key is returned to the
  host unconsumed. Never `RequestEditSession` from a WndProc / timer / focus callback
  (rakukan `docs/DESIGN.md:745-747`); the ONE other entry is the UI-less
  `ITfCandidateListUIElementBehavior::Finalize` / `Abort` — a host-initiated synchronous
  call on the TIP thread outside any session of ours, which runs the key path with the
  commit / cancel intent (khiin `candidate_list_ui.rs:346-356`) and is refused when the
  engine is busy (the host re-entered us). Focus / context callbacks (`OnSetFocus`,
  `OnPushContext`, `OnPopContext`, `OnKillThreadFocus`) take the window down through a
  POSTED message to the popup (PR6 Codex), never synchronously; the window itself is
  shown / hidden only AFTER the edit session returned and the engine lock dropped, so
  the host's `BeginUIElement` / `SetWindowPos` re-entry never meets a held lock. DB writes go to a per-process bounded writer
  thread; the frequency snapshot is read on the key thread with `busy_timeout = 0`
  (busy → no boost this keystroke; macOS reads sync too, `ComposingManager.swift:292-298`).
  Engine + dictionaries initialise lazily on the first key the TIP handles, never in
  `Activate` (rakukan `factory.rs:413-419`). `DllCanUnloadNow` returns `S_FALSE` forever
  (rakukan `lib.rs:157-169`: `RegisterClassW` keeps the wnd_proc pointer past
  `FreeLibrary`); `DllMain` only stores the HINSTANCE. Release profile keeps
  `panic = "unwind"`. `ITfThreadMgrEventSink::OnSetFocus` only queues via `PostMessage`
  (rakukan `factory.rs:1304-1330`); `ITfThreadFocusSink` covers hosts that never raise
  it. Terminals call `OnKeyDown` without `OnTestKeyDown` (rakukan `factory.rs:399-422`)
  → both run the same classifier. Password / read-only contexts (`GUID_PROP_INPUTSCOPE`
  `IS_PASSWORD`, `TS_STATUS_READONLY`) disable composing and learning (Codex F14).
- **W4 Candidate window** — Win32 popup (`WS_POPUP`; `WS_EX_TOPMOST | WS_EX_TOOLWINDOW
  | WS_EX_NOACTIVATE`; `SW_SHOWNA` + `SWP_NOACTIVATE`; khiin
  `candidate_window.rs:62-64,152-169`), Direct2D + DirectWrite renderer (khiin
  `render_factory.rs`; Codex F3 CONFIRM — device-loss recovery + DWrite font fallback
  are explicit paths; the renderer owns NO composition state), rounded corners via
  DWM, DPI computed from the window's monitor inside a **thread** DPI-awareness scope
  (Codex: an in-proc DLL must not change the host process's DPI context), light/dark
  from the appearance setting or the system theme (read once, re-read on
  `WM_SETTINGCHANGE` / `WM_THEMECHANGED` / `WM_DWMCOLORIZATIONCOLORCHANGED`, never per
  keystroke). Every monitor query runs inside that same DPI scope (`GetDpiForMonitor`
  answers per the calling thread's awareness), `WM_DPICHANGED` re-anchors to the caret on
  the new monitor (the OS's suggested frame only when the caret is on no monitor), and a
  caret rect from a host that is not per-monitor aware is mapped through
  `LogicalToPhysicalPointForPerMonitorDPI` before use. The three layouts, metrics, paging,
  grid, positioning and index-label rules are the macOS pure models ported verbatim
  (`Candidates/HorizontalPageLayout.swift`, `ExpandedGridLayout.swift`,
  `CandidateMetrics.swift`, `CandidatePanelPositioning.swift`) with their test
  oracles as Rust tests. Sequoia geometry (6 pt corners, full-cell highlight). Caret
  rect from `ITfContextView::GetTextExt` on the collapsed selection (rakukan
  `on_compose.rs:29-43`), treating clipped / empty / failed rects as "no anchor" —
  composition end → composition start → selection (khiin `composition_utils.rs:31-69`),
  then NO window (the Mac's `presentCandidates` rule; a host-window-corner fallback was
  refused by the PR6 Codex review). Mouse click selects, never commits
  (`CandidateItemView.swift:47-48`, identical semantics), so no edit session is needed
  from the window. **Codex: UI-less mode is a v1 architecture item, not a dogfood
  note** — the TIP registers `GUID_TFCAT_TIPCAP_UIELEMENTENABLED`, implements
  `ITfUIElement` + `ITfCandidateListUIElement` over the same list model, and calls
  `ITfUIElementMgr::BeginUIElement`; when the host answers "do not show", the popup
  stays hidden and the host renders the list itself (khiin `tip/candidate_list_ui.rs`);
  `ITfUIElement::Show` from the host hides / re-shows the popup with the list kept, and
  the list the host sees is the same `MAX_DISPLAY_CANDIDATES`-capped one the layouts hold.
- **W5 Keys** — `KeyEventSnapshot{characters, charactersIgnoringModifiers, vk, shift,
  ctrl, alt, win, navigationKey, isNamedSpecialKey}`; modifiers sampled ONCE at entry
  (Codex: `GetKeyState` drifts under autorepeat/re-entrancy); characters via
  `ToUnicodeEx` with flag `0x4` ("do not change keyboard state", Windows 10 1607+) so
  dead keys are not consumed, negative returns / surrogate pairs handled, on the
  focused thread's `HKL`; `charactersIgnoringModifiers` from a state copy with the
  Ctrl bits cleared (khiin `key_event.rs:39-74`; macOS needed the same field,
  `ComposingKeyIntent.swift:41-46`) — AltGr chords (Ctrl+Alt) are left untouched so
  international layouts keep their glyphs. The 7-tier intent table
  (`ComposingKeyIntent.swift:196-292`) ports verbatim (identical semantics). Shortcut
  defaults:
  `openLastSettingsPane` Ctrl+Alt+S, `toggleRomanization` Ctrl+Alt+C — the Mac's ⌃⌘ roster
  under this platform's ⌘→Ctrl / ⌃→Alt mapping, so the letters and the family both carry over
  (USER 2026-08-31). Ctrl+Shift is not free with those letters (另存新檔 / DevTools picker) and
  no other modifier pair is either; Ctrl+Alt is allowed on the GLOBAL tier only — its chords are
  preserved keys, live only while this TIP is, so an AltGr layout is never the one in force. The
  composing tier still refuses it,
  `toggleTranslateSwapped` bare `` ` `` (consumed in the key sink only while the TIP is
  active and the classifier says so — a bare key is not a preserved key). Slot-key sets
  identical (`ComposingKeyBindings.swift:15-36`): bare `q w d f z x v y ;` default,
  Shift / Ctrl / Alt + 1–9; the stored value stays `option` (portable settings shape)
  and is rendered as Alt. Chords are registered with `ITfKeystrokeMgr::PreserveKey`
  from the stored settings and re-registered on reload.
- **W5b 中/英 Shift tap (2026-09-04)** — Windows-only, because macOS reaches English through
  the system's Caps Lock input-source switch (#617) and has no mode of its own. Tap Shift with
  no other key in between and under 500 ms and the service switches between Taigi and English;
  in English every key goes to the document, including the ones this TIP consumes outside a
  composition (the auto-space swap, full-width punctuation, the bare 漢羅 key), while the global
  Ctrl+Alt chords keep working. Ported from 新酷音 (`references/PIME/python/input_methods/
  chewing/chewing_ime.py:701-737`, default on at `chewing_config.py:70`); recognition is pure
  (`taigi-windows-core` `keys/shift_tap.rs`), auto-repeat is rejected by `lParam` bit 30, and the
  clock is `GetTickCount64` — NOT `GetMessageTime`, whose value is the last message this thread
  pulled off its queue and need not be the key a COM sink was handed (Codex F3). `OnTestKeyUp`
  answers TRUE for the eligible release, which is what asks TSF for the delivery; `OnKeyUp` runs
  the switch and answers FALSE, so the host still sees the release and its own Shift state stays
  honest. Switching commits whatever is half-typed, spends the auto-space arm and starts a new
  next-word session (Codex F8: an arm left standing would swap a space typed in the other mode),
  then publishes the mode three ways — the conversion-mode compartment (W6), the tray letter
  台/英, and the mode flash. Mode is per activation (per application), never persisted. **No
  setting**: `shiftTogglesEnglishEnabled` was retired 2026-09-05 (USER) and the tap is
  unconditional — the 一般 pane is a 1:1 mirror of the Mac's, which has no such row, and the tray
  letter plus the mode flash already say which mode is on. The spelling stays reserved; a stored
  `false` is inert in existing `settings.json` files (Windows has no retired-key sweep), so a
  future configurable feature needs a NEW key. ⚠ DOGFOOD ORACLE OPEN:
  mid-composition the switch commits; 微軟注音 may cancel an incomplete one instead, and nothing
  in the references settles it (Codex F6).
- **W6 Lang bar / menu** — one `ITfLangBarItemButton` (`GUID_LBI_INPUTMODE`,
  `TF_LBI_STYLE_BTN_MENU | TF_LBI_STYLE_SHOWNINTRAY`; rakukan `language_bar.rs:21-23`)
  whose `InitMenu` mirrors the macOS input-source menu exactly: 設定 / separator /
  檢查更新 (`TaigiInputController.swift:264-329`). **Codex: REFUTE the category list**
  — declared categories are ONLY what is true: `GUID_TFCAT_TIP_KEYBOARD`,
  `GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER`, `GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT` (rakukan
  `registration.rs:118`), `GUID_TFCAT_TIPCAP_UIELEMENTENABLED` (W4). NOT `COMLESS`
  (means COM-less activation support, which this TIP does not implement).
  **`IMMERSIVESUPPORT` was refused here and is declared since 2026-09-05**: withholding it
  is not a statement about AppContainer data access, it is what keeps the TIP out of the
  modern text-input path altogether — Microsoft's IME requirements call the category the way
  "IMEs declare that they are compatible". Real symptom that settled it: in our own WinUI 3
  settings window the user could type English and 微軟注音 but never Taigi, and no candidate
  window ever appeared. Classic Win32 controls were unaffected, which is why it went unseen
  until the WinUI settings window shipped (W17). **`INPUTMODECOMPARTMENT`
  became true 2026-09-04** and is now declared: the Shift tap gave this TIP a 中/英 mode, so it
  keeps `GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION` current (`conversion_mode.rs`, only the
  `TF_CONVERSIONMODE_NATIVE` bit; other flags are read back and preserved). It was refused while
  there was no mode to publish — the category says what is TRUE, and the answer changed with the
  feature, not with the policy. macOS still has no ABC mode of its own; there the OS switches
  input sources (`TaigiInputController.swift:132-138`).
- **W7 Registration** — LANGID `0x0404` only (Codex F5 CONFIRM: no duplicate profile
  under en-US; discoverability is the installer/onboarding's job). `ITfInputProcessorProfiles::Register`
  first, then `ITfInputProcessorProfileMgr::RegisterProfile` (rakukan
  `registration.rs:80-115`), icon = DLL resource. Unregistration is **item-by-item
  symmetric** (CLSID, profile, each category, display-attribute provider) — Microsoft
  requires individual category removal. Fresh GUIDs allocated in Phase 0. rakukan's
  `HKLM → HKCU CTF\TIP` copy for Windows 11 Settings visibility is NOT adopted (its
  uninstall deletes other IMEs' keys) — dogfood item. **Localized name (2026-08-29,
  parity with macOS #613)**: the profile description is the indirect string
  `@<dll>,-100`, resolved by Windows against the SYSTEM UI language from the DLL's
  STRINGTABLE (`build-support/resource.rs`, one `LANGUAGE` block per entry of
  `product_name_strings.rs`, which `make i18n` writes from the same key as the Mac's
  `InfoPlist.strings`: en / ja / zh-Hant); the tray button and its tooltip `LoadString`
  the same id at runtime. Other UI languages fall to en-US through Windows' resource
  search order — the Mac's untranslated `CFBundleName` fallback. **System display
  language** reads `GetUserPreferredUILanguages` (the UI language, `Locale.preferredLanguages`
  on the Mac), not `GetUserDefaultLocaleName` (the regional format) — an English-UI
  machine in a Taiwan region draws English, as macOS does.
- **W8 Installer + release** — Inno Setup 6 (`windows/installer/TaigiKeyboard.iss`;
  rakukan `rakukan_installer.iss`; Codex F6 CONFIRM): admin, `{autopf}\TaigiKeyboard`,
  explicit 32/64-bit registry views and the matching `regsvr32` binary per architecture
  (PIME `installer.nsi:299-303,633-650`), uninstall reverses, settings exe stopped
  first, "switch input method → sign out → sign in" guidance when a host still holds
  the DLL (rakukan `iss:317-335`). **Architectures (Codex): x64 is v1; x86 (WOW64
  hosts) ships once x64 registration/uninstall is proven on a real box; ARM64 is a
  built target but not published without a native smoke test.** Release =
  `windows/scripts/release-app.sh` (bash via Git Bash, mirroring
  `macos/scripts/release-app.sh`): clean-tree preflight → cargo release builds → `iscc`
  → optional `signtool` (env-gated) → SHA-256 → `windows/scripts/publish-release.sh`
  (mirror of the macOS publisher: the shared `desktop-<version>` GitHub release
  in this repo, anonymous read-back checks, then the ONE committed file
  `_data/windows_release.json` — the site renders `appcast/windows.json` from it —
  poll live). Root `make windows-release`,
  `make windows-check`. Version source of truth = `windows/Cargo.toml`
  `[workspace.package] version`; `tools/release_notes.py` `set-versions` /
  `check-versions` extended so `make version-desktop x.y.z` moves macOS + Windows together
  (the desktop train; iOS + Android are the separately numbered mobile train — USER 2026-08-29).
- **W9 Update check** — manifest `https://taigikeyboard.tw/appcast/windows.json`, the
  macOS wire schema unchanged (`macos/updates/README.md` § Wire format). **Codex: REFUTE
  the TIP-spawns-updater trigger** (host policy, security products, process ancestry).
  Adopted order: (1) the installer registers a **per-user scheduled task** (logon +
  delay, then daily) running `TaigiKeyboardSettings.exe --check-updates` headless;
  (2) the settings exe runs an overdue check on launch; (3) manual check from the
  一般 pane and the lang-bar menu; (4) the TIP only READS `updatePendingManifest`. Result
  surfaced in the 一般 pane (macOS `GeneralSettingsView.swift:89-117`) and as a Windows
  toast (unpackaged desktop toast: Start-menu shortcut carrying the AUMID, created by
  the installer; no activation handler — the toast is informational). Two-stage
  in-app install as on macOS (`UpdateInstallation.swift`): download + verify never
  launches anything; the second press opens the installer. Verification =
  `WinVerifyTrust` Authenticode + the signer certificate's **thumbprint/subject pinned
  against the running exe's own signature** (Codex: a bare subject-string compare is
  too weak; an unsigned running copy → download-page path only, as macOS ad-hoc builds).
  ⚠ **SUPERSEDED 2026-09-04** for the unsigned era: verification is `verify::admit` —
  the manifest's `packageSHA256` always, the Authenticode checks only when the running
  copy has an identity — so an unsigned copy DOES install in-app
  (`docs/architecture/windows-release.md` § Signing status).
  **PR9 decisions (Codex post-impl)**: the pin is the signer LEAF's thumbprint — stronger
  than a subject compare, with a known break: a renewed certificate is not accepted by
  the copies signed with the old one, so the first release under a new certificate
  ships through the download page and in-app updates resume from it (Windows has no
  team-id equivalent to pin). The settings window is single-instance (a named mutex),
  so one process owns the download stage; the announcement is CLAIMED inside the
  locked `settings.json` write before the toast is posted (the scheduled task and an
  open window cannot both toast); the scheduled task forbids parallel instances
  (PR10). Staging = `%LOCALAPPDATA%\TaigiKeyboard\Updates\<uuid>\<version>.exe`;
  no `%LOCALAPPDATA%` ⇒ no in-app install (never a roaming stage); stale folders are
  removed at the next launch, which PR10's installer must not trigger while it still
  reads its own payload.
- **W10 Settings reload** — `settings.json` carries a monotonically increasing
  `revision`; the TIP uses mtime/size as the cheap change detector on `OnSetFocus` and
  on the first key of each composition, reloads, and only then adopts the new
  revision; a parse failure keeps last-known-good. Which settings may change
  mid-composition follows the macOS live-read invariant §11 (`SettingsStore.swift:18-33`)
  item by item (romanization dismisses candidates, script swap re-renders, etc.).
- **W11 i18n** — `"windows"` joins `VALID_PLATFORMS`; a Rust emitter writes
  `windows/crates/taigi-windows-core/src/strings/generated.rs` (`StringKey` enum +
  `resolve(key, lang)` with Hanji → raw-value fallback mirroring
  `StringResolver.swift:25-46`, format keys as functions). **Codex F10 + Core
  Principle #6 (direction-first): the `macos` namespace is renamed `desktop`** —
  `i18n/macos.json` → `i18n/desktop.json`, accessors `macosGeneralTab` →
  `desktopGeneralTab`, mechanical across JSON + generated Swift + macOS call sites (78
  keys), verified by `swift build` on this Mac. Every desktop key is scoped to both
  platforms. `tools/i18n/check.py` gates the generated Rust byte-for-byte
  automatically; `windows/Makefile` runs it before build.
- **W12 Proto** — `PLATFORM_WINDOWS = 4` in `engine/protos/proto/envelope.proto`
  (nextword rejects `PLATFORM_UNSPECIFIED`, `engine/nextword/src/decide.rs:50-51`);
  `make build` regenerates the committed iOS/Android/macOS protos
  (`rust-migration-policy.md` §4 triple-touch). Its own PR (Codex: keep the mechanical
  regen apart from the i18n migration).
- **W13 Verification without a Windows machine** — per PR, `make windows-check`:
  `cargo test` for core / storage / update (host-native), `cargo clippy --workspace
  --all-targets --target x86_64-pc-windows-gnu -- -D warnings` (full graph incl.
  rusqlite via mingw), `cargo check --target x86_64-pc-windows-msvc` for the crates
  with no C dependency (core, update, tsf-without-storage features are NOT separable —
  so msvc covers core + update; Codex: state this honestly), `cargo fmt --check`, and a
  **GNU release link of the DLL followed by an export-table check**
  (`x86_64-w64-mingw32-objdump -p` must list `DllGetClassObject`, `DllCanUnloadNow`,
  `DllRegisterServer`, `DllUnregisterServer` unmangled). Spikes proved on 2026-08-29
  that `windows` 0.62 `#[implement]` COM, rusqlite bundled and eframe 0.31 all check on
  this Mac. **Codex strongly recommends a GitHub Actions Windows runner** (MSVC build,
  `regsvr32` in an isolated runner, settings-exe process smoke, Inno compile) as a v1
  gate. The repo removed CI deliberately (PR #274, USER) → recorded as a **user-gated
  open item**, not adopted here. What `cargo check` cannot catch is listed in
  `.claude/rules/windows-guidelines.md` § TSF / COM discipline.
  **W17 amendment (2026-08-30)**: `windows-reactor-setup`'s build script refuses the
  gnu target (`unsupported target environment: gnu`, spike), while `windows-reactor`
  itself type-checks on it. So the settings crate's build script stages the runtime
  only for MSVC and the gnu `cargo check` stays a type-check; the REAL gate for the
  settings crate is `make check-box` = `ssh win cargo clippy -p taigi-windows-settings
  --all-targets -- -D warnings` on the MSVC box, folded into `make windows-check`
  (Codex Q4: an unreachable box is a gate FAILURE, never a silent skip; the daily gate
  now depends on that box — the only clean alternative is Windows CI, user-gated).
- **W14 Toolchain** — `windows/rust-toolchain.toml`: stable + `x86_64-pc-windows-msvc`
  (v1 ship), `i686-pc-windows-msvc`, `aarch64-pc-windows-msvc` (prepared — listed, not
  built by the release script) + `x86_64-pc-windows-gnu` (host check). `+crt-static`
  for the MSVC DLL and exe, verified per artifact by the release script's import-table
  check (`dumpbin /dependents`: no `vcruntime*.dll` / `msvcp*.dll` imports).
  **W17 amendment**: `windows-reactor` needs rust-version 1.95 (box has 1.98, edition
  2024). The no-VC-runtime gate covers OUR two binaries only; the Windows App Runtime
  DLLs staged beside the exe are Microsoft's (they import the system UCRT
  `api-ms-win-crt-*`, no `vcruntime140` — string-inspected on the box) and are never
  judged by it.
- **W15 UI framework for settings — SUPERSEDED by W17 (2026-08-30)**. History: PR7/PR8/#641/#645
  shipped an eframe/egui `=0.31.1` window (system faces prepended, DWM accent, dark
  caption, then a Fluent-token restyle in #645). USER 2026-08-30 after seeing it run:
  「egui很醜,放棄egui方案 … win設定選單改為windows原生UI元件, modern win style」. The egui
  code stays on `main` until the W17 cutover PR deletes it; its named limitations (no
  Mica, no high contrast, egui IME path, accesskit tree) die with it.
- **W17 Settings window on WinUI 3 via `windows-reactor`** (Codex ANALYSIS-ONLY
  2026-08-30: GO WITH CHANGES, all folded in). Framework = `windows-reactor` 0.100
  (microsoft/windows-rs `crates/libs/reactor`, pure-Rust declarative WinUI 3:
  `Component { create / update / view }`, typed messages, `spawn_background`, generated
  builders for 86 controls, `WindowVisuals { theme, backdrop: Mica, client_size }`,
  `ThemeBrush::{CardBackground, CardStroke, Accent, …}`). Chosen over C# WinUI 3 (a
  fourth language, .NET on the box, SQLite/engine over FFI/IPC) and raw WinUI through
  `windows-bindgen` (XAML bootstrap, event tokens and tree diff by hand): zero FFI —
  `taigi-windows-core/-storage/-update/-platform` are called directly. **Risk
  register**: git dependency pinned to the spike-proven commit
  `dc720b3674c46ceb82d758ed20959977b32e60a9` (crates.io holds only `0.0.0`
  placeholders); rust-version 1.95; API 3 months old — a bump is its own round on the
  box. Flip-to-C# triggers (Codex Q1): reconciliation/state loss that cannot be
  worked around, unreliable native `TextBox` IME / UIA / `ContentDialog` focus /
  `NavigationView` lifetime on the box, upstream churn forcing a fork. A missing
  property on one control is NOT a trigger. **Spike 2026-08-30 on the box**: both
  reactor samples build in 1m57s and run; self-contained runtime = 65 MB / 42 files +
  3 MB exe.
  **Deployment = self-contained** (Codex Q2): `windows_reactor_setup::as_self_contained()`
  in the settings crate's `build.rs` (MSVC only; skipped on the gnu host check) stages
  the `Microsoft.WindowsAppSDK.Runtime` 2.4.0 files beside the exe and embeds the
  activatable-class app manifest via `/MANIFEST:EMBED` (coexists with the rc-embedded
  icon + VERSIONINFO — verified in PR A0). `release-app.sh` copies the runtime files to
  staging; `.iss` installs them into `{app}` beside the exe; uninstall removes them
  (user data untouched). No Windows App Runtime install, no MSIX provisioning, offline
  install, deterministic version; the installer grows ~25–35 MB compressed.
  Framework-dependent (116 MB `WindowsAppRuntimeInstall.exe` chained in Inno) rejected.
  **The TSF DLL inherits nothing**: `dumpbin /imports TaigiKeyboard.dll` must not name
  `Microsoft.UI.Xaml*` / `Microsoft.WindowsAppRuntime*`; Reactor lives only in the
  settings binary's graph (release-script gate). `Fonts\` stays — the candidate window
  needs it.
  **What WinUI gives natively** (all of §W15's hand-rolled parts): Mica, system
  light/dark + the app's own 淺色/深色 (`WindowTheme`), accent, high contrast, Segoe UI
  Variable + Segoe Fluent Icons, hanji via the system font fallback (no bundled UI
  face; `fonts.rs` / `theme.rs` deleted), UIA/Narrator, keyboard navigation, and a
  `TextBox` that is a real TSF host — the 自訂詞庫 fields take 台語 from our own TIP.
  **Shape**: one `SettingsWindow` component (NavigationView Left, pane toggle / back /
  settings hidden, `open_pane_length` 215, `FontIcon` glyphs U+E713/E790/E765/E82D/E8F1)
  + one page per pane; SettingsCard = `Border(CardBackground, CardStroke, 1px, 4px,
  padding 16×12)` over `Grid[★, auto]`; 教典 subcollections in an `Expander`; the
  destructive / reset actions are real `Button`s carrying the card (Codex Q6: keyboard
  activation + UIA role); alerts = `ContentDialog`; banners = `InfoBar`; busy =
  `ProgressRing`. Window: `client_size(760, 560)` + min constraints; frame position is
  NOT persisted (`settings-window.ron` retired) — named divergence from the Mac's
  autosaved frame.
  **Codex Q6's spike is answered by the API, not by the box (W17-A)**: `ResourceValue`
  carries only `Color` / `CornerRadius` / `Thickness` — there is no theme-brush resource
  value, so any `ButtonBackground` override would freeze a literal colour against
  light / dark / high contrast. The action card is therefore a full-width DEFAULT-style
  `Button` wearing its own native actionable-card chrome, with only the theme-independent
  geometry overridden (`ControlCornerRadius` 4, `ButtonPadding` 16×12) and the label in
  `ThemeBrush::Accent` (`SystemCritical` when destructive). Not "looks like a card" —
  native actionable-card chrome, to be judged on the box in normal / hover / pressed /
  disabled / high contrast.
  **Two more named divergences from the Mac, decided in W17-A**: (a) the 外觀 mode is a
  native pop-up, not the Mac's three drawn light / dark / auto thumbnails — the setting's
  semantics (three options, their order, the stored value, live apply, `Auto` following
  the system) are what parity owns, and Windows 11 Settings itself uses a pop-up for
  "Choose your mode"; (b) the window's width floor is 600, not the Mac's fixed 760, so
  `NavigationView` can actually reach the ~641px threshold its adaptive pane behaviour —
  and the acceptance line below — exists for.
  **Timers** (Codex Q5): live reload (W10) = a re-armed single-flight
  `spawn_background(sleep 1 s → Tick{generation})`; the closure checks cancellation after
  the sleep; every message that can arrive late (tick, filter debounce, spinner delay,
  recorder poll) carries a generation and is a no-op when stale; OS resources (the
  hook) are freed by RAII/component drop, never by a message.
  **Shortcut recorder** — Reactor exposes NO keyboard events (only static
  `KeyAccelerator`s; `TextBox::on_text_changed` cannot see Tab/Escape/modifiers) →
  while a row records, a THREAD-scoped `SetWindowsHookExW(WH_KEYBOARD, …,
  GetCurrentThreadId())` in `taigi-windows-platform` (Codex Q3, all conditions
  adopted): installed and removed on the XAML thread; static `extern "system"`
  callback under `catch_unwind`, `nCode < 0` → `CallNextHookEx`, pass-through when not
  recording / untranslatable / channel gone; swallows BOTH key-down and key-up of the
  keys it takes (return 1) so XAML never sees a lone key-up; bounded work only
  (`try_send` into an `mpsc`); `HHOOK` in an RAII guard released on recorded / Escape /
  Tab / explicit `StopRecording` from every interaction that ends it (Codex: do not
  rely on root `Border::on_pointer_pressed` bubbling — children that handle the press
  swallow it) / window focus loss (the egui behaviour, kept) / pane change / window
  close / drop; a recording generation discards stale presses. Bare-press names come
  from `ToUnicodeEx` with the W5 rules (HKL, dead keys, surrogates, AltGr) — shared with
  the TSF crate's `key_translation`, not a weaker copy; chords use the VK table. The
  decision stays `keys::evaluate_press` (core, untouched). The old egui limitations
  (keypad `+`, Win key, AltGr) are re-classified against the hook in PR B1.
  **W17-B1 contracts** (Codex ANALYSIS-ONLY 2026-08-31: GO WITH CHANGES on all seven
  forks; four of them were behaviour bugs in the plan, all folded in):
  (a) **Tab is reported but NOT swallowed** — `evaluate_press` answers a bare Tab with
  `PassThrough`, meaning recording ends AND the key walks the form, which it cannot do
  if the hook ate it. It is never remembered as swallowed, so its key-up goes to the
  window too. **W17-B**: Reactor exposes no activation event at this pin, so a row the
  user walked away from is released by the window's own beat instead — the tick asks
  `platform::is_foreground_thread()` while (and only while) a row records. Up to a
  second later than the egui field's `WindowFocused(false)`, and no key can be recorded
  in between: the hook is thread-scoped, so while another app has focus none reaches it.
  (b) **The hook drains, it does not just unhook**: dropping the guard stops
  the reporting, but the hook stays installed until every key it swallowed has been
  released — WinUI invokes a focused `Button` on the key-UP of Space, so a lone up is a
  press the user did not make. A later `install()` on the same thread takes over a
  still-draining hook (the user has since clicked another row with the mouse; nothing is
  held). (c) **A full queue still swallows** — a key lost beats a key typed into the
  form — while a gone one passes through; the two are distinct answers, not one bool.
  (d) **Delivery is a closure, not a channel the UI polls**: a press queued for the next
  1 s / 100 ms tick would be a visibly late keystroke, so the hook calls a bounded
  `Fn(RecordedPress) -> Delivery` that hands the press to Reactor's own local queue and
  wakes it. The thread-local state is read with `try_borrow_mut` (a re-entrant callback
  passes the key through rather than panicking) and its borrow is never held across the
  delivery closure or across Win32; the recording generation lives in the message that
  closure sends, not in the platform. **OPEN, and the first dogfood question of W17-B**:
  whether `WH_KEYBOARD` sees the keys of a WinUI 3 XAML island at all, or whether input
  arrives through the CoreMessaging dispatcher queue — if it does not, the fallback is
  `WH_GETMESSAGE` or the App SDK's `InputKeyboardSource` / `PreTranslateKeyboardSource`.
  Nothing else in W17-B depends on which.
  **Behaviour freeze** (Codex Q7): every `settings.json` write, store call, the
  one-work-slot rule, CSV codec, the `Offer` machine, `--check-updates`, the launcher
  contract stay byte-for-byte. Named observable changes: window frame not persisted;
  label click no longer toggles a switch (reset/destructive cards DO fire on the whole
  card, being buttons); UI hanji = system fallback face; native `TextBox` IME / caret /
  undo / clipboard; native focus order, focus visuals, Narrator; high contrast now
  supported; `ContentDialog` Escape/Enter/focus-trap semantics; `ListView` selection +
  keyboard semantics; `rfd` owner HWND re-verified; the installed exe (including the
  headless `--check-updates` run) now needs the runtime files beside it; second launch
  with a different `--pane` must still switch + foreground the existing window
  (single-instance mutex kept, activation re-verified).
  **Gate**: § W13 amendment (`make check-box`). Each PR: builds on the box, launches
  headless without crashing (ssh session 0 cannot show a window), USER screenshots on
  the box's console.
- **W16 Docs** — roadmap = decisions + PR DAG + acceptance; memory = round hand-off;
  `docs/architecture/windows-release.md` = operator procedure. **PR10 contract for PR9's
  code** (Codex): sign `TaigiKeyboardSettings.exe` and every installer with the SAME
  Authenticode leaf; VERSIONINFO on both with an identical non-empty `ProductName` and
  the installer's `ProductVersion` = the workspace version; `packageURL` names the
  final `.exe` (⚠ 2026-09-04: releases ship UNSIGNED until a certificate exists, so the
  signing half of this contract is dormant and a package is admitted by the manifest's
  `packageSHA256` instead — `docs/architecture/windows-release.md` § Signing status);
  a Start-menu shortcut to the settings exe with
  `System.AppUserModel.ID = TaigiKeyboard.Settings`; a per-user scheduled task running
  the installed exe with exactly `--check-updates`, `MultipleInstances=IgnoreNew`;
  `Dictionaries\` and `Fonts\` beside the exe (never the working directory) — and,
  from W17, the Windows App Runtime files beside the exe (the headless updater depends
  on the complete install directory); no
  settings-exe launch from the installer before its payload is fully consumed; a
  certificate-rotation release goes through the download page once;
  `.claude/rules/windows-guidelines.md` = durable constraints; `windows/updates/README.md`
  = manifest contract only. No duplicated truth between them.

## Windows-specific acceptance matrix (Codex W15/F14)

Not "chrome differences" — each needs its own acceptance line in the run-book:
UI-less candidate hosts (games, some Store apps) · password / secure fields (no
composition, no learning) · AltGr layouts · high contrast · screen reader (candidate
window exposes text via UIA — deferred, named) · touch keyboard · remote desktop ·
Chrome child-HWND focus · Notepad composition length cap · Windows Terminal.
W17 adds (settings window): native `TextBox` 台語 IME · Narrator/UIA + Tab order ·
`ContentDialog` focus trap + Escape/Enter · `NavigationView` adaptive width (pane
collapse below its threshold) · Mica on Windows 10 (falls back) · runtime files
missing → a readable failure, not a silent exit · clean/offline install · hook
released on every exit path, keys swallowed only while recording · live system
theme / high-contrast change · `--check-updates` from the non-interactive scheduled
task session · uninstall removes the runtime files, keeps `%APPDATA%`.

## Phase / PR table

USER sizing preference (2026-08-17): ~600–1000 LOC per PR, fewer and larger. Every
coding PR runs the Codex sandwich (pre-impl on that PR's design fork, post-impl on the
diff) and the W13 gates. Order revised per Codex F12.

| PR | Phase | Scope | Status |
|---|---|---|---|
| PR0 | Admin | this roadmap + memory topic + `.claude/rules/windows-guidelines.md` + docs index | direct-to-main |
| PR1a | Proto | `PLATFORM_WINDOWS` + `make build` regen (mechanical) | **Merged** #623 (`b7090482`) |
| PR1b | i18n + tooling | `macos` → `desktop` namespace rename; `windows` platform + Rust emitter; `release_notes.py` Windows version writer/check; root Makefile `windows-check` / `windows-release` | **Merged** #624 (`d1fc40a5`) |
| PR2 | Scaffold + core composing | `windows/` workspace + toolchain; `taigi-windows-core`: settings model + revision, engine bridge (envelope, AppConfig, generation, lexicon install, logger), `ComposingSessionCoordinator` keyed by context token, ComposingManager port (3-phase apply, effects, fetch protocol, commit outcomes), `ComposingKeyIntent` 7-tier table + `KeyEventSnapshot`; engine round-trip tests against `ios/Resources/Dictionaries`. **Locks**: word identity `(漢字, canonical TL)`, context ownership + handover, engine generation rules, effect ordering + failure semantics, settings revision | **Merged** #625 (`e4367ef6`) + #626 (`e557297a`) |
| PR3 | Core candidates | `CandidateMetrics<TextMeasurer>`, `HorizontalPageLayout`, `VerticalLayout`, `ExpandedGridLayout`, positioning, index labels, cell content, document text; macOS oracle numbers as tests | **Merged** #627 (`94b702cb`) |
| PR4 | Storage + policies | `taigi-windows-storage`: rusqlite stores (freq v2 / assoc v6 / custom v3, byte-identical SQL, WAL + bounded busy handling, migration under `BEGIN IMMEDIATE`), `LearningCapacity`, CSV codec, seeds, settings file store (atomic replace); core: `AutoSpacePolicy` + attaching set, `FullWidthPunctuation`, `NextWordLearner`, shortcuts model (chords, registries, conflicts, recorder gate) | **Merged** #628 (`ff52af09`) |
| PR5a | TSF lifecycle | COM exports + class factory + symmetric registration + GUIDs; `TextService` activate/deactivate; thread-mgr / thread-focus sinks; context identity; lang-bar button + menu; settings reload; spawn settings exe; **smoke TIP that composes nothing** | **Merged** #629 (`b9361e72`) |
| PR5b | TSF composing | key sink → snapshot → intent → manager inside sync edit sessions; composition start/update/commit per context; display attribute; preserved keys; password/read-only gating; handover | **Merged** #630 (`78514a8e`) |
| PR6 | TSF UI | candidate window (D2D/DWrite renderer, 3 layouts, DPI scope, theme, mouse, private fonts, caret positioning + fallbacks, device loss) + UI-less `ITfCandidateListUIElement` contract; mode flash panel; unfold animation | **Merged** #631 (`7f481296`) |
| PR7 | Settings exe I | eframe shell + sidebar + 一般 / 外觀 / 快捷鍵 panes + shortcut recorder + display language + fonts; `taigi-windows-platform` (locale / open URL / beep, host stubs); `settings::launch` CLI contract shared with the DLL; core `keys::recorder` decision + choice `label_key`s | **Merged** #632 (`d6247c82`) |
| PR8 | Settings exe II | 自訂詞庫 (paged `egui_extras` table, CRUD sheet, CSV import/export via `rfd`, delete all, clear learning — writes on a background thread behind one work slot + 400 ms spinner) + 詞庫來源 (three sections, 教典 subcollections indented/disabled) + unlisted 辭典搜尋 (`--pane dictionarySearch`; lexicon loaded on first query) + external lookup URLs; core `engine::lexicon` search ops + `DictionarySource::from_bitmask/badge_key` + `LexiconRow::sorted_for_search`, `engine::external_lookup`, `dictionary_artifacts::dictionary_version` shared with the DLL | **Merged** #633 (`995eb5ee`) |
| PR9 | Updates | `taigi-windows-update`: `manifest` (wire format = macOS's, `DottedVersion` zero-padded), `checker` (due / stamp-before-fetch / record / announce-once, pure over `SettingsDocument`), `transport` (`ManifestFetcher` + `PackageDownloader` traits; `ureq` over schannel, 64 KiB / 200 MiB ceilings, HTTPS+200 only), `installation` (the `Offer` state machine, staging `%LOCALAPPDATA%\TaigiKeyboard\Updates\<uuid>\<version>.exe`, download + verify on a thread), `verify` (WinVerifyTrust + signer thumbprint pinned to the running exe + VERSIONINFO product/version; ⚠ since 2026-09-04 `verify::admit` also requires the manifest's `packageSHA256` and runs the Authenticode half only when the running copy is signed), `toast` (WinRT, AUMID `TaigiKeyboard.Settings`); settings exe: overdue check at launch, `--check-now` alert, `--check-updates` headless, 一般-pane pending row per offer | **Merged** #634 (`101034f5`) |
| PR10 | Installer + release | `windows/installer/TaigiKeyboard.iss` (admin, `{autopf}\TaigiKeyboard`, x64compatible, Inno 6.5+; languages = macOS bundle's zh-Hant/en/ja with Hanji first as fallback, messages generated from `desktop.installer*` into `Messages.iss` by `make i18n` — USER 2026-08-29「macos有什麼語言，windows就有什麼」; unregister + stop settings exe + rename lock-probe with the sign-out recipe before copying; regsvr32 x64 + SysWOW64 for a staged x86 DLL; AUMID Start-menu shortcut; per-user scheduled task from `update-check-task.xml` created as the original user, `IgnoreNew`; symmetric uninstall, `%APPDATA%` kept) · `build-support/resource.rs` + both crates' `build.rs` (icon id 1 + VERSIONINFO with `VFT_APP`/`VFT_DLL` via rc.exe / llvm-rc / windres; `TAIGI_REQUIRE_RESOURCES=1` = compile failure fatal, no windres on MSVC) · `resources/TaigiKeyboard.ico` (`tools/windows/make-ico.py`) · `scripts/{lib/identity.sh,release-app.sh,publish-release.sh}` (mirror of macOS; signtool by thumbprint, env-gated; VERSIONINFO read back from DLL/exe/installer via PowerShell and compared to the checkout; `dumpbin /dependents` no-VC-runtime gate; publisher pins the installer's signer to `WINDOWS_SIGNING_THUMBPRINT`; `windows-v<ver>` release on the website repo; `appcast/windows.json` + `_data/windows_release.json`; poll live) · root `make windows-release`, `windows/Makefile release` · `tools/release_notes.py` set/check-versions += `windows/Cargo.toml` · docs `windows-release.md`, `windows/updates/README.md`, README | **Merged** #635 (`05c06cd0`) — 2026-08-29 USER「merge all PR」; installer languages = macOS bundle set (hanji/en/ja) via `desktop.installer*` → `Messages.iss` |
| PR11 | Dogfood fixes | first real-Windows smoke: fixes from the run-book + memory hand-off (not admin-only). W17 adds the whole WinUI window to it — the settings exe has been built and its tests run on the box, but no pane has been drawn | Pending |
| W17-A0 | Reactor foundation | pinned `windows-reactor` git dep + MSVC-only `as_self_contained()` in `build.rs` (rc resources coexist), `make check-box` (ssh MSVC clippy) folded into `windows-check`, `release-app.sh` runtime staging + DLL no-WinUI import gate, `.iss` runtime files + uninstall, docs; egui entry point UNTOUCHED | **Merged** #647 (`fdbe647b`) |
| W17-A | Shell + 一般 + 外觀 | `SettingsWindow` component (NavigationView, Mica, theme, size, title, live-reload tick, InfoBar banner, ContentDialog alerts), SettingsCard / choice / switch / action-card widgets, 一般 + 外觀 pages, update row + outcome dialog; ALTERNATE entry `--winui` (replaces A0's `--winui-smoke`, which the real window subsumes) — egui stays production until W17-C. Shared seams so nothing is written twice: `updates::UpdateHost` (both windows drive one check / offer / announce), `presentation` (display language, pane titles + glyphs, sponsor link), `user_data::open_at_launch` (the launch store migrations both entries owe). The Reactor sidebar lists only the panes this build has pages for; a stored or `--pane` selection it has no page for opens on 一般 IN MEMORY and is never written back, so a preview launch cannot move the egui window's selection | **Merged** #648 (`cf498ea6`) |
| W17-B1 | Platform recorder hook | `taigi-windows-platform`: `WH_KEYBOARD` thread hook (RAII + drain, `catch_unwind`, down+up swallow, bounded delivery closure, host stub) + the shared `ToUnicodeEx` translation moved out of the TSF crate so the recorder reads a key exactly as the classifier does (`key_translation::recorded_press`, AppKit reserved scalars for the keys that type nothing); pure tests for the `lParam` decode, the swallow bookkeeping and the scalar table. `make check-box` widens to the platform crate — its key translation IS the Win32 keyboard API, so its tests cannot run on the macOS host | **Merged** #649 (`33c0f844`) |
| W17-B | 快捷鍵 + 詞庫來源 | recorder rows over the B1 hook (`RecorderTarget` routes BOTH registries through one `store`, so the conflict pass cannot be forgotten on one of them) + shortcuts page; 詞庫來源 with 教典 as an `Expander` header over its eleven subcollections. Reactor exposes no focus event, so every message that is not the recording itself ends it — the "clicked elsewhere" the egui field watched for — and the tick releases a row the user walked away from. The slot-key-set picker keeps its `resolve_after_slot_key_set_change` pass: `choice_row` answers with the MESSAGE the row means, not a single-key write | **Merged** #650 (`758fe4a0`) |
| W17-C | 自訂詞庫 + 辭典搜尋 + cutover | `ListView` table + header, paging, CRUD `ContentDialog`, `rfd` CSV over `platform::dialog_owner()` (Reactor hands out no HWND), busy card after 400 ms, delete-all / clear-learning; the unlisted search page; THEN egui goes — `app.rs`, `theme.rs`, `fonts.rs`, `keys.rs`, `panes/**`, `widgets/**`, and the eframe / egui / egui_extras pins. Background work moves from `work::PendingWork` to Reactor's `spawn_background` (`work.rs` survives ONLY for `UpdateState`, which is not a component and has no context). `main()` returns `ExitCode`; `--winui` goes with the window it selected. `check-exe` retires — the exe can only link on MSVC now, so `check-box` builds it | **Merged** #651 (`fdd64e34`) |

W17 merge rule (Codex Q8): A0 may merge alone (no behaviour change). A / B1 / B / C are
**stacked** — none merges to `main` on its own; `main` never carries a Reactor build
that lacks a pane. Installer + release plumbing lands in A0 so the first Reactor exe
starts on a clean install.

**W17 first dogfood, 2026-08-31 — what the blind build cost.** Two panes (詞庫來源, 自訂詞庫)
fail-fasted the process the moment they were opened: each handed a multi-root `View::fragment`
to a single-child content / slot, which the reactor answers with
`PumpError::StructureUnsupported` — an unhandled `E_FAIL` out of `OnLaunched` that XAML turns
into a crash with no message anywhere but Windows Error Reporting. Because the shortcut opens
the pane the user last had open, visiting either one made every later Ctrl+Shift+S (the settings
chord at the time) die at once,
which read as "the shortcut is broken". The tray button was the second miss: a
`TF_LBI_STYLE_BTN_MENU` item shows no menu in the Windows 8+ taskbar input indicator, which
routes clicks to `OnClick` and never drives `InitMenu`. Both are fixed, and every pane is now
mounted headlessly against the reactor's `RecordingRuntime` by `winui::pane_planning` — the
planning layer is platform-independent, so that test catches this whole class before a device
ever sees it. Rules in `.claude/rules/windows-guidelines.md` § Authored without a Windows
machine.

Dependencies: PR2 → PR3/PR4 (parallelisable) → PR5a → PR5b → PR6; PR7 → PR8; PR9 needs
PR4 + PR7; PR10 last.

## Shared-surface coordination register

| Change | PR | Additive? | ios/android/macos files touched? |
|---|---|---|---|
| `PLATFORM_WINDOWS = 4` in `envelope.proto` + regen | PR1a | yes | **yes — generated `.pb.swift` / `.java` only, semantically inert** |
| `i18n/macos.json` → `desktop.json`, key rename, `windows` scope; Rust emitter | PR1b | rename | `macos/**/*.swift` call sites + generated Swift (mechanical, `swift build` verified) |
| `tools/release_notes.py` Windows version file | PR1b | yes | no |
| Root `Makefile` `windows-*` targets | PR1b | yes | no |
| `engine/` crates consumed by path from `windows/` | PR2 | no change | no |
| Website repo: `appcast/windows.json`, `_data/windows_release.json` | PR10 | yes | separate repo (the release itself moved to `desktop-*` tags here, 2026-09-09) |

## 最佳實踐對齊 (references)

Entry point per repo policy: `docs/references/mainstream-ime-comparison.md` cards #12
(khiin-rs), #18 (rakukan), #19 (PIME), plus Microsoft's TSF SampleIME pattern and the
TSF documentation Codex cited (edit-session flags, UI-less mode, predefined categories,
registration, desktop toast requirements).

| 主流做法 | 來源 file:line | 本 plan 對應 |
|---|---|---|
| `DllCanUnloadNow` → `S_FALSE` forever (class wnd_proc outlives FreeLibrary) | rakukan `crates/rakukan-tsf/src/lib.rs:157-169` | W3 |
| Lazy engine init on first handled key, never in `Activate` | rakukan `factory.rs:413-419` | W3 |
| `OnSetFocus` re-entrancy → `PostMessage`, return | rakukan `factory.rs:1304-1330` | W3 |
| `ITfThreadFocusSink` for non-TSF-aware hosts | rakukan `factory.rs:505-512` | W3 |
| Terminals call `OnKeyDown` without `OnTestKeyDown` | rakukan `factory.rs:399-422` | W3/W5 |
| `SetSelection` before `EndComposition`; commit+restart in ONE session; take composition inside the session | rakukan `on_compose.rs:559-617, 232-236` | PR5b |
| Sync edit session from the key sink, engine mutation inside the closure | khiin `tip/edit_session.rs:11-34`; SampleIME | W3 |
| `ToUnicode` with Ctrl cleared for `charactersIgnoringModifiers` | khiin `tip/key_event.rs:39-74` | W5 |
| Display attribute: `RegisterGUID` atom + `GUID_PROP_ATTRIBUTE` `Clear` then `SetValue` | rakukan `on_compose.rs:379-396`; khiin `composition_mgr.rs:140-166` | PR5b |
| `Register` then `RegisterProfile`; SYSTRAYSUPPORT needed for the tray | rakukan `registration.rs:80-143` | W6/W7 |
| LANGID 0x0404 | khiin `reg/registrar.rs:88` | W7 |
| Popup window styles + `SW_SHOWNA` + DWM round corners + thread DPI scope around create | khiin `candidate_window.rs:62-64,152-169`, `ui/dwm.rs`, `wndproc.rs:122-143` | W4 |
| D2D/DWrite render factory, DC render target, RECREATE_TARGET handling | khiin `ui/render_factory.rs`, `candidate_window.rs:379-420` | W4 |
| `GetTextExt` on collapsed selection + three-tier fallback | rakukan `on_compose.rs:29-43`; khiin `composition_utils.rs:31-69` | W4 |
| `ITfUIElement` / `ITfCandidateListUIElement` for UI-less hosts | khiin `tip/candidate_list_ui.rs:54-59,160-174` | W4/PR6 |
| Lang-bar button `GUID_LBI_INPUTMODE`, `InitMenu` / `AddMenuItem` | rakukan `language_bar.rs:21-46`, `factory.rs:1145-1231` | W6 |
| Settings exe launched from DLL directory | rakukan `tsf/settings_launcher.rs` | W1 |
| Config reload by mtime poll | rakukan `engine/config.rs:330-443` | W10 |
| Inno Setup, `regsvr32 /s` in `[Run]`, DLL-locked rollback + sign-out guidance | rakukan `rakukan_installer.iss:120-158, 290-370` | W8 |
| `regsvr32` per architecture, `SetRegView 64` | PIME `installer/installer.nsi:299-303, 633-650` | W8 |
| `+crt-static` for the TIP DLL | khiin `windows/ime/.cargo/config.toml:1-6` | W14 |
| Candidate geometry / paging / positioning oracles | macOS `Candidates/*.swift` + tests | W4, PR3 |
| Key table, modes, auto-space, full-width, learning identity | macOS `Controller/*.swift`, `Composing/*.swift` | W5, PR2/PR4 |
| Settings keys/defaults, storage schemas, CSV, update flow | macOS `Settings/*.swift`, `Storage/*.swift` | PR2/PR4/PR7-9 |

**Deliberately not adopted**

| Practice | Source | Why not |
|---|---|---|
| Out-of-process engine host over a named pipe | khiin `engine_coordinator.rs`; rakukan `rakukan-engine-rpc`; PIME | Those engines drag llama.cpp / Python / msvcp into hosts; ours is a few MB of static Rust plus shared mmap. In-proc = no IPC, no ACL, no service lifecycle (Codex F1: conditional CONFIRM). Revisit only if dogfood shows AppContainer hosts matter (W2). |
| GDI candidate rendering | rakukan `candidate_window.rs:295-400` | No DPI, no private fonts, no dark mode there — all three are macOS parity items. |
| `panic = "abort"` in the DLL | rakukan `Cargo.toml:55-59` | Every COM entry is a `catch_unwind` boundary per `docs/engine/ffi-safety.md`; abort would take the host down. |
| TIP spawning the updater from a keystroke | (own first draft) | Codex W9: host policy / security products / process ancestry. Scheduled task instead. |
| Ctrl+Alt **composing** bindings | (own first draft) | Codex F7: AltGr conflict on non-US layouts — a composing binding stands for as long as it is bound. The GLOBAL tier does allow Ctrl+Alt (USER 2026-08-31): those chords answer only while this TIP is selected, and they are what carries the Mac's ⌃⌘ roster over. |
| `GUID_TFCAT_TIPCAP_COMLESS`, `IMMERSIVESUPPORT` | khiin/rakukan register all seven | Codex W6: declaring capabilities the TIP does not have. |
| HKLM → HKCU `CTF\TIP` registry copy | rakukan `rakukan_installer.iss:134-138` | Uninstall deletes the whole HKCU TIP key — other IMEs' settings. Dogfood decides if Windows 11 needs anything. |
| `MessageBox` in `DllRegisterServer`, `panic!` in edit sessions, advising a second sink object | khiin `dll.rs:173-180`, `edit_session.rs:32`, `key_event_sink.rs:87-88` | Live bugs, not patterns. |
| Threads spawned from `DllMain` | rakukan `lib.rs:127-130` | Loader lock. |
| ~~Shift-tap 中/英 toggle~~ — **ADOPTED 2026-09-04**, see below | Windows CJK convention | Was: macOS has neither, keep the macOS table as the default (Codex F14). USER dogfooded 微軟注音 on the box, confirmed Shift is the platform's switch, and asked for it. It shipped as a settings row (`shiftTogglesEnglishEnabled`) with a tray letter and a mode flash; the row was retired 2026-09-05 (USER: the 一般 pane mirrors the Mac's 1:1, which has no such row) and the tap is unconditional — the tray letter and the flash are what keep it from being the silent default change F14 refused. Space-to-convert stays unadopted. |
| WiX / MSI-written registry | khiin `installer/Registry.wxs` | `DllRegisterServer` is the single source of registration truth; an MSI mirror drifts. |
| Resident tray agent | rakukan `rakukan-tray` | Codex F8: a scheduled task + settings-exe overdue check covers the update trigger without a resident process. |

## Dogfood run-book (first Windows machine)

Ordered in layers — stop at the first foundation failure:

1. Toolchain: `rustup target add` the MSVC targets; `cargo build --release -p
   taigi-windows-tsf -p taigi-windows-settings`; `iscc` installed.
2. `regsvr32 TaigiKeyboard.dll` from an elevated prompt → the TIP appears under
   設定 → 時間與語言 → 語言 → 中文(台灣) → 鍵盤. If invisible on Windows 11, evaluate
   rakukan's HKLM→HKCU copy (W7 open item) before anything else. `regsvr32 /u` removes
   every key it added (W7 symmetry). **Name**: Settings and the tray show 台語齒盤 on a
   zh-TW UI, 台湾語キーボード on ja, TaigiKeyboard on en and on any other UI language —
   never the literal `@C:\...\TaigiKeyboard.dll,-100` (that would mean the shell did not
   resolve the indirect description: fall back to a plain string in `register_profile`).
   Switch the Windows display language and sign out/in to see it follow; check all three
   consumers separately (Settings profile name, tray item text, tray tooltip — different
   processes resolve them); a UI language with no block (fr-FR) must show `TaigiKeyboard`;
   an install path with spaces must still resolve; the MSVC release build (rc.exe, not
   the host's windres) must carry all three `LANGUAGE` blocks — `Get-Content` the
   installer's DLL through PowerShell's `[System.Diagnostics.FileVersionInfo]` is not
   enough, read the string table (`resource hacker` or `LoadString` from the settings exe).
3. Notepad: type `taigi` → underlined preedit, candidate window below the caret,
   `q` commits slot 0, Return commits highlighted, Space commits the alternate script,
   Esc cancels, digits are tones. Then Word / Chrome / Windows Terminal / a UWP app
   (expect W2 degradation there) / a password field (expect no composition).
4. Lang-bar button in the tray: menu 設定 opens the exe; Ctrl+Alt+S / Ctrl+Alt+C /
   `` ` `` work while the TIP is active.
5. Settings exe (W17, WinUI 3): each pane matches the macOS pane order and controls;
   typing Taiwanese INTO the custom-dict `TextBox`es with our own TIP (composition,
   commit, backspace, caret — a native TSF host); Mica behind the window on Windows 11,
   a solid ground on Windows 10; the window follows 個人化 → 色彩 (accent) and the
   system light/dark live, and the app's own 淺色/深色 overrides it; high contrast
   (Alt+Shift+PrintScreen) redraws the whole window in the scheme; the five
   NavigationView items carry Segoe Fluent Icons glyphs; Narrator reads every card's
   header for its switch/combo; Tab order walks nav → cards → controls with visible
   focus; 快捷鍵 recording swallows Tab/Space/Enter only while a row records and
   releases the hook on Escape, click-away, focus loss, pane change and close (no key
   ever stays swallowed); `ContentDialog`s trap focus and answer Escape; second launch
   with `--pane` switches the existing window; changing a setting is visible on the
   next keystroke without restart; display language 自動: an English Windows UI with a
   Taiwan region draws English (UI language wins, as on the Mac), a zh-TW UI draws
   Hanji in the system's CJK face.
6. Installer: fresh install, upgrade over a running IME (expect the sign-out note),
   uninstall leaves `%APPDATA%\TaigiKeyboard` in place; scheduled task exists.
7. Update: `--check-updates` reads the live manifest; toast appears; download + verify +
   install from the 一般 pane.
8. Acceptance matrix: UI-less host, AltGr layout, high contrast, remote desktop.
   High contrast: turn on a scheme (Alt+Shift+PrintScreen) — the candidate window
   must draw in the scheme's window / text / highlight colours with no grey of ours,
   and flip back the moment the scheme goes off, without restarting the host. One
   black-background and one white-background scheme; check selected, unselected,
   annotation, separator, the expanded list and the mode flash; toggle the scheme
   both while a candidate window is showing and while none is.

Per-PR dogfood lists are appended in each PR body.
