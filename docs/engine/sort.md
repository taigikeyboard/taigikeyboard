# UserFrequency and Sorting

> **Type**: Feature
> **Keywords**: `UserFrequency`, `Sort`, `Score`, `FrequencyData`, `ranking`
> **Related**: continuous-candidate-display.md, binary-format.md

---

## Summary

- Candidate ranking is the Continuous `FetchAtPos` sort key — see [continuous-input-ranking.md](continuous-input-ranking.md) for the formula (source rank, `calculate_continuous_score`, decayed user weight).
- User frequency is stored platform-side (SQLite, keyed by the `(word, tl)` pair) and sent to the engine per fetch as `FrequencyEntry` rows.
- The earlier additive score (`userFreqScore + recencyBonus + exactBonus + closenessBonus + baseFreqScore + completionPenalty`, `ProcessCandidates` op) had no production caller since v3.5.8 and was removed 2026-09-25.

---

## Data Structure

### FrequencyData

| Field | Type | Description |
|-------|------|-------------|
| `count` | Int | Usage count |
| `lastUsedMillis` | Int64 | Last used time (ms) |

### Database Schema

Identity is the `(word, tl)` pair — `UNIQUE(word, tl)`; `tl = ''` is the legacy pre-pair bucket. Full contract: `docs/architecture/behavioral-invariants.md` § user-frequency pair key.

---

## Frequency Sources

| Source | Database | Field | Description |
|--------|----------|-------|-------------|
| Dictionary frequency | `dictionary.bin` | `frequency` | Static (binary mmap) |
| User frequency | `user_frequency.db` | `count` | Dynamic, incremented on selection |
| Last used | `user_frequency.db` | `last_used` | Dynamic, updated on selection |

---

## Ownership

Ranking math lives in Rust `engine/ranking` (since v3.5.2). Frequency storage stays platform-side SQLite (`wont_migrate`).

| Item | Location |
|------|----------|
| Frequency map + boost / decay | Rust `engine/ranking/src/score.rs` (`build_frequency_map`, `FrequencyMap`, `user_freq_boost`, `decayed_user_weight_delta`) |
| Candidate sort | Rust `engine/lexicon/src/continuous/` (Continuous `FetchAtPos`) |
| iOS frequency service (SQLite) | `Lexicon/Services/UserFrequencyService.swift` + `Lexicon/Database/UserFrequencyRepository.swift` |
| Android frequency service (SQLite) | `ime/text/composing/UserFrequencyService.kt` |
| Bridge | `FrequencyEntry` rows built by `ComposingManager.buildFrequencyEntries` (iOS) / `RustEngineBridge.frequencyRowsToProtoEntries` (Android), sent with `FetchAtPos` |

---

## API

| Method | Description |
|--------|-------------|
| `recordUsage(word, tl)` | On selection: count+1, last_used=now for the `(word, tl)` pair |
| `frequencyDataBatch(words)` | Batch get frequency data |

---

## Notes

### fst Range Scan Characteristics

- `Map::range().ge(prefix).lt(prefix_upper_bound)` returns keys in lexicographic byte order — short keys naturally come before longer keys with the same prefix.
- Caller applies `limit` AFTER the range scan; the Continuous fetch dedups + reorders, so trim points are insensitive to fst iteration order.
- Toneless / abbrev / numeric-tone variants are all stored under the same `tl:` / `poj:` prefix family, so a single range scan covers exact + completion together.
