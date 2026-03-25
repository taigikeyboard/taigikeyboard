# Syllable Segmentation (Archived)

> **Status**: REMOVED — Auto-segmentation was removed. This document is preserved for historical reference.
> **Type**: Feature (archived)
> **Keywords**: `SyllableSegmenter`, `DAG`, `DP`, `onset`, `trie`, `segmentContinuous`
> **Related**: composing.md, autocomplete.md, tone.md

---

## Summary

- Segments continuous Taigi romanization input into individual syllables
- Uses a **syllable trie** + **DAG** + **dynamic programming** pipeline
- Supports both TL and POJ input forms
- Multi-character onsets (`tsh`, `ph`, `th`, `kh`, `ts`, `ch`, `chh`) are atomic units
- **Dictionary tie-breaking** resolves CVC+V boundary ambiguity via optional `WordPrefixChecker`

---

## Algorithm Overview

```
rawInput → lowercase → build DAG (trie walk) → DP (squared scoring) → reconstruct → segments[]
```

### Four Phases

1. **Trie Walk (DAG construction)**: At each position, walk the syllable trie to find all valid syllable spans
2. **DP Optimization**: Find the path maximizing `sum(len^2)` — strongly favors longer matches
3. **Tie-Breaking**: On score ties, use optional `WordPrefixChecker` to prefer dictionary-backed paths
4. **Reconstruction**: Backtrack from end to start, extract segments from original (case-preserved) input

---

## Syllable Trie

- Built once (lazy static), from `TaigiPhonetics.tlInitials` x `TaigiPhonetics.tlFinals`
- Contains all valid TL syllables (e.g. `tsha`, `phong`, `gua`) + POJ variants (e.g. `chha`, `koa`)
- Multi-character onsets and nasalization suffix inserted as standalone entries for partial-input atomicity

### Trie Contents

| Source | Example Entries | Count |
|--------|----------------|-------|
| TL `initial + final` | `tsha`, `phong`, `khi`, `gua`, `a`, `ng` | ~1350 |
| POJ `initial + final` | `chha`, `koa`, `peng`, `chhoe` | ~200 (differences only) |
| Multi-char onsets (TL) | `ph`, `th`, `kh`, `ts`, `tsh`, `ng` | 6 |
| Multi-char onsets (POJ) | `ch`, `chh` | 2 |
| Nasalization suffix | `nn` | 1 |

### POJ Input-Form Conversion

- Finals differ from TL: `ua`->`oa`, `ue`->`oe`, `ing`->`eng`, `ik`->`ek`
- Initials differ: `ts`->`ch`, `tsh`->`chh`
- Both forms coexist in trie so either input is recognized

---

## Onset Atomicity

- **Requirement**: Multi-character onsets must never be split during segmentation
- **Reference**: MOE2 layout, where onsets are single keys (`tsh`, `ph`, `th`, `kh`, `ts`, `ch`, `chh`)
- **Mechanism**: Onsets are inserted into the trie as valid entries
- **Safety**: DP squared scoring ensures complete syllables always outscore onset + remainder

### Scoring Examples

| Input | Onset-split score | Full-syllable score | Winner |
|-------|-------------------|---------------------|--------|
| `tsha` | `tsh`(9) + `a`(1) = 10 | `tsha`(16) | `tsha` |
| `phong` | `ph`(4) + `ong`(9) = 13 | `phong`(25) | `phong` |
| `tsh` (incomplete) | `t`(1)+`s`(1)+`h`(1) = 3 | `tsh`(9) | `tsh` |
| `ka2tsh` | `ka2`(9)+`t`+`s`+`h`(3) = 12 | `ka2`(9)+`tsh`(9) = 18 | `ka2`+`tsh` |

---

## Nasalization Suffix Atomicity

- **`nn`** is a single key on MOE2 (nasalization marker, e.g. `kann`, `phiann`)
- Inserted into the trie as a standalone valid entry
- Prevents `nn` from splitting into `n` + `n` during incomplete input
- Complete nasalized syllables (`kann`, `ann`, `iunn`) always outscore `nn` as fragment

