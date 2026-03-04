# khiin-rs Reference Research

> **Type**: Reference
> **Keywords**: `khiin`, `起引`, `DPSegment`, `Bigram`, `Trie`
> **Related**: rime-reference.md, azookey-reference.md

---

## Summary

- Dual Trie architecture: WordTrie (prefix search) + SyllableTrie (syllable validation)
- N-gram model: Unigram + Bigram frequency sorting
- DP dynamic programming segmentation: Supports continuous input (no spaces)
- Three input modes: Continuous / Classic / Manual

---

## Architecture Overview

### Dual Trie System

| Trie | Use | Methods |
|------|-----|---------|
| WordTrie | Romanization → word ID | `find_words_by_prefix`, `find_words_from_start` |
| SyllableTrie | Syllable validation | `is_valid_prefix`, `is_valid_syllable` |

### Database Structure

```sql
-- Unigram: single word frequency
CREATE TABLE unigrams (
    gram TEXT NOT NULL UNIQUE,
    n INTEGER NOT NULL
);

-- Bigram: word association
CREATE TABLE bigrams (
    lgram TEXT,      -- Previous word
    rgram TEXT,      -- Next word
    n INTEGER NOT NULL
);
```

---

## DP Segmentation Algorithm

### Concept

```
Input: "goabehchiahpng"
  ↓
DP split: ["goa", "beh", "chiah", "png"]
  ↓
Query output: "我欲食飯"
```

### Cost Calculation

```rust
const FREQUENCY_BIAS: f64 = 1.0;
const LETTER_COUNT_BIAS: f64 = 0.2;
const SYLLABLE_COUNT_BIAS: f64 = 0.2;

cost = ln(1.0 / frequency^FREQUENCY_BIAS);
cost = cost / letter_bias * syllable_bias;
```

### Kotlin Implementation Example

```kotlin
fun segment(input: String, syllableFreq: Map<String, Int>): List<String> {
    val n = input.length
    val dp = DoubleArray(n + 1) { Double.MAX_VALUE }
    dp[0] = 0.0

    for (i in 0 until n) {
        for (j in (i + 1)..minOf(i + 6, n)) {
            val syllable = input.substring(i, j)
            val freq = syllableFreq[syllable] ?: continue
            val cost = ln(1.0 / freq)
            if (dp[i] + cost < dp[j]) {
                dp[j] = dp[i] + cost
            }
        }
    }
    // Backtrack for result
}
```

---

## Taigi Keyboard Application Suggestions

### P0 - Immediately Usable

**1. Tone Position Detection**
```
Priority order: oa > o > a > e > u > i > ng > n > m
```

**2. Valid Syllable Validation (Regex)**
```regex
^·?((chh|[ckpt]h|[bhgjklmnpst])?(iau|io͘|oai|a[iu]|i[aou]|o[ae͘]|ui|[aeiou])?(ng|[mnptkh])?|(chh|[ckpt]h|[hkmnpst])ng?|(ng|m)h?)(n|ⁿ|ᴺ)?$
```

**3. Special Symbol Conversion**

| Input | Output | Description |
|-------|--------|-------------|
| `nn` | `ⁿ` | Nasalization |
| `oo` | `o͘` | Long o |

**4. Telex Tone Input**

| Tone | Number | Telex |
|------|--------|-------|
| 2 | 2 | s |
| 3 | 3 | f |
| 5 | 5 | l |
| 7 | 7 | j |

### P1 - Short-term Integration

| Feature | Description |
|---------|-------------|
| Three input modes | Continuous / Classic / Manual |
| Multi-level query | Exact → Fuzzy → Segmented |
| Buffer state machine | Empty → Composing → Converted → Selecting |

### P2 - Mid-term Planning

| Feature | Description |
|---------|-------------|
| Bigram sorting | Consider previous word context |
| Neutral tone handling | Khinless / Hyphen / Dot modes |
| DP segmentation | Support continuous input |

---

## Implementation Priority Order

```
Phase 1 (Tone Processing)
├── Tone position detection
├── Valid syllable validation
├── Special symbol conversion
└── Telex tone input

Phase 2 (Candidate Optimization)
├── Three input modes
├── Multi-level query
└── Buffer state machine

Phase 3 (Advanced Features)
├── Bigram sorting
├── Neutral tone handling
└── DP segmentation
```

---

## Comparison with This Project

### Overview

