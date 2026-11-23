# NextWord Prediction

> **Type**: Feature
> **Keywords**: `NextWord`, `Bigram`, `user_association`, `PredictNext`, `nextword crate`
> **Related**: continuous-candidate-display.md, continuous-input-ranking.md §3.2, architecture/nextword-engine-boundary.md, binary-format.md

---

## Summary

- Predicts the next word after a candidate commit.
- Hybrid: dictionary bigram (`association.bin`, read-only mmap, byte-identical cross-platform) + user learning (`user_association.db`, SQLite, engine-owned in `engine/userdata`).
- Decay-weighted scoring with generation-tagged async queries.
- User associations capped at 50,000 entries (engine-side enforcement, `UserAssociationStore::shipped_capacity`).
- Engine state machine + scoring live in Rust `engine/nextword`.

---

## Data Model

### Hybrid Bigram

| Source | Query key | Description |
|--------|-----------|-------------|
| Dictionary | Word key `hanji\u{1}tl` of the committed word; when the TL is empty or the word key has no rows under the source mask, the character key of its last character | Cold start; word → character backoff, never a merge (`engine/lexicon/src/search.rs` `lookup_associations`, `behavioral-invariants.md` §24) |
| User | Full committed word (Hanji), rows whose `prev_tl` matches first, then `prev_tl = ''`, then other readings | After learning (`UserAssociationStore::rows_following`) |

### Storage

**Dictionary associations** — `association.bin` (binary mmap, read-only)
- Sorted key table with binary search by key (word key or single character)
- Entries per key sorted by count DESC
- Fields: bitmask (u16), count (u32), next_word, next_tl
- Bitmask filter replaces SQL WHERE for dictionary source filtering

**User learning** — `user_association.db` (SQLite, writable, schema v7, `engine/userdata/src/association.rs`)

```sql
CREATE TABLE user_association (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prev_word TEXT NOT NULL,   -- Previous word (full word, hanzi)
    prev_tl TEXT DEFAULT '',   -- Previous word TL
    next_word TEXT NOT NULL,   -- Next word (full word, hanzi)
    next_tl TEXT DEFAULT '',   -- Next word TL
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(prev_word, prev_tl, next_word, next_tl)
);
CREATE INDEX idx_user_prev_word_tl ON user_association(prev_word, prev_tl);
```

`prev_tl` is part of the UNIQUE key so the two readings of a polyphonic previous word stay separate observations (`behavioral-invariants.md` §24). Older schemas are upgraded by one convergent rebuild in `association.rs` that reads `pragma_table_info` for the columns actually present.

---

## Core Flow

```
User selects candidate
    ↓
Record association (if previous word exists and interval < 10 seconds, `decide.rs` ASSOCIATION_TIMEOUT_MS)
    ↓
Query predictions
  ├─ Dictionary: association.bin lookup (word key → last-character backoff) + bitmask filter
  └─ User: user_association.db SQL query (prev_word = full word)
    ↓
Time decay calculation
    ↓
Merge and sort, display candidates
```

The same committed word also re-ranks the continuous-input candidate list through `context_rank` (`behavioral-invariants.md` §56): same tables, user rows over bundled, never the segmentation.

---

## Weight Calculation

Authoritative scoring lives in Rust `engine/nextword/src/scorer.rs`. The decay constants and per-source weights are pinned by `architecture/behavioral-invariants.md` §7-§8 and the boundary doc `architecture/nextword-engine-boundary.md` §13.3.

| Source | Score component |
|--------|----------------|
| User association (`user_association.db`) | `count × USER_WEIGHT × decay(ageHours)` plus `LEARNING_BONUS` for fresh learns |
| Dictionary bigram (`association.bin`) | `count × DICT_WEIGHT`, summed per prediction and capped at `DICT_SCORE_CAP` (250) — below the lowest possible user score, so every learned prediction outranks every bundled-only one |
| Decay model | RIME-style exponential, one-week half-life |

`scorer.rs` holds the exact constants.

---

## Triggers and Reset

### Trigger Conditions

| Action | Hanzi mode | Romanization mode |
|--------|------------|-------------------|
| Tap candidate | Trigger | Trigger |
| Enter confirm | No trigger | Trigger |

### Context Reset

- Input sentence-ending punctuation (`。！？.!?`) or a clause mark (`，、；：,;:`)
- Long idle time (> 30 seconds, `decide.rs` CONTEXT_TIMEOUT_MS)
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

Engine state machine + decision tables + scoring all live in Rust `engine/nextword`. The engine also reads `user_association.db` for `PredictNext` and writes the bigrams a decision records (`nextword::Handled.associations`, persisted in `engine/dispatch/src/user_data/with_stores.rs`). Platform side handles timer / threading.

| Component | Location |
|-----------|----------|
| Persisted state (`last_selected_word`, `last_selection_time_ms`, `predictions_visible`, `current_generation`) | Rust `nextword::api::PersistedState` |
| Intent set (`WordSelected`, `Backspace`, `ContextTimeoutFired`, `ClearForNewComposing`, `ResetAll`, `SetPredictionsVisible`) | Rust `nextword::api::Intent` |
| Decide / filter / score | Rust `engine/nextword/src/{decide,filter,scorer}.rs` |
| Generation guard (drops stale async results) | Rust `nextword::PersistedState.current_generation` |
| Bigram source (read-only) | `association.bin` via Rust `engine/lexicon::lookup_associations` |
| User association source | `user_association.db` via Rust `userdata::UserAssociationStore` (`rows_following` read, `record` write), wired in `engine/dispatch/src/user_data/with_stores.rs` `handle_nextword` |
| iOS bridge | `Engine/RustEngineBridge+NextWord.swift` |
| Android bridge | `engine/NextWordBridge.kt` |
| iOS platform executor | `NextWord/NextWordController.swift` (Timer, DispatchQueue.main, @MainActor) |
| Android platform executor | `ime/text/nextword/NextWordController.kt` |
