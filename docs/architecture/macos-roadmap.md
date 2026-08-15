# macOS Desktop IME — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `macos`, `InputMethodKit`, `IMKit`, `third platform`, `engine reuse`
> **Status**: Active — Phase 0 (this doc) approved 2026-08-15; implementation PRs pending
> **Session memory**: `memory/project_macos_ime.md` (phase status + active pointer)
> **Plan provenance**: Phase-0 research + Codex pre-impl design review (ANALYSIS-ONLY, 2026-08-15) — FFI-reuse / SwiftPM-bundle / platform_id-deferral all confirmed; generation-ownership, proto-gen isolation, PR sizing, bundle-metadata cautions incorporated.

---

## Goal

Add macOS as the third platform (after iOS/Android) to prove the shared Rust
engine reuses cleanly behind a thin native shell. Native InputMethodKit app
(IMKServer / IMKInputController — NOT KeyboardKit), TL + POJ only (no TPS),
modern SwiftUI settings (macOS 14+), custom-dictionary support, macOS-conventional
shortcuts.

**Hard constraints** (active while the concurrent iOS/Android session runs):
no modification of `ios/`, `android/`, `i18n/`, `content/`; `engine/` changes
purely additive and flagged in the coordination register below; release actions
user-gated.

## Architecture (approved)

```
Host app — IMKTextInput client
        ▲ setMarkedText/insertText        │ keyDown/flagsChanged
┌───────┴──────────────────────────────────▼───────────────────┐
│ TaigiInputController : IMKInputController (thin, per-session) │
├───────────────────────────────────────────────────────────────┤
│ ComposingSessionCoordinator (process-wide)                    │
│   single ComposingManager owner; monotonic generation         │
│   allocator; ownership transfer on activate/client-switch     │
├───────────────────────────────────────────────────────────────┤
│ ComposingManager (port of iOS contract: 3-phase apply, effect │
│   order, EnterContinuous promotion, Model B commit, FetchAtPos│
│   full carrier — neutral phase only until PR8b)               │
├──────────────┬─────────────────────┬──────────────────────────┤
│ CandidatePanel│ Settings window     │ CustomDictionaryStore   │
│ NSPanel non-  │ WindowManager +     │ SQLite schema v2 + side │
│ activating +  │ NSHostingController │ table + 30k cap         │
│ NSHostingView │ (SwiftUI)           │ (iOS-invariant port)    │
├──────────────┴─────────────────────┴──────────────────────────┤
│ RustEngineBridge port — protobuf envelope over the existing   │
│ 4-fn swift-ffi surface; live-read settings → AppConfig per    │
│ request; platform_id = 0 until PLATFORM_MACOS lands (PR8a)    │
├───────────────────────────────────────────────────────────────┤
│ macos/RustEngine/RustTaigi.xcframework (aarch64-apple-darwin, │
│ new additive script) + macOS-owned generated Swift copies     │
└───────────────────────────────────────────────────────────────┘
```

Runtime data: `dictionary.fst` / `dictionary.bin` / `association.bin` /
`syllables.fst` read from `ios/Resources/Dictionaries/` at bundle-assembly time
(read-only, fail-fast validated) into `.app/Contents/Resources`; absolute paths
passed via `lexiconInstall`.

## Design decisions (D1–D10, grounded in code)

All grounded in actual code reads (actual code reading); full citations in the
Phase-0 plan and `memory/project_macos_ime.md`.

- **D1 Engine artifact** — engine `swift-ffi` crate reused UNCHANGED (4 fns,
  bytes-in/bytes-out protobuf, platform-neutral; `engine/swift-ffi/src/lib.rs:41-54`).
  New additive `engine/scripts/build-macos-xcframework.sh` builds
  `aarch64-apple-darwin` → `macos/RustEngine/RustTaigi.xcframework` + macOS-owned
  copies of generated `RustTaigi.swift` / `SwiftBridgeCore.swift`. iOS artifacts
  untouched. arm64-only; x86_64 is a later user-gated additive step.
  ⚠ **Drift rule: after any `engine/swift-ffi` change, regenerate BOTH
  `ios/RustEngine/` (iOS session) and `macos/RustEngine/`.** Unification of the
  two outputs is deferred until the concurrent session closes.
