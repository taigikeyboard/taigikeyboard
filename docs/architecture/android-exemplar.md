# Android Exemplar — Phase II Alignment Target

**Status**: authored 2026-04-20 as Phase II A8-skeleton deliverable. Companion to
`ios-exemplar.md` (Phase I G8, PR #134). This doc is intentionally **short** and
documents *only* Android-specific deviations from the iOS exemplar; shared conventions
(marker syntax, ViewModel shape, naming, access levels, pure-logic criteria) are defined
once in `ios-exemplar.md` / `rules/android-guidelines.md` and referenced from here.

## 1. Purpose

`ios-exemplar.md` is the cross-platform alignment target. Android converges toward that
shape through Phase II rounds A0–A10 per `android-state-audit.md` §7. Where Android
*must* diverge from iOS — for platform, framework, or language-semantic reasons — this
doc records the divergence so future rounds do not re-litigate it.

## 2. Android-specific deviations from the iOS exemplar

### 2.1 Composition scope

iOS composes the engine graph from the keyboard extension entry point + a shared-app
extension wiring hook (`TaigiKeyboard/KeyboardRoot.swift`). Android has two live scopes:

- **IME service graph** — `TaigiKeyboard : LifecycleInputMethodService`. Created in
  `onCreate`, torn down in `onDestroy`. Owns per-session state (composing, candidates,
  next-word history).
- **App-tab graph** — currently ephemeral: `PrefHelper(context)` is constructed per
  `Activity`, no `Application` subclass exists. **Decision 2026-04-20 (Claude + Codex
  joint, auto-mode)**: `TaigiKeyboardApplication : Application` lands in A7, bundled
  with the IME manager-graph unwind — see `android-state-audit.md` §8 #1. Until A7
  closes, `CompositionRoot.shared(context)` continues as the process-wide service
  holder (A1 deliverable) that both scopes reach into. Post-A7: `Application.onCreate`
  owns the warmup chain (`prefs.warmUp()` + `migrateFromSharedPreferences` + per-service
  `init(context)`); `TaigiKeyboard.onCreate` drops those calls (services are idempotent
  per A1 so a duplicate call during migration stays safe).

Engine-layer code MUST receive its dependencies via constructor injection from whichever
scope constructs it. No `.INSTANCE` reach-ins. See `rules/android-guidelines.md` §4.

### 2.2 Settings live-read

Android stores settings in DataStore behind `PrefHelper.cachedPrefs`. iOS stores them in
`UserDefaults` / `SharedSettings`. `ios-exemplar.md` §3 warns about the `val x = prefs.getX()`
snapshot anti-pattern: capturing a setting value at engine-initialization time means the
engine never sees user edits made from the Settings tab.

A2 landed the Android mirror: `EngineSettings` interface + `EngineSettingsProvider.current`
returning `this`. Each field on `EngineSettings` is a `get()` property that re-reads
`cachedPrefs` on every access. Concrete rule, enforced at PR-review time:

- Inside a shared-core candidate that receives `EngineSettings`, read through the property
  at the point of decision — do not capture into a local `val` at constructor time.
- `EngineSettingsProvider.current` returns a live view, not a frozen snapshot. If a caller
  needs multi-field consistency within one operation, it captures a local copy explicitly.

### 2.3 Coroutines and threading

Shared-core candidates on Android MUST NOT import `kotlinx.coroutines.*`, `Dispatchers.*`,
or own a `CoroutineScope`. Async orchestration lives in the platform wrapper exclusively.
This matches iOS (no `Task {}` or `DispatchQueue` inside shared-core Swift files).

Concrete platform-side conventions:

- `TaigiKeyboard.serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)` —
  cancelled in `onDestroy`.
- `ViewModel.viewModelScope` — cancelled automatically by AndroidX.
- `Dispatchers.IO` for DB / file / network; `Dispatchers.Default` for pure CPU.
- Clock injection: shared-core callers that need the wall clock take `nowMs: Long` as
  a parameter. No `System.currentTimeMillis()` inside candidate files.
- Long-running timers (context-timeout, association-timeout) use `kotlinx.coroutines.delay`
  from the platform wrapper — a straight port of the iOS `Timer.scheduledTimer` pattern
  from `NextWordController`. See `nextword-engine-boundary.md` §13 (Android binding).

### 2.4 InputConnection binding

`InputConnection.finishComposingText()` on Android **commits** the current composing
region by default — it is NOT a clear-without-commit primitive. The platform wrapper
MUST zero the composing region with `ic.setComposingText("", 1)` before calling
`ic.finishComposingText()` when the intent is `clearPreeditWithoutCommit`. Similarly,
`commitTextReplacingPreedit` atomically replaces the composing region via a single
`ic.commitText(text, 1)` — do NOT pre-finish or the preedit is double-committed.

Full binding table and test hooks live in `composing-state-boundary.md` §11.2 (Android
Binding Addendum). A4-impl is the round that lands the `ComposingState` extract and
honors these rules in code.

### 2.5 Package layout (deferred rename)

iOS target layout (`ios-exemplar.md` §5.1): `Lexicon/`, `NextWord/`, `Phonetics/`,
`Settings/` as sibling package trees. Android current: `ime/dictionary/*` catch-all with
`ime/core/{logging,nextword,settings}/*` subtrees introduced by A1/A2/A5-design.

Per `android-state-audit.md` §8 #6, package-tree reshuffle to match iOS is **deferred**
until A8-sweep finishes. Moving files across packages invalidates the refactor-freeze
observability claim and churns every import in the module; a single rename PR after all
splits settle is the planned path (also satisfies the Phase III Gradle `:shared-core`
module carve-out prerequisite).

## 3. Marker convention — `// region Shared-Core Candidate`

File header placed as three adjacent lines at the **top of the file, before `package`**:

```kotlin
// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import ...
```

Android Studio treats the paired `region` / `endregion` as a foldable code block (same
UX as Swift `// MARK:`). The grep verification in
`../engine/shared-core-readiness.md` §Verification looks for the literal string
`Shared-Core Candidate` on a comment line, so either the paired form above or a single
`// region` label satisfies it.

Files that deliberately do NOT qualify as shared-core candidates begin with
`// NOTE: Not shared-core — <reason>` so audit state is visible at file head.
Marker criteria and purity rules live in `rules/android-guidelines.md` §1 — read that
before applying the marker to a new file; it lists every forbidden import and the
`Any?` / reflection / Moshi exclusions.

### 3.1 Verification grep

```bash
grep -rln "region Shared-Core Candidate" android/app/src/main/java
```

Expected count after each round is tracked in `android-state-audit.md` §3.7 and bumps
monotonically toward the ≥ 40 gating target (§9 signal #3).

## 4. `CROSS-PLATFORM INVARIANT` comment convention

Kotlin comment syntax (from `rules/android-guidelines.md` §2):

```kotlin
// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/NextWord/NextWordScorer.swift:<line>.
// Drift causes silent divergence.
```

This marker is required **only** on the four surfaces enumerated in `ios-exemplar.md`
§5.3 — where a numeric constant (or short constant group) is duplicated literally across
platforms and drift would silently change behavior:

| # | Surface | Android owner | Status |
|---|---------|---------------|--------|
| 1 | NextWord scoring constants (`userWeight`, `dictWeight`, `decayHalfLifeHours`, `learningBonus`, `highUsageDecayFloor`, `lowUsageDecayFloor`, `highUsageThreshold`) | `NextWordService.kt:57-73` | present ✅ |
| 2 | Candidate scoring (recency window, cappedUserFreq cap, user-freq multiplier, completion penalty, closeness max, exact bonus, recency bonus) | `CandidateProcessor.kt` Kdoc + constants 5.3 #2 | present ✅ — A8-sweep expanded the Kdoc to cite all 7 constants (`USER_FREQ_CAP`, `USER_FREQ_WEIGHT`, `RECENCY_WINDOW_MS`, `RECENCY_BONUS`, `EXACT_BONUS`, `COMPLETION_PENALTY`, `CLOSENESS_WEIGHT`) + iOS file path |
| 3 | NextWord timing (`associationTimeoutMs`, `contextTimeoutSeconds`) | `NextWordEngine.kt:28-38` (`ASSOCIATION_TIMEOUT_MS`, `CONTEXT_TIMEOUT_MS`) | present ✅ — A5-impl added inline |
| 4 | Taigi Unicode preprocessing (U+207F, U+1D3A, U+0358 handling) | `TaigiUnicode.kt` | present ✅ |

Two additional Android-only INVARIANT comments exist on `AssociationBinaryReader.kt` and
`DictionaryBinaryReader.kt` (data-file schema pinned to an iOS artifact). These are
legitimate additions — Android is the only platform reading those files at rest — and
are retained.

The policy (constants + tests + docs update together, `INVARIANT_*` test-label prefix)
lives in `rules/cross-platform-alignment.md` §3a.

## 5. Roster target

Phase II gating target per `android-state-audit.md` §9 signal #3: ≥ 40 files carrying the
`// region Shared-Core Candidate` marker (mirrors iOS 43). Current progress:

| Round | Files marked | Running total |
|-------|--------------|---------------|
| Pre-A8-skeleton | 6 — `FrequencyData.kt`, `LoggerBackend.kt`, `InputNormalizer.kt`, `CustomDictionaryDerivation.kt`, `AutocompleteInputClassifier.kt`, `AutocompleteContextBooster.kt` (from A1 / A6) | 6 |
| A8-skeleton | 5 — `TaigiUnicode.kt`, `TaigiPhonetics.kt`, `SuggestionCaseTransformer.kt`, `TPSConverter.kt`, `ToneRestoration.kt` | 11 |
| A5-impl (PR #153) | 4 — `NextWordEngine.kt`, `NextWordOutcome.kt`, `RawNextWordPrediction.kt`, `EnginePrediction.kt` (`ComposingState` extract reserved for A4-impl; no shared-core marker added on it — transitively imports `ToneConverter`) | 15 |
| A4-impl (PR #152) | 0 — `ComposingState.kt` / `ComposingDelegate.kt` were created this round, but `ComposingState` transitively imports `ToneConverter` (which still calls `android.util.Log` + `BuildConfig`). Markers deferred until ToneConverter is purified. | 15 |
| A8-sweep (this round) | 13 — `TaigiWord.kt`, `InputType.kt`, `DictionarySource.kt`, `DictionaryConstants.kt`, `ToneUtilities.kt`, `ToneConverterModels.kt`, `ExternalLookupURLBuilder.kt`, `DictionarySearchResult.kt`, `EnabledDictionaries.kt`, `ComposingTransition.kt`, `EngineSettings.kt`, `EngineSettingsProvider.kt`, `ToneToggles.kt` | 28 |

### 5.1 Gate shortfall (≥40 target — 12 short)

A8-sweep closes the post-split sweep cleanly, but the running total is 28 vs the Phase II gating target of ≥40 (`android-state-audit.md` §9 signal #3). The shortfall is **not** the result of missed candidates — Codex pre-review (2026-04-20) confirmed no additional pure-lexicon / pure-phonetics / pure-composing candidates were overlooked. The gap comes from four files that are structurally near-miss and need small follow-up rounds before they can qualify:

| File | Blocker | Follow-up round |
|------|---------|-----------------|
| `ToneConverter.kt` | `android.util.Log` + `BuildConfig` debug trace | Route through `LoggerBackend` param (default `NullLoggerBackend`) |
| `CandidateProcessor.kt` | `BuildConfig.DEBUG` guard + `currentTime: Long = System.currentTimeMillis()` default | Remove `BuildConfig`; make `currentTime` a required parameter (callers already pass it in production) |
| `ComposingState.kt` | transitively imports `ToneConverter` | Unblocks once `ToneConverter.kt` is purified (above) |
| `DictionaryError.kt` | `sealed class : Exception()` conflicts with `rules/android-guidelines.md` §10 "no Throwable across shared-core" | Convert to a `sealed class` without `Exception` inheritance; callers already switch on subtype |

Closing these four candidates adds 4 markers; the remaining 8 come from splitting a few lexicon files the audit flagged as "needs split" (`CustomDictionaryService` derivation helpers not yet extracted, etc.) — those land as A9-adjacent cleanup or a later A-round. Gate is achievable without another large A-round.

Detailed roster + per-file audit lives in `../engine/shared-core-readiness.md`.

## 6. Where to apply the marker during Phase II

Round-by-round intent (full spec in `android-state-audit.md` §7):

- **A8-skeleton** — 5 terminal files only (this PR). Terminal = not reshaped by A4-impl
  or A5-impl. Documents the convention so later rounds have a reference.
- **A4-impl** — new files from `ComposingState` / `ComposingTransition` extract get the
  marker inline with creation. Parity-correction flag on `clearPreeditWithoutCommit` —
  see `composing-state-boundary.md` §11.
- **A5-impl** — new files from `NextWordEngine` / `NextWordScorer` / `RawNextWordPrediction`
  extract get the marker inline with creation. A5-impl also lands the §5.3 #3 (NextWord
  timing) INVARIANT comment as part of the extract.
- **A8-sweep** — marker applied to every remaining **eligible** candidate file post-splits
  (13 files — see §5 running-total table), plus the `CandidateProcessor.kt` §5.3 #2
  INVARIANT comment expanded to all 7 constants with iOS file-path citation. Three files
  held back (`CandidateProcessor.kt`, `ComposingState.kt`, `DictionaryError.kt`) — each
  has a structural blocker documented in §5.1. Additionally, `ToneConverter.kt`,
  `ComposingDelegate.kt`, and `AndroidLoggerBackend.kt` received explicit
  `// NOTE: Not shared-core — <reason>` headers so audit state is visible at file head.
  No runtime code change in that round.

## 7. ViewModel pattern (reference)

Android Compose tab ViewModels follow the Kotlin block in `ios-exemplar.md` §6
(`MutableStateFlow` private + `asStateFlow()` public; constructor DI for service +
`EngineSettingsProvider`). That block is authoritative; A3 round implements the pattern
for the four tab3 / tab4 screens once §8 #1 (Application subclass) lands.

## 8. Naming and access levels (reference)

`ios-exemplar.md` §5.4 (naming) and §5.5 (access levels) apply verbatim, with the
following Kotlin-specific refinements from `rules/android-guidelines.md`:

- `object` allowed only for stateless utilities / pure namespaces. Converted to `class`
  with constructor DI in A1 whenever state was held.
- `public` kept tight — shared-core types are `public` because a future `:shared-core`
  Gradle module will export them; internal platform wiring stays `internal`.
- No `inline` / `reified` at the shared-core boundary (does not survive FFI extraction).
  No cross-boundary extension functions — prefer top-level functions.

## 9. Cross-references

- iOS alignment target: `ios-exemplar.md`.
- Audit driving Phase II rounds: `android-state-audit.md`.
- Kotlin rules (marker criteria, invariant syntax, DI, coroutines, IME lifecycle):
  `rules/android-guidelines.md`.
- Cross-platform policy (refactor-freeze, invariant-discipline, divergence docs):
  `rules/cross-platform-alignment.md`.
- Composing-state boundary + Android binding: `composing-state-boundary.md` §11.
- NextWord-engine boundary + Android binding: `nextword-engine-boundary.md` §13.
- Shared-core roster + verification greps: `../engine/shared-core-readiness.md`.
- Behavioral invariants both platforms must preserve: `behavioral-invariants.md`.
