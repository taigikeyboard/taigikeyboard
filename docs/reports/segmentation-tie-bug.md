# Segmentation Score Tie Bug

> **Status**: RESOLVED — Fix implemented via `WordPrefixChecker` closure injection. See `engine/segmentation.md` for current design.

**File**: `ios/Sources/TaigiKeyboard/Input/SyllableSegmenter.swift`

## Problem
DP scoring `sum(len²)` is commutative — segment length permutations produce identical scores, causing wrong segmentation at CVC+V boundaries.

## Example
Input `kina2jit8` (intended: `kin-á-ji̍t` 今仔日):
- Path `ki`(2²) + `na2`(3²) = 4+9 = **13** ← wins (evaluated first)
- Path `kin`(3²) + `a2`(2²) = 9+4 = **13** ← correct but loses tie
- DP uses strict `>`, first arrival wins → wrong segmentation `ki + na2 + jit8`

## Systematic Pattern
Any **CVC + V** boundary where the consonant can attach to either side:
- `kina2jit8` → `ki+na2` (wrong) vs `kin+a2` (correct, 今仔日)
- `hita2e5` → `hi+ta2` (wrong) vs `hit+a2` (correct, 彼个)
- `sina2` → `si+na2` vs `sin+a2` (ambiguous without word frequency)

## Why `kin1a2jit8` Works
Tone digit makes `kin1` a 4-char segment (score 16), clearly beats `ki` (score 4). No tie.

## Root Cause
- Syllable-level scoring has no word frequency data
- Khiin solves this with word-level cost function (frequency-weighted)
- RIME solves this with word lattice + Viterbi (bigram model)
- Both do segmentation + word selection in one unified step
- Taigi Keyboard separates them: segmenter (syllable) → autocomplete (word)

## Caution with `>=` Approach
- `ama`: `>` gives `a+ma` (阿媽, correct); `>=` gives `am+a` (wrong)
- No universal tie-breaking rule works for all cases without frequency data

## Fix Applied
**Approach 1 (tie-breaker)** implemented via `WordPrefixChecker` closure:
- On score tie, reconstruct both candidate paths
- Prefix-search MARISA dictionary trie for each path
- Prefer the path with dictionary matches
- Zero overhead for non-tie cases (ties are rare, only at CVC+V boundaries)

Future directions (if needed):
- **Freq bonus in DP**: `sum(len² + freq_bonus)` — medium risk
- **Word lattice**: Unified segmentation + word selection (see `khiin-lattice-research.md`) — high risk