- **D2 Project format** — SwiftPM executable + `macos/scripts/bundle-app.sh`
  script-assembled `.app` (khiin-rs osx pattern); NOT an Xcode project (pbxproj
  is user-only, hook-enforced). Info.plist committed with concrete values
  (no Xcode `$(…)` placeholders); bundle ID `com.siansiansu.inputmethod.TaigiKeyboard`
  (mandatory `.inputmethod.` third segment). Mach-register temporary exception
  entitlement required if sandboxed. Ad-hoc codesign for local dev; Developer-ID /
  notarization user-gated (azooKey-Desktop `pkgbuild.sh` is the pipeline reference).
- **D3 Controller + composing state** — thin per-session `IMKInputController`;
  process-wide `ComposingSessionCoordinator` owns the single ComposingManager +
  monotonic generation allocator (engine composing state is a process singleton,
  `engine/composing/src/handle.rs:21-56`; per-controller counters would collide
  across clients — Codex must-fix). Effect vocabulary = existing cross-platform
  proto effects; attributed marked text (underline + markedClauseSegment);
  `commitComposition` without `super`; `recognizedEvents = [.keyDown, .flagsChanged]`;
  logger sink installed once at bootstrap. **Chromium deadlock rule**: never
  query the client synchronously inside `activateServer` (azooKey-Desktop
  regression-test model).
- **D4 Candidate window** — borderless non-activating NSPanel + NSHostingView
  (SwiftUI), horizontal single-row; window level = client window level + 1;
  headless navigation model unit-tested, panel is renderer only; positioning =
  pure functions ported from azooKey-Desktop `WindowPositioning` incl.
  multi-display stale-screen fix. Keys: 1-9 select · Space commit highlighted ·
  ←/→ highlight · ↑↓/PgUp/PgDn page · Enter commit raw literal · Esc cancel.
  NOT `IMKCandidates`.
- **D5 Settings** — in-process SwiftUI window (WindowManager pattern +
  activate-before-show + programmatic Edit menu); opened from IMK `menu()` AND
  `Ctrl+Shift+,` chord (Cmd+, belongs to the host app). `UserDefaults.standard`;
  live-read `EngineSettingsProvider` port; `@ConfigState`-style wrapper for
  SwiftUI re-render on external changes. Minimal defaults provider ships in PR3.
- **D6 Custom dict** — `~/Library/Application Support/<bundle-id>/custom_dictionary.db`;
  FULL iOS contract port (schema v2 + `custom_search_key` side table + migrator +
  repository + same-transaction 30000-cap guard + derivation via existing engine
  FFI). Entries ride `FetchAtPos.custom_entries` — zero new FFI. CSV via
  NSOpen/SavePanel. Backup-exclusion policy = user decision at PR7.
- **D7 User freq / nextword** — deferred. PR3–PR7 implement the full FetchAtPos
  carrier but execute the neutral phase only; `platform_id = 0` is safe —
  the only validator is nextword (`engine/nextword/src/decide.rs:53-61`) and
  macOS sends no nextword requests until PR8b.
- **D8 Dictionary artifacts** — read from `ios/Resources/Dictionaries/`
  (read-only) at bundle time with fail-fast existence/non-empty validation;
  `deploy.sh` macos destination = later coordination.
- **D9 Proto codegen** — standalone `gen-macos-protos` target with independent
  output dir `macos/Sources/TaigiInputMethod/Engine/Generated/` (committed);
  NOT wired into root `make build`.
- **D10 Dev loop** — `macos/Makefile`: build → bundle (+ plutil lint,
  codesign --verify, arch check) → install (delete-before-kill, kill by
  bundle-ID/PID, `lsregister -f -R -trusted`, re-login hint only on
  input-mode-set change). Installed-copy guard in AppDelegate; IMKServer held
  in a strong property.

## Phase / PR table

