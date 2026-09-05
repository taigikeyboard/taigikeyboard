# Taigi Incident Appendix

Project-specific incidents that produced the abstracted cross-project rules in `~/.claude/rules/` (managed by the [`configurations`](https://github.com/siansiansu/configurations) dotfiles repo). Use these as the "why" when revisiting a global rule and wondering whether it really applies here.

Read this file alongside any global rule whose abstract version you want grounded in real Taigi events.

## Maps to `~/.claude/rules/diagnosis-discipline.md`

### Confirm bug before round

- **2026-05-16 Bug 7** — USER said `確認:input "taiuantaigi" → expected hanji "臺灣台語" / roman "Tâi-uân tâi-gí" 段間要空格`. Static code-reading inferred a runtime symptom; full Codex pre-impl was about to run. USER stopped: "等等,已經確認 Bug 7 是 real bug 嗎". It wasn't — PRs #281/#282 weren't merged/built, so the roman continuous path had never executed. Symptom was pure code-reading inference.

### Trace before assert

- **PR #310 B-3+B-4 round** (commit `ad085b3b`):
  - Test `assert_eq!(canonical_tl_form("chiah goa", Poj), "tsia̍h góa")` assumed `to_tl` adds default tone diacritics for toneless input; real behaviour at `engine/phonetics/src/tables.rs:43-47` returns "" for tones 1/4, so `to_tl("ts","iah","4") = "tsiah"`. Codex post-impl flagged two false BLOCKs against my wrong oracle.
  - Pre-impl prompt claimed `poj_display_to_tl_display` was safe for non-Taigi pass-through; grep of `tables.rs` showed uppercase `TH`/`IN`/`OR` parse as `th` initial + `e`/`in`/`or` finals, so unconditional fold corrupts uppercase custom-dict entries.

- **2026-05-31 `tai5`→`ta` prefix-collision bug** (dogfood S5 / `INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`). Reported: typing `tai5` surfaced the 2-letter `ta` (`乾/焦/大/…`) tone family. The syllabifier masking test `numeric_tone_input_suppresses_orphan_toneless_boundary` built its hermetic inventory as `build_inventory(&["tai5"])` — `ta` was **never in the test inventory**, so the production collision (`ta` AND `tai5` both valid, `ta`⊂`tai`) never fired. The #367 explicit-tone integration fixture even **had** the shorter control syllable `tsu` (`build_syllables_fst(&["tsua2","tsua5","tsu"])`) but only asserted the wrong-tone homophone absent, never the shorter-prefix syllable absent — so its own `["紙","珠"]` output carried the leak unnoticed. **Fixture-coverage rule**: a syllabifier / continuous-fetch / golden fixture that exercises a syllable `X` MUST also include every production syllable that is a strict prefix of `X` (`ta`⊂`tai`⊂`tai5`, `tsu`⊂`tsua`) AND assert the shorter one's absence (or presence, per spec) — otherwise prefix-collision behaviour is structurally untestable. Confirm via the `engine/composing/tests/candidate_dump.rs` dev harness against production artifacts before claiming a candidate-strip bug fixed. Empirically observed first (probe: `valid_span_endings("tai5")=[2,4]`; production `fetch_hanji("tsua2")=["紙","珠"]`), then fixed — never asserted from code-reading alone. **Fix-location lesson**: the first attempt put longest-match in the segmentation primitive (`syllabifier::valid_span_endings`), which silently reintroduced the PR #290 P1 regression (`span_min_syllable_count("tania")` → `None` instead of `Some(2)` — the non-greedy `ta`+`nia` recovery broke). The lattice/segmentation primitive feeds the walker + min-hop counter, so a **display-only** candidate-strip change belongs in the span-local key builder (`composing::shadow::left_anchored_keys_from_lattice`), never in the shared primitive. Suppress only shorter SINGLE-syllable prefixes there; multi-syllable phrase edges (台語 sub-word of taigikhipuann) must stay.

### Verify pipeline claims

- **v3.5.8 Phase 1 plan** referenced 3 nonexistent file paths (`build_dictionary.py`, `phonetics::syllable::split`, `handle.rs` for the version check); pre-impl review corrected them in the roadmap (PR #249 commit `9eb7b890`).
- **v3.5.8 Phase 1b plan** claimed FST kept syllable separators for multi-syllable toneless keys; live grep showed `dictionary/common/notone.py::remove_tone()` already strips both digits and hyphens, so `tl_notone`/`poj_notone` were already fused. The entire phase was a NO-OP that had been marked CRITICAL.

### Verify hard-prerequisite claims

- **v3.5.8 Phase 9 Item 5** (PR #270 → slim-down `27c8672c`, 2026-05-14). Memory + Codex agreed `(roman, hanji)` ranker dedupe was a hard prerequisite for Item 12. USER pushed back; grep of `dictionary/build/merge_csv.py:107` showed `groupby((hanzi, _tl_key))` already enforced uniqueness at builder stage. Production had zero realistic duplicates today. Shipped ~155 LOC + 9 tests + locked D2 winner policy — all reverted in the slim-down.

### Workaround circuit-breaker

- **v3.5.0 IME-dismiss investigation** (2026-04-25). 8 commits on PR #179 each had a plausible theory and Codex sign-off, but failure rate stayed non-zero and new host scenarios kept appearing (Chrome → Discord → …). Comparing against `references/aiongtaigi-sushi` and `references/florisboard` surfaced the architectural hazard: `MATCH_PARENT × MATCH_PARENT` IME window + custom child-position insets + `TOUCHABLE_INSETS_VISIBLE`. PR #180 reverted to platform-default sizing/inset model; see `memory/project_ime_window_arch.md` for the durable summary.

### No unilateral release scope

- **2026-05-16 v3.5.8 Bug 3 closeout**. I repeatedly framed the missing inline-composing underline as "documented v3.5.8 known limitation" and pushed segmentation redesign to a "post-v3.5.8 track", calling R4 the "only remaining round → tag". USER corrected: 「不要擅自決定哪些超出 v3.5.8 的範圍,v3.5.8 該 release 的時候我會給你明確的指示」. Both deferrals were actually must-fix for v3.5.8.

## Maps to `~/.claude/rules/round-workflow.md`

### Dual gate (Codex sandwich + PR-bot) — RETIRED 2026-08-16

The `/codex-pr-review` skill and the round-workflow step that invoked it were removed at USER
request (「移除 codex-pr-review,這個已不需要」). The incident below is kept as evidence, not as a
live rule: it is why a second pass over the OPENED diff was once mandatory, and what a future
round loses by not running one. `/code-review` covers the same ground on demand.

- **PR #227** (`KeyboardLayoutSolver` extraction, 2026-05-07). Codex sandwich said "math fidelity PASS"; PR-bot caught a 1px refactor-freeze divergence within minutes. Original `(resources.getDimension(R.dimen.key_height) * keyHeightFactor).toInt()` truncates once; my refactor truncated twice via an intermediate `baseKeyHeight: Int`. Drift was ±1px on non-integer-density devices. Sandwich passes "the function does what its plan says"; PR-bot catches "the diff preserves every observable property of HEAD~1".

## Maps to `~/.claude/rules/code-review-rules.md`

### Qualitative perf gate (§9)

- Base checklist: **S1 POJ diacritics**, **S2 TPS composition**, **S3 Hanji candidate scroll**, plus iOS keyboard-extension 64 MB hard cap, leak-free + no-keyboard-dismiss.
- Per-feature acceptance items **S4–S28** (type X → expect Y + the `INVARIANT_*` each pins) live in `docs/architecture/dogfood-checklist.md` — read it when preparing a device dogfood pass or when an `Sn` is cited; root-cause receipts stay in `docs/architecture/behavioral-invariants.md §N` + `memory/project_*.md`.

## Maps to `~/.claude/rules/planning.md`

### Grounded in code

- **v3.5.8 連續輸入 plan** (2026-05-10 era). I drafted a `nextword` integration based on the assumption "nextword is the bigram prediction engine". Actual read of `engine/nextword/src/api.rs:42-84` + `dispatch.rs:14-99` showed nextword does NOT fetch predictions — platform feeds `Vec<RawNextWordPrediction>` from outside; nextword only filters/scores. That single read flipped both direction AND reasoning of the integration question.

### Cite best practices

- Starting point in this project: `docs/references/mainstream-ime-comparison.md` (TL;DR matrix + topic index → drill into per-repo cards).
- Example IME-source cites that have proven useful: `references/khiin-rs/khiin/src/buffer_mgr.rs` (commit-and-resegment semantics), `references/librime/src/rime/...` (segment status state machine), `references/aiongtaigi-sushi` / `references/florisboard` (IME window/inset reference).

## Maps to `~/.claude/rules/phonetics.md`

### Authoritative-source-only (CLAUDE.md Core Principle #3)

- **2026-04-26 `iri/erk/eeh` audit**. Scanned 1151 dictionary syllables, found no matching words, proposed removing them everywhere. USER corrected: "iri/erk/eeh 這個有意義,是特殊字尾". They are legitimate special/dialectal finals per `knowledge/taigi-phonetics-reference.md` §3.2.6 (`irinn`, `irk`, `irp`, `irt`, `irm`, `irn`, `irng`, `er`, `erh`, `erm`, `ee`, `ere`, …).
- **2026-05-20 三索引 round TPS schema proposal**. I proposed `tps_num = digit-tone` and listed `Zhuyin (bopomofo) tps_num = ㄏㄛ2ㄙㄝ3` as an `AskUserQuestion` option. USER rejected twice: (a) "TPS 有自己的聲調表示方法,不是用數字輸入"; (b) "TPS 不是 bopomofo,TPS 和 bopomofo 是不同的系統". The answer was in `knowledge/taigi-phonetics-reference.md` §5 + `engine/phonetics/src/tps.rs::ZHUYIN_TONES`. Always read those before proposing schema.
