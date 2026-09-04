---
paths: ["windows/**"]
---

# Windows Project Guidelines

Mandatory rules for the Windows input method (`windows/`, Rust: TSF DLL + settings exe over the
shared engine). Read before modifying Windows code. Design record: `docs/architecture/windows-roadmap.md`.

## Authored without a Windows machine

- The platform was written blind (USER 2026-08-29). Every PR passes `make windows-check` on the
  macOS host: `cargo test -p taigi-windows-core` (native), `cargo clippy --workspace --all-targets
  --target x86_64-pc-windows-gnu -- -D warnings` (full graph incl. rusqlite via mingw-w64), and
  `cargo check --target x86_64-pc-windows-msvc` for the non-C crates. These gates prove compilation,
  not behaviour — behaviour is the dogfood run-book's job. Never claim "works on Windows".
- Put logic in `taigi-windows-core` (`unsafe_code = forbid`, host-testable) whenever it does not
  need a Win32 handle. `taigi-windows-tsf` and `taigi-windows-settings` are thin shells.
- Pin UI-framework versions to what has actually run on the Windows box: `windows-reactor` is a
  git dependency pinned to a commit SHA (W17; crates.io has only placeholders) — bump only in its
  own round, built and smoke-run on the box. The settings crate's real gate is `make check-box`
  (ssh MSVC clippy); the macOS gnu check is a type-check only (`reactor-setup` refuses gnu).
- The TSF DLL never links or loads WinUI / the Windows App Runtime (release-script import +
  manifest-marker gates). Bumping the `windows-reactor` pin changes the staged runtime set: the
  bump PR must add `[InstallDelete]` entries for every name in the PREVIOUS
  `build-support/windows-app-runtime-files.txt` that the new list drops (Inno never removes files
  a newer `[Files]` wildcard stops covering), and `build-support/resource.rs` must never emit an
  `RT_MANIFEST` (the linker embeds the runtime's). It also changes which runtime images the window
  actually maps: the bump round must re-dump a running settings window's install-directory modules
  ON THE BOX and update `taigi-windows-settings::prewarm`'s `RUNTIME_IMAGES` + its snapshot
  revision (`the_snapshot_belongs_to_the_pinned_reactor` goes red until it does). A name that
  starts loading without being listed costs nothing visible — the window just opens slowly again.
  Reactor exposes no keyboard events: the shortcut recorder uses a THREAD-scoped `WH_KEYBOARD`
  hook in `taigi-windows-platform` (RAII, `catch_unwind`, swallow down+up while recording,
  pass-through otherwise) — never a global hook, never a raw XAML object mutation.
- **A single-child content or slot takes exactly ONE view.** `Border::content`, `Button::content`
  and every non-collection `SlotView::new` resolve to one native root; `View::fragment` /
  `View::keyed_fragment` flatten to one root per child, so a fragment in one of those places is
  `PumpError::StructureUnsupported` — an unhandled `E_FAIL` out of `OnLaunched`, which XAML turns
  into a PROCESS FAIL-FAST with no message anywhere but Windows Error Reporting. Group the
  children in a panel (`StackPanel::keyed_children` to keep their identity); `SlotView::collection`
  only works on slots the reactor marks as collections, which `ExpanderSlot::Content` is not.
  Shipped twice in W17 and found only on the device. `cards::frame` now stacks what it is given
  so that one card helper cannot fail this way again, and every pane is mounted headlessly by
  `winui::pane_planning` against the reactor's `RecordingRuntime` — **a new pane belongs in that
  test.** The net reaches each pane's LAUNCH state only: a subtree behind user state (a dialog,
  a busy overlay, a search result row) is `View::empty()` there and is still dogfood-only.
- **A lang-bar item's menu is drawn by us, not by TSF.** The Windows 8+ taskbar input indicator
  hosts `GUID_LBI_INPUTMODE` and routes clicks to `ITfLangBarItemButton::OnClick`; it never drives
  `InitMenu`, so a `TF_LBI_STYLE_BTN_MENU` item shows nothing at all there. The item is a
  `TF_LBI_STYLE_BTN_BUTTON` and `lang_bar::show_popup` builds a Win32 popup — mozc
  (`tip_lang_bar.cc:196-240`) and khiin-rs (`lang_bar_indicator.rs:53-58`) both do exactly this.

## macOS is the behaviour oracle

- Key table, modes, auto-space, full-width punctuation, candidate geometry, settings keys and
  defaults, storage schemas, CSV, update flow all mirror `macos/Sources/TaigiInputMethodCore/**`.
  A port carries a `// mirrors macos/.../<File>.swift:<line>` comment on the mirrored constant or
  rule. Drift is a bug unless the roadmap names it as an intentional divergence.
