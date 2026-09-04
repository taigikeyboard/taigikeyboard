---
paths:
  - "ios/**/*.swift"
  - "android/**/*.kt"
  - "engine/**"
  - "dictionary/**"
  - "docs/architecture/behavioral-invariants.md"
---

# Cross-Platform Alignment Rules

Prevent iOS and Android implementations from diverging in ways that make shared-core (Rust) maintenance harder. Phase II (Android align) closed 2026-04-22 and Phase IV-B (shared-core extraction) closed 2026-05-05 — every pure-logic candidate now lives in Rust. The cross-platform-invariant and divergence-discipline rules below still govern every PR: invariants must stay aligned across iOS / Android / engine, and intentional divergence must be documented.

References: Claude auto-memory `project_shared_core_roadmap.md` (session-persistent state) for current phase status · `docs/engine/migration-inventory.csv` for the live Rust / native ownership inventory · `docs/architecture/behavioral-invariants.md` for observable-behavior contracts · `.claude/rules/ios-guidelines.md` / `.claude/rules/android-guidelines.md` for platform idioms.

## 1. Refactor phases are behavior-frozen

Phase I (iOS exemplar) and Phase II (Android align) are refactor-only. Every PR must preserve observable behavior — moving code is allowed, changing what the user sees or persists is not.

### 1a. Emergency exception — narrow tier only

- Security vulnerabilities
- Crashes (reproducible)
- Loss of user-persisted data — dictionary, user frequency, settings

**Not covered by the emergency exception**: performance regressions, UX glitches, cosmetic issues, feature gaps, "nice to fix while we're here". These wait until the Phase II end decision point.

Each emergency PR must:

- Prefix title with `emergency:` and state the severity tier in the description
- Evaluate impact on the other platform — mirror the fix, or document why not
- Cross-check against `docs/architecture/behavioral-invariants.md` — if the fix changes an invariant, update the doc and its test in the same PR

### 1b. Parity-correction tier

When a refactor surfaces an **existing** divergence between platforms — one platform already had a subtle behavior the other didn't, and the split/unification round forces a choice — the PR may correct the divergence toward the documented invariant in `behavioral-invariants.md`. Examples from iOS G5-impl: generation-counter race elimination, `clearPreeditWithoutCommit` binding on `finishComposingText` (see audit §7 A4-impl / A5-impl parity-correction flags).

Each parity-correction PR must:

- Title prefix `parity:` (or note "parity correction" in the description)
- Describe before/after behavior on **both** platforms
- Confirm which platform's behavior is the target, and that the target matches `behavioral-invariants.md`
- Land a test that pins the post-correction behavior on **both** platforms
- Not be bundled with unrelated refactor work — a parity correction is its own observable change and deserves an isolated review

Parity corrections are the **only** permitted intentional behavior change outside the emergency tier. Do not use this tier as a loophole for feature work.

### 1c. Shared-core-candidate bug-fix constraint

Active from Phase II code-complete (2026-04-22) through Phase IV-B. Governs every PR — including the v3.5.0 release window bug-fix rounds — that touches a file marked `Shared-Core Candidate`. The purpose is explicit: each bug fix during this window must **shrink** future Rust migration cost by keeping the candidate surface clean, not accumulate platform-specific scar tissue.

Any change touching a shared-core-candidate file must:

1. Accept only **immutable value inputs** OR inject services through interfaces **already declared** in the shared-core contract (e.g. `EngineSettings`, `LoggerBackend`, `NextWordPredictor` on Android; `EngineSettingsProvider`, `LoggerBackend` on iOS). Legitimate immutable-context structs, DTO mappers, and batching objects are permitted; they are not banned as "stateful" merely because they carry multiple fields.
2. Introduce **no new** platform / framework singleton reads inside candidate code. Explicitly forbidden: `SharedSettings.shared`, any `*.shared`, `Application.getInstance()`, `BuildConfig.*`, `android.util.Log`, `OSLog`, `KeyboardKit.*`, `androidx.*`, `UIKit`/`SwiftUI`/`Combine`, `kotlinx.coroutines.*`. See `.claude/rules/ios-shared-core-candidates.md` §1 (iOS-specific platform bans — `SharedSettings.shared`, `*.shared`, `UIKit`, `SwiftUI`, `KeyboardKit`, `Combine`, `OSLog`, `@MainActor`) and `.claude/rules/android-guidelines.md` §1 (Android-specific — `android.*`, `androidx.*`, `kotlinx.coroutines.*`, `java.util.concurrent.*`) for the authoritative per-platform enforcement lists. The list above is the merged set enforced at code review; items like `BuildConfig.*` and `Application.getInstance()` extend the per-platform lists because they surfaced in real violations during Phase II.
3. **Mirror any new heuristic or tunable constant** on the other platform in the same PR, with a `CROSS-PLATFORM INVARIANT` comment citing `<mirror file>:<line>`. §3a drift-detection still applies.
4. Pass **Codex + `/simplify` pre-implementation review** for any change introducing a new stateful dependency into a candidate file. Pure refactors, constant-tweak bug fixes, and fixes without new state are exempt from the pre-impl review (post-draft review still applies per `~/.claude/rules/round-workflow.md` Codex sandwich). The Codex pass checks correctness + FFI-safety intent; the `/simplify` pass (Claude Code official skill) checks reuse, quality, and dead-code before implementation lands. Run both in parallel per `~/.claude/rules/claude-workflow.md` §Subagent Usage.

