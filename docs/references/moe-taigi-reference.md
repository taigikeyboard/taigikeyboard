# MOE Taigi IME Reference Analysis

> **Type**: Reference
> **Keywords**: `MOE`, `Segmentation`, `Nail`, `InputLine`, `Trie`
> **Related**: ../engine/trie.md, ../engine/composing.md, ../engine/autocomplete.md

---

## Summary

- MOE uses C++ closed-source engine + SWIG JNI interface
- Core concepts: InputLine (input line) + Nail (pinning) + Segmentation
- Data storage: Custom binary trie (tailo.tab), not SQLite
- Only supports Tailo (TL), not POJ

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                   Android App                        │
├─────────────────────────────────────────────────────┤
│  TaigiIME.java → ViewModel → Tailo.java (SWIG)      │
├─────────────────────────────────────────────────────┤
│                  TailoJNI.java                       │
├─────────────────────────────────────────────────────┤
│              C++ Native Library                      │
│         (Closed source, Trie + Segmentation)        │
├─────────────────────────────────────────────────────┤
│                  tailo.tab                           │
│            (Binary Trie Database)                    │
└─────────────────────────────────────────────────────┘
```

---

## Core Data Structures

### InputLine

Represents the line of text being input, contains state and cursor position.

```java
// Create and destroy
CreateInputLine(database) → InputLine
DestroyInputLine(inputLine)
ClearInputLine(inputLine)

// Input operations
InsertKey(inputLine, char)
RemoveKey(inputLine, Direction)  // FORWARD | BACKWARD

// Cursor control
GetCaretPosition(inputLine) → long
MoveBack(inputLine) → boolean
MoveForward(inputLine) → boolean
MoveHome(inputLine)
MoveEnd(inputLine)
MoveTo(inputLine, position) → boolean

// State query
GetInputLineSize(inputLine) → long
```

### Nail Mechanism

"Nail" is MOE's core concept, used for segment-by-segment candidate confirmation.

```java
// Get candidates at current nail position
GetNailPosition(inputLine) → long
GetCandidateCountAtNailPos(inputLine, vocType) → long
GetCandidatesAtNailPos(inputLine, vocType, offset, limit, handler)

// Confirm candidate (nail)
NailCandidate(inputLine, vocType, candidateIndex) → boolean
DoneNailing(inputLine) → boolean

// Commit confirmed text
PopFront(inputLine) → int
GetConfirmedCandidates(inputLine, handler)
```

**Flow Diagram:**

```
Input: "guasiaitaigi"
      ↓ Auto segment
Segments: [gua][si][ai][tai][gi]
          ↑
          NailPosition = 0

Select "我" → NailCandidate(0)
Segments: [我][si][ai][tai][gi]
              ↑
              NailPosition = 1

Select "是" → NailCandidate(1)
...continue similarly
```

### Segmentation

```java
// Get segmentation results
GetSegmentations(inputLine, handler)

// Toggle segmentation display mode
ToggleSegmentation(inputLine)

// Get segment info
GetKeySections(inputLine, pointer, vocType, handler)
```

**KeySectionsModel:**

```java
class KeySectionsModel {
    String composedCharacters;   // Confirmed part (e.g., "我是")
    String composingCharacters;  // Currently composing (e.g., "aitaigi")
}
```

---

## Candidate Model

### CandidateModel

```java
class CandidateModel {
    String mode;           // "hant" | "tailo"
    long position;         // Corresponding input position
    long sourceID;         // Source ID
    int spanUnits;         // Span units (important!)
    List<String> tailos;   // Multiple romanization readings
    boolean virtual;       // Whether dynamically generated
    String vocabulary;     // Vocabulary content
    double weight;         // Weight score
    int words;             // Word count
}
```

**spanUnits Explanation:**

```
Input: "taigi"
Candidate: "台語" (spanUnits=2) → Corresponds to [tai][gi] two segments
Candidate: "台" (spanUnits=1)   → Only corresponds to [tai] one segment
```

### VocType (Vocabulary Type)

```java
VT_HANT    // Hanzi
VT_TAILO   // Romanization
VT_MIXED   // Mixed (Han-lo parallel)
```

---

## Composition Mode

### CompositionMode

```java
CM_STANDARD  // Standard mode (Hanzi priority)
CM_EAZY      // Easy mode
CM_TAILO     // Pure romanization mode
```

---

## Learning Mechanism

### User Vocabulary vs Learned Vocabulary

```java
// User manually added words
AddUserVoc(database, key, value, score) → UserVocError
UpdateUserVoc(database, key, value, oldKey, oldValue)
EraseUserVoc(database, key, value)
GetUserVocs(database, prefix, ordering, desc, offset, limit, handler)
ClearUserVocs(database)

