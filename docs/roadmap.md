# Taigi Keyboard — Roadmap

> **Type**: Planning
> **Keywords**: `roadmap`, `planning`, `refactor`, `structure`
> **Status**: Active
> **Last updated**: 2026-05-07

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

#### P4 — `docs/` root-level orphans ✅ DONE

- **Done**:
  - `docs/file-structure.md` → `docs/architecture/file-structure.md`
  - `docs/keywords.md` → `docs/references/keywords.md`
  - `docs/simplify.md` → `docs/reports/2026-03-21-simplify-pass.md` (date-prefixed per P3 convention; `-pass` disambiguates from `/simplify` slash command)
  - Updated `docs/README.md` index — moved entries from root table into `architecture/` / `references/` / `reports/` tables; root table now only lists `roadmap.md`
  - Updated sweep sites: `CLAUDE.md` tree comment, `docs/ui/layout.md` relative ref, `.claude/agents/refactor-reviewer.md` (×3), `.claude/commands/port-feature.md`
  - Out of scope: `changelog/v3.4.5.md` historical refs frozen by release-history policy; `docs/reports/2026-03-11-audit-report.md` historical narrative refs left as-is
- **Why P4**: Root-level files imply top-level importance; these are reference/architecture/historical

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

## Item 4 — Android UI architecture: align with `rules/android-guidelines.md` best practices

**Status**: Proposed
**Target release tag for first round (P1)**: `v3.5.7` (per user directive 2026-05-07). v3.5.7 will bundle accumulated unreleased work since `v3.5.6` plus Item 4 P1. P2 / P3 round version assignments TBD per round.
**Source**: 2026-05-07 Taigi Android vs FlorisBoard architecture comparison (in-conversation; `references/florisboard/` as upstream best-practice reference). Latest released tag at proposal time = `v3.5.6`.
**Rationale**: Three concrete gaps where the Taigi Android IME diverges from project-documented best practices (`rules/android-guidelines.md` §4 lifecycle/DI, §7 Compose patterns, §8 IME-specific) and from FlorisBoard's modern IME idioms. Together they (a) eliminate the IME window/inset hazard class documented in auto-memory `project_ime_window_arch.md`, (b) cut canvas-render bug surface, (c) unblock JVM-side keyboard-layout testing. Goal anchor is **project Android best practices**, not feature parity with FlorisBoard's plugin / extension / NLP stack (see "What NOT to adopt" below).
**Risk**: Phased — P1/P2 are mechanical and contained; P3 (Compose migration of keyboard rendering) is a multi-PR slice on par with a release round. Each phase is independently shippable. UI-layer only — no Rust shared-core impact (Phase IV-B closed at v3.5.8 covers algorithm extraction; this item covers presentation).

### What NOT to adopt from FlorisBoard (out of scope)

- **Extension / plugin system** (theme addons, keyboard addons via zip extensions) — single-language IME has no third-party plugin demand; adds attack surface (zip parsing) and maintenance cost
- **Java NLP stack** (`ime.nlp.NlpManager`, spell-check service) — Rust shared core already owns morphology, lookup, case transforms; a parallel Kotlin NLP layer would be redundant
- **jetpref preference DSL** — current `PreferenceKeys` + `PrefHelper` + DataStore meets `rules/android-guidelines.md` §6; jetpref's KSP code-gen adds build complexity for no gain at our scale
- **Snygg stylesheet system** + theme extension bundles — current XML themes + `KeyboardColorSettings` JSON is sufficient for single-locale theming
- **Multi-module Gradle split** — solo-maintainer YAGNI; single `app/` module is appropriate for current scope

### Issues (priority order)

#### P1 — Weak-reference singleton accessor for IME service `[B]` `[A]`

- The Android IME service (`TaigiKeyboard`) is reachable today via `CompositionRoot.shared(context)` and direct references held by managers. After A7 audit (Phase II) the IME path was decoupled from Application-singleton, but Activity / settings-UI paths can still hold a strong reference to an IME service that has been destroyed (system reclaim, IME swap, configuration change), risking memory leak and use-after-destroy.
- FlorisBoard pattern at `references/florisboard/app/src/main/kotlin/dev/patrickgold/florisboard/FlorisImeService.kt:83` — `private var FlorisImeServiceReference = WeakReference<FlorisImeService?>(null)`. Set in `onCreate` (line 281), cleared in `onDestroy` (line 357). Public typed accessors at line 94+ (`currentInputConnection()` etc.) safely return `null` if the service is gone.
- **Action**:
  1. Keep the weak ref **private**: `companion object { private var instance: WeakReference<TaigiKeyboard?> = WeakReference(null); ... }` in `TaigiKeyboard.kt`. Do **not** expose a raw `current()` getter — that invites broad service reach-ins from ViewModels / engine code, which contradicts `rules/android-guidelines.md` §4 ("Constructor injection preferred. No `.INSTANCE` global reach-ins").
  2. Expose **typed, IME-only helpers** instead — e.g. `currentInputConnectionOrNull(): InputConnection?`, `withImeService(block: (TaigiKeyboard) -> Unit)`. Each helper is restricted to operations that genuinely require the live IME service (InputConnection / InputBinding / language switch).
  3. Set `instance = WeakReference(this)` in `onCreate`; `instance = WeakReference(null)` in `onDestroy`.
  4. Audit non-IME-context call sites that today hold a captured service reference; route them through the typed helpers, not the raw weak ref. Settings UI / ViewModel paths that don't need `InputConnection` should not gain new access — keep them on the constructor-injected dependencies they already have.
