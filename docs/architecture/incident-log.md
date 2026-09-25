# Incident log — the dated narratives behind the project rules

> **Type**: Reference (append-only)
> **Keywords**: `incident`, `rules`, `why`, `receipt`
> **Related**: ../../.claude/rules/taigi-incidents.md, dogfood-checklist.md, behavioral-invariants.md

---

## Summary

One entry per incident: what went wrong, the USER's words where they set the rule, and the pointer for the full receipt. The imperative rules distilled from these live in `.claude/rules/taigi-incidents.md` (always-on); the abstracted cross-project rules in `~/.claude/rules/`. PR numbers before 2026-09-07 refer to the archive repository — see `pr-number-migration.md`.

## Maps to `~/.claude/rules/diagnosis-discipline.md`

### Confirm bug before round

- **2026-05-16 Bug 7** — USER `確認:input "taiuantaigi" → expected …` was treated as an observed bug; a fix round was about to open. USER: "wait, has Bug 7 actually been confirmed as a real bug?". It was not — PRs #281/#282 were unmerged, the path had never run.

### Trace before assert

- **PR #310** (`ad085b3b`) — a test oracle assumed `to_tl` adds default tone diacritics; `engine/phonetics/src/tables.rs:43-47` returns "" for tones 1/4. Two false BLOCKs raised against the wrong oracle. Same round: grep of `tables.rs` showed uppercase `TH`/`IN`/`OR` parse as valid finals, so an "unconditional fold is safe" pre-impl claim was false.
- **2026-05-31 `tai5`→`ta` prefix collision** (S5 / `INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`) — the masking test built its inventory as `build_inventory(&["tai5"])`; `ta` was never in the fixture, so the production collision could not fire. Produced the fixture rule. The first fix attempt in `syllabifier::valid_span_endings` reintroduced the PR #290 `span_min_syllable_count("tania")` regression — produced the fix-location rule.

### Verify pipeline claims

- **v3.5.8 Phase 1 / 1b plans** — plan cited 3 nonexistent paths (fixed in PR #249 `9eb7b890`); a CRITICAL phase turned out to be a NO-OP because `dictionary/common/notone.py::remove_tone()` already stripped digits and hyphens.

### Verify hard-prerequisite claims

- **PR #270 → slim-down `27c8672c`** (2026-05-14) — memory + Codex agreed a `(roman, hanji)` ranker dedupe was a hard prerequisite; `dictionary/build/merge_csv.py:107` `groupby((hanzi, _tl_key))` already enforced uniqueness. ~155 LOC + 9 tests reverted.

### Workaround circuit-breaker

- **PR #179 → #180** (2026-04-25) — 8 Codex-approved mitigations for IME dismiss, failure rate never zero, new hosts kept appearing. Comparing to `references/aiongtaigi-sushi` / `references/florisboard` exposed the `MATCH_PARENT × MATCH_PARENT` + custom-inset architecture; #180 reverted to platform defaults. Receipt: project memory `project_ime_window_arch.md`.

### No unilateral release scope

- **2026-05-16 v3.5.8 Bug 3** — a missing underline was labelled "documented known limitation" and the segmentation redesign pushed "post-v3.5.8". USER: "don't decide on your own what falls outside the v3.5.8 scope; when v3.5.8 is due for release I will give you explicit instructions". Both were must-fix.

## Maps to `~/.claude/rules/round-workflow.md`

### Second pass over the opened diff

- **PR #227** (2026-05-07) — the sandwich passed "math fidelity"; a diff-level review caught a double `toInt()` truncation (±1px on non-integer-density devices). Ask for `/code-review` when a refactor diff carries numeric / geometry fidelity risk.

## Maps to `~/.claude/rules/planning.md`

### Grounded in code

- **v3.5.8 continuous-input plan** — drafted a `nextword` integration assuming it fetches bigram predictions; `engine/nextword/src/api.rs:42-84` showed it only filters / scores what the platform feeds. One read flipped direction and reasoning.

- **2026-09-18 iPad external keyboard (PRs #77 / #78 / #80 / #83, reverted)** — three PRs built hardware-key composing on `UIInputViewController.pressesBegan`, with "iPadOS delivers `pressesBegan` to the extension" listed only as an unverified dogfood assumption. Real-iPad dogfood: hardware keys reach the host as plain ASCII; the extension never receives `UIPress` events (iPadOS routes them to the host app's responder chain only — the same limit every third-party keyboard has, e.g. PTT iOS 2023-07-24"iPad not accepting third-party input methods on the (hardware) keyboard has long been criticised"). USER: "why didn't you tell me at the planning stage that it couldn't be done?" / "iOS doesn't support external keyboards for third-party input methods; revert the external-keyboard features". A platform-capability assumption that the whole plan rests on gets a 20-line spike PR on device BEFORE the plan, not a dogfood row after it.

## Maps to `.claude/rules/phonetics.md`

### Authoritative-source-only (CLAUDE.md Core Principle #3)

- **2026-04-26 `iri/erk/eeh`** — proposed removing finals absent from the dictionary. USER: "iri/erk/eeh are meaningful; they are special finals". They are dialectal finals per `knowledge/taigi-phonetics-reference.md` §3.2.6.
- **2026-05-20 TPS schema** — proposed `tps_num = digit-tone` and a "bopomofo" option. USER: "TPS has its own tone notation; it is not typed with digits" / "TPS is not bopomofo". Answer was in `knowledge/taigi-phonetics-reference.md` §5 + `engine/phonetics/src/tps.rs::ZHUYIN_TONES`.

## Privacy (`.claude/rules/taigi-incidents.md` § Privacy)

### Personal identifiers in the public tree

- **2026-09-24 privacy scrub** — an open-source readiness audit (`docs/reports/2026-09-24-open-source-readiness-and-layout.md`) found, 17 days after the repository went public, a former work address written out in `docs/go-public-checklist.md` (the very address the 2026-09-07 history rewrite had removed from commit metadata), the Gmail-triage skill's Google Cloud project ID and personal mailbox label layout, and the Discord-triage skill's server and channel IDs plus a runtime `state.json` committed on every run. gitleaks passed all of them: none is a credential. USER: "Go Round 0; don't rewrite history, but I want to avoid this kind of thing happening in future". The tree was scrubbed and both skills moved to the private dotfiles repo; prevention = the `personal-email` gitleaks rule (pre-commit + CI) and a private-denylist pre-commit check for identifiers that cannot be listed publicly. Older commits still carry the values.