### Scoring Examples

| Input | Split score | Atomic score | Full-syllable score | Winner |
|-------|------------|--------------|---------------------|--------|
| `nn` alone | `n`(1)+`n`(1) = 2 | `nn`(4) | — | `nn` |
| `ka2nn` | `ka2`(9)+`n`+`n`(2) = 11 | `ka2`(9)+`nn`(4) = 13 | — | `ka2`+`nn` |
| `kann` | — | `ka`(4)+`nn`(4) = 8 | `kann`(16) | `kann` |
| `ann` | — | `a`(1)+`nn`(4) = 5 | `ann`(9) | `ann` |

---

## DAG Construction

- For each position `i` in lowercased input:
  - Skip if `chars[i]` is a digit (digits are consumed by preceding syllable)
  - Walk trie from `chars[i]` forward
  - On each valid syllable node, record an edge `(i, end, len)`
  - If next char after valid node is a digit (1-9): consume it as tone → `end = j+1`
  - Otherwise: `end = j`

### Tone Digit Consumption

- Digits 1-9 after a valid syllable are consumed as tone terminators
- Creates unambiguous syllable boundaries: `gua2si7` → `gua2` + `si7`
- Digits at non-syllable positions fall through to single-char fallback

---

## DP Scoring

- Objective: maximize `sum(segment_length^2)`
- Squared scoring strongly favors fewer, longer segments over many short ones
- Single-char fallback score: `1` (always available, handles unknown input)

### DP Recurrence

```
For each position i:
  For each edge (i → end, len):
    newScore = score[i] + len*len
    if newScore > score[end]:
      update (strictly better)
    else if newScore == score[end] AND wordPrefixChecker != nil:
      resolve tie via dictionary lookup
  Fallback:
    score[i+1] = max(score[i+1], score[i] + 1)
```

---

## CVC+V Tie-Breaking

### Problem

`sum(len^2)` is commutative — segment length permutations produce identical scores.
This causes wrong segmentation at CVC+V boundaries where a consonant can attach to either side.

| Input | Path A | Path B | Score |
|-------|--------|--------|-------|
| `kina2jit8` | `ki`(4) + `na2`(9) = 13 | `kin`(9) + `a2`(4) = 13 | **tie** |
| `hita2e5` | `hi`(4) + `ta2`(9) = 13 | `hit`(9) + `a2`(4) = 13 | **tie** |
| `ama` | `a`(1) + `ma`(4) = 5 | `am`(4) + `a`(1) = 5 | **tie** |

Without tie-breaking, the first path evaluated wins (strict `>`), which is arbitrary.

### Why `len^2` only fails at ties

Mathematically, for any fixed total length, `sum(len^2)` is maximized when one segment is
as long as possible (power mean inequality). The only case where two paths score equally is
when their segment lengths are permutations of each other (e.g., [2,3] vs [3,2]).
**No non-tie failure case exists.**

### Solution: `WordPrefixChecker`

When a score tie occurs AND an optional `WordPrefixChecker` closure is provided:

1. Reconstruct both candidate segment paths from `prev[]`
2. Build continuous search keys with default tones (open → 1, stop → 4), joined without separator to match MARISA trie key format
3. Prefix-search the MARISA dictionary trie via the checker closure
4. If one path has dictionary matches and the other doesn't, prefer the matching path
5. If both or neither match, keep the current path (existing behavior preserved)

### Trace: `kina2jit8`

```
At end=5, tie (13 == 13):
  Current: ["ki", "na2"] → key "ki1na2" → prefix search → no match
  New:     ["kin", "a2"] → key "kin1a2" → prefix search → matches "kin1a2jit8" (今仔日)
  → Update to new path
Result: ["kin", "a2", "jit8"] ✓
```

### Trace: `ama`

