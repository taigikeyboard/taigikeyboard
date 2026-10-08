# Android Project Guidelines

Mandatory rules for Android development. Core architecture + Kotlin idioms + DI + DataStore + null/error handling + Gradle. UI / IME-specific patterns / testing / refactor-round checklist live in `docs/contributing/android-ime-patterns.md`.

**Three goals** every rule below serves at least one of:

- **R — Rust-friendly**: reduce future friction if a `native_keep` file (`docs/engine/migration-inventory.csv`) later moves to a Rust crate
- **B — Best practice**: Kotlin / Android idiomatic code
- **A — Anti-regression**: reduce "fix A, break B" outcomes during refactor rounds

Each rule is tagged with one or more of `[R]`, `[B]`, `[A]`.

## 1. Shared-core candidate rules `[R]`

Files marked `// region Shared-Core Candidate` must satisfy ALL criteria below. Mirrors the iOS Shared-Core contract in `docs/contributing/ios-architecture.md` §5 — same criteria, Kotlin-translated.

1. Imports Kotlin stdlib only. Forbidden: `android.*`, `androidx.*`, `kotlinx.coroutines.*`, `java.util.concurrent.*`, `com.squareup.moshi.*`.
2. No `object` with mutable state, no `companion object` state, no reflection, no Moshi / serialization.
3. No clock reads — caller supplies `nowMs: Long` at call boundaries. No `System.currentTimeMillis()`, `SystemClock.*`, `Instant.now()`.
4. No I/O — no SQLite, no File, no SharedPreferences, no DataStore.
5. No coroutines or `Dispatchers.*`. Executor / async lives in the platform wrapper.
6. No logging inside candidate logic. If a candidate must emit a diagnostic, it accepts a `LoggerBackend` interface (the Android equivalent of iOS `Logging/LoggerBackend.swift`) via constructor/function parameter. The `LoggerBackend` interface itself is shared-core; concrete `AndroidLoggerBackend` lives in platform code and wraps `android.util.Log`. Candidates never call `android.util.Log` directly.

### File header marker

Every candidate file begins with:

```kotlin
// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
```

### Kotlin-to-Rust shape preferences

- Values → `data class` (maps to Rust struct).
- Tagged unions → `sealed class` / `sealed interface` (maps to Rust enum). Avoid `Any?` + `when`.
- Collections at module boundaries → prefer `List<T>` (read-only) and explicit DTOs. Avoid `Map<String, Any?>` — Rust has no equivalent.
- Nullability at shared-core boundaries → prefer `Result<T>` or a `sealed class Outcome` over nullable returns. Null across the FFI boundary costs a sentinel.
- Numeric types → explicit `Long` / `Double` (maps to Rust `i64` / `f64`). Avoid `Int` in signatures crossing shared-core boundaries unless the range is genuinely 32-bit.
- No `inline` / `reified` functions in candidates — these do not survive extraction to a separately compiled Rust lib.
- No extension functions crossing shared-core boundaries — prefer top-level functions.

## 2. Cross-platform invariant discipline — Android syntax `[A]` `[R]`

The **policy** (constants + tests + docs update together, comment format, `INVARIANT_*` test label prefix) lives in `docs/contributing/cross-platform-alignment.md` §3a. This section only captures Kotlin-specific details:

- Comment syntax in Kotlin:
  ```kotlin
  // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/EngineSettings.swift:<line>.
  // Drift causes silent divergence.
  ```
- Surfaces carrying the marker on Android: authoritative list = `grep -rl 'CROSS-PLATFORM INVARIANT' android/`.
- Async pipelines that may produce stale results use a monotonic generation counter. Late callbacks drop on generation mismatch. Matches the iOS `NextWordController` envelope-generation pattern (`bumpEnvelopeGeneration()`).

## 3. Kotlin idioms `[B]`

