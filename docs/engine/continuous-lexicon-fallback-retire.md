# Continuous Mode — Architectural Extension: Eliminate Lexicon Fallback

> Added 2026-05-11 (night). Original [`continuous-candidate-display.md`](continuous-candidate-display.md) §1–§14 still holds for the display-fix subset; this file holds the architectural simplification user requested after reviewing the §3 "interleave" root-cause analysis.
> **Provenance**: extracted 2026-05-26 from `continuous-candidate-display.md` §15 as part of the P3 doc-size split. Section numbering (`§15.1`, `§15.4`, …) is **preserved** so existing references in code comments (Swift / Kotlin / proto), other docs, and historical archives continue to resolve.
> **Status**: Item 13 (capstone) shipped — engine is the single candidate source on both platforms. The §15.X subsections record the end state for archeology.

### 15.1 Motivation

User wording: 「fallback 是冗餘設計,讓邏輯複雜化,應像 MOE 那樣在 engine 內部就處理所有切音節邏輯」.

Three reinforcing reasons:

1. **Two-path complexity** is the root cause of dogfood Finding 2. Even after Option A (§4) ships dual-line cells on both paths, the platform still has `if continuous returns empty → fallback to lexicon` branching. The branching itself is what produces the temporal toggle described in §3.2. Eliminating it gives a **truly single-directional data flow**:

   ```
   user input → engine (handles all syllabification + lookup) → candidates → UI
   ```

   This is the MOE `tutgInputLine` model, captured in `references/moe_taigi_apk/decompiled/sources/moe/taigi/Tailo.java:77` — there is no second `GetCandidatesAtBackupPath` API.

2. **Engine is already authoritative on Composing state** ([`engine/composing/src/api.rs`](../../engine/composing/src/api.rs) Phase enum + Phase 4 auto-promote contract). The platform calling `lexicon::search` directly from `autocomplete()` bypasses Composing state entirely — a layering violation that worsens as Phase::Continuous coverage broadens.

3. **MOE parity audit**:
   - Engine-side carrier already has both fields ([`CandidateModel.java`](../../references/moe_taigi_apk/decompiled/sources/android/moe/taiwanese/taigi/data/local/model/CandidateModel.java) `tailos: List<String>` + `vocabulary: String`)
   - MOE's keyboard-level `HanjiMode` / `TailoMode` filter is a different UX axis from our `isTranslateSwapped` — **we deliberately keep dual-line** as the differentiator (§11 cite + user wording 「雙行是我比 MOE 做到更優秀的部分」)
   - But MOE's engine-as-single-source architecture aligns with what we're moving toward

### 15.2 Scope clarification — what stays platform-side

Per Q1 clarification 2026-05-11, **the only input modes are TL / POJ / TPS romanization** + composing-text slot-0. There is **no hanzi input mode**. A hanzi (CJK) composing buffer must never produce keyboard candidates — historically a **defensive D-8 guard** in the platform `LexiconService.search`. **DONE (Item 11 + Item 13)**: the guard now lives entirely in the engine (`handle_fetch_at_pos` `is_hanzi(raw)` → empty carrier, §15.3.E); the platform `LexiconService.search` D-8 guard + its parity tests were deleted with the lexicon-fallback retire (§15.4). See `docs/architecture/behavioral-invariants.md` §14 (re-pointed engine-ward).

### 15.3 Engine coverage gaps to close

