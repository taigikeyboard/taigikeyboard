# Accent-variation contribution site — roadmap

> **Type**: Planning (multi-PR)
> **Keywords**: `腔口差`, `accent variations`, `contribution`, `issue form`, `GitHub App`, `khpoo`, `齒盤補充辭典`
> **Status**: Draft 2026-11-12 — Phase 1 in review; all decisions settled

## Goal

A minimal contribution site, styled like the web version: anyone with a GitHub account searches TaigiKeyboard's dictionary from a kautian-mirror-like search bar and, when a word or reading is missing, submits Hanji + TL + accent + a reason. The submission becomes a pull request that appends the entry to 齒盤補充辭典 (khpoo); CI validates it and rejects duplicates; the maintainer merges. The contributor's GitHub account appears in the repository's Contributors.

Decisions (maintainer, 2026-11-12):

- **D1 Submit path** — a static site cannot finish a GitHub OAuth login on its own: PKCE is supported since 2025-07, but the token endpoint has no CORS, and the GitHub API or `gh pr create` both need a token. The site therefore opens a prefilled GitHub issue form; a workflow in this repository runs `gh pr create` with a GitHub App token. One extra **Create** click on github.com, no backend, no secret in the browser.
- **D2 Contributor credit** — "I want the Contributors list to include the GitHub accounts that submitted accent data." The intake commit's author is the contributor (`<user.id>+<login>@users.noreply.github.com`, from the issue event); the App is only the committer and the PR opener. The PR is merged with a merge commit or rebase, not squash, so the author survives onto `main`. The `main` ruleset (`deletion`, `non_fast_forward`, `pull_request`) requires no signed commits, so an App-committed, contributor-authored commit is accepted. GitHub's Contributors graph counts non-merge commits on the default branch whose author email belongs to an account (the GitHub noreply address counts, so contributors who hide their email still match) and commit co-authors; it shows the top 100 and can lag ~24 h. **Acceptance**: after the first merged contribution, the submitter's account is listed at `github.com/taigikeyboard/taigikeyboard/graphs/contributors`.
- **D3 Accent categories** — the ten Kautian accents (`dictionary/sources/official/kautian/config.yaml:35-45`, 鹿港偏泉腔 … 臺中偏漳腔), multi-select, plus 其他 with a free-text place name.
- **D4 Site location** — a separate repository (like `taigikeyboard/kautian-mirror`), published under `taigikeyboard.tw/<name>/`; its build reads this repository's dictionary data. The intake workflow and the checks stay here.
- **D5 Data destination** — append to `dictionary/sources/supplementary/khpoo/data/raw/khpoo.csv`. Entries ship as 齒盤補充辭典 (source bit 8) through the existing khpoo pipeline; no new data file, no new source bit, no engine or platform change. The accent and the reason are not in the data (the file has two columns); they are recorded as commit trailers and in the PR body.

- **D6 TL, reviewed by hand** — the romanization field asks for Tâi-lô only (label and placeholder say so). Whether the TL is correct is decided by the maintainer's review of the PR: only entries that pass review are merged. No automated TL validator in the site or in CI — the reviewer is the gate, and a machine check would add a second, possibly disagreeing, definition of TL.
- **D7 Licence of contributed rows** — Apache-2.0, like `dev` (`dictionary/LICENSE:34`). khpoo's original rows stay **Unverified**; contributed rows are identified by their `Accent:` commit trailer. The form carries a required consent checkbox. Recorded in `dictionary/LICENSE` and `dictionary/docs/SOURCES.md`.

## Today (grounded in code)

| Piece | Today | Where |
|---|---|---|
| khpoo raw | Headerless `hanzi,tl`, 4,202 lines, every line CRLF, not sorted | `dictionary/sources/supplementary/khpoo/data/raw/khpoo.csv` |
| khpoo pipeline | `extract` (`dedup: true`) → `cleanup.minimal` (only `normalize_roman`: NFC + lowercase + space→hyphen; no hanlo / length checks) → … → `source` (bit 8) | `dictionary/sources/supplementary/khpoo/config.yaml`, `README.md` |
| Regeneration | `./run.sh` rewrites `khpoo/data/khpoo.csv`; `build.sh` rebuilds `dictionary.csv` + `assets/dictionaries/` (release-time `make dict`) | `dictionary/README.md` § Quick start |
| Search data | `dictionary/output/dictionary.csv` is tracked (~170k rows, per-source flags, POJ / TL / no-tone keys) | `dictionary/output/dictionary.csv` |
| Search UI reference | kautian-mirror search bar and engine | `taigikeyboard/kautian-mirror` `extension/src/search/engine.js`, `src/suggest.js` |
| Issue templates | bug report, feature request | `.github/ISSUE_TEMPLATE/` |

## Appended row and commit

```
八,pueh\r\n
```

Commit (author = contributor):

```
feat(khpoo): add 八 pueh

Accent: 鹿港偏泉腔, 金門偏泉腔
Reason: <contributor's reason>
Closes #<issue>
```

