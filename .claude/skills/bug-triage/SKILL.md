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
- Fixed reports: `label:"開發/問題回報/已修復"`.
- Feature requests: `label:"開發/功能建議"`.
- List query = `label:"開發/問題回報" -label:"開發/問題回報/已修復" -label:"開發/功能建議"` (open bug queue = reported, not fixed, not yet categorized as a feature).
- This label also auto-catches ECPay / 藍新 payment mail — the helper skips those (keyword filter) and reports the skipped count.

## Commands

Run from the repo root:

```
python3 .claude/skills/bug-triage/gmail_cli.py list [n]       # list unfixed reports (default 20)
python3 .claude/skills/bug-triage/gmail_cli.py body <id>      # full headers + plaintext body
python3 .claude/skills/bug-triage/gmail_cli.py fixed <id>..   # add "開發/問題回報/已修復"
python3 .claude/skills/bug-triage/gmail_cli.py feature <id>.. # add "開發/功能建議" (feature request)
python3 .claude/skills/bug-triage/gmail_cli.py auth           # verify/refresh token
python3 .claude/skills/bug-triage/gmail_cli.py labels         # debug: list all labels + ids
```

## Triage flow

1. `list` → show unfixed reports (date / sender / subject / snippet). End-user reports have subject `台語齒盤 …回報 (vX.Y.Z)`.
2. `body <id>` → read the full report for any that need detail.
3. Classify each: a real bug whose fix is merged → `fixed <id>`; a feature request → `feature <id>`; a still-open real bug → leave as-is (`開發/問題回報` is the open-queue state).

## Notes

- Do NOT `fixed` a report until its fix is merged — the label is the "done" signal.
- New machine / expired token → the run opens a browser for consent (installed-app loopback), then caches the token. Re-consent is **interactive** — the user runs it (e.g. type `! python3 .claude/skills/bug-triage/gmail_cli.py auth` in the session), the agent cannot complete the browser step.
- **7-day expiry**: the OAuth consent screen is in "testing" publishing status, so refresh tokens expire ~7 days after issue (`invalid_grant: Token has been expired or revoked`). The helper auto-falls-back to the login flow when that happens. Durable fix (optional, user-side): publish the consent screen to "In production" in the Google Cloud console (`alexs-project-361418`) so refresh tokens stop expiring.
- A bug surfaced here becomes a per-incident round per project CLAUDE.md Core Principle #4 (confirm root cause before fixing). See memory `reference_gmail_bug_triage`.
