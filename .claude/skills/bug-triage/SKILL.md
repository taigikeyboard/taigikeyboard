---
name: bug-triage
description: Triage Taigi Keyboard user bug reports from Gmail. Lists UNFIXED reports under label "開發/問題回報" (excludes "開發/問題回報/已修復", skips payment noise), reads a report's full body, marks a report fixed, or marks a report a feature request. Stdlib-only OAuth via client_secret.json — NOT gcloud (gcloud cannot mint a Gmail token).
disable-model-invocation: false
---

# Bug Triage

Read + label Taigi Keyboard user bug reports in Gmail. Backed by `gmail_cli.py` (this dir) — stdlib-only, no third-party deps.

## Auth model (important)

- **gcloud CLI does NOT work for Gmail.** `gcloud auth login` only grants the cloud-platform scope. Gmail needs an OAuth **installed-app** token. The helper mints/refreshes that token itself.
- Secret: `client_secret.json` at repo root (gitignored, `installed` type, project `alexs-project-361418`, Gmail API enabled). Override with `GMAIL_CLIENT_SECRET=<path>`.
- Token cached at `~/.config/gmail-cli/token.json`, scope `gmail.modify`, has refresh_token → re-login is rare. Helper refreshes the access token on every run and on a mid-run 401.

## Labels (CJK house style — do NOT change the query syntax)

Label taxonomy is all-CJK type names, hierarchical `parent/child[/leaf]` (matches the whole Gmail account — e.g. `銀行/中國信託/登入通知`). English is reserved for proper-noun brands only. The dev subtree was CJK-aligned 2026-06-29:

- Unfixed user reports: `label:"開發/問題回報"` — quoted + hierarchical. An unquoted `label:開發-問題回報` **fails** (Gmail reads `-`/spaces oddly); always quote.
- Fixed reports: `label:"開發/問題回報/已修復"` — ONLY when a merged fix exists.
- Not reproducible: `label:"開發/問題回報/無法重現"` — the symptom could not be reproduced on the current build (no fix was made). USER rule 2026-08-20: NOT-repro is never 已修復.
- Feature requests: `label:"開發/功能建議"`.
- List query = `label:"開發/問題回報" -label:"開發/問題回報/已修復" -label:"開發/功能建議" -label:"開發/問題回報/無法重現"` (open bug queue = reported, not fixed, not a feature, not closed as unreproducible).
- This label also auto-catches ECPay / 藍新 payment mail — the helper skips those (keyword filter) and reports the skipped count.

## Commands

Run from the repo root:

```
python3 .claude/skills/bug-triage/gmail_cli.py list [n]       # list unfixed reports (default 20)
python3 .claude/skills/bug-triage/gmail_cli.py body <id>      # full headers + plaintext body
python3 .claude/skills/bug-triage/gmail_cli.py fixed <id>..   # add "開發/問題回報/已修復"
python3 .claude/skills/bug-triage/gmail_cli.py feature <id>.. # add "開發/功能建議" (feature request)
python3 .claude/skills/bug-triage/gmail_cli.py unresolved <id>.. # add "開發/問題回報/無法重現" (NOT-repro; also clears a mis-applied 已修復)
python3 .claude/skills/bug-triage/gmail_cli.py auth           # verify/refresh token
python3 .claude/skills/bug-triage/gmail_cli.py labels         # debug: list all labels + ids
```

## Mail traceability (mandatory in any written output)

USER reads the reports in Gmail, so every bug item written to a doc, memory file, PR body, or
chat summary MUST carry enough to find the mail without re-running the CLI. Record all five:

| Field | Where it comes from |
|---|---|
| Sender name + address | `from:` line of `list` / `body` |
| Subject (verbatim, incl. the `(vX.Y.Z)` suffix) | `subj:` |
| Date | `date:` |
| Gmail message id | `id=` |
| Gmail deep link | `link:` — `https://mail.google.com/mail/u/0/#all/<id>` |

`list` and `body` both print the deep link. A Gmail search string
(`from:<address> subject:"<subject>"`) is a useful sixth field for backlog tables — it survives
a message id changing hands better than the link does.

Reference shape: the mail-index table at the top of `docs/reports/user-bug-backlog-2026-08-18.md`.
Never write a bug item as "zw, 2026-04-21" alone — the sender address and message id are what
make it findable.

**Also cross-check the reported version against the changelog** before calling an item "already
fixed": if the fix PR shipped in a release **older than** the reported version, the reporter
already had it and the item is live, not a dogfood leftover. (2026-08-19: this refuted the
"already fixed" premise on two backlog items.)

## Triage flow

1. `list` → show unfixed reports (date / sender / subject / snippet). End-user reports have subject `台語齒盤 …回報 (vX.Y.Z)`.
2. `body <id>` → read the full report for any that need detail.
3. Classify each: a real bug whose fix is merged → `fixed <id>`; a feature request → `feature <id>`; symptom not reproducible on the current build → `unresolved <id>`; a still-open real bug → leave as-is (`開發/問題回報` is the open-queue state).

## Notes

- Do NOT `fixed` a report until its fix is merged — the label is the "done" signal.
- New machine / expired token → the run opens a browser for consent (installed-app loopback), then caches the token. Re-consent is **interactive** — the user runs it (e.g. type `! python3 .claude/skills/bug-triage/gmail_cli.py auth` in the session), the agent cannot complete the browser step.
- **7-day expiry**: the OAuth consent screen is in "testing" publishing status, so refresh tokens expire ~7 days after issue (`invalid_grant: Token has been expired or revoked`). The helper auto-falls-back to the login flow when that happens. Durable fix (optional, user-side): publish the consent screen to "In production" in the Google Cloud console (`alexs-project-361418`) so refresh tokens stop expiring.
- A bug surfaced here becomes a per-incident round per project CLAUDE.md Core Principle #4 (confirm root cause before fixing). See memory `reference_gmail_bug_triage`.