| # | Gap | Current state | Engine work to do |
|---|---|---|---|
| **A** | TPS tone-1 untoned (`ㄉㄞ`) | ~~`tps::valid_span_endings` only sees tone-mark / 入聲韻尾 terminators~~ | **DONE — Phase 9 Item 7 / 9.4a**: `tps::valid_span_endings` next-initial-seen + trailing-tone-1 rule; `phonetics::is_tps_initial` / `is_tps_char`; `build_keys_tps` accepts digitless toneless fragment |
| **B** | POJ-diacritic input (`pe̍h`, `chóa`, `peⁿ`) | ~~`to_ascii_lowercase` in `build_keys_tl` doesn't strip diacritics~~ | **DONE — Item 9**: `dispatch.rs::canonicalize_poj_shadow` per-syllable canonicalize + offset map composed with the Item 8 hyphen shadow |
| **C** | Hyphenated TL (`tai-bak`, `pe̍h-ōe-jī`) | ~~`tl_syll::valid_span_endings` BFS, no hyphen inventory entries~~ | **DONE — Item 8**: `dispatch.rs::build_hyphen_shadow` shadow buffer + byte-offset map |
| **D** | Partial prefix (`t`, `gu`, anything shorter than first valid syllable ending) | ~~Syllabifier returns `{}` → continuous empty → fallback to lexicon prefix-match~~ | **DONE — Item 10**: `handle_fetch_at_pos` falls through to `fetch_partial_prefix_candidates` (`prefix_index.lookup_prefix`) with the `coverage_kind` SortKey dim (§15.5) ranking partial below full-syllable |
| **E** | Hanzi accidentally in composing buffer | ~~Platform D-8 guard in `LexiconService` returns `[]`~~ | **DONE — Item 11 + Item 13**: `handle_fetch_at_pos` checks `is_hanzi(raw)` at top of dispatch → empty `ContinuousResponse`; the platform `LexiconService.search` D-8 guard was deleted with the §15.4 retire |

All engine coverage gaps closed by Items 7–12; **Item 13 (capstone) then retired the platform lexicon fallback** so the engine is the single candidate source.

### 15.4 Platform simplification — DONE (Item 13)

After §15.3 landed, both platforms collapsed to a single path:

```swift
// iOS — TaigiAutocompleteService.swift autocomplete(_:)
func autocomplete(_ text: String) async throws -> Autocomplete.Result {
    guard !text.isEmpty, let composing = activeComposingContext() else {
        return Autocomplete.Result(inputText: text, suggestions: [])
    }
    let candidates = continuousFetcher?.fetchContinuousCandidates() ?? []
    let suggestions = buildContinuousSuggestions(from: candidates)
    return Autocomplete.Result(inputText: text, suggestions: suggestions)
}
```

```kotlin
// Android — TaigiAutocompleteService.kt autocomplete(...)
suspend fun autocomplete(rawInput: String, displayText: String, ...): List<TaigiWord> {
    if (rawInput.isEmpty() || displayText.isEmpty()) return emptyList()
    val candidates = continuousFetcher()
    return buildContinuousSuggestionsForCandidates(candidates)
}
```

Empty-engine state: per §15.6 acceptance criteria, the Continuous strip is empty (`buildContinuousSuggestions` returns `[]`); the inline pre-edit retains the composing buffer, and Enter still commits the raw tail via Item 3's `Phase::Continuous` `Intent::CommitRaw` arm. The pre-§10 "always insert slot-0 composing-text cell" affordance was retired in Item 4 (`docs/engine/continuous-input-ranking.md` §10.1.2 supersedes notice).

What was deleted — **asymmetric** because iOS Tab3 was already decoupled to a separate `DictionarySearchService` while Android Tab3 shares the `LexiconService` class:

