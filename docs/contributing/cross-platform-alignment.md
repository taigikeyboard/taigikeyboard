# Cross-Platform Alignment Rules

Prevent iOS and Android implementations from diverging in ways that make shared-core (Rust) maintenance harder. Logic every platform needs lives in the engine; the rules below govern every PR: invariants must stay aligned across iOS / Android / engine, and intentional divergence must be documented.

References: `docs/engine/migration-inventory.csv` for the live Rust / native ownership inventory · `docs/architecture/behavioral-invariants.md` for observable-behavior contracts · `docs/contributing/ios-guidelines.md` / `docs/contributing/android-guidelines.md` for platform idioms.

## 1. Refactor rounds are behavior-frozen

A refactor PR must preserve observable behavior — moving code is allowed, changing what the user sees or persists is not.

### 1a. Emergency exception — narrow tier only

- Security vulnerabilities
- Crashes (reproducible)
- Loss of user-persisted data — dictionary, user frequency, settings

**Not covered by the emergency exception**: performance regressions, UX glitches, cosmetic issues, feature gaps, "nice to fix while we're here". These get their own bugfix / feature round.

Each emergency PR must:

- Prefix title with `emergency:` and state the severity tier in the description
- Evaluate impact on the other platform — mirror the fix, or document why not
- Cross-check against `docs/architecture/behavioral-invariants.md` — if the fix changes an invariant, update the doc and its test in the same PR

### 1b. Parity-correction tier

When a refactor surfaces an **existing** divergence between platforms — one platform already had a subtle behavior the other didn't, and the split/unification round forces a choice — the PR may correct the divergence toward the documented invariant in `behavioral-invariants.md`. Examples: generation-counter race elimination, `clearPreeditWithoutCommit` binding on `finishComposingText`.

Each parity-correction PR must:

- Title prefix `parity:` (or note "parity correction" in the description)
- Describe before/after behavior on **both** platforms
- Confirm which platform's behavior is the target, and that the target matches `behavioral-invariants.md`
- Land a test that pins the post-correction behavior on **both** platforms
- Not be bundled with unrelated refactor work — a parity correction is its own observable change and deserves an isolated review

Parity corrections are the **only** permitted intentional behavior change outside the emergency tier. Do not use this tier as a loophole for feature work.

### 1c. Shared-core-candidate bug-fix constraint

Governs every PR that touches a file marked `Shared-Core Candidate` (the `native_keep` roster in `docs/engine/migration-inventory.csv`). Each bug fix must **shrink** future Rust migration cost by keeping the candidate surface clean, not accumulate platform-specific scar tissue.

Any change touching a shared-core-candidate file must:

1. Accept only **immutable value inputs** OR inject services through interfaces **already declared** in the shared-core contract (e.g. `EngineSettings`, `LoggerBackend` on Android; `EngineSettingsProvider`, `LoggerBackend` on iOS). Legitimate immutable-context structs, DTO mappers, and batching objects are permitted; they are not banned as "stateful" merely because they carry multiple fields.
2. Introduce **no new** platform / framework singleton reads inside candidate code. Explicitly forbidden: `SharedSettings.shared`, any `*.shared`, `Application.getInstance()`, `BuildConfig.*`, `android.util.Log`, `OSLog`, `KeyboardKit.*`, `androidx.*`, `UIKit`/`SwiftUI`/`Combine`, `kotlinx.coroutines.*`. See `docs/contributing/ios-architecture.md` §5 (iOS-specific platform bans — `SharedSettings.shared`, `*.shared`, `UIKit`, `SwiftUI`, `KeyboardKit`, `Combine`, `OSLog`, `@MainActor`) and `docs/contributing/android-guidelines.md` §1 (Android-specific — `android.*`, `androidx.*`, `kotlinx.coroutines.*`, `java.util.concurrent.*`) for the authoritative per-platform enforcement lists. The list above is the merged set enforced at code review; items like `BuildConfig.*` and `Application.getInstance()` extend the per-platform lists because they surfaced in real violations.
3. **Mirror any new heuristic or tunable constant** on the other platform in the same PR, with a `CROSS-PLATFORM INVARIANT` comment citing `<mirror file>:<line>`. §3a drift-detection still applies.
4. Get the **design reviewed before implementation** for any change introducing a new stateful dependency into a candidate file — correctness, FFI-safety intent, reuse and dead code. Pure refactors, constant-tweak bug fixes, and fixes without new state need only the normal diff review.

A PR in violation is rejected at review regardless of whether the fix itself is correct. Correct fixes that violate this constraint are rebased to comply.

## 2. Android mirrors the iOS exemplar

Android structural changes match the iOS reference at `docs/architecture/ios-exemplar.md` — module boundaries, DI shape, data model types, threading model, naming.

Deliberate structural divergence (e.g., FlorisBoard forces a different ownership pattern than KeyboardKit) requires explicit user sign-off and a written justification in the PR description before merge.

## 3. Divergence must be identified and documented

When the same logical behavior requires different code on each platform — platform-idiomatic API, capability gap, KeyboardKit / FlorisBoard-specific integration — the PR must:

- Name the divergence in the description
- Classify as **intentional** (platform-specific, keep) or **pending** (to reconcile, unscheduled)
- For **pending** items, open a follow-up tracking task and link it

Silent divergence is the failure mode this rule exists to prevent.