// System auto-learned
EraseLearnedVoc(database, key, value)
```

### LearnThreshold

```java
MAX_LEARNED_RULES          // Max learned rules
MAX_LEARNED_VOC_CANDIS     // Max learned vocabulary candidates
MAX_LEARNED_WORD_CANDIS    // Max learned word candidates
RIPE_RULE_APPROVALS        // Rule maturity approval count
RIPE_VOC_CANDI_APPROVALS   // Vocabulary candidate maturity approvals
RIPE_WORD_CANDI_APPROVALS  // Word candidate maturity approvals
```

---

## Related Choices

```java
// Get related choices from InputLine
GetCountOfRelatedChoices(inputLine, vocType) → long
GetRelatedChoices(inputLine, vocType, offset, limit, handler)
InsertRelatedChoice(inputLine, vocType, index) → boolean

// Get related choices from specific vocabulary
CreateRelatedChoicesFromVoc(database, key, value) → RelatedCache
GetCountOfRelatedChoicesFromVoc(cache, vocType) → long
GetRelatedChoicesFromVoc(cache, vocType, offset, limit, handler)
DestroyRelatedChoicesFromVoc(cache)
```

---

## Other Features

### Input Options

```java
EnableAutoAddSpuriousSpaces(inputLine, enabled)  // Auto add spaces
EnableFullShapeSymbols(inputLine, enabled)       // Full-width symbols
EnableIslandDoctrine(inputLine, enabled)         // Island doctrine (word boundaries)
EnableRawKeyOnly(inputLine, enabled)             // Raw input only
SetInputLineLimit(inputLine, limit)              // Input length limit
```

### Hanzi to Romanization

```java
Voc2tailos(database, hanzi, handler)  // "我" → ["gua2", "ngoo2"]
```

---

## Segmentation Algorithm Inference

Based on API design, MOE's segmentation algorithm may use:

### 1. Greedy Longest Match

```
Input: "guasiaitaigi"
Steps:
1. Try longest match from start: gua (✓)
2. Remaining "siaitaigi", match: si (✓)
3. Remaining "aitaigi", match: ai (✓)
4. Remaining "taigi", match: tai (✓) or taigi (✓✓)
5. Result: [gua][si][ai][taigi] or [gua][si][ai][tai][gi]
```

### 2. Trie Prefix Query

```
tailo.tab contains valid syllable table:
a, ai, aih, ain, ...
b, ba, bah, bai, ...
g, ga, gau, gi, gu, gua, ...

Query Trie to find all possible split points during segmentation
```

### 3. Dynamic Programming (Possible)

If optimal segmentation needed, may use Viterbi or similar:

```
Input: "taigi"
Possible splits:
- [tai][gi] → score = P(tai) × P(gi)
- [ta][i][gi] → score = P(ta) × P(i) × P(gi)
- [taigi] → score = P(taigi) (if exists)

Choose highest scoring split
```

---

## Comparison with This Project

| Item | MOE Taigi | This Project |
|------|-----------|--------------|
| Core language | C++ (closed source) | Swift / Kotlin |
| Data storage | Binary Trie | MARISA-trie + SQLite |
| Segmentation timing | Real-time (during input) | At query time |
| Segmentation display | Visual separation | None |
| Selection unit | Segment (Nail) | Whole |
| Cursor control | Full API | Basic |
| Learning mechanism | User + Learned separate | Merged |
| Romanization system | TL only | TL + POJ |
| Vocabulary type | Hanzi/Roman/Mixed | Hanzi or Roman |

---

## Segmentation Algorithm Comparison: Khiin vs MOE

### Overview

| Item | Khiin | MOE Taigi |
|------|-------|-----------|
| **Algorithm** | DP dynamic programming (open source) | Unknown (closed source) |
| **Core language** | Rust | C++ |
| **Syllable validation** | SyllableTrie + Regex | Built into binary trie |
| **Cost function** | Frequency + char count + syllable count | Unknown |
| **Selection flow** | Output best result at once | Nail segment-by-segment |

### Khiin: DP Dynamic Programming

**Cost Function:**
```rust
cost = ln(1.0 / frequency^FREQUENCY_BIAS)
cost = cost / letter_bias * syllable_bias
```

**Flow:**
```
Input: "goabehchiahpng"
      ↓
