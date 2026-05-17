# Continuous-Input Ranking — Known Limitation

> **Type**: Specification (problem statement, no implementation plan)
> **Keywords**: `Continuous`, `Ranking`, `phrase-priority`, `language-model`, `user_freq_boost`, `taiuantaigi`
> **Related**: [composing.md](composing.md), [sort.md](sort.md), [autocomplete.md](autocomplete.md), [binary-format.md](binary-format.md)
> **Status**: Open. v3.5.8 ships with this limitation documented; resolution deferred.
> **Audit date**: 2026-05-11 (during v3.5.8 Phase 9)

---

## Summary

The v3.5.8 「連續輸入 (Continuous Input)」 candidate ranking compares candidates from **different consumed spans** of the same input buffer using a **flat frequency × syllable-bias** formula with **`user_freq_boost` hardcoded to `1.0`**. This produces user-facing ranking that is misaligned with mainstream IME behavior whenever a low-frequency multi-syllable phrase exactly matches the full input buffer while high-frequency single characters match a short prefix.

**Concrete example (input `taiuantaigi`, 11 chars, 4 TL syllables `tâi-uân-tâi-gí`):**

- Expected (mainstream IME, MOE Tâi-gí, Rime, Google Pinyin behavior): full-buffer phrase 「臺灣台語」 / 「台灣台語」 surfaces in the first 1–3 candidate slots.
- Actual: 「台」 (`freq=31281`, `syll=1`) ranks #1; 「臺灣台語」 (`freq=12`, `syll=4`) ranks **near last** (`score = 12 × 1.3 = 15.6` vs `台 score = 31281`).

This document records the gap, the evidence chain, and the architectural reasoning for why no Phase 9 fix is attempted. Resolution is left to a future release.

---

## 1. Problem Statement

### 1.1 User-expected behavior

For input `taiuantaigi` typed continuously without explicit segmentation:

| Slot | Expected candidate | Reasoning |
|---|---|---|
| #1 | 臺灣台語 (full-buffer exact) | 4-syllable phrase consuming entire input |
| #2 | 台灣台語 (full-buffer exact, alt hanzi) | same TL romanization, common variant |
| #3+ | 臺灣 / 台灣 / 台 / 臺 / etc. | partial/shorter matches as fallback |

Sequential commit flow:

```
taiuantaigi → tap 「臺」 → engine consumes 3 bytes → pending = uantaigi
            → tap 「灣」 → engine consumes 3 bytes → pending = taigi
            → tap 「台語」 → engine consumes 5 bytes → pending = "" → final commit
            → document text: 臺灣台語
```

The mid-commit / final-commit transition mechanics (engine state, byte slicing, NextWord handshake) all work — see §2 below. The disagreement is purely about **which candidate occupies slot #1 at each fetch**.

### 1.2 Why this matters

User of a Continuous-input IME types whole phrases and expects phrase-level matches to surface as units. Burying full-buffer phrase matches below high-frequency single-char prefixes forces the user to scroll past 5–10 single-char candidates per phrase commit, which negates the ergonomic premise of Continuous mode.

---

## 2. Current Implementation (mechanically correct)

### 2.1 Pipeline (cite-and-trace)

| Stage | Code | Behavior |
|---|---|---|
| Syllabifier BFS | [`engine/composing/src/syllabifier/tl.rs:49-84`](../../engine/composing/src/syllabifier/tl.rs) | For `taiuantaigi`, produces endings `{3, 6, 9, 11}` (cap = 8 syllables) |
| Key construction | [`engine/composing/src/dispatch.rs:181-207`](../../engine/composing/src/dispatch.rs) `build_keys_tl` | Strips ASCII tone digits + lowercases + prepends `tl:` → 4 fused-toneless keys |
| Span-local FST fetch | [`engine/lexicon/src/continuous.rs:193-230`](../../engine/lexicon/src/continuous.rs) `fetch_candidates_for_keys` | `prefix_index.lookup_exact` per key + filter + NaN-safe descending sort |
| Score formula | [`engine/ranking/src/score.rs:178-181`](../../engine/ranking/src/score.rs) `calculate_continuous_score` | `freq × (1.0 + 0.1 × max(0, syllable_count − 1)) × user_freq_boost` |
| Mid-commit | [`engine/composing/src/transition.rs:613-679`](../../engine/composing/src/transition.rs) `commit_continuous` | Slices `raw[consumed_bytes..]`, emits 4 effects (commit + preedit + NextWord + autocomplete) |
| Final commit | same fn, `new_pending.is_empty()` branch | `exit_to_idle` + `NextWordWordSelected(trigger_prediction=true)` |
| Frequency record | iOS [`ActionHandler+Suggestions.swift:81-83`](../../ios/Sources/TaigiKeyboard/Actions/ActionHandler+Suggestions.swift) / Android [`CandidateClickHandler.kt:345-349`](../../android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/smartbar/CandidateClickHandler.kt) | Writes `displayText` to `user_frequency.db` on every successful commit |

**Status**: every stage above is implemented and tested. This document does not propose changing any of them.

### 2.2 Generated keys for `taiuantaigi`

| Span | Toneless key | Lookup result |
|---|---|---|
| `0..3` | `tl:tai` | 69 entries — top: 台 (31281), 代 (14215), 大 (11861), 臺 (2050), 帶 (1197), 事 (397), 呆 (300), … |
| `0..6` | `tl:taiuan` | 5 entries — 台員 (1379), 台灣 (1379), 大員 (25), 臺員 (25), 臺灣 (25) |
| `0..9` | `tl:taiuantai` | 0 entries |
| `0..11` | `tl:taiuantaigi` | **1 entry — 臺灣台語 (12)** |

(Frequencies sourced from `dictionary/output/dictionary.csv` column 3.)

---

## 3. The Gap

### 3.1 Computed ranking under current formula

With `user_freq_boost = 1.0`:

