# UserFrequency and Sorting

> **Type**: Feature
> **Keywords**: `UserFrequency`, `Sort`, `Score`, `FrequencyData`
> **Related**: autocomplete.md, trie.md

---

## Summary

- User frequency dominates sorting (stability priority)
- Recency and exact match only for fine-tuning
- Binary mmap lookup + in-memory sort

---

## Scoring Formula (v5)

```
score = userFreqScore + recencyBonus + exactBonus + closenessBonus + baseFreqScore + completionPenalty
```

| Item | Description | Score range |
|------|-------------|-------------|
| `userFreqScore` | User frequency (dominant) | count × 100, max 10000 |
| `completionPenalty` | Penalizes candidates extending beyond input | 0 or -1000 |
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

### completionPenalty (v4 → v5)

v4 used a flat additive sum where dictionary frequency (`baseFreqScore`) could push
multi-syllable completions above single-syllable exact matches for short inputs like "a".
v5 adds a -1000 penalty for completions (candidateBase != inputBase), aligned with
RIME's quality penalty and Mozc's cost penalty. Cold-start exact matches always rank
above cold-start completions; user frequency (~15+ uses) can still overcome the penalty.

### exactBonus fix (v3 → v4)

v3 compared display roman (with diacritics) against normalized input (numeric tones) —
these formats never matched, making exactBonus effectively dead code.
v4 compares tone-stripped base forms so exact matches correctly receive +100.

---

## Design Philosophy

1. **User frequency dominates**: Common words stay at top, unaffected by other factors
2. **Exact > completion in cold-start**: -1000 penalty separates tiers; user frequency (~15+ uses) overcomes it
3. **Closeness rewards precision**: Shorter, closer matches rank higher among completions
4. **Fine-tuning factors**: Recency/exactBonus only matter when scores are similar

---

## Priority Order

1. **User frequency**: max +10000
2. **Completion penalty**: -1000 for non-exact matches (separates exact vs completion tiers)
3. **Match closeness**: max +500 (orders completions among themselves)
4. **Recency**: Tiebreaker at same frequency, +200
5. **Exact match**: +100
6. **Dictionary frequency**: Fallback for new words

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
| Dictionary frequency | `dictionary.bin` | `frequency` | Static (binary mmap) |
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

| Candidate | userFreq | recency | exact | closeness | completion | base | total |
|-----------|----------|---------|-------|-----------|------------|------|-------|
| `tāi-tsì` 代誌 (freq=800) | 0 | 0 | 100 | 500 | 0 | 80 | **680** |
| `tāi-tsì-lâng` (freq=2000) | 0 | 0 | 0 | 333 | -1000 | 200 | **-467** |

Result: `tāi-tsì` > `tāi-tsì-lâng` (exact match wins; completion penalty separates tiers)

**Input `a` (cold-start, short input)**:

| Candidate | userFreq | recency | exact | closeness | completion | base | total |
|-----------|----------|---------|-------|-----------|------------|------|-------|
| `à` 亞 (exact) | 0 | 0 | 100 | 500 | 0 | ~5 | **~605** |
| `a-kong` 阿公 (completion, high freq) | 0 | 0 | 0 | ~100 | -1000 | ~80 | **~-820** |
| `a-kong` 阿公 (20 uses) | 2000 | 0 | 0 | ~100 | -1000 | ~80 | **~1180** |

Result: cold-start exact > cold-start completion; frequent completion rises above exact

**Input `gua` (with user frequency)**:

| Candidate | userFreq | recency | exact | closeness | completion | base | total |
|-----------|----------|---------|-------|-----------|------------|------|-------|
| `gua2-ho2` (freq=80, just used) | 8000 | 200 | 0 | 250 | -1000 | ~10 | ~7460 |
| `gua2` (freq=50) | 5000 | 0 | 0 | 500 | -1000 | ~8 | ~4508 |
| `gua` (freq=0) | 0 | 0 | 100 | 500 | 0 | ~5 | ~605 |

Result: `gua2-ho2` > `gua2` > `gua` (user frequency still dominates over penalty)

---

## Notes

### MARISA-trie Sorting Characteristics

- `predictive_search` returns depth-first, longer words first
- Short words easily truncated by limit

### Solutions

1. Combine exact match + prefix search
2. In-memory sort by frequency after binary mmap read
3. notone index allows toneless words to exact match