- **Files affected**: `ime/core/TaigiKeyboard.kt` + audit pass over `settings/`, `ui/`, manager classes (expected ≤ 10 call sites)
- **Why P1**: Smallest change, highest safety win. Removes a leak class flagged in auto-memory `project_ime_window_arch.md`. Aligns with `rules/android-guidelines.md` §4 ("Never store an `Activity` Context inside an `object` or a long-lived `class`") and §4 DI principle (no global reach-ins) by gating access through typed helpers rather than a public service handle.

#### P2 — Decouple keyboard layout data from rendering `[B]` `[A]`

- Current keyboard geometry (key widths, popup anchor positions, extended-popup overflow) is computed inside the `View` hierarchy (`KeyboardView`, `KeyboardRowView`) and interleaved with `onMeasure` / `onLayout` / `onDraw`. Layout math cannot be unit-tested without instrumentation.
- FlorisBoard pattern: keyboard structure lives in `TextKeyboard` / `TextKey` model objects, and bounds are computed in `TextKeyboard.layout()` (`references/florisboard/app/src/main/kotlin/dev/patrickgold/florisboard/ime/text/keyboard/TextKeyboard.kt:25-30, 46-152`). This is a useful separation from rendering, **but it is not a pure immutable solver** — `TextKeyboard.layout()` mutates `TextKey.computedData` / `touchBounds` / `visibleBounds` in place, and popup state lives in a `MutablePopupSet`. Taigi should go one step further: extract immutable `KeyboardLayoutData` plus a pure solver, instead of porting FlorisBoard's mutable model verbatim.
- **Action**:
  1. Extract **immutable** DTOs first — `KeyboardLayoutData` (rows + keys + popup mapping) using `data class` + read-only `List`. Do **not** mark these as `// region Shared-Core Candidate` until the migration also removes/wraps any mutable surface ported across (e.g. `TextKey.computedData` / `computed*` `var` fields, `MutablePopupSet` popup containers, `KeyboardMode` coupling) into stdlib-only value types per `rules/android-guidelines.md` §1. Premature shared-core marking would lock in coupling that fails the §1 criteria.
  2. Move geometry computation into a stateless `KeyboardLayoutSolver` — pure function `(KeyboardLayoutData, ContainerSize) -> SolverResult`. Solver returns geometry; it does not mutate the input.
  3. `KeyboardView` consumes `(KeyboardLayoutData, SolverResult)` instead of holding the math.
  4. Add JVM unit tests for the solver (`./gradlew test`) — overflow row, popup anchor near screen edge, popup overflow at screen edges — without instrumentation. Run alongside existing `INVARIANT_*` tests per `rules/android-guidelines.md` §9.
- **Files affected**: `ime/text/keyboard/KeyboardView.kt`, `KeyboardRowView.kt`, `ime/popup/KeyPopupManager.kt` (if exists), new `ime/text/keyboard/KeyboardLayoutData.kt` + `KeyboardLayoutSolver.kt`
- **Why P2**: **Pre-condition for P3 Phase D only.** P3 Phase A/B/C can start without P2 if they avoid keyboard-body geometry — popup work (Phase C) depends on an explicit anchoring contract, which P2 also provides but is not strictly blocking until Phase D. Independently valuable: converts dogfood-only layout bugs into JVM-testable. Adheres to `rules/android-guidelines.md` §1 Kotlin-to-Rust shape preferences; the solver may be a future shared-core extraction candidate (out of scope for this item).

#### P3 — Migrate keyboard rendering to Compose `[B]` `[A]`