| Candidate | `syll` | `freq` | `score` | Actual rank |
|---|---|---|---|---|
| 台 | 1 | 31281 | 31281.0 | **#1** |
| 代 | 1 | 14215 | 14215.0 | #2 |
| 大 | 1 | 11861 | 11861.0 | #3 |
| 臺 | 1 | 2050 | 2050.0 | #4 |
| 台灣 | 2 | 1379 | 1517.0 | #5 |
| 台員 | 2 | 1379 | 1517.0 | #6 |
| 帶 | 1 | 1197 | 1197.0 | #7 |
| 事 | 1 | 397 | 397.0 | #8 |
| 呆 / 獃 | 1 | 300 | 300.0 | #9–10 |
| ... (60+ more 1-syllable `tai` candidates) ... | | | | |
| **臺灣台語** *(user-expected #1)* | **4** | **12** | **15.6** | **near last** |

### 3.2 Three independent contributors to the gap

The gap is the product of three orthogonal design / data choices:

#### Gap A — Ranking formula has no exact-buffer / phrase-priority signal

`calculate_continuous_score` is purely multiplicative on `freq` with a weak syllable bias:

| `syllable_count` | bias multiplier |
|---|---|
| 1 | 1.0× |
| 2 | 1.1× |
| 3 | 1.2× |
| 4 | 1.3× |

Maximum 1.3× cannot bridge a 100×–1000× freq disparity between phrases and single chars. There is no exact-match bonus, no completion-penalty, no language-model probability, no length-priority tier. The legacy `calculate_score` ([`engine/ranking/src/score.rs:86-120`](../../engine/ranking/src/score.rs)) has `EXACT_BONUS = 100` and `COMPLETION_PENALTY = -1000` but those apply only to the non-Continuous lexicon path.

**Code citation**: [`engine/ranking/src/score.rs:178-181`](../../engine/ranking/src/score.rs).

**Design rationale**: Phase 5 explicitly chose a simpler multiplicative form per `docs/roadmap.md:460` — see §6 below for cited mainstream-IME parallel.

**Numeric-precision sub-issue (cross-reference §5.1)**: MOE Tâi-gí carries its candidate ranking as `CandidateModel.weight: Double` (IEEE 754 binary64). Our wire format is `CandidateMessage.score: f32` produced from `freq: u32` × bias × boost. Two implications follow if Gap A is ever addressed by adding a real ranking signal:

- A **language-model log-probability** (e.g. `ln(p)` for `p ∈ (0, 1]`) lives in `(-∞, 0]` with sub-unit fractional precision; `f32`'s ~7 decimal digits is borderline for unigram + bigram + tier sums on a 100k-entry dictionary. MOE's choice of `Double` is the conservative path.
- The current `freq: u32` representation forecloses **non-integer aggregated scores** at the storage layer (the FST + `dict.bin` schema). Even if `score: f32` widens to `f64`, `freq: u32` upstream means the score is just a recoded integer. A real LM or pre-aggregated score would require a `dict.bin` schema bump (analogous to v1 → v2 in Phase 1).

This is a **secondary concern** — the formula gap is the dominant problem, and `f32` is fine for the current `freq × syllable_bias` formula. But any future plan that introduces LM probabilities or pre-computed scores must address numeric width at three layers (`dict.bin` storage / `RawCandidate` in-engine / `CandidateMessage` proto wire) together. See §9 question #6.

#### Gap B — `user_freq_boost` is hardcoded to `1.0`

> **CLOSED by S3 (branch `lattice-s3-userfreq`, 2026-05-16)** for the whole-sentence walker path. The walker's `Σ edge_score` objective now folds in `ranking::decayed_user_weight_delta` (librime `formula_d` wall-clock adaptation, cap-before-decay) multiplicatively, with a McBopomofo epsilon-boost and syllable-aware damping. See the §STATUS 2026-05-16 callout near §7 for the full mechanism and rationale. The literal-`1.0` description below documents the pre-S3 span-local state and the gap evidence chain; it is retained for the audit trail.

`fetch_via_lexicon` calls `fetch_candidates_for_keys` with `user_freq_boost = 1.0` literal:

```rust
// engine/composing/src/dispatch.rs:312-323
fn fetch_via_lexicon(keys: &[(ConsumedSpan, String)]) -> Vec<RawCandidate> {
    LexiconHandle::with_state(|state| {
        ...
        Ok(fetch_candidates_for_keys(keys, u32::MAX, 1.0, prefix, dict))
    })
    .unwrap_or_default()
}
```

This means **`user_frequency.db` is not consulted during Continuous candidate fetch**. The lexicon (non-Continuous) path does read user frequency via the legacy additive `calculate_score`; only Continuous skips it. The platform side records frequency on every commit (§2.1, last row), but the recorded data has no read path back into Continuous ranking — repeated user selection of 「臺灣台語」 has zero effect on the next Continuous fetch's ranking.

This is documented as a Phase 6 → Phase 9 deferred item:

- `docs/roadmap.md:381` — "Phase 6 限制 (deferred to Phase 9 dogfood)"
- [`engine/composing/src/dispatch.rs:301-309`](../../engine/composing/src/dispatch.rs) — inline note explaining the intentional defer

**Cross-IME contrast (MOE Tâi-gí)**: MOE's native API exposes `AddUserVoc(database, hanji, tailo, weight: float)` ([`Tailo.java:5-7`](../../references/moe_taigi_apk/decompiled/sources/moe/taigi/Tailo.java)) — user vocabulary entries carry a **floating-point weight** that is read directly by the C++ ranker via the same `tutgDataBase` handle the dictionary uses. This means MOE's user-selection feedback enters the ranking path on the very next keystroke, with no separate plumbing layer. Our user_frequency.db lives platform-side and is read only by the legacy non-Continuous lexicon path; closing this loop for Continuous is what Gap B fix would entail. (See §7 Goal G2.)

#### Gap C — Dictionary phrase coverage and frequency calibration

The dictionary contains:
- 101,773 distinct `tl_notone` keys
- Single-character entries with frequencies up to **184,693** (e.g., 「的」 `e`)
- Multi-syllable phrase entries with frequencies typically **100×–1000× lower**:
  - 「臺灣台語」 = 12
  - 「台灣人」 = 169
  - 「台語」 = 394

Additionally, the phrase 「台灣台語」 (with 「台」 not 「臺」) does **not exist** in the dictionary at all. Only 「臺灣台語」 is present, with `freq = 12`.

**This is a dictionary pipeline / data-source issue** (`dictionary/`, `taigi-converter/`), not an engine algorithm issue. Even if Gap A and Gap B are addressed, the user-expected variant 「台灣台語」 will not appear until added.

### 3.3 Why mid-commit and final-commit work despite the ranking gap

The transition state machine (`commit_continuous`) operates on byte spans and `display_text` provided by the caller (engine produces them; platform forwards them verbatim from `ContinuousCandidate`). Once the user **does** select a candidate, the slicing, NextWord handshake, and frequency recording are all correct and fully-tested (Phase 4 + 5 + 7B + 8 work). The gap is strictly upstream of selection — at the point where the candidate strip is rendered.

---

## 4. Independent Co-confirmation (Codex)

A second-opinion review was conducted via Codex on 2026-05-11 (transcript: `/tmp/codex-v358-ranking-coconfirm.txt`). Findings:

| Question | Codex verdict |
|---|---|
| Is the ranking gap real? Any hidden rescue path? | **YES** real. No re-rank hook on iOS or Android; both wrap engine candidates in order. `1.3×` bias cannot bridge `freq=12` vs `freq=31281`. |
| Is the user-expected behavior aligned with mainstream IME convention? | **YES**. Google Pinyin / Microsoft IME / Rime all surface full-buffer phrase matches at or near top. Khiin-rs's `cost = ln(1/p) / word_len_bias × syllable_bias` is segmentation cost, not candidate UI ordering — citing it does not justify burying phrases on the candidate strip. |
| Is the user expectation closer to LM-based phrase ranking than flat frequency? | **YES**. Rime explicitly uses Viterbi over a lattice with bigram/trigram LM + user dictionary; the user's expectation is "IME phrase conversion / LM ranking," not flat span-local frequency. |
| Is a small `×10` exact-match multiplier (option B) sufficient? | **NO**. `12 × 10 = 120` vs `31281` — still buried. A meaningful exact-buffer rule would need to be a tiering / rank-key change, not a small multiplier. |
| What about Phase 9 scope? | **Defer**. None of the candidate fixes (LM, boost plumbing, dict reorganization, exact-match tier, dict completion) fit Phase 9's "≤ 200 LOC + dogfood notes" budget. **Document and ship.** |

Codex's direct quote on architectural verdict:

> "Continuous ranking is architecturally wrong for full-buffer phrase expectation because it compares candidates from different consumed spans on raw dictionary frequency. If v3.5.8 is dogfood-oriented, ship with a dogfood note. If it is meant as a polished public 'Continuous Input' release, this specific behavior is bad enough to block that claim."

---

## 5. Mainstream IME Comparison

### 5.1 MOE Tâi-gí (decompiled APK, primary benchmark)

Decompiled APK at `references/moe_taigi_apk/`. Although Java side is obfuscated (single-letter class names: `A0`, `B0`, …), the design surface visible above the JNI boundary is informative:

| Signal | Evidence |
|---|---|
| Native-side ranking | `decompiled/sources/moe/taigi/TailoJNI.java` exposes `GetCandidatesAtNailPos()` — candidates are returned **pre-ranked from C++** native library. Java does no post-sort (zero hits on `Comparator` / `.sorted()` / `.sort()` in candidate-flow code paths). |
| Per-candidate metadata | `decompiled/sources/android/moe/taiwanese/taigi/data/local/model/CandidateModel.java` declares first-class fields: `double weight` (ranking weight), `int spanUnits` (phrase length in syllables), `int words` (phrase word count), `long position` (byte position in buffer). |
| Position-aware fetch | `GetCandidatesAtNailPos(...)` — note the **`AtNailPos`** semantics: candidates are fetched at a specific buffer position, with `position` and `spanUnits` carried back as part of each candidate. This mirrors our `ContinuousCandidate.consumed_span_{start,end}` design. |
| Dictionary | `extracted/assets/tailo.tab` (3.1 MB, opaque binary) — contains tailo romanizations and (presumably) per-entry weights. Not human-readable, no SQLite, no TSV. |

**Likely strategies (medium confidence, since C++ side not decompiled):**

- **(A) LM-based ranking** in C++ — Viterbi over phrase lattice with ngram probabilities. Would naturally surface full-buffer phrase as a unit.
- **(B) Phrase-priority tier** — `spanUnits` field is persistent (not computed on the fly), suggesting dictionary entries are pre-categorized by phrase length. The native ranker may bias toward larger `spanUnits` when a buffer-spanning match exists.

The two are not mutually exclusive. What is **not** consistent with the visible surface:

- **(C) Pure flat frequency** — the explicit `weight` (double, not int) field and the `AtNailPos` position-aware API suggest more nuance than a `freq × syllable_bias` constant.

#### 5.1.1 Segmentation API surface (斷詞)

The full JNI surface ([`TailoJNI.java`](../../references/moe_taigi_apk/decompiled/sources/moe/taigi/TailoJNI.java)) reveals MOE's input model:

| Concept | API | Our equivalent |
|---|---|---|
| Buffer ownership | C++ stateful `tutgInputLine` (opaque handle) | Rust `Phase::Continuous { raw, nailed }` |
| Per-keystroke entry | `InsertKey(InputLine, char)` (single char per call; 11 calls for `taiuantaigi`) | `Intent::Append { ch: String }` (batch-friendly) |
| Caret cursor (edit position) | `MoveBack` / `MoveForward` / `MoveTo` / `GetCaretPosition` | None — buffer-end-only |
| Nail cursor (commit anchor) | `GetNailPosition` / `NailCandidate(VocType, id)` / `DoneNailing` | `commit_continuous(consumed_bytes)` advances implicitly |
| Segmentation query | `GetSegmentaions(InputLine, MessageHandler)` — **never called by Java** (engine maintains internally) | `valid_span_endings` — internal to dispatch, not exposed |
| Segmentation toggle | `ToggleSegmentation(InputLine)` — exists but no Java call site found | None |
| Front-segment pop | `PopFront(InputLine)` + `SetPopFrontHandler` | `commit_continuous` final-commit branch (`pending.is_empty()`) |
| Backspace | Two-step: `MoveBack` + `RemoveKey(Direction.FORWARD)` | One-step: `Intent::DeleteBackward` |
| Composition mode | `CompositioMode { CM_STANDARD, CM_EAZY, CM_TAILO }` (3 modes; sic, "Compositio" not "Composition") | `input_mode { TL, POJ, TPS }` — different axis |
| Candidate VocType | `VocType { VT_HANT, VT_TAILO, VT_MIXED }` (3 vocab tiers) | `form: u8 = 1` always | 

**Java consumer pattern** (from obfuscated decompiled call sites):
- `D4/x.java:111` — `EnableIslandDoctrine(handle, true)` set **once at init, no UI toggle, always ON**
- `J/v.java:45` / `J/w.java:51` — `Tailo.InsertKey` per-keystroke
- `J/n.java:66` — `Tailo.GetCandidatesAtNailPos(handle, VocType, range_start, range_end, callback)`
- `J/y.java:57` — `Tailo.NailCandidate` per user selection
- `J/o.java:51` — `Tailo.GetConfirmedCandidates` queries already-nailed segments
- `J/z.java:42-43` — backspace path

Implication: MOE's flow is **eager-segment-internal / lazy-segment-exposed**. Engine re-segments after every `InsertKey`, but UI never reads segment endpoints — it only queries candidates **at the current nail position** via `GetCandidatesAtNailPos`. Segmentation is an implementation detail of the native engine; not part of the Java/UI contract.

#### 5.1.2 Black-box limitations

| Item | Status | Implication for our spec |
|---|---|---|
| C++ ranking algorithm (`libtutg.so`) | Opaque, not decompiled | Cannot copy MOE's ranking; we can borrow design surface only |
| `tailo.tab` (3.1 MB binary dictionary) | Opaque format, no schema dump | Cannot directly inspect per-entry weights or phrase tier data |
| **`IslandDoctrine`** (`EnableIslandDoctrine`) | Public toggle, hard-coded `true` at init, no UI exposure, no docstring | **Unknown semantic.** Name suggests "isolated-island disambiguation" — possibly a strategy where ambiguous syllable boundaries are kept as isolated candidates until user resolves them. This is a research item, not a copyable design. |
| `CompositioMode CM_EAZY` (sic) | Toggle exists, semantic unknown | Possibly "tolerance" or "skill level" mode for novice users |
| `tutg` codename | C++ project name | No public docs found |

(Research notes: `references/moe_taigi_apk/README.md` is a one-liner; full agent research summaries archived in session memory of session b5ed5e81.)

### 5.2 Khiin-rs (`references/khiin-rs/`)

| Component | Behavior |
|---|---|
| Segmenter | `khiin/src/data/segmenter.rs:122` uses `cost = ln(1/p) / word_len_bias × syllable_bias` — **segmentation cost**, not candidate UI ordering. |
| Buffer manager | `BufferMgr` (`buffer_mgr.rs:44-66, 1096-1129`) — nailed segments + raw_char_count (mirrored by our `Phase::Continuous { raw, nailed }`). |
| Translation | Khiin-rs's pipeline goes Segmentation → Conversion → Buffer, with frequency-driven scoring at each stage. |

Khiin-rs has the same flat-frequency limitation we have. Its segmentation cost formula is a useful reference for the segmentation half of the problem but does **not** answer how to rank cross-span candidates on the candidate strip.

### 5.3 Rime / librime (`references/rime/` if present, else community docs)

| Component | Behavior |
|---|---|
| `script_translator.cc` | Has explicit exact-match / full-code handling and user-phrase preference logic; not just flat frequency across all prefix spans. |
| `UnionTranslation` (`script_translator.cc:111-178`) | Variable syllable_count candidates merged in one list (mirrored by our `fetch_candidates_for_keys`). |
| Language model | Rime is modular; supports Viterbi with bigram/trigram + per-user phrase dictionary. |

Rime is the canonical reference for "candidate UI exact-match phrase priority." Their approach is **language-model-driven**, not frequency-tweak-driven.

### 5.4 Google Pinyin / Microsoft Chinese IME (documentation only)

Both vendors document candidate lists prioritizing user-typed phonetic exact matches against a user dictionary that improves accuracy over time. Specific algorithms not public, but the user-facing contract is clear: typing `taiwantaiyu` (Pinyin equivalent) should surface 「台灣台語」 in the first slots, not bury it.

---

## 6. Architectural Classification

The gap decomposes into **three independent problems**, each with different scope:

| Gap | Root cause | Engine vs platform | Scope estimate |
|---|---|---|---|
| **A. Formula** | `calculate_continuous_score` has no phrase-priority / exact-buffer / LM signal | Engine (`ranking/src/score.rs`) | Algorithm change; needs benchmark dataset to tune |
| **B. Boost wiring** | `user_freq_boost = 1.0` hardcoded; `user_frequency.db` unread by Continuous | Engine + proto + platform | Cross-cutting (proto field + AppConfig + DB read + dispatch wire-up) |
| **C. Dictionary data** | Phrase entries have 100×–1000× lower freq than single chars; some user-expected variants missing | Dictionary pipeline (`dictionary/`, `taigi-converter/`) | Data calibration; orthogonal to engine code |

Fixing only A while leaving B and C alone produces **cold-start better, no learning** — user selection still doesn't influence ranking. Fixing only B leaves the cold-start short-bias intact. Fixing only C is a dictionary-frequency recalibration project independent of engine code.

A holistic fix requires all three — which is by definition out of Phase 9 scope.

---

## 7. Long-term Goals — Align with Mainstream IME

**Target**: bring Continuous-input behavior into alignment with the conventions established by mainstream IMEs ── primarily **MOE Tâi-gí**, **Rime / librime**, and **Google Pinyin**. These three are the reference set; alignment with one is generally consistent with all three because they share architectural primitives.

This section states **direction**, not schedule. Per `feedback_no_future_planning.md`, no specific release version is committed here. The goals are the criteria a future plan must satisfy to "close" this spec.

### 7.1 Goal axes

| ID | Goal | What "aligned" looks like | Mainstream evidence |
|---|---|---|---|
| **G1** | **Phrase-priority candidate ranking** — when input matches a phrase exactly, that phrase surfaces in the first 1–3 slots; partial / shorter matches fall to lower slots. | For input `taiuantaigi`: 「臺灣台語」 / 「台灣台語」 in slot #1–2; 「台」 / 「臺」 / 「臺灣」 below. | librime `script_translator.cc` exact-match handling; Google Pinyin documents phrase-conversion preference; MOE's `weight: Double` + `spanUnits` + position-aware `AtNailPos` design. |
| **G2** | **User-selection feedback closes the loop** — selecting a candidate raises its rank on the next type-then-fetch of the same input, with effect visible within one or two interactions. | Selecting 「臺灣台語」 once causes it to surface above 「台」 on the next `taiuantaigi` input. | MOE `AddUserVoc(db, hanji, tailo, weight: float)` enters the same C++ ranker the dictionary uses; librime `user_dict` + `Memory::CommitEntry` updates per-user phrase weights; Google Pinyin learns from selections. |
| **G3** | **Engine is the ranking authority** — platform side never re-ranks, never injects an exact-match bonus, never filters by length. | Already mostly met (see §2 audit). Goal is to keep this invariant as G1 + G2 land. | All three reference IMEs: ranking is in the native / engine layer; the UI is a thin renderer of an ordered list. |
| **G4** | **Multi-vocabulary candidate axis** — distinguish hanji vs roman vs mixed candidate types as a first-class proto field, not a string-content sniff. | Each candidate carries an explicit `mode: HANT/TAILO/MIXED` (or equivalent); UI can offer mode-toggle without re-querying. | MOE `VocType { VT_HANT, VT_TAILO, VT_MIXED }`; librime translator chain produces typed candidates; Google Pinyin distinguishes hanzi from pinyin completion. |
| **G5** | **Stateful buffer with explicit commit semantics** — segment-by-segment commit (mid-commit advances anchor; final-commit pops buffer). | Already met for byte-level (`commit_continuous`). MOE/librime go further with caret + nail dual cursor enabling mid-buffer edit; this is **stretch**, not a baseline G. | MOE `caret` + `nail` two cursors; librime `BufferMgr` with explicit `commit_history`; khiin-rs same. |

### 7.2 Non-goals — mainstream patterns we explicitly do NOT pursue

To prevent goal-creep:

| Mainstream feature | Reason we exclude |
|---|---|
| MOE `IslandDoctrine` semantics | Black-box C++ algorithm; cannot reverse-engineer without C++ source. We may borrow design surface but not algorithmic detail. |
| MOE `CompositioMode { STANDARD, EAZY, TAILO }` skill modes | Two simultaneous experience axes (input difficulty × romanization scheme) over-complicates the AppConfig surface for solo-maintainer scope. We keep one axis (`input_mode = TL/POJ/TPS`). |
| Rime SchemaYAML user customization system | Per `docs/roadmap.md` §刻意不採用; over-engineered for this product. |
| Google Pinyin cloud-based language model | Privacy + offline-capability constraint; must run on-device. |
| Rime full lattice + Viterbi over arbitrary length | Possible long-term, but MSRV / runtime budget on iOS keyboard extension may not allow; treat as stretch evaluation, not baseline G. |

### 7.3 Goal-to-Gap traceability

| Goal | Closes which Gap (§3.2) |
|---|---|
| G1 | Gap A (formula) — primary. Possibly Gap C (dict data) as enabler. **Closed by S2** (whole-sentence walker). |
| G2 | Gap B (boost wiring) — primary. **Closed by S3** (`decayed_user_weight_delta` + epsilon-boost into walker edge cost; see §STATUS 2026-05-16). |
| G3 | Already met by current architecture. Goal is to **preserve** through G1 + G2 changes. |
| G4 | Independent of A/B/C — proto schema extension. Tracked at §9 question #6. |
| G5 (stretch) | Independent of A/B/C — input-state-machine extension. Tracked at §9 question #7. |

A future plan that **closes G1 + G2 + G4** while preserving G3 is the success criterion for retiring this spec. G5 may close in the same plan or in a follow-up; it is the boundary between "ranking-aligned" and "full-IME-aligned."

---

## 8. Decision for v3.5.8

> **STATUS 2026-05-16 (整句 lattice + walker 進度)**: Gap A (§3.2 — no phrase-priority signal, the §1 `taiuantaigi` motivation) is now **closed by the whole-sentence walker**: `docs/roadmap.md` §整句 lattice + walker **S1 DONE & MERGED** (main `4caa0c24` #284, behavior-neutral lattice builder) + **S2 DONE** (branch `lattice-s2-walker`, engine-only `walk_best` relaxation walker emitting one synthesized full-buffer best path at slot 0 — `taiuantaigi`→臺灣台語, no-hanji path→synthesized roman, subsuming paused Bug 2). G1 converges at S2. **Gap B (§3.2 — `user_freq_boost` hardcoded `1.0` in the walker path objective) is now closed by S3** (branch `lattice-s3-userfreq`): `ranking::decayed_user_weight_delta` is a librime `formula_d` wall-clock adaptation (`delta = (user_freq_boost(count) − 1) × exp(−age_ms / τ)`, **cap applied before decay** so a huge stale count is not pinned high — Codex pre-impl S3 Q4a/Q4c BLOCK condition; τ = `USER_WEIGHT_DECAY_TAU_MS` = 30 days, dogfood-tunable 14–90 days), folded multiplicatively into `composing::lattice::cost::edge_score` together with a McBopomofo-style additive epsilon-boost on multi-syllable edges (`WALKER_PHRASE_EPSILON` = 0.001) and **syllable-aware damping** (`WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE` = 0.0) so a hot single character cannot ride the boost to sweep the whole sentence. Seam (Codex S3 Q4d): `EdgeChoice.user_weight_delta` computed in `dispatch::fetch_walker_slot0`; `lexicon::best_candidate_for_key` / `record_to_candidate` untouched (record selection and path objective are orthogonal — no double counting). The S2 no-dict tie lever is preserved (no-dict edge → `user_weight_delta = 0.0` → `edge_score` still exactly `1.0`). **G2 converges at S3.** G4/G5 unchanged. Engine-only; forward-only Model B commit preserved (Codex pre-impl S2 Q1c = option ii). The pre-2026-05-11 "ship with limitation documented" path below is fully superseded.

> **REVISED 2026-05-11 (evening)**: Original decision was "ship with limitation documented." User pivot: **v3.5.8 will not ship until Continuous-input ranking is fixed.** Phase 9 scope expanded from "dogfood + cleanup" (~100 LOC) to "ranking 修復 + 主流 IME 對齊" (TBD;Goal axes G1+G2+G4 base, G5 stretch). See `docs/roadmap.md` § Phase 9 (revised) and `memory/project_v358_continuous_input.md`.
>
> **Original §8 text preserved below for hand-off recovery.** It describes the deferred path that was rejected by the 2026-05-11 pivot.

---

**[ORIGINAL — superseded]** Ship with this limitation documented. Phase 9 budget (≤ 200 LOC + dogfood notes per `docs/roadmap.md:447`) does not accommodate a meaningful fix. Per `feedback_no_future_planning.md`, this document does **not** propose specific scoping for v3.5.9+.

Constraints binding the decision:

- v3.5.8 is dogfood-oriented (per `docs/roadmap.md` Active item header).
- `feedback_no_slice_toggles.md` — no fallback toggle to disable Continuous; we ship as-is or we don't ship.
- Solo maintainer (`feedback_solo_maintainer.md`) — ranking work blocks other v3.5.8 phases if attempted now.
- `feedback_round_hygiene.md` — context-clear between rounds; this decision must be recoverable from this document alone.

**What v3.5.8 release notes should communicate to users:**

- Continuous-input候選詞 ranking 目前以單字頻率為主。對長 phrase 的全 buffer exact-match 不會優先;若全 phrase 沒在第一個候選位,請選短 candidate 逐段 commit。
- 此版本記錄候選使用 (frequency learning) 用於後續 release 的 ranking 調整。

(Final wording belongs in the release CHANGELOG, not this spec.)

---

## 9. Open Questions (deferred — not in scope for this spec)

These are noted to prevent re-discovery in future sessions. Not committed to any release.

1. **Should phrase-priority be a hard tier or a soft bias?** Hard: "if any candidate consumes the full buffer, show only those." Soft: "boost full-buffer matches by a tier multiplier." Hard is simpler but may show empty strips for nonsense input; soft requires LM probability or curated phrase weights.
2. **Should `user_freq_boost` plumb through `AppConfig` (live-read each fetch) or extend `FetchAtPos` (per-candidate boost)?** Cross-platform AppConfig has live-read semantics per `behavioral-invariants.md` §11; per-candidate boost requires the platform to send N values per fetch. AppConfig path is cheaper for the Continuous use case.
3. **Should the dictionary build pipeline derive phrase frequencies from corpus rather than per-source dictionary presence flags?** Currently freq = sum/max over `kautian / taigitv / itaigi / etc.` source bits, which under-represents real usage frequency for multi-character phrases. Corpus-derived frequencies would re-calibrate the spread.
4. **Should we adopt MOE Tâi-gí's `position`-keyed `AtNailPos` design?** Our `consumed_span_{start,end}` is similar; the question is whether to expose it to platform UI for navigation features (e.g. tap a Hanji segment in the document to revisit its candidate strip), which is a UX feature, not a ranking fix.
5. **Bigram / trigram LM** — feasible scope and data source? Outside v3.5.8 explicit non-goals (`docs/roadmap.md` § Phase 5 deferred); recovery requires a corpus. Open whether to commission one.
6. **Should `CandidateMessage` proto schema absorb four MOE-style metadata fields?** MOE Tâi-gí's `CandidateModel` + `VocType` carry concepts we do not currently emit:
   - **`mode: HANT / TAILO / MIXED`** — explicit candidate-type discriminator. MOE's `VocType` enum has **three** values: `VT_HANT` (漢字 only), `VT_TAILO` (羅馬字 only), and **`VT_MIXED`** (mixed-script entries like 「台BAR」). Our `form: u8 = 1` (FORM_NOTONE) is currently fixed and carries no hanji-vs-roman-vs-mixed signal — UI must sniff `display_text` for non-ASCII to distinguish, which is fragile for mixed entries. Closing G4 (§7) requires this field.
   - **`words: Int`** — phrase **word count**, distinct from our `syllable_count`. A 3-syllable single-word entry (`tâi-uân-uē` 「台灣話」, 1 word) and a 3-syllable 2-word compound (`tâi-uân + lâng` 「台灣 + 人」, 2 words) would carry different `words` even though both have `syllable_count = 3`. Library-call equivalent in librime is the segment-level `Confirmed` vs `Selected` distinction (`librime/src/rime/context.h:101-105`).
   - **`vocabulary: <source tag>`** — per-candidate dictionary-source attribution. We have a 12-bit `source_bitmask` on each `DictionaryRecord` ([`engine/lexicon/src/dictionary_reader.rs`](../../engine/lexicon/src/dictionary_reader.rs)) but it is consumed only by `Filter::from_enabled_bitmask` for inclusion/exclusion — never surfaced to the UI. Exposing the first-match source label (matching `SOURCE_TIERS` priority order in [`engine/ranking/src/score.rs:65-70`](../../engine/ranking/src/score.rs)) would enable optional source-attribution UI without a new dictionary scan.
   - **Numeric widening for `weight`** — see §3.2 Gap A "Numeric-precision sub-issue." If LM probabilities or pre-aggregated scores land, `freq: u32` must widen across `dict.bin` storage / `RawCandidate` / `CandidateMessage` together.
   
   Decision pressure: the `mode` field is the cheapest add (one new u32 in proto, one branch in `record_to_candidate` to set it; data partly derivable from existing `hanzi.is_some()`). `words` requires a builder change to `dict.bin` (count word boundaries during the notone stage) — touches the Phase-1 v2 schema. `vocabulary` is a pure proto/wire addition; the data already exists in `DictionaryRecord.bitmask`. `weight` widening is a v2→v3 dict.bin schema change. Defer all four until a plan stage decides whether to ship with Gap A / B fixes.

7. **Should the proto evolve from byte-level commit (`consumed_bytes`) to segment-level commit (`NailCandidate(VocType, candidate_id)` style)?** MOE's commit API takes a `VocType` + `candidate_id` pair and lets the engine compute byte advancement internally; ours requires the platform to forward `consumed_bytes` verbatim from the candidate (which the engine itself produced). Functionally equivalent today, but MOE's design enables: (a) caret + nail dual-cursor model that supports mid-buffer edit (`MoveTo` + `InsertKey` mid-segment), (b) less platform-side metadata round-tripping (`additionalInfo[CONSUMED_BYTES]` sidechannel becomes redundant), (c) tighter coupling of commit semantics to engine state. **G5 (§7) — stretch.** Closing this requires re-shaping `Phase::Continuous` to track caret position separately from buffer end, expanding `Intent` with `MoveTo` / `MoveBack` / `MoveForward` variants, and shifting `commit_continuous`'s contract from "platform tells us how many bytes" to "platform tells us which candidate at the current nail." Out of scope for v3.5.8.

---

## 10. Commit Behavior & Display Split

> **Status**: **Model B — normative, implemented (engine P1).** Rewritten 2026-05-16 (v3.5.8 Phase 9 Bug 3 closeout). Codex design co-review PASS.
> **Scope**: v3.5.8 Phase 9. The commit/display contract is an **engine effect-model** decision (`engine/composing/src/transition.rs`), not a UI-layer one; bindings are thin effect translators.
> **Replaces / supersedes**: the pre-2026-05-16 "nailed segments are literal document text; composing buffer = pending tail only" model (clarification β). Under **Model B** the whole composition (nailed segments + pending tail) lives in **one** marked / composing region until a hard finalize; nailed segments are **not** in the host document. This is the mainstream-IME-unanimous model (librime / khiin-rs / MOE / azooKey — see §10.1.1) and it eliminates the iOS mid-commit tail-leak (former Bug 3) by construction rather than by compensation.

### 10.1 Motivation

§1–§9 address **which candidates appear and in what order**. They do not specify **what the user sees being typed** versus **what gets committed** when the user presses Enter or taps a candidate. Two dogfood findings (2026-05-11) converged on the same root cause — an unspecified split between the composing buffer and candidate slot 0:

1. The position-0 candidate cell carries a dashed border that visually conflates it with the composing buffer.
2. The lexicon path and the continuous path each build candidates independently; the lexicon path emits `subtitle=nil`, producing inconsistent slot-0 rendering when both paths fire.

This section establishes a single normative contract for **display split + commit dispatch** in Continuous mode.

### 10.1.1 Mainstream IME source — Model B is unanimous

The contract in §10.2–§10.4 is **Model B**: during continuous input the whole composition — already-**nailed** segments **plus** the pending raw tail — stays inside **one** preedit / marked / composing region until a hard finalize (Enter / final-commit / external-suggestion commit). Nailed segments are **never** written to the host document mid-composition; a candidate tap *nails* a segment inside the composition, and only a hard finalize writes literal text. This is the **unanimous** behavior of every mainstream IME surveyed (research 2026-05-16) — no surveyed IME does immediate partial literal commit mid-composition:

| IME | Evidence (file:line) | Mechanism |
|---|---|---|
| librime / RIME | `references/librime/src/rime/context.h:48-61`, `composition.cc:24-119` | One `Context::GetPreedit()` over the whole `Composition`; selecting a candidate only raises segment `status >= kSelected`; literal text leaves only via `GetCommitText()` on hard `Commit()`. |
| khiin-rs | `references/khiin-rs/swift/osx/src/controller/InputController.swift:103-275`, `IMKTextInputExtensions.swift:14-33` | One `client.mark(currentDisplayText())` over all segments; `client.insert()` only at hard commit; **re-marks the remainder** after a partial commit rather than leaving a bare tail. |
| MOE Tâi-gí | `decompiled/.../KeySectionsModel.java` (via `docs/references/moe-taigi-reference.md:68-124`) | `KeySectionsModel { composedCharacters; composingCharacters }` — **both** fields live inside the Android composing region (`setComposingText`); `composedCharacters` = nailed text, `composingCharacters` = pending raw. Literal text reaches the document only via the explicit `PopFront` / `GetConfirmedCandidates` commit API. |
| azooKey | `references/azooKey-Desktop/Core/Sources/Core/InputUtils/SegmentsManager.swift:1009-1062`, `azooKeyMacInputController.swift:753-792` | One `setMarkedText` whose attributed string carries `.focused` (nailed prefix) + `.unfocused` (pending tail) runs; `insertText` only at hard finalize. |

**MOE re-interpretation (corrects the pre-2026-05-16 reading).** The earlier spec read MOE's `composedCharacters` as text already committed to the document and treated the composing buffer as the pending tail only. The decompile evidence is the opposite: `composedCharacters` + `composingCharacters` **both** sit in MOE's composing region (`setComposingText`), exactly Model B. The `CandidateModel.spanUnits` per-candidate segmentation still maps to slot-0's "segmented version" (§10.2, unchanged). Adopting Model B therefore strengthens — not weakens — alignment with the de-facto Taigi baseline.

**The iOS / macOS technique (the crux).** azooKey (iOS/macOS) and every RIME front-end keep the whole composition in a **single** `setMarkedText(_, selectedRange:)` call; nailed-vs-pending is encoded as attribute runs *within* that one string. The host never confirms a sub-region into literal text because the document is never told a sub-region is final. Our former approach — `insertText(nailed_prefix)` then `setMarkedText(tail)` per mid-commit — is the inverse and has **zero precedent** in any surveyed IME; it is the root cause of the iOS tail-leak (former Bug 3, §10.6).

**Why this is the right call:**

- **Convention familiarity** — matches the IME many Taiwanese users already know (MOE) and the cross-IME consensus; satisfies CLAUDE.md rule 16 (mainstream-comparison-driven design) with a four-IME "Project X already does Y" cite.
- **Decouples "what I typed" from "what the engine guessed"** — preserves user agency under uncertain segmentation (`rules/cross-platform-alignment.md`; G3 in §7).
- **Eliminates the iOS leak by construction** — nothing is literal-committed mid-flow, so there is no sub-region for the host to confirm; the iOS arm/detect/compensate workaround is deleted, not extended.
- **Maps onto the existing engine** — no proto / wire change; `Phase::Continuous` already retains every nailed segment's `display_text`, so the engine simply emits one combined `UpdatePreedit` instead of an eager per-segment `CommitTextReplacingPreedit`.

### 10.1.2 Supersedes — slot-0 model unification (Codex co-review clarification α, 2026-05-13)

Prior to §10, [`continuous-candidate-display.md`](continuous-candidate-display.md) modeled slot 0 as a **dedicated composing-text cell** with `isComposingText="true"` metadata — visually distinct (dashed border, rounded background, vertical inset) and **separate from ranker-produced candidates**. Specifically:

- `continuous-candidate-display.md` §4.6 "Slot-0 stays single-line. Pending preedit has no hanji. **This is intentional**."
- `continuous-candidate-display.md` §15.4 "`buildContinuousSuggestions` already always insert slot-0 composing-text cell — so the 'no candidates' UX is automatically preserved (strip shows slot-0 only)."

**§10 supersedes both for Continuous mode**: slot 0 is the **engine ranker's top candidate rendered with segmentation** (§10.2). There is no separate composing-text cell in the strip; the only composing-text surface is the inline host-app pre-edit (`markedText` / `setComposingText`), which under **Model B** carries the **whole composition** — `Σ nailed[i].display_text` + the pending-tail derived form ([`Phase::composing_display`](../../engine/composing/src/api.rs)) — **not** `rawInput` (pending-tail only). `rawInput` ([`Phase::raw_input`](../../engine/composing/src/api.rs)) is demoted to an internal *component* of that surface.

| Aspect | Pre-§10 legacy model | §10 (current spec for Continuous mode) |
|---|---|---|
| Slot 0 content | Composing-text cell (raw, no hanji) | Engine ranker top candidate (segmented; may have hanji) |
| Slot 0 visual | Dashed border + rounded bg + vertical inset | Identical to slots 1..n (no visual distinction — maintainer call 2026-05-13) |
| Tap slot 0 | Commit raw | Commit `candidate[0].display_text` (segmented; see §10.3 + clarification γ) |
| `isComposingText` metadata | Drives visual affordance + click routing | Click-routing only (visual gone in commit `224a8aa3`) |

**Non-Continuous mode** (legacy lexicon path) retains the pre-§10 slot-0 model. §10.5 Mode Gating is the boundary. `continuous-candidate-display.md` §4.6 / §15.4 wording therefore remains accurate **for non-Continuous mode only** — both will carry inline "superseded by §10 in Continuous mode" notes after Item 1 (this section) ships.

Implementation status: visual unification shipped in `224a8aa3` (Item 14, slot-0 dashed border + Android inset/bg/corner removed). Click-routing semantics shipped in Item 4 — the Continuous path no longer inserts a composing-text cell at slot 0; `candidate[0]` is the engine ranker top and Tap-0 routes through `commitContinuous(candidate[0].display_text)` (clarification γ). Segmented content of slot 0 (visual dual-line roman/hanji rendering) lands in Items 5+6.

### 10.2 Display Contract

| Element | Content | Source |
|---|---|---|
| **Composing buffer** (inline pre-edit in the host app — iOS `markedText` / Android `InputConnection.setComposingText`) | **The whole composition** (Model B): `Σ nailed[i].display_text` joined with the pending-tail **`rawInput`** by the **§10.2 word-boundary separator** (see *Segmented-spacing contract* below — **v3.5.8 change**, was "no inter-segment spaces" through v3.5.7). The nailed prefix is the accepted candidates' `display_text` (already swap/TPS/both-scripts-formatted); the tail is the derived display of the pending raw — hyphen-delimited chunks NFC-normalized and tone-marked where convertible, unhyphenated input passed through verbatim. **No** engine syllabification of the tail. | [`Phase::composing_display`](../../engine/composing/src/api.rs) (= [`api::nailed_prefix`](../../engine/composing/src/api.rs) `Σ display_text` separator-joined + [`Phase::raw_input`](../../engine/composing/src/api.rs) → [`derived::derived_display`](../../engine/composing/src/derived.rs)) |
| **Candidate strip, index 0** | **Segmented version** — segmenter + ranker top candidate over the same raw input bytes, with word boundaries inserted by the segmenter (**roman line gets word-boundary spaces; hanji line rendered as-is** — see segmented-rendering rule below). Independent of the inline-preedit `rawInput` contract above; produced from dictionary records, not from the `Phase::raw_input` derived string. | Segmenter + ranker top candidate |
| **Candidate strip, index N ≥ 1** | As defined by §1–§7 (existing ranker output). | Existing path; unchanged |

**Composing-buffer surface (Model B):** the host's single marked / composing region renders [`Phase::composing_display`](../../engine/composing/src/api.rs) = the nailed prefix (`Σ nailed[i].display_text`, verbatim — already formatted at nail time) followed by the pending-tail `rawInput` defined below. The nailed prefix is **not** in the host document; it is part of the marked region until a hard finalize (§10.3). The platform caret/`selectedRange` sits at the **end** of this combined string.

**Segmented-spacing contract (v3.5.8, `output_both_scripts` AppConfig field).** The Model B composing buffer joins adjacent `nailed[i].display_text` — and the nailed prefix ↔ pending tail — with a single ASCII space **iff the rendered script is roman-ish**: roman-first (`!is_translate_swapped`), or both-scripts (`hit (彼)`). Hanji-first (`is_translate_swapped` without `output_both_scripts`) and TPS render hanji/bopomofo as-is with **no** inter-segment space. Predicate (single-sourced in [`api::continuous_word_space`](../../engine/composing/src/api.rs), mirrored by the platform `appendAutoSpaceIfApplicable`): `!(effective_swapped && !output_both_scripts)` where `effective_swapped = is_translate_swapped || input_mode == "tps"`; the separator is additionally suppressed after a hyphen-continuation segment (`tai-`). This corrects the v3.5.7 dogfood bug `Hittui → HitTui` (expected `Hit tui` in roman-first). `is_translate_swapped` alone could not distinguish hanji-first (no space) from both-scripts (space) — both set it `true` — hence the dedicated `output_both_scripts` flag (Codex pre-impl 2026-05-18). The separator is a pure presentation/commit-render concern and is **never** stored in `NailedSegment.display_text` (backspace-pop restores the editable tail from `NailedSegment.raw_text`, which must stay separator-free). It applies identically to the hard-finalize `CommitTextReplacingPreedit` document write (Model B finalizes the same combined string).

**Per-segment case (v3.5.8, 2A).** A continuous candidate's presentation `roman` is cased to mirror the user's *raw input for that candidate's own byte span* (`Hittui` → `Hit` then `tui`), engine-side at candidate construction ([`dispatch::recase_roman`](../../engine/composing/src/dispatch.rs); span-local by `consumed_span`, slot-0 per-edge via `shadow_to_raw_end`). The legacy platform `SuggestionCaseTransformer` is **bypassed for continuous candidates** (its global-caps + typed-prefix model is invalid under Model B), so the engine is the single casing source. Canonical sidechannel (`display_text` → `user_frequency.db`/NextWord key) and `hanji` are untouched.

**Precise pending-tail `rawInput` definition** (the tail component of the surface above; option (c) of the three considered, amended 2026-05-13 to match actual engine behavior):

- Source: [`Phase::raw_input(&self, &AppConfig)`](../../engine/composing/src/api.rs) — delegates to [`derived::derived_display`](../../engine/composing/src/derived.rs) which runs the same POJ doubletap → tone-mark → nasal-case chain that builds `Preedit.display_text` today. Operates on `Phase::Continuous.raw` (the still-editable pending tail), not the nailed prefix.
- Transformations applied: tone-marker rendering, NFC normalization, POJ doubletap pre-processing, nasal-marker case adjustment.
- Transformations **NOT** applied: word-boundary inference (no spaces), candidate matching, ranking, **engine-driven syllable segmentation** (user-typed `-` is the only syllable boundary signal).

Rationale for (c) over (a) raw keystrokes / (b) syllabified-without-normalization: keystrokes (`goa2 ai3 li2`) are not human-readable in the host app; normalization is the minimum to make the composing buffer faithfully echo "what the user typed in displayable form" without inferring word groupings.

**Engine-driven auto-hyphenation is out of scope for v3.5.8** (Codex pre-impl consult 2026-05-13, Item 2). `Intent::AppendHyphen` (api.rs) is evidence the keyboard treats `-` as a user-typed character; `phonetics::api::to_tone_marks` splits on `-` but does not insert hyphens. If a user types `goa2ai3li2` with no hyphens, `rawInput` returns `goa2ai3li2` verbatim — no tone marks, no auto-segmentation. Adding syllabifier-driven hyphen insertion would be a follow-up enhancement (likely paired with §10.2 segmented dual-line rendering in Item 6) rather than part of the Item 2 contract.

**Scope of the `rawInput` rule** (Codex post-impl review 2026-05-13): the contract above governs **only the inline composing buffer** in the host app. Candidate strip rendering (slots 0..N) draws from dictionary records via the segmenter and ranker; the roman/hanji pair on a candidate cell is unaffected by whether the user typed hyphens in their raw input. See [`continuous-candidate-display.md`](continuous-candidate-display.md) §4 (dual-line carrier) and §15 (fallback retire) for the candidate-rendering path.

**Segmented version rendering rule (Codex co-review clarification γ, 2026-05-13)**

When `candidate[0]` is dual-line (HANT / MIXED — proto carries both `roman` and `hanji` per §4 of [`continuous-candidate-display.md`](continuous-candidate-display.md)):

| Line | Content | Word-boundary spaces? |
|---|---|---|
| Roman line (`CandidateMessage.roman`) | Segmented romanization | **YES** — visible spaces between word groups, e.g., `goa ai-li` |
| Hanji line (`CandidateMessage.hanji`) | Hanzi vocabulary tokens, **rendered as-is** | **NO** — e.g., `我愛你`, not `我 愛你` |

Rationale: Roman/Latin script tokenization conventionally uses spaces; hanji as a logographic script does not. Inserting visible spaces in the hanji line breaks convention and risks character-rendering oddities. Word segmentation is implicit in the hanji line through the candidate's `consumed_span` and `spanUnits` metadata, not visual whitespace.

When `candidate[0]` is single-line (TAILO — roman only, no hanji): roman line shows segmented romanization with word spaces; no hanji line exists.

### 10.3 Commit Contract

Under **Model B** there is exactly **one** literal document write per continuous session — at a hard finalize. Mid-composition candidate taps *nail* segments **inside** the marked region (no document write).

| Trigger | Effect |
|---|---|
| **Tap candidate, pending tail remains** (mid-commit / *nail*) | **No document write.** The candidate's swap/TPS/both-scripts-formatted string (exactly what the legacy lexicon path would commit — clarification γ) becomes the new segment's `NailedSegment.display_text` and is appended to the marked region; the canonical key (`hanji.unwrap_or(roman)`) is carried on `CommitContinuous.canonical_text` and fires `NextWordUpdateLastSelectedWord` (learning at nail time). The host sees one re-rendered `UpdatePreedit` of the **whole composition** — never `CommitTextReplacingPreedit`. |
| **Tap candidate, consumes the rest** (final-commit) | One `CommitTextReplacingPreedit` of the **whole composition** = `Σ nailed[i].display_text` (including this final segment), then exit. Single terminal `NextWordWordSelected(canonical, raw_text, true)` for the final segment (earlier nails already fired `NextWordUpdateLastSelectedWord`; not replayed). |
| **Enter** | One `CommitTextReplacingPreedit` of the **whole composition** = `Σ nailed[i].display_text` + the derived display of the pending raw tail (`Phase::composing_display`). This is exactly what the user saw inline — the entire marked region becomes literal text. Terminal `NextWordWordSelected` for the last "word": the pending tail when one exists, else the last nailed segment (canonical key). |

**Clarification β — REWRITTEN (Model B, 2026-05-16): Enter commits the whole composition, not the pending tail**

> The pre-2026-05-16 β said Enter commits only the pending tail because
> nailed segments were already literal document text. **Under Model B that
> premise is false**: nailed segments are inside the marked region, never in
> the document. Enter therefore commits `Σ nailed[i].display_text +
> derived(pending tail)` — the whole `Phase::composing_display`. "What you
> see is what you typed" still holds, because the marked region *is* the
> whole composition (§10.2). There is no `CommitRaw`-commits-literal-
> keystrokes gap anymore: `commit_raw_continuous` builds the combined string.

**Clarification γ — Tap formats the segment as the swap-aware string; `canonical_text` is the canonical key (v3.5.8 Phase 9 Bug 1; Model-B-adjusted)**

The roman-with-spaces rendering in slot 0 (§10.2 segmented rule) is **display-only**. On tap, the platform formats the candidate's `roman` / `hanji` through the **same swap/TPS/both-scripts formatter the legacy lexicon path uses**; that formatted string becomes the segment's `NailedSegment.display_text` and (Model B) joins the marked region immediately and the single combined `CommitTextReplacingPreedit` at hard finalize — so Continuous and lexicon produce identical document text for the same candidate under the same settings. The canonical key (`hanji.unwrap_or(roman)`) is sent **separately** on `CommitContinuous.canonical_text` and used only for the NextWord/frequency effects. This preserves:

- Frequency-recording keys (platform records on the canonical sidechannel — `ActionHandler.handleSuggestionSelection` continuous branch / `CandidateClickHandler.handleContinuousCandidateClick`)
- NextWord association keys (engine routes `canonical_text` to `NextWordWordSelected` / `NextWordUpdateLastSelectedWord`, including the backspace **unnail** correction — §10.7)
- Canonical word boundary of dictionary vocabulary tokens
- Mode-independent learning: frequency / NextWord do not fork by display mode

Empty `canonical_text` (legacy callers) falls back to `display_text` — unchanged.

Design intent:

- A tap = accept the engine's segmentation for that span; the segment is *nailed* into the composition (visible, editable via backspace-unnail) but **not** committed to the document until the user finishes.
- Enter / final-commit = the single hard finalize; the whole composition becomes literal text in one write. The underline disappears **only** here — matching every mainstream IME (§10.1.1) and the maintainer requirement.

**Commit side effects (Codex B1 item 4; Bug 1 2026-05-15; Model B 2026-05-16)** — frequency recording (`user_frequency.db`) and NextWord are **payload-orthogonal** to the document write and key off `canonical_text`. They fire **at nail time** (`NextWordUpdateLastSelectedWord` per nailed segment) and once at hard finalize (`NextWordWordSelected`), independent of when/whether literal text is written. No double-count: already-nailed segments are not replayed at final commit (Codex risk (i)).

### 10.4 Data-Flow Invariant

```
keystrokes
  → syllabifier (Rust)
    → Phase::Continuous { nailed[], raw }
        composing_display = Σ nailed[i].display_text + derived(raw)
          ───────────────────────────────► ONE marked / composing region
                                            ▲ hard finalize (Enter / final-
                                              commit / external) commits THIS
                                              whole string, once
      → segmenter + ranker (Rust)
        → candidates[]
            [0]   ─────────────────────────► strip slot 0 (display)  — tap nails / finalizes
            [N≥1] ─────────────────────────► strip slot N (display)  — tap nails / finalizes
```

Invariants (every edge case in §10.7 falls out from these — no per-case branching):

| ID | Invariant |
|---|---|
| **I1** | Composing-buffer content = `Phase::composing_display` = `Σ nailed[i].display_text` (verbatim, formatted at nail time) **followed by** the derived display of `Phase::Continuous.raw` (NFC + tone-mark across user-typed `-`; unhyphenated passes through verbatim; no engine syllabification of the tail). Nailed segments are part of this single marked region, **not** the host document. `raw` is the still-editable pending tail (bytes after the last nail), not the original keystroke history. Backspace, append, nail, and unnail all mutate `Phase::Continuous`; I1 re-establishes the whole combined string. |
| **I2** | `candidate[0]` display content = ranker top output rendered with segmentation. (Unchanged.) |
| **I3** | Any mutation of input (insert / backspace / nail / unnail) re-runs syllabifier → segmenter → ranker for the pending tail and re-renders the combined composition; nailed segments are preserved or unnailed per the §10.7 boundary rules. I1 and I2 re-establish automatically. |
| **I4** | Platform performs no re-ranking, no candidate-order rewriting, no exact-match injection — preserves G3 (§7.1). Output-mode formatting of the *chosen* candidate at nail/commit time (swap / TPS / both-scripts → the segment's `display_text`, canonical key on `canonical_text`) is **not** a violation — it formats one chosen candidate exactly as the legacy lexicon path does (clarification γ). **Additionally (Model B): a platform MUST NOT split a mid-composition candidate tap into `commitText(prefix)` / `insertText(prefix)` + a new preedit.** A nailed prefix is rendered inside the single marked region via one `UpdatePreedit`; literal document text is written only by the engine's single hard-finalize `CommitTextReplacingPreedit`. This is the invariant whose violation caused the iOS tail-leak (§10.6). |

### 10.5 Mode Gating

| Mode | Display split active? | Slot-0 = composing? |
|---|---|---|
| Continuous input (Phase 9 path, `fetch_via_continuous`) | **YES** | No — slot 0 differs from composing whenever ≥ 2 syllables |
| Non-Continuous lexicon (legacy `fetch_via_lexicon`) | **NO** | Yes — current behavior preserved |

The split is bound to the engine-side dispatch branch in [`engine/composing/src/dispatch.rs`](../../engine/composing/src/dispatch.rs) — there is no separate platform-side toggle.

### 10.6 Cross-Platform Touch Points

| Layer | iOS | Android |
|---|---|---|
| Composing-buffer write | `KeyboardInputViewController` → `markedText` | `InputConnection.setComposingText` |
| Slot-0 render | `CandidateButtonView.swift` — **remove dashed border** | `CandidatesView` / `CandidateButtonView.kt` — **remove `composingDashedBorder`** |
| Enter commit dispatch | `ActionHandler+Suggestions.swift` | `CandidateClickHandler.kt` (or IME keyboard view) |
| Tap-0 / Tap-N dispatch | `ActionHandler+Suggestions.swift` | `CandidateClickHandler.kt` |
| Swap/TPS/both-scripts segment formatting (γ, Bug 1) | `parseRomanAndHanzi` + `formatOutputText` (shared with legacy branch) → `commitContinuous(displayText:canonicalText:)` | legacy `bracketRoman` + when-expr → `commitContinuous(displayText, canonicalText, …)` |
| Canonical key wire | `CommitContinuous.canonical_text` (= sidechannel `displayText`) | `CommitContinuous.canonical_text` (= `TaigiWord.MetadataKeys.DISPLAY_TEXT`) |
| Mid-commit (nail) | One `UpdatePreedit(whole composition)` → `setMarkedText(combined, caret=end)`. **No `insertText`.** | One `UpdatePreedit(whole composition)` → `setComposingText(combined, 1)`. **No `commitText`.** |
| Hard finalize (Enter / final-commit / external) | One `CommitTextReplacingPreedit(whole composition)` → `clearMarkedText()` + `insertText` | One `CommitTextReplacingPreedit(whole composition)` → `commitText(combined, 1)` |

`rules/cross-platform-alignment.md` §3a applies: I1–I4 hold identically on both platforms.

**Convergence — Model B removes the former Bug-3 divergence (2026-05-16).** Under the pre-2026-05-16 model a mid-commit emitted `commit_text_replacing_preedit(segment)` + `update_preedit(tail)`; iOS hosts confirmed the small re-marked tail into literal text during their `textWillChange→textDidChange` settle (real-device trace), losing the underline mid-composition. iOS carried an arm/detect/compensate workaround that hit a 3-strike circuit-breaker. **Model B eliminates this by construction**: a mid-commit emits **no** `CommitTextReplacingPreedit` — only one `UpdatePreedit` of the whole composition — so there is no sub-region for the host to confirm. The iOS `armContinuousMidCommitTail` / `detectContinuousMidCommitTailLeak` / `compensateLeakedContinuousMidCommitTail` layer is **deleted** (P2), not kept. There is no longer any platform-specific divergence here; both platforms run the identical effect sequence and `behavioral-invariants.md` needs no entry.

**External-composing-region clear (field/app switch, host clears mid-composition).** Policy (single, documented per Codex risk (iii)): treat it as a **hard abort** — drop all nailed + pending, exit Idle, no document write (mirrors `reset_continuous`). Not a compensation attempt. iOS field-switch path + Android `onUpdateSelection → onExternalComposingRegionCleared` both route to this abort.

**Engine-side coupling — resolved**: the `subtitle=nil` symptom was closed by fix-plan Item 5/6 (`CandidateMessage` `roman` + optional `hanji`; dual-line render). v3.5.8 Phase 9 Bug 1 added `CommitContinuous.canonical_text` so the segment's formatted display and the canonical freq/NextWord key are decoupled on the wire. §10.2/10.3/10.4 hold under Model B with no proto change.

### 10.7 Edge Cases (fall out from §10.4 invariants)

| Case | Behavior | Why |
|---|---|---|
| Single syllable | composing == `candidate[0]` | Segmenter inserts zero word boundaries → identity |
| Continuous disabled | Split inactive | §10.5 mode gating |
| Backspace, pending tail non-empty | Last char of pending tail drops; the combined composition (`Σ nailed.display_text` + shrunk tail) re-renders; `candidate[0]` recomputes. **No** `DeleteBackwardFromDocument` (nothing is in the document). | I3 |
| Backspace, pending tail empty, ≥1 nailed (**unnail**) | The last nailed segment is popped; its `raw_text` becomes the new pending tail; combined composition re-renders. NextWord last-selected rolls back to the prior nailed segment (or clears). **No** `DeleteBackwardFromDocument` — authority is `raw_text`, never a display-char count (swap/TPS/both-scripts display can desync from raw). | I3 + Codex risk (v) |
| Empty buffer | No composing display, no candidates | Pre-Phase-9 behavior; unchanged |
| Syllabifier partial-parse failure | `rawInput` shows partial parse + raw tail; candidates may be empty | Existing syllabifier error path; this section does not modify it |
| **Partial prefix below first syllable ending** (e.g., `raw = "gu"`, no completed syllable) | Composing buffer shows `gu` literally. Slot 0 is **not** the "segmented version" rule:<br/>• **Pre-§15.3.D state** (current code, before fix-plan item 10): strip is empty.<br/>• **Post-§15.3.D state** (after fix-plan item 10 lands): strip shows engine-prefix candidates per `continuous-candidate-display.md` §15.3.D, ranked below full-syllable candidates via `coverage_kind` (§15.5).<br/>In both states, Enter commits literal `gu`. | Segmenter has no word boundaries to insert when `raw` is sub-syllable; the "candidate[0] = segmented version" rule is **undefined** below first valid syllable ending. Codex co-review clarification (B3, 2026-05-13). |
| **Enter after segments already nailed** | Commits the **whole composition** = `Σ nailed[i].display_text` + derived(pending tail), in one `CommitTextReplacingPreedit`. The marked region (which *was* the whole composition) becomes literal text and the underline clears. | Clarification β REWRITTEN (Model B): nailed segments were in the marked region, not the document. |
| **Abort / reset mid-composition** | The whole marked region is cleared and all nailed segments dropped; nothing reaches the document (mirrors `reset_continuous`). Not "keep nailed, drop pending". | Codex risk (ii): nailed were never in the document. |
| **Hanji accidentally in composing buffer** | Composing shows the hanji literals; engine returns no candidates (per §15.3.E of `continuous-candidate-display.md`); Enter commits the hanji literals (as the pending tail of the combined string). | Engine hanzi guard short-circuits to empty `ContinuousResponse`; composing surface unaffected. Fix-plan item 11. |

### 10.8 Regression Test Hooks

Minimum coverage to declare §10 closed (Model B acceptance criterion):

1. **Single syllable** — type `goa` → assert composing == `candidate[0].display_text`.
2. **Three-syllable two-word** — `candidate[0]` roman line shows word-boundary spaces, hanji line shows the hanzi **without** added spaces (clarification γ; unchanged).
3. **Tone-marker spanning syllables** — NFC + tone rendering apply to the pending-tail component without affecting segmentation.
4. **Mid-commit emits no document write** — type ≥2 syllables, tap `candidate[0]` for a prefix (pending tail remains) → assert effects are `[UpdatePreedit(combined), NextWordUpdateLastSelectedWord, PerformAutocomplete]`; assert **no `CommitTextReplacingPreedit`**; assert composing buffer == `Σ nailed.display_text + derived(pending)` (still one marked region / underlined).
5. **Primary Bug-3 acceptance** — type `taiuantaigi`, tap 「臺灣」 (nails `taiuan`, `taigi` pending) → composing shows `臺灣` + derived(`taigi`), **still underlined**, no `CommitTextReplacingPreedit`. Then Enter → one `CommitTextReplacingPreedit("臺灣" + derived("taigi"))`, exit Idle, underline clears **only here**. (Never commits just `taigi`.)
6. **Final-commit commits whole composition** — nail "tsu"→珠 (pending "a"), then tap final candidate consuming "a"→仔 → one `CommitTextReplacingPreedit("珠仔")` (not "仔"), exit; single terminal `NextWordWordSelected`.
7. **Backspace never writes/deletes document** — across pending-shrink, unnail (pop), and empty-out: assert **no `DeleteBackwardFromDocument`** is ever emitted in Continuous; unnail restores the popped segment's `raw_text` as pending and re-renders combined.
8. **Abort / reset clears whole region** — with ≥1 nailed segment, `Reset` / `ResetContinuous` → `[ClearPreeditWithoutCommit, …]`, exit Idle, **no `CommitTextReplacingPreedit`**, all nailed dropped.
9. **`select_suggestion` / `commit_preedit_then_insert_external` under Continuous** — committed string = `Σ nailed.display_text` + (text | derived(raw)+external); with zero nailed, unchanged from legacy.
10. **No-nailed parity** — pure Composing→EnterContinuous→Enter (no tap) commits `derived(raw)` exactly as before (combined == derived(raw) when nailed empty); translate-swapped / TPS pass-through unchanged.
11. **Hanji line no added spaces** — HANT/MIXED multi-word `candidate[0]`: roman line has word spaces, hanji line is the exact dictionary hanzi (clarification γ; unchanged).

Cross-platform: every case must pass identically on iOS and Android per `rules/cross-platform-alignment.md` — including the I4 "no `commitText(prefix)` / `insertText(prefix)` mid-composition" assertion.

### 10.9 Relationship to §1–§9

| § | Connection |
|---|---|
| §1–§3 (ranking gap) | Orthogonal. Ranking determines candidate **order**; §10 determines **display and commit**. Both ship in Phase 9. |
| §4 (Codex co-confirm) | Separate co-confirm pass required for §10 after quota recovery. |
| §5 (mainstream IME) | §10.2–§10.4 is **Model B**, the unanimous pattern across librime / khiin-rs / MOE / azooKey — see §10.1.1 for the four-IME cite-and-trace. Our former eager partial-literal-commit had zero precedent and caused the iOS leak; Model B closes that parity gap and the bug together. |
| §6 (architectural classification) | §10 adds no new Gap; it is a normative UI/IME contract. |
| §7 (long-term goals) | Reinforces G3 (engine is ranking authority). Compatible with G1/G2/G4 trajectory. |
| §8 (v3.5.8 decision) | §10 is part of the expanded Phase 9 scope per the 2026-05-11 revision. |
| §9 (open questions) | Adds implicit Q8: should §10's display split extend to non-Continuous mode if Continuous becomes default in a future release? Currently NO per §10.5; revisit when default flips. |

### 10.10 Codex Co-Review Log (2026-05-13)

Codex spec co-review pass against §10 + [`continuous-candidate-display.md`](continuous-candidate-display.md) (transcript: `/tmp/v358-codex-review-out.txt`). Outputs synthesized into §10 inline:

| Codex finding | Landed in |
|---|---|
| A1: §10.2/10.3/10.4 directionally consistent; no-candidate boundary unpinned | §10.7 partial-prefix row + hanji-in-buffer row |
| A2/B2: §10 vs §4.6/§15.4 cross-spec contradiction on slot-0 model | §10.1.2 Supersedes notice + reciprocal notes in `continuous-candidate-display.md` |
| A3: segmentation target ambiguous between `display_text` / roman / hanji | §10.2 segmented-rendering rule + §10.3 clarification γ |
| A4 + B3: Enter-after-nail / partial-prefix semantics | §10.3 clarification β + §10.7 new rows |
| C1: MOE Enter evidence weaker than spec claims | §10.1.1 clarification δ |
| D fix plan (14 items, ordered) | **ALL 14 items shipped.** Items 1–6 + 14 = spec foundation + dual-line carrier; Items 7–12 closed every engine syllabification gap; **Item 13 (v3.5.8 capstone) retired the platform lexicon fallback** so the Continuous engine is the single candidate source. v3.5.8 feature-complete after Item 13. |

**Clarifications recorded inline** (β/δ superseded by the Model B rewrite, 2026-05-16 — see §10.10a):
- **α (slot-0 supersedes)** → §10.1.2 (still valid)
- **β** — REWRITTEN: Enter commits the **whole composition** (`Phase::composing_display`), not the pending tail → §10.3 + §10.4 I1 + §10.7
- **γ (tap formats the segment as the swap-aware string; `canonical_text` is the canonical key, `display_text` is the formatted marked-region/document string)** → §10.2 + §10.3 (Model-B-adjusted: the formatted string is the segment's display inside the marked region, joined into the single hard-finalize commit; the canonical key rides `CommitContinuous.canonical_text`)
- **δ** — SUPERSEDED: the "MOE Enter→raw" black-box claim is no longer load-bearing; Model B rests on the four-IME source cite-and-trace (§10.1.1), not on MOE Enter behavior

### 10.10a Model B rewrite (2026-05-16, Bug 3 closeout)

Codex design co-review PASS (auto-mode batched consult). The pre-2026-05-16 model (nailed = literal document text; composing buffer = pending tail; mid-commit emits `CommitTextReplacingPreedit`) was the root cause of the iOS mid-commit tail-leak (former Bug 3) and had no mainstream precedent. Model B (whole composition in one marked region; single literal write at hard finalize) is mainstream-unanimous and removes the bug by construction. Codex-surfaced design points landed: (i) per-segment learning fires at nail, single terminal `NextWordWordSelected`, no replay; (ii) `reset_continuous` clears the whole region + drops all nailed; (iii) external-region-clear = single hard-abort policy (§10.6); (iv) caret at end of combined marked text; (v) backspace unnails via `raw_text`, never display-char count, never `DeleteBackwardFromDocument`; (vi) Model B eases the future lattice/walker (§ "整句 lattice + walker" — orthogonal). Engine landed in P1 (`engine/composing/`); iOS converges + deletes the Bug-3 workaround in P2; Android converges in P3. The former §10.6 platform divergence is **deleted** (convergence).

Fix-plan ordering **(historical log — the Item 3 Enter/commit semantics below are SUPERSEDED by §10.10a Model B 2026-05-16: `Intent::CommitRaw` in `Phase::Continuous` now commits `Phase::composing_display` (the whole composition), NOT `derived_display(pending)`; mid-commit emits no `CommitTextReplacingPreedit`)**: item 14 (visual unification) shipped 2026-05-13. Item 1 (doc reconcile) + Item 2 (`Phase::raw_input` accessor + invariant tests + §10.2 amendment for actual engine behavior) shipped on branch `v358-continuous-display-spec`. Item 3 (`Intent::CommitRaw` in `Phase::Continuous` commits `derived_display(pending)`, mid-commit preserves nailed segments) shipped 2026-05-14 **(Enter/commit semantics later superseded by Model B — see §10.10a)**. Item 4 (Tap-0/Tap-N commit semantics — slot-0 cell removed from Continuous path, strict-required `displayText` sidechannel) shipped 2026-05-14. Item 5 (`CandidateMessage` `roman` + `hanji` proto fields + bridge decode + defensive `roman.isEmpty → displayText` fallback) shipped 2026-05-14. Item 6 (pin segmented dual-line rendering at platform UI — iOS `buildContinuousSuggestions` + Android `buildContinuousSuggestionsForCandidates` swap `text/title` ← `c.roman`, `subtitle/hanzi` ← `c.hanji`, present-empty hanji collapses to nil) shipped in PR #271. Multi-word chain segmentation (§10.7 #2 case) is **out of scope** — each `RawCandidate` today maps to exactly one dictionary record, so chain segmentation requires a future lattice/chain-producer slice. Items 7–12 (engine syllabification gaps) shipped PR #272–#278; **Item 13 (capstone) retired the platform lexicon fallback** (`continuous-candidate-display.md` §15.4) — v3.5.8 feature-complete.

`(roman, hanji)` ranker dedupe is **deferred to Item 12** (custom-dict integration) — the first slice that can emit cross-source duplicates. Today the default `dict.bin` builder collapses duplicates via `dictionary/build/merge_csv.py`'s `groupby(["hanzi", "_tl_key"])`, so no realistic input surfaces a duplicate `(roman, hanji)` pair into the Continuous ranker. Locking the winner policy (lowest `source_tier_rank` vs SortKey winner) without real custom-dict plumb context would be premature; Item 12 picks it then.

Durable re-grounding of the fix plan in a future session: re-run the Codex consult (`/tmp/v358-spec-review.txt` prompt) against the latest spec. Plan does not need to live in a separate doc — the spec itself now carries enough structure for an implementer to plan from.

---

## 11. Cross-references

| Reference | Section / Key |
|---|---|
| `docs/roadmap.md` | Active item v3.5.8 § Phase 5 deferred / § Phase 6 limitations / § Phase 9 |
| `docs/architecture/behavioral-invariants.md` | §11 live-read settings (relevant for Q2 above) |
| `rules/cross-platform-alignment.md` | §3a CROSS-PLATFORM INVARIANT (relevant if ranking constants get tuned per platform — they must not) |
| `rules/rust-best-practices.md` | §3a domain↔proto boundary (relevant if `FetchAtPos` proto evolves) |
| `feedback_no_future_planning.md` | Why §8/§9 here record the limitation but do not schedule a fix |
| `feedback_no_slice_toggles.md` | Why no `useContinuousV2` toggle is proposed |
| `references/moe_taigi_apk/decompiled/sources/moe/taigi/TailoJNI.java` | MOE native ranking entry point |
| `references/moe_taigi_apk/decompiled/sources/android/moe/taiwanese/taigi/data/local/model/CandidateModel.java` | MOE per-candidate metadata schema |
| `references/khiin-rs/khiin/src/data/segmenter.rs:122` | Khiin segmentation cost formula |
| `/tmp/codex-v358-ranking-coconfirm.txt` | Codex co-confirmation transcript (transient) |
