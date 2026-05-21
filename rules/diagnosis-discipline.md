# Diagnosis Discipline

Mandatory rules before opening a bug-fix round OR writing test assertions OR framing "current behavior" claims.

## Confirm bug before opening a round

A user-stated "expected behavior" — especially a message prefixed `確認:input X → expected Y` — during a dogfood→fix loop is a **dogfood acceptance criterion**, not a confirmed bug.

- Do NOT escalate static code-reading + Codex agreement into a fix round (branch / pre-impl / impl) until the runtime symptom is **empirically observed** via dogfood.
- Classify each user statement explicitly:
  - **(a) observed failure** ("I typed X and got Y") → confirmed; root-cause + fix round OK.
  - **(b) expected behavior** ("X should produce Y" / "確認:…") → add to dogfood acceptance checklist; do NOT open a round.
- For (b): you may pre-analyze root cause and even run ONE ANALYSIS-ONLY Codex pre-impl as a ready head-start, but **record it as an UNCONFIRMED hypothesis**. Do not branch / implement. Gate the actual fix on the user's real-device dogfood.
- If dogfood shows the behavior already works, delete the hypothesis.
- Pending prerequisite PRs that are unmerged or unbuilt mean the path was never exercised; static analysis + Codex agreement ≠ verification.

**Why**: 2026-05-16 Bug 7. User said "確認: input taiuantaigi → 漢字 `臺灣台語` / 羅馬 `Tâi-uân tâi-gí` 段間要空格". I read the iOS/Android continuous branch, saw auto-space gated on `didFinalCommit`, inferred the runtime must produce `Tâi-uântâi-gí`, ran a full Codex pre-impl, and was about to open a branch. User stopped me: "等等,已經確認 Bug 7 是 real bug 嗎". It was not — #281/#282 weren't merged or built, so the roman continuous path had never been run; the symptom was pure code-reading inference.

## Trace before assert

Before writing any `assert_eq!(actual, expected)`, trace the actual code path and compute the expected value from real tables — not from "what I think should happen".

- Walk the function body against the relevant lookup tables / helpers in a doc-comment on the test: `// trace: bare="chiah", normalize_to_tl→"tsiah", split→("ts","iah"), tone="4" (stop), to_tl→"tsiah"`.
- If unsure → run `cargo test`, read the actual output, then write that into the assert. Do NOT reverse-rationalize an expected from the actual; trace first, then verify.
- Before claiming "property X is safe" to Codex pre-impl, grep the source tables (`engine/phonetics/src/tables.rs`, etc.). Codex catches things by really grepping; so should you.
- "Now vs. before" architecture/behavior claims must be backed by `git log -- <path>` + grep of existing non-test callers. The user's memory is usually accurate; any "newly added X" claim should be verified that it isn't actually "filling a missing X".

**Why**: PR #310 B-3+B-4 round hit this twice. (1) `assert_eq!(canonical_tl_form("chiah goa", Poj), "tsia̍h góa")` — I assumed `to_tl` adds default tone diacritics to toneless input; actually `tl_tone_mark("1"/"4")` returns "" (`engine/phonetics/src/tables.rs:43-47`), so `to_tl("ts","iah","4") = "tsiah"`. Codex post-impl flagged 2 false BLOCKs from my wrong oracle, wasting a review round. (2) Told Codex "`poj_display_to_tl_display` is safe for non-Taigi pass-through". Codex grep'd `tables.rs` in 10 seconds and found `TH`/`IN`/`OR` parse as `th` initial + `e`/`in`/`or` finals — unconditional fold corrupts uppercase custom-dict entries.

## Verify pipeline claims

For roadmap-derived implementation plans, verify claims against the actual build pipeline, generated artifacts, and live query behavior **before coding**. Don't trust roadmap text describing the current state of the codebase — it can be stale.

- Before opening any phase PR: grep the production data + run a real query for any factual claim about "current behaviour" in the roadmap.
  - `grep -P "^珠仔," dictionary/output/dictionary.csv` for actual column values.
  - `engine/target/release/fst-builder query <fst> <key>` for actual FST contents.
  - `cargo test -p <crate>` for actual pass/fail of any "today this fails" assertion.
- If the audit refutes the roadmap premise: stop, `AskUserQuestion` with evidence + small option set (usually "skip + invariant lock + roadmap correction"). Do NOT power through the original plan just because it was approved.
- For numeric/derived columns (`tl_num`, `tl_notone`, `tl_abbrev`, `poj_*`), trace the upstream stage (`dictionary/common/stages/`) and helper (`dictionary/common/<helper>.py`) before assuming current FST shape.
- `codex exec --cd <repo> < /tmp/prompt.txt` with `ANALYSIS-ONLY` is good for sanity-checking fact-finding before pivoting a phase plan.

**Why**: v3.5.8 Phase 1 plan referenced 3 nonexistent file paths. Phase 1b plan claimed "FST 對多音節 entry 的 toneless key 仍保留 syllable separator" — pre-impl audit showed `notone.py::remove_tone()` already strips both digits and hyphens via `[\d\-]`; FST already contains fused multi-syllable keys; the entire phase was a NO-OP that had been marked CRITICAL and listed as blocking 4 downstream phases.

## Verify hard-prerequisite claims

When memory carry-forward says "this is a hard prerequisite for downstream slice X" and Codex agrees in pre-impl, **still verify what concretely breaks in slice X if the dependency doesn't land here**. Don't ship speculative protection logic just because memory + Codex form a quorum.

