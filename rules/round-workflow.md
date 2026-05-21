# Round Workflow

Mandatory rules for starting, continuing, or wrapping a coding round.

## Branching & rounds

- Every coding round runs on its own feature/refactor branch, merged via PR to `main`. No `develop` intermediary.
- One round = one PR. Don't merge two task groups into one PR.
- Context is cleared between rounds — `MEMORY.md` + `memory/project_*.md` + repo state are the only hand-off.
- Before the round closes, update the relevant `project_*.md` so a cold-start next session can resume from where this round ended.

**Admin tier — direct-to-main allowed** (no branch, no PR):

- Pure docs/memory edits, `Makefile` micro-edits, `.gitignore` additions, `.claude/skills/**` prose-only, `docs/**` policy, `CLAUDE.md`, `rules/**`, changelog policy.
- Must NOT touch app source (`ios/Sources/**`, `android/app/src/**`, `engine/**`, `dictionary/**`).
- If in doubt → branch + PR.

## Auto mode

When the user says **"use auto mode"** / **"不要再一個一個問我"** / **"你/codex/你們決定"**:

- Drive the entire remaining round autonomously — no per-step confirmation, no mid-stream narration of micro-actions.
- For design forks: bundle ALL open questions into ONE `codex exec --cd <repo> < /tmp/prompt.txt` ANALYSIS-ONLY consultation, with explicit options + rationale per fork. Combine Codex's call with own reasoning; flag any disagreement explicitly. Then proceed.
- Read whole files (`Read` tool) over incremental `grep`/`sed` slices when extracting review findings.
- Only stop for: a true BLOCK needing a design decision the user must own, a destructive/irreversible op, or genuine scope ambiguity.
- Round-end summary still required (1-2 sentences). No play-by-play.
- Auto mode does NOT skip the Codex sandwich or `/codex-pr-review`.

**Cadence**: batch Xcode/file/tool actions per phase to reduce test handbacks. Xcode pbxproj edits are still user-only — see `rules/ios-guidelines.md`.

## Commit & push

- The verbal cue **"commit"** = atomic commit + push, no second confirmation.
- On a feature branch with a clean fix landed → commit + push without asking. Pause only for risky ops.
- **Never** `git stash` unilaterally — even "stash to unblock pull". Ask first. Enforced by `ask` permission on `Bash(git stash:*)` in `.claude/settings.json`.
- Read-only Bash (`sed`/`grep`/`cat`/`git log`/`cargo build|test|clippy|fmt`) doesn't need confirmation. Prefer the `Read` tool for files. Risky ops still confirm.
- **Commit authorship**: run `git config user.email` first. If personal (`minsiansu@gmail.com`), include `Co-Authored-By: Claude Code <noreply@anthropic.com>`. If any other email (company), omit the trailer.

## Codex review sandwich

**Codex only — never Gemini.** Pick the right entry point:

| Use case | Entry point |
|---|---|
| Plan/doc/design review with multiple questions or >300 words of context | `codex exec --cd <repo> < /tmp/prompt.txt` |
| Bounded "go fix bug X / continue prior rescue" hand-off | `Skill(codex:rescue)` ↔ `/codex:rescue` |
| Quick PR-style review pass | `/codex:review` or `/codex:adversarial-review` |
| Post-PR-open bot sweep | `/codex-pr-review` |

`codex exec` prompt rules: ≤500 words, 2-5 concrete numbered questions, ALWAYS include `ANALYSIS-ONLY: do not write code, do not create patches` (Codex defaults to implementing). Reference files by repo-relative path; `--cd` lets Codex read them.

**The sandwich (mandatory for any coding round with design judgment)**:

1. **Pre-impl Codex + `Skill(simplify)` review** — root cause + proposed approach via `codex exec` or `codex:rescue`. Run `Skill(simplify)` against the planned change in parallel. Evaluation criteria + "fix all findings" discipline lives in `rules/code-review-rules.md` §8. Revise plan based on findings BEFORE writing code.
2. **Implement** on the round's branch.
3. **Post-impl Codex review** — same channel, `git diff` + regression-check ask.
4. **`Skill(simplify)`** again — surface dead code, redundant abstractions, missed reuse in the actual diff. Apply findings.
5. **Commit + push + open PR.**
6. **`/codex-pr-review`** — PR-bot sweep BEFORE merge. Catches things the sandwich misses: caller-side type juggling, off-by-one, accidental invariants. Don't poll — user-trigger; if user defers, exit the round.
7. **Merge** — user-gated per `rules/diagnosis-discipline.md` (no unilateral release scope).

**Sandwich is NOT required for** (calibrated 2026-04-27 / 2026-05-08): pure grep-replace renames (wire-compatible), cross-file docstring/comment cleanup, formatter/linter auto-fixes, bounded build-script tweaks with loud failure modes. Pure mechanical changes with no design surface skip the sandwich.

**Why the dual gate (sandwich + PR-bot)**: PR #227 (2026-05-07) shipped with a 1px refactor-freeze divergence — Codex post-impl said "math fidelity PASS", PR-bot caught caller-side `.toInt()` precision drift within minutes. Sandwich reviews "this code does what its plan said + what its own logic intends"; PR-bot reviews "the diff preserves every observable property of HEAD~1 under all input shapes". Both gates do real work.

## Merge

- Terse **"merge PR"** cue = merge ALL of this round's ready PRs (Codex SHIP + clean status). Don't `AskUserQuestion` for scope.
- Use `gh pr merge --delete-branch`. Locally sync with `git merge --ff-only origin/main` (never stash).

## Pre-commit quality gates

**`code-simplifier` agent — judgment-based, NOT mandatory each round.**

- **Apply when**: new helpers may overlap existing code shapes; single PR adds >200 LOC logic; diff shows repeated patterns hinting at missed abstraction; touched a crate where this round didn't already refactor and the diff added new functions.
- **Skip when**: pure doc edit; mechanical rename; behaviour-preserving inline; <50 LOC new code; single-purpose helper with test coverage.
- Round summary should state whether `code-simplifier` ran + reason.

**`cargo test` scope — default touched crates only via `-p`, not `--workspace`.**

```
cargo test --manifest-path engine/Cargo.toml -p <crate>
```

Multiple touched crates → multiple `-p` flags. Escalate to `--workspace` only for:

- `/release-helper` round
- USER explicit ask
- Cross-crate golden / parity tests in scope (`fetch_at_pos.golden`, `poj_notone_parity`, `op_coverage`)
- Pre-merge final catch-net pass (optional, risk-driven)

Round summary should state the test scope so the user can spot over-narrowing.

## Build & changelog

- **User runs ALL builds and tests manually** — never invoke `xcodebuild` / `./gradlew` / `make build`. User rejects build hooks, auto-rebuild, and build reminders.
- **Changelog updates only at release time** — never edit `changelog/<version>.md` mid-round. The `/release-helper` skill is the canonical release flow.
- Release scope and timing are user-gated — see `rules/diagnosis-discipline.md`.
