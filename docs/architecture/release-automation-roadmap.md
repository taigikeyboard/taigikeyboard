# Desktop release automation — roadmap

> **Type**: Planning (multi-PR)
> **Keywords**: `release`, `release train`, `CalVer`, `generate-notes`, `GitHub App`, `schedule`
> **Status**: Accepted 2026-11-12 — all phases merged; first hosted runs 2026-12-01 (cut) and 2026-12-05 (publish)

## Goal

The desktop train (macOS + Windows + Linux + ChromeOS) releases itself every month; the only person in the loop approves the jobs that read release secrets (D5) (maintainer, 2026-11-12: "release notes are no longer reviewed by hand in the AI era; auto-cut, auto-publish on the 5th; everything automated unless I hold it for a hotfix"):

| Day (Asia/Taipei 09:00) | What runs | Person |
|---|---|---|
| 1st | Cut: version `YYYY.M.0`, release commit on `main`, every platform staged on the draft, beta page + Discord | approves each `release`-environment job (D5) |
| 1st–5th | Beta testers use the build from `/beta` | optional |
| 5th | Publish the draft → announcement (R2, website, appcasts) → Homebrew cask bump (the Chrome Web Store upload stays manual, D1) | approves each `release`-environment job (D5) |
| any day | Hold the train, or ship a hotfix patch | maintainer |

Decisions (maintainer, 2026-11-12):

- **D1 Scope** — desktop + ChromeOS, including the Homebrew cask bump. The Chrome Web Store upload stays manual for now (maintainer, 2026-11-14: "I'll handle the Chrome Web Store by hand for now"). Mobile stays manual (`release-mobile`); revisit after the desktop train has run 1–2 cycles.
- **D2 Release notes = GitHub's generated notes** (maintainer, 2026-11-13: "nobody reads the release notes") — the release body is the Release Notes API's list of pull requests merged since the previous desktop release, nothing hand-written, no per-PR step, no model in CI. Replaces the fragment design (closed PR #898). GitHub Models (free inference from Actions) was retired 2026-07-30, and a Copilot or API-key step would add a cost and an outage path to the cut.
- **D3 Writing to `main`** — a GitHub App on the `main` ruleset's bypass list pushes the release commit. Its token also publishes the draft, which matters: a release published with `GITHUB_TOKEN` does not fire `announce-release.yml` (GitHub suppresses workflow events caused by `GITHUB_TOKEN`).
- **D4 Hold + hotfix** — a repository variable `RELEASE_HOLD` stops the train; a hotfix is a patch staged by hand with `/release-desktop <platform>` as today.
- Version scheme: CalVer `YYYY.M.P`, no zero-padded month (decided earlier, memory `project_release_train`).
- **D5 Approval gate** (maintainer, 2026-11-14: "I'd rather approve; at least the build no longer has to run on my machine") — the `release` environment has the maintainer as required reviewer again, so every job that reads release secrets (cut push, macOS signing, beta, publish, announcement) waits for one click. Reason: maintainers may merge their own pull requests without approval (ruleset bypass, kept on purpose), so the branch gate alone would let a merged workflow read the secrets. Schedules still start the train; phases 5–6 should group the secret-using jobs into as few runs as possible to keep the clicks few. Phase 5: two approvals per cut (`cut`, `sign-and-beta`), maintainer 2026-11-14.

## Today (grounded in code)

| Step | Today | Where |
|---|---|---|
| Version | `make version-desktop X.Y.Z` → `release_notes.py set-versions --train desktop`; refuses a downgrade | `Makefile:271`, `tools/release_notes.py:614` |
| macOS build number | `MAJOR*10000 + MINOR*100 + PATCH`; minor / patch ≤ 99. CalVer `2026.12.0` → `20261200` > `30900` (3.9.0), so the upgrade order holds | `tools/release_notes.py:468` |
| Changelog | Written by an agent from `git log <base>..HEAD` during `/release-desktop` (up to 3.9.0) | `.claude/skills/release-desktop/SKILL.md` § 2, § 4 |
| Stage | `make desktop-release` on the maintainer's Mac: `make macos-release` locally (Developer ID identities from the login keychain, `notarytool` keychain profile), Windows / Linux / ChromeOS dispatched to hosted runners | `tools/release/stage-desktop.sh:93-165`, `macos/scripts/release-app.sh:179-192` |
| Beta | `desktop-beta.yml` dispatched at the end of a stage | `tools/release/stage-desktop.sh:203` |
| Publish | The maintainer presses **Publish release** | `docs/architecture/desktop-release.md` § The flow |
| Announce | `release: published` → `announce-release.yml` | `.github/workflows/announce-release.yml:15-17` |
| Chrome Web Store | Manual upload in the Developer Dashboard | `docs/architecture/chromeos-release.md` § Upload |
| Homebrew | Manual edit of `Casks/taigikeyboard.rb` in `taigikeyboard/homebrew-tap` | `docs/architecture/desktop-release.md` § The flow |

The macOS half is the only one bound to a person's machine; every other half already runs on hosted runners. (Baseline before phase 4, kept as the plan's starting point; phase 4 moved the macOS half to `macos-build.yml`.)