- Build a failing fixture: what concrete production input would trigger the X-side bug today, without this piece? If you can't construct one, the prerequisite is likely speculative.
- Trace the upstream builder / pipeline guarantees before assuming downstream X has a raw duplicate / collision / race problem. Builders (`dictionary/build/merge_csv.py`), FST encoders (`engine/lexicon/src/prefix_index.rs`), and per-source dedupers often already enforce the invariant the downstream code path looks like it needs to defend.
- Memory carry-forward + Codex agreement is **not** verification — both can inherit the same blind spot from a prior session. Watch for "memo D1-D4 forks already prepared" framing — that's design momentum, not evidence.
- When pushed back ("過度設計嗎?" / "really needed?"), default to genuine YAGNI re-eval rather than steelmanning the shipped code. Cost of slim-down is small; cost of locking in a wrong opinionated policy is paid later.
- Co-locate cross-slice rules with their actual trigger PR, not the prep PR. Real evidence is available there.

**Why**: v3.5.8 Phase 9 Item 5 (PR #270 → slim-down `27c8672c`, 2026-05-14). Memory claimed `(roman, hanji)` ranker dedupe was a "hard prerequisite for Item 12". Codex agreed. Shipped ~155 LOC + 9 tests + spec §10.11 + a locked D2 winner policy. User pushed back. Grep showed `dictionary/build/merge_csv.py:107` already `groupby((hanzi, _tl_key))`; production has zero realistic `(roman, hanji)` duplicates today. Slim-down ripped out the dedupe.

## Workaround circuit-breaker

When a single recurring bug has resisted 3+ fixes — particularly when each fix moves failure rate down but never to zero, OR each silences one host only for the bug to surface in another — **STOP coding**. Re-read the failure logs from scratch and compare against working reference implementations.

| Failed fixes for same bug | Action |
|---|---|
| 1 | Try the obvious mitigation; OK to ship if Codex green-lights |
| 2 | Tighten the mitigation; widen test scope; OK to ship cautiously |
| **3+** | **Circuit-break.** Stop adding code. Re-read failure logs without bringing prior theories. Compare to ≥1 working reference IME in `references/`. The fix is almost certainly removing code, not adding it. |

Specific rules during a circuit-break:

1. **Re-read the original failure log without prior theories.** Diagnostic data is fixed; only your interpretation was wrong.
2. **Compare to references aggressively.** This codebase forks from FlorisBoard and ships alongside Aiongtaigi as references. If their equivalent path differs from ours, that delta is suspect.
3. **Check what the platform default does.** A custom override that "follows X-android approach" without a documented reason is a smell. Reverting to platform default is often the answer.
4. **Resist "but my last fix made it better".** 100% → 5% feels like progress, but if architecture is wrong, the residual 5% resurfaces as new forms. Partial improvement is evidence of partial diagnosis.
5. **Treat the contaminated branch as failed investigation.** Don't add the architectural fix on top of accumulated band-aids — test matrix becomes unanalyzable. Open a clean branch from `main`, apply only the architectural fix.

The Codex sandwich catches per-commit code-correctness issues; it does NOT catch "we are stacking mitigations on a wrong-architecture branch". This circuit-breaker is the meta-process that does.

**Why**: v3.5.0 IME-dismiss investigation (2026-04-25). Eight commits on PR #179 — reactive pipeline, composing pre-zero skip, isGlobeKeyEnabled flow, layout-before-flipper ordering, Chrome `TYPE_NULL` `requestShowSelf` mitigation, widened timing windows — each had a plausible theory and Codex sign-off, but failure rate stayed non-zero and new host scenarios kept appearing. Comparing against `references/aiongtaigi-sushi` and `references/florisboard` immediately surfaced the actual hazard: `MATCH_PARENT × MATCH_PARENT` IME window + custom child-position insets + `TOUCHABLE_INSETS_VISIBLE`. None of the eight commits had touched that layer. Architectural-hazard durable: that exact combination dismisses the IME window in some host apps; reverting to platform-default sizing + inset model was the fix. PR #180 shipped the architectural correction.

## No unilateral release scope

Do NOT decide, propose, or record that a piece of work is "out of vX scope", "post-vX", "deferred", a "documented known limitation", or that a version is "ready to tag / release". **Release scope and timing are the user's decision** — they will give explicit instructions.

- Present large/architectural work **factually** — cost, options, trade-offs — WITHOUT assigning it to (or excluding it from) a release.
- Never write "out of scope for vX", "post-vX", "deferred", "documented limitation", "ready to tag", or "only remaining round" in memory, specs, PRs, or replies unless the user explicitly said so (cite the date + their words).
- Do not run `/release-helper` or cut a tag until the user explicitly instructs it for that version.
- If unsure whether something blocks a release: leave it **unscoped** and ask, or record as an open item for the user's planning session — do not resolve scope yourself.
- Recording the user's *explicit* scope instruction (dated, quoted) is correct; inferring or proposing scope is not.

**Why**: 2026-05-16. After v3.5.8 Bug 3 dogfood, I repeatedly framed the missing inline-composing underline as something to "document as a v3.5.8 known limitation" and pushed segmentation redesign to a "post-v3.5.8 track". User corrected: "不要擅自決定哪些超出 v3.5.8 的範圍,v3.5.8 該 release 的時候我會給你明確的指示". In fact both were must-fix for v3.5.8. My scope calls were wrong AND not mine to make.