| Platform | Deleted | Notes |
|---|---|---|
| iOS | **Whole** `LexiconService.swift` (~273 LOC) + `CompositionRoot.lexiconService` | iOS `LexiconService` was 100% autocomplete-owned (verified: sole consumer was `AutocompleteService`). iOS Tab3 uses `DictionarySearchService` → bridge `lexiconSearchByHanzi/WithSources`, NOT `LexiconService`. |
| iOS | `AutocompleteInputClassifier.swift` (~22 LOC); `AutocompleteService` lexicon path (`searchLexicon` / `applyContextBoost` / `remapBoostedWords` / `convertToSuggestions` / `buildSuggestions` / `createComposingTextSuggestion`) + ctor/provider slim (`lexiconService` / `nextWordService` / `settingsProvider` / `selectionContext` / `setSelectionContextProvider`) | `autocomplete(_:)` collapsed to engine-only; no `await` remains so the stale-result guard was dropped. |
| iOS | `LexiconServiceHanziGuardTests.swift` (~57 LOC) | D-8 guard now engine-owned (Item 11) — see `behavioral-invariants.md` §14. |
| Android | `LexiconService.kt` `search()` + `lookupCustomDictionary` / `querySystemDictionaries` / `processCandidates` + ctor slim (`customDict` / `userFreq`) | **Class kept** — Tab3 (`DictionarySearchViewModel`) uses `searchWithSources` / `searchByHanzi` on the same class, so only the autocomplete-only method surface was removed. |
| Android | `AutocompleteInputClassifier.kt` (~18 LOC); `TaigiAutocompleteService.kt` lexicon path (`applyContextBoost` / `remapBoostedWords` / `createComposingTextCell`) + ctor slim (mode-agnostic; `lexicon` / `nextWord` / `settings` dropped); `CandidateUpdateCoordinator` mode-cache + dead-arg cleanup | — |
| Android | `LexiconServiceHanziGuardTest.kt` (~28 LOC) | Same engine-owned guard rationale. |

iOS = whole-file deletion; Android = method-level deletion within a kept class. Same intended behavior (engine single source); different impl surface. (Earlier drafts of this section estimated ~120 LOC iOS + "LexiconService stays for Tab3" — that was Android-centric and **wrong for iOS**, corrected here from grounded code.)

Empty-engine state: per §15.6, the strip is empty; the inline pre-edit retains the composing buffer and Enter commits the raw tail via Item 3's `Phase::Continuous` `Intent::CommitRaw` arm.

`LexiconService` as a class **survives only on Android** (Tab3 `searchWithSources` / `searchByHanzi`). On iOS the class is gone entirely; Tab3 is served by the orthogonal `DictionarySearchService`. iOS file deletions require a manual Xcode project (`pbxproj`) update by the maintainer (`feedback_xcode_manual` / `feedback_pbxproj_sync_gap`).

### 15.5 Ranking adjustment for partial-prefix candidates

Partial-prefix candidates (§15.3.D, `consumed_span = (0, raw.len())` where raw covers no complete syllable) must **rank below** full-syllable candidates so a user typing `gu` still sees regular `gua` / `guá` lexicon hits but ranked under any actual phrase match if one exists.

**Proposed `SortKey` extension**: prepend a new `coverage_kind` dimension (`u8`) ahead of the existing `tier`:

| `coverage_kind` | Meaning |
|---|---|
| `0` | Full-syllable match (existing — `valid_span_endings` produced an ending) |
| `1` | Partial-prefix match (new — syllabifier returned empty, `prefix_index.lookup_prefix` produced hits) |

This pushes ALL partial candidates strictly below ALL full-syllable candidates regardless of frequency. Inside `coverage_kind == 1`, the `(tier, recency_rank, -score, -freq, -coverage_bytes, ...)` lexicographic policy applies; `tier` is always 0 here because partial-prefix candidates pin `consumed_span = (0, raw.len())`, so `consumed_span_end == raw_len` holds by construction.

> **v3.5.8 整句 lattice + walker S8 — `-coverage_bytes` demoted (dim 3 → dim 6).** Pre-S8 a graded longest-coverage-first rule sat directly above `-score`/`-freq` inside a tier (the original pre-walker Gap-A surfacing of phrases). With the slot-0 whole-sentence walker now owning phrase priority (§7.3 "G1 closed by S2"), that rule only buried the short single-syllable first-segment candidate the user wants for segment-by-segment selection (dogfood: typing `guaikingkahuekhoo`, 「我」/Guá ranked behind even 2-syllable candidates). `-coverage_bytes` is now a weak tiebreak below `-score`/`-freq` — it separates two candidates only when score AND freq are equal. This matches librime's per-segment menu (`references/librime/src/rime/gear/script_translator.cc` `kNumExactMatchOnTop`): keep multi-length candidates visible, but never let a longer code-length bury a shorter strict match. Engine-only, no wire/proto/Model-B change.

