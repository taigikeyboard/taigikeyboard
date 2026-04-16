# NextWord Prediction

> **Type**: Feature
> **Keywords**: `NextWord`, `Bigram`, `WordAssociation`, `UserLearning`
> **Related**: autocomplete.md, sort.md

---

## Summary

- Predicts next possible word after composition ends
- Hybrid Bigram: dictionary (character-level) + user learning (word-level)
- Time-decay weighting (one-week half-life)
- User associations capped at 50,000 entries

---

## Data Model

### Hybrid Bigram

| Source | Query key | Prediction result | Description |
|--------|-----------|-------------------|-------------|
| Dictionary | Last character of selected word | Bigram (1 char) + Phrase (2-3 chars) | Cold start |
| User | Full selected word | Full word | After learning |

### Storage

**Dictionary associations** — `association.bin` (binary mmap, read-only)
- Sorted key table with binary search by `prev_word` (single character)
- Entries per key sorted by count DESC
- Fields: bitmask (u16), count (u32), next_word, next_tl
- Bitmask filter replaces SQL WHERE for dictionary source filtering

**User learning** — `user_association.db` (SQLite, writable, schema v4)

```sql
CREATE TABLE user_association (
    prev_word TEXT NOT NULL,   -- Previous word (full word, hanzi)
    prev_tl TEXT DEFAULT '',   -- Previous word TL (added in v4)
    next_word TEXT NOT NULL,   -- Next word (full word, hanzi)
    next_tl TEXT,              -- Next word TL
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP,
    UNIQUE(prev_word, next_word, next_tl)
);
CREATE INDEX idx_user_prev_word_tl ON user_association(prev_word, prev_tl);
```

**Migration history**:
- v0 → v3: `UNIQUE(prev_word, next_word)` → `UNIQUE(prev_word, next_word, next_tl)` (drop + recreate)
- v3 → v4: `ALTER TABLE ... ADD COLUMN prev_tl TEXT DEFAULT ''` + new index

---

## Core Flow

```
User selects candidate
    ↓
Record association (if previous word exists and interval < 10 seconds)
    ↓
Query predictions
  ├─ Dictionary: association.bin binary search (prev_word = last character) + bitmask filter
  └─ User: user_association.db SQL query (prev_word = full word)
    ↓
Time decay calculation
    ↓
Merge and sort, display candidates
```

---

## Weight Calculation

### Score Formula

```kotlin
// User score = count × 50 × decay
// Dictionary score = count × 1

decay = exp(-ageHours / 168.0 * 0.693)  // One-week half-life
```

### Decay Effect

| Time | Decay factor |
|------|--------------|
| Just used | 1.0 |
| 1 week later | 0.5 |
| 2 weeks later | 0.25 |
| 1 month later | 0.06 |

---

## Triggers and Reset

### Trigger Conditions

| Action | Hanzi mode | Romanization mode |
|--------|------------|-------------------|
| Tap candidate | Trigger | Trigger |
| Enter confirm | No trigger | Trigger |

### Context Reset

- Input sentence-ending punctuation (。！？)
- Long idle time (> 30 seconds)
- Switch input field
- Backspace to empty

---

## Noise Filtering

Characters not recorded as associations:
- Punctuation
- Whitespace (half/full-width)
- Numbers 0-9

---

## Implementation Status

### Android ✅

- NextWordService.kt (predict, recordAssociation)
- Time decay calculation
- User association cap (50,000 entries)
- Noise filtering

### iOS ✅

- NextWordService.swift
- Same logic as Android

---

## Platform Correspondence

| Component | Android | iOS |
|-----------|---------|-----|
| Service | `NextWordService.kt` | `NextWordService.swift` |
| Manager | `SmartbarManager.kt` | `ActionHandler+NextWord.swift` |
| Dict data | `association.bin` (binary mmap) + `user_association.db` (SQLite) | Same |

---

## Reference: librime-predict Ideas

> Source: [librime-predict](https://github.com/rime/librime-predict)

### 1. Sentence-start Prediction

Use a special `$` key to query predictions at the beginning of a sentence (no previous word context).

```kotlin
fun predict(word: String?): List<Prediction> {
    val query = word ?: "$"  // null indicates sentence start
    return db.lookup(query)
}
```

### 2. Continuous Prediction Limit

Prevent infinite loop when user keeps tapping predictions. librime-predict uses `max_iterations`.

```kotlin
private var iterationCounter = 0
private val maxIterations = 3

fun onWordSelected(word: String) {
    if (isShowingPrediction) {
        iterationCounter++
        if (iterationCounter >= maxIterations) {
            clearPrediction()
            return
        }
    }
    predict(word)
}
```

### 3. Comparison

| | librime-predict | Taigi Keyboard |
|--|-----------------|----------------|
| Storage | DoubleArray Trie (mmap, read-only) | Binary mmap (dict) + SQLite (user) |
| User learning | None | Yes |
| Sentence-start | `$` symbol | Not yet |
| Iteration limit | `max_iterations` config | Not yet (timeout only) |