DP find minimum cost split
      ↓
Result: ["goa", "beh", "chiah", "png"]
      ↓
Query: "我欲食飯" (output at once)
```

**Characteristics:**
- Global optimization (considers entire input)
- Customizable preferences (frequency vs long word priority)
- Transparent algorithm, adjustable
- High automation

### MOE: Nail Segment-by-segment

**Flow:**
```
Input: "goabehchiahpng"
      ↓
Segment: [goa][beh][chiah][png]
         ↑ NailPos=0

Show "goa" candidates: 我、餓、牙...
Select "我" → NailCandidate(0)
      ↓
      [我][beh][chiah][png]
          ↑ NailPos=1
...select segment by segment
```

**Characteristics:**
- User controls each segment selection
- Good for languages with many homophones
- Slower input but precise

### Comparison Summary

| Aspect | Khiin (DP) | MOE (Nail) |
|--------|-----------|------------|
| Automation | High | Low |
| User control | Low | High |
| Error correction | Need to reselect candidate | Only change specific segment |
| Input efficiency | Fast | Slow |
| Implementation complexity | Medium (algorithm clear) | High (needs UI support) |
| Use case | Common vocabulary | Proper nouns, homophones |

### Recommendation for This Project

**Keep it simple, choose Khiin's DP approach**

Reasons:
1. **Transparent algorithm** - Open source reference, easy to understand and maintain
2. **Simple implementation** - No complex Nail UI interaction needed
3. **Efficiency first** - Output results at once, matches common usage
4. **Architecture compatible** - Fits well with existing MARISA-trie + SQLite architecture

For more precise control in the future, can use:
- Candidate list alternative selection
- User frequency learning auto-adjustment

Not recommended to implement MOE's Nail mechanism because:
- Increases UI complexity
- Reduces input efficiency
- Too different from existing architecture

---

## Implementation Suggestions for This Project

### Core Principle: Keep It Simple

Choose Khiin's DP segmentation approach, don't implement MOE's Nail mechanism.

### Features Worth Referencing

| Priority | Feature | Description |
|----------|---------|-------------|
| High | Multiple readings support | `List<String> tailos` instead of single romanization |
| High | DP segmentation | Reference Khiin's dynamic programming algorithm |
| Medium | Han-lo mixed candidates | Show both hanzi and romanization in candidate list |
| Medium | User / Learned separation | Separate manual additions from auto-learning |
| Low | spanUnits tracking | Candidate corresponds to input range (if needed) |

### Not Recommended

| Feature | Reason |
|---------|--------|
| Nail segment-by-segment | UI complex, reduces efficiency, architecture too different |
| Real-time segmentation display | Adds complexity, limited benefit |
| Full cursor control API | Over-engineering, regular users don't need |

---

## Related Files

| File | Description |
|------|-------------|
| `references/moe_taigi_apk/decompiled/sources/moe/taigi/Tailo.java` | SWIG Wrapper |
| `references/moe_taigi_apk/decompiled/sources/moe/taigi/TailoJNI.java` | JNI method declarations |
| `references/moe_taigi_apk/decompiled/sources/android/.../CandidateModel.java` | Candidate model |
| `references/moe_taigi_apk/decompiled/sources/android/.../KeySectionsModel.java` | Segment model |
| `references/moe_taigi_apk/extracted/assets/tailo.tab` | Binary dictionary |

---

## References

- MOE Taigi IME APK decompilation analysis
- SWIG (Simplified Wrapper and Interface Generator) documentation
