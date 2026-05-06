# Taigi Keyboard — Roadmap

> **Type**: Planning
> **Keywords**: `roadmap`, `planning`, `refactor`, `structure`
> **Status**: Active
> **Last updated**: 2026-05-05

---

## Summary

- Single source of truth for forward-looking work items not yet scheduled into a release slice
- Items are ordered by priority. Each item has scope, rationale, and concrete change set
- When an item is picked up, move it to a release slice plan and remove from this file

---

## Item 1 — Project Structure & File Naming Cleanup

**Status**: Proposed
**Source**: 2026-05-05 cross-cutting structure review (see "Strengths" + "Issues" below)
**Rationale**: Multiple naming inconsistencies and catch-all packages reduce readability and break cross-platform alignment (CLAUDE.md §5)
**Risk**: Touches many files but no behavior change; suitable for a single non-release-blocking PR per priority tier

### Strengths (preserve, do not touch)

- **Top-level layering** — `ios/ android/ engine/ dictionary/ taigi-converter/ docs/ rules/ knowledge/` boundaries are clean
- **`engine/` Rust workspace naming** — `composing / lexicon / nextword / phonetics / ranking / dispatch` are SRP and semantic; internal `api.rs / dispatch.rs / handle.rs / lib.rs` follow consistent intra-crate pattern
- **iOS `Sources/` feature-based grouping** — `Actions/ Autocomplete/ Composition/ Engine/ Lexicon/ NextWord/ Layout/ Overlays/ Settings/` matches modern Swift conventions
- **Cross-platform parity names** — `RustEngineBridge`, `SettingsResetCoordinator`, `KeyboardColorSettings`, `CompositionRoot` exist on both platforms with matching names
- **`rules/` vs `docs/` separation** — mandatory rules and reference docs do not mix

### Issues (priority order)

#### P1 — Cross-platform naming misalignment (Tab*Texts + tab1~4/)

| iOS (semantic) | Android (positional) |
|---|---|
| `Strings/HomeTexts.swift` | `localization/Tab1Texts.kt` |
| `Strings/LayoutTexts.swift` | `localization/Tab2Texts.kt` |
| `Strings/DictionaryTexts.swift` | `localization/Tab3Texts.kt` |
| `Strings/SettingsTexts.swift` | `localization/Tab4Texts.kt` |

- Android uses positional `Tab1/2/3/4` — breaks if tab order changes; filename does not convey content
- Same problem at directory level: `android/.../ui/tabs/tab1/ tab2/ tab3/ tab4/`
- **Action**: Rename Android side to match iOS — `Tab1Texts.kt` → `HomeTexts.kt`, `tab1/` → `home/`, etc.
- **Files affected**: 4 `*Texts.kt` files + 4 `tabN/` directories + all imports (~30+ files)
- **Why P1**: Violates CLAUDE.md §5 cross-platform alignment; positional naming is fragile

#### P2 — Catch-all `Common/` and `util/` anti-pattern

**iOS `Common/`** (3 unrelated files):
- `SettingsIcons.swift` → move to `Settings/` or `Styling/`
- `StringExtensions.swift` → move to a new `Extensions/` or merge with feature that uses it
- `LoggerBackend.swift` → move to a new `Logging/`

