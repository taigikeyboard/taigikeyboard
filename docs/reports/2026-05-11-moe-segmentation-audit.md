# MOE Tâi-gí 斷詞架構/流程對照 Audit

> **Type**: Architecture audit (read-only, no code change)
> **Scope**: Compare Taigi Keyboard's segmentation (syllabifier + dispatch) against MOE Tâi-gí's `tutgInputLine` model
> **Method**: SWIG-decompiled Java surface (`Tailo.java` + Kotlin call sites) — native `librime_taigi`-equivalent `.so` is **not** in scope (out-of-scope; would require IDA/Ghidra). Conclusions are bounded by the FFI signatures + observable data shapes.
> **Sources**: `references/moe_taigi_apk/decompiled/sources/`, `references/moe_taigi_apk/extracted/assets/`
> **Authored**: 2026-05-11

---

## TL;DR

| Dimension | Verdict | Notes |
|---|---|---|
| 1. Segmentation location (engine vs platform) | ✓ Aligned | Both run inside the engine — no platform syllabifier |
| 2. Segmentation unit (nail vs ending) | △ Same concept, different lifecycle |
| 3. Algorithm | △ Inventory-driven on both sides; full algo black-boxed in MOE |
| 4. Multi-cut at one position | △ Equivalent at candidate granularity (`spanUnits`) |
| 5. POJ diacritic normalisation | ✗ Not yet aligned (we defer; MOE pushes one char at a time) |
| 6. Hyphen handling | ✗ Not yet aligned (same root cause as #5) |
| 7. Partial-prefix candidates inside engine | ✗ Not yet aligned (we fall through to lexicon; MOE always engine) |

**Bottom line**: Our high-level posture matches MOE — engine owns segmentation, candidate fetch is keyed off the engine's segmentation state, the platform never re-cuts. The remaining gaps are exactly the items already on the Phase 9 plan (9.4a tone-1, 9.4b hyphen, 9.4c POJ canonicalisation, §15.3.D partial-prefix). **No architectural surprise.**

The one design difference that won't close without an explicit decision: **state lifecycle.** MOE holds a long-lived `tutgInputLine` C++ object that persists across keystrokes; we re-derive segmentation from `Phase::Continuous { raw }` on every `FetchAtPos`. Functionally equivalent today, but the trade-off is documented in §6 below for future ranking/learning features.

---

## 1. How MOE Segments — Mechanism Walkthrough

### 1.1 The `tutgInputLine` object

MOE wraps a stateful C++ object that the JVM holds an opaque handle to:

- `Tailo.CreateInputLine(database) → SWIGTYPE_p_T_tutgInputLine` (`moe/taigi/Tailo.java:21-27`)
- `Tailo.DestroyInputLine(inputLine)` (`moe/taigi/Tailo.java:37-39`)
- `Tailo.ClearInputLine(inputLine)` (`moe/taigi/Tailo.java:9-11`)
- `Tailo.SetInputLineLimit(inputLine, long)` (`moe/taigi/Tailo.java:221-223`)

The Kotlin side stores it in `D4.x.f2059p` (referenced from `J/w.java:49`, `J/n.java:62`, `J/o.java:49`, `J/p.java:54`, `J/q.java:42`). One instance per IME session; **MOE never rebuilds it per keystroke.**

### 1.2 Char-by-char ingestion (no platform syllabifier)

The Kotlin layer pushes one `char` at a time:

```java
// references/moe_taigi_apk/decompiled/sources/J/w.java:46-53
Iterator it = this.f12642q.iterator();
while (it.hasNext()) {
    int iIntValue = ((Number) it.next()).intValue();
    SWIGTYPE_p_T_tutgInputLine sWIGTYPE_p_T_tutgInputLine = (SWIGTYPE_p_T_tutgInputLine) xVar.f2059p;
    if (sWIGTYPE_p_T_tutgInputLine != null) {
        Tailo.InsertKey(sWIGTYPE_p_T_tutgInputLine, (char) iIntValue);
    }
}
```

`Tailo.InsertKey(inputLine, char)` (`moe/taigi/Tailo.java:141-143`) is the **only** input ingestion API. The Kotlin/Java side has zero syllable knowledge — every segmentation decision is delegated to `inputLine`'s C++ implementation.

Editing operations are also char-granular, not segment-granular:
- `MoveBack / MoveForward / MoveHome / MoveEnd / MoveTo(long)` — caret motion (`Tailo.java:165-183`)
- `RemoveKey(inputLine, Direction)` — backspace (`Tailo.java:205-207`)
- `PopFront(inputLine) → int` — drop the leftmost token when the line overflows the configured limit (`Tailo.java:197-199`)

### 1.3 Two cursors: caret and nail

MOE distinguishes the *edit* cursor from the *commit* cursor:

| Concept | API | Role |
|---|---|---|
| Caret | `GetCaretPosition` / `MoveTo` / `MoveBack` / `MoveForward` (`Tailo.java:81-83, 165-183`) | Where the user is editing inside the pending composition |
| Nail | `GetNailPosition` (`Tailo.java:117-119`) | Boundary between committed segments and the still-pending tail |

Kotlin reads the nail as a plain `long` (`J/q.java:43`: `(int) Tailo.GetNailPosition(...)`) and uses it as an opaque address — never to slice raw bytes. **Committing happens by ID, not by offset:**

```java
// references/moe_taigi_apk/decompiled/sources/moe/taigi/Tailo.java:185-187
public static boolean NailCandidate(SWIGTYPE_p_T_tutgInputLine inputLine,
                                    VocType vocType, long candidateId) { ... }
```

The candidate ID is the index into the result of `GetCandidatesAtNailPos`. The engine advances the nail internally. **Caller never says "consume N bytes."**

### 1.4 Reading state — three orthogonal queries

| Query | Returns | Purpose |
|---|---|---|
| `GetCandidatesAtNailPos(inputLine, vocType, offset, count, handler)` (`Tailo.java:77-79`) | Paged list of `CandidateModel` | Candidates at the *current* nail position only |
| `GetConfirmedCandidates(inputLine, handler)` (`Tailo.java:89-91`) | List of already-nailed segments | Used by `J/o.java:51` to render committed history |
| `GetKeySections(inputLine, longArray, 1, handler)` (`Tailo.java:113-115`) | `KeySectionsModel { composedCharacters, composingCharacters }` | Splits the raw key stream by segment so the UI can render committed-vs-pending differently (`J/p.java:55-62`) |

`GetSegmentaions(inputLine, handler)` (`Tailo.java:129-131` — note the engine-side typo) is the diagnostic full-segmentation dump, presumably returning every nail boundary in one shot.

### 1.5 Two mode axes (orthogonal, both engine-side)

MOE separates **how to interpret input** from **how to filter output**:

```java
// references/moe_taigi_apk/decompiled/sources/moe/taigi/CompositioMode.java:5-7
public static final CompositioMode CM_EAZY;
public static final CompositioMode CM_STANDARD;
public static final CompositioMode CM_TAILO;
```

```java
// references/moe_taigi_apk/decompiled/sources/moe/taigi/VocType.java:5-7
public static final VocType VT_HANT;
public static final VocType VT_MIXED;
public static final VocType VT_TAILO;
```

- `CompositioMode` is set per-`inputLine` via `SetCompositionMode` (`Tailo.java:213-215`) — it changes how raw keys are syllabified (`CM_TAILO` = canonical TL, `CM_EAZY` = relaxed, `CM_STANDARD` = default).
- `VocType` is passed per-query to `GetCandidatesAtNailPos` / `NailCandidate` / `GetCandidateCountAtNailPos` (`Tailo.java:73, 77, 185`) — same segmentation, different output filter.

Both live inside the engine. Platform code only flips switches.

### 1.6 Tunable behaviour knobs

Each toggle is engine-side state on the `inputLine`:

| API | Likely meaning |
|---|---|
| `EnableAutoAddSpuriousSpaces` (`Tailo.java:49-51`) | Auto-insert separator on ambiguous boundary |
| `EnableFullShapeSymbols` (`Tailo.java:53-55`) | Full-width punctuation passthrough |
| `EnableIslandDoctrine` (`Tailo.java:57-59`) | "Island" lookup mode (single-segment fallback?) |
| `EnableRawKeyOnly` (`Tailo.java:61-63`) | Bypass syllabifier — pure passthrough |
| `ToggleSegmentation` (`Tailo.java:233-235`) | Flip whether segmentation runs at all |

These are visible only as setters/getters — semantics are inferred from naming and would need behavioural probing to confirm.

### 1.7 Data files shipped with the APK

```
references/moe_taigi_apk/extracted/assets/
├── tailo.tab     3.1 MB binary, magic FF FE 10 01 F3 21 04 69 — serialized vocab + lattice tables
├── cats.tab      184 B   POS-tag dictionary ("pron, adv, interj, numeral, ...")
├── emoji_all.json
└── symbol_*.json
```

`tailo.tab` is the equivalent of our `dictionary.bin` + `dictionary.fst` + `syllables.fst` rolled into one proprietary blob. Format is not self-describing; the binary header is consistent with an mmap-friendly index but reverse engineering it is out of audit scope. **The presence of a single inventory file confirms MOE's segmenter is inventory-driven** (i.e. uses a fixed list of legal TL syllables), same family as our `SyllableInventory`.

### 1.8 Candidate output shape

```java
// references/moe_taigi_apk/decompiled/sources/android/moe/taiwanese/taigi/data/local/model/CandidateModel.java:18-29
public final class CandidateModel {
    public static final String MODE_HANT = "hant";
    public static final String MODE_TAILO = "tailo";
    private final String mode;          // "hant" | "tailo"
    private final long position;
    private final long sourceID;
    private final int spanUnits;        // ← syllable count consumed by this candidate
    private final List<String> tailos;  // ← per-syllable TL romanisation
    private final boolean virtual;
    private final String vocabulary;    // ← display string (hanji or romanisation)
    private final double weight;
    private final int words;
}
```

`spanUnits` is the candidate-level multi-cut indicator. Two candidates at the same `position` can have different `spanUnits` — that is how MOE surfaces e.g. `tsua` → 紙 (1) vs 珠仔 (2).

---

## 2. How We Segment — Mechanism Walkthrough

### 2.1 Stateless syllabifier, called per fetch

```rust
// engine/composing/src/syllabifier/tl.rs:49-84
pub fn valid_span_endings(input: &str, pos: usize,
                          inv: &SyllableInventory, max_syllables: usize)
    -> Vec<usize> { /* BFS, depth-capped, returns every reachable ending */ }
```

```rust
// engine/composing/src/syllabifier/tps.rs:65-93
pub fn valid_span_endings(input: &str, pos: usize) -> Vec<usize> { /* O(n) scan */ }
```

Both return ending byte offsets. They hold no state across calls. The state lives in `Phase::Continuous`:

```rust
// engine/composing/src/api.rs:28-32
Continuous {
    raw: String,
    committed: Vec<CommittedSegment>,
}
```

Each `FetchAtPos` (= MOE's `GetCandidatesAtNailPos`) takes a fresh snapshot of `raw`, re-runs the syllabifier, and rebuilds FST keys (`engine/composing/src/dispatch.rs:127-174`).

### 2.2 Key construction (deferred normalisation)

`build_keys_tl` (`engine/composing/src/dispatch.rs:208-234`) and `build_keys_tps` (`engine/composing/src/dispatch.rs:278+`) emit `(consumed_span, "tl:<fused-toneless>")` pairs. Both currently **strip ASCII tone digits only** — POJ diacritics and hyphens are explicitly deferred (cited in the source as Phase-6 limitations, lines 192-204 and 261-274).

### 2.3 Commit is offset-driven, not ID-driven

```rust
// engine/composing/src/api.rs:149-153
CommitContinuous {
    display_text: String,
    consumed_bytes: usize,
    syllable_count: u8,
}
```

The platform tells the engine *how many bytes to eat*. The engine slices `raw[..consumed_bytes]` into a new `CommittedSegment` and keeps `raw[consumed_bytes..]` as the new pending tail (`engine/composing/src/transition.rs:484-490+`). This is the inverse of MOE's `NailCandidate(candidateId)` — but both end up advancing a nail.

### 2.4 Candidate output shape

```rust
// engine/lexicon/src/continuous.rs (RawCandidate + record_to_candidate path)
pub struct RawCandidate {
    pub display_text: String,        // hanji ?? tl  ← roman dropped here
    pub consumed_span: (u32, u32),
    pub syllable_count: u8,
    pub source_tier: u8,
    pub adjusted_score: f32,
    pub user_freq_boost: f32,
    pub recency_rank: u8,
}
```

Compared to MOE's `CandidateModel`, we are **missing the roman field on the wire** — this is the v3.5.8 dogfood bug that `docs/engine/continuous-candidate-display.md` §1-14 already addresses.

---

## 3. Seven-Dimension Comparison

### Dim 1 — Where does segmentation happen?  **✓ Aligned**

- **MOE**: Inside `tutgInputLine` C++ object. Kotlin pushes one char at a time (`J/w.java:51`) and never sees boundaries. `Tailo.GetKeySections` only *reports* the split; it doesn't compute it on the caller side.
- **Us**: Inside the `composing` crate's syllabifier (`engine/composing/src/syllabifier/{tl,tps}.rs`). The platform (`ios/.../RustEngineBridge.swift`, `android/.../ComposingManager.kt`) never re-cuts.

**Conclusion**: Architecturally identical posture — engine owns segmentation. The mainstream-IME pattern §15 of `continuous-candidate-display.md` already locks this in.

### Dim 2 — Segmentation unit (state lifecycle)  **△ Same concept, different lifecycle**

- **MOE**: Long-lived `tutgInputLine` holds `(buffer, caret, nail, segmentation)`. Segmentation is updated incrementally on each `InsertKey`. Calls like `GetCandidatesAtNailPos` are O(read-current-state).
- **Us**: `Phase::Continuous { raw, committed }` holds the buffer + committed segments. Segmentation is re-derived on every `FetchAtPos` from `raw` (`engine/composing/src/dispatch.rs:127-174`).

**Verdict**: △ — both ultimately address the "nail position" and never let the platform see byte offsets it didn't compute. But MOE's incremental model means the engine can persist per-segment scratch state (alternative-segmentations cache, partial lattice, etc.); ours starts from scratch each fetch.

**Cost today**: Negligible. Our buffers are bounded by `MAX_SYLLABLES = 8` (`engine/composing/src/dispatch.rs` constant) and the FST lookup dominates the syllabifier work.

**Cost if we grow features**: Lattice-based ranking, cross-segment learning, or hypothesis caching would benefit from stateful segmentation. Not on the Phase 9 roadmap.

### Dim 3 — Algorithm  **△ Same family (inventory-driven); MOE's exact algo black-boxed**

What we can prove from the surface:

| Evidence | Implication |
|---|---|
| `Tailo.GetSegmentaions` returns plural results | MOE supports multiple parallel segmentations (= lattice, not single best path) |
| `Tailo.ToggleSegmentation` exists | Segmentation can be turned off → it is a discrete pass, not a side-effect of `InsertKey` |
| `CandidateModel.spanUnits` varies per candidate | Multiple span lengths surface at the same position |
| `tailo.tab` is a single 3.1 MB blob | Inventory + vocab live together; segmenter is inventory-aware |

What we have:

- TL: BFS over `SyllableInventory` (FST-backed, 4,899 keys, `engine/lexicon/src/syllable_inventory.rs`), depth-capped at 8 syllables, returns every reachable ending (`engine/composing/src/syllabifier/tl.rs:49-84`).
- TPS: O(n) scan over Bopomofo tone-marks + entering-coda + 8th-tone dot (`engine/composing/src/syllabifier/tps.rs:65-93`).

**Verdict**: △ — both are inventory-driven and produce multi-cut results. The exact MOE algorithm (whether it is BFS, DAG lattice, or longest-match with branch retention) cannot be confirmed without reversing `librime_taigi.so`. Behaviourally, MOE's `spanUnits` variation means it surfaces equivalent multi-cut output to our BFS.

### Dim 4 — Multi-cut at one position  **△ Equivalent semantics, different encoding**

- **MOE**: `GetCandidatesAtNailPos` returns N candidates each with `spanUnits` ∈ {1, 2, 3, …}. The "cut points" are implicit in the candidate list — caller picks by ID, engine advances nail by `spanUnits`.
- **Us**: `valid_span_endings` returns `[3, 4]`; `build_keys_tl` emits a key per ending; `fetch_candidates_for_keys` returns N candidates each tagged with `consumed_span = (start, end)` (`engine/lexicon/src/continuous.rs:343-388`).

**Verdict**: △ — semantically equivalent. The user-visible behaviour ("type `tsua`, see both 紙 and 珠仔") is achievable on both architectures. Our representation is more explicit (we hand the platform the byte range; MOE hides it behind an opaque ID).

### Dim 5 — POJ diacritic normalisation  **✗ Not yet aligned**

- **MOE**: `InsertKey` accepts any `char`, so `pe̍h` enters as 3 chars and `tutgInputLine` is responsible for handling combining marks internally. The engine certainly supports POJ — `CompositioMode.CM_STANDARD` vs `CM_TAILO` is the spelling switch.
- **Us**: `build_keys_tl` does `to_ascii_lowercase` + strip ASCII digits only (`engine/composing/src/dispatch.rs:221-227`). Combining marks survive into the lookup key and miss the fused-toneless FST. `engine/phonetics::canonicalize_syllable` exists but is **not yet wired into `build_keys_tl`** — flagged as Phase 9.4c work in `project_v358_continuous_input.md`.

**Verdict**: ✗ — this is the largest functional gap. Already on the plan as Phase 9.4c.

### Dim 6 — Hyphen handling  **✗ Not yet aligned**

- **MOE**: `InsertKey` accepts `-` as a regular `char`. The engine internally decides whether to treat it as a separator or as part of the previous syllable.
- **Us**: `valid_span_endings` walks contiguous bytes via `inv.contains(...)` and the inventory has no hyphenated entries (`engine/composing/src/dispatch.rs:198-201` source comment), so syllabification halts at the first hyphen. Stripping at `build_keys_tl` would not help because the syllabifier itself is the cut-off point.

**Verdict**: ✗ — known gap. Already on the plan as Phase 9.4b (shadow buffer + offset map).

### Dim 7 — Partial-prefix candidates inside the engine  **✗ Not yet aligned**

- **MOE**: `GetCandidatesAtNailPos(offset, count)` returns whatever the engine has — including incomplete spellings — paged for the UI. No "fallback to lexicon" exists; there is only one query API.
- **Us**: When `Phase::Continuous` returns an empty `ContinuousResponse.candidates`, the platform falls through to the legacy lexicon path (`AutocompleteService.swift:309-322` lexicon branch; `TaigiAutocompleteService.kt:88-124` lexicon branch). This is the two-path architecture that `docs/engine/continuous-candidate-display.md` §15 explicitly proposes to retire.

**Verdict**: ✗ — known gap. Already on the plan as §15.3.D inside the continuous-candidate-display spec.

---

## 4. Beyond the Seven Dimensions — Other Findings

| Observation | Significance |
|---|---|
| MOE has a separate `CompositioMode` (input spelling) vs `VocType` (output filter) | We collapse both into `AppConfig.input_mode` + UI `isTranslateSwapped`. Deliberate divergence; not a defect. |
| MOE exposes `SetInputLineLimit` | Our `MAX_SYLLABLES = 8` cap (`engine/composing/src/dispatch.rs`) is the analogue, but hard-coded. |
| MOE has `Voc2tailos(database, hanzi, handler)` (`Tailo.java:245-247`) — reverse Hanji→TL lookup | We have no equivalent. Used by MOE for the Hanji-input path that we deliberately don't support. |
| MOE has `EnableIslandDoctrine` / `EnableRawKeyOnly` / `EnableAutoAddSpuriousSpaces` toggles | Our engine has none of these. Whether we need them is feature-driven, not architecture-driven. |
| MOE candidate carrier has `tailos: List<String>` per syllable + `vocabulary: String` (= our `display_text`) | This is exactly the dual-channel display we're adding in §1-14 of `docs/engine/continuous-candidate-display.md`. |
| MOE candidate has `virtual: bool` + `sourceID: long` + `weight: double` | Maps to our `source_tier` + `adjusted_score`. No mismatch. |
| MOE has user-vocab APIs (`AddUserVoc`, `EraseUserVoc`, `UpdateUserVoc`, `ClearUserVocs`, `EraseLearnedVoc`, `GetCountOfUserVocs`, `GetUserVocs` ordered by `UserVocOrdering`) | We have `user_association.db` + `custom_dictionary.db`. The shape is similar; whether the schemas align is a separate audit. |
| MOE has `GetRelatedChoices` + `InsertRelatedChoice` + `CreateRelatedChoicesFromVoc` (`Tailo.java:121-127, 145-147`) | Maps to our NextWord. Both architectures push next-word into the engine. |

---

## 5. What Is Already Aligned (Don't Touch)

1. **Engine owns segmentation.** Both architectures keep the platform stupid. The Phase II rule "Android mirrors iOS" (`rules/cross-platform-alignment.md`) is reinforced by this: there is no per-platform syllabifier to drift.
2. **Multi-cut at candidate granularity.** Our `consumed_span` ↔ MOE's `spanUnits` are semantically interchangeable.
3. **Nail concept.** Our `CommittedSegment` history ↔ MOE's `GetConfirmedCandidates`. Same data, different serialisation.
4. **Two-axis mode separation.** Our (input_mode, isTranslateSwapped) ↔ MOE's (CompositioMode, VocType). The fact that they differ in *which* axis is the keyboard-level toggle is the deliberate UX choice already locked in (`docs/engine/continuous-candidate-display.md` §11 + §15.1).
5. **Inventory-driven syllabification.** `syllables.fst` ↔ `tailo.tab` (the inventory portion).
6. **Bounded composition window.** `MAX_SYLLABLES = 8` ↔ `SetInputLineLimit`.

---

## 6. Trade-off Analysis — State Lifecycle (the one open architectural question)

We do **stateless re-derivation** on every fetch. MOE does **stateful incremental update** on every keystroke.

| Axis | Stateless (us) | Stateful (MOE) |
|---|---|---|
| Memory footprint per session | Zero overhead beyond `raw: String` + `Vec<CommittedSegment>` | One C++ object per IME session |
| Threading | Trivial — `FetchAtPos` is pure given state snapshot | Requires lock or single-thread discipline around the `inputLine` handle |
| Wire shape | Plain proto messages | Opaque handle + callbacks |
| Cost of adding lattice/learning features | Need to either persist lattice externally or restart from `raw` each time | Free — the lattice can live on the `inputLine` |
| Cost of crash recovery | Trivial — `raw` is the only source of truth | Need to checkpoint the `inputLine` |
| Cost of multi-line / async composition | Trivial | Need per-line `inputLine` instances |

**Today**: stateless is the right choice for v3.5.8. The buffer is small, the syllabifier is fast, and there's no per-segment state worth caching.

**Tomorrow**: if Phase 10+ introduces lattice-based ranking, cross-segment phrase learning, or async/predictive candidate generation, the stateless model becomes a constraint. Worth re-evaluating at that point, not before.

**No fix proposed in this audit.** This is captured here so a future redesign discussion has the trade-offs spelled out.

---

## 7. Open Questions Left for the User

1. **Are the engine-side knobs (`IslandDoctrine`, `RawKeyOnly`, `AutoAddSpuriousSpaces`, `ToggleSegmentation`) features we want?** They are not architectural primitives — they are UX toggles. Each could be added on top of our current architecture without redesign.
2. **Do we want `Voc2tailos` (Hanji→TL reverse lookup)?** Currently a non-goal per project posture (the keyboard is romanisation-in, hanji-out). Documenting in case it ever comes up.
3. **Is `SetInputLineLimit` (currently hard-coded `MAX_SYLLABLES = 8`) something to expose as a setting?** Mainstream IME convention varies.

These are *features* not *defects*. Audit recommends no action.

---

## 8. Sources Cited

### MOE side
- `references/moe_taigi_apk/decompiled/sources/moe/taigi/Tailo.java` — SWIG-generated FFI surface, 256 lines, all public methods inventoried
- `references/moe_taigi_apk/decompiled/sources/moe/taigi/CompositioMode.java:5-7`
- `references/moe_taigi_apk/decompiled/sources/moe/taigi/VocType.java:5-7`
- `references/moe_taigi_apk/decompiled/sources/J/w.java:46-53` — `InsertKey` loop
- `references/moe_taigi_apk/decompiled/sources/J/n.java:60-72` — `GetCandidatesAtNailPos` coroutine wrapper
- `references/moe_taigi_apk/decompiled/sources/J/o.java:49-54` — `GetConfirmedCandidates` wrapper
- `references/moe_taigi_apk/decompiled/sources/J/p.java:55-62` — `GetKeySections` wrapper
- `references/moe_taigi_apk/decompiled/sources/J/q.java:42-44` — `GetNailPosition` wrapper
- `references/moe_taigi_apk/decompiled/sources/android/moe/taiwanese/taigi/data/local/model/CandidateModel.java:18-29`
- `references/moe_taigi_apk/decompiled/sources/android/moe/taiwanese/taigi/data/local/model/KeySectionsModel.java:6-16`
- `references/moe_taigi_apk/extracted/assets/tailo.tab` — 3.1 MB binary, first 16 bytes inspected
- `references/moe_taigi_apk/extracted/assets/cats.tab` — 184 B POS-tag dictionary

### Our side
- `engine/composing/src/syllabifier/tl.rs:49-84` — TL BFS syllabifier
- `engine/composing/src/syllabifier/tps.rs:33-93` — TPS scan syllabifier
- `engine/composing/src/dispatch.rs:127-174` — `handle_fetch_at_pos`
- `engine/composing/src/dispatch.rs:208-234` — `build_keys_tl`
- `engine/composing/src/dispatch.rs:278+` — `build_keys_tps`
- `engine/composing/src/api.rs:20-32` — `Phase::Continuous` definition
- `engine/composing/src/api.rs:149-153` — `Intent::CommitContinuous`
- `engine/lexicon/src/syllable_inventory.rs` — `SyllableInventory`
- `engine/lexicon/src/continuous.rs:235-293` — `fetch_candidates_for_endings`
- `engine/lexicon/src/continuous.rs:343-388` — `fetch_candidates_for_keys`

### Out of scope
- Native `.so` segmentation algorithm reverse engineering (would require IDA/Ghidra on the C++ implementation inside `base.apk`).
- MOE's user-vocab schema (separate audit if needed).
