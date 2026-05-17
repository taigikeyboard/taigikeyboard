# Documentation Lookup (platform IME / framework APIs)

Mandatory rule. Read **before** adding or changing a call to (or a contract with) a framework/OS API whose current surface you are not certain of from the repo itself — in particular **KeyboardKit**, Apple **`UIInputViewController` / `UITextDocumentProxy`**, Android **`InputMethodService` / `InputConnection` / `EditorInfo`**, **Jetpack Compose**, **DataStore**. Trivial edits that do not introduce or alter a framework API call (rename, comment, formatting, pure-Swift/Kotlin logic) do not trigger this.

**Active window**: every platform implementation round, indefinitely. Verified working 2026-05-18.

## The rule

**Never code a platform IME / framework API from model memory. Verify the current API against authoritative docs first.** Pre-trained knowledge of KeyboardKit / Android IME APIs drifts between versions (confirmed: live KeyboardKit docs already expose a newer setup API — `viewWillSetupKeyboardKit()` / `setupKeyboardKit(for:)` — than parts of this repo use).

## How (in priority order)

1. **`find-docs` skill → Context7 `ctx7` CLI** (primary; returns real code + APIDOC, no auth needed; rate-limited → `ctx7 login` or `CONTEXT7_API_KEY`). Two steps; `npx -y ctx7@latest …` (macOS has no `timeout` — do not wrap it); ≤3 calls per question.
   - Verified library IDs (skip re-resolution):
     - iOS KeyboardKit (Xcode pins the range **9.7.2 ..< 11.0.0**; local clone = 9.9.0 — confirm the resolved version in Xcode before pinning a query): **`/keyboardkit/keyboardkit`**. Version-pin `/keyboardkit/keyboardkit/<ver>` if `ctx7 library` lists one.
     - Android official IME: **`/websites/developer_android`** (canonical) or **`/websites/developer_android_training`**.
     - Apple keyboard-extension primitives (when not via KeyboardKit): resolve per task — `ctx7 library "apple swift uikit UIInputViewController"`.
   - `ctx7 library <name> "<task question>"` → `ctx7 docs <id> "<task question>"`.
2. **Local authoritative fallback** (offline / version-pinned — **MAIN repo only**, gitignored, not in git worktrees):
   - `references/KeyboardKit-Documentation/` — DocC archive pinned to the cloned KK version (`documentation/keyboardkit/` JSON). Treat as gospel before WebSearching (per `docs/references/mainstream-ime-comparison.md`).
   - `references/keyboardkit9.9.0/Sources/KeyboardKit/` — KK source to read a symbol directly.
   - Android has **no** local clone → use `ctx7` or `WebFetch https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method`.
3. **`WebFetch` / `WebSearch`** — a specific Apple/Android doc URL or release notes when Context7 lacks coverage.

## Per-PR gate

Every platform-IME PR description states: **which API was looked up, via which source, and the doc version/date** — same discipline tier as the Codex review sandwich. A reviewer may reject a platform-IME change that cites no doc source.

Aligns with global `CLAUDE.md` ("prefer online docs over pre-trained knowledge; verify current APIs/versions before suggesting"); this file makes it a **hard per-change gate**, not a soft default.
