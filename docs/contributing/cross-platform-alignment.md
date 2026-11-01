# Cross-Platform Alignment (iOS ↔ Android)

iOS and Android are two shells over one Rust engine. Align them on **intended behaviour**, not on API calls; logic both phones need belongs in the engine (`rust-migration-policy.md`). Observable contracts live in `docs/architecture/behavioral-invariants.md`; the Rust / native ownership inventory in `docs/engine/migration-inventory.csv`.

## 1. Refactors freeze behaviour

A refactor may move code; it does not change what the user sees or what is persisted. Two exceptions, each named in the PR:

### 1a. Emergency

Security hole, reproducible crash, or loss of user data (custom dictionary, frequency, settings). Mirror the fix on the other platform or say why not; if it changes an invariant, update `behavioral-invariants.md` and its test in the same PR. Performance, UX glitches and "while we're here" fixes are not emergencies — they get their own round.

### 1b. Parity correction

A refactor exposes an existing iOS / Android divergence and forces a choice. Correct toward the documented invariant, describe before / after on both platforms, pin the result with a test on both, and keep it out of unrelated refactor work. Not a route for feature work.

### 1c. Shared-core candidates

A file marked `// MARK: - Shared-Core Candidate` (Swift) or `// region Shared-Core Candidate` (Kotlin) is pure platform logic with no platform dependency. The marker is a dependency contract, not a claim that the logic is in the engine. Roster: `native_keep` rows of `migration-inventory.csv`; marker syntax in `ios-architecture.md` §5 / `android-guidelines.md` §1.