**Android `util/`** (8 `*Utils.kt` files — classic god-bag):
- `CsvUtils.kt` → `CsvParser.kt`
- `LocaleUtils.kt` → `LocaleResolver.kt`
- `ResourceUtils.kt` → behavior-specific name, or split per resource type
- `ThemeColorUtils.kt` → `ThemeColors.kt` (extension)
- `FontUtils.kt` → `FontLoader.kt` or similar
- `LauncherIconUtils.kt` → `LauncherIcons.kt`
- `AppVersionUtils.kt` → `AppVersion.kt`
- `ActivityExtensions.kt` → keep but move to `ui/` (it's UI-layer)

- **Action**: Rename behavior-first; relocate to feature directories
- **Why P2**: `*Utils` is a known anti-pattern (Robert Martin, Kotlin style guide); files lose discoverability

#### P3 — `docs/` one-off reports vs evergreen specs ✅ DONE

- `docs/engine/migration-residue-2026-05-04.md` was a single-pass audit report sitting alongside evergreen design specs (`autocomplete.md / binary-format.md / composing.md / ...`)
- `docs/reports/` was inconsistent: `2026-03-11-audit-report.md` (dated prefix) vs `architecture-review.md / codebase-health.md` (no date)
- **Done**:
  - Moved `docs/engine/migration-residue-2026-05-04.md` → `docs/reports/2026-05-04-migration-residue.md`
  - Added date prefix to one-off reports (`architecture-review`, `codebase-health`, `segmentation-tie-bug`); kept evergreen `khiin-lattice-research` and live `refactor-backlog` undated
- **Why P3**: Mixes report lifecycle with spec lifecycle; readers can't tell what's still authoritative

#### P4 — `docs/` root-level orphans

- `docs/file-structure.md` → `docs/architecture/file-structure.md`
- `docs/keywords.md` → `docs/references/keywords.md`
- `docs/simplify.md` → `docs/reports/simplify.md` (it's a historical log)
- **Action**: Move + update `docs/README.md` index
- **Why P4**: Root-level files imply top-level importance; these are reference/historical

### Minor observations (not action items)

- iOS `RustEngineBridge+Lexicon.swift` (Swift extension files) vs Android `LexiconBridge.kt` (Kotlin top-level) — each follows platform idiom, leave as-is
- `changelog/` missing `v3.5.4 / v3.5.7 / v3.5.8` files — consistent with `feedback_changelog_timing` policy (only update at release time), not an issue
- Top-level `Makefile` + `engine/Makefile.toml` coexist — verify root `Makefile` still in use
- iOS `Lexicon/Database/` has 11 files but each is SRP (Schema / Repository / Migrator / Pruner / Capacity / BindingHelpers / ConnectionManager) — acceptable

### Suggested execution order

1. **P1 first** — rename Android `Tab*Texts` + `tab1~4/` (highest cross-platform impact, mostly mechanical)
2. **P2 second** — break apart `Common/` and `util/` (improves discoverability immediately)
3. **P3 + P4 together** — `docs/` reorg in a single PR (pure file moves + README index update)

Each tier should be its own PR per `feedback_branching` (one PR per round).

---

## Item 2 — Borrow librime spelling-algebra for derived-key generation

**Status**: Proposed
**Source**: 2026-05-05 librime architecture comparison (see `references/librime/src/rime/algo/algebra.cc:54-100` and `data/minimal/luna_pinyin.schema.yaml` lines 73–86)
**Rationale**: Today's FST stores `tl:` and `poj:` as parallel namespaces, and abbrev / notone variants are emitted by ad-hoc Python branches in `dictionary/build/create_fst.py:101-115`. New derivation kinds (TL fuzzy, n/ng merge, 文白並列, 客語拼音, etc.) require code changes to the build pipeline; runtime ranking has no signal to downrank derived hits because every entry looks identical in the FST. librime solves both problems with a build-time **spelling algebra**: one canonical key per entry, plus a regex rule pipeline (`derive` / `abbrev` / `fuzz` / `erase`) that emits derived spellings tagged with `SpellingType` + credibility — ranking can then downrank derived entries naturally.
**Risk**: Build-pipeline-only refactor; FST wire format gains two optional trailing bytes — `derivation_type_u8` (this item) and `form_u8` (Item 3) — within a single shared trailer extension, both of which ranking and runtime can ignore until ready. Backward-compatible if rolled out in two slices (rule-driven derivation first, type tagging second).

### What to borrow

1. **Rule file replaces hard-coded derivations** — extract `tl_abbrev` / `poj_abbrev` / `*_notone` / future fuzzy-spelling logic from `create_fst.py` into a declarative rules file (TOML or YAML). Each rule is `{op: derive|abbrev|erase, pattern: <regex>, replace: <template>, type: <tag>}`. Adding "TL n/ng 不分" or "POJ 文白並列" becomes one rule entry, not a code change.
2. **Derivation-type tag on FST entries** — extend the wire format from `key_bytes ‖ 0xFF ‖ rowid_le_4` to `key_bytes ‖ 0xFF ‖ rowid_le_4 ‖ derivation_type_u8 ‖ form_u8`. This item owns `derivation_type_u8` ∈ {`0=base`, `1=derived`, `2=abbrev`, `3=fuzzy`}, encoding *where the entry came from in the build pipeline* (ranking credibility signal). The companion `form_u8` byte is owned by Item 3 and encodes *which stored form the key represents* (consumed-length signal); the two are orthogonal dimensions packed as two distinct bytes in the same trailer extension — see cross-reference below. Ranking layer reads `derivation_type_u8` and applies a credibility multiplier — derived hits rank below base hits without needing dictionary-level weight tweaks.
3. **POJ stays as its own derivation, not a separate namespace** — POJ keys become rule-derived from TL canonical (e.g. `tsh→chh`, `gua→goa`, `ts→ch`, `oo→o͘`) instead of being independently generated. The `poj:` namespace can stay for now, but the algebra approach makes it possible to retire either the prefix or the parallel build path in a later slice if the trade-off proves worthwhile.

### What NOT to borrow (deferred / rejected)

- **Schema YAML mode-config registry** (`src/rime/schema.h`) — librime supports arbitrary new input schemes via dropping a YAML; we have only TL / POJ / TPS, solo-maintainer YAGNI says keep `KeyMode` enum hard-coded
- **Per-mode separate prism files** — current single-FST + dual-namespace is simpler at three modes; only revisit if the mode count grows past ~5
- **Ticket / name_space dynamic component composition** — C++ dynamic-dispatch flavour, awkward in Rust's static type system
- **Speller / Syllabifier / Translator three-phase split** — partially mirrored already in `composing/` + `phonetics/` + `lexicon/`; stricter typed boundaries are over-engineering until fuzzy-match work demands them

### Concrete change set (when picked up)

1. Define `dictionary/build/spelling_rules.toml` with the existing TL/POJ abbrev + notone derivations expressed as rules
2. Add a Rust-side rule engine (likely under `engine/build-helpers/`) that consumes the rules file and a base-key stream, emits `(derived_key, base_rowid, type_tag)` triples
3. Extend `fst-builder` wire format to append two trailing bytes — `derivation_type_u8` (this item) and `form_u8` (Item 3); default both to `0` for callers that haven't migrated
4. Update `engine/lexicon/src/prefix_index.rs` to expose the type tag via the lookup result; teach `engine/ranking/` to apply the credibility multiplier
5. Migrate `create_fst.py` to invoke the rule engine instead of hard-coded derivation loops; the abbrev / notone Python paths delete

> **Cross-reference**: the wire-format extension in step 3 carries **two orthogonal bytes** in the same trailer — `derivation_type_u8` (this item, ranking credibility) and `form_u8` (Item 3, consumed-length calculation). They cannot share a single byte: a candidate can be both "derived" (derivation_type) and "notone" (form) at the same time, so the dimensions must encode independently. Sequence both items so the trailer extension lands once and serves both. See Item 3 Phase A.

### Why P2 (after Item 1's structure cleanup)

- Foundation for future derivation kinds (TL fuzzy spelling, 文白並列, future Hakka mode) without per-feature build-script edits
- Adds the missing ranking signal for abbrev hits — current ranking can't tell `tl:hs` (abbrev) apart from `tl:hoo-se` (canonical) at the FST layer
- Self-contained: no platform code (iOS / Android) changes; pure Rust + build pipeline
- Should be split into two PRs: rule-driven derivation first (no behavior change vs current FST), type tagging + ranking second (visible ranking improvement)

---

## Item 3 — Continuous input (連續輸入) support

**Status**: Proposed
**Source**: 2026-05-05 IME feature gap analysis vs mainstream IMEs (Pinyin / Zhuyin / Cangjie all support this; Taiwanese IMEs Khiin / Aiongtaigi support partial variants)
**Rationale**: User types a multi-syllable string (e.g. `taigikhipuann` or `ㄉㄞˊㄨㄢˊㄉㄞˊㆣㄧˋㄌㄛˊㄇㄚˋㆢㄧ˫`) and selects candidates one by one — the matched portion is consumed from the buffer, the rest re-queried until empty. Today's lookup only handles full-input match. This is table-stakes for any modern IME.
**Risk**: Mostly a runtime feature (lattice + buffer state machine, ~500 LOC Rust estimated). Storage layer needs three small additions, all backward-compatible (callers ignoring new fields keep working).

### What the feature requires (4 sub-capabilities)

| Capability | Storage or Runtime? |
|---|---|
| ① Segment input string into syllable boundaries | storage (need syllable inventory) |
| ② Return all candidates of varying syllable lengths at each boundary | **storage already sufficient** (FST prefix search already does this) |
| ③ Know how many bytes / syllables of buffer a selected candidate consumed | storage (need entry metadata) |
| ④ Re-query remaining buffer + lattice-aware ranking | runtime (lattice DP) |

### Storage additions (priority order)

1. **Entry form byte (P1)** — extend FST wire format from `key ‖ 0xFF ‖ rowid_le_4` to `key ‖ 0xFF ‖ rowid_le_4 ‖ derivation_type_u8 ‖ form_u8`. This item owns `form_u8` ∈ {`0=numeric_tone`, `1=notone`, `2=abbrev`, `3=hanzi`}, encoding *which stored form the key represents*. The companion `derivation_type_u8` byte is owned by Item 2 and encodes a different dimension (ranking credibility); the two enums are orthogonal and cannot share a single byte (a "derived-notone" candidate would be ambiguous). `form_u8` is used to compute how many bytes the user buffer should consume when a candidate is selected — different forms have different stored-key lengths, so we can't infer consumed length from the FST key alone. **Same trailer extension carries Item 2's `derivation_type_u8`** — one wire-format migration appends both bytes, not two.
2. **Per-entry syllable count (P2)** — verify `dictionary.bin` already exposes `syllable_count`; if not, add `u8 syllable_count` per record (~160 KB total). Required because (a) TPS mode consumes by syllable count, not byte count (bopomofo bytes ≠ TL bytes); (b) abbrev consumption — input `tgkp` (4 bytes) matches 4-syllable abbrev, must consume 4 syllable boundaries from the buffer, not 4 bytes.
3. **Syllable inventory FST (P3)** — separate small file `dictionary/output/syllables.fst` (~50 KB) listing all valid Taiwanese syllables (numeric + notone forms, ~2000 entries). Built by extracting all 1-syllable entries from the canonical dictionary at build time — no external data needed. Used by runtime TL syllabifier for longest-match segmentation.

### What NOT to add (deferred / rejected)

- **Khiin / RIME-style two-level Prism + Table** — adding a `syllable_id` indirection layer between FST and `dictionary.bin` doesn't meaningfully shrink data (we have ~160K entries either way) and adds FFI boundary cost on the hot path. The current FST + dict.bin two-layer design is sufficient.
- **Pre-indexed "starts at position N" inverted index** — user input is runtime-only, precomputing position-keyed indexes is impossible. Lattice DP handles this at runtime.

### Runtime additions (out of storage scope, listed for sequencing)

- **Syllabifier** in `engine/composing/` — for TL does longest-match against `syllables.fst`; for TPS reads tone-mark boundaries (`ˊˇˋ˙˫`) for free, no inventory needed
- **Lattice + DP ranker** in `engine/ranking/` — weights longer matches + bigram probability between consecutive user selections
- **Buffer state machine** in `engine/dispatch/` (or platform side) — tracks consumed prefix, selection history, supports back-edit / undo

### TPS asymmetry (simpler than TL)

TPS continuous input is structurally simpler because bopomofo tone marks (`ˊˇˋ˙˫`) are unambiguous syllable terminators — no syllabifier inventory needed, segmentation is O(n) scan. Same lattice / DP / buffer logic otherwise. `syllables.fst` is TL-only.

### Suggested execution order

1. **Phase A — wire-format trailer extension** (1 PR; appends two orthogonal bytes — `derivation_type_u8` for Item 2's ranking credibility and `form_u8` for Item 3's consumed-length calculation; jointly serves Item 2 storage step 3 and Item 3 storage step 1)
2. **Phase B — confirm or add `syllable_count`** in `dictionary.bin` (1 small PR; storage prep, no behavior change)
3. **Phase C — syllable inventory FST + TL syllabifier** (1 PR; runtime adds segmentation but no UI yet)
4. **Phase D — lattice runtime + iOS/Android UI integration** (multi-PR slice; visible feature)

Phases A and B can ship ahead of the user-visible feature — zero-impact storage prep that unblocks both Item 2 and Item 3 D.

### Why P3 (after Items 1 + 2)

- Largest user-visible UX win on the list — closes the remaining gap vs Pinyin / Zhuyin IMEs
- Storage prep (Phases A + B) is shared with Item 2 and is purely additive — can ship even if user-facing UI is delayed
- Runtime work (Phase D) is substantial (~500 LOC) but contained to `engine/composing/` + `engine/ranking/` + platform-side buffer logic; no cross-cutting refactor
- Mature dogfood path: ship behind a settings toggle initially, migrate to default-on after Round-A/B/C cycles

---

<!-- Add new roadmap items below as Item 4, Item 5, ... -->
