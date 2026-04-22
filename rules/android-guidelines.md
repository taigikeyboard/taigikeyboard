# Android Project Guidelines

Mandatory rules for Android development. Read before modifying Android code.

**Three goals** every rule below serves at least one of:

- **R — Rust-friendly**: reduce future friction when shared-core slices move to a Rust crate (Phase IV-A onward)
- **B — Best practice**: Kotlin / Android idiomatic code
- **A — Anti-regression**: reduce "fix A, break B" outcomes during refactor rounds

Each rule is tagged with one or more of `[R]`, `[B]`, `[A]`.

## 1. Shared-core candidate rules `[R]`

Files marked `// region Shared-Core Candidate` must satisfy ALL criteria below. Mirrors the iOS Shared-Core contract in `rules/ios-architecture.md` §4 — same criteria, Kotlin-translated.

1. Imports Kotlin stdlib only. Forbidden: `android.*`, `androidx.*`, `kotlinx.coroutines.*`, `java.util.concurrent.*`, `com.squareup.moshi.*`.
2. No `object` with mutable state, no `companion object` state, no reflection, no Moshi / serialization.
3. No clock reads — caller supplies `nowMs: Long` at call boundaries. No `System.currentTimeMillis()`, `SystemClock.*`, `Instant.now()`.
4. No I/O — no SQLite, no File, no SharedPreferences, no DataStore.
5. No coroutines or `Dispatchers.*`. Executor / async lives in the platform wrapper.
6. No logging inside candidate logic. If a candidate must emit a diagnostic, it accepts a `LoggerBackend` interface (the Android equivalent of iOS `Common/LoggerBackend.swift`) via constructor/function parameter. The `LoggerBackend` interface itself is shared-core; concrete `AndroidLogLoggerBackend` lives in platform code and wraps `android.util.Log`. Candidates never call `android.util.Log` directly.

### File header marker

Every candidate file begins with:

```kotlin
// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
```

Engine-layer files that deliberately do **not** qualify begin with `// NOTE: Not shared-core — <reason>` so audit state is visible at file head.

### Kotlin-to-Rust shape preferences

- Values → `data class` (maps to Rust struct).
- Tagged unions → `sealed class` / `sealed interface` (maps to Rust enum). Avoid `Any?` + `when`.
- Collections at module boundaries → prefer `List<T>` (read-only) and explicit DTOs. Avoid `Map<String, Any?>` — Rust has no equivalent.
- Nullability at shared-core boundaries → prefer `Result<T>` or a `sealed class Outcome` over nullable returns. Null across the FFI boundary costs a sentinel.
- Numeric types → explicit `Long` / `Double` (maps to Rust `i64` / `f64`). Avoid `Int` in signatures crossing shared-core boundaries unless the range is genuinely 32-bit.
- No `inline` / `reified` functions in candidates — these do not survive extraction to a separately compiled Rust lib.
- No extension functions crossing shared-core boundaries — prefer top-level functions.

## 2. Cross-platform invariant discipline — Android syntax `[A]` `[R]`

The **policy** (constants + tests + docs update together, comment format, `INVARIANT_*` test label prefix) lives in `rules/cross-platform-alignment.md` §3a. This section only captures Kotlin-specific details:

- Comment syntax in Kotlin:
  ```kotlin
  // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/NextWord/NextWordScorer.swift:<line>.
  // Drift causes silent divergence.
  ```