## Phases

One phase = one PR unless marked admin.

| # | Phase | Change | Needs from the maintainer | Status |
|---|---|---|---|---|
| 0 | Plan | This doc + project memory (admin lane) | — | Merged (direct to main 2026-11-12) |
| 1 | CalVer | `set-versions --train desktop` accepts `YYYY.M.P` (month 1–12, no leading zero, year ≥ 2026) and keeps the downgrade guard; `check-versions` and the macOS build number follow; `desktop-release.md` § Version numbers rewritten (full release `YYYY.M.0`, patch `YYYY.M.P`). CalVer only from 3.9.0 on — a patch before the first train is `2026.11.P` (maintainer, 2026-11-13) | — | Merged #895 |
| 2 | Generated release notes | Staging (`tools/release/lib/desktop-release.sh` `_write_release_notes`) asks the Release Notes API (`POST /repos/{owner}/{repo}/releases/generate-notes`) for the body, from the previous published `desktop-*` release to the staged commit, and appends the Code signing policy link. No `changelog/desktop-v*.md` after 3.9.0; `CHANGELOG.md` points at the release pages. `/release-desktop` stops writing a changelog | — | Merged #899 |
| 3 | Release identity | GitHub App `taigikeyboard-app` (shared with the accent-contribution intake), tokens minted with `actions/create-github-app-token`; `release-identity.yml` (dispatch only) checks the setup. Setup: `desktop-release.md` § One-time: the release App | Create the App, install it, add it to both rulesets' bypass lists, store the variable and the secret (done 2026-11-14) | Merged #900; `release-identity.yml` green 2026-11-14 |
| 4 | macOS on a hosted runner | `macos-build.yml` (dispatch, `source_sha` like the Linux / ChromeOS builds): imports both Developer ID `.p12` into a temporary keychain, notarizes with an App Store Connect API key, attaches the `.pkg` to the draft. `stage-desktop.sh` dispatches it like the other three halves, so staging needs no Mac. `macos/scripts/release-app.sh` takes the API key in place of the keychain profile when it is set; `make macos-release` and `release-app.sh --publish` are removed (no local staging path) | Export both `.p12` (+ password) and an App Store Connect Team API key into the `release` environment: secrets `MACOS_DEVELOPER_ID_APPLICATION_P12`, `MACOS_DEVELOPER_ID_INSTALLER_P12` (base64), `MACOS_P12_PASSWORD`, `APPLE_API_KEY_P8`; variables `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID` | Merged #904 (credentials stored 2026-11-14; build / signing split into two jobs, no credential on a machine that runs third-party code) |
| 5 | Cut on the 1st | `desktop-train.yml`, `schedule: '0 1 1 * *'` + `workflow_dispatch` (`dry_run`: plan + every build from a scratch branch, nothing pushed, staged or signed, no approval). `plan` (no secret): stop if `RELEASE_HOLD` is set; stop with a Discord note (repository webhook `DISCORD_PR_WEBHOOK`, no approval) if the generated notes list no pull request (empty month); `set-versions` to `YYYY.M.0`, commit as `github-actions[bot]`, push a scratch branch with `GITHUB_TOKEN`. **Approval 1** `cut`: App token fast-forwards `main` to that one commit. `draft` (`reset_desktop_draft`, shared with `stage-desktop.sh`), then the four build workflows as `workflow_call` (each checks out the staged commit). **Approval 2** `sign-and-beta` on one macOS VM: signs + notarizes the `.pkg` (composite `macos-sign-attach`, shared with `macos-build.yml`), checks every asset, mirrors `/beta` (composite `desktop-beta`). A month whose release commit is already on `main` re-stages from `main` (one approval). Failures post to Discord. Script: `tools/release/desktop-train.sh` | — | Merged #914 (no hosted run yet) |
| 6 | Publish on the 5th | `desktop-publish.yml`, `schedule: '0 1 5 * *'` + `workflow_dispatch`; gate rules in `desktop-release.md` § The flow (Publish row). `check` (no secret, `desktop-train.sh check-publish`) emits the draft's fingerprint (tag, target, draft flag, asset ids); **approval** `publish` (no checkout) re-reads it and publishes with the App token, which fires `announce-release.yml` — two approvals on the 5th (maintainer 2026-11-14). No concurrency group shared with the cut: a publish waiting on approval must not block a re-stage, which replaces the draft anyway. A failure posts to Discord | — | Merged #917 |
| 7 | Homebrew | `announce-release.yml`: once the announcement succeeded for a release carrying a macOS `.pkg`, the same (already approved) job runs `tools/release/bump-cask.sh` (cask `version` + `sha256` through the contents API) with an App token limited to `homebrew-tap` contents, minted as the job's first step so the App key never shares a job with repository code; then `cask-audit` (no secret, `macos-26`) runs `brew audit --cask --strict --online`. Push first, audit after: a separate approved job would be a third approval on the 5th (maintainer 2026-11-14). An announce or audit failure posts to Discord. The Chrome Web Store upload stays manual (D1, maintainer 2026-11-14) | — (the App already reaches `homebrew-tap`, phase 3) | Merged #921 + fix #922; verified 2026-11-14 (cask 3.9.0, audit green, run 43591499881) |
| 8 | Hand-off docs | `desktop-release.md` § The flow rewritten around the schedule (phases 5–7) + § Hold, hotfix, re-stage; `/release-desktop` narrowed to a hotfix patch; platform docs and announcement comments name the schedule; memory `feedback_release_manual_by_user` replaced | — | In progress |