**Wire impact**: `coverage_kind` stays internal to `RawCandidate` (does NOT enter `CandidateMessage`); same pattern as `recency_rank` ([`continuous.rs:195-206`](../../engine/lexicon/src/continuous.rs)).

**SortKey insertion order**:

```rust
struct SortKey {
    coverage_kind: u8,             // Item 10 — 0 = full-syllable, 1 = partial-prefix
    tier: u8,                      // 0 = consumed_span_end == raw_len, else 1
    recency_rank: u8,              // 0 = recent, 1 = stale
    neg_score: Reverse<NonNanF32>, // freq × syll_bias × boost, desc
    neg_freq: Reverse<u32>,        // raw freq, desc
    neg_coverage: Reverse<u32>,    // S8 — DEMOTED here (was dim 3); weak tiebreak
    source_rank: u8,               // custom=0 … default=5
    stable_idx: u32,               // insertion order
}
```

(8 dimensions. PR-9.1 = 7; Item 10 prepended `coverage_kind`; S8 relocated `neg_coverage` from dim 3 to dim 6.)

### 15.6 Test matrix for fallback retire — DONE

| Test class | Location | What it pins |
|---|---|---|
| `partial_prefix_engine_path` | `engine/composing/tests/dispatch_continuous.rs` | `Phase::Continuous` + `raw = "gu"` (no valid ending) → `FetchAtPos` returns `prefix_index.lookup_prefix("tl:gu")` hits with `coverage_kind = 1` |
| `poj_diacritic_canonicalize` | same | `Phase::Continuous` + `raw = "pe̍h"` → canonicalized to `pek` → `tl:pek` key → expected hits |
| `hanzi_guard_in_engine` | `engine/composing/src/dispatch.rs` (mod test) | `raw = "我好"` (CJK chars) → `FetchAtPos` returns empty `ContinuousResponse` (carrier present, candidates empty) |
| `sort_key_partial_below_full` | `engine/lexicon/src/continuous.rs` (mod sort_key_tests) | Construct two `RawCandidate` — one with `coverage_kind = 0` lowest freq, one with `coverage_kind = 1` highest freq — assert full-syllable wins |
| `platform_autocomplete_no_lexicon_branch` | iOS `TaigiAutocompleteServiceContinuousTests.swift::testAutocomplete_EmptyEngine_NoLexiconBranch_EmptyResult` / Android `TaigiAutocompleteServiceTest.kt` | Mock `continuousFetcher` returning empty → `autocomplete` result is **empty `[]`** (no slot-0 composing cell, no lexicon path — the path no longer exists). Plus a positive control: non-empty engine result passes through 1:1 with `isContinuous` set and no `isComposingText`. |

### 15.7 Open Questions for Codex Pre-Impl Consult — RESOLVED

> **Resolved across Items 7–13** (each ran its own Codex pre-impl sandwich). Q15.1–Q15.5 settled in Items 10–11 (`coverage_kind` = internal `u8`; partial-prefix pass-through `syllable_count`; `tl:`-namespaced key; partial-prefix always final-commit; `MIN_PREFIX_LEN = 1`). **Q15.6 (Tab3 untouched)**: confirmed in Item 13 from grounded code — iOS Tab3 = `DictionarySearchService` (never used `LexiconService`); Android Tab3 = `LexiconService.searchWithSources`/`searchByHanzi` (kept; only the autocomplete `search()` surface was deleted). Q15.7 sub-PR plan superseded by the Item 7–13 numbering. Original text retained below for provenance.

Adds to §9 (historical — resolved as above):

**Q15.1 — `coverage_kind` enum or u8?**

Same dilemma as Phase 9.2 `CandidateMode` — does this graduate to a typed enum, or stay a u8 ordinal? Recommendation: **u8 internal-only** (not on wire, not user-facing) — keeps the `derive(Ord)` SortKey clean without an extra enum.

**Q15.2 — Partial-prefix `syllable_count` value?**

