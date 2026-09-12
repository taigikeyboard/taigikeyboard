# macOS Desktop IME — Roadmap

> **Type**: Reference (shipped; kept as the design record)
> **Keywords**: `macos`, `InputMethodKit`, `IMKit`, `desktop`, `engine reuse`
> **Status**: shipped — PR0–PR13 merged 2026-08-17; candidate-window port 2026-08-18 (D11); released in desktop v3.6.7 (first macOS + Windows desktop release) and v3.6.8 (`changelog/desktop-v3.6.7.md`, `changelog/desktop-v3.6.8.md`). Device dogfood runs as one batch per USER 2026-08-15 (「我想等 desktop 實作完成再 dogfood」); the batch is still open. Release mechanics: `desktop-release.md`.
> **Session memory**: project memory `project_macos_ime.md` (Claude auto-memory)
> **Plan provenance**: Phase-0 research + Codex pre-impl design review (ANALYSIS-ONLY, 2026-08-15) — FFI-reuse / SwiftPM-bundle / platform_id-deferral all confirmed; generation-ownership, proto-gen isolation, PR sizing, bundle-metadata cautions incorporated.

---

## Goal

Add macOS (one of the two desktop platforms beside Windows TSF; iOS and Android
are the mobile pair) to prove the shared Rust engine reuses cleanly behind a
thin native shell. Native InputMethodKit app
(IMKServer / IMKInputController — NOT KeyboardKit), TL + POJ only (no TPS),
modern SwiftUI settings (macOS 14+), custom-dictionary support, macOS-conventional
shortcuts.

