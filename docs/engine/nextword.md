# NextWord Prediction

> **Type**: Feature
> **Keywords**: `NextWord`, `Bigram`, `WordAssociation`, `UserLearning`, `nextword crate`
> **Related**: continuous-candidate-display.md, sort.md, architecture/nextword-engine-boundary.md, binary-format.md

---

## Summary

- Predicts next possible word after a candidate commit.
- Hybrid: dictionary bigram (`association.bin`, read-only mmap, byte-identical cross-platform) + user learning (`user_association.db`, SQLite, engine-owned in `engine/userdata`).
- Decay-weighted scoring with generation-tagged async queries.
- User associations capped at 50,000 entries (engine-side enforcement, `UserAssociationStore::shipped_capacity`).
- Engine state machine + scoring lives in Rust `engine/nextword` (since v3.5.5 / PR #198).

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

**User learning** — `user_association.db` (SQLite, writable, schema v6)

```sql
CREATE TABLE user_association (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prev_word TEXT NOT NULL,   -- Previous word (full word, hanzi)
    prev_tl TEXT DEFAULT '',   -- Previous word TL (added in v4)
    next_word TEXT NOT NULL,   -- Next word (full word, hanzi)
    next_tl TEXT DEFAULT '',   -- Next word TL
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(prev_word, prev_tl, next_word, next_tl)
);
CREATE INDEX idx_user_prev_word_tl ON user_association(prev_word, prev_tl);
```

**Migration history**:
- v0 → v3: `UNIQUE(prev_word, next_word)` → `UNIQUE(prev_word, next_word, next_tl)` (drop + recreate)
- v3 → v4: `ALTER TABLE ... ADD COLUMN prev_tl TEXT DEFAULT ''` + new index
- v4 → v5: dropped the redundant single-column `idx_user_prev_word`
- **→ v6**: `prev_tl` joins the UNIQUE key, so the two readings of a polyphonic previous word stay separate observations (`behavioral-invariants.md` §24). One convergent rebuild replaces the per-version ladder (today the engine takeover in `engine/userdata/src/association.rs`): create-new / copy preserving `id` / drop / rename, reading `pragma_table_info` for the columns actually present rather than inferring them from the version stamp.

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

Authoritative scoring lives in Rust `engine/nextword/src/scorer.rs`. The decay constants and per-source weights are pinned by `architecture/behavioral-invariants.md` §7-§8 and the boundary doc `architecture/nextword-engine-boundary.md` §13.3.

| Source | Score component |
|--------|----------------|
| User association (`user_association.db`) | `count × USER_WEIGHT × decay(ageHours)` plus `LEARNING_BONUS` for fresh learns |
| Dictionary bigram (`association.bin`) | `count × DICT_WEIGHT` |
| Decay model | RIME-style exponential, ~one-week half-life (configurable in `scorer.rs`) |

Refer to `engine/nextword/src/scorer.rs` for the exact constants — they are the single source of truth and may evolve between releases.

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

## Ownership

Engine state machine + decision tables + scoring all live in Rust `engine/nextword`. The engine also reads `user_association.db` for `PredictNext` and writes the bigrams a decision records (`nextword::Handled.associations`, persisted in `engine/dispatch/src/user_data.rs`). Platform side handles timer / threading.

| Component | Location |
|-----------|----------|
| Persisted state (`last_selected_word`, `last_selection_time_ms`, `is_showing`, `current_generation`) | Rust `nextword::api::PersistedState` |
| Intent set (`WordSelected`, `Backspace`, `ContextTimeoutFired`, `ClearForNewComposing`, `ResetFull`, `UpdateLastSelectedWord`, `SetIsShowing`) | Rust `nextword::api::Intent` |
| Decide / filter / score | Rust `engine/nextword/src/{decide,filter,scorer}.rs` |
| Generation guard (drops stale async results) | Rust `nextword::PersistedState.current_generation` |
| Bigram source (read-only) | `association.bin` via Rust `engine/lexicon::assoc_lookup` |
| User association source | `user_association.db` via Rust `userdata::UserAssociationStore` (`rows_following` read, `record` write), wired in `engine/dispatch/src/user_data.rs` `handle_nextword` |
| iOS bridge | `Engine/RustEngineBridge+NextWord.swift` |
| Android bridge | `engine/RustEngineBridge.kt` |
| iOS platform executor | `NextWord/NextWordController.swift` (Timer, DispatchQueue.main, @MainActor) |
| Android platform executor | `ime/text/smartbar/NextWordController.kt` |

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

| | librime-predict | TaigiKeyboard |
|--|-----------------|----------------|
| Storage | DoubleArray Trie (mmap, read-only) | Burntsushi fst + binary mmap (dict) + SQLite (user) |
| User learning | None | Yes |
| Sentence-start | `$` symbol | Not yet |
| Iteration limit | `max_iterations` config | Not yet (timeout only) |
