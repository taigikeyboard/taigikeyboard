# Segmentation Score Tie Bug

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

## Proposed Fix Directions
1. **Tie-breaker (Low risk)**: When DP scores tie, query dictionary via closure to see which segmentation forms a known word
2. **Freq bonus in DP (Medium risk)**: Change scoring from `sum(len²)` to `sum(len² + freq_bonus)`
3. **Word lattice (High risk)**: Unify segmentation + word selection into one step (see `khiin-lattice-research.md`)

Recommendation: Start with approach 1 (tie-breaker), lowest risk and solves known issues.
