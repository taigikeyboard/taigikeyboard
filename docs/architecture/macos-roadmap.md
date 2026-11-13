# macOS Desktop IME — Roadmap

> **Type**: Reference (shipped; kept as the design record)
> **Keywords**: `macos`, `InputMethodKit`, `IMKit`, `desktop`, `engine reuse`
> **Status**: shipped. Release mechanics: `desktop-release.md`, `macos-release.md`. The composing path, the archive and the shortcut rules later moved into Rust (`macos-desktop-core-roadmap.md`); the user databases moved into the engine (`user-data-engine-roadmap.md` P6). Decisions below that those replaced are marked **Replaced by**.

---

## Goal

A native InputMethodKit app (IMKServer / IMKInputController — not KeyboardKit) over the
shared Rust engine behind a thin native shell: TL + POJ (TPS added by
`desktop-tps-roadmap.md`), SwiftUI settings (macOS 14+), custom-dictionary support,
macOS-conventional shortcuts. Release actions stay maintainer-gated.

## Architecture

```
Host app — IMKTextInput client
        ▲ setMarkedText/insertText        │ keyDown/flagsChanged
┌───────┴──────────────────────────────────▼───────────────────┐
│ TaigiInputController : IMKInputController (thin, per-session) │
├───────────────────────────────────────────────────────────────┤
│ ComposingSessionCoordinator (process-wide; session tokens)    │
│ ComposingBackend → DesktopCoreBridge: key / commit / represent│
│   requests to desktop-core, effects replayed by Swift         │
├──────────────┬─────────────────────┬──────────────────────────┤
│ CandidatePanel│ Settings window     │ Lexicon/UserDataClient  │
│ (D11)         │ (SwiftUI, D5)       │ (user data = engine)    │
├──────────────┴─────────────────────┴──────────────────────────┤
│ macos/RustEngine/RustTaigi.xcframework — one archive:         │
│ macos/crates/taigi-macos-ffi = engine swift-ffi + desktop-core│
└───────────────────────────────────────────────────────────────┘
```

Runtime data: `dictionary.fst` / `dictionary.bin` / `association.bin` /
`syllables.fst` copied from the repo-root `assets/dictionaries/` into
`.app/Contents/Resources` at bundle time (D8); the desktop core installs the lexicon
through `Prepare` (`macos-desktop-core-roadmap.md` P6).
## Design decisions (D1–D11, grounded in code)

- **D1 Engine artifact** — **Replaced by** `macos-desktop-core-roadmap.md` D1 for the
  archive's contents: `engine/scripts/build-macos-xcframework.sh` now builds
  `macos/crates/taigi-macos-ffi` (engine `swift-ffi` as an rlib + the desktop-core
  bridge) into one static library. What stands: the script builds
  `aarch64-apple-darwin` + `x86_64-apple-darwin`, `lipo`s them into one universal
  archive → `macos/RustEngine/RustTaigi.xcframework` + macOS-owned copies of the
  generated swift-bridge Swift (`macos-release.md` § Architectures). `make build` runs the iOS and macOS xcframework scripts together, so the two
  outputs cannot drift behind a `swift-ffi` change. The shared swift-bridge
  post-processing (OUT_DIR discovery, modulemap, `import` injection,
  `@retroactive` patch) lives in `engine/scripts/lib/swift-bridge-artifacts.sh`.
  OUT_DIR discovery asks cargo (`--message-format=json`,
  `build-script-executed`) replaying the caller's exact argv, fail-closed on
  anything but one unique match — newest-by-mtime picked the `panic-injector`
  fingerprint for a cached release build. Also noted: `xcodebuild -create-xcframework`
  orders `AvailableLibraries` nondeterministically, so the committed iOS
  `Info.plist` can churn between otherwise identical builds — compare it
  semantically, do not treat a slice-order flip as a real diff.
