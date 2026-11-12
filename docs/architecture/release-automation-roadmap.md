# Desktop release automation — roadmap

> **Type**: Planning (multi-PR)
> **Keywords**: `release`, `release train`, `CalVer`, `changelog fragments`, `GitHub App`, `schedule`
> **Status**: Accepted 2026-11-12 — Phase 0

## Goal

The desktop train (macOS + Windows + Linux + ChromeOS) releases itself every month with no person in the loop (maintainer, 2026-11-12: "release notes are no longer reviewed by hand in the AI era; auto-cut, auto-publish on the 5th; everything automated unless I hold it for a hotfix"):

| Day (Asia/Taipei 09:00) | What runs | Person |
|---|---|---|
| 1st | Cut: version `YYYY.M.0`, changelog assembled from fragments, release commit on `main`, every platform staged on the draft, beta page + Discord | none |
| 1st–5th | Beta testers use the build from `/beta` | optional |
| 5th | Publish the draft → announcement (R2, website, appcasts) → Chrome Web Store upload → Homebrew cask bump | none |
| any day | Hold the train, or ship a hotfix patch | maintainer |

Decisions (maintainer, 2026-11-12):

- **D1 Scope** — desktop + ChromeOS, including the Chrome Web Store upload and the Homebrew cask bump. Mobile stays manual (`release-mobile`); revisit after the desktop train has run 1–2 cycles.
- **D2 Release notes without an LLM** — no model runs in CI. Each pull request carries a changelog fragment; the cut only assembles them. GitHub Models (free inference from Actions) was retired 2026-07-30, and a Copilot or API-key step would add a cost and an outage path to the cut.
- **D3 Writing to `main`** — a GitHub App on the `main` ruleset's bypass list pushes the release commit. Its token also publishes the draft, which matters: a release published with `GITHUB_TOKEN` does not fire `announce-release.yml` (GitHub suppresses workflow events caused by `GITHUB_TOKEN`).
- **D4 Hold + hotfix** — a repository variable `RELEASE_HOLD` stops the train; a hotfix is a patch staged by hand with `/release-desktop <platform>` as today.
- Version scheme: CalVer `YYYY.M.P`, no zero-padded month (decided earlier, memory `project_release_train`).

## Today (grounded in code)

| Step | Today | Where |
|---|---|---|
| Version | `make version-desktop X.Y.Z` → `release_notes.py set-versions --train desktop`; refuses a downgrade | `Makefile:271`, `tools/release_notes.py:614` |
| macOS build number | `MAJOR*10000 + MINOR*100 + PATCH`; minor / patch ≤ 99. CalVer `2026.12.0` → `20261200` > `30900` (3.9.0), so the upgrade order holds | `tools/release_notes.py:468` |
| Changelog | Written by an agent from `git log <base>..HEAD` during `/release-desktop` | `.claude/skills/release-desktop/SKILL.md` § 2, § 4 |
| Stage | `make desktop-release` on the maintainer's Mac: `make macos-release` locally (Developer ID identities from the login keychain, `notarytool` keychain profile), Windows / Linux / ChromeOS dispatched to hosted runners | `tools/release/stage-desktop.sh:93-165`, `macos/scripts/release-app.sh:179-192` |
| Beta | `desktop-beta.yml` dispatched at the end of a stage | `tools/release/stage-desktop.sh:203` |
| Publish | The maintainer presses **Publish release** | `docs/architecture/desktop-release.md` § The flow |
| Announce | `release: published` → `announce-release.yml` | `.github/workflows/announce-release.yml:15-17` |
| Chrome Web Store | Manual upload in the Developer Dashboard | `docs/architecture/chromeos-release.md` § Upload |
| Homebrew | Manual edit of `Casks/taigikeyboard.rb` in `taigikeyboard/homebrew-tap` | `docs/architecture/desktop-release.md` § The flow |

The macOS half is the only one bound to a person's machine; every other half already runs on hosted runners.

## Phases

One phase = one PR unless marked admin.

