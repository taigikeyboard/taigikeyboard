# Planning Rules

Mandatory rules before writing a multi-PR plan, design doc, or architectural proposal.

## Persistent hand-off

Multi-PR plans (5+ phases) must live in TWO places:

1. **`docs/roadmap.md`** — public-facing, git-tracked, stable reference. Future PRs link here in their description.
2. **`memory/project_<feature>.md`** — auto-loaded each session; tracks phase status table + active PR pointer + key design decisions.

The transient plan file at `~/.claude/plans/*.md` is NOT enough — it disappears between sessions and is not git-tracked.

**Phase 0 of any multi-PR feature** = "rewrite roadmap + create memory" as its own admin PR. Then phase work starts.

**Memory shape**:

- Phase status table — one row per phase, status ∈ Pending / In progress / Merged / Blocked.
- Active PR pointer — current round's phase / branch / PR / last dogfood conclusion.
- Cross-session-required design decisions called out explicitly.

**PR sizing**: target 200-500 LOC. Don't shard <100 (high overhead). Don't pile >700 (unreviewable). PR boundary = phase boundary; multi-commit within a phase is fine.

After each PR merges: update the memory status row → `Merged + <commit>`; advance the active pointer to the next phase.

## Grounded in actual code

Before proposing new modules, new APIs, or cross-module integration, **read the actual code being affected** and describe expected behavior with `file:line` citations. Do not speculate.

- Any "should X live in Y crate" question → first `Read` `Y/api.rs` / `dispatch.rs` / `lib.rs`.
- Describe "today, input X does what" with `file:line` citations.
- If the module's real responsibility differs from your initial mental model, table-compare them explicitly in the plan.
- After the design change is drafted, trace the new call chain and verify it matches the plan.

Mark plan sections that have been verified this way with `(grounded in code)` or `(actual code reading)` so a future session can tell signal apart from speculation.

**Why**: in the v3.5.8 連續輸入 plan, I drafted a nextword integration based on the assumption "nextword is the bigram prediction engine". The user asked me to read the actual code. `engine/nextword/src/api.rs:42-84` + `dispatch.rs:14-99` showed nextword does NOT fetch predictions — platform feeds `Vec<RawNextWordPrediction>` from outside; nextword only filters/scores. That single read flipped both the conclusion's direction AND its reasoning.

## Cite best practices explicitly

Every architecture plan must have a **"最佳實踐對齊" / "Best practices alignment"** section that:

1. **Per phase, cite the relevant `rules/<file>.md` §<n>** when a project rule constrains the design. e.g. `rules/android-guidelines.md §1 shared-core candidate`.
2. **Per architectural choice, cite a mainstream IME source `file:line`** when borrowing from established IME conventions. e.g. `khiin-rs buffer_mgr.rs:1096-1129` for commit-and-resegment.
3. **Explicitly list "刻意不採用" / "deliberately not adopted"** patterns from those references with reasoning. Avoids the reader assuming we missed them.

Starting point is ALWAYS `docs/references/mainstream-ime-comparison.md` (TL;DR matrix + topic index → drill into per-repo cards). Do NOT re-explore `references/` from scratch.

Table format for IME borrowings: `| 主流做法 | 來源 file:line | 本 plan 對應 phase |`.

YAGNI exclusions belong in this section too — e.g. "librime SyllableGraph + Translator full pipeline → over-engineering, we only need forward span endings".

## No future-version planning

When planning the current slice, do NOT propose new release versions or schedule future slices.

- A slice scope audit reveals adjacent code that fits the same engine module → pull into the CURRENT slice.
- Adjacent code that doesn't fit → mark as **outside the current slice / PR**, **no version commitment**. Release scope is user-gated per `rules/diagnosis-discipline.md`.
- Pre-existing release-map entries from prior strategic decisions stay; do not append new entries.

This rule blocks:

- "Recommend v3.5.2 = X" / "Schedule X for v3.6.x" style proposals.
- Adding new entries to the release map mid-conversation.
- Pre-committing scope of unstarted future slices.
- Round-end editorializing like "next step: ① cut v3.5.8 ② non-feature round ③ start S1".

This rule does NOT block:

- Pulling discovered work INTO the current slice.
- Marking other code as outside the current slice/PR without naming a version.
- Acknowledging existing pre-planned versions when relevant.
- Recording an *adopted, explicitly-unscheduled* strategy in roadmap/memory at the user's request.

Answer the question actually asked. Don't append release-cadence menus.

## No redundant fallback

"A condition fails → fallback to B" is an anti-pattern. Push B's capability into A; let the data flow stay one direction.

- See `if X.is_empty() { return Y_alternate_path() }` on the platform side → push Y's capability into the engine as part of X, not maintain an outer fallback.
- See "main path doesn't support case Z, old path catches it" framing in a PR → ask "why doesn't the main path catch it; can it?".
- Scope: applies to data fetch / candidate query / segmentation / other core flow. **Defensive guards** (boundary checks, cold-start neutral boost) are NOT fallbacks.
- For special-perf / temporary-compatibility fallbacks: question whether you actually need them; if necessary, the spec/PR must explicitly state "temporary compatibility, remove after period X".