Order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8. Phases 1 and 2 need nothing from the maintainer and come first; 3 and 4 wait on credentials. Until phase 4 merges, the cut cannot run unattended (the macOS half needs the Mac); until phase 6 merges, publishing stays a button press.

## Hold and hotfix (D4)

The procedure (hold, resume, re-stage, hotfix) and the tree-version rule live in [`desktop-release.md` § Hold, hotfix, re-stage](desktop-release.md#hold-hotfix-re-stage) since phase 8; `/release-desktop` stages only the hotfix patch.

A fix while the month's draft is staged (between the 1st and the 5th) goes onto `main`, then `gh workflow run desktop-train.yml` re-stages the draft from `main`'s tip (the tree already carries `YYYY.M.0`, so no version commit and only the signing approval). A separate patch would number below the tree's `YYYY.M.0`, which the downgrade guard refuses (phase 5, 2026-11-14).

Empty month: the generated notes list every merged pull request, Dependabot's included, and do not filter by path, so a month with only dependency bumps or mobile changes still cuts (D2: no label-based filtering).

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| Release notes generated from merged pull requests | GitHub docs "Automatically generated release notes" (Release Notes API); release-please / git-cliff derive notes from history the same way | Phase 2 |
| Fixed-cadence release train, ship what is ready | Chrome / Firefox rapid release trains | Phases 5–6 |
| App installation token for automation that must bypass branch protection or trigger workflows | GitHub docs: "Triggering a workflow from a workflow" (`GITHUB_TOKEN` events do not start runs); `actions/create-github-app-token` | Phase 3 |
| Temporary keychain + notarytool API key for CI signing | Apple notarytool `--key/--key-id/--issuer`; GitHub docs "Installing an Apple certificate on macOS runners" | Phase 4 |

Rules: `planning.md` § Persistent hand-off (this doc + memory), § No redundant fallback (phase 4 replaces the local macOS half rather than keeping both paths); `round-workflow.md` § Codex review sandwich — phases 3–6 are High tier (release tooling, credentials).

Deliberately not adopted:

- **LLM-written notes in CI** — D2.
- **Per-PR changelog fragments** (changesets, towncrier) — user-facing wording at the cost of one file per PR and a CI check; built as PR #898 and closed, since nobody reads the notes (D2).
- **Filtering the generated list** (`.github/release.yml` categories / exclusions) — needs a label on every PR; not worth it for notes nobody reads.
- **Bot PR for the release commit** — `main` needs code-owner approval, so a PR would wait for a person; the App bypass is the same trust with no wait.
- **Keeping the local macOS stage as a fallback** — two staging paths for one artifact; phase 4 makes the hosted build the only one.