| PR | Phase | Scope | Status |
|---|---|---|---|
| PR0 | Admin | this roadmap + memory topic | this commit |
| PR1 | Engine build surface | darwin toolchain target; `build-macos-xcframework.sh`; `gen-macos-protos`; root Makefile `macos-*` targets. Zero ios/android changes. | Pending |
| PR2 | Scaffold + IMK spike | Package.swift; AppDelegate + strong-ref IMKServer + installed-copy guard; echo controller; concrete Info.plist; bundle script + validation; install loop; darwin FFI smoke test | Pending |
| PR3 | Composing core | bridge port (from iOS shape), coordinator + ComposingManager port, effect executor, lexiconInstall, attributed preedit, minimal settings provider | Pending |
| PR4 | Candidate model + window | headless nav model + tests; NSPanel + SwiftUI bar; caret anchor; selection keys; stale-owner guard | Pending |
| PR5 | Settings + menubar | WindowManager, SwiftUI form, menu items, Ctrl+Shift+, chord, live-read provider, TL↔POJ toggle, OSLog bootstrap | Pending |
| PR6 | Custom dict persistence | store: schema v2 + side table + migrator + capacity + derivation; custom_entries injection | Pending |
| PR7 | Custom dict UI | CRUD UI, CSV import/export, backup-exclusion decision (user-gated) | Pending |
| PR8a | Proto coordination ⚠ | `PLATFORM_MACOS` + macOS nextword decide contract + triple-touch regen (touches ios/android GENERATED files only; timing user-coordinated) | Pending |
| PR9 | Freq/nextword + polish | phase-2 boosted fetch, user_frequency.db, association learning, edge cases, docs, dogfood fixes | Pending |

Every PR: Codex sandwich + `/simplify`; PR1/PR8a additionally FFI-adjacent
review per `.claude/rules/rust-ffi-safety.md` §5. Engine-touched PRs run
`cargo test --workspace`; macOS PRs run `swift test` (from PR2 on).

## Shared-surface coordination register

| Change | PR | Additive? | ios/android files touched? |
|---|---|---|---|
| `rust-toolchain.toml` + `aarch64-apple-darwin` | PR1 | yes | no |
| `engine/scripts/build-macos-xcframework.sh` (new) | PR1 | yes | no |
| `gen-macos-protos` isolated target | PR1 | yes | no (proto unchanged → no regen diff) |
| Root Makefile `macos-*` targets | PR1 | yes | no |
| `PLATFORM_MACOS` + regen | PR8a | yes | **yes — generated `.pb.swift`/`.java` only, semantically inert; user-coordinated timing** |
| `engine/swift-ffi` crate | — | **no change needed** | — |

## 最佳實踐對齊 (references)

Entry point per repo policy: `docs/references/mainstream-ime-comparison.md`
(cards #2 azooKey-Desktop, #11 McBopomofo, #12 khiin-rs, #20 MacishType).
Key borrowings (full file:line table in the Phase-0 plan / memory):

- khiin-rs osx — SwiftPM → script-assembled .app; IME Info.plist key set; darwin staticlib build.
- MacishType — install Makefile (lsregister + mode-dict diff), action-list interpreter, attributed marked text, in-process settings, installed-copy guard.
- azooKey-Desktop — Chromium activateServer deadlock rule + regression test, pure `WindowPositioning` + multi-display fix, `@ConfigState` wrapper, sandbox mach-register entitlement, notarization pipeline.
- McBopomofo — recognizedEvents mask, mature IMK lifecycle patterns.
- iOS in-repo — ComposingManager contract, EngineSettingsProvider live-read, custom-dict schema v2 invariant.

**刻意不採用**: khiin settings helper app · Xcode project (pbxproj user-only) ·
`IMKCandidates` · macOS-local action enum · in-IME English mode · TPS seams ·
universal binary now · third committed dictionary copy · shielding window level ·
azooKey out-of-process XPC daemon (+ its async queue/idempotency machinery — our
FFI is in-process sync) · OS-level `ComponentInputModeDict` modes for TL/POJ
(settings toggle instead) · azooKey NSTableView candidate UI · `/Library/Input
Methods` sudo install.

## User-gated open items

- PR8a timing (proto regen window vs concurrent iOS/Android session).
- Intel/x86_64 support (distribution decision).
- Custom-dict Time-Machine/backup-exclusion policy (PR7).
- macOS dogfood acceptance checklist contents (proposed at PR9).