- Current keyboard rendering is custom canvas-drawn `View` subclasses (`KeyboardView : LinearLayout` overrides `onDraw`). Window-inset workarounds in `InputView.onApplyWindowInsets` + `requestApplyInsets()` (API 35 compat) are documented in auto-memory `project_ime_window_arch.md` as a known hazard (MATCH_PARENT × MATCH_PARENT + custom child-position insets + `TOUCHABLE_INSETS_VISIBLE` historically caused dismiss bugs; Option A workaround shipped via PR #180). Touch bounds and visual bounds live in separate code paths and can drift.
- FlorisBoard pattern: full Compose tree (`ImeRootView` → `TextKeyboard` composable). `WindowInsets` API handles inset state declaratively; `Modifier.pointerInput { ... }` ties touch to the visible bounding box automatically.
- **Action** (multi-phase slice — each phase is its own PR per `feedback_branching` + `feedback_round_hygiene`):
  1. **Phase A — Compose host shell** — replace XML root inside `InputView` with a `ComposeView` host. Existing custom views remain inside as Compose-View interop fallbacks. No visual change; no behavior change. Establishes the host without committing to migration.
  2. **Phase B — Smartbar + candidate strip to Compose** — convert candidate UI **only after** defining a stable state contract from `SmartbarManager` / `CandidateUpdateCoordinator` (a `data class` snapshot exposed as `StateFlow<CandidateStripState>`). Preserve candidate **selection order**, **scroll reset on input change**, **selected index**, and **update latency**. Treat any timing or selection change as *behavior* under `rules/cross-platform-alignment.md` §1 — that requires the parity-correction tier (§1b), not refactor-freeze.
  3. **Phase C — Popup layer to Compose** — convert popup hierarchy (`KeyPopupManager` + extended popups) to Compose. Define an explicit anchoring contract first (anchor key bounds + screen-edge clamp rule); popups are otherwise state-isolated and visually verifiable.
  4. **Phase D — Keyboard body to Compose** — convert `KeyboardView` + `KeyboardRowView` + per-key rendering to a single Compose tree consuming P2's `KeyboardLayoutData`. Drop custom-view subclasses. Remove `InputView.onApplyWindowInsets` workaround. This phase is the largest and ships the hazard-class removal.
- **Files affected**: `ime/core/InputView.kt`, `ime/text/keyboard/KeyboardView.kt`, `KeyboardRowView.kt`, smartbar XML + manager, candidate overlay XML + coordinator, popup hierarchy
- **Compose IME risks (per-phase regression checks)**: `ComposeView` disposal / `LifecycleOwner` mismatch when the IME service is recreated; `AndroidView` interop event leakage (touch / IME action bypasses); nested-scroll height feedback loops; popup window-token mismatch (popups live on a separate window from the IME service window); pointer cancellation under multi-touch; minimum touch-target compliance; long-press cancellation thresholds; popup drag-selection. **Each phase needs real-device S1/S2/S3 dogfood plus an explicit candidate-tap-latency regression check.**
- **Why P3**: Largest hazard-class removal — Compose `WindowInsets` API replaces hand-rolled inset routing, eliminating the `project_ime_window_arch.md` trap by design. Compose **reduces** touch / visual drift, but pointer hit boxes, minimum touch targets, long-press cancellation, and popup drag-selection still need explicit tests; the invariant is not free. Unifies settings-preview rendering (already Compose) with keyboard rendering (currently custom Views) — same theme/color path. Adheres to `rules/android-guidelines.md` §7.

### Suggested execution order

1. **P1 first** — single-PR mechanical change; ship and observe in one release window
2. **P2 second** — depends on nothing; independently shippable; pre-condition for **P3 Phase D only**
3. **P3** — multi-PR slice (A → B → C → D); each phase is a release-cycle round

P1 is independent and can ship alongside anything. P3 Phase A/B/C can run concurrently with or before P2 (they avoid keyboard-body geometry); **P3 Phase D should not start before P2 lands** (Phase D consumes `KeyboardLayoutData`).

### Per-round gates (apply to every PR in this item)

- Refactor-freeze observed per `rules/cross-platform-alignment.md` §1 — UI-only, no behavior change. Any visible behavior change uses the parity-correction tier (§1b) and labels accordingly.
- Codex + `/simplify` pre-impl review per `rules/android-guidelines.md` §11 + `feedback_review_before_impl` + `feedback_codex_review_sandwich`
- Codex post-edit review on git diff per `feedback_codex_post_edit_review`
- Qualitative dogfood pass on real Android device (S1 / S2 / S3) per `feedback_perf_gate`
- No fallback toggle per `feedback_no_slice_toggles` — ship as direct swap, revert via PR if regression
- Manual `pbxproj`-equivalent: Android `build.gradle` + resource changes are user-handled per `feedback_xcode_manual.md` (extends to Android per project-wide manual-build policy `feedback_manual_build_test`)

### Cross-references

- Project memory: `project_ime_window_arch.md` (IME window/inset hazard class), `feedback_no_slice_toggles.md`, `feedback_review_before_impl.md`, `feedback_codex_review_sandwich.md`
- Reference upstream: `references/florisboard/app/src/main/kotlin/dev/patrickgold/florisboard/FlorisImeService.kt:83-94,281,357` (weak-ref accessor), `.../ime/text/keyboard/TextKeyboard.kt` (data + layout split)
- Project rules: `rules/android-guidelines.md` §1 shared-core candidate / §4 lifecycle+DI / §7 Compose / §8 IME-specific / §11 refactor-round checklist

---

<!-- Add new roadmap items below as Item 5, Item 6, ... -->