**Intentional-divergence example (key-press feedback)**: the in-app sound/vibration toggle gates feedback on both platforms, but the OS-master interaction differs — Android drives a direct `Vibrator` that bypasses the OS touch-haptic gate (`HAPTIC_FEEDBACK_ENABLED`), while iOS has no app-side bypass of the System Haptics master, so app-ON + System-Haptics-OFF → no vibration on iOS is expected, not a bug. Classified **intentional** (platform-imposed). Full contract: `docs/architecture/behavioral-invariants.md` §36 `INVARIANT_KEYPRESS_FEEDBACK_APP_TOGGLE_GATE`.

### 3a. Cross-platform invariants — constants, tests, docs update together

Behavior that MUST match between iOS and Android is captured in `docs/architecture/behavioral-invariants.md` with named `INVARIANT_*` labels.

- Constants that mirror the other platform carry a `CROSS-PLATFORM INVARIANT — mirrors <other file>:<line>. Drift causes silent divergence.` comment. Current examples (`grep -rl "CROSS-PLATFORM INVARIANT"`): iOS `Settings/EngineSettings.swift` / `Theme/BuiltInThemes.swift`; Android `ime/settings/EngineSettings.kt` / `ime/theme/BuiltInThemes.kt`. A rule both platforms compute belongs in the engine, not in a mirrored pair: the auto-space attaching set (`IsAttachingPunctuation`), the external-lookup digit-tone fold (`ExternalLookupDigitForm`) and the per-row dictionary sources (`TaigiWord.sources`) are engine ops the platforms call. Files marked `// MARK: - Shared-Core Candidate` (iOS) / `// region Shared-Core Candidate` (Android; criteria in `ios-architecture.md` §5) are pure, Foundation- / stdlib-only platform code — DTOs, enums, settings types, input helpers such as `CharacterInputPipeline` and Android `EnglishWordMatcher` — that stays on the platform; the marker says the file has no platform dependency, not that its logic is already in the engine. Because Android JVM unit tests cannot load the `.so`, a mirrored invariant that does stay on the platforms is pinned iOS↔Android in source rather than via FFI.
- The test that pins a documented label names it — a `// INVARIANT_<LABEL> (<doc> §N)` comment directly above the test (the common form), or the test-name prefix (`fn invariant_<label>_…` / `func testINVARIANT_<label>_…` / `fun INVARIANT_<label>_…`). CI `INVARIANT labels` (`.github/workflows/invariants.yml`) runs `tools/invariant_labels.py`: every label in `docs/architecture` / `docs/engine` / `docs/ui` / `docs/contributing` must appear in test code (a comment in production code does not count). Labels with no test yet sit in `tools/invariant_labels_pending.txt`, which only shrinks — a new label never joins it; a behavior that is gone loses its label.
- Modifying any invariant constant requires **iOS source + Android source + `behavioral-invariants.md` + invariant test** all updated in the **same PR**. A diff that updates only one side is rejected at review.

Platform-specific comment syntax (Swift `// MARK:` vs Kotlin `// region`) lives in the per-platform guides. The cross-platform policy lives here.

### 3b. Verify alleged divergence before asserting it

Before writing any "iOS does X, Android does Y" sentence in an audit / invariant spec / divergence report, **grep the actual source files, case-insensitively, for inline alignment comments**: `(matches|mirrors) (the )?(iOS|Android)` and `CROSS-PLATFORM INVARIANT`.

Treat matching comments as authoritative signal that the original author intended parity — divergence in observable behavior is then a **bug**, not a design decision. When using an Explore agent for a summary read, explicitly ask the agent to report any `Mirrors …` / `matches …` comments in the flagged files.

## 4. Out of scope for this rule

- Feature planning and bug triage process — owned by user, not code rules
- Behavioral invariants content — see `docs/architecture/behavioral-invariants.md`
- Rust FFI design, ownership model, API shape — see `docs/contributing/rust-ffi-safety.md`
- Platform-specific idioms — see `docs/contributing/ios-guidelines.md` (day-to-day), `docs/contributing/ios-architecture.md` (structural), and `docs/contributing/android-guidelines.md`

### 4.1 Rust shared-core non-goals (codified)

The Rust shared-core does **not** own any of the following — they stay in Swift (iOS) / Kotlin (Android) forever. Informed by the khiin-rs reference study:

- **Candidate UI navigation, layout semantics, or styling**. The engine returns neutral candidate values + preedit segments; navigation ownership (focus, page scroll, selection) is platform-side. Matches `khiin-rs/protos/src/command.proto:114`, which leaves candidate display to the client app.
- **Platform text-region types**. The engine never sees `NSRange`, `ExtractedText`, `TextPosition`, `UITextDocumentProxy`, `InputConnection`, `EditorInfo`, or any SwiftUI / UIKit / Compose view type. Proto message envelopes carry only `String`, numeric types, and engine-defined value types.
- **KeyboardKit / FlorisBoard bindings**. These stay in Swift / Kotlin forever. Their shape may be aligned across platforms as a "shared-core-adjacent" executor layer (the Platform engine executor box in `docs/architecture/ios-exemplar.md` §1), but the types themselves are never ported to Rust.
- **DB asset-copy and path resolution**. The engine receives an open file handle or filesystem path; it does not touch `Bundle.main`, `context.filesDir`, or `AssetManager`. Asset-copy mechanics, update-in-place policy, and stamp-file versioning stay platform-side (see `docs/architecture/data-artifacts-portability.md` §7 for Android specifics).

These are hard non-goals, not preferences.
