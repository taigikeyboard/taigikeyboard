# Accent-variation contribution site — roadmap

> **Type**: Design record (superseded)
> **Keywords**: `腔口差`, `accent variations`, `contribution`, `issue form`, `GitHub App`, `khpoo`, `齒盤補充辭典`
> **Status**: Superseded 2026-11-20 — accent variation is collected in the separate `taigikeyboard/community-dictionary` repository instead (its own roadmap). Only Phase 1 (#873, the khpoo append check) was built and stays; the intake workflow, issue form and site (Phases 2–3) were never built. Kept for decisions D1–D7, which `dictionary/LICENSE` and `verify_khpoo_append.py` cite.

## Goal

The plan was a minimal contribution site, styled like the web version: anyone with a GitHub account searches TaigiKeyboard's dictionary from a kautian-mirror-like search bar and, when a word or reading is missing, submits Hanji + TL + accent + a reason. The submission becomes a pull request that appends the entry to 齒盤補充辭典 (khpoo); CI validates it and rejects duplicates; the maintainer merges. The contributor's GitHub account appears in the repository's Contributors.

Decisions (maintainer, 2026-11-12):

- **D1 Submit path** — a static site cannot finish a GitHub OAuth login on its own: PKCE is supported since 2025-07, but the token endpoint has no CORS, and the GitHub API or `gh pr create` both need a token. The site therefore opens a prefilled GitHub issue form; a workflow in this repository runs `gh pr create` with a GitHub App token. One extra **Create** click on github.com, no backend, no secret in the browser.
- **D2 Contributor credit** — "I want the Contributors list to include the GitHub accounts that submitted accent data." The intake commit's author is the contributor (`<user.id>+<login>@users.noreply.github.com`, from the issue event); the App is only the committer and the PR opener. The PR is merged with a merge commit or rebase, not squash, so the author survives onto `main`. The `main` ruleset (`deletion`, `non_fast_forward`, `pull_request`) requires no signed commits, so an App-committed, contributor-authored commit is accepted. GitHub's Contributors graph counts non-merge commits on the default branch whose author email belongs to an account (the GitHub noreply address counts, so contributors who hide their email still match) and commit co-authors; it shows the top 100 and can lag ~24 h. **Acceptance**: after the first merged contribution, the submitter's account is listed at `github.com/taigikeyboard/taigikeyboard/graphs/contributors`.
- **D3 Accent categories** — the ten Kautian accents (`dictionary/sources/official/kautian/config.yaml`, 鹿港偏泉腔 … 臺中偏漳腔), multi-select, plus 其他 with a free-text place name.
- **D4 Site location** — a separate repository (like `taigikeyboard/kautian-mirror`), published under `taigikeyboard.tw/<name>/`; its build reads this repository's dictionary data. The intake workflow and the checks stay here.
- **D5 Data destination** — append to `dictionary/sources/supplementary/khpoo/data/raw/khpoo.csv`. Entries ship as 齒盤補充辭典 (source bit 8) through the existing khpoo pipeline; no new data file, no new source bit, no engine or platform change. The accent and the reason are not in the data (the file has two columns); they are recorded as commit trailers and in the PR body.

- **D6 TL, reviewed by hand** — the romanization field asks for Tâi-lô only (label and placeholder say so). Whether the TL is correct is decided by the maintainer's review of the PR: only entries that pass review are merged. No automated TL validator in the site or in CI — the reviewer is the gate, and a machine check would add a second, possibly disagreeing, definition of TL.
- **D7 Licence of contributed rows** — Apache-2.0, like `dev` (`dictionary/LICENSE`). khpoo's original rows stay **Unverified**; contributed rows are identified by their `Accent:` commit trailer. The form carries a required consent checkbox. Recorded in `dictionary/LICENSE` and `dictionary/docs/SOURCES.md`.

## Phase 1 — checks (built)

`dictionary/tools/verify_khpoo_append.py` (its docstring is the check list), run by `python.yml` on PRs that change the khpoo raw file against the merge base (`HEAD^1`): append-only, format, and `(hanzi, tl)` dedup against khpoo and `dictionary/output/dictionary.csv` keyed like the build merge (space = hyphen). No TL correctness check (D6): the reviewer's merge is the only gate on TL. Tests: `dictionary/tests/test_verify_khpoo_append.py`.

Phases 2 (issue form + intake workflow on the shared release App) and 3 (static search + form site) were not built.

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| Contribution via issue form → bot PR, no backend | GitHub issue forms (`docs.github.com` § Syntax for GitHub's form schema) | Phase 2 |
| OAuth for static sites needs a server-side token exchange | GitHub OAuth docs (no CORS on `/login/oauth/access_token`); Simon Willison, "GitHub OAuth for a static site using Cloudflare Workers" | D1: avoided by issuing in GitHub instead |
| Contributors credited by commit author email | GitHub Docs § About contributors | D2 |
| Static site, data built at deploy, no backend | kautian-mirror `planning.md` § Scope | Phase 3 |

- Deliberately not adopted: a Cloudflare Worker OAuth login (extra secret and service for one saved click); an accent column in khpoo (breaks the two-column `csv_noheader` contract every stage reads; git trailers carry it); per-accent toggles for contributed rows (maintainer chose khpoo, D5); fuzzy dedup beyond normalisation (a near-duplicate reading may be the accent difference itself).
