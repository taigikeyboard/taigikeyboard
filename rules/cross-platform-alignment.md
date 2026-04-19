# Cross-Platform Alignment Rules

Prevent iOS and Android implementations from diverging in ways that make shared-core (Rust) extraction harder. Applies to every code change until Phase IV-B completes.

References: Claude auto-memory `project_shared_core_roadmap.md` (Claude session-persistent state) for current phase status · `docs/architecture/ios-exemplar-plan.md` for Phase I history · `docs/architecture/android-state-audit.md` for Phase II task groups · `docs/architecture/behavioral-invariants.md` for observable-behavior contracts · `rules/ios-guidelines.md` / `rules/android-guidelines.md` for platform idioms.

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

## 2. Android mirrors the iOS exemplar through Phase II

Until Phase II gating conditions in `docs/architecture/android-state-audit.md` §9 are met, Android structural changes must match the iOS reference at `docs/architecture/ios-exemplar.md` — module boundaries, DI shape, data model types, threading model, naming.

Deliberate structural divergence (e.g., FlorisBoard forces a different ownership pattern than KeyboardKit) requires explicit user sign-off and a written justification in the PR description before merge.

## 3. Divergence must be identified and documented

When the same logical behavior requires different code on each platform — platform-idiomatic API, capability gap, KeyboardKit / FlorisBoard-specific integration — the PR must:

- Name the divergence in the description
- Classify as **intentional** (platform-specific, keep) or **deferred** (reconcile later)
- For **deferred** items, open a follow-up tracking task and link it

Silent divergence is the failure mode this rule exists to prevent.

### 3a. Cross-platform invariants — constants, tests, docs update together

Behavior that MUST match between iOS and Android is captured in `docs/architecture/behavioral-invariants.md` with named `INVARIANT_*` labels.

- Constants that mirror the other platform carry a `CROSS-PLATFORM INVARIANT — mirrors <other file>:<line>. Drift causes silent divergence.` comment. Platform examples: iOS `NextWordScorer.swift` / `CandidateProcessor.swift` / `TaigiUnicode.swift`; Android `NextWordService.kt` / `TaigiUnicode.kt` / `AssociationBinaryReader.kt` / `DictionaryBinaryReader.kt`.
- Tests that pin a cross-platform behavior use the `INVARIANT_*` prefix, matching a label in `behavioral-invariants.md`.
- Modifying any invariant constant requires **iOS source + Android source + `behavioral-invariants.md` + invariant test** all updated in the **same PR**. A diff that updates only one side is rejected at review.

Platform-specific comment syntax (Swift `// MARK:` vs Kotlin `// region`) lives in the per-platform guides. The cross-platform policy lives here.

## 4. Phase II end — hybrid decision gate

After Phase II completes, evaluate both gates before any further work:

- **Technical gate** — `/shared-core-confidence` composite ≥ 95% AND D9 FFI POC passes
- **User-facing gate** — bug severity / backlog acceptable, no accumulation of high-severity user-reported issues

| Result | Next step |
|---|---|
| Both pass | Phase III is **available** (user decides — not automatic) |
| Either fails | Enter bugfix / feature-adjustment mode **before** Phase III |

Passing both gates does not mandate extraction. User retains judgment on whether the shared-core cost/benefit still holds at that moment.

## 5. Out of scope for this rule

- Feature planning and bug triage process — owned by user, not code rules
- Behavioral invariants content — see `docs/architecture/behavioral-invariants.md`
- Rust FFI design, ownership model, API shape — Phase IV-A
- Platform-specific idioms — see `rules/ios-guidelines.md` (day-to-day), `rules/ios-architecture.md` (structural), and `rules/android-guidelines.md`
- Review checklist mechanics — see `rules/code-review-rules.md`
