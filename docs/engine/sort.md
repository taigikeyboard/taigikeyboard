# UserFrequency and Sorting

> **Type**: Feature
> **Keywords**: `UserFrequency`, `Sort`, `Score`, `FrequencyData`
> **Related**: autocomplete.md, trie.md

---

## Summary

- User frequency dominates sorting (stability priority)
- Recency and exact match only for fine-tuning
- SQLite pre-sort + in-memory precise sort

---

## Scoring Formula (v4)

```
score = userFreqScore + recencyBonus + exactBonus + closenessBonus + baseFreqScore
```

| Item | Description | Score range |
|------|-------------|-------------|
| `userFreqScore` | User frequency (dominant) | count × 100, max 10000 |
| `closenessBonus` | Match length closeness (cold-start key factor) | 0-500 |
| `recencyBonus` | Used within last hour | +200 |
| `exactBonus` | base roman == input (tone-stripped) | +100 |
| `baseFreqScore` | Dictionary frequency (fallback) | ~0-100 |

### closenessBonus detail

```
candidateBase = romanToBase(word.roman)   // strip tones, hyphens, diacritics
inputBase     = inputToBase(normalizedInput) // strip tone digits
matchRatio    = min(inputLen, candidateLen) / max(inputLen, candidateLen)
closenessBonus = int(matchRatio × 500)
```

Rewards candidates whose romanization length closely matches the input.
For input "taitsi" (6 chars): exact 2-syllable match → 500, 3-syllable prefix → ~333.

### exactBonus fix (v3 → v4)

v3 compared display roman (with diacritics) against normalized input (numeric tones) —
these formats never matched, making exactBonus effectively dead code.
v4 compares tone-stripped base forms so exact matches correctly receive +100.

---

## Design Philosophy

1. **User frequency dominates**: Common words stay at top, unaffected by other factors
2. **Closeness rewards precision**: Shorter, closer matches rank higher for cold-start
3. **Fine-tuning factors**: Recency/exactBonus only matter when scores are similar

---

## Priority Order

1. **User frequency**: max +10000
2. **Match closeness**: max +500 (dominant for cold-start)
3. **Recency**: Tiebreaker at same frequency, +200
4. **Exact match**: +100
5. **Dictionary frequency**: Fallback for new words

---

## Data Structure

### FrequencyData

| Field | Type | Description |
|-------|------|-------------|
| `count` | Int | Usage count |
| `lastUsedMillis` | Int64 | Last used time (ms) |

### Database Schema

```sql
CREATE TABLE user_frequency (
    word TEXT NOT NULL UNIQUE,
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

---

## Frequency Sources

| Source | Database | Field | Description |
|--------|----------|-------|-------------|
| Dictionary frequency | `dictionary.db` | `frequency` | Static |
| User frequency | `user_frequency.db` | `count` | Dynamic, incremented on selection |
| Last used | `user_frequency.db` | `last_used` | Dynamic, updated on selection |

---

## Platform Correspondence

| Item | iOS | Android |
|------|-----|---------|
| Scoring | `CandidateProcessor.calculateScore()` | `LexiconService.calculateScore()` |
| Frequency service | `UserFrequencyService.swift` | `UserFrequencyService.kt` |
| Database | `UserFrequencyRepository.swift` | Built into Service |

---

## API

| Method | Description |
|--------|-------------|
| `recordUsage(word)` | On selection: count+1, last_used=now |
| `frequencyData(word)` | Get single word frequency data |
| `frequencyDataBatch(words)` | Batch get frequency data |

---

## Example

**Input `taitsi` (cold-start, no user frequency)**:

| Candidate | userFreq | recency | exact | closeness | base | total |
|-----------|----------|---------|-------|-----------|------|-------|
| `tāi-tsì` 代誌 (freq=800) | 0 | 0 | 100 | 500 | 80 | **680** |
| `tāi-tsì-lâng` (freq=2000) | 0 | 0 | 0 | 333 | 200 | **533** |

Result: `tāi-tsì` > `tāi-tsì-lâng` (exact match wins despite lower frequency)

**Input `gua` (with user frequency)**:

| Candidate | userFreq | recency | exact | closeness | base | total |
|-----------|----------|---------|-------|-----------|------|-------|
| `gua2-ho2` (freq=80, just used) | 8000 | 200 | 0 | 250 | ~10 | ~8460 |
| `gua2` (freq=50) | 5000 | 0 | 0 | 500 | ~8 | ~5508 |
| `gua` (freq=0) | 0 | 0 | 100 | 500 | ~5 | ~605 |

Result: `gua2-ho2` > `gua2` > `gua` (user frequency still dominates)

---

## Notes

### MARISA-trie Sorting Characteristics

- `predictive_search` returns depth-first, longer words first
- Short words easily truncated by limit

### Solutions

1. Combine exact match + prefix search
2. SQLite sort by frequency preserves high-frequency words
3. notone index allows toneless words to exact match
