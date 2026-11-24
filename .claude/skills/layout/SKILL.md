---
name: layout
description: Run the layout test (版面測試) — type real corpus sentences word by word into the staged web page across viewports × candidate layouts × candidate text sizes, assert the candidate window's layout after every word, screenshot each failure, and publish the findings. Use when asked to "run the layout test", 「跑 Web 的版面測試」, 「版面測試」, or to check that the candidate window never overlaps, clips or overflows. Args: a platform (only `web` today) and optionally browsers / views / layouts. Reports findings; never opens a fix round itself.
disable-model-invocation: false
---

# Layout test

Born from the 2026-11-24 cross-platform layout audit; the web harness is `web/layout-sim/`. Only the web has one; another platform joins this skill after its own layout fix round.

## 1. Run

```sh
make -C web layout-sim                                     # 3 browsers × every view × layout × size
BROWSERS=chromium VIEWS=phone LAYOUTS="vertical horizontal" make -C web layout-sim
```

`make site` runs first (the page is staged into `web/target/site/`). A fresh worktree needs `git submodule update --init corpus/taigi-typing` first. A full run takes about six minutes.

| Variable | Values (default: all) |
|---|---|
| `BROWSERS` | `chromium` `firefox` `webkit` |
| `VIEWS` | `desktop` (1280×800), `phone` (360×800), `zoom200` (180×400 = the phone at 200 % page zoom; size 17 only, 7 sentences) |
| `LAYOUTS` | `vertical` `horizontal` `expandable` `verticalExpandable` |
| `ROOT_TEXT_SIZE` | unset. A browser text-size setting (`150%`, `200%`) — **accessibility text sizes run only when the maintainer names them** |

Candidate text sizes are 13 / 17 / 23. Sentences: a fixed pick from `corpus/taigi-typing/exampleSentences.js` (every 600th plus the longest), read at run time — the corpus licence allows quoting, not rewriting, so never copy them into the repo. Each word is typed toneless; the second word of a clause browses the candidates with the arrows; each clause ends with Space until the composition empties, each sentence with Enter.

The port is 4175. "already used" → another run's server; wait for it, never kill a process you did not start. Never start `make -C web e2e` during a run: it deletes `test-results/`, this run's output with it.

## 2. Read the output

`web/test-results/layout-sim/` (gitignored; cleaned at the start of every run and by `make -C web e2e`):

- `summary.json` — browser → `"<view> <layout> <size>"` → issue kind → count; `{}` = clean; `incomplete: 1` = a step threw before the last sentence (Playwright's `layout-*/error-context.md` says where). The run also prints every combination with issues.
- `report-<browser>-<view>-<layout>-<size>.json` — every issue with the sentence / clause index and the word typed (`<word>+browse` = after the arrows).
- `FAIL-<browser>-<combination>-<kind>.png` — the first occurrence of each kind; `ok-<combination>-s<n>.png` — up to three Chromium shots per combination of clauses without issues.

A combination with any issue fails its test, so `make` exits non-zero; read `summary.json` either way.

| Kind | Means |
|---|---|
| `panel-covers-composition` | the candidate window overlaps the composition text |
| `panel-past-editor-bottom` / `panel-past-viewport` | the window extends past the editor's bottom / the viewport's sides |
| `page-scrolls-sideways` / `list-scrolls-sideways` / `candidates-scroll-sideways` | horizontal overflow of the page / list / candidate row |
| `arrows-past-panel` | the page arrows extend past the window |
| `row-taller-than-box` | a row's content is taller than the row |
| `candidate-cut` | a candidate that would fit in its row, were the row alone in the list with the romanization giving way, is still cut |
| `script-past-cell` / `script-past-row-height` | a candidate or its romanization extends past its cell / its row |

## 3. Real defect or design?

Open the `FAIL-*.png` for every kind before calling it either. Design behavior, not defects:

- A candidate wider than the whole list less the row's index and padding, cut with "…" — the check already skips those (at `zoom200` a five-Hanji word does not fit a `horizontal` / `expandable` cell). The romanization is truncated with "…" by design (the candidate stays whole, `web/e2e/look.spec.mjs`) and is never checked for cuts.
- A kind in one browser only, once, that a rerun of that combination (`BROWSERS=<b> VIEWS=<v> LAYOUTS=<l>`) does not repeat — a timing flake; report it as such.

Everything else is a candidate defect. Trace it to the CSS / layout code (`web/src/style.css`, `web/crates/taigi-web/src/layout.rs`, `desktop/crates/taigi-desktop-core/src/candidates/expandable.rs`) before naming a cause, and say whether the same kind also shows in the other browsers.

## 4. Report, do not fix

Publish an Artifact page: per browser a table of combinations with issues (kind × count), each issue kind with its `FAIL-*.png` beside the words that triggered it and your verdict (defect + traced `file:line` / design / flake), and two or three `ok-*.png` for comparison. Give the maintainer the link, plus one line per candidate defect in the reply.

A candidate defect is an observed failure: a fix is its own round, opened by the maintainer (`AGENTS.md` Core Principle #4).