| Criterion | Swift | Kotlin |
|---|---|---|
| Framework imports | Foundation only | Kotlin stdlib only (+ the project's `engine/*Bridge.kt`) |
| Engine calls | `RustEngineBridge` allowed | `engine/*Bridge.kt` allowed |
| Singletons | no `*.shared`, no KeyboardKit settings store; settings arrive as `EngineSettingsProvider` | no `object` / companion mutable state, no `Application` / `BuildConfig` reach-in; settings arrive as `EngineSettingsProvider` |
| I/O | no `FileManager`, App Group, `Bundle`, DB | no `File`, DB, SharedPreferences, DataStore |
| Clock | caller passes the time | caller passes `nowMs` |
| Concurrency / side effects | no `NotificationCenter`, `Timer`, `DispatchQueue.main`, `@MainActor` / `@Published` / `ObservableObject` | no coroutines / `Dispatchers`, no `java.util.concurrent` |
| Logging | `LoggerBackend` protocol only | `LoggerBackend` interface only |
| External | no URL building, no network | same |

A new tunable constant in a candidate is mirrored on the other platform in the same PR (§3a).

## 2. Where each concept lives

Same concept, different names — check both columns before porting. Paths: iOS under `ios/Sources/TaigiKeyboard/`, Android under `android/app/src/main/java/com/siansiansu/taigikeyboard/`.

| Concept | iOS | Android |
|---|---|---|
| Engine bridge | `Engine/RustEngineBridge+<Domain>.swift` | `engine/<Domain>Bridge.kt` (+ DTOs in `engine/RustEngineBridge.kt`) |
| Composing executor | `Input/Composing/ComposingManager`, `ComposingDelegate` | `ime/text/composing/ComposingManager`, `ComposingDelegate` |
| Key events | `Actions/ActionHandler` + `+KeyActions` | `ime/text/keyboard/TextInputKeyHandler`, `ime/text/TextInputManager` |
| Candidate taps | `Actions/ActionHandler+Suggestions` | `ime/text/candidates/CandidateClickHandler` |
| Candidate strip | `Candidates/Views/CandidateView` | `ime/text/candidates/CandidateStrip`, `ime/text/smartbar/` |
| Autocomplete | `Candidates/Services/{Taigi,English}AutocompleteService` | `ime/text/candidates/{Taigi,English}AutocompleteService` |
| Next word | `NextWord/NextWordController` | `ime/text/nextword/NextWordController` |
| Dictionary / user data clients | `Lexicon/Services/` | `ime/dictionary/` |
| Settings store | `Settings/SharedSettings` (`SettingsKey`) | `ime/settings/PrefHelper` (`PreferenceKeys`) |
| Engine settings | `Settings/EngineSettings`, `EngineSettingsProvider` | `ime/settings/EngineSettings`, `EngineSettingsProvider` |
| Service graph | `ServiceGraph/CompositionRoot` | `ime/core/CompositionRoot` |
| Keyboard theme | `Theme/` (model) + `Styling/` (render) | `ime/theme/` + `ime/text/keyboard/KeyContent` |
| Keyboard layouts | `Layout/TaigiLayouts.swift` (code tables) | `assets/ime/text/` JSON + `ime/text/layout/LayoutManager` |
| Long-press callouts | `Callouts/` | `ime/popup/` + `extended_popups/*.json` |
| Overlays | `Overlays/` | `ime/text/overlays/` |
| App tabs | `App/Tabs/<Tab>/` (`*Tab`, `*View`) | `ui/tabs/<tab>/` (`*Screen`) + `settings/*Activity` |
| i18n runtime | `Strings/` | `i18n/` |

## 3. Name every divergence

When the same behaviour needs different code, the PR names the divergence and classes it **intentional** (platform-imposed, keep) or **pending** (to reconcile). Silent divergence is what this rule prevents.

Intentional today:

- Key-press haptics: Android drives `Vibrator` directly; iOS cannot bypass the System Haptics master (`behavioral-invariants.md` §36 `INVARIANT_KEYPRESS_FEEDBACK_APP_TOGGLE_GATE`).
- Emoji palette: iOS vendored `ISEmojiView`; Android Compose palette (`docs/ui/emoji.md`).
- Layouts and callouts: Swift tables on iOS, JSON assets on Android (loader modeled on FlorisBoard's).
- Settings UI: SwiftUI `Form` + SF Symbols on iOS; the `ui/components/` row kit + `TaigiIcons` on Android; native toggle colours on each (`ui-style-guide.md` § Switch / Toggle).
- Committed-text deletion: Android sends a raw key only to `TYPE_NULL` editors (`android-guidelines.md` §5); iOS always uses `deleteBackward()`.
- Default keyboard font: iOS `system`, Android `openHuninn` — iOS bundles rare-character fonts Android does not (maintainer 2026-10-12).
- Spacebar: iOS long-press drags the cursor (KeyboardKit); Android horizontal swipe moves the cursor and long-press opens the IME picker (Gboard / FlorisBoard convention). While composing, the Android spacebar ignores the swipe.
- Layout conventions: Android English bottom row carries `,` `.` and the number row has fraction / superscript popups; iOS symbols bottom row has the emoji key where Android has the numeric-pad key. Letter-page bottom rows with a `,` key put it after the emoji key on iOS (`123 😊 ,`, the iOS keyboard's emoji-next-to-123 slot) and before it on Android (`?123 , 😊`, Gboard's order); custom layouts keep the same stored order and each platform renders its own.
- Feedback and pickers: Settings reset gives a haptic on iOS and nothing on Android (a failure toast only); a dictionary search result opens an action sheet on iOS, expands inline on Android.

### 3a. Invariants: constants, tests and docs move together

- Behaviour that must match carries an `INVARIANT_*` label in `behavioral-invariants.md`. A rule both platforms compute belongs in the engine (examples: `IsAttachingPunctuation`, `ExternalLookupDigitForm`, `TaigiWord.sources`); a mirrored platform pair is the fallback for what cannot move.
- A mirrored constant carries `// CROSS-PLATFORM INVARIANT — mirrors <other file>[:<symbol>]. Drift causes silent divergence.`
- The pinning test names its label: a `// INVARIANT_<LABEL> (<doc> §N)` comment above it, or a name prefix (`fn invariant_<label>_…` / `func testINVARIANT_<label>_…` / `fun INVARIANT_<label>_…`). CI `INVARIANT labels` (`tools/invariant_labels.py`) fails when a documented label is in no test; `tools/invariant_labels_pending.txt` only shrinks.
- Changing an invariant constant updates iOS source, Android source, `behavioral-invariants.md` and the test in one PR.

### 3b. Verify a divergence before asserting it

Before writing "iOS does X, Android does Y", grep both sides case-insensitively for `(matches|mirrors) (the )?(iOS|Android)` and `CROSS-PLATFORM INVARIANT`. Such a comment means parity was intended — an observable difference there is a bug, not a design choice.

## 4. Boundaries

### 4.1 Engine non-goals

The engine never owns, and these stay in Swift / Kotlin:

- Candidate UI navigation, layout and styling — the engine returns neutral candidate values (as `khiin-rs/protos/src/command.proto:114` leaves display to the client).
- Platform text-region types (`NSRange`, `UITextDocumentProxy`, `InputConnection`, `EditorInfo`, view types) — protos carry strings, numbers and engine value types only.
- KeyboardKit / Android IME bindings.
- Asset copy and path resolution (`Bundle.main`, `filesDir`, `AssetManager`) — the engine receives a path or handle (`docs/architecture/data-artifacts-portability.md` §7).
