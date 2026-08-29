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
- Pin UI-framework versions the session can author against (`eframe`/`egui` 0.31). Do not bump
  them without a Windows box to run the result on.

## macOS is the behaviour oracle

- Key table, modes, auto-space, full-width punctuation, candidate geometry, settings keys and
  defaults, storage schemas, CSV, update flow all mirror `macos/Sources/TaigiInputMethodCore/**`.
  A port carries a `// mirrors macos/.../<File>.swift:<line>` comment on the mirrored constant or
  rule. Drift is a bug unless the roadmap names it as an intentional divergence.
- Named divergences so far: egui chrome (not SwiftUI), ⌘→Ctrl / ⌃→Alt modifier mapping, AppContainer
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
  differ between crate versions. `doc-lookup.md` applies: TSF is a framework API.

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
- Version source of truth = `windows/Cargo.toml` `[workspace.package] version`, written only by
  `make version x.y.z`.
