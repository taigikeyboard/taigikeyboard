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

**Hard constraints**: release actions stay user-gated. The
"no modification of `ios/`, `android/`, `i18n/`, `content/`; `engine/` changes
purely additive" rule came from a concurrent iOS/Android session — **lifted
2026-08-15 (USER: 「目前沒有並行 session」)**. Shared-surface changes are now
allowed on their merits; the coordination register below stays as the record of
what each PR touches, and PR8a's proto regen is still user-timed.

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
│ NSPanel non-  │ SettingsWindow-     │ SQLite schema v2 + side │
│ activating +  │ Controller +        │ table + 30k cap         │
│ NSHostingView │ NSHostingController │ (iOS-invariant port)    │
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
  `make build` runs the iOS and macOS xcframework scripts together, so the two
  outputs cannot drift behind a `swift-ffi` change. The shared swift-bridge
  post-processing (OUT_DIR discovery, modulemap, `import` injection,
  `@retroactive` patch) lives in `engine/scripts/lib/swift-bridge-artifacts.sh`
  — extracted in #518 `9bb849cb`, verified by a 22-artifact byte-identity
  manifest. OUT_DIR discovery asks cargo (`--message-format=json`,
  `build-script-executed`) replaying the caller's exact argv, fail-closed on
  anything but one unique match (#519 `4809b5c3`); the previous newest-by-mtime
  heuristic was measurably selecting the `panic-injector` fingerprint for a
  cached release build. Also noted: `xcodebuild -create-xcframework`
  orders `AvailableLibraries` nondeterministically, so the committed iOS
  `Info.plist` can churn between otherwise identical builds — compare it
  semantically, do not treat a slice-order flip as a real diff.
- **D2 Project format** — SwiftPM executable + `macos/scripts/bundle-app.sh`
  script-assembled `.app` (khiin-rs osx pattern); NOT an Xcode project (pbxproj
  is user-only, hook-enforced). Info.plist committed with concrete values
  (no Xcode `$(…)` placeholders); bundle ID `com.siansiansu.inputmethod.TaigiKeyboard`
  (mandatory `.inputmethod.` third segment). Mach-register temporary exception
  entitlement required if sandboxed. Ad-hoc codesign for local dev; Developer-ID /
  notarization user-gated (azooKey-Desktop `pkgbuild.sh` is the pipeline reference).
  **Refined at PR2 #520**: the package is a library (`TaigiInputMethodCore`) plus
  a thin executable, not a single executable target, because `swift test` needs
  an importable module and PR3–PR9 are unit-test-heavy. Generated `.pb.swift`
  therefore live under `Sources/TaigiInputMethodCore/Engine/Generated/`. The
  swift-bridge wrappers compile as their own `RustTaigiSwift` target pinned to
  Swift 5 language mode (generated sources cannot be annotated for Swift 6
  strict concurrency); everything above it is Swift 6 mode.
  `macos/scripts/lib/bundle-identity.sh` is the only reader of Info.plist and
  the only place the assembled bundle's path is named. Info.plist omits
  `ComponentInputModeDict` (TL/POJ are an app setting per D5) and sets
  `LSUIElement` — not khiin's `LSBackgroundOnly`, because D5's settings window
  must be able to take key focus — plus `TISIntendedLanguage = mul`, matching
  the neutral OS language tag iOS adopted in #494.
- **D3 Controller + composing state** — thin per-session `IMKInputController`;
  process-wide `ComposingSessionCoordinator` owns the single ComposingManager +
  monotonic generation allocator (engine composing state is a process singleton,
  `engine/composing/src/handle.rs:21-56`; per-controller counters would collide
  across clients — Codex must-fix). Effect vocabulary = existing cross-platform
  proto effects; attributed marked text (underline + markedClauseSegment);
  `commitComposition` without `super`; **`recognizedEvents = .keyDown` ONLY**
  (revised at PR3b: IMK sends `commitComposition:` on a click outside the marked
  region only for the exact default keydown mask, `IMKInputController.h:154-157`,
  so a later chord slice must find another route rather than widen this — PR5
  took that route, binding `Ctrl+Shift+,` as a menu key equivalent, see D5);
  logger sink installed once at bootstrap. **Chromium deadlock rule**: never
  query the client synchronously inside `activateServer` (azooKey-Desktop
  regression-test model). **Pinned at PR2 #520** by
  `macos/Tests/.../ActivateServerClientQueryTests.swift`, which drives the real
  `activateServer` with an `IMKTextInput` double that records every read;
  mutation-checked (adding a `selectedRange()` call fails it).
  Note two distinct log scales cross the FFI seam — `set_log_level`'s filter
  (`0=Off … 5=Trace`) and the sink callback's record byte (`0=Error … 4=Trace`).
  A shared "log level" constant is a bug; `RustEngineBridge` names them apart.
- **D4 Candidate window** — borderless non-activating NSPanel + NSHostingView
  (SwiftUI), horizontal single-row; window level = client window level + 1;
  headless navigation model unit-tested, panel is renderer only; positioning =
  pure functions derived from azooKey-Desktop `WindowPositioning` incl.
  multi-display stale-screen fix. NOT `IMKCandidates`.
  **Key bindings decided 2026-08-15 (USER answered the gate below)**: bare 0-9
  always stay text / tone digits and never start a composition · ←/→ move the
  highlight · ↑↓ / PgUp / PgDn page · Space commits the highlighted candidate ·
  Enter commits the literal the marked region shows · Esc cancels · `Ctrl+1…9`
  direct-selects, labelled `⌃1`, never bare `1`. The earlier "1-9 select" line
  is superseded — 1-9 are the numeric tones of TL/POJ (`tai5`).
  **Semantics pinned by the PR4 Codex pre-impl (2026-08-15)**: candidates come
  from `FetchAtPos` only (`dispatch.rs:97` is the sole arm that fills the
  continuous carrier); a candidate commit is `CommitContinuous` only, with
  `consumed_bytes == candidate.consumed_span_end` (`composing.proto:228`);
  Enter keeps the existing `CommitRaw` (`transition.rs:443`) — feeding the
  marked-region text to `SelectSuggestion` instead would double-count the
  nailed prefix, since `select_suggestion_under_continuous` computes
  `nailed_prefix` and then `push_str`s the argument (`transition.rs:724`):
  `台北` nailed + `台北大學` marked → `台北台北大學`. macOS never sends
  `SetSelectedCandidateIndex`: no engine code reads
  `state.selected_candidate_index` (it is stored, echoed, reset — `:576`,
  `:585`), and candidate navigation is a permanent platform-side non-goal
  (`cross-platform-alignment.md §5.1`). Highlight CLAMPs at both ends
  (McBopomofo `HorizontalCandidateController.swift:509`; azooKey wraps — not
  adopted); paging moves the highlight to the first item of the new page, so
  page start / highlight / `⌃1` label always agree.
- **D5 Settings** — in-process SwiftUI window (activate-before-show +
  programmatic Edit menu); opened from IMK `menu()` AND `Ctrl+Shift+,`
  (Cmd+, belongs to the host app). `UserDefaults.standard`; live-read
  `EngineSettingsProvider` port. Minimal defaults provider ships in PR3.
  **Revised at the PR5 Codex pre-impl (2026-08-16)**, four points:
  (1) **`Ctrl+Shift+,` is a `keyEquivalent` on the IMK menu item, not a branch
  in `handle(_:client:)`** — every reference input method binds its shortcuts
  that way (MacishType `InputController.swift:60-67`, McBopomofo
  `InputMethodController.swift:73-84`, azooKey
  `azooKeyMacInputControllerHelper.swift:8-20`) and none classifies a settings
  chord in its key handler. This leaves `ComposingKeyIntent` and the
  keydown-only `recognizedEvents` mask untouched, and avoids classifying a
  chord out of `characters`, which Control rewrites. It also has to be verified
  on an installed copy: the SDK does not document whether a Text Input Menu key
  equivalent fires while the menu is closed and the host is frontmost.
  (2) **No `@ConfigState` port** — SwiftUI `@AppStorage` already re-renders on
  external `UserDefaults` writes, which is the whole requirement for six
  settings in one process; azooKey's notification-backed wrapper
  (`Windows/ConfigState.swift:10-77`) exists for a far larger config surface,
  and MacishType uses plain `@AppStorage` (`GeneralSettingsView.swift:10-13`).
  (3) **No general `WindowManager`** — one window, so one
  `SettingsWindowController`. (4) **`NSApp.activate()` only**: MacishType's
  comment that `activate(ignoringOtherApps:)` is required for an LSUIElement
  input method (`WindowManager.swift:24-38`) does not override the
  deployment-target contract — the SDK marks that variant `API_DEPRECATED`
  ("Use NSApp.activate instead") and `activate()` is macOS 14+. Activation is a
  request, so `orderFrontRegardless()` still follows `makeKeyAndOrderFront`.
  Also settled there: menu built fresh per call (`IMKInputController.h:307-310`),
  the settings item uses the reserved `showPreferences:` selector without
  `super` (the inherited implementation looks for a `preferences.nib` this
  SwiftPM package has none of, `:165-170`), one selector per romanization rather
  than one reading the sender (IMK delivers menu commands through
  `doCommandBySelector:commandDictionary:` with an info dictionary as sender,
  `:283-296`), main menu installed at launch with **no Quit item**, and
  `EngineSettings.defaults` kept as the single source of every default with the
  persistence descriptors referencing it.
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
- **D9 Proto codegen** — `gen-macos-protos` writes to an independent output dir
  `macos/Sources/TaigiInputMethodCore/Engine/Generated/` (committed). **Revised
  2026-08-15 (USER: 「我覺得可以併入到 make build,只是現階段不 release」)**: both
  macOS steps now run inside root `make build`, so no committed artefact can go
  stale behind an engine change and no separate drift rule is needed.
  `make macos-protos` / `make macos-engine` remain as macOS-only shortcuts.
  Building macOS ≠ releasing it — release stays user-gated.
- **D10 Dev loop** — `macos/Makefile`: build → bundle (+ plutil lint,
  codesign --verify, arch check) → install (delete-before-kill, kill by
  bundle-ID/PID, `lsregister -f -R -trusted`, re-login hint only on
  input-mode-set change). Installed-copy guard in AppDelegate; IMKServer held
  in a strong property. **Refined at PR2 #520**: the Makefile targets are
  one-liners over `macos/scripts/{bundle-app,install-app}.sh` — multi-step shell
  does not belong in a recipe. Delete-before-kill became move-aside-before-kill
  so a failed copy can roll back. Processes are stopped by exact executable path
  read from `ps -axo pid=,comm=`; `pkill -f <path>` is an unanchored substring
  match that can hit the shell running the recipe. Bundle validation also checks
  that the Objective-C class names in Info.plist and the swift-bridge FFI entry
  point are actually present in the linked executable, and that nothing pulled
  in an `@rpath` dependency the bundle cannot satisfy. Beware `nm … | grep -q`
  under `set -o pipefail`: `grep -q` closes the pipe, `nm` takes SIGPIPE, and the
  check fails spuriously — read the symbol table into a variable first.

## Phase / PR table

| PR | Phase | Scope | Status |
|---|---|---|---|
| PR0 | Admin | this roadmap + memory topic | this commit |
| PR1 | Engine build surface | darwin toolchain target; `build-macos-xcframework.sh`; `gen-macos-protos`; root Makefile `macos-*` targets. Zero ios/android changes. | **Merged** #514 `6329f152` |
| PR2 | Scaffold + IMK spike | Package.swift; AppDelegate + strong-ref IMKServer + installed-copy guard; echo controller; concrete Info.plist; bundle script + validation; install loop; darwin FFI smoke test | **Merged** #520 `e0938bbd` |
| PR3a | Composing engine seam | bridge composing ops + full 10-effect decode, `lexiconInstall`, dictionary artefacts copied into the bundle, minimal settings provider | **Merged** #522 `433a1f3b` |
| PR3b | Composing IMK integration | ComposingSessionCoordinator, ComposingManager port, effect executor (attributed preedit), controller rewrite | **Merged** #523 `5a7b487c` |
| PR4a | Candidate engine seam | `CommitContinuous` bridge op; manager `fetchCandidates` + effect-backed `commitCandidate`; document-string formatter; pure `CandidateListModel`; tests. No key-table change, no window, no user-visible behaviour | **Merged** #524 `4b155238` |
| PR4b | Candidate IMK integration | `KeyEventSnapshot` named-key discriminator; Space / arrows / paging / `Ctrl+1…9` intents; controller routing; `CandidatePanel` + SwiftUI bar; caret anchor + screen selection; panel owner token; `hidePalettes` | **Merged** #525 `a93fa507` |
| PR5 | Settings + menubar | `SettingsStore` + live-read provider replacing `DefaultEngineSettingsProvider`; SwiftUI form; `SettingsWindowController`; programmatic main menu; IMK `menu()` with `showPreferences:` + TL/POJ items; `Ctrl+Shift+,` as the settings item's key equivalent | PR open #527 |
| PR6 | Custom dict persistence | store: schema v2 + side table + migrator + capacity + derivation; custom_entries injection | Pending |
| PR7 | Custom dict UI | CRUD UI, CSV import/export, backup-exclusion decision (user-gated) | Pending |
| PR8a | Proto coordination ⚠ | `PLATFORM_MACOS` + macOS nextword decide contract + triple-touch regen incl. `make macos-protos` (`rust-migration-policy.md` §4; touches ios/android GENERATED files only; timing user-coordinated) | Pending |
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
| `rust-migration-policy.md` §4 + `CLAUDE.md` stale-binary gate: macOS regen rows | PR1 | yes | no (docs / rules only) |
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

- ~~D4 candidate-selection keys vs numeric tone digits~~ — **CLOSED 2026-08-15**: USER took
  the Codex recommendation whole (bare 0-9 stay text/tone and never start a composition ·
  ←/→ highlight · ↑↓/PgUp/PgDn page · Space commits the highlighted candidate · Enter commits
  the literal · Esc cancels · `Ctrl+1…9` direct-select as `⌃1`). Bindings now live in D4 above
  and bind at PR4b.
- PR8a timing (proto regen window vs concurrent iOS/Android session).
- Intel/x86_64 support (distribution decision).
- Custom-dict Time-Machine/backup-exclusion policy (PR7).
- macOS dogfood acceptance checklist contents (proposed at PR9). **Dogfood cadence decided
  2026-08-15 (USER: 「我想等 desktop 實作完成再 dogfood」)** — device dogfood is not a per-PR
  gate; it runs once as a batch after the desktop IME is implemented.
