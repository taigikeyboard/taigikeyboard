# Composing Slice (v3.5.4 / D9.3) — Scope Audit

> Branch: `main` (HEAD `955bf62`)
> Date: 2026-04-30
> Status: pre-impl planning; pre-Codex sandwich.

## 1. Core state machine inventory (the slice)

### iOS — `ios/Sources/TaigiKeyboard/Input/Composing/` (507 LOC, 4 files)

| File | LOC | Role |
|---|---|---|
| `ComposingState.swift` | 274 | Pure state machine. `mutating func apply(intent, mode, toneToggles, …) → ComposingTransition`. Owns `Phase` (idle / composing(raw)) + `Intent` (11 cases) + derived-display computation. |
| `ComposingTransition.swift` | 50 | Pure data — `(newPhase, newSelectedIndex, effects[], derivedDisplay)`. |
| `ComposingManager.swift` | 168 | Orchestrator. Holds state, calls `state.apply(...)`, dispatches `Effect`s to `ComposingDelegate`. |
| `ComposingDelegate.swift` | 15 | Protocol — sink for effects. |

### Android — `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/composing/` (1953 LOC, 9 files)

| File | LOC | Slice classification |
|---|---|---|
| `ComposingState.kt` | 393 | **CORE** — paired with iOS `ComposingState.swift`. |
| `ComposingTransition.kt` | 66 | **CORE** — paired with iOS. |
| `ComposingManager.kt` | 359 | **CORE** — paired with iOS. |
| `ComposingDelegate.kt` | 82 | **CORE** — paired with iOS. |
| `AutocompleteInputClassifier.kt` | 68 | adjacent helper — deciding whether input is taigi/english/hanji. |
| `AutocompleteContextBooster.kt` | 39 | **NextWord-adjacent** — pure context boost helper, mirrors `ios/.../NextWord/AutocompleteContextBooster.swift`. |
| `TaigiAutocompleteService.kt` | 141 | **NextWord-bridging** — imports `NextWordService`, calls `nextWord.predict(...)`. |
| `EnglishAutocompleteService.kt` | 264 | autocomplete orchestration (English path). |
| `UserFrequencyService.kt` | 541 | dictionary-side; **NOT composing-side** despite directory placement. |

**Cohesion observation**: Android's `composing/` directory is over-broad — it bundles state-machine (740 LOC) + autocomplete-orchestration (444 LOC) + NextWord-context (39 LOC) + frequency-tracking (541 LOC). iOS scopes the directory tighter: only the state machine sits in `Input/Composing/`; autocomplete orchestration is in `Autocomplete/Services/`, NextWord in `NextWord/`.

This directory mismatch is itself a candidate cleanup; it is NOT slice scope, but worth noting for follow-up.

## 2. NextWord overlap — concrete analysis

**iOS `Input/Composing/*.swift`**: zero `NextWord` references. State machine is NextWord-clean.

**Android `composing/*.kt`**: NextWord references confined to two files:

- `AutocompleteContextBooster.kt:11,14` — comment-only references (it is a pure helper that takes the context as a param; does not import NextWord).
- `TaigiAutocompleteService.kt:9,30,119,130` — actual import + call: `nextWord.predict(...)`.

**Verdict**: the *state machine* has zero overlap with NextWord. The *autocomplete-orchestration* layer (TaigiAutocompleteService) is the only real bridge, and it bridges via `NextWordService` (a dictionary-side service), not via composing internals. This means slice (a) Composing state machine and slice (b) NextWord can be sequenced cleanly without leaking each other's responsibilities.

## 3. Upstream call sites — must be touched in this slice

### iOS callers of `ComposingManager` (12 files)

`Actions/ActionHandler.swift`, `Actions/ActionHandler+KeyActions.swift`, `Actions/ActionHandler+Suggestions.swift`, `Autocomplete/Services/AutocompleteProviders.swift`, `Autocomplete/Services/AutocompleteService.swift`, `KeyboardExtension/KeyboardViewController.swift`, `KeyboardExtension/KeyboardViewController+Cleanup.swift`, `KeyboardExtension/KeyboardViewController+EmojiDelegate.swift`, `KeyboardExtension/KeyboardViewController+Setup.swift`, `KeyboardExtension/TaigiKeyboardOverlays.swift`, `KeyboardExtension/TaigiKeyboardView.swift`, `App/Tabs/Layout/KeyboardPreviewPanel.swift`.

### Android callers of `ComposingManager` (7 files)

`ime/text/TextInputManager.kt`, `ime/text/CandidateUpdateCoordinator.kt`, `ime/text/key/KeyView.kt`, `ime/media/MediaInputManager.kt`, `ime/text/smartbar/SmartbarManager.kt`, `ime/text/smartbar/CandidateClickHandler.kt`, `engine/RustEngineBridge.kt` (existing bridge).

## 4. Anti-pattern adjacency check (per slice scoping rule)

State machine has no per-codepoint scans, no algorithm-constants tables shared cross-language. The Phonetics + Ranking slices already removed the cross-language algorithm mirrors that bordered composing. No anti-pattern adjacencies identified — clean slice boundary.

## 5. Joint decisions (local + Codex sandwich, 2026-04-30) — RESOLVED

### (a) `EngineHandle` lifecycle — singleton vs per-text-field

**Option A — per-IME-instance singleton.** `EngineHandle` lives on `KeyboardViewController` (iOS) / `TaigiKeyboard` IME service (Android). One `Mutex<Engine>` per process; reset called explicitly on `textWillChange` / `onStartInput`.

