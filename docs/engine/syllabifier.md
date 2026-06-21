# Syllabifier — Syllable Parsing & Inventory

> **Type**: Reference
> **Keywords**: `syllabifier`, `syllable`, `SyllableInventory`, `syllables.fst`, `valid_span_endings`, `phonotactics`
> **Related**: composing.md, binary-format.md, tps.md, continuous-input-ranking.md, ../architecture/behavioral-invariants.md

---

## Summary

- Two distinct primitives feed continuous input: a **table-based syllable parser** (`phonetics::syllable`) and a **runtime FST membership reader** (`lexicon::SyllableInventory`).
- The composing crate's `syllabifier` walks a buffer into valid syllable spans (BFS, not longest-match) using the inventory, building the segmentation lattice.
- `syllables.fst` is a tagged single FST holding three families — `tl:` / `poj:` / `tps:` — each carrying both numeric and toneless canonical keys.

> **Two senses of "valid syllable" — do not conflate.** `phonetics::is_valid_syllable` is a *phonotactic* table test (does this string split into a legal initial+final?). `SyllableInventory::contains_in` is *FST membership* (does this exact canonical syllable appear in the built dictionary's inventory?). They answer different questions.

---

## Files

| File | Responsibility |
|------|----------------|
| `engine/phonetics/src/syllable.rs` | Parse primitive: tone strip, POJ→TL spelling normalize, initial/final split, per-syllable canonicalizers, acronym-key detection |
| `engine/phonetics/src/tables.rs` | `TL_INITIALS` / `TL_FINALS` / `TONE_NUM_TO_COMBINING` lookup tables |
| `engine/lexicon/src/syllable_inventory.rs` | `SyllableInventory` — mmap'd `syllables.fst` reader, tagged-family membership |
| `engine/composing/src/syllabifier/mod.rs` | Mode-aware dispatch `valid_span_endings_lowered` |
| `engine/composing/src/syllabifier/tl.rs` | BFS scanner for TL/POJ/English families |
| `engine/composing/src/syllabifier/tps.rs` | BFS scanner for the TPS family (same shape) |
| `engine/composing/src/shadow.rs` | `span_min_syllable_count` (min-hop BFS) |
| `engine/build-helpers/fst-builder/src/syllables.rs` | Offline `build-syllables` — emits `syllables.fst` |
| `dictionary/build/create_syllables_fst.py` | Build driver: stages family inputs, invokes the builder |

---

## 1. `syllable.rs` — parse primitive

Ported from `taigi-converter/src/phonetics.js`. Two public canonicalizers share one pipeline:

- **`canonicalize_syllable(token) -> Option<(canonical_toneless, tone_digit)>`** — `strip_tone_mark` → lowercase → `normalize_to_tl` (POJ→TL **spelling** fold) → `split_initial_final` membership gate. Emits **TL** ASCII.
- **`canonicalize_poj_syllable(token)`** — phonotactic gate via TL tables, but emits the **POJ** ASCII shape. So `chit8` → `poj:chit` while `tsit8` → `tl:tsit`.

Sub-primitives:

- `strip_tone_mark(text) -> (bare_NFC, tone_digit)` — ASCII fast-path strips a trailing `1..=9`; non-ASCII path NFD-decomposes and finds the combining mark. `0` is **not** a tone.
- `split_initial_final(text) -> Option<(initial, final)>` — accepts only when `initial ∈ TL_INITIALS` AND `final ∈ TL_FINALS` (input must be pre-lowercased + TL-normalized).
- `is_valid_syllable(token) -> bool` = `canonicalize_syllable(token).is_some()`.
- `is_roman_acronym_key(body) -> bool` — true when every char is an ASCII consonant AND the body does NOT `splits_into_syllables`. The continuous partial-prefix path uses it to drop acronym surfaces; the two-condition gate deliberately keeps fused nasals (`tngtng`, `mngkng`) and syllabic-nasal singles (`m`, `ng`).

**Normalization tables (order is load-bearing):** `NORMALIZE_TO_TL_RULES` = `ch→ts, ou→oo, o͘→oo, ⁿ→nn, oa→ua, oe→ue, eng→ing, ek→ik, oonn→onn` (`oonn` after `oo`). `normalize_to_tl_keep_tl_finals` drops `eng→ing`/`ek→ik` to preserve real TL finals `eng`/`ek`. POJ variants: `NORMALIZE_TO_POJ_RULES` (per-syllable) + `NORMALIZE_TO_POJ_GLYPH_RULES` (whole-buffer glyph-only).

`TL_FINALS` covers the special/dialectal finals (`er`, `erh`, `erk`, `ir*`, `iri`, `irinn`, `ee`, `eeh`, `or`, …) — per `knowledge/taigi-phonetics-reference.md` §3.2.6; do not prune these.

---

## 2. `SyllableInventory` — `syllables.fst`

`SyllableInventory` wraps an `fst::Set` over a readonly-mmap'd `syllables.fst`.

| Method | Purpose |
|--------|---------|
| `open(path) -> Result<Self, LexiconError>` | mmap readonly + validate fst parse |
| `contains_in(mode, syllable) -> bool` | Membership test. Maps mode → prefix: `Poj`→`poj:`, `Tps`→`tps:`, `Tl`/`English`→`tl:`. **Caller must pre-canonicalize** for the family |
| `contains(syllable)` | `#[deprecated]` alias of `contains_in(Tl, …)` (B-1 grace period) |
| `entry_count()` / `is_empty()` | inventory size |

The inventory does **not** expose `valid_span_endings` / `span_min_syllable_count` — those live in the composing crate and call `contains_in`.

**Key families.** Each phonotactically valid syllable appears as **two keys per family** — numeric (`tl:tsua7`) and toneless (`tl:tsua`) — under one of three prefixes: `tl:` (POJ rows folded to TL), `poj:` (POJ rows kept as POJ ASCII), `tps:` (Bopomofo).

**Family-split rationale (v3.5.9 B-1).** ~80,000 of ~159,000 dictionary rows have `tl_num != poj_num` (`chit8`/`tsit8`, `goa2`/`gua2`). A single TL-folded inventory loses POJ-side syllable boundaries, so a continuous POJ buffer like `chiah` could not be recognized as one valid syllable. Tagging all families into one FST keeps storage shared (prefix-tree overlap) while `contains_in(mode, …)` serves the correct family. Design + alternatives: `docs/reports/2026-05-20-v359-b-plan.md` §B-1; the TPS family was added in v3.5.9 D / C-0.

---

## 3. Syllabifier consumption (composing crate)

**`valid_span_endings(input, pos, inv, mode, max_syllables) -> Vec<usize>`** — FIFO BFS from `pos`: for each char-boundary `end` within `MAX_SYLLABLE_BYTES`, accept when `inv.contains_in(mode, &lowered[cur..end])` AND it is not a false toneless boundary AND it is the first arrival. Returns ascending, deduped byte offsets.

- **BFS, not longest-match** — typing `tsua` must surface both `珠` (span 3) and `紙`/`珠仔` (span 4) as candidate sources; pure longest-match (khiin-rs style) would commit to `tsua` and lose `珠`. A full global lattice (librime style) was over-built for scope; depth-capped BFS is the chosen middle ground.
- `is_false_toneless_boundary` suppresses a toneless match sitting immediately before a tone digit — the digit belongs to the syllable (the FST also holds the numeric form).
- `MAX_SYLLABLE_BYTES = 10` (max initial `tsh` + max final `uainnh` + optional tone digit).

**Dispatch** — `mod.rs` routes `Tps` → `tps::…`, `Tl`/`Poj`/`English` → `tl::…`, so the syllabification family always ties to the same `mode` that drives the emitted FST key prefix (no drift).

**`span_min_syllable_count(span, inv, mode) -> Option<usize>`** (`shadow.rs`) — unweighted shortest-path (fewest hops) from 0 to end, each hop one valid single syllable. Returns the fewest-syllable valid reading. It replaced `greedy_longest_syllabification().unwrap_or(1)`, which could dead-end on a span still syllabifiable via a **non-greedy** split (`ta`+`nia` when `tan`+`ia` dead-ends — PR #290 P1). `None` only when the end is not single-syllable-reachable; the caller fail-closes by dropping that lattice edge. This is the **metadata** syllable-sum for the synthesized OOV slot-0 candidate, NOT a ranking penalty.

**Lattice** — `lattice/builder.rs::build_lattice` pushes every `(start, end)` edge from `valid_span_endings_lowered` (both atomic single-syllable and multi-syllable phrase ends). The lattice keeps every edge; the display-layer longest-match suppression lives separately in `composing::continuous` (`INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`, behavioral-invariants.md).

---

## 4. Build + runtime

**Build (offline).** `make dict` → `build.sh` Step 4/9 runs `create_syllables_fst.py`, which loads `dictionary.csv`, stages three temp inputs (`tl_num` lines, `poj_num` lines, and per-syllable TPS lines — the fused `tps_num` is not reusable because tone-1 syllables carry no Bopomofo mark, losing boundaries), then invokes `fst-builder build-syllables` once. The builder splits TL/POJ lines on ASCII tone digits, canonicalizes per family, emits prefixed numeric + toneless keys, sort+dedups, and writes the `fst::Set`. Phonotactically invalid syllables are counted + sample-logged but do not abort the build. The builder is an offline producer, outside the runtime crate dependency graph.

**Runtime.** `lexicon::EngineHandle::install` opens `syllables.fst` via `SyllableInventory::open` (optional path) and stores it on `EngineState`. The composing crate threads `&SyllableInventory` through `build_lattice` / `valid_span_endings_lowered` / `span_min_syllable_count` on the per-keystroke continuous-input path.

---

## Constants

- `MAX_SYLLABLE_BYTES = 10`; tone digits `1..=9` (`0` is not a tone); two keys per valid syllable per family; three families in one `fst::Set`.
- ~80,000 / ~159,000 rows diverge `tl_num != poj_num` (B-1 driver).

---

## See also

- `binary-format.md` (`syllables.fst` byte format), `tps.md` (TPS canonicalization), `composing.md` (lattice/walker).
- `continuous-input-ranking.md`, `continuous-commit-and-display.md` (the pipeline this feeds).
- `architecture/behavioral-invariants.md` (`INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`, §30 literal-no-fold).
- `.claude/rules/phonetics.md` + `knowledge/taigi-phonetics-reference.md` (table-contract authority).

> Note: `syllabifier/tps.rs` shares the TL inv-driven BFS shape but its TPS-specific coda/tone handling was not line-by-line verified for this doc — read it directly when documenting TPS-specific span behavior.
