# Documentation Lookup (platform IME / framework APIs)

Read before adding or changing a call to (or a contract with) a framework/OS API whose current surface the repo itself does not settle — in particular KeyboardKit, Apple `UIInputViewController` / `UITextDocumentProxy`, Android `InputMethodService` / `InputConnection` / `EditorInfo`, Jetpack Compose, DataStore. Edits that do not introduce or alter a framework API call (rename, comment, formatting, pure-Swift/Kotlin logic) do not trigger this.

## The rule

Verify a platform IME / framework API against current docs before coding it: KeyboardKit and Android IME APIs drift between versions (KeyboardKit's setup API moved to `setupKeyboardKit(for:)`; this repo deliberately keeps the legacy `KeyboardSettings.setupStore(for:)` path — read `KeyboardExtension/KeyboardViewController+Setup.swift` before touching it).

## How (in priority order)

1. **Context7 `ctx7` CLI** (primary — the `find-docs` skill in Claude Code; returns real code + APIDOC, no auth needed; rate-limited → `ctx7 login` or `CONTEXT7_API_KEY`). Two steps; `npx -y ctx7@latest …` (macOS has no `timeout` — do not wrap it); ≤3 calls per question.
   - Verified library IDs (skip re-resolution):
     - iOS KeyboardKit (the pbxproj allows `11.0.0` up to (not including) `12.0.0`; the version actually built is in the gitignored `ios/TaigiKeyboard.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved` of the checkout you build from — read it before pinning a query): **`/keyboardkit/keyboardkit`**. Version-pin `/keyboardkit/keyboardkit/<ver>` if `ctx7 library` lists one.
     - Android official IME: **`/websites/developer_android`** (canonical) or **`/websites/developer_android_training`**.
     - Apple keyboard-extension primitives (when not via KeyboardKit): resolve per task — `ctx7 library "apple swift uikit UIInputViewController"`.
   - `ctx7 library <name> "<task question>"` → `ctx7 docs <id> "<task question>"`.
2. **Local authoritative fallback** (offline / version-pinned — **MAIN repo only**, gitignored, not in git worktrees):
   - `references/KeyboardKit-Documentation/` — DocC archive pinned to the cloned KK version (`documentation/keyboardkit/` JSON). Prefer it over a web search (per `docs/references/mainstream-ime-comparison.md`).
   - `references/keyboardkit9.9.0/Sources/KeyboardKit/` — last open-source KK tree (10.x and later are closed-source); symbol names lag the current 11.x API, confirm via `ctx7`.
   - Android has **no** local clone → use `ctx7` or https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method.
3. **The vendor docs site** — a specific Apple/Android doc URL or release notes when Context7 lacks coverage.

## Per-PR gate

Every platform-IME PR description states: **which API was looked up, via which source, and the doc version/date** — same discipline tier as code review. A reviewer may reject a platform-IME change that cites no doc source.