- One row per `(hanzi, tl)`; several accents go into one `Accent:` trailer.
- The TL is normalised before it is written, with the pipeline's own `normalize_roman`, so the raw file holds what `cleanup.minimal` would produce.
- Provenance query: `git log --format='%an %(trailers:key=Accent)' -- dictionary/sources/supplementary/khpoo/data/raw/khpoo.csv`.

## Phases

| Phase | PR | Scope | Status |
|---|---|---|---|
| 0 | admin | This roadmap + project memory | Done (533e2d093, 59bc9b98f) |
| 1 | this repo | `dictionary/tools/verify_khpoo_append.py` + CI job + licence record | In review |
| 2 | this repo | Issue form + intake workflow + GitHub App wiring | Pending |
| 3 | new repo | Static site: search + form + Pages deploy | Pending |

### Phase 1 — checks

`dictionary/tools/verify_khpoo_append.py` (its docstring is the check list), run by `python.yml` on PRs that change the khpoo raw file against the merge base (`HEAD^1`): append-only, format, and `(hanzi, tl)` dedup against khpoo and `dictionary/output/dictionary.csv` keyed like the build merge (space = hyphen). No TL correctness check (D6): the reviewer's merge is the only gate on TL. Tests: `dictionary/tests/test_verify_khpoo_append.py`.

The intake workflow runs the same script before it commits, so a duplicate or malformed submission is answered on the issue and never becomes a PR. Everything else becomes a PR and waits for review.

### Phase 2 — intake

- `.github/ISSUE_TEMPLATE/khiunnkhau.yml`: inputs `hanzi`, `tl`, `reason`; accents (ten + 其他, multi), `other_place`; a required licence checkbox (D7).
- Spike first (≤20 lines): does an issue-form URL prefill a dropdown / checkbox by field `id`? The docs name `id` as the prefill key but show no dropdown example. If not, accents travel in a prefilled text input that the workflow validates.
- `.github/workflows/khiunnkhau-intake.yml`, on `issues: opened` with the form's label:
  1. Parse the form body; run `verify_khpoo_append.py`; on failure comment the reason and close as not planned.
  2. Branch `khiunnkhau/issue-<n>`, append the row (CRLF), commit with `--author "<login> <id+login@users.noreply.github.com>"` and the trailers above, push with the App token.
  3. `gh pr create` (body: row, accents, reason, `Closes #<n>`), comment the PR link on the issue.
- Token: a GitHub App (contents + pull requests + issues write on this repository). A PR opened with `GITHUB_TOKEN` does not trigger `pull_request` workflows, so Phase 1's check would never run. The release-automation App (`architecture/release-automation-roadmap.md` D3) can carry these permissions, or a separate App — maintainer setup either way.
- Merge with a merge commit or rebase (D2).
- Abuse guard: input lengths capped; form fields treated as data (passed through env to a script file, never interpolated into shell).
- Two open intake PRs both append at the end → the second conflicts after the first merges. The workflow rebases open `khiunnkhau/*` branches on push to `main` (re-append), keeping the contributor as author.

### Phase 3 — site

- Repo `taigikeyboard/<name>`, Pages under `taigikeyboard.tw/<name>/`; style tokens and fonts from `web/src/style.css` / kautian-mirror.
- Build: fetch `dictionary.csv` and the khpoo raw file at a pinned `main` commit → compact JSON (hanzi, tl, poj, no-tone keys). The raw file is included because merged contributions reach `dictionary.csv` only at the next `make dict`.
- Search: Hanji, TL, POJ, toneless; a hit shows its readings and sources. "Add" offered when the exact `(hanzi, tl)` is absent.
- Form: Hanji, TL (D6: labelled Tâi-lô, reviewed by the maintainer after submit), accents (ten + 其他), reason, licence consent → opens the prefilled issue URL. The form says the contributor's GitHub account will appear as the commit author.
- Rebuild daily (the kautian-mirror schedule).

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| Contribution via issue form → bot PR, no backend | GitHub issue forms (`docs.github.com` § Syntax for GitHub's form schema) | Phase 2 |
| OAuth for static sites needs a server-side token exchange | GitHub OAuth docs (no CORS on `/login/oauth/access_token`); Simon Willison, "GitHub OAuth for a static site using Cloudflare Workers" | D1: avoided by issuing in GitHub instead |
| Contributors credited by commit author email | GitHub Docs § About contributors | D2 |
| Static site, data built at deploy, no backend | kautian-mirror `planning.md` § Scope | Phase 3 |

- Rules: `rules/planning.md` (grounded in code, roadmap + memory), `rules/code-review-rules.md` §2 (reuse the pipeline's `normalize_roman` rather than a second normaliser), `rules/round-workflow.md` (one PR per phase).
- Deliberately not adopted: a Cloudflare Worker OAuth login (extra secret and service for one saved click); an accent column in khpoo (breaks the two-column `csv_noheader` contract every stage reads; git trailers carry it); per-accent toggles for contributed rows (maintainer chose khpoo, D5); fuzzy dedup beyond normalisation (a near-duplicate reading may be the accent difference itself).