A PR in violation is rejected at review regardless of whether the fix itself is correct. Correct fixes that violate this constraint are rebased to comply — the constraint is the rule, not a recommendation.

## 2. Android mirrors the iOS exemplar through Phase II

Phase II is closed (concluded 2026-04-22, PR #166); Android structural changes continue to match the iOS reference at `docs/architecture/ios-exemplar.md` — module boundaries, DI shape, data model types, threading model, naming.

Deliberate structural divergence (e.g., FlorisBoard forces a different ownership pattern than KeyboardKit) requires explicit user sign-off and a written justification in the PR description before merge.

## 3. Divergence must be identified and documented

When the same logical behavior requires different code on each platform — platform-idiomatic API, capability gap, KeyboardKit / FlorisBoard-specific integration — the PR must:

- Name the divergence in the description
- Classify as **intentional** (platform-specific, keep) or **deferred** (reconcile later)
- For **deferred** items, open a follow-up tracking task and link it

Silent divergence is the failure mode this rule exists to prevent.

**Intentional-divergence example (key-press feedback, PR #444)**: the in-app sound/vibration toggle gates feedback on both platforms, but the OS-master interaction differs — Android drives a direct `Vibrator` that bypasses the OS touch-haptic gate (`HAPTIC_FEEDBACK_ENABLED`), while iOS has no app-side bypass of the System Haptics master, so app-ON + System-Haptics-OFF → no vibration on iOS is expected, not a bug. Classified **intentional** (platform-imposed). Full contract: `docs/architecture/behavioral-invariants.md` §36 `INVARIANT_KEYPRESS_FEEDBACK_APP_TOGGLE_GATE`.

### 3a. Cross-platform invariants — constants, tests, docs update together

Behavior that MUST match between iOS and Android is captured in `docs/architecture/behavioral-invariants.md` with named `INVARIANT_*` labels.

- Constants that mirror the other platform carry a `CROSS-PLATFORM INVARIANT — mirrors <other file>:<line>. Drift causes silent divergence.` comment. Platform examples: iOS `NextWordScorer.swift` / `CandidateProcessor.swift` / `TaigiUnicode.swift` / `Input/ToneUtilities.swift`; Android `NextWordService.kt` / `TaigiUnicode.kt` / `ToneUtilities.kt` / `AssociationBinaryReader.kt` / `DictionaryBinaryReader.kt`. (Note: `TaigiUnicode` and `ToneUtilities` are **platform-stays** helpers per PR #187 — Rust crate retains canonical implementations but Android JVM unit tests can't load `.so`, so the cross-platform invariant lives between iOS↔Android source rather than via FFI.)
- Tests that pin a cross-platform behavior use the `INVARIANT_*` prefix, matching a label in `behavioral-invariants.md`.
- Modifying any invariant constant requires **iOS source + Android source + `behavioral-invariants.md` + invariant test** all updated in the **same PR**. A diff that updates only one side is rejected at review.

Platform-specific comment syntax (Swift `// MARK:` vs Kotlin `// region`) lives in the per-platform guides. The cross-platform policy lives here.

### 3b. Verify alleged divergence before asserting it

Before writing any "iOS does X, Android does Y" sentence in an audit / invariant spec / divergence report, **grep the actual source files for inline alignment comments**:

- `// matches iOS`
- `// matches Android`
- `// CROSS-PLATFORM INVARIANT`
- `// mirrors iOS` / `// mirrors Android`

Treat matching comments as authoritative signal that the original author intended parity — divergence in observable behavior is then a **bug**, not a design decision. When using an Explore agent for a summary read, explicitly ask the agent to report any `// matches …` comments in the flagged files.

Incident: G10 (PR #141) — I wrote that `DictionaryBinaryReader.kt` treated both `hanzi` and `tl` as required, a claim that would have driven a phantom Phase IV-A decision. The Android file had explicit `// matches iOS` comments on lines 81 and 88 right next to the relevant code. Two prior passes (Explore agent + my own pre-review code read) both missed them; the Codex PR-bot caught it by reading the diff line-by-line.

## 4. Phase II end — hybrid decision gate

After Phase II completes, evaluate both gates before any further work:

- **Technical gate** — `/shared-core-confidence` composite ≥ 95% AND D9 FFI POC passes
- **User-facing gate** — bug severity / backlog acceptable, no accumulation of high-severity user-reported issues

| Result | Next step |
|---|---|
| Both pass | Phase III is **available** (user decides — not automatic) |
| Either fails | Enter bugfix / feature-adjustment mode **before** Phase III |

Passing both gates does not mandate extraction. User retains judgment on whether the shared-core cost/benefit still holds at that moment.

### 4a. Phase II.5 prerequisite docs (pre-Phase-III)

Before any D9 FFI POC work begins, two Phase II.5 artifacts must exist and be Codex-reviewed:

- **`docs/engine/ffi-safety.md`** — mandatory rules for any Rust FFI entry: `std::panic::catch_unwind` at every boundary, `Mutex<Engine>` (engine `!Sync` but `Send`), explicit `shutdown(handle)` + `Drop` impl, error sentinels (`Response.ErrorCode` in protobuf, out-of-band null only for "engine is sick, restart"), logging bridge (`log` crate routed to `OSLog` / `android.util.Log` via a platform-side adapter — candidate code never touches platform log APIs). Informed by khiin-rs reference (`references/khiin-rs/`) failure modes: see `references/khiin-rs/` for the failure modes cited.
- **`docs/engine/rust-core-proto.md`** — thin Request/Response schema draft covering only the first two slices (Phonetics + Composing). Includes request-id correlation, generation-counter semantics for stale-response discard, `CMD_SET_CONFIG`-equivalent settings-snapshot push (engine must not cache settings — live-read per `behavioral-invariants.md` §11). Full Lexicon / NextWord / SQLite proto design is deferred to Phase III.

Both docs must comply with `.claude/rules/rust-best-practices.md` (workspace layout §1, error handling §2, crate choices §3, non-goals §8) and `.claude/rules/rust-ffi-safety.md` (FFI boundary discipline §1, `unsafe` discipline §3, opaque handle pattern §4). Rule deviations require `// JUSTIFICATION:` prose in the spec.

These docs land in the post-v3.5.0 round per the approved roadmap revision plan. Until both exist, Phase III entry is blocked regardless of `/shared-core-confidence` score or user-facing gate status.

## 5. Out of scope for this rule

- Feature planning and bug triage process — owned by user, not code rules
- Behavioral invariants content — see `docs/architecture/behavioral-invariants.md`
- Rust FFI design, ownership model, API shape — Phase IV-A (thin proto draft in Phase II.5 per §4a)
- Platform-specific idioms — see `.claude/rules/ios-guidelines.md` (day-to-day), `.claude/rules/ios-architecture.md` (structural), and `.claude/rules/android-guidelines.md`
- Review checklist mechanics — see `~/.claude/rules/code-review-rules.md`

### 5.1 Rust shared-core non-goals (codified)

The Rust shared-core, when it eventually exists, will **not** own any of the following — they stay in Swift (iOS) / Kotlin (Android) forever. Informed by the khiin-rs reference study (2026-04-22); derived from that study:

- **Candidate UI navigation, layout semantics, or styling**. The engine returns neutral candidate values + preedit segments; navigation ownership (focus, page scroll, selection) is platform-side. Matches `khiin-rs/protos/src/command.proto:114`, which leaves candidate display to the client app.
- **Platform text-region types**. The engine never sees `NSRange`, `ExtractedText`, `TextPosition`, `UITextDocumentProxy`, `InputConnection`, `EditorInfo`, or any SwiftUI / UIKit / Compose view type. Proto message envelopes carry only `String`, numeric types, and engine-defined value types.
- **KeyboardKit / FlorisBoard bindings**. These stay in Swift / Kotlin forever. Their shape may be aligned across platforms as a "shared-core-adjacent" executor layer (the platform engine executor described in `docs/architecture/ios-exemplar.md:32` layer diagram and elaborated in §Platform executor contract around line 196–200), but the types themselves are never ported to Rust.
- **DB asset-copy and path resolution**. The engine receives an open file handle or filesystem path; it does not touch `Bundle.main`, `context.filesDir`, or `AssetManager`. Asset-copy mechanics, update-in-place policy, and stamp-file versioning stay platform-side (see `docs/architecture/data-artifacts-portability.md` §7 for Android specifics).

These are hard non-goals, not preferences. A proposal to "just push this one UI helper into Rust" is rejected on sight; the boundary is the boundary.