The current `syllable_count: u8` field in `RawCandidate` is dictionary-source (`DictionaryRecord::syllable_count`). For partial-prefix hits, the dictionary record's syllable_count still applies (`紙` = 1, `珠仔` = 2) — should it be passed through unchanged, or set to `0` to signal "partial"?

Recommendation: **pass through unchanged**. `coverage_kind` already discriminates partial vs full; `syllable_count` retains its dictionary meaning for downstream sort tie-breaks.

**Q15.3 — `prefix_index.lookup_prefix` key shape**

`lookup_prefix` returns all rowids matching a prefix. For partial `gu`, the key is `tl:gu` and matches all dictionary entries starting with `tl:gu` (gua, gun, guan, ...). But what about `gu1` (with tone digit)? Per the existing `build_keys_tl` digit-strip rule, `gu1` → `tl:gu` prefix. Same hits.

Edge case: user types `1` alone (degenerate). `build_keys_tl` strips → empty key → return empty (avoid scanning entire FST). Pin in test.

**Q15.4 — `consumed_span_end` for partial-prefix candidates on commit**

When user taps a partial-prefix candidate, commit semantics: consume the full raw buffer or just the prefix? Lexicon path's current behavior = consume full input as one block (no continuous segmentation). Partial-prefix in continuous: same? Or per-syllable consumption based on the candidate's `syllable_count`?

Recommendation: **consume full raw** (`consumed_span = (0, raw.len())`). This matches the user's mental model — "I started typing and tapped one of these suggestions before I finished a syllable" → commit what I typed + the candidate's display, no leftover pending.

But this conflicts with the multi-syllable phrase commit pattern (`taiuantaigi → tap 「臺」 → leftover uantaigi`). Codex consult: should partial-prefix never appear in `coverage_kind = 1` mode AND simultaneously enable mid-commit? Or always force final-commit for partial?

Initial proposal: **partial-prefix candidates always final-commit** (consume full buffer, exit to Idle). Multi-syllable mid-commit only applies to `coverage_kind = 0` full-syllable candidates. This keeps tap semantics predictable.

**Q15.5 — Engine prefix-fetch threshold**

Should engine ALWAYS try prefix-match when syllabifier returns empty, or only when `raw.len() >= MIN_PREFIX_LEN`?

- ALWAYS: even `t` returns prefix hits. Consistent, no surprise empty strips.
- Threshold (e.g., MIN = 2): single char returns empty (avoid noise from `t` matching thousands of entries).

Codex consult: pick threshold based on `prefix_index.lookup_prefix` performance characteristics + UX preference. Default proposal: **MIN_PREFIX_LEN = 1** (no threshold) — matches v3.5.7 lexicon behavior where single char also returned candidates.

**Q15.6 — Tab3 dictionary search untouched?**

Confirm `LexiconService.search` for Tab3 hanzi search stays exactly as is — no shared retire. Tab3 doesn't go through `Phase::Continuous` and has its own UX (search bar, browsing). Per existing `docs/architecture/codex-review-2026-04-19.md` Tab3 is fully separate.

**Q15.7 — Sub-PR breakdown**

Combined display fix + fallback retire is ~700-950 LOC. Per `~/.claude/rules/planning.md` § Persistent hand-off (200-500 LOC/PR discipline), suggest:

| Sub-PR | Scope | LOC est. |
|---|---|---|
| **Phase 9.4a** | TPS tone-1 syllabifier (original) | ~80 |
| **Phase 9.4b** | Hyphenated TL shadow buffer (original) | ~150 |
| **Phase 9.4c** (NEW) | POJ diacritic canonicalize in `build_keys_tl` | ~80 |
| **Phase 9.4d** (NEW) | Partial-prefix engine path + `coverage_kind` SortKey extension | ~250 |
| **Phase 9.4e** (NEW) | Hanzi guard port + Option A wire-level display dual-field carrier (combined — both touch `handle_fetch_at_pos` / `CandidateMessage`) | ~200 |
| **Phase 9.6** | Custom dict integration (original) | ~150 |
| **Phase 9.7** (NEW) | Platform `autocomplete()` simplification + lexicon-path retire + Finding 1 dashed border removal | ~80 deletion + UI |