```
At end=3, tie (5 == 5):
  Current: ["a", "ma"] → key "ama" → prefix search → matches "ama" (阿媽)
  New:     ["am", "a"] → key "am1a" → prefix search → no match
  → Keep current path
Result: ["a", "ma"] ✓
```

### Design Decisions

- **Closure injection** (`WordPrefixChecker` typealias): keeps `SyllableSegmenter` as a pure
  utility with no forced dependencies. Default `nil` preserves full backward compatibility.
- **Only on ties**: zero overhead for the common case. Ties are rare (only at CVC+V boundaries
  with matching length permutations).
- **Prefix search** (not exact match): handles partial input gracefully (user may be mid-word).
- **Default tone logic**: reuses the same rules as `AutocompleteService.buildSearchKey` —
  open syllable gets tone 1, stop final (p/t/k/h) gets tone 4.

### Integration

Both callers pass a checker closure using `TrieService.shared.prefixSearch`:

```
ComposingManager.deriveDisplay()     → SyllableSegmenter.segment(raw, wordPrefixChecker: checker)
AutocompleteService.buildSearchKey() → SyllableSegmenter.segment(raw, wordPrefixChecker: checker)
```

### Test Cases

| Input | Without checker | With checker | Notes |
|-------|----------------|--------------|-------|
| `kina2jit8` | `["ki", "na2", "jit8"]` | `["kin", "a2", "jit8"]` | Checker fixes tie (今仔日) |
| `ama` | `["a", "ma"]` | `["a", "ma"]` | Both correct (阿媽) |
| `hita2e5` | `["hi", "ta2", "e5"]` | `["hit", "a2", "e5"]` | Checker fixes tie (彼个) |

---

## Word Grouping

After segmentation, syllables are grouped into words for composing display using
`SyllableSegmenter.groupIntoWords()`.

### Algorithm

Greedy left-to-right, longest match first:

```
i = 0
while i < syllables.count:
  for len in remaining...2:
    key = buildContinuousKey(syllables[i..<i+len], hasTones)
    if checker(key):
      groups.append(syllables[i..<i+len])
      i += len
      break
  else:
    groups.append([syllables[i]])
    i += 1
```

### Display Rules

- Within a word group: syllables joined with `-` (hyphen)
- Between word groups: joined with ` ` (space)
- Explicit user hyphens (trailing `-` on syllable) preserved as-is

### Examples

| Syllables | Groups | Display |
|-----------|--------|---------|
| `["gua2", "kin", "a2", "jit8"]` | `[["gua2"], ["kin", "a2", "jit8"]]` | `guá kin-á-ji̍t` |
| `["a", "ma"]` | `[["a", "ma"]]` | `a-má` |
| `["gua2", "si7", "soo"]` (no match) | `[["gua2"], ["si7"], ["soo"]]` | `guá sī soo` |

### Backward Compatibility

When `wordPrefixChecker` is nil, each syllable is its own group → all spaces (existing behavior).

---

## Hyphen Handling

- If input contains `-`, split by hyphens first, segment each part independently
- Hyphens are preserved by appending to the preceding segment
- Enables backward-compatible hyphen-separated input alongside continuous input

| Input | Segments |
|-------|----------|
| `gua2-si7` | `["gua2-", "si7"]` |
| `gua2si7-soo` | `["gua2", "si7-", "soo"]` |
| `ka2-lang5-e5` | `["ka2-", "lang5-", "e5"]` |

---

## Integration Points

### ComposingManager (`deriveDisplay`)

```
rawInput → SyllableSegmenter.segment() → ToneConverter per segment → join with spaces
```

- Segments joined with spaces (display), except after hyphen-ending segments
- Example: `gua2si7` → segments `["gua2", "si7"]` → display `"gua si7"` or `"gua si7"` depending on tone conversion

### AutocompleteService (`buildSearchKey`)

```
rawInput → SyllableSegmenter.segment() → add default tones to non-final segments → join with hyphens
```