| Item | khiin-rs | TaigiKeyboard |
|------|----------|---------------|
| Database | Single khiin.db | Separate dictionary.db + word_association.db |
| Association model | Bigram (lgram → rgram) | Bigram (NextWordService) |
| Segmentation | Word-level DP with frequency cost | Syllable-level DP with `len^2` + dictionary tie-break |
| Engine | Rust + JNI | Native Kotlin/Swift |
| Target platform | Desktop (Windows/Linux) | Mobile (iOS/Android) |

### Segmentation Architecture Comparison

**Core difference: DP edges are words (Khiin) vs syllables (TaigiKeyboard).**

| Aspect | khiin-rs | TaigiKeyboard |
|--------|----------|---------------|
| DAG edge unit | Word (multi-syllable entries in `cost_map`) | Syllable (from syllable trie) |
| Scoring | `cost = ln(1/p) / word_len^0.2 * n_syls^0.2` (minimize) | `score = sum(len^2)` (maximize) |
| Frequency data | Core: `p` field drives segmentation directly | Tie-break only: prefix search on MARISA trie |
| Segmentation + word selection | Unified (one step) | Separated (segmenter → autocomplete) |
| Memory | Entire dictionary loaded into `HashMap<String, f64>` | Syllable trie (~1550 entries) + on-demand trie queries |
| Testability | Requires dictionary fixture | Pure function (no dictionary dependency when checker is nil) |

### Khiin Cost Function (segmenter.rs)

```rust
// Three tuning constants
const FREQUENCY_BIAS: f64 = 1.0;       // Weight of word frequency
const LETTER_COUNT_BIAS: f64 = 0.2;    // Preference for longer words
const SYLLABLE_COUNT_BIAS: f64 = 0.2;  // Preference for fewer syllables

// Cost calculation per word
cost = ln(1 / p^FREQUENCY_BIAS)
     / word_len^LETTER_COUNT_BIAS
     * n_syls^SYLLABLE_COUNT_BIAS

// DP: minimize total cost (sum of word costs)
// Uses <= for tie-breaking (later-evaluated path wins)
```

Key properties:
- Higher frequency → lower cost → preferred
- Longer word → cost divided by `word_len^0.2` → preferred
- More syllables → cost multiplied by `n_syls^0.2` → slight penalty

### Why TaigiKeyboard Does NOT Need Khiin's Approach (Current Assessment)

1. **Different use case**: Mobile keyboard users input 1-3 words at a time with a candidate bar.
   Khiin targets desktop continuous input of entire sentences (10+ words).

2. **`len^2` only fails at ties**: Mathematically, `sum(len^2)` gives wrong results only when
   segment lengths are permutations (e.g., 2^2+3^2 = 3^2+2^2). Dictionary tie-breaking already
   resolves these cases. No non-tie failure case exists.

3. **Two-stage architecture has advantages**: Keeping segmenter as a pure function (no dictionary
   dependency) makes it simpler to test and maintain. Word selection in autocomplete is a separate
   concern.

4. **Missing prerequisite**: Khiin's cost function requires per-word frequency data (`p` field).
   TaigiKeyboard's dictionary does not currently have frequency weights. Adding frequency data
   is a data engineering task, not an algorithm task.

### When to Reconsider

Switching to Khiin-style word-level DP would be warranted if:

| Trigger | Why |
|---------|-----|
| Sentence-level continuous input (5+ words without selecting) | Current syllable DP can't resolve long ambiguity chains |
| Users report poor candidate ranking despite correct segmentation | Indicates autocomplete stage needs more upstream context |
| Word frequency data becomes available | Removes the main prerequisite barrier |

### Evolution Path

```
Current (sufficient for mobile):
  len^2 + dictionary tie-break (syllable-level DP)

If needed (medium-term):
  Add word frequency to dictionary → use as scoring signal in DP

If needed (long-term):
  Word-level DP with frequency cost (Khiin-style unified segmentation + selection)
```

---

## khiin-rs Source Code Reference

| File | Content |
|------|---------|
| `ji/src/lomaji.rs` | Tone conversion, syllable validation |
| `ji/src/tone.rs` | Tone processing, Telex |
| `khiin/src/data/segmenter.rs` | DP segmentation |
| `khiin/src/input/converter.rs` | Candidate generation |
| `khiin/src/buffer/buffer_mgr.rs` | Buffer management |