- Pros: lowest init/teardown cost; matches reference IMEs (khiin-rs `EngineController` is service-scoped, McBopomofo `KeyHandler` is single instance); aligns with current dispatch model where Phonetics + Ranking are stateless and shared singletons.
- Cons: lifecycle policy must be explicit — `engine.reset_composing()` op required on focus change; subtle bugs if reset is missed.

**Option B — per-text-field handle.** New `EngineHandle` constructed per `textWillChange` / `onStartInput`.

- Pros: zero risk of cross-field state leak.
- Cons: init cost on every focus change (which can fire many times during a typing session — e.g. password autofill, candidate panel taps); `Mutex<Engine>` reconstruction; teardown ordering hazards.

**Decision: Option A — singleton-per-IME-instance.** Local + Codex agree. Both reference IMEs (McBopomofo's long-lived `KeyHandler`, khiin-rs's service-scoped `EngineController`) chose A. Per-field handles solve leaks by construction but the cost of focus-churn (autofill, input-view restarts, iOS candidate-panel taps that fire `textWillChange`) is unacceptable.

**Codex-added invariant (must bake into slice plan):** `reset_composing` MUST be idempotent AND guarded by input-context generation / self-commit suppression. iOS `textWillChange` fires on candidate taps and self-driven commits too — naive "reset on every textWillChange" will erase the composing buffer mid-candidate-selection. Concrete mitigation: track an `InputContextGeneration` (UInt) on the platform side; only call `reset_composing` when generation actually advances (new field), not on intra-field re-entry. Pair with a `selfCommitInProgress` flag during candidate selection / commit paths.

### (b) v3.5.4 scope — absorb NextWord call sites?

**Option A — strict boundary.** v3.5.4 = state machine only (CORE 4 files iOS / 4 files Android). `TaigiAutocompleteService` + `AutocompleteContextBooster` stay platform-side, pulled into v3.5.5 NextWord slice.

- Pros: smaller PR; respects "one slice per release"; state machine has zero NextWord coupling so the boundary is natural.
- Cons: one platform layer (`TaigiAutocompleteService`) keeps a transitional shape until v3.5.5.

**Option B — absorb autocomplete services into v3.5.4.** Pull `TaigiAutocompleteService` + helpers in too.

- Pros: Android `composing/` directory ends up cleaner.
- Cons: `TaigiAutocompleteService.predict(...)` calls `NextWordService.predict(...)` — pulling it into Rust forces NextWord into Rust early; violates v3.5.5 boundary.

**Decision: Option A — strict state-machine boundary.** Local + Codex agree. Hidden-coupling check passes: `Intent.selectSuggestion(String)` and `commitDerived` consume plain text + emit platform-neutral effects (commit / reset autocomplete / reset autocomplete context). That is an effect contract, NOT a dependency on `TaigiAutocompleteService` or `NextWordService`. `performAutocomplete` remains a platform-side effect — autocomplete orchestration stays on the platform until v3.5.5.

### (c) Version numbering — settled

Per user (2026-04-30): **v3.5.4 → v3.5.5 → v3.5.6 incrementing**, NOT v3.6.0. Cadence memory `project_rust_migration_cadence.md` already has this map. `docs/engine/rust-core-proto.md` + `docs/engine/v3.5.3-cleanup-audit.md` still reference v3.6.0 for D9.3 — these will be updated within the v3.5.4 PR (not as a separate cleanup round).

## 6. Slice scope — proposed

**In scope (v3.5.4):**

- iOS 4 files in `Input/Composing/` migrated to thin `RustEngineBridge.composing*` wrappers. Legacy Swift state machine deleted.
- Android 4 core files (`ComposingState.kt`, `ComposingTransition.kt`, `ComposingManager.kt`, `ComposingDelegate.kt`) migrated similarly. Legacy Kotlin state machine deleted.
- Rust crate `engine/composing` (NEW) — `Mutex<Engine>` + `EngineHandle` lifecycle + `Composing` op family on `LexiconRequest` (or new `ComposingRequest`).
- Proto `phonetics.proto` / `lexicon.proto` / `envelope.proto` — fill `CMD_COMPOSING` reserved at `envelope.proto:18,53`.
- `engine/dispatch` — add `ComposingRequest` arm.
- `RustEngineBridge.swift` + `RustEngineBridge.kt` — add wrappers for each composing op; expose `reset_composing()` op for focus-change hook.
- INVARIANT_composing_*  parity tests on both platforms.
- 12 iOS upstream callers + 7 Android upstream callers — call site rewrites to consume new bridge surface (no behavior change).
- Doc updates in `rust-core-proto.md` + `v3.5.3-cleanup-audit.md` — D9.3 / v3.6.0 wording → v3.5.4.

**Out of scope (deferred to v3.5.5):**

- `TaigiAutocompleteService.kt`, `AutocompleteContextBooster.kt`, `EnglishAutocompleteService.kt`.
- `UserFrequencyService.kt` (frequency tracking — separate concern).
- `AutocompleteInputClassifier.kt` — re-evaluate during v3.5.5 planning.

**Out of scope (deferred to v3.5.6):**

- `iri/erk/eeh` and other phonetic-completeness items (already retained per `feedback_phonetic_completeness.md`).

## 7. Next steps

1. Send §5 (a)+(b) to Codex via `codex exec` for joint decision per `feedback_codex_review_sandwich.md` pre-impl gate.
2. Apply Codex's recommendation; write proto draft + commit slicing plan.
3. Second Codex pass on the full plan.
4. ExitPlanMode after APPROVE.
