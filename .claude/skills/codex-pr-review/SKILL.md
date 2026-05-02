---
name: codex-pr-review
description: Fetch Codex bot review comments on the current branch's PR, evaluate each finding, fix the ones that need fixing, reply on each thread with the commit SHA, and resolve the thread. Use when the user wants to clear out Codex review feedback in one pass — typically right before merging, or after a recent push triggered a fresh Codex review run.
---

# Codex PR Review Sweep

GitHub PRs in this repo automatically trigger Codex bot reviews that post P1/P2/P3 findings as line comments. This skill sweeps every unresolved Codex thread on the current branch's PR, evaluates each one, applies fixes when warranted, and closes the loop on GitHub.

## Steps

### 1. Identify the target PR

```
gh pr view --json number,headRefName -q '{n:.number,branch:.headRefName}'
```

Abort with a clear message if no PR is open for the current branch.

### 2. Fetch unresolved review threads

Use GraphQL — REST `pulls/{n}/comments` cannot tell which threads are resolved.

```
gh api graphql -f query='
query($owner:String!, $name:String!, $pr:Int!) {
  repository(owner:$owner, name:$name) {
    pullRequest(number:$pr) {
      reviewThreads(first:100) {
        nodes {
          id
          isResolved
          isOutdated
          comments(first:5) {
            nodes {
              databaseId
              author { login }
              body
              path
              line
              originalLine
            }
          }
        }
      }
    }
  }
}' -F owner=taigikeyboard -F name=taigikeyboard -F pr=<PR>
```

Filter: `isResolved == false` AND first comment author is the Codex bot. The Codex bot login varies; detect by body containing the badge marker `![P1 Badge]` / `![P2 Badge]` / `![P3 Badge]` or a `**Useful?** React with 👍 / 👎.` footer.

Keep `threadId`, `firstCommentDatabaseId`, `priority` (P1/P2/P3), `path`, `line`, `body`.

### 3. Evaluate each finding

For each thread, decide one of:

- **FIX** — the finding is correct and the fix is small enough to apply now (single file, < ~50 lines, no design choice). Capture the planned change in one sentence.
- **DISCUSS** — the finding raises a real design question. Don't fix in this skill run; reply asking the user for direction, leave thread unresolved.
- **DECLINE** — the finding is wrong, already-handled, or not applicable. Reply with the reason; resolve.
- **ALREADY-FIXED** — the latest commit on the branch already addresses this. Reply with the commit SHA; resolve.

Show the evaluation table to the user before any code changes. For FIX entries, also include the Step 4 review tier (A / B / C) so the user can sanity-check the planned review depth before you commit.

```
| # | Prio | Path:Line              | Verdict      | Tier | Plan / Reason                          |
|---|------|------------------------|--------------|------|----------------------------------------|
| 1 | P1   | foo/bar.rs:42          | FIX          | C    | Thread &AppConfig through snapshot()   |
| 2 | P2   | ios/RustEngine/...     | FIX          | A    | Rebuild xcframework — no source change |
| 3 | P2   | ios/.../X.swift:88     | ALREADY-FIXED| —    | Addressed in c385451                   |
| 4 | P3   | docs/engine/y.md:10    | DECLINE      | —    | Stale — section was removed in commit 19|
```

### 4. Apply FIX entries

`feedback_codex_review_sandwich.md` makes pre-impl + post-impl Codex review the **default for coding rounds**, but Codex-PR-fix sweeps often include changes that carry no logic risk (artifact rebuilds, comment fixes). Pick a review tier per FIX batch using this rubric, and surface the chosen tier in the evaluation table from Step 3.

**Tier A — skip sandwich entirely** (no source-code logic change):

- Pure artifact regeneration (xcframework / `librust_taigi.a` / generated proto bindings) — the underlying source already passed sandwich at the original commit; this commit only contains the regenerated binary.
- Whitespace-only / trailing-newline / EOF cleanup.
- Comment-only edits with no semantic content (typo fix, link update).
- DECLINE / ALREADY-FIXED replies — no diff at all.

**Tier B — post-impl only** (low-risk source change):

- Single-file doc/comment edit that carries semantic content (wrong cross-ref, stale API name).
- Renaming a private symbol or local variable for clarity, no API surface impact.
- Removing a single clearly-dead helper the PR already orphaned (verifiable by grep).
- Test-only edits.