- Surfaces currently carrying the marker on Android (from `docs/architecture/ios-exemplar.md` §5.3 — all landed as of Phase II code-close 2026-04-22): NextWord scoring constants (`NextWordScorer.kt` — extracted by A9 PR #158, marker present), CandidateProcessor recency window + score caps (`CandidateProcessor.kt` — markers expanded to 7/7 via A8-sweep PR #154), NextWord timing constants (`NextWordEngine.kt` — extracted by A5-impl PR #153, marker present), TaigiUnicode preprocessing codepoints (`TaigiUnicode.kt` — present), Association + Dictionary binary-reader layouts (`AssociationBinaryReader.kt` / `DictionaryBinaryReader.kt` — present).
- Async pipelines that may produce stale results use a monotonic generation counter. Late callbacks drop on generation mismatch. Matches the iOS `NextWordController.currentGeneration` pattern; A5-impl ports it to Android per audit §7 parity-correction flag.

## 3. Kotlin idioms `[B]`

- `val` over `var`; immutable collections over mutable where exposed.
- Prefer `requireNotNull(x)` / `checkNotNull(x)` / `x ?: error("…")` over `!!`.
- `sealed class` / `sealed interface` for restricted hierarchies; `enum class` for pure enums with no associated state.
- `data class` for value objects.
- Scope functions have distinct roles:
  - `let` — null-safe chaining or transforming a non-null reference into a different shape.
  - `apply` — configuring a receiver (builder style), returns the receiver.
  - `also` — side effect on the receiver, returns the receiver.
  - `run` — executing a block with the receiver as `this`, returns the block result.
  - `with` — same as `run` but with an explicit first argument.
- Single-expression functions where the body is a single expression.
- Trailing lambda syntax when the last parameter is a lambda.
- Avoid `!!`, `lateinit` on public API (private `lateinit` OK inside lifecycle-bound classes).

## 4. Android lifecycle + DI `[B]` `[A]`

- Use the three-scope model from `docs/architecture/android-state-audit.md` §2.2:
  1. **App-tab graph** — `Application` scope, consumed by `Activity` / Compose UI.
  2. **IME service graph** — `TaigiKeyboard : LifecycleInputMethodService` scope.
  3. **Per-input-session state** — `onStartInput` / `onFinishInput`.
- Manual DI via `CompositionRoot` holder in Phase II. Hilt is **not** in scope until Phase IV at earliest — avoid annotation-based runtime magic that complicates Rust boundary design.
- Constructor injection preferred. No `.INSTANCE` global reach-ins inside engine or ViewModel code.
- `object` is acceptable only for **stateless** utilities (pure functions / constants). If it holds DB handles, Context, cached state, or reads the clock, convert to `class` with constructor DI.
- Never store an `Activity` Context inside an `object` or a long-lived `class` — memory leak. Use `Application` Context (`applicationContext`) for process-lifetime references.
- Lifecycle scopes are paired with cancellation:
  - `TaigiKeyboard.serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)` is cancelled in `onDestroy`.
  - ViewModels use `viewModelScope` (cancelled automatically by AndroidX ViewModel).
  - No new `GlobalScope` or `MainScope()` in singleton-scoped objects.

## 5. Coroutines + threading `[B]` `[A]`

- Structured concurrency — every coroutine has an explicit parent scope owned by a lifecycle.
- Dispatcher choice:
  - `Dispatchers.Main.immediate` — UI state updates, `InputConnection` calls.
  - `Dispatchers.IO` — SQLite reads/writes, file I/O, network.
  - `Dispatchers.Default` — CPU-bound work (candidate scoring, display derivation).
- `SupervisorJob` at roots where one child failure must not cancel siblings. Pair with a `CoroutineExceptionHandler` where failures must be logged.
- Cancel before reschedule — if a new request supersedes an old one, cancel the old `Job` first (see `CandidateUpdateCoordinator.kt` for the working pattern).
- **IME thread rule**: every `InputConnection` call runs on `Dispatchers.Main.immediate`. The platform executor (`ComposingManager`, `NextWordController`-equivalent) is the thread gate; shared-core code stays thread-agnostic.

## 6. DataStore + settings access `[B]` `[R]`

- `PrefHelper` is the authoritative settings facade. DataStore is the storage; Flow subscriptions are the live-push channel.
- For engine-facing reads, implement `EngineSettings` with `get()` properties that re-read `PrefHelper.cachedPrefs` on each access. **Never** snapshot via `val x = prefs.getX()` in the initializer — subsequent DataStore updates would be invisible until object re-creation. See `docs/architecture/ios-exemplar.md` §3 Android-live-read warning for the verbatim anti-pattern.
- Cross-process / cross-lifecycle settings updates (IME extension ↔ host app) travel via DataStore Flow collection in `TaigiKeyboard.onCreate`, not via a notification broadcast.
- SharedPreferences → DataStore migration runs once in `TaigiKeyboard.onCreate`. Do not add new SharedPreferences writes in new code.

## 7. Compose patterns `[B]`

- `remember { … }` for UI-only state that survives recomposition but not configuration change.
- `rememberSaveable { … }` when the state should survive process death (e.g. user input in a form).
- `collectAsStateWithLifecycle(initial)` — preferred over `collectAsState` for Flow observation in Compose. Pauses collection when the lifecycle stops, avoiding unnecessary recomposition.
- State hoisting — stateless `@Composable` functions take state + callbacks as parameters. Stateful versions delegate to the stateless version.
- `derivedStateOf { … }` for computed state that depends on multiple observable sources.
- Side-effect APIs choose by scope:
  - `LaunchedEffect(key)` — coroutine bound to composition, restarts on key change.
  - `DisposableEffect(key)` — setup + teardown pair (listeners, subscriptions).
  - `SideEffect` — synchronous side effect on every recomposition.
- ViewModel owns async; View invokes ViewModel methods. No `LaunchedEffect { viewModelScope.launch { … } }` inside a Compose function.

## 8. IME-specific rules `[B]` `[A]`

- `InputConnection.finishComposingText()` **commits the composing region by default.** To honor "clear without commit" semantics, call `setComposingText("", 1)` **before** `finishComposingText()`. Documented in `docs/architecture/ios-exemplar.md` §4.1 and `docs/architecture/composing-state-boundary.md` §2.2 (Android mapping addendum is A4-design deliverable).
- All `InputConnection` calls on `Dispatchers.Main.immediate` — Android IME requires main-thread for InputConnection.
- `InputMethodService.onStartInput` / `onFinishInput` bracket per-input-session state — reset composing context here, not in `onCreate` / `onDestroy`.
- `LifecycleInputMethodService` provides a `Lifecycle` + `ViewModelStore`. Scoped ViewModels inside the IME use the service's `ViewModelStoreOwner`, not a plain `androidx.lifecycle.ViewModel()` (which has no owner).
- FlorisBoard-derived code (keyboard view hierarchy, layout JSON loaders) is **platform**, not shared-core. Do not try to purify it.

## 9. Testing `[B]` `[A]`

Cross-platform test naming + assertion conventions follow `rules/ios-guidelines.md` "Test Conventions"; Android-specific additions only here.

- JUnit 4 — project default (see existing `app/src/test/java/.../ime/dictionary/*Test.kt`). Do not mix JUnit 5.
- Turbine (`app.cash.turbine`) for Flow assertions — pattern `flow.test { … }`.
- `kotlinx-coroutines-test` — `runTest { … }` block with injectable `TestDispatcher` for time-controlled tests.
- `INVARIANT_*` function-name prefix for cross-platform-invariant tests; labels match `docs/architecture/behavioral-invariants.md` (policy: `rules/cross-platform-alignment.md` §3a).
- Tests must be runnable via `./gradlew test` (wired into `testImplementation` in the test source set).

## 10. Nullability + errors `[B]` `[R]`

- Forbidden: `!!` in production code. Use `requireNotNull(x) { "explanation" }` at the boundary where null is a programmer error, or `x ?: return` / `x ?: error("…")` when null is a runtime condition.
- At module boundaries inside shared-core candidates, prefer `Result<T>` or `sealed class Outcome { class Success(val value: T); class Failure(val reason: String) }` over nullable returns. Nullable types map poorly to Rust's `Option<T>` across FFI.
- Java exceptions do not cross shared-core boundaries. Catch and convert to `Outcome` / `Result` at the edge. Rust has `Result<T, E>`, not `throw`.
- No `try { … } catch (e: Exception) { /* swallow */ }`. At minimum, log via `LoggerBackend` and return a typed failure.

## 11. Refactor-round checklist `[A]`

Durable checklist for every Android refactor PR:

- [ ] Refactor-freeze observed per `rules/cross-platform-alignment.md` §1. If the PR intentionally changes behavior, it uses the emergency tier (§1a) or parity-correction tier (§1b) and labels accordingly.
- [ ] Codex + `/simplify` pre-review on plan before implementation (per `rules/claude-workflow.md` §Review Before Implementation). `/simplify` is the Claude Code official skill — run in parallel with Codex to catch reuse / quality / dead-code issues Codex does not flag.
- [ ] Codex post-review on diff before merge.
- [ ] Qualitative dogfooding pass (S1 / S2 / S3 sequences) on a real Android device for any hot-path round.
- [ ] Invariant tests stay green.
- [ ] `// CROSS-PLATFORM INVARIANT` comments updated if constants moved (policy in `rules/cross-platform-alignment.md` §3a).
- [ ] No new `android.util.Log` / `GlobalScope` / `!!` / `object`-with-state introduced.

Phase II-specific task labels (A0–A10) and sequencing live in `docs/architecture/android-state-audit.md` §7; avoid duplicating them here so this doc does not age with the phase. **Phase II code work closed 2026-04-22** (last round: PR #166 parity fix); the A0–A10 labels are now historical. Current round-by-round state is tracked in auto-memory (`project_android_phase_ii_audit.md`); the audit doc §9.1 table is a point-in-time snapshot, not a living counter.

During the v3.5.0 release bug-fix window, every Android PR touching a shared-core-candidate file additionally honors the §1c constraint in `rules/cross-platform-alignment.md` — immutable inputs, no new platform-singleton reads, mirror constants with `CROSS-PLATFORM INVARIANT` comments, Codex + `/simplify` pre-impl review if a new stateful dependency enters a candidate file.

## 12. References

- Companion documents on the iOS side: `rules/ios-guidelines.md` (day-to-day), `rules/ios-architecture.md` (structural).
- Cross-platform behavior contract: `rules/cross-platform-alignment.md`.
- Architectural target: `docs/architecture/ios-exemplar.md` (the contract Android Phase II aligns toward).
- Current Android state and Phase II task groups: `docs/architecture/android-state-audit.md`.
- Invariants to preserve: `docs/architecture/behavioral-invariants.md`.
- Code review checklist: `rules/code-review-rules.md`.
- Naming + comment rules (cross-platform): `rules/ai-friendly-code.md`.
- Security: `rules/security-rules.md` (logging guards, SQL binding, Android exported-component rules).
- UI style: `rules/ui-style-guide.md`.
- Claude Opus 4.7 workflow tuning: `rules/claude-workflow.md`.