Total: 7 sub-PRs. All v3.5.8. (Original 9.5 handled by user offline, not in PR plan.)

Codex confirms ordering / dependencies. Suggest dependency:
- 9.4a, 9.4b, 9.4c, 9.4d independent of each other (each engine-internal)
- 9.4e depends on 9.4a-d for `coverage_kind` enum to land first
- 9.7 depends on 9.4a-e all merged (platform retire safe only after engine completeness)
- 9.5 / 9.6 independent of fallback work

### 15.8 Risk register

> **Status (Item 13 shipped)**: partial-prefix flood + `coverage_kind` ranking risks closed by Items 10/11 engine tests. The **dogfood regression risk is the live capstone-acceptance gate** — Codex pre-impl F8 found no unimplemented input form (Items 7–12 cover TPS tone-1 / hyphen / POJ diacritic / partial-prefix / hanzi guard / custom dict); confirm on-device with the row-3 input matrix.

| Risk | Mitigation |
|---|---|
| Partial-prefix hits flood candidate strip (e.g., `t` matches 500+ entries) | Two-stage cap in `fetch_partial_prefix_candidates`: hydrate up to `PARTIAL_PREFIX_HYDRATE_CAP = 500` rowids (worst-case `dict.record` budget), then truncate to `PARTIAL_PREFIX_OUTPUT_CAP = 30` after dedupe + `SortKey` sort. Pre-fix the cap was a single `take(30)` on the FST byte-sorted rowid stream BEFORE hydration — globally high-frequency short candidates landing past byte-rank 30 were evicted without ever entering the sort (R6 limitation). Post-fix the output stays bounded but is the globally best-scoring subset of the hydrated pool. |
| Ranking surprises after `coverage_kind` insertion (full-syllable should still dominate when both present) | `partial_prefix_engine_path` test + `sort_key_partial_below_full` test pin invariant |
| Dogfood regression on input that worked via fallback but breaks under engine | Test matrix §15.6 + acceptance: type every input form (`t`, `gu`, `gua`, `gua2`, `guá`, `ㄍㄨㄚˋ`, `tai-bak`, `pe̍h`) and verify candidate strip non-empty after engine retire |
| Tab3 broken by accident during retire | Tab3 is a separate consumer on both platforms (iOS `DictionarySearchService`; Android `LexiconService.searchWithSources`/`searchByHanzi`, kept). Only the autocomplete `search()` surface was deleted; pinned by `platform_autocomplete_no_lexicon_branch`. |
| Codex consult overhead (8 sub-PRs × pre+post sandwich) | per `~/.claude/rules/round-workflow.md` § Codex review sandwich — each sub-PR small enough that sandwich is ~30min round-trip; total ~8-10 hours Codex consult time across all 8 PRs, spread over v3.5.8 dogfood window |

### 15.9 Out-of-scope confirmation (under §15)

Still out of scope even under expanded §15:
- MOE-style keyboard-level mode toggle (HanjiMode / TailoMode) — we keep `isTranslateSwapped` UX (§10 + §11)
- Hanzi-input mode as a primary input flow — Q1 clarified there is no such mode in our keyboard
- Tab3 dictionary search refactor — orthogonal feature
- Custom dict early-bind into engine (still Phase 9.6 as platform-side query path)
- v3.5.9+ planning — `feedback_v358_full_scope.md` rule reinforced ("修復到滿意為止")

---

## Cross-references

- Parent doc (display-fix subset §1–§14): [`continuous-candidate-display.md`](continuous-candidate-display.md).
- Sister doc (Model B commit/display contract): [`continuous-commit-and-display.md`](continuous-commit-and-display.md).
- Cross-platform parity invariants: [`../architecture/behavioral-invariants.md`](../architecture/behavioral-invariants.md) §14.
- Engine source: [`engine/composing/src/dispatch.rs`](../../engine/composing/src/dispatch.rs), [`engine/lexicon/src/continuous.rs`](../../engine/lexicon/src/continuous.rs).
