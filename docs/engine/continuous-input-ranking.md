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
| Buffer ownership | C++ stateful `tutgInputLine` (opaque handle) | Rust `Phase::Continuous { raw, committed }` |
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
| Buffer manager | `BufferMgr` (`buffer_mgr.rs:44-66, 1096-1129`) — committed segments + raw_char_count (mirrored by our `Phase::Continuous { raw, committed }`). |
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
| G1 | Gap A (formula) — primary. Possibly Gap C (dict data) as enabler. |
| G2 | Gap B (boost wiring) — primary. |
| G3 | Already met by current architecture. Goal is to **preserve** through G1 + G2 changes. |
| G4 | Independent of A/B/C — proto schema extension. Tracked at §9 question #6. |
| G5 (stretch) | Independent of A/B/C — input-state-machine extension. Tracked at §9 question #7. |

A future plan that **closes G1 + G2 + G4** while preserving G3 is the success criterion for retiring this spec. G5 may close in the same plan or in a follow-up; it is the boundary between "ranking-aligned" and "full-IME-aligned."

---

## 8. Decision for v3.5.8

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

## 10. Cross-references

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