- Named divergences so far: WinUI 3 chrome (not SwiftUI; window frame not persisted, label click does not toggle a switch, 外觀 mode is a native pop-up rather than the Mac's drawn thumbnails, the width floor is 600 so `NavigationView` can compact its pane), ⌘→Ctrl / ⌃→Alt modifier mapping, AppContainer
  hosts run on defaults, Windows toast instead of `UNUserNotification`, no `.taigi` pane (macOS
  retired it too).

## TSF / COM discipline

- Every exported COM method body is wrapped in `catch_unwind`; the release profile stays
  `panic = "unwind"`. A panic must never cross into the host process.
- `DllCanUnloadNow` returns `S_FALSE` forever; `DllMain` only stores the HINSTANCE (no threads,
  no COM, no file I/O).
- Engine + dictionaries initialise lazily on the first key the TIP handles, never in `Activate`.
- Edit sessions: `TF_ES_SYNC | TF_ES_READWRITE` from inside the key sink, or from the UI-less
  element's host-initiated `Finalize` / `Abort` (same key path, refused when the engine is busy).
  Never call `RequestEditSession` from a WndProc or timer callback.
- `ITfThreadMgrEventSink::OnSetFocus` (and the other focus / context sinks) only flag state and
  post the window's hide (`PostMessage`), then return. The candidate window is shown / hidden
  after the edit session returned, never inside it.
- Every `HWND` owns nothing: the `PopupWindow` owns the handler box, the window borrows a pointer.
  Monitor / DPI queries run inside the per-monitor-v2 thread scope (`with_per_monitor_dpi`).
- `OnTestKeyDown` and `OnKeyDown` run the same classifier — terminals skip the former.
- Every `unsafe` block carries a `// SAFETY:` comment (`rust-ffi-safety.md` §3).
- Verify any Win32/TSF API shape against the `windows` crate metadata (`cargo doc` / the crate
  source under `~/.cargo/registry`) before use — parameter shapes (`Ref<T>`, `BOOL`, `PCWSTR`)
  differ between crate versions. `doc-lookup.md` applies: TSF is a framework API. The generated
  SIGNATURE is not the contract: read the wrapper's BODY too (see the next bullet).
- **A Win32 or COM call that FILLS a buffer we read back goes through
  `taigi_windows_platform::os_out_buffer` (free functions) or `taigi-windows-tsf`'s
  `com_out_buffer` (COM methods) — never the `windows` crate's `&mut [T]` wrapper.** Those
  wrappers hand the OS `core::mem::transmute(slice.as_ptr())`, a READ-ONLY provenance; writing
  through it is UB, and an optimized build folds the caller's reads of the buffer back to its
  initializer. That is how every keyboard modifier came to read as "not held" in release builds
  (2026-09-04, real device — `os_out_buffer`'s module doc carries the measurement, and the two
  found earlier by reasoning, `ToUnicodeEx`'s output buffer and `IEnumGUID::Next`, had not
  misbehaved yet). Invisible to every gate the project runs: it does not reproduce in debug,
  `check-gnu` / `check-msvc` are compile gates, and the host tests cannot reach a Win32 handle.
  `windows/clippy.toml` denies the wrappers already wrapped; add the next one there as it is
  wrapped (the enabled feature set holds ~138 more of the same shape).

## Data + settings

- `%APPDATA%\TaigiKeyboard\` for `settings.json` + the three user DBs; the install dir for
  dictionaries and fonts, resolved from the DLL's module path. No registry-stored settings.
- SQLite schemas are byte-identical to the macOS `CREATE TABLE` text; multi-process access uses
  WAL + `busy_timeout`. Writes are best-effort (AppContainer hosts cannot write).
- Settings are live-read (invariant §11): the TIP re-reads on mtime change; the settings exe
  writes atomically (tmp + rename).

## Release

- `make windows-release` runs on a Windows host (Git Bash); `windows/scripts/publish-release.sh`
  mirrors the macOS publisher and targets the same website repo. Never run it without USER's
  explicit release instruction (`diagnosis-discipline.md` § No unilateral release scope).
- **Releases ship UNSIGNED** (owner 2026-09-04, no certificate for a year or two):
  `make windows-release RELEASE_FLAGS=--skip-sign`, which passes `--allow-unsigned` down to the
  publisher; the artifact keeps the plain `TaigiKeyboard-<version>.exe` name. What admits a
  downloaded package is `verify::admit` — the manifest's `packageSHA256` ALWAYS, plus the
  Authenticode checks only when the running copy has a signature of its own, never one instead of
  the other. `windows-release.md` § Signing status is the single source for this; do not restate
  the policy elsewhere.
- Signing, when a certificate exists, is by thumbprint (`WINDOWS_SIGNING_THUMBPRINT`); the updater
  pins the signer's LEAF, so a certificate rotation costs the in-app Authenticode check for the
  copies signed with the old one (`windows-release.md` § Notes). Both binaries and the installer
  carry VERSIONINFO `ProductName = TaigiKeyboard` + `ProductVersion = <workspace version>` — the
  updater's package check reads them; keep the `build-support/resource.rs` block and the `.iss`
  `VersionInfo*` directives in step.
- Version source of truth = `windows/Cargo.toml` `[workspace.package] version`, written only by
  `make version-desktop x.y.z` (the desktop train — macOS + Windows share one number;
  iOS + Android are the separately numbered mobile train).
