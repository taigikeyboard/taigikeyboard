# Continuous-Input Candidate Display

> **Type**: Specification (anchor doc — the code holds the detail)
> **Keywords**: `Continuous`, `Candidate`, `CandidateMessage`, `roman`, `hanji`, `dual-line`, `partial-prefix`
> **Related**: [continuous-input-ranking.md](continuous-input-ranking.md), [continuous-commit-and-display.md](continuous-commit-and-display.md), [composing.md](composing.md)

The Continuous-input engine is the single candidate source on every platform. One carrier (`CandidateMessage`), one cell shape.

## 4. Candidate carrier

### 4.2 `CandidateMessage`

[`engine/protos/proto/composing.proto`](../../engine/protos/proto/composing.proto) `CandidateMessage`, the wire mirror of `lexicon::RawCandidate`:

| Field | Tag | Meaning |
|---|---|---|
| `consumed_span_end` | 2 | Byte offset in the raw pending input; sent back as `CommitContinuous.consumed_bytes` |
| `syllable_count` | 3 | Sent back as `CommitContinuous.syllable_count` |
| `display_text` | 4 | Canonical key = `hanji.unwrap_or(roman)`; the `user_frequency.db` / NextWord key, sent back as `CommitContinuous.canonical_text` |
| `roman` | 8 | Display romanization: TL, or POJ-rendered in POJ mode, recased to the typed span. Always present |
| `hanji` | 9 | `optional string`: absent = roman-only candidate (no Hanji exists), distinct from a present empty string |
| `canonical_tl` | 10 | Canonical TL identity (Core Principle #6), sent back as `CommitContinuous.association_tl` |

Tags 1, 5, 6, 7 are reserved. `roman` and `hanji` are display sidechannels; the engine resolves the committed text from them ([continuous-commit-and-display.md §10.3](continuous-commit-and-display.md#103-commit-contract)). Duplicates collapse on `(reading, hanji, consumed_span)` (§10.10 there).

## 5. Edge cases

| Candidate | `roman` | `hanji` | Cell |
|---|---|---|---|
| Hanji word | `tâi-uân` | `臺灣` | dual-line: `tâi-uân` / `臺灣` |
| Roman-only word | `tāi` | absent | single-line: `tāi` |
| Mixed-script word | `hip-siòng` | `hip相` | dual-line: `hip-siòng` / `hip相` |

The platform bridges decode `hanji` with its presence accessor (`hasHanji`); treating absent as `""` would turn a roman-only candidate into a defective dual-line cell. Hanji-first output and the TPS layout show the Hanji as the primary line; a roman-only candidate stays single-line (on TPS, its roman renders as Bopomofo).

## 15. Engine-only candidate source

- **15.3.D — Partial prefix.** When the syllabifier finds no valid syllable ending in the buffer (`gu`, `t`, `ㄉ`), the engine scans the whole-buffer key by prefix (`lexicon::fetch_partial_prefix_candidates`, called from `composing::continuous::assemble_candidates` Step 2) so the strip still shows dictionary words while the user types toward the first syllable boundary. Each hit spans the whole buffer, `consumed_span = (0, raw.len())`. The key builder returns no key for a buffer that is empty after tone-stripping, so there is no unbounded scan.
- **15.3.E — Hanji guard.** A composing buffer containing any Hanji character (`is_hanji`, any CJK code point) gets an empty `ContinuousResponse` — never a syllabified guess (`composing::requests` `FetchAtPos`; behavioral-invariants §14 `INVARIANT_LEX_HANJI_GUARD`).
- **15.4 — No platform lexicon branch.** The keyboard candidate path calls only `FetchAtPos`. The platform `LexiconService` serves dictionary search, never the candidate strip.
- **15.5 — Partial-prefix ordering.** A partial-prefix hit carries `coverage_kind = COVERAGE_KIND_PARTIAL_PREFIX`, an internal sort field that is not on the wire. Within a tier and preference class it follows every full-syllable candidate and every abbreviation, whatever its user weight, score, frequency or source; a whole-input partial hit (tier 0) still leads a front-segment full-syllable row (tier 1) ([continuous-input-ranking.md §3.2](continuous-input-ranking.md#32-user-weight-and-sort-key)).
- **15.6 — Empty engine, empty strip.** An empty engine result is an empty candidate row: no lexicon fallback, no slot-0 composing-text cell. Pinned by iOS `TaigiAutocompleteServiceContinuousTests` and Android `TaigiAutocompleteServiceTest`.
- **15.8 — Bounded work.** A partial-prefix lookup hydrates at most `PARTIAL_PREFIX_HYDRATE_CAP` (500) rowids, sorts them by `CandidateSortKey`, and returns at most `PARTIAL_PREFIX_OUTPUT_CAP` (30). The visible top N is therefore the best-ranked subset of the hydrated pool, not the FST byte-order prefix.