**Tier C — full sandwich** (default for anything else):

- Any source change that affects runtime behavior.
- Multi-file edits.
- Anything touching `engine/dispatch/`, `engine/protos/`, FFI surface (`engine/swift-ffi/`, `engine/android-jni/`), or shared-core boundary.
- Refactors, new code paths, public API additions/removals.
- When in doubt, default to Tier C.

Apply per batch:

- Group fixes by logical unit when natural; otherwise one commit per finding. Mixed-tier fixes in one batch upgrade to the highest tier present.
- Tier C: run **one** combined Codex pre-impl review covering the planned fixes (cite the specific finding IDs).
- Apply fixes.
- Tier B & C: run Codex post-impl review on the resulting diff. Tier A skips post-impl too.
- Commit per `feedback_auto_commit_push.md` (commit + push without asking, no risky ops). Commit message must reference the discussion comment ID, e.g. `(Codex PR #197 r3169707395)`. For Tier A artifact-only commits, the message must explicitly say "regenerated artifact, no source change" so the audit trail reflects why sandwich was skipped.
- If a fix touches Rust under `engine/`, rebuild the xcframework via `engine/scripts/build-xcframework.sh` and include the regenerated artifacts in the commit. Per `feedback_no_rust_ci.md` the gate runs locally — never push without rebuilding. The rebuild itself is Tier A even when the source change is Tier C — bundle if same batch, otherwise separate commit.
- If a fix touches `engine/composing/src/transition.rs` or `api.rs`, also run `cargo test -p composing` (per `feedback_manual_build_test.md` the user runs platform builds, but Rust workspace tests are scripted-safe).

### 5. Reply + resolve on GitHub

For each FIX / ALREADY-FIXED / DECLINE thread:

```
gh api repos/taigikeyboard/taigikeyboard/pulls/<PR>/comments/<firstCommentDatabaseId>/replies \
  -f body="<one-or-two-sentence reply, cite commit SHA when applicable>"
```

Then resolve:

```
gh api graphql -f query='mutation($id:ID!) {
  resolveReviewThread(input:{threadId:$id}) { thread { isResolved } }
}' -F id=<threadId>
```

Reply tone: terse, factual, English. State the change and SHA — the reader is the bot's audit trail, not a human collaborator. Examples:

- FIX: `Fixed in <SHA> — <one-sentence summary of the change>.`
- ALREADY-FIXED: `Already addressed in <SHA>.`
- DECLINE: `Not applicable — <reason>.`

DISCUSS threads: reply with the open question, do NOT resolve.

### 6. Final summary

Print:

- Total threads swept: N (P1: a, P2: b, P3: c)
- Fixed: x (commits: SHA1, SHA2, …)
- Already-fixed: y
- Declined: z
- Discuss (left open): w
- Open follow-ups for the user (DISCUSS list with PR comment URLs)

## Constraints

- Per `feedback_codex_only.md`: never call Gemini or other reviewers; the bot under sweep IS Codex, and the sandwich reviewer is also Codex.
- Per `feedback_workaround_circuit_breaker.md`: if a fix attempt fails review twice, STOP — list it as DISCUSS and surface to the user, do not stack patches.
- Per `feedback_review_before_impl.md` + `feedback_codex_review_sandwich.md`: pre-impl + post-impl Codex review is the default, but Step 4's tier rubric scopes it to Tier C (logic changes). Tier A (artifact regen, comment-only) skips both gates; Tier B (low-risk source edit) runs post-impl only. The rubric only relaxes review *for this PR-fix sweep skill* — general coding rounds still follow the feedback memory's full sandwich rule.
- Per `feedback_auto_commit_push.md`: commit + push without asking on the feature branch; pause for risky ops (push to main, force-push, scope drift). This skill should never push to main.
- Per `feedback_round_hygiene.md`: each Codex sweep is its own coding round when fixes are applied — bundle the swept fixes into commits within the same PR (do NOT open a new PR per finding).
- Project rule #4 (CLAUDE.md): never edit `.xcodeproj` / `.pbxproj` / `build.gradle`. If a finding requires editing one of these files, mark DISCUSS and surface to the user.