- **D2 Project format** — SwiftPM executable + `macos/scripts/bundle-app.sh`
  script-assembled `.app` (khiin-rs osx pattern); NOT an Xcode project (pbxproj
  is user-only, hook-enforced). Info.plist committed with concrete values
  (no Xcode `$(…)` placeholders); bundle ID `com.siansiansu.inputmethod.TaigiKeyboard`
  (mandatory `.inputmethod.` third segment). Mach-register temporary exception
  entitlement required if sandboxed. Ad-hoc codesign for local dev; Developer-ID
  signing + notarization ship as `make macos-release`
  (`macos/scripts/release-app.sh`). azooKey-Desktop `pkgbuild.sh` was the
  pipeline reference; what was and was not adopted from it, and why, lives in
  `docs/architecture/macos-release.md`.
  The package is a library (`TaigiInputMethodCore`) plus
  a thin executable, not a single executable target, because `swift test` needs
  an importable module. Generated `.pb.swift`
  therefore live under `Sources/TaigiInputMethodCore/Engine/Generated/`. The
  swift-bridge wrappers compile as their own `RustTaigiSwift` target pinned to
  Swift 5 language mode (generated sources cannot be annotated for Swift 6
  strict concurrency); everything above it is Swift 6 mode.
  `macos/scripts/lib/bundle-identity.sh` is the only reader of Info.plist and
  the only place the assembled bundle's path is named. Info.plist omits
  `ComponentInputModeDict` (TL/POJ are an app setting per D5) and sets
  `LSUIElement` — not khiin's `LSBackgroundOnly`, because D5's settings window
  must be able to take key focus — plus `TISIntendedLanguage = mul`, matching
  the neutral OS language tag iOS uses, and
  `TICapsLockLanguageSwitchCapable` with a `Hant`-only character repertoire so
  macOS classes the source as non-Latin and lets Caps Lock switch it to and
  from ABC (Info.plist documents the probe behind that).
