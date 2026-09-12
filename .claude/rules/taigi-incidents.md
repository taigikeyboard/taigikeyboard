# Taigi Incident Rules

Project-specific imperative rules distilled from past incidents. The dated narratives that produced them live in `docs/architecture/incident-log.md`; the abstracted cross-project rules in `~/.claude/rules/` (managed by the [`configurations`](https://github.com/siansiansu/configurations) dotfiles repo).

## Diagnosis (`~/.claude/rules/diagnosis-discipline.md`)

- **Confirm before a round**: a USER `確認:input X → expected Y` line is a dogfood criterion, not an observed bug. Unmerged / unbuilt prerequisite PRs mean the path never ran.
- **Trace before assert**: grep the real tables (`engine/phonetics/src/tables.rs`) before writing an oracle or claiming a fold is safe; `to_tl` returns "" for tones 1 / 4.
- **Fixture rule**: a syllabifier / continuous-fetch / golden fixture exercising syllable `X` must also include every production syllable that is a strict prefix of `X` and assert its absence (or presence). Confirm against production artifacts via `engine/composing/tests/candidate_dump.rs` first.
- **Fix-location rule**: display-only candidate strips belong in the span-local key builder (`composing::shadow::left_anchored_keys_and_restrictions`), never in `syllabifier::valid_span_endings` — touching the primitive reintroduces the `span_min_syllable_count("tania")` regression.
- **Verify pipeline claims**: grep production data + run a real query before trusting a roadmap's "current behaviour"; `dictionary/common/notone.py::remove_tone()` already strips digits and hyphens.
- **Verify hard-prerequisite claims**: memory + Codex agreement is not verification; `dictionary/build/merge_csv.py` `groupby((hanzi, _tl_key))` already enforces `(hanzi, tl)` uniqueness.
- **Circuit-break at 3+ failed fixes**: compare to `references/aiongtaigi-sushi` / `references/florisboard` before adding a mitigation; the IME window uses platform-default insets, never `MATCH_PARENT × MATCH_PARENT` + custom inset.
- **Release scope is USER-gated**: never write "documented known limitation", "post-vX", "deferred" or "ready to tag" without the USER's dated words (USER: 「不要擅自決定哪些超出 v3.5.8 的範圍,v3.5.8 該 release 的時候我會給你明確的指示」).

## Review (`~/.claude/rules/round-workflow.md`, `code-review-rules.md`)

- Ask for `/code-review` on any diff carrying numeric / geometry fidelity risk (refactor freeze, wide caller surface); the Codex sandwich alone misses e.g. a double `toInt()` truncation.
- Qualitative perf gate: base checklist = **S1 POJ diacritics**, **S2 TPS composition**, **S3 Hanji candidate scroll**, plus iOS keyboard-extension 64 MB hard cap, leak-free + no-keyboard-dismiss. Per-feature `Sn` acceptance items live in `docs/architecture/dogfood-checklist.md`; root-cause receipts in `docs/architecture/behavioral-invariants.md §N` + project memory (Claude auto-memory, `~/.claude/projects/-Users-alexsu-Workspace-taigikeyboard/memory/`).

## Planning (`~/.claude/rules/planning.md`)

- Read the module's entry point (`api.rs` / `dispatch.rs`) before proposing integration; `engine/nextword` only filters / scores what the platform feeds, it fetches nothing.
- Cite best practices from `docs/references/mainstream-ime-comparison.md` first. Proven cites: `references/khiin-rs/khiin/src/buffer_mgr.rs` (commit-and-resegment), `references/librime/src/rime/...` (segment status state machine), `references/aiongtaigi-sushi` / `references/florisboard` (IME window / inset).

## Phonetics (`.claude/rules/phonetics.md`, CLAUDE.md Core Principle #3)

- **Authoritative-source-only**: never remove or infer a TL / POJ / TPS rule from dictionary or test absence. `iri` / `erk` / `eeh` are dialectal finals (`knowledge/taigi-phonetics-reference.md` §3.2.6). TPS has its own tone marks (`engine/phonetics/src/tps.rs::ZHUYIN_TONES`) — not digit tones, not bopomofo.