Generic Kotlin idioms (`val` over `var`, sealed hierarchies, data classes, coroutines) follow the [Kotlin coding conventions](https://kotlinlang.org/docs/coding-conventions.html); this section keeps only the project-specific ones.

- Follow the [Android Keyboard Design Guidelines](https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method) for IME components.
- `enum class` for pure enums with no associated state (sealed hierarchies and data classes: the Kotlin coding conventions above).
- No `lateinit` on public API (private `lateinit` OK inside lifecycle-bound classes). `!!`: §7.

## 4. Android lifecycle + DI `[B]` `[A]`

- Use the three-scope model:
  1. **App-tab graph** — `Application` scope, consumed by `Activity` / Compose UI.
  2. **IME service graph** — `TaigiKeyboard : LifecycleInputMethodService` scope.
  3. **Per-input-session state** — `onStartInput` / `onFinishInput`.
- Manual DI via `CompositionRoot` holder. Hilt is **not** currently in scope — avoid annotation-based runtime magic that complicates Rust boundary design.
- Constructor injection preferred. No `.INSTANCE` global reach-ins inside engine or ViewModel code.
- `object` is acceptable only for **stateless** utilities (pure functions / constants). If it holds DB handles, Context, cached state, or reads the clock, convert to `class` with constructor DI.
- Never store an `Activity` Context inside an `object` or a long-lived `class` — memory leak. Use `Application` Context (`applicationContext`) for process-lifetime references.
- Lifecycle scopes are paired with cancellation:
  - `TaigiKeyboard.serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)` is cancelled in `onDestroy`.
  - ViewModels use `viewModelScope` (cancelled automatically by AndroidX ViewModel).
  - No new `GlobalScope` or `MainScope()` in singleton-scoped objects.

## 5. Coroutines + threading `[B]` `[A]`

Structured concurrency and dispatcher choice follow the Kotlin coroutines guide; every coroutine's parent scope is one of the lifecycle scopes in §4. Project-specific rules:

- Cancel before reschedule — if a new request supersedes an old one, cancel the old `Job` first (see `CandidateUpdateCoordinator.kt` for the working pattern).
- **IME thread rule**: every `InputConnection` call runs on `Dispatchers.Main.immediate`. The platform executor (`ComposingManager`, `NextWordController`) is the thread gate; shared-core code stays thread-agnostic.

## 6. DataStore + settings access `[B]` `[R]`

- `PrefHelper` is the authoritative settings facade. DataStore is the storage; Flow subscriptions are the live-push channel.
- For engine-facing reads, implement `EngineSettings` with `get()` properties that re-read `PrefHelper.cachedPrefs` on each access. **Never** snapshot via `val x = prefs.getX()` in the initializer — subsequent DataStore updates would be invisible until object re-creation. See `docs/architecture/ios-exemplar.md` §3 Android-live-read warning for the verbatim anti-pattern.
- Cross-process / cross-lifecycle settings updates (IME extension ↔ host app) travel via DataStore Flow collection in `TaigiKeyboard.onCreate`, not via a notification broadcast.
- SharedPreferences → DataStore migration runs once in `TaigiKeyboardApplication.onCreate`. Do not add new SharedPreferences writes in new code.

## 7. Nullability + errors `[B]` `[R]`

- Forbidden: `!!` in production code. Use `requireNotNull(x) { "explanation" }` at the boundary where null is a programmer error, or `x ?: return` / `x ?: error("…")` when null is a runtime condition.
- At module boundaries inside shared-core candidates, prefer `Result<T>` or `sealed class Outcome { class Success(val value: T); class Failure(val reason: String) }` over nullable returns. Nullable types map poorly to Rust's `Option<T>` across FFI.
- Java exceptions do not cross shared-core boundaries. Catch and convert to `Outcome` / `Result` at the edge. Rust has `Result<T, E>`, not `throw`.
- No `try { … } catch (e: Exception) { /* swallow */ }`. At minimum, log via `LoggerBackend` and return a typed failure.

## 8. Kotlin extension shadowing rule `[B]`

Kotlin resolves a member before a same-name extension, whatever the argument types, so an extension named like an existing member is unreachable. `LoggerBackend` (`ime/core/logging/LoggerBackend.kt`) has members `d` / `i` / `w` / `e`; its lazy-logging extensions take distinct names (`debug`, and `info` / `warn` / `error` if added). The same holds for extension properties.

## 8a. User-data SQLite is the engine's `[A]`

The four user-data stores (`user_frequency.db`, `user_association.db`, `custom_dictionary.db`, `learned_phrases.db`) are opened, migrated and written by the engine's bundled SQLite (`engine/userdata`, `docs/architecture/user-data-engine-roadmap.md` P8b); Android reaches them only through `engine/UserDataBridge.kt` / `ime/dictionary/UserDataClient.kt`, and `TaigiKeyboardApplication.onCreate` opens them. The app runs no SQL of its own, so the OS SQLite's `minSdk` dialect (3.22 at `minSdk` 28) does not constrain these stores.

Never open these files with `android.database.sqlite` (or any other SQLite): two SQLite copies in one process lock independently and can corrupt a file both hold (roadmap U2 / U6). A new store or query belongs in the engine.

## 9. Gradle files are editable `[B]`

`android/build.gradle`, `android/app/build.gradle.kts`, `android/settings.gradle`, and other Android Gradle scripts are **editable by agents directly** — gradle edits are routine (plugin wiring, dep bumps, lint config).

- ✅ Edit gradle files directly.
- ❌ Still off-limits: `*.xcodeproj`, `*.pbxproj`, `*.xcworkspace` (`docs/contributing/ios-guidelines.md`).
- After gradle edits, surface what changed in plain text and remind the user that an Android Studio Gradle sync is needed.

## 10. References

- `docs/contributing/android-ime-patterns.md` — companion: Compose, IME-specific patterns, testing, refactor-round checklist
- Companion documents on the iOS side: `docs/contributing/ios-guidelines.md` (day-to-day), `docs/contributing/ios-architecture.md` (structural).
- Cross-platform behavior contract: `docs/contributing/cross-platform-alignment.md`.
- Architectural target: `docs/architecture/ios-exemplar.md` (the contract Android aligns toward).
- Live Rust / native ownership inventory: `docs/engine/migration-inventory.csv`.
- Invariants to preserve: `docs/architecture/behavioral-invariants.md`.
- Security: `docs/contributing/security-rules.md` (logging guards, SQL binding, Android exported-component rules).
- UI style: `docs/contributing/ui-style-guide.md`.
