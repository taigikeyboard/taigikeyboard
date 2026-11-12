# Accent-variation contribution site — roadmap

> **Type**: Planning (multi-PR)
> **Keywords**: `腔口差`, `accent variations`, `contribution`, `issue form`, `GitHub App`, `supplementary`, `kautian accent`
> **Status**: Draft 2026-11-12 — Phase 0; D5 and D6 await the maintainer

## Goal

A minimal contribution site, styled like the web version: anyone with a GitHub account searches TaigiKeyboard's dictionary from a kautian-mirror-like search bar and, when a word or reading is missing, submits Hanji + TL + accent + a reason. The submission becomes a pull request that adds the row to the accent-variation supplement; CI checks it and removes duplicates; the maintainer merges.

Decisions (maintainer, 2026-11-12):

- **D1 Submit path** — a static site cannot finish a GitHub OAuth login on its own: PKCE is supported since 2025-07, but the token endpoint has no CORS, and the GitHub API or `gh pr create` both need a token. The site therefore opens a prefilled GitHub issue form; a workflow in this repository runs `gh pr create` with a GitHub App token. One extra **Create** click on github.com, no backend, no secret in the browser.
- **D2 Contributor credit** — "I want the contributor's account to appear in the repo." The intake commit's author is the contributor (`<user.id>+<login>@users.noreply.github.com`, from the issue event); the App is only the committer and the PR opener. The PR is merged with a merge commit (not squash), so the author survives onto `main` and the contributor is listed in the repository's Contributors. The `main` ruleset (`deletion`, `non_fast_forward`, `pull_request`) requires no signed commits, so an App-committed, contributor-authored commit is accepted. GitHub's Contributors graph counts non-merge commits on the default branch whose author email belongs to an account (the GitHub noreply address counts, so contributors who hide their email still match) and commit co-authors; it shows the top 100 and can lag ~24 h. **Acceptance**: after the first merged contribution, the submitter's account is listed at `github.com/taigikeyboard/taigikeyboard/graphs/contributors`.
- **D3 Accent categories** — the ten Kautian accents (`dictionary/sources/official/kautian/config.yaml:35-45`, 鹿港偏泉腔 … 臺中偏漳腔), multi-select, plus 其他 with a free-text place name.
- **D4 Site location** — a separate repository (like `taigikeyboard/kautian-mirror`), published under `taigikeyboard.tw/<name>/`; its build reads this repository's `dictionary/output/dictionary.csv`. The data file, the checks and the intake workflow stay here.
- **Data destination** — contributed rows join the existing 腔口差 (Accent Variations) toggles under Manage Dictionaries (`i18n/common.json` key value 腔口差, `i18n/dictionary.json` 補充在地腔口差異).

Open (maintainer):

- **D5 Source attribution** — the accent bits are Kautian subtags: `engine/lexicon/src/dictionary_reader.rs:73-84` reads bits 1..=10 only under the kautian source. Options: (a, recommended) a new source bit for community accent data, with the engine applying the ten accent toggles to it too — Kautian data stays official, but dictionary + engine + every platform's Manage Dictionaries change; (b) tag rows as kautian with an accent mask — no engine change, but community rows pass as Ministry of Education data, and kautian-mirror may read them.
- **D6 其他 rows** — no toggle exists for them. Options: (a, recommended) kept in the CSV, not shipped until enough rows share a place name to discuss a new toggle; (b) drop 其他; (c) an 11th bit, one shared 其他 toggle (bit 11 is the name subtag today, so this means a wire change).

## Today (grounded in code)

| Piece | Today | Where |
|---|---|---|
| Accent data | Kautian 語音差異 sheet: single characters only, one reading per accent; word-level variants generated | `dictionary/common/kautian_provenance.py:82`, `dictionary/common/kautian_accent_wordgen.py` |
| Accent mask | `kautian_accent_mask` column in `dictionary.csv`; 10 bits reserved by the encoder | `dictionary/common/kautian_provenance.py:43-48`, `dictionary/build/create_dictionary_bin.py:48` |
| Engine gating | bits 1..=10 = accent, gated per toggle under the kautian source only | `engine/lexicon/src/dictionary_reader.rs:73-84`, `:439` |
| Hand-kept supplements | `hanzi,tl` CSVs merged after the sources | `dictionary/supplementary/{dev,lkk}/data/`, `dictionary/build/merge_csv.py:178-202` |
| Search data | `dictionary/output/dictionary.csv` is tracked (~170k rows, per-source flags, POJ / TL / no-tone keys) | `dictionary/output/dictionary.csv` |
| Search UI reference | kautian-mirror search bar and engine | `taigikeyboard/kautian-mirror` `extension/src/search/engine.js`, `src/suggest.js` |
| Issue templates | bug report, feature request | `.github/ISSUE_TEMPLATE/` |

## Data file

`dictionary/supplementary/khiunnkhau/data/khiunnkhau.csv`, sorted by `(hanzi, tl, accent)`:

```
hanzi,tl,accent
八,pueh,鹿港偏泉腔
```

