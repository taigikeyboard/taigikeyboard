# Syllabifier — Syllable Parsing & Inventory

> **Type**: Reference
> **Keywords**: `syllabifier`, `syllable`, `SyllableInventory`, `syllables.fst`, `valid_span_endings`, `phonotactics`
> **Related**: [composing.md](composing.md), [binary-format.md](binary-format.md), [tps.md](tps.md), [continuous-input-ranking.md](continuous-input-ranking.md), [behavioral-invariants.md](../architecture/behavioral-invariants.md)

---

## Summary

- Two primitives feed continuous input: a **table-based syllable parser** (`phonetics::syllable`) and a **runtime FST membership reader** (`lexicon::SyllableInventory` over `syllables.fst`).
- The composing crate's `syllabifier` walks a buffer into valid syllable spans (BFS, not longest-match) using the inventory, building the segmentation lattice.

> **Two senses of "valid syllable" — do not conflate.** `phonetics::is_valid_syllable` is a *phonotactic* table test (does this string split into a legal initial+final?). `SyllableInventory::contains_in` is *FST membership* (does this exact canonical syllable appear in the built dictionary's inventory?).

---

## Files

| File | Responsibility |
|------|----------------|
| `engine/phonetics/src/syllable.rs` | Parse primitive: tone strip, POJ→TL spelling normalize, initial/final split, per-syllable canonicalizers |
| `engine/phonetics/src/tables.rs` | `TL_INITIALS` / `TL_FINALS` / `TONE_NUM_TO_COMBINING` lookup tables |
| `engine/lexicon/src/syllable_inventory.rs` | `SyllableInventory` — mmap'd `syllables.fst` reader, per-family membership |
| `engine/composing/src/syllabifier/mod.rs` | Mode-aware dispatch `valid_span_endings_lowered_with_barriers` |
| `engine/composing/src/syllabifier/tl.rs` | BFS scanner for TL / POJ / English |
| `engine/composing/src/syllabifier/tps.rs` | BFS scanner for TPS |
| `engine/composing/src/shadow/keys.rs` | `span_min_syllable_count` (min-hop BFS) |
| `engine/build-helpers/fst-builder/src/syllables.rs`, `dictionary/build/create_syllables_fst.py` | Offline build of `syllables.fst` |

---

## 1. `syllable.rs` — parse primitive

Ported from `taigi-converter/src/phonetics.js`. Two public canonicalizers share one pipeline:

- **`canonicalize_syllable(token) -> Option<(canonical_toneless, tone_digit)>`** — `strip_tone_mark` → lowercase → `normalize_to_tl` (POJ→TL **spelling** fold) → `split_initial_final` membership gate. Emits **TL** ASCII.
- **`canonicalize_poj_syllable(token)`** — phonotactic gate via TL tables, but emits the **POJ** ASCII shape. So `chit8` → `poj:chit` while `tsit8` → `tl:tsit`.

Sub-primitives:

- `strip_tone_mark(text) -> (bare_NFC, tone_digit)` — ASCII fast-path strips a trailing `1..=9`; non-ASCII path NFD-decomposes and finds the combining mark. `0` is **not** a tone.
- `split_initial_final(text) -> Option<(initial, final)>` — accepts only when `initial ∈ TL_INITIALS` AND `final ∈ TL_FINALS` (input must be pre-lowercased + TL-normalized).
- `is_valid_syllable(token) -> bool` = `canonicalize_syllable(token).is_some()`.
- Abbreviation keys live in their own `*-abbrev:` key family (`phonetics::KeyFamily`), so a `tl:` prefix scan never meets them. Fused nasals (`tngtng`, `mngkng`) and syllabic-nasal singles (`m`, `ng`) stay ordinary syllable keys.

**Normalization tables (order is load-bearing):** `NORMALIZE_TO_TL_RULES` = `ch→ts, ou→oo, o͘→oo, ⁿ→nn, oa→ua, oe→ue, eng→ing, ek→ik, oonn→onn` (`oonn` after `oo`). `normalize_to_tl_keep_tl_finals` drops `eng→ing`/`ek→ik` to preserve real TL finals `eng`/`ek`. POJ variants: `NORMALIZE_TO_POJ_RULES` (per-syllable) + `NORMALIZE_TO_POJ_GLYPH_RULES` (whole-buffer glyph-only).

`TL_FINALS` covers the special/dialectal finals (`er`, `erh`, `erk`, `ir*`, `iri`, `irinn`, `ee`, `eeh`, `or`, …) — per `docs/phonetics/taigi-phonetics-reference.md` §3.2.6; do not prune these.

---

## 2. `SyllableInventory` — `syllables.fst`

File layout, key families and build: [binary-format.md §3.5](binary-format.md#35-syllablesfst--syllable-inventory).

| Method | Purpose |
|--------|---------|
| `open(path) -> Result<Self, LexiconError>` | mmap readonly + validate fst parse |
| `contains_in(mode, syllable) -> bool` | Membership in the family of `mode` (`Tl` / `English` → `tl:`, `Poj` → `poj:`, `Tps` → `tps:`). **Caller must pre-canonicalize** for the family |
| `contains_in_tps_readings(syllable, final_only_offsets) -> bool` | TPS: true when any reading of `syllable` under the TPS ambiguity families is in the `tps:` inventory (`INVARIANT_TPS_DEFOLD_ENUMERATE` §35); a superset of `contains_in(Tps, …)` |
| `entry_count()` / `is_empty()` | Inventory size |

The inventory does **not** compute span endings — the composing crate does, calling `contains_in`.

### Family-split rationale

About half of the dictionary's rows (~80,000 of ~159,000) have `tl_num != poj_num` (`chit8`/`tsit8`, `goa2`/`gua2`). A single TL-folded inventory loses POJ-side syllable boundaries, so a continuous POJ buffer like `chiah` could not be recognized as one valid syllable. Tagging all families into one FST keeps storage shared (prefix-tree overlap) while `contains_in(mode, …)` serves the correct family.

---

## 3. Syllabifier consumption (composing crate)

**`valid_span_endings_lowered_with_barriers(lowered, pos, inv, mode, max_syllables, barriers) -> Vec<usize>`** (`syllabifier/mod.rs`) routes `Tps` → `tps::valid_span_endings_lowered`, `Tl` / `Poj` / `English` → `tl::valid_span_endings_lowered`, so the syllabification family always matches the FST key prefix the same `mode` emits. `barriers` are stripped-separator offsets no single-syllable link may cross: TL / POJ treat a typed `-` as the user's syllable boundary (§52, `khi--ah` is never `khiah`) and never cut inside a closed one-syllable stretch (`siam-` is never `sia|m`); TPS also expands ambiguity families and marks the glyph before a barrier Final-only (§35); English ignores them.

The scanners are FIFO BFS from `pos`: for each char-boundary `end` within `MAX_SYLLABLE_BYTES` (TL 10 = max initial `tsh` + max final `uainnh` + tone digit; TPS 24), accept when the span is in the inventory AND it is not a false toneless boundary AND it is the first arrival. Returns ascending, deduped byte offsets.

- **BFS, not longest-match** — typing `tsua` must surface both `珠` (span 3) and `紙`/`珠仔` (span 4) as candidate sources; pure longest-match (khiin-rs style) would commit to `tsua` and lose `珠`.
- `is_false_toneless_boundary` suppresses a toneless match sitting immediately before a tone digit — the digit belongs to the syllable (the FST also holds the numeric form).

**`span_min_syllable_count(span, inv, mode) -> Option<usize>`** (`shadow/keys.rs`) — unweighted shortest path (fewest hops) from 0 to end, each hop one valid single syllable, so a span still syllabifiable via a non-greedy split (`ta`+`nia` when `tan`+`ia` dead-ends) is counted. `None` only when the end is not single-syllable-reachable; the caller fail-closes by dropping that lattice edge. It is the **metadata** syllable sum for the synthesized OOV slot-0 candidate, not a ranking penalty.

**Lattice** — `lattice/builder.rs::build_lattice_with_barriers` pushes every `(start, end)` edge the scanner returns (single-syllable and multi-syllable phrase ends). The display-layer longest-match suppression lives separately (`INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`, behavioral-invariants §18).

---

## 4. Runtime

`lexicon::EngineHandle::install` opens `syllables.fst` via `SyllableInventory::open` (optional path, `InstallRequest.syllable_inventory_path`) and stores it on the engine state. The composing crate threads `&SyllableInventory` through the lattice builder, the span-ending scanners and `span_min_syllable_count` on every continuous-input fetch. No inventory → empty keys and no walker.

---

## See also

- `tps.md` (TPS canonicalization), `composing.md`, `continuous-input-ranking.md` (the pipeline this feeds).
- `docs/contributing/phonetics.md` + `docs/phonetics/taigi-phonetics-reference.md` (table-contract authority).
