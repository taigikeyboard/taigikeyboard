# Taigi Incident Appendix

Project-specific incidents that produced the abstracted cross-project rules in `~/.claude/rules/` (managed by the [`configurations`](https://github.com/siansiansu/configurations) dotfiles repo). One entry per incident: what went wrong, the USER's words where they set the rule, and the pointer for the full receipt. Read alongside a global rule when you want its concrete Taigi "why".

## Maps to `~/.claude/rules/diagnosis-discipline.md`

### Confirm bug before round

- **2026-05-16 Bug 7** — USER `確認:input "taiuantaigi" → expected …` was treated as an observed bug; a fix round was about to open. USER: "等等,已經確認 Bug 7 是 real bug 嗎". It was not — PRs #281/#282 were unmerged, the path had never run.

### Trace before assert

- **PR #310** (`ad085b3b`) — a test oracle assumed `to_tl` adds default tone diacritics; `engine/phonetics/src/tables.rs:43-47` returns "" for tones 1/4. Two false BLOCKs raised against the wrong oracle. Same round: grep of `tables.rs` showed uppercase `TH`/`IN`/`OR` parse as valid finals, so an "unconditional fold is safe" pre-impl claim was false.
- **2026-05-31 `tai5`→`ta` prefix collision** (S5 / `INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`) — the masking test built its inventory as `build_inventory(&["tai5"])`; `ta` was never in the fixture, so the production collision could not fire. **Fixture rule**: a syllabifier / continuous-fetch / golden fixture exercising syllable `X` must also include every production syllable that is a strict prefix of `X` and assert its absence (or presence). Confirm against production artifacts via `engine/composing/tests/candidate_dump.rs` first. **Fix-location rule**: display-only candidate strips belong in the span-local key builder (`composing::shadow::left_anchored_keys_and_restrictions`), never in `syllabifier::valid_span_endings` — the first attempt there reintroduced the PR #290 `span_min_syllable_count("tania")` regression.

### Verify pipeline claims

- **v3.5.8 Phase 1 / 1b plans** — plan cited 3 nonexistent paths (fixed in PR #249 `9eb7b890`); a CRITICAL phase turned out to be a NO-OP because `dictionary/common/notone.py::remove_tone()` already stripped digits and hyphens.

### Verify hard-prerequisite claims

- **PR #270 → slim-down `27c8672c`** (2026-05-14) — memory + Codex agreed a `(roman, hanji)` ranker dedupe was a hard prerequisite; `dictionary/build/merge_csv.py:107` `groupby((hanzi, _tl_key))` already enforced uniqueness. ~155 LOC + 9 tests reverted.

### Workaround circuit-breaker

- **PR #179 → #180** (2026-04-25) — 8 Codex-approved mitigations for IME dismiss, failure rate never zero, new hosts kept appearing. Comparing to `references/aiongtaigi-sushi` / `references/florisboard` exposed the `MATCH_PARENT × MATCH_PARENT` + custom-inset architecture; #180 reverted to platform defaults. Receipt: `memory/project_ime_window_arch.md`.

### No unilateral release scope

- **2026-05-16 v3.5.8 Bug 3** — I labelled a missing underline "documented known limitation" and pushed segmentation redesign "post-v3.5.8". USER: 「不要擅自決定哪些超出 v3.5.8 的範圍,v3.5.8 該 release 的時候我會給你明確的指示」. Both were must-fix.

## Maps to `~/.claude/rules/round-workflow.md`

### Second pass over the opened diff

- **PR #227** (2026-05-07) — sandwich passed "math fidelity"; a diff-level review caught a double `toInt()` truncation (±1px on non-integer-density devices). The `/codex-pr-review` step was retired 2026-08-16 at USER request (「移除 codex-pr-review,這個已不需要」); ask for `/code-review` when a refactor diff carries numeric / geometry fidelity risk.

## Maps to `~/.claude/rules/code-review-rules.md`

### Qualitative perf gate (§9)

- Base checklist: **S1 POJ diacritics**, **S2 TPS composition**, **S3 Hanji candidate scroll**, plus iOS keyboard-extension 64 MB hard cap, leak-free + no-keyboard-dismiss.
- Per-feature acceptance items **S4–S28** live in `docs/architecture/dogfood-checklist.md`; root-cause receipts in `docs/architecture/behavioral-invariants.md §N` + `memory/project_*.md`.

## Maps to `~/.claude/rules/planning.md`

### Grounded in code

- **v3.5.8 連續輸入 plan** — drafted a `nextword` integration assuming it fetches bigram predictions; `engine/nextword/src/api.rs:42-84` showed it only filters / scores what the platform feeds. One read flipped direction and reasoning.

### Cite best practices

- Start at `docs/references/mainstream-ime-comparison.md` (TL;DR matrix + topic index → per-repo cards). Proven cites: `references/khiin-rs/khiin/src/buffer_mgr.rs` (commit-and-resegment), `references/librime/src/rime/...` (segment status state machine), `references/aiongtaigi-sushi` / `references/florisboard` (IME window / inset).

## Maps to `~/.claude/rules/phonetics.md`

### Authoritative-source-only (CLAUDE.md Core Principle #3)

- **2026-04-26 `iri/erk/eeh`** — proposed removing finals absent from the dictionary. USER: "iri/erk/eeh 這個有意義,是特殊字尾". They are dialectal finals per `knowledge/taigi-phonetics-reference.md` §3.2.6.
- **2026-05-20 TPS schema** — proposed `tps_num = digit-tone` and a "bopomofo" option. USER: "TPS 有自己的聲調表示方法,不是用數字輸入" / "TPS 不是 bopomofo". Answer was in `knowledge/taigi-phonetics-reference.md` §5 + `engine/phonetics/src/tps.rs::ZHUYIN_TONES`.