- One row per `(hanzi, tl, accent)`; a submission with several accents writes several rows.
- `accent` is one of the ten names or `其他:<place>`.
- The reason lives in the PR body and the commit message, not the CSV. The contributor lives in git authorship, not the CSV (no personal identifiers in tracked files beyond what the contributor's own commit carries).

## Phases

| Phase | PR | Scope | Status |
|---|---|---|---|
| 0 | admin | This roadmap + project memory | In progress |
| 1 | this repo | Data file + `dictionary/tools/verify_khiunnkhau.py` + CI job | Pending |
| 2 | this repo | Issue form + intake workflow + GitHub App wiring | Pending |
| 3 | new repo | Static site: search + form + Pages deploy | Pending |
| 4 | this repo | Build + engine + platform integration per D5 / D6 | Blocked on D5, D6 |

### Phase 1 — data file and checks

`verify_khiunnkhau.py` (stdlib + the dictionary pipeline's own helpers), run by `python.yml` when the CSV changes:

1. Schema: header, three columns, sorted, NFC, lowercase TL.
2. TL validity: each syllable through the pipeline's strict TL→POJ conversion (`dictionary/common/romanization.py`, the same check `kautian_accent_wordgen` uses); Hanji count = syllable count, as the cleanup stage's Han-Lo check.
3. `accent` ∈ ten names ∪ `其他:<non-empty>`.
4. Dedup: exact duplicate rows fail; rows equal after normalisation (NFC, case, hyphen / space) fail; a `(hanzi, tl)` already carrying that accent bit in `dictionary.csv` fails with "already in Kautian"; a `(hanzi, tl)` already present without the bit passes with a note (that is the point of the data).
5. Tests in `dictionary/tests/`.

### Phase 2 — intake

- `.github/ISSUE_TEMPLATE/khiunnkhau.yml`: inputs `hanzi`, `tl`, `reason`; accents as checkboxes or dropdown (multi), `other_place`.
- Spike first (≤20 lines): does an issue-form URL prefill a dropdown / checkbox by field `id`? The docs name `id` as the prefill key but show no dropdown example. If not, accents travel in a prefilled text input that the workflow validates.
- `.github/workflows/khiunnkhau-intake.yml`, on `issues: opened` with the form's label:
  1. Parse the form body; run `verify_khiunnkhau.py` on the candidate rows; on failure comment the reason and close as not planned.
  2. Branch `khiunnkhau/issue-<n>`, append + sort rows, commit with `--author "<login> <id+login@users.noreply.github.com>"`, push with the App token.
  3. `gh pr create` (body: rows, reason, `Closes #<n>`), comment the PR link on the issue.
- Token: a GitHub App (contents + pull requests + issues write on this repository). A PR opened with `GITHUB_TOKEN` does not trigger `pull_request` workflows, so Phase 1's check would never run. The release-automation App (`architecture/release-automation-roadmap.md` D3) can carry these permissions, or a separate App — maintainer setup either way.
- Merge with a merge commit (D2); squash would rewrite the author.
- Abuse guard: workflow input lengths capped; issue body treated as data (no shell interpolation of form fields — pass through env and a script file).

### Phase 3 — site

- Repo `taigikeyboard/<name>`, Pages under `taigikeyboard.tw/<name>/`; style tokens and fonts from `web/src/style.css` / kautian-mirror.
- Build: fetch `dictionary.csv` at a pinned `main` commit → compact JSON (hanzi, tl, poj, no-tone keys, kautian accent mask).
- Search: Hanji, TL, POJ, toneless; a hit shows its readings and accent tags. "Add" is always available (new reading of an existing word, or a new word).
- Form: Hanji, TL, accents (ten + 其他), reason → opens the prefilled issue URL. The form says the contributor's GitHub account will appear as the commit author.
- Rebuild daily (the kautian-mirror schedule) so merged contributions show up in search.

### Phase 4 — integration (after D5 / D6)

Merge the CSV into the build so the 腔口差 toggles cover it. Scope depends on D5: (a) touches `dictionary/common/source_bits.py`, `merge_csv.py`, `create_dictionary_bin.py`, `engine/lexicon/src/dictionary_reader.rs`, the proto source list and every platform's Manage Dictionaries; (b) touches `merge_csv.py` only.

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| Contribution via issue form → bot PR, no backend | GitHub issue forms (`docs.github.com` § Syntax for GitHub's form schema); common in data repos | Phase 2 |
| OAuth for static sites needs a server-side token exchange | GitHub OAuth docs (no CORS on `/login/oauth/access_token`); Simon Willison, "GitHub OAuth for a static site using Cloudflare Workers" | D1: avoided by issuing in GitHub instead |
| Static site, data built at deploy, no backend | kautian-mirror `planning.md` § Scope | Phase 3 |

- Rules: `rules/planning.md` (grounded in code, roadmap + memory), `rules/code-review-rules.md` §2 (reuse the pipeline's TL checks rather than a second validator), `rules/round-workflow.md` (one PR per phase).
- Deliberately not adopted: a Cloudflare Worker OAuth login (extra secret and service for one saved click); storing the reason or the contributor in the CSV (git already records both); fuzzy dedup beyond normalisation (a near-duplicate reading may be the accent difference itself).
