# Claude Workflow Rules

Mandatory conventions for collaborating with Claude Code (Opus 4.7) on this project.
Aligns with the global Opus 4.7 interaction guidance; this file encodes the project-specific expectations.

## Reasoning Depth

Match thinking depth to task class. Do not overthink simple work.

| Task class | Expected behavior |
|------------|-------------------|
| Single-file small edit, lookup, status question | Short, direct answer. No extended reasoning |
| Cross-platform alignment (iOS + Android) | Deep reasoning; state behavior first, then per-platform notes |
| Multi-file refactor, engine (next-word / composing-state / phonetics) changes | Deep reasoning + staged plan (`IMPLEMENTATION_PLAN.md`) |
| Ambiguous debugging, regression hunts | Deep reasoning; enumerate hypotheses before tool calls |
| Code review | Follow `rules/code-review-rules.md`; depth matches diff scope |

Prompt cues: `think carefully / step-by-step` → go deeper; `respond quickly / directly` → skip deep analysis.

## Subagent Usage

**Spawn subagents only when work is genuinely parallel.** Fan out in a single turn (multiple Agent calls in one message).

**Good parallel cases:**
- Reading the iOS and Android counterparts of the same feature simultaneously
- Searching multiple independent areas (e.g., `engine/`, `ui/`, `dictionary/`) for the same pattern
- Research + plan agents running together before implementation
- Codex review + simplify review running together (per `feedback_review_before_impl.md`)

**Do not spawn subagents for:**
- A task completable in a single response
- A lookup you can do faster with Grep/Glob/Read directly
- Sequential steps where each depends on the previous result

## Tool vs. Reasoning

Prefer reasoning when the answer is already derivable from loaded context or CLAUDE.md.
Call tools only when external state is needed (files not yet read, git status, search results).
When a tool call is warranted, state the reason briefly in user-facing text.

## Clarification Batching

Ask all open questions in the first turn. Do not drip questions across turns.
If multiple ambiguities exist, list them together and let the user answer in one reply.

## Response Length

- **Simple questions** — one sentence or a few bullets
- **Status updates** — terse; the diff speaks for itself (per `feedback_review_before_impl.md`)
- **Cross-platform analysis / plans** — bullet-point breakdown, per-platform sections
- **Code explanation for the user** — match the complexity of the code; no filler

Explicit length requests from the user override the defaults.

## Plan Mode Usage

Non-trivial implementation tasks (3+ files, new features, multi-stage refactors) → use Plan mode or `IMPLEMENTATION_PLAN.md` before coding.
Trivial fixes → go straight to the edit.

## Review Before Implementation

For significant changes, run Codex + simplify review **before** implementing. Address all findings including low-priority ones (per `feedback_review_before_impl.md`).
