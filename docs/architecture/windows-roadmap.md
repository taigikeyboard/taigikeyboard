# Windows Desktop IME — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `windows`, `TSF`, `Text Services Framework`, `fourth platform`, `engine reuse`, `macOS parity`
> **Status**: Phase 0 approved 2026-08-29 (Fable 5 research + Codex ANALYSIS-ONLY pre-impl review, verdicts folded in below); implementation in progress, authored WITHOUT a Windows machine
> **Session memory**: `memory/project_windows_ime.md` (phase status + active pointer)
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
2. **No Windows machine exists yet.** Everything is authored blind and verified on the
   macOS host (§ W13). First-run breakage on a real Windows box is expected and is the
   dogfood's job — see § Dogfood run-book.

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
│ TaigiKeyboardSettings.exe (crate taigi-windows-settings, eframe/egui =0.31.1)      │
│  sidebar: 一般 / 外觀 / 快捷鍵 / 自訂詞庫 / 詞庫來源 (+ unlisted 辭典搜尋)           │
│  custom-dict CRUD + CSV · shortcut recorder · update check/download/verify/install  │
│  `--check-updates` headless mode (run by a per-user scheduled task)                 │
└────────────────────────────────────────────────────────────────────────────────────┘
Runtime data: %APPDATA%\TaigiKeyboard\{settings.json, user_frequency.db,
user_association.db, custom_dictionary.db}; install dir %ProgramFiles%\TaigiKeyboard\
{TaigiKeyboard.dll (x64), x86\TaigiKeyboard.dll, arm64\TaigiKeyboard.dll,
TaigiKeyboardSettings.exe, dictionaries\*, fonts\*}.
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
  release-build time from `ios/Resources/Dictionaries/` (macOS D8: no third committed
  copy) and fonts from `ios/Resources/Fonts/` (`macos/scripts/bundle-app.sh:178-201`)
  into the install dir, resolved from the DLL's **own `HMODULE` saved in `DllMain`**
  (Codex: never from the current exe). **Codex: CONFIRM WITH CHANGES** — AppContainer
  (UWP) hosts cannot read `%APPDATA%`: the TIP holds an explicit per-process
  `DataCapability { settings, custom_dictionary, learning }` state, probed once and
  logged once, so it never re-opens files or re-logs per keystroke; classified
  **unsupported host capability**, and `GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT` is
  therefore NOT declared (see W6). Atomic settings replace = write temp + `rename`
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
  (rakukan `docs/DESIGN.md:745-747`). DB writes go to a per-process bounded writer
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
  from the appearance setting or the system theme. The three layouts, metrics, paging,
  grid, positioning and index-label rules are the macOS pure models ported verbatim
  (`Candidates/HorizontalPageLayout.swift`, `ExpandedGridLayout.swift`,
  `CandidateMetrics.swift`, `CandidatePanelPositioning.swift`) with their test
  oracles as Rust tests. Sequoia geometry (6 pt corners, full-cell highlight). Caret
  rect from `ITfContextView::GetTextExt` on the collapsed selection (rakukan
  `on_compose.rs:29-43`), treating clipped / empty / failed rects as "no anchor" with
  khiin's fallbacks (`composition_utils.rs:31-69`). Mouse click selects, never commits
  (`CandidateItemView.swift:47-48`, identical semantics), so no edit session is needed
  from the window. **Codex: UI-less mode is a v1 architecture item, not a dogfood
  note** — the TIP registers `GUID_TFCAT_TIPCAP_UIELEMENTENABLED`, implements
  `ITfUIElement` + `ITfCandidateListUIElement` over the same list model, and calls
  `ITfUIElementMgr::BeginUIElement`; when the host answers "do not show", the popup
  stays hidden and the host renders the list itself (khiin `tip/candidate_list_ui.rs`).
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
  defaults — **Codex F7: Ctrl+Shift, not Ctrl+Alt (AltGr is a real conflict)**:
  `openLastSettingsPane` Ctrl+Shift+S, `toggleRomanization` Ctrl+Shift+C,
  `toggleTranslateSwapped` bare `` ` `` (consumed in the key sink only while the TIP is
  active and the classifier says so — a bare key is not a preserved key). Slot-key sets
  identical (`ComposingKeyBindings.swift:15-36`): bare `q w d f z x v y ;` default,
  Shift / Ctrl / Alt + 1–9; the stored value stays `option` (portable settings shape)
  and is rendered as Alt. Chords are registered with `ITfKeystrokeMgr::PreserveKey`
  from the stored settings and re-registered on reload.
- **W6 Lang bar / menu** — one `ITfLangBarItemButton` (`GUID_LBI_INPUTMODE`,
  `TF_LBI_STYLE_BTN_MENU | TF_LBI_STYLE_SHOWNINTRAY`; rakukan `language_bar.rs:21-23`)
  whose `InitMenu` mirrors the macOS input-source menu exactly: 設定 / separator /
  檢查更新 (`TaigiInputController.swift:264-329`). **Codex: REFUTE the category list**
  — declared categories are ONLY what is true: `GUID_TFCAT_TIP_KEYBOARD`,
  `GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER`, `GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT` (rakukan
  `registration.rs:118`), `GUID_TFCAT_TIPCAP_UIELEMENTENABLED` (W4). NOT `COMLESS`
  (means COM-less activation support, which this TIP does not implement), NOT
  `IMMERSIVESUPPORT` (contradicts the W2 AppContainer degradation), NOT
  `INPUTMODECOMPARTMENT` (no 中/英 mode; macOS has no ABC mode either — the OS switches
  input sources, `TaigiInputController.swift:132-138`).
- **W7 Registration** — LANGID `0x0404` only (Codex F5 CONFIRM: no duplicate profile
  under en-US; discoverability is the installer/onboarding's job). `ITfInputProcessorProfiles::Register`
  first, then `ITfInputProcessorProfileMgr::RegisterProfile` (rakukan
  `registration.rs:80-115`), icon = DLL resource. Unregistration is **item-by-item
  symmetric** (CLSID, profile, each category, display-attribute provider) — Microsoft
  requires individual category removal. Fresh GUIDs allocated in Phase 0. rakukan's
  `HKLM → HKCU CTF\TIP` copy for Windows 11 Settings visibility is NOT adopted (its
  uninstall deletes other IMEs' keys) — dogfood item.
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
  (mirror of the macOS publisher: GitHub release on the website repo tagged
  `windows-v<version>`, anonymous 200/206 checks, then `appcast/windows.json` +
  `_data/windows_release.json`, poll live). Root `make windows-release`,
  `make windows-check`. Version source of truth = `windows/Cargo.toml`
  `[workspace.package] version`; `tools/release_notes.py` `set-versions` /
  `check-versions` extended so `make version x.y.z` moves all four platforms.
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
- **W14 Toolchain** — `windows/rust-toolchain.toml`: stable + `x86_64-pc-windows-msvc`
  (v1 ship), `i686-pc-windows-msvc`, `aarch64-pc-windows-msvc` (prepared) +
  `x86_64-pc-windows-gnu` (host check). `+crt-static` for the MSVC DLL and exe, verified
  per artifact by the release script's import-table check (no `vcruntime*.dll` /
  `msvcp*.dll` imports).
- **W15 UI framework for settings** — eframe/egui **`=0.31.1`** (its API is the one
  this session can author against; 0.36 changed `App::update` and the panel API).
  Codex F2 CONFIRM WITH CONDITIONS: egui-winit 0.31 has IME integration, but
  preedit/commit/backspace in `TextEdit`, CJK font fallback, and `rfd` dialog parenting
  are **unverified until Windows dogfood** and sit in the run-book. Chosen over C#
  WinUI3/WPF (a fourth language, unbuildable here, P/Invoke for search + SQLite) and
  raw Win32 controls. Parity = information architecture, control semantics and
  workflow (ported); the chrome is egui's — **platform-adapted presentation**. CJK
  glyphs from the bundled `jf-openhuninn-2.1.ttf` (the file macOS ships) loaded into
  egui's font definitions.
- **W16 Docs** — roadmap = decisions + PR DAG + acceptance; memory = round hand-off;
  `docs/architecture/windows-release.md` = operator procedure;
  `.claude/rules/windows-guidelines.md` = durable constraints; `windows/updates/README.md`
  = manifest contract only. No duplicated truth between them.

## Windows-specific acceptance matrix (Codex W15/F14)

Not "chrome differences" — each needs its own acceptance line in the run-book:
UI-less candidate hosts (games, some Store apps) · password / secure fields (no
composition, no learning) · AltGr layouts · high contrast · screen reader (candidate
window exposes text via UIA — deferred, named) · touch keyboard · remote desktop ·
Chrome child-HWND focus · Notepad composition length cap · Windows Terminal.

## Phase / PR table

USER sizing preference (2026-08-17): ~600–1000 LOC per PR, fewer and larger. Every
coding PR runs the Codex sandwich (pre-impl on that PR's design fork, post-impl on the
diff) and the W13 gates. Order revised per Codex F12.

| PR | Phase | Scope | Status |
|---|---|---|---|
| PR0 | Admin | this roadmap + memory topic + `.claude/rules/windows-guidelines.md` + docs index | direct-to-main |
| PR1a | Proto | `PLATFORM_WINDOWS` + `make build` regen (mechanical) | Pending |
| PR1b | i18n + tooling | `macos` → `desktop` namespace rename; `windows` platform + Rust emitter; `release_notes.py` Windows version writer/check; root Makefile `windows-check` / `windows-release` | Pending |
| PR2 | Scaffold + core composing | `windows/` workspace + toolchain; `taigi-windows-core`: settings model + revision, engine bridge (envelope, AppConfig, generation, lexicon install, logger), `ComposingSessionCoordinator` keyed by context token, ComposingManager port (3-phase apply, effects, fetch protocol, commit outcomes), `ComposingKeyIntent` 7-tier table + `KeyEventSnapshot`; engine round-trip tests against `ios/Resources/Dictionaries`. **Locks**: word identity `(漢字, canonical TL)`, context ownership + handover, engine generation rules, effect ordering + failure semantics, settings revision | Pending |
| PR3 | Core candidates | `CandidateMetrics<TextMeasurer>`, `HorizontalPageLayout`, `VerticalLayout`, `ExpandedGridLayout`, positioning, index labels, cell content, document text; macOS oracle numbers as tests | Pending |
| PR4 | Storage + policies | `taigi-windows-storage`: rusqlite stores (freq v2 / assoc v6 / custom v3, byte-identical SQL, WAL + bounded busy handling, migration under `BEGIN IMMEDIATE`), `LearningCapacity`, CSV codec, seeds, settings file store (atomic replace); core: `AutoSpacePolicy` + attaching set, `FullWidthPunctuation`, `NextWordLearner`, shortcuts model (chords, registries, conflicts, recorder gate) | Pending |
| PR5a | TSF lifecycle | COM exports + class factory + symmetric registration + GUIDs; `TextService` activate/deactivate; thread-mgr / thread-focus sinks; context identity; lang-bar button + menu; settings reload; spawn settings exe; **smoke TIP that composes nothing** | Pending |
| PR5b | TSF composing | key sink → snapshot → intent → manager inside sync edit sessions; composition start/update/commit per context; display attribute; preserved keys; password/read-only gating; handover | Pending |
| PR6 | TSF UI | candidate window (D2D/DWrite renderer, 3 layouts, DPI scope, theme, mouse, private fonts, caret positioning + fallbacks, device loss) + UI-less `ITfCandidateListUIElement` contract; mode flash panel; unfold animation | Pending |
| PR7 | Settings exe I | eframe shell + sidebar + 一般 / 外觀 / 快捷鍵 panes + shortcut recorder + display language + fonts | Pending |
| PR8 | Settings exe II | 自訂詞庫 (table, CRUD sheet, CSV import/export, delete all, clear learning) + 詞庫來源 + unlisted 辭典搜尋 + external lookup URLs | Pending |
| PR9 | Updates | `taigi-windows-update`: manifest model + checker + download + Authenticode pin + install flow; `--check-updates` headless mode; toast; 一般-pane rows | Pending |
| PR10 | Installer + release | Inno script (x64 first; x86/ARM64 gated), scheduled task, `release-app.sh`, `publish-release.sh`, `windows/Makefile` dev loop, `windows-release.md`, `windows/updates/README.md` | Pending |
| PR11 | Dogfood fixes | first real-Windows smoke: fixes from the run-book + memory hand-off (not admin-only) | Pending |

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
| Website repo: `appcast/windows.json`, `_data/windows_release.json`, `windows-v*` tags | PR10 | yes | separate repo |

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
| Ctrl+Alt shortcut defaults | (own first draft) | Codex F7: AltGr conflict on non-US layouts. |
| `GUID_TFCAT_TIPCAP_COMLESS`, `IMMERSIVESUPPORT` | khiin/rakukan register all seven | Codex W6: declaring capabilities the TIP does not have. |
| HKLM → HKCU `CTF\TIP` registry copy | rakukan `rakukan_installer.iss:134-138` | Uninstall deletes the whole HKCU TIP key — other IMEs' settings. Dogfood decides if Windows 11 needs anything. |
| `MessageBox` in `DllRegisterServer`, `panic!` in edit sessions, advising a second sink object | khiin `dll.rs:173-180`, `edit_session.rs:32`, `key_event_sink.rs:87-88` | Live bugs, not patterns. |
| Threads spawned from `DllMain` | rakukan `lib.rs:127-130` | Loader lock. |
| Shift-tap 中/英 toggle, Space-to-convert | Windows CJK convention | macOS has neither; USER asked for an identical experience. Codex F14: keep the macOS table as the default; Windows-native alternatives may be offered as settings later, never as a silent default change. |
| WiX / MSI-written registry | khiin `installer/Registry.wxs` | `DllRegisterServer` is the single source of registration truth; an MSI mirror drifts. |
| Resident tray agent | rakukan `rakukan-tray` | Codex F8: a scheduled task + settings-exe overdue check covers the update trigger without a resident process. |

## Dogfood run-book (first Windows machine)

Ordered in layers — stop at the first foundation failure:

1. Toolchain: `rustup target add` the MSVC targets; `cargo build --release -p
   taigi-windows-tsf -p taigi-windows-settings`; `iscc` installed.
2. `regsvr32 TaigiKeyboard.dll` from an elevated prompt → the TIP appears under
   設定 → 時間與語言 → 語言 → 中文(台灣) → 鍵盤. If invisible on Windows 11, evaluate
   rakukan's HKLM→HKCU copy (W7 open item) before anything else. `regsvr32 /u` removes
   every key it added (W7 symmetry).
3. Notepad: type `taigi` → underlined preedit, candidate window below the caret,
   `q` commits slot 0, Return commits highlighted, Space commits the alternate script,
   Esc cancels, digits are tones. Then Word / Chrome / Windows Terminal / a UWP app
   (expect W2 degradation there) / a password field (expect no composition).
4. Lang-bar button in the tray: menu 設定 opens the exe; Ctrl+Shift+S / Ctrl+Shift+C /
   `` ` `` work while the TIP is active.
5. Settings exe: each pane matches the macOS pane order and controls; typing Taiwanese
   INTO the custom-dict fields (egui IME path: preedit, commit, backspace, caret); CJK
   glyphs render; file dialogs return focus; changing a setting is visible on the next
   keystroke without restart.
6. Installer: fresh install, upgrade over a running IME (expect the sign-out note),
   uninstall leaves `%APPDATA%\TaigiKeyboard` in place; scheduled task exists.
7. Update: `--check-updates` reads the live manifest; toast appears; download + verify +
   install from the 一般 pane.
8. Acceptance matrix: UI-less host, AltGr layout, high contrast, remote desktop.

Per-PR dogfood lists are appended in each PR body.