- **D3 Controller + composing state** — **Replaced by** `macos-desktop-core-roadmap.md`
  D3 for the composing state: the Swift ComposingManager is gone; `ComposingBackend`
  sends session requests to desktop-core and replays its effects. What stands: thin
  per-session `IMKInputController`; process-wide `ComposingSessionCoordinator` owning
  the session tokens (engine composing state is a process singleton (engine composing state is a process singleton,
  `engine/composing/src/handle.rs:21-56`; per-controller counters would collide
  across clients). Effect vocabulary = existing cross-platform
  proto effects; attributed marked text (underline + markedClauseSegment);
  `commitComposition` without `super`; **`recognizedEvents = .keyDown` ONLY**
  (IMK sends `commitComposition:` on a click outside the marked
  region only for the exact default keydown mask, `IMKInputController.h:154-157`,
  so chords bind as menu key equivalents rather than widen this, see D5);
  logger sink installed once at bootstrap. **Chromium deadlock rule**: never
  query the client synchronously inside `activateServer` (azooKey-Desktop
  regression-test model). Pinned by
  `macos/Tests/.../ActivateServerClientQueryTests.swift`, which drives the real
  `activateServer` with an `IMKTextInput` double that records every read;
  mutation-checked (adding a `selectedRange()` call fails it).
  Note two distinct log scales cross the FFI seam — `set_log_level`'s filter
  (`0=Off … 5=Trace`) and the sink callback's record byte (`0=Error … 4=Trace`).
  A shared "log level" constant is a bug; `RustEngineBridge` names them apart.
- **D4 Candidate window** — the panel itself is **replaced by** D11; the key bindings
  below stand (their classification now lives in desktop-core,
  `macos-desktop-core-roadmap.md` P14). Window level = client window level + 1;
  positioning = pure functions derived from azooKey-Desktop `WindowPositioning` incl.
  multi-display stale-screen fix. NOT `IMKCandidates`.
  **Key bindings**: a bare digit is always text / a tone digit (`tai5`) and never
  starts or picks · ←/→ move the highlight · ↑↓ / PgUp / PgDn page · Space commits the
  highlighted candidate · Enter commits the literal the marked region shows · Esc
  cancels · slots 1–9 are picked with the chosen slot-key set (`CandidateSlotKeySet`:
  bare `q w d f z x v y ;` by default, or `⇧` / `⌃` / `⌥` + 1…9, one live at a time),
  drawn per slot. Candidates come from `FetchAtPos` only; a candidate commit is
  `CommitContinuous` with `consumed_bytes == candidate.consumed_span_end`; Enter
  commits raw. Candidate navigation is a permanent platform-side non-goal
  (`cross-platform-alignment.md §4.1`). Highlight CLAMPs at both ends
  (McBopomofo `HorizontalCandidateController.swift:509`; azooKey wraps — not
  adopted); paging moves the highlight to the first item of the new page, so
  page start / highlight / slot 1 always agree.
- **D5 Settings** — in-process SwiftUI window (activate-before-show +
  programmatic Edit menu); opened from IMK `menu()` AND a recorded chord (default
  `⌃⌘S`, `ShortcutActions.swift:31-33`; Cmd+, belongs to the host app).
  `UserDefaults.standard`, read live; the key-path subset is sent to desktop-core as a
  snapshot (`macos-desktop-core-roadmap.md` D5). Four points:
  (1) **The settings chord is a `keyEquivalent` on the IMK menu item, not a branch
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
- **D6 Custom dict** — **Replaced by** `user-data-engine-roadmap.md` P6: the store is
  the engine's (`engine/userdata`); macOS sends `OpenUserData` at launch, the engine
  reads the entries itself inside `FetchAtPos` (`custom_entries` is `reserved`), and the
  Custom Dictionary page goes through `Lexicon/UserDataClient.swift`. What stands: the
  databases live under `~/Library/Application Support/<bundle-id>/` inside Time Machine
  scope; CSV via NSOpen/SavePanel; seed parity with iOS (`gâu-tsá/𠢕早`,
  `tsia̍h-pá--buē/食飽未`).
- **D7 User freq / nextword** — storage **replaced by** `user-data-engine-roadmap.md`
  P6 / P9b (both learning databases are the engine's; the engine counts each candidate
  commit it resolves and writes the associations its decisions record; the manager only
  reports handshakes through `NextWord/NextWordPort.swift`). The decide arms stand:
  - macOS's decide arms are **designed, not copied** (as this entry always
    required). `split_compound` splits on whitespace only — a hyphen is inside
    a macOS word (`tâi-gí`, `hōo--guá`), a space is only emitted between
    segments the walker calls separate words. `compound_association_pairs`
    emits nothing when the Hanji and romanization sides split into different
    counts, rather than padding an empty TL onto a real word (Core Principle
    #7). `is_noise_text` is "contains no letter" rather than a third
    punctuation table.
  - Learning is **write-and-rank for frequency, write-only for associations**:
    macOS has no idle candidate surface, and putting one on screen means
    intercepting keys in a state where this input method forwards everything
    to the host. Recording now means a future prediction surface starts with
    real history. No context timer — the engine's strict 10 s association
    window already fences recording, and `is_showing` is never true here.
  - `user_association.db` ships schema v6, one column wider in its unique key
    than the old iOS/Android v5 shape, so multi-reading characters stay separate on the
    PREVIOUS word too; iOS and Android now use v6 as well.
  - Learning databases are NOT excluded from Time Machine, unlike the iOS
    backup exclusion (`behavioral-invariants.md` §29): that decision was about
    user data leaving the device through iCloud.
- **D8 Dictionary artifacts** — read from the repo-root `assets/dictionaries/` (read-only)
  at bundle time with fail-fast existence/non-empty validation. That directory is
  the single committed copy all four platforms package from; `dictionary/build/deploy.sh`
  writes it.
- **D9 Proto codegen** — `gen-macos-protos` writes to an independent output dir
  `macos/Sources/TaigiInputMethodCore/Engine/Generated/` (committed). Both macOS steps
  run inside root `make build` only — no per-platform shortcut targets: regenerating one
  platform's artefacts from shared sources is what leaves the others stale against the
  same commit.
  Building macOS ≠ releasing it — release stays user-gated.
- **D10 Dev loop** — `macos/Makefile`: build → bundle (+ plutil lint,
  codesign --verify, arch check) → install (delete-before-kill, kill by
  bundle-ID/PID, `lsregister -f -R -trusted`, re-login hint only on
  input-mode-set change). Installed-copy guard in AppDelegate; IMKServer held
  in a strong property. The Makefile targets are
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

- **D11 Candidate window — MacishType port** — a port of
  MacishType's candidate window (`references/MacishType/macos/MacishType/`,
  MIT, © 2026 Luke Chang; ported files carry the attribution in their header)
  so the panel matches the native input-method look: system accent colour
  (Multicolour → host app's `NSAccentColorName` when it resolves, else system),
  three layouts (`horizontal` width-packed pages · `vertical` scrolling ·
  `expandable` one row + chevron grid; `candidateLayout`, default
  `expandable`), light/dark following the system appearance, Sequoia
  (`NSVisualEffectView`, 6pt corners, row bar) vs Tahoe (`NSGlassEffectView`,
  capsule, inset pill) chrome resolved from the running OS. Navigation moved
  from the controller-side `CandidateListModel` into the panel behind the
  `CandidatePresenter` seam (`show` / `navigate` / `selectedCandidateIndex` /
  `candidateIndex(forSlot:)` / `hide`), every entry owner-token guarded; the
  panel is the single selection authority, `isShowingCandidates` stays
  `!fetchedCandidates.isEmpty`. Pure geometry (`HorizontalPageLayout`,
  `ExpandedGridLayout`, `CandidatePanelPositioning`, `ScreenLookup`) is
  value-typed and unit-tested; AppKit layout / scroll / animation are
  dogfood-gated. Presentation keys (`candidateLayout`,
  `candidateAppearanceMode`, `candidateTextSize`, `fontType`) sit beside the
  store's macOS-only keys, not in `EngineSettings`.
  `candidateTextSize` is the window's one size knob (`candidateWindowSize` is retired).
  One private-API carve-out survives, guarded by `responds(to:)`: upstream's
  `_adaptiveAppearance` KVC on `NSGlassEffectView`. The
  `candidateWindowStyle` override was removed (asymmetric below
  macOS 26) before any release shipped it. `maxDisplayCandidates =
  200` is kept as a stated decision.

  Deliberately NOT carried over:

  | Upstream thing | Why not |
  |---|---|
  | `Candidate.annotation` + vertical column-alignment / top-3 width heuristic | Taigi candidates are one composed string; widths are measured eagerly over the displayed list |
  | `Candidate.payload` | absolute index into the controller's retained array is the identity |
  | `-1` no-selection sentinel, "first arrow reveals", empty confirm, `initialHighlight < 0` | the IME always selects index 0 on every fresh fetch; the suspended-selection state is unreachable |
  | `indexLabels` + `candidateIndex(for:)` / `navigationIntent(...)` | key classification stays in `ComposingKeyIntent`; slot keys are a fixed nine (`CandidateSlotKeySet`) |
  | upstream placement (`topLeftPoint`, composition-start/end fallback, screen lookup) | `CandidatePanelPositioning` + `ScreenLookup` are pure, tested, and clamp oversized panels |
  | `windowEffectiveAppearance` client-appearance read | undocumented selector; system appearance instead |
  | double-click commit + `candidateConfirmed` delegate | a mouse commit is a new asynchronous engine entry needing owner-token + generation + live-client validation; a click selects, only keys commit |

  References borrowed: MacishType `CandidateWindow.swift:150-300` (panel owns
  candidates + selection), `MacishHorizontalBasePanel.swift:12-34`
  (width-packed page), `MacishBasePanel.swift:129-177` (accent + luminance
  clamp), `MacishVerticalPanel.swift:255-330` (lazy vertical render), `MacishHorizontalExpandablePanel.swift:99-220`;
  McBopomofo `HorizontalCandidateController.swift:509` (highlight clamps at
  both ends — the D4 no-wrap rule).

## Best practices alignment (references)

Entry point per repo policy: `docs/references/mainstream-ime-comparison.md`
(cards #2 azooKey-Desktop, #11 McBopomofo, #12 khiin-rs, #20 MacishType).
Key borrowings:

- khiin-rs osx — SwiftPM → script-assembled .app; IME Info.plist key set; darwin staticlib build.
- MacishType — install Makefile (lsregister + mode-dict diff), action-list interpreter, attributed marked text, in-process settings, installed-copy guard.
- azooKey-Desktop — Chromium activateServer deadlock rule + regression test, pure `WindowPositioning` + multi-display fix, `@ConfigState` wrapper, sandbox mach-register entitlement, notarization pipeline.
- McBopomofo — recognizedEvents mask, mature IMK lifecycle patterns.
- iOS in-repo — ComposingManager contract, EngineSettingsProvider live-read, custom-dict schema v2 invariant.

**Deliberately not adopted**: khiin settings helper app · Xcode project (pbxproj user-only) ·
`IMKCandidates` · macOS-local action enum · in-IME English mode ·
third committed dictionary copy · shielding window level ·
azooKey out-of-process XPC daemon (+ its async queue/idempotency machinery — our
FFI is in-process sync) · OS-level `ComponentInputModeDict` modes for TL/POJ
(settings toggle instead) · azooKey NSTableView candidate UI · `/Library/Input
Methods` sudo install.
