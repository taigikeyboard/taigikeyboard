# Khiin/RIME Word Lattice Architecture Research (2026-03-11)

## Khiin's Approach: Word-Level DP

- **Unified** segmentation + word selection in one step
- Pre-loads all words + frequencies into `HashMap<String, f64>` for O(1) lookup
- Cost function: `ln(1/freq^1.0) / word_len^0.2 * syl_count^0.2` (minimize)
- Source: `segmenter.rs` → `segment_min_cost()`
- Uses `<=` tie-breaking (later-evaluated path wins; moot with frequency data)

### DP Algorithm
1. Maintain `costs` array: `Vec<(f64, i32)>` — `(cumulative_cost, prev_word_start)`
2. For each position `i`, examine all possible words ending at `i`
3. Word length bounded by `max_word_len` (5-6 chars)
4. For valid word `[j..i]`: `curr_cost = costs[j].0 + cost_map[word]`
5. If `curr_cost <= min_cost`, update
6. Reconstruct path by backtracking

## RIME's Approach: Bigram Viterbi

- Word lattice (DAG) + Viterbi + bigram language model
- Score: `Σ log P(word_i | word_{i-1})` — captures word sequence patterns
- More accurate but requires corpus-trained bigram data

## Current Taigi Keyboard Gap

| Aspect | Khiin | RIME | Taigi Keyboard |
|--------|-------|------|----------------|
| Edges | Words | Words | Syllables |
| Cost data | Per-word freq | Bigram probs | Per-word freq in DB (unused in DP) |
| DP stage | Unified | Unified | Separated |
| Model | Unigram + length bias | Bigram | Syllable DP + user freq sorting |

Blockers:
- No word-to-cost mapping accessible to segmenter (freq locked in SQLite)
- Syllable trie has ~1550 entries vs thousands of word edges needed
- Memory overhead of loading all words + costs at init

## Migration Path

| Stage | Content | Risk |
|-------|---------|------|
| **1** | Expose word frequency to segmenter via closure for tie-breaking | Low |
| **2** | Word-level DP with pre-computed cost map (Khiin-style) | Medium |
| **3** | Bigram + Viterbi (RIME-style, needs corpus) | High |

## Data Structures for Stage 2

```swift
let costMap: [String: Double]  // word → cost (O(1) lookup)
// Or closure: (word: String) -> Double?
```

Dictionary already has `frequency` field — need to expose it to segmenter.

## Key Insight

Mobile keyboard = 1-3 words at a time with candidate bar. **Stage 1 is sufficient** for current needs. Stage 2+ only if sentence-level continuous input becomes a feature request.