**Hard constraints**: release actions stay user-gated. The
"no modification of `ios/`, `android/`, `i18n/`, `content/`; `engine/` changes
purely additive" rule came from a concurrent iOS/Android session — **lifted
2026-08-15 (USER: 「目前沒有並行 session」)**. Shared-surface changes are now
allowed on their merits; the coordination register below stays as the record of
what each PR touches. PR8a's proto regen landed in #528 (2026-08-17).

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
│ macos/RustEngine/RustTaigi.xcframework (universal arm64 +     │
│ x86_64, new additive script) + macOS-owned generated Swift    │
│ copies                                                        │
└───────────────────────────────────────────────────────────────┘
```

Runtime data: `dictionary.fst` / `dictionary.bin` / `association.bin` /
`syllables.fst` read from the repo-root `dictionaries/` at bundle-assembly time
(read-only, fail-fast validated) into `.app/Contents/Resources`; absolute paths
passed via `lexiconInstall`.

## Design decisions (D1–D11, grounded in code)

All grounded in actual code reads (actual code reading); full citations in the
Phase-0 plan and project memory `project_macos_ime.md` (Claude auto-memory).

- **D1 Engine artifact** — engine `swift-ffi` crate reused UNCHANGED (4 fns,
  bytes-in/bytes-out protobuf, platform-neutral; `engine/swift-ffi/src/lib.rs:41-54`).
  New additive `engine/scripts/build-macos-xcframework.sh` builds
  `aarch64-apple-darwin` + `x86_64-apple-darwin`, `lipo`s them into one universal
  archive → `macos/RustEngine/RustTaigi.xcframework` + macOS-owned copies of
  generated `RustTaigi.swift` / `SwiftBridgeCore.swift`. iOS artifacts untouched.
  (Shipped arm64-only at D1; x86_64 added 2026-08-23 — see
  `docs/architecture/macos-release.md` § Architectures.)
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
  entitlement required if sandboxed. Ad-hoc codesign for local dev; Developer-ID
  signing + notarization ship as `make macos-release`
  (`macos/scripts/release-app.sh`). azooKey-Desktop `pkgbuild.sh` was the
  pipeline reference; what was and was not adopted from it, and why, lives in
  `docs/architecture/macos-release.md`.
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
  the neutral OS language tag iOS adopted in #494, and
  `TICapsLockLanguageSwitchCapable` with a `Hant`-only character repertoire so
  macOS classes the source as non-Latin and lets Caps Lock switch it to and
  from ABC (Info.plist documents the probe behind that; USER 2026-08-28).
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
  direct-selects, never bare `1`. The earlier "1-9 select" line is superseded —
  1-9 are the numeric tones of TL/POJ (`tai5`). The chord is not drawn beside
  the candidate (§42, USER 2026-08-21); it addresses the visible slots.
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
  (`cross-platform-alignment.md §4.1`). Highlight CLAMPs at both ends
  (McBopomofo `HorizontalCandidateController.swift:509`; azooKey wraps — not
  adopted); paging moves the highlight to the first item of the new page, so
  page start / highlight / slot 1 always agree.
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
  FULL iOS contract port (schema v2 + `custom_search_key` side table +
  repository + same-transaction 30000-cap guard + derivation via existing engine
  FFI). **No migrator** — macOS never shipped a v1 schema (revised 2026-08-17;
  same dead-code rationale as `UserFrequencyStore`). Entries ride
  `FetchAtPos.custom_entries` — zero new FFI. CSV via NSOpen/SavePanel.
  Backup-exclusion policy decided at PR12: inside Time Machine scope. Seed parity with iOS: two
  default entries (`gâu-tsá/𠢕早`, `tsia̍h-pá--buē/食飽未`). Re-activated
  2026-08-17 as PR11 (store) + PR12 (UI).
- **D7 User freq / nextword** — **CLOSED at PR8a+PR9 (#528)**. PR3–PR7 ran the
  FetchAtPos carrier's neutral phase only, with `platform_id = 0`, which was
  safe because the only validator is nextword
  (`engine/nextword/src/decide.rs:55-63`) and macOS sent no nextword requests.
  Now settled:
  - macOS's decide arms are **designed, not copied** (as this entry always
    required). `split_compound` splits on whitespace only — a hyphen is inside
    a macOS word (`tâi-gí`, `hōo--guá`), a space is only emitted between
    segments the walker calls separate words. `compound_association_pairs`
    emits nothing when the 漢字 and romanization sides split into different
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
    than the iOS/Android v5 shape, so 一字多音 stay separate on the PREVIOUS
    word too. The same gap on iOS and Android is real and unfixed — correcting
    it there is a migration over existing user data.
  - Learning databases are NOT excluded from Time Machine, unlike the iOS
    backup exclusion (`behavioral-invariants.md` §29): that decision was about
    user data leaving the device through iCloud.
- **D8 Dictionary artifacts** — read from the repo-root `dictionaries/` (read-only)
  at bundle time with fail-fast existence/non-empty validation. That directory is
  the single committed copy all four platforms package from; `dictionary/build/deploy.sh`
  writes it.
- **D9 Proto codegen** — `gen-macos-protos` writes to an independent output dir
  `macos/Sources/TaigiInputMethodCore/Engine/Generated/` (committed). **Revised
  2026-08-15 (USER: 「我覺得可以併入到 make build,只是現階段不 release」)**: both
  macOS steps now run inside root `make build`, so no committed artefact can go
  stale behind an engine change and no separate drift rule is needed.
  **Revised 2026-08-26 (USER: 「我覺得可以一起,讓指令簡單」)**: the `make
  macos-protos` / `make macos-engine` shortcuts PR1 added are gone. They only
  ever saved the 3-5 min mobile rebuild, and regenerating one platform's
  artefacts from shared sources is what leaves the others stale against the same
  commit — the hazard `make build` exists to prevent.
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

- **D11 Candidate window — MacishType port (2026-08-18, three PRs
  `bac7a983` → `1849349f` → `ba682ceb`; Codex pre-impl 2026-08-18)** — the
  one-row SwiftUI bar in a borderless `NSPanel` was replaced by a port of
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
  `candidateAppearanceMode`, `candidateWindowSize`, `candidateTextSize`,
  `fontType`) sit beside the store's macOS-only keys, not in `EngineSettings`.
  One private-API carve-out survives, guarded by `responds(to:)`: upstream's
  `_adaptiveAppearance` KVC on `NSGlassEffectView`. The
  `candidateWindowStyle` override PR3 shipped was removed (asymmetric below
  macOS 26) and is swept by `RetiredSettingsCleanup`. `maxDisplayCandidates =
  200` is kept as a stated decision.

  Deliberately NOT carried over (each confirmed by the Codex pass):

  | Upstream thing | Why not |
  |---|---|
  | `Candidate.annotation` + vertical column-alignment / top-3 width heuristic | Taigi candidates are one composed string; widths are measured eagerly over the displayed list |
  | `Candidate.payload` | absolute index into the controller's retained array is the identity |
  | `-1` no-selection sentinel, "first arrow reveals", empty confirm, `initialHighlight < 0` | the IME always selects index 0 on every fresh fetch; the suspended-selection state is unreachable |
  | `indexLabels` + `candidateIndex(for:)` / `navigationIntent(...)` | key classification stays in `ComposingKeyIntent`; slot keys are a fixed nine (`CandidateSlotKeySet`, shipped `q w d f z x v y ;` since 2026-08-28; drawn per #604) |
  | upstream placement (`topLeftPoint`, composition-start/end fallback, screen lookup) | `CandidatePanelPositioning` + `ScreenLookup` are pure, tested, and clamp oversized panels |
  | `windowEffectiveAppearance` client-appearance read | undocumented selector; system appearance instead |
  | double-click commit + `candidateConfirmed` delegate | a mouse commit is a new asynchronous engine entry needing owner-token + generation + live-client validation; a click selects, only keys commit |

  References borrowed: MacishType `CandidateWindow.swift:150-300` (panel owns
  candidates + selection), `MacishHorizontalBasePanel.swift:12-34`
  (width-packed page), `MacishBasePanel.swift:129-177` (accent + luminance
  clamp), `MacishVerticalPanel.swift:255-330` (lazy vertical render — later
  extended by #47 on-demand rows), `MacishHorizontalExpandablePanel.swift:99-220`;
  McBopomofo `HorizontalCandidateController.swift:509` (highlight clamps at
  both ends — the D4 no-wrap rule).

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
| PR5 | Settings + menubar | `SettingsStore` + live-read provider replacing `DefaultEngineSettingsProvider`; SwiftUI form; `SettingsWindowController`; programmatic main menu; IMK `menu()` with `showPreferences:` + TL/POJ items; `Ctrl+Shift+,` as the settings item's key equivalent | **Merged** #527 `62ff7d40` |
| PR6 | Custom dict persistence | **re-activated 2026-08-17 (USER), folded into PR11 below** (supersedes 2026-08-16「先不實作」). No migrator — macOS never shipped a v1 schema | folded → PR11 |
| PR7 | Custom dict UI | **re-activated 2026-08-17 (USER), folded into PR12 below.** Backup-exclusion decided at PR12 (inside TM scope) | folded → PR12 |
| PR8a+PR9 | Proto coordination + freq/nextword ⚠ | **merged into ONE PR at USER instruction.** `PLATFORM_MACOS` + triple-touch regen (`rust-migration-policy.md` §4; ios/android GENERATED files only) · macOS nextword decide contract · `user_frequency.db` + `user_association.db` · phase-2 boosted fetch · learning settings | **Merged** #528 `652eb28d` |

### Settings v2 + 詞庫 page track (2026-08-17, USER-approved plan)

Goals: (a) 詞庫 settings page with FULL iOS/Android Tab3 parity — 資料管理 4 sub-pages
(自訂詞庫/詞頻/詞關聯/備份還原) + 24 dictionary source toggles + live dictionary search
with external lookup; (b) native macOS preferences style — NSTabViewController toolbar
tabs ([⚙ 一般] [📖 詞庫]), resizable window; (c) Magnet-style configurable shortcuts
(sindresorhus/KeyboardShortcuts lib — first non-Apple dependency, USER-approved) for
開啟設定視窗 (default `Ctrl+Shift+,`) / TL↔POJ 切換 / 候選開關 (漢羅對調·漢羅並列; the 顯示當咧拍的字 SHORTCUT stays retired 2026-09-02 — its 一般-pane settings row came back 2026-09-03, §34).
Strings stay Traditional-Chinese literals (PR5 convention). Zero new FFI — all ops already
in the generated protos. PR sizing ~600-1000 LOC each (USER chose fewer/larger PRs
2026-08-17 over the 200-500 default; session-per-PR overhead).

| PR | Phase | Scope | Status |
|---|---|---|---|
| PR10 | Window restyle + shortcuts | NSTabViewController `.toolbar` tabs + resizable window; `GeneralSettingsView` + `DictionarySettingsPane` shell; KeyboardShortcuts dep + `ShortcutActions` registry + recorder UI; IMK menu key equivalent read from the stored chord | **Merged** #534 `43cd8d6a` |
| PR11 | 詞庫 data layer (no UI) — **Merged** #537 `83bfa549` | 24 source-toggle keys (iOS `SharedSettings.swift:53-84` spellings) + `DictionarySourceToggles` in `EngineSettings`; `lexiconDictionaryFilters` bridge + `FetchAtPos.enabled_sources_bitmask` + `custom_entries` wiring; `CustomDictionaryStore` (schema v2 + side table, cap 30000, no migrator) + phonetics derive bridge; `UserDataDatabase.perform` + freq/assoc list/delete/clear/batchImportMerge APIs; CSV codecs third mirror. ⚠ intended behavior change: defaults exclude iTaigi/台日/台華/植物/異體/khiin from continuous candidates (was sentinel all-on) | done |
| PR12 | 詞庫 tab UI — **Merged** #538 `29bc5ada` | toggles view (MOE + kautian 11 + other + supplement); custom-dict CRUD + CSV + seed; 詞頻/詞關聯 viewers (limit 100, pair-key delete, clear, CSV); 備份還原 `.taigi` (BackupService JSON v2, `platform:"macos"`); NSOpen/NSSavePanel helpers. Custom-dict Time-Machine policy: stays inside TM scope (plan default, unchanged) | done |
| PR13 | 辭典搜尋 — **Merged** #539 `88e0b223` | lexicon search bridge (searchWithSources/searchByHanzi/isHanzi) + tlToPoj; `DictionarySearchService` (toggle snapshot per query, kautian-first sort, badge retag, custom-dict prefix merge); search UI (300ms debounce, field at TOP of 詞庫 tab — mac idiom) + 萌典教典/ChhoeTaigi external links via NSWorkspace.open | done |

**TRACK COMPLETE 2026-08-17** — all four merged, `swift test` 329/329 on main (was 170 before PR10). Nothing from this track is outstanding except the batched device dogfood and the two open items below.

Open items this track produced:
- `lexicon.proto` documents `DEV` as always-on in three places (`:231`, `:239`, `:500`); the toggle shipped and made that false. The comment lands in the committed generated trees, so correcting it is a regen round touching iOS + Android.
- **iOS/Android carry two behaviours macOS now corrects**, both classified deferred: the all-off toggle state re-enables every dictionary there (macOS sends a no-sources mask), and a v2 `.taigi` restore folds POJ→TL over readings already declared canonical TL (macOS folds only v1).
- Custom-dictionary Time Machine policy stayed the plan's default — inside Time Machine's scope, like the other two user databases — and is the USER's to change.

Dependencies: PR10 ⊥ PR11 (parallelizable); PR12 needs PR10+PR11; PR13 needs PR11.
Per-PR gate: `swift test` only (no engine change). Codex sandwich each PR (pre-impl
mandatory). doc-lookup gates: KeyboardShortcuts · NSTabViewController.TabStyle.toolbar ·
NSWindow.toolbarStyle · NSOpen/NSSavePanel in LSUIElement · UTType for `.taigi` ·
NSWorkspace.open.

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

## Settings pane roster (current)

一般 · 外觀 · 快捷鍵 · 自訂詞庫 · 辭典管理.

The 詞頻紀錄, 詞關聯紀錄 and 備份還原 panes that PR12 shipped were removed (USER
2026-08-24). Both learning tables are self-trimming — `LearningCapacity` caps
them at 20000 / 50000 rows and evicts least-used-first, so they cannot grow
without bound (a row ceiling, not a byte one: SQLite does not shrink the file) —
and the decision was that learned records are not the user's to administer. What replaced the three panes is one destructive button in 一般,
清除學習紀錄, which empties both tables at once.

Consequences, accepted with the decision: macOS no longer reads or writes
`.taigi`, so an iOS/Android backup cannot be imported here and learned data does
not move between Macs. The custom dictionary keeps its own CSV import/export,
and this Mac's databases stay inside Time Machine scope. The `.taigi` format
itself is unchanged — iOS and Android still read and write it.

`RetiredSettingsCleanup` sweeps a stored selection pointing at one of the three
retired panes, and clears the two recording toggles they carried so a stored
`false` cannot outlive the UI that set it.

## User-gated open items

- ~~D4 candidate-selection keys vs numeric tone digits~~ — **CLOSED 2026-08-15**: USER took
  the Codex recommendation whole (bare 0-9 stay text/tone and never start a composition ·
  ←/→ highlight · ↑↓/PgUp/PgDn page · Space commits the highlighted candidate · Enter commits
  the literal · Esc cancels · `Ctrl+1…9` direct-select as `⌃1`). Bindings now live in D4 above
  and bind at PR4b. **Superseded 2026-08-28** (kept as the decision of record): the shipped
  slot keys are now the bare keys `q w d f z x v y ;` (slots 1–9); `⇧1…9` / `⌃1…9` / `⌥1…9`
  are the other three picker choices, one live at a time (`CandidateSlotKeySet`), and the only
  way to pick: the bare-digit-after-a-tone rule (#602, 2026-08-24) and the `↓` latch (#610) are
  both retired 2026-08-28; a bare digit is always the tone, and the window always draws the
  chosen set.
- ~~PR8a timing~~ — resolved: regen landed with #528 (2026-08-17), no concurrent session.
- ~~Custom-dict Time-Machine/backup-exclusion policy~~ — resolved at PR12 with the plan default: all three macOS databases stay inside Time Machine scope (the iOS iCloud exclusion in `behavioral-invariants.md` §29 was about data leaving the device). Still the USER's to change.
- ~~macOS dogfood acceptance checklist contents~~ — resolved: per-feature `Sn` items live in `dogfood-checklist.md`. **Dogfood cadence decided
  2026-08-15 (USER: 「我想等 desktop 實作完成再 dogfood」)** — device dogfood is not a per-PR
  gate; it runs once as a batch, and that batch is still open.