- Non-final toneless segments get default tone: `1` (open syllable) or `4` (stop final)
- Joined with hyphens for trie prefix search: `gua2si7` → `gua2-si7`

---

## Case Handling

- Trie walk uses **lowercased** input internally
- Segment extraction uses **original input** offsets for case preservation
- Example: `Gua2si7` → segments `["Gua2", "si7"]`

---

## Platform Correspondence

| Item | iOS | Android |
|------|-----|---------|
| Segmenter | `SyllableSegmenter.swift` | `SyllableSegmenter.kt` |
| Phonetics data | `TaigiPhonetics.swift` | `TaigiPhonetics.kt` |
| Tests | `SyllableSegmenterTests.swift` | — |

---

## Test Cases

### Basic Segmentation

| Input | Expected | Notes |
|-------|----------|-------|
| `gua2si7soo` | `["gua2", "si7", "soo"]` | Multi-syllable continuous |
| `ka2` | `["ka2"]` | Single syllable with tone |
| `ka` | `["ka"]` | Single syllable, no tone |
| `""` | `[]` | Empty input |

### Onset Atomicity

| Input | Expected | Notes |
|-------|----------|-------|
| `tsh` | `["tsh"]` | Standalone onset, not `["t","s","h"]` |
| `ph` | `["ph"]` | 2-char onset |
| `th` | `["th"]` | 2-char onset |
| `kh` | `["kh"]` | 2-char onset |
| `ts` | `["ts"]` | 2-char onset |
| `ch` | `["ch"]` | POJ onset |
| `chh` | `["chh"]` | POJ onset |
| `ka2tsh` | `["ka2", "tsh"]` | Trailing incomplete onset |
| `gua2si7th` | `["gua2", "si7", "th"]` | Trailing onset after multiple syllables |

### Onset + Final (Regression)

| Input | Expected | Notes |
|-------|----------|-------|
| `tsha2` | `["tsha2"]` | Full syllable, not `["tsh","a2"]` |
| `phong` | `["phong"]` | Full syllable, not `["ph","ong"]` |
| `thau5` | `["thau5"]` | Full syllable, not `["th","au5"]` |
| `chhi2ka1` | `["chhi2", "ka1"]` | POJ multi-syllable |

### Nasalization Suffix Atomicity

| Input | Expected | Notes |
|-------|----------|-------|
| `nn` | `["nn"]` | Standalone, not `["n","n"]` |
| `ka2nn` | `["ka2", "nn"]` | Trailing `nn` stays atomic |
| `kann2` | `["kann2"]` | `nn` part of complete syllable |
| `ann` | `["ann"]` | `nn` part of complete syllable |
| `phiann3` | `["phiann3"]` | `nn` part of complete syllable |
| `iunn5` | `["iunn5"]` | `nn` part of complete syllable |

### Hyphen Handling

| Input | Expected |
|-------|----------|
| `gua2-si7` | `["gua2-", "si7"]` |
| `gua2si7-soo` | `["gua2", "si7-", "soo"]` |

### CVC+V Tie-Breaking

| Input | Without checker | With checker | Notes |
|-------|----------------|--------------|-------|
| `kina2jit8` | `["ki", "na2", "jit8"]` | `["kin", "a2", "jit8"]` | Checker fixes tie (今仔日) |
| `ama` | `["a", "ma"]` | `["a", "ma"]` | Both correct (阿媽) |
| `hita2e5` | `["hi", "ta2", "e5"]` | `["hit", "a2", "e5"]` | Checker fixes tie (彼个) |

### Edge Cases

| Input | Expected | Notes |
|-------|----------|-------|
| `xyz` | `["x", "y", "z"]` | Unrecognized → single-char fallback |
| `Gua2si7` | `["Gua2", "si7"]` | Case preserved |
| `gua2si7hak8sing1e5` | `["gua2", "si7", "hak8", "sing1", "e5"]` | 5 syllables |
