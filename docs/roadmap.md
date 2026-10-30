# TaigiKeyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `open candidates`
> **Status**: Active

---

## Summary

- **Forward-looking work only.** Shipped versions: [`CHANGELOG.md`](../CHANGELOG.md) (release notes per train); the v3.5.8 plan archive is [`releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md). Shipped designs live in the `architecture/*-roadmap.md` docs and in `architecture/behavioral-invariants.md`.
- Device dogfood items: [`architecture/dogfood-checklist.md`](architecture/dogfood-checklist.md) (none pending as of 2026-10-18).
- Release scope, timing and tags are the maintainer's call; only the v3.9.0 fix list below is assigned to a release (maintainer, 2026-10-30).

---

## Active / In-flight items

- **Maintainability audit follow-up (2026-10)** — [`architecture/maintainability-roadmap-2026-10.md`](architecture/maintainability-roadmap-2026-10.md), rounds over [`reports/2026-10-09-audit-all.md`](reports/2026-10-09-audit-all.md). **Paused** by the maintainer 2026-10-14; resume point is the maintainer's call.

### v3.9.0 fix list (Discord #issues, maintainer 2026-10-30)

Work order = row order. One round per row; a bug row starts with a repro (`AGENTS.md` Core Principle #4). Rows 8–10 need a maintainer design decision first; rows 11–12 need reporter input or a maintainer call.

| # | Item | Kind | Platforms | Next step |
|---|---|---|---|---|
| 1 | Candidates sometimes missing from the candidate bar again; tapping + twice restores them | bug (regression) | Android | Closed 2026-10-30: not reproduced on mobile v3.8.0 (120 device runs, 8 scenarios); no matching earlier fix in git |
| 2 | Continuous input: a shorter-syllable split ranks above the single word (`uê` → ū ê before 鞋; `khikhi`; `guhi`) — TL and TPS, POJ `ôe` fine | bug | engine | `candidate_dump` for `ue5` / `khikhi`: ranking or segmentation |
| 3 | Google Docs in Chrome loses candidates on `ko͘` + `c`; same input fine elsewhere | bug | Windows | Reproduce on the Windows box; #538 covered macOS only |
| 4 | Show a notice when toggling Hanji / Romanization | feature | Windows | Check macOS / Linux behaviour for parity |
| 5 | Custom layouts: allow ⁿ on the main keyboard | feature | iOS, Android | — |
| 6 | Vertical expandable candidate window layout | feature | Windows (+ macOS asked) | Linux vertical window as reference |
| 7 | Custom layouts: user-editable long-press callouts (half-width symbols on half-width `,` `.`; `{ }` on `( )`) | feature | iOS, Android | — |
| 8 | Custom layouts are not separated by TL / POJ, so picking one when switching romanization is hard | feature | iOS, Android | Design decision (idea 2026-10-29: per-layout TL / POJ setting) |
| 9 | `c` types `ch` / `ts` (toggle); desktop Hanji → romanization reverse lookup | feature | desktop | Decision: toggle vs. losing a selection key; reverse lookup scope |
| 10 | TPS requests: confirm key commits TPS symbols; desktop TPS layout (awaiting reporter: where ㄝ goes); mobile TPS layout; ↓ selection side; custom-dictionary ↓ pick; tone-8 `˙` placement vs. converter | feature (6 parts) | desktop, mobile | Split into separate items, then decide |
| 11 | Neutral-tone candidates for 起來 / 起去 / 一下 / ah | feature | engine | Needs input → expected-output examples |
| 12 | Dialectal D spellings for L / J readings | feature | engine | Maintainer call; phonetics rules need an authoritative source (`docs/contributing/phonetics.md`) |

---

## Open candidates (unscheduled)

One line each; the linked section holds the design, the measurements and the open questions.

**Engine and dictionary**

| Item | Status | Where |
|---|---|---|
| Retire the old dictionary `frequency` after E1 | Recorded, not started. Still read: the equal-cost tie-break (`CandidateSortKey` −score / −freq; ~69 % of rows are corpus-unseen and share one cost), the search-page tie-break, `association.bin` dictionary-derived counts (`associations.py`), CSV / FST row order + duplicate max-aggregation (`merge_csv.py`), the `dict.bin` u32 field, `CandidateMessage.score`. Steps: pick a replacement tie-break for unseen words, a count source for the association rows, then the format bump. A custom twin of a dictionary word carries no corpus cost (sorts below its priced homophone in the list) | [`reports/2026-10-08-e1-p5b-list-sort.md`](reports/2026-10-08-e1-p5b-list-sort.md) §5 |

**Mobile parity (inventory 2026-10-12)** — pending code; directions decided unless marked.

| Item | Direction | Where |
|---|---|---|
| Needs device repro first (UNCONFIRMED) | Android symbols-page `、` / numeric pad wipes the preedit; Android next-word predictions survive a cursor move | `+Cleanup.swift`, `TextInputManager.kt`, `TextInputKeyHandler.kt` |

## Not to re-propose

Closed by the maintainer; one line each.

- `zh-TW` README; xcconfig signing; Android Gradle proto plugin; dictionary-source licensing follow-up.
- Romanization spelling correction (fuzzy / edit-distance tolerance in the engine) — dropped when the Candidate Display = Romanization mode shipped without it.
- Corpus expansion P1b, including the TAT application.
- A full TL / POJ / en / ja proofreading pass of the app UI — reported typos are fixed per incident.
- `$` sentence-start opener (its `association.bin` data is removed in its own PR).
- Smart-suggestion techniques outside the bigram model: 3 zero-query continuation from learned history, 5 negative feedback on rarely picked entries, 6 zero-query rule table (`「`→`」`, number → measure word), 9 word → emoji, 10 offensive-word filter, 11 privacy gates (incl. the Android `IME_FLAG_NO_PERSONALIZED_LEARNING` gap). Names of all eleven: [`architecture/bigram-lm-roadmap.md`](architecture/bigram-lm-roadmap.md).
- Windows candidate-window paint latency (frame drawn before its candidates on a busy host UI thread) and UIA exposure.
- Converted-romanization commit on Enter (`suann2ting3` → `suán-tìng`).
- TL mode (臺羅模式) tone key commits the syllable without a candidate window (MOE Mac IME parity).
- Mobile predictions after Space.
- Bigram P6 walker term, P7 hanji-only sources, D7 two-word context.
- e2e drivers for macOS, Windows, Android, iOS.
- Invariant-label PR2; naming batch C (persisted names stay frozen).
- Shared Swift package for macOS + iOS — see [`architecture/maintainability-roadmap-2026-10.md`](architecture/maintainability-roadmap-2026-10.md) § Best practices alignment (deliberately not adopted).
