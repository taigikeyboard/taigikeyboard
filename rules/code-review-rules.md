# Code Review Rules (Cross-Platform)

Mandatory checklist for reviewing code changes. Apply in order before approving any modification.

## 1. Platform Best Practices

Idiomatic patterns for the target platform. Consistency with existing codebase overrides personal preference.

## 2. No Redundancy

- Duplicated logic → extract shared function
- Duplicated constant/string → extract named constant
- Cross-platform: consistent naming and structure for shared behavior

**Check**: Search for the same string, value, or logic elsewhere before adding new code.

## 3. Clean Code

- Single responsibility — one function/class does one thing
- Proper decoupling — no hidden dependencies between unrelated modules
- Clear boundaries — public API vs internal logic should be obvious

**Check**: Can you describe what this function does in one sentence without "and"?

## 4. No Over-Engineering

- Simplest solution that works — no speculative abstractions
- No wrappers/frameworks for a single use case
- Three similar lines is better than a premature abstraction

## 5. Regression Risk

- Trace all callers of modified functions — verify behavior is preserved
- Check cross-file dependencies (imports, protocols/interfaces, shared state)
- If one platform changes, verify the other is unaffected or needs a matching update

**Check**: List every file that calls the modified code. Confirm each still works.

## 6. Readability

Follow `rules/ai-friendly-code.md` for naming and structure. Additionally:

- Prefer early returns over deep nesting
- A function should be understandable from its file alone where possible
- No misleading or outdated comments — wrong comments are worse than none

## 7. Efficient Review Scope

- Only read relevant files and sections — not entire files for a one-function change
- Review the change as submitted, not aspirational improvements
- Unrelated changes mixed in → request split into separate commits

## 8. Review Before Implementation

When a round has design judgment, run Codex + `Skill(simplify)` review **BEFORE** implementing, not as a post-implementation check. Reviewing after creates unnecessary rework; evaluating first catches issues before they're coded.

Evaluation criteria for the pre-impl pass:

1. Best practices — does the approach follow platform conventions and idiomatic patterns?
2. No redundancy — will the result contain duplicated code, constants, or logic that should be shared?
3. Clean code — single responsibility, proper decoupling, no leaky abstractions.
4. No over-engineering — simplest solution that works, no speculative abstractions.
5. Regression risk — identify all call sites, cross-file dependencies, behavioral changes that could break existing functionality.
6. Readability & maintainability — clear structure, consistent naming (variables, functions, classes, files).
7. AI-friendliness — easy for AI tools to reason about, low ambiguity, minimal unnecessary cross-file context, no misleading/outdated comments.
8. Efficient review scope — only read relevant files/sections.

**Fix ALL findings — including low-priority items.** Do not defer or skip any issue regardless of severity. Everything identified in the pre-impl review must be resolved before implementation lands. After implementation, run `Skill(simplify)` again to verify the final result is clean.

**Understand deeply before remedy**: for any bug fix, achieve a deep, careful, holistic understanding of relevant best-practice + **mainstream IME implementations** first. Start from `docs/references/mainstream-ime-comparison.md` (CLAUDE.md mandatory rule) → drill into per-IME deep-dives needed (librime / MOE / khiin / McBopomofo). Root-cause from actual code (`file:line`), run the Codex pre-impl gate, THEN fix. Do NOT patch symptomatically or guess.

If a bug is subsumed by an adopted architecture direction, prefer fixing it via that architecture rather than a throwaway special-case (e.g. lattice S2 subsuming the `taiuantai` / `taiuantaigi` bugs).

## 9. Performance Gate Philosophy

Default to **qualitative dogfooding**, not quantitative P50/P95 or peak RSS numbers.

- Solo-dev IME. User is QA; perceptible regression on real typing sequences (S1 POJ diacritics, S2 TPS composition, S3 Hanji candidate scroll) is the acceptance criterion.
- Platform enforces a hard 64 MB cap on keyboard extensions — capacity regressions surface as visible keyboard dismiss, not silent numbers.
- Refactor mode: steady-state memory should not grow; leak-free is a sufficient memory gate.
- Instruments signpost + per-refactor re-measurement cost exceeds the benefit of catching microsecond-level regressions the user cannot feel.

How to apply:

- "Enforce latency gate" / "memory gate" / "perf baseline" in a task → translate to "dogfooding pass on S1/S2/S3 + no keyboard dismiss + no progressive memory growth during extended session".
- Do NOT propose `OSSignposter` patches, Instruments workflows, or P50/P95 capture unless the user explicitly asks for numbers.
- `docs/perf/keyboard-baseline-*.md` exists as a deferred template — point to it only when the user wants quantitative CI capture later.
- If a refactor ADDS features (not pure refactor scope), re-open the quantitative discussion — feature growth can shift steady-state capacity.
