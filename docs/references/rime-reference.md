# librime Reference Research

> **Type**: Reference
> **Keywords**: `librime`, `RIME`, `Pipeline`, `DAG`, `SpellingAlgebra`
> **Related**: khiin-reference.md, azookey-reference.md

---

## Summary

- Modular Pipeline architecture: Processor → Segmentor → Translator → Filter
- DAG segmentation algorithm for continuous input
- Spelling Algebra for rule-based spelling variants
- Dynamic weight + time decay for user learning
- YAML configuration-driven design

---

## Architecture Overview

### Pipeline Processing Flow

```
Key input
    ↓
Processors   Handle key events (Speller, Selector, Punctuator)
    ↓
Segmentors   Segment input (AbcSegmentor, PunctSegmentor)
    ↓
Translators  Generate candidates (ScriptTranslator, TableTranslator)
    ↓
Filters      Filter/sort (Uniquifier, Simplifier)
    ↓
Candidate list
```

### Core Classes

| Class | Responsibility |
|-------|----------------|
| Engine | Orchestrator, manages Pipeline components |
| Context | Context, stores input, candidates, state |
| Schema | Configuration, defines IME behavior |

---

## User Dictionary and Sorting

### Data Structure

```cpp
struct UserDbValue {
    int commits = 0;      // Commit count
    double dee = 0.0;     // Dynamic weight factor
    TickCount tick = 0;   // Timestamp
};
```

### Dynamic Weight Algorithm

```cpp
// Time decay formula
inline double formula_d(double d, double t, double da, double ta) {
    return d + da * exp((ta - t) / 200);
}
```

### Candidate Sorting

| Priority | Description |
|----------|-------------|
| 1 | Exact match > Predictive match |
| 2 | User vocabulary +0.5 bonus |
| 3 | Complete input > Autocomplete (-1 penalty) |
| 4 | Sort by quality value |

### Storage

- LevelDB key-value database
- Key: `{code}\t{phrase}`
- Value: `c={commits} d={dee} t={tick}`

---

## Configuration System

### Schema Structure

```yaml
schema:
  schema_id: luna_pinyin
  name: Luna Pinyin

engine:
  processors: [speller, selector, punctuator]
  segmentors: [abc_segmentor, punct_segmentor]
  translators: [script_translator]
  filters: [uniquifier]

speller:
  alphabet: zyxwvutsrqponmlkjihgfedcba
  algebra:
    - derive/^([nl])ve$/$1ue/
```

### Configuration Directives

| Directive | Function |
|-----------|----------|
| `__include` | Include other configs |
| `__patch` | Patch configs |
| `__append` | Append to list |
| `__merge` | Merge tree structures |

### User Customization

`.custom.yaml` files override original configs:

```yaml
# luna_pinyin.custom.yaml
patch:
  menu/page_size: 9
  speller/algebra/+:
    - derive/^([jqxy])u/$1v/
```

---

## Spelling and Segmentation

### Spelling Algebra

| Type | Syntax | Function | Penalty |
|------|--------|----------|---------|
| `xform` | `xform/pattern/replacement/` | Regex transform | None |
| `derive` | `derive/pattern/replacement/` | Derive variant | None |
| `fuzz` | `fuzz/pattern/replacement/` | Fuzzy phonetics | -0.693 |
| `abbrev` | `abbrev/pattern/replacement/` | Abbreviation | -0.693 |
| `erase` | `erase/pattern/` | Delete | - |

### DAG Segmentation

Segmentation graph for input `xian`:

```
Position:  0 ──── 1 ──── 2 ──── 3 ──── 4
           └──xi──┴──an──┘
           └─────────xian──────────────┘
```

Possible splits: `xi + an` (Xi'an) or `xian` (先)

### Penalty Values

| Type | Penalty |
|------|---------|
| Abbreviation/Fuzzy | -0.693 |
| Autocomplete | -0.693 |
| Error correction | -4.605 |
| Ambiguity | -23.026 |

---

## Other Features

| Feature | Description |
|---------|-------------|
| OpenCC | Simplified/Traditional conversion |
| Reverse lookup | Query one IME using another |
| Punctuation | Configuration-driven punctuation |
| Chord | Multiple keys pressed simultaneously |
| Sentence | Poet + Grammar for best sentence |
| History | Record recent input |

---

## Taigi Keyboard Application Suggestions

### P0 - Immediately Usable

**1. Time Decay Weight**
```kotlin
fun calculateWeight(count: Int, lastUsedMs: Long): Double {
    val ageHours = (System.currentTimeMillis() - lastUsedMs) / 3600000.0
    val decay = exp(-ageHours / 168.0)  // One week half-life
    return count * decay
}
```

**2. User Vocabulary Fixed Bonus**
```kotlin
// Change to addition instead of multiplication
val finalScore = dictScore + (if (isUserPhrase) USER_BONUS else 0)
```

### P1 - Short-term Implementation

| Feature | Description |
|---------|-------------|
| History | Record recent N words, quick recall |
| Undo input | Undo wrong selection within 3 seconds |
| Reverse lookup | Hanzi to romanization, assist learning |

### P2 - Mid-term Planning

| Feature | Description |
|---------|-------------|
| POJ/TL unified | Either romanization finds the word |
| Fuzzy phonetics | Handle n/l, in/ing confusion |
| Continuous segmentation | Support spaceless continuous input |

### P3 - Long-term Reference

| Feature | Description |
|---------|-------------|
| Pipeline | Modularize processing flow |
| Configuration-driven | Define IME behavior with YAML |

---

## Implementation Priority

```
Phase 1 (NextWord optimization)
├── Time decay weight
├── User vocabulary bonus
└── History

Phase 2 (Input experience)
├── Undo function
├── Reverse lookup
└── POJ/TL unified

Phase 3 (Advanced features)
├── Fuzzy phonetics
└── Continuous segmentation

Phase 4 (Architecture upgrade)
├── Pipeline architecture
└── Configuration-driven
```

---

## Comparison with This Project

| Item | librime | TaigiKeyboard |
|------|---------|---------------|
| Storage | LevelDB | SQLite |
| Weight | Dynamic weight + time decay | count counter |
| Segmentation | DAG + Viterbi | Single word search |
| Variants | Spelling Algebra | Manually create multiple entries |
| Extension | Plugin system | Directly modify code |

---

## librime Source Code Reference

| Directory | Content |
|-----------|---------|
| `src/rime/algo/` | Algorithms (dynamics, syllabifier, algebra) |
| `src/rime/dict/` | Dictionary (user_dictionary, prism, table) |
| `src/rime/gear/` | Components (processors, translators, filters) |
| `src/rime/config/` | Configuration (config_compiler, config_types) |