| # | Phase | Change | Needs from the maintainer | Status |
|---|---|---|---|---|
| 0 | Plan | This doc + project memory (admin lane) | — | In progress |
| 1 | CalVer | `set-versions --train desktop` accepts `YYYY.M.P` (month 1–12, no leading zero, year ≥ 2026) and keeps the downgrade guard; `check-versions` and the macOS build number follow; `desktop-release.md` § Version numbers rewritten (full release `YYYY.M.0`, patch `YYYY.M.P`) | — | Pending |
| 2 | Changelog fragments | `changelog/unreleased/<slug>.md` — front matter `platforms:` (macos / windows / linux / chromeos) + `section:` (Input / Candidates / Settings / Appearance / Updates / Install), body one bullet with `(#PR)`. `release_notes.py assemble --train desktop --version V` writes `changelog/desktop-vV.md` in today's shape (lead paragraph listing the platforms touched, `###` per platform, `####` per section, "No changes." for an empty platform) + the `CHANGELOG.md` link, then deletes the fragments. `checks.yml` fails a PR that changes desktop-shipping paths without a fragment, unless labelled `no-changelog`. `/release-desktop` assembles instead of writing; AGENTS.md § Conventions names the rule | — | Pending |
| 3 | Release identity + hold | GitHub App `taigikeyboard-release` (contents + actions write on this repo, contents write on `homebrew-tap`); on the bypass list of the `main` and release-tag rulesets; `RELEASE_APP_ID` (variable) + `RELEASE_APP_PRIVATE_KEY` (secret) in the `release` environment; tokens minted with `actions/create-github-app-token`. Repository variable `RELEASE_HOLD` documented | Create the App, install it, add it to both rulesets' bypass lists, store the two secrets | Pending |
| 4 | macOS on a hosted runner | `macos-build.yml` (dispatch, `source_sha` like the Linux / ChromeOS builds): imports both Developer ID `.p12` into a temporary keychain, notarizes with an App Store Connect API key, attaches the `.pkg` to the draft. `stage-desktop.sh` dispatches it like the other three halves, so staging needs no Mac. `macos/scripts/release-app.sh` takes the API key in place of the keychain profile when it is set | Export both `.p12` (+ password) and an App Store Connect Team API key into the `release` environment: secrets `MACOS_DEVELOPER_ID_APPLICATION_P12`, `MACOS_DEVELOPER_ID_INSTALLER_P12` (base64), `MACOS_P12_PASSWORD`, `APPLE_API_KEY_P8`; variables `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID` | Pending |
| 5 | Cut on the 1st | `desktop-train.yml`, `schedule: '0 1 1 * *'` + `workflow_dispatch`: stop if `RELEASE_HOLD` is set; stop with a Discord note if there are no fragments (empty month, no release); `set-versions` to `YYYY.M.0`, `assemble`, commit + push with the App token, then `stage-desktop.sh` | — | Pending |
| 6 | Publish on the 5th | `desktop-publish.yml`, `schedule: '0 1 5 * *'` + `workflow_dispatch`: stop if `RELEASE_HOLD` is set; require the month's draft to carry every expected asset + `.sha256` and to name the commit the cut made; publish with the App token (fires `announce-release.yml`); a failure posts to Discord | — | Pending |
| 7 | Stores | Chrome Web Store API upload + submit, and the Homebrew cask bump (version + sha256, `brew audit`), as jobs after the announcement in `announce-release.yml`; each runs only when the release carries its asset | Chrome Web Store API V2 (the V1 API stopped 2026-10-15) through a Google Cloud service account added in the Developer Dashboard: secret `CWS_SERVICE_ACCOUNT_JSON`, variables `CWS_PUBLISHER_ID`, `CWS_EXTENSION_ID`. The store item must exist first (V2 cannot create one), so the first upload stays manual | Pending |
| 8 | Hand-off docs | `desktop-release.md` § The flow rewritten around the schedule; `/release-desktop` narrowed to a hotfix patch; memory `feedback_release_manual_by_user` replaced | — | Pending |

Order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8. Phases 1 and 2 need nothing from the maintainer and come first; 3, 4 and 7 wait on credentials. Until phase 4 merges, the cut cannot run unattended (the macOS half needs the Mac); until phase 6 merges, publishing stays a button press.

## Hold and hotfix (D4)

| Situation | What the maintainer does | Effect |
|---|---|---|
| Something is wrong with this month's build | `gh variable set RELEASE_HOLD --body "<reason>"` | The 5th publishes nothing; the 1st cuts nothing while it is set |
| Resume | `gh variable delete RELEASE_HOLD`, then fix on `main` and re-run the cut (`gh workflow run desktop-train.yml`) — it re-stages the draft from the new commit — or let the next month's cut pick it up | Publishing follows on the next scheduled 5th, or `gh workflow run desktop-publish.yml` |
| A published release needs a fix | Fix on `main`, `make version-desktop YYYY.M.P`, `/release-desktop <platform>`, publish by hand | One-platform patch as today; the train is untouched |

Open for phase 1 / 5: a patch while the month's draft is staged (between the 1st and the 5th) would number below the tree's `YYYY.M.0`, which the downgrade guard refuses. Proposed: in that window the fix goes onto `main` and the train draft is re-cut instead of a separate patch.

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| Per-change changelog fragments assembled at release | changesets (`.changeset/*.md`), towncrier (`newsfragments/`) | Phase 2 |
| Fixed-cadence release train, ship what is ready | Chrome / Firefox rapid release trains | Phases 5–6 |
| App installation token for automation that must bypass branch protection or trigger workflows | GitHub docs: "Triggering a workflow from a workflow" (`GITHUB_TOKEN` events do not start runs); `actions/create-github-app-token` | Phase 3 |
| Temporary keychain + notarytool API key for CI signing | Apple notarytool `--key/--key-id/--issuer`; GitHub docs "Installing an Apple certificate on macOS runners" | Phase 4 |

Rules: `planning.md` § Persistent hand-off (this doc + memory), § No redundant fallback (phase 4 replaces the local macOS half rather than keeping both paths); `round-workflow.md` § Codex review sandwich — phases 3–6 are High tier (release tooling, credentials).

Deliberately not adopted:

- **LLM-written notes in CI** — D2.
- **release-please / notes from PR titles** — titles are written for developers; refactor and CI PRs would need labels to drop out. Fragments carry user-facing text written when the change is understood best.
- **Bot PR for the release commit** — `main` needs code-owner approval, so a PR would wait for a person; the App bypass is the same trust with no wait.
- **Keeping the local macOS stage as a fallback** — two staging paths for one artifact; phase 4 makes the hosted build the only one.
