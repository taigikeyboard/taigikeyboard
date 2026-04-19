# Cross-Platform Alignment Rules

Prevent iOS and Android implementations from diverging in ways that make shared-core (Rust) extraction harder. Applies to every code change until Phase IV-B completes.

References: `memory/project_shared_core_roadmap.md` for phase status · `docs/architecture/behavioral-invariants.md` for observable-behavior contracts · `rules/ios-guidelines.md` / Android equivalents for platform idioms.

## 1. Refactor phases are behavior-frozen

Phase I (iOS exemplar) and Phase II (Android align) are refactor-only. Every PR must preserve observable behavior — moving code is allowed, changing what the user sees or persists is not.

**Emergency exception — narrow tier only**:

- Security vulnerabilities
- Crashes (reproducible)
- Loss of user-persisted data — dictionary, user frequency, settings

**Not covered by the exception**: performance regressions, UX glitches, cosmetic issues, feature gaps, "nice to fix while we're here". These wait until the Phase II end decision point.

Each emergency PR must:

- Prefix title with `emergency:` and state the severity tier in the description
- Evaluate impact on the other platform — mirror the fix, or document why not
- Cross-check against `docs/architecture/behavioral-invariants.md` — if the fix changes an invariant, update the doc and its test in the same PR

## 2. Android mirrors the iOS exemplar through Phase II

Until Phase II gating conditions in `project_shared_core_roadmap.md` are met, Android structural changes must match the iOS reference — module boundaries, DI shape, data model types, threading model, naming.

Deliberate structural divergence (e.g., FlorisBoard forces a different ownership pattern than KeyboardKit) requires explicit user sign-off and a written justification in the PR description before merge.

## 3. Divergence must be identified and documented

When the same logical behavior requires different code on each platform — platform-idiomatic API, capability gap, KeyboardKit / FlorisBoard-specific integration — the PR must:

- Name the divergence in the description
- Classify as **intentional** (platform-specific, keep) or **deferred** (reconcile later)
- For **deferred** items, open a follow-up tracking task and link it

Silent divergence is the failure mode this rule exists to prevent.

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
- Platform-specific idioms — see `rules/ios-guidelines.md` and Android equivalents
- Review checklist mechanics — see `rules/code-review-rules.md`
