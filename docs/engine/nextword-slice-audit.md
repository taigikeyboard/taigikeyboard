# NextWord Slice — Audit (v3.5.5)

**Status**: pre-impl audit, authored 2026-05-01 on branch `main` before opening `phase4b/v3.5.5-nextword-slice`. Companion to `nextword-slice-plan.md`.

**Goal**: catalogue NextWord-domain code on both platforms, lock slice scope, identify anti-pattern adjacencies, justify the SQLite/mmap deferral to v3.5.6.

**Cadence ref**: `project_rust_migration_cadence.md` release map — v3.5.5 = NextWord predictions; v3.5.6 = Lexicon (SQLite + Trie behind FFI).

**Architecture target**: D mode for NextWord engine logic — platform side has zero NextWord state machine / zero scoring math. Persistence (SQLite + association.bin mmap) stays platform-side until v3.5.6.

---

## 1. NextWord domain definition

NextWord = bigram-driven next-character prediction for Taiwanese. Five concerns:

1. **Lifecycle state machine** — Intent → (newState, [Effect]) decision per user lifecycle event (selection / backspace / context timeout / new composing / full reset).
2. **Scoring math** — pure RIME-style exponential decay + learning bonus; constants pinned by INVARIANT_* labels in `behavioral-invariants.md` §§7, 8.
3. **Compound-word splitting** — `"a-b-c" → [(a, b), (b, c)]` for sequential bigram recording.
4. **Prediction filtering** — `[RawNextWordPrediction] → [EnginePrediction]` shape transformation incl. POJ/TL display conversion + Hanji/roman fallback.
5. **Stale-result discard** — generation counter mechanism so async SQLite results landing after state invalidation are dropped.

**NOT NextWord (out of slice)**:
- SQLite persistence (`user_association.db` schema / migrations / CRUD) — Lexicon slice (v3.5.6).
- `association.bin` mmap reader — Lexicon slice (v3.5.6).
- Smartbar UI integration — platform forever.
- KeyboardKit / FlorisBoard wiring — platform forever.
- Autocomplete classifier (English / TPS / hanji input-type triage) — different concern; reconsider in a future autocomplete slice.

---

## 2. Inventory — iOS

Path root: `ios/Sources/TaigiKeyboard/NextWord/` unless noted.

| File | LOC | Role | v3.5.5 disposition |
|---|---:|---|---|
| `NextWordEngine.swift` | 290 | Pure intent → outcome decider; `enum NextWordEngine` namespace | **Delete after Rust port** (logic moves to `engine/nextword`) |
| `NextWordOutcome.swift` | 96 | `Intent / EngineSettings / PersistedState / DecisionInput / AssociationPair / Outcome / Effect` types | **Delete** (types ported to proto) |
| `NextWordScorer.swift` | 64 | Pure scoring math + ranking constants | **Delete** (constants + math move to Rust) |
| `RawNextWordPrediction.swift` | 27 | DTO at engine/service boundary | **Delete** (proto type) |
| `EnginePrediction.swift` | 23 | UI-facing DTO | **Delete** (proto type) |
| `AutocompleteContextBooster.swift` | 33 | Pure first-char-set reorder helper | **Delete** (Rust port) |
| `NextWordController.swift` | 230 | iOS executor — Timer / @MainActor / settings read / generation bookkeeping / UI fan-out | **Reduce** — interpret engine effects against Timer + @MainActor; keep public API surface for callers |
| `Repository/NextWordRepository.swift` | 215 | SQLite CRUD on `user_association.db` | **Keep** (SQLite — v3.5.6) |
| `Repository/NextWordSchema.swift` | not read | SQLite schema + migrations | **Keep** (SQLite — v3.5.6) |
| `Services/NextWordService.swift` | 386 | Facade — assoc.bin reader + SQLite + scoring + capacity policy | **Reduce** — drop scoring (`scoreDict` / `calculateUserScore` callers route to `RustEngineBridge.nextword*` for filter step; query path stays SQLite); keep DB I/O |

**Tests** (`ios/TaigiKeyboardTests/`):

| File | Disposition |
|---|---|
| `NextWordEngineTests.swift` | Delete (Rust workspace tests cover) — keep iOS-side `RustEngineBridgeTests` parity tests |
| `NextWordScorerTests.swift` | Delete |
| `AutocompleteContextBoosterTests.swift` | Delete |
| `AutocompleteInputClassifierTests.swift` | **Untouched** (classifier out of slice) |

**Call sites** (consumers of NextWord public API):

- `Actions/ActionHandler+Suggestions.swift:53` — `nextWordController.process(text:roman:)` on suggestion tap
- `Actions/ActionHandler+KeyActions.swift:60, 82` — `nextWordController.clearDisplay()` on new composing
- `Actions/ActionHandler+KeyActions.swift:147, 215` — `nextWordController.process(...)` on Space / Enter
- `Actions/ActionHandler+KeyActions.swift:188` — `nextWordController.resetAndClearUI()` on backspace-empty
- `Actions/ActionHandler+KeyActions.swift:193` — `nextWordController.rePredictAfterBackspace(lastChar:)`
- `KeyboardExtension/KeyboardViewController.swift:152` — `actionHandler?.nextWordController.resetAndClearUI()` on document change
- `KeyboardExtension/KeyboardViewController+Setup.swift:54, 98` — context updater wiring + selection-context provider registration
- `Autocomplete/Services/AutocompleteService.swift:157` — `AutocompleteContextBooster.boost(words:predictedFirstChars:)`

**NextWordEngine direct callers (outside the engine file itself)**:
- Test target: `NextWordEngineTests` (deletes with the engine port)
- `ActionHandler+Suggestions.swift:70` — referenced in comment only (no code call)

NextWordEngine is **only** called by `NextWordController.apply(intent:)` in production code → port keeps the same shape.

**NextWordScorer direct callers (outside the scorer file itself)**:
- Tests only on iOS (Android uses scorer in `NextWordService.predict` via `NextWordScorer.scoreDict` / `calculateUserScore` — see §3).

---

## 3. Inventory — Android

Path root: `android/app/src/main/java/com/siansiansu/taigikeyboard/`.

### 3.1 `ime/core/nextword/` (shared-core candidate package)

| File | LOC | Role | v3.5.5 disposition |
|---|---:|---|---|
| `NextWordEngine.kt` | 381 | Pure intent → outcome decider; `object NextWordEngine` | **Delete after Rust port** |
| `NextWordOutcome.kt` | 158 | Mirror of iOS Outcome.swift types | **Delete** (proto) |
| `NextWordScorer.kt` | 87 | Mirror of iOS scorer | **Delete** |
| `RawNextWordPrediction.kt` | 22 | DTO | **Delete** (proto) |
| `EnginePrediction.kt` | 32 | UI DTO (Android adds `score: Double` field) | **Delete** (proto — surface unifies on Rust shape) |
| `NextWordPredictor.kt` | 43 | Interface seam over `NextWordService` for testability | **Keep** (platform IO seam — not engine logic) |

### 3.2 Platform executor + persistence + smartbar

| File | LOC | Role | v3.5.5 disposition |
|---|---:|---|---|
| `ime/text/smartbar/NextWordHandler.kt` | 457 | Android executor — coroutine job / settings read / generation / UI callbacks / `NextWordPredictor` IO | **Reduce** — interpret effects with same coroutine pattern; companion helpers `isNoise` / `splitCompoundWord` route to `RustEngineBridge` |
| `ime/dictionary/NextWordService.kt` | 703 | SQLite + assoc.bin facade + scoring | **Reduce** — drop scorer references (route to bridge filter); SQLite stays |
| `ime/text/composing/AutocompleteContextBooster.kt` | 40 | Pure boost helper | **Delete** (Rust port; bridge wrapper) |

**Tests**:
- `app/src/test/.../ime/core/nextword/NextWordEngineTest.kt` — delete
- `app/src/test/.../ime/core/nextword/NextWordScorerTest.kt` — delete
- `app/src/test/.../ime/text/smartbar/NextWordHandlerTest.kt` — **keep** (platform executor harness; rewrite assertions against bridge mock)

**Call sites** (consumers of Android NextWord public API):

- `ime/text/smartbar/SmartbarManager.kt:78–565` — owns `nextWordHandler`; routes show/hide/reset/select/backspace
- `ime/text/smartbar/CandidateClickHandler.kt:99, 156` — `NextWordHandler.extractCurrentWord(...)` companion helper
- `ime/text/composing/TaigiAutocompleteService.kt:118, 139` — `AutocompleteContextBooster.boost(...)`
- `ime/dictionary/NextWordService.kt:234, 269, 272` — `NextWordScorer.{scoreDict, calculateUserScore, calculateDecay}` (the SQLite query path scores rows BEFORE returning to the executor; this scoring moves into Rust filter step in §4.4)

### 3.3 NextWordHandler companion helpers (call-site-driven)

`NextWordHandler` ships three companion helpers exposed because pre-A5 callers depended on them:

- `isNoise(word) → Bool` — delegates to `NextWordEngine.isNoiseText`
- `splitCompoundWord(word) → List<String>` — delegates to `NextWordEngine.splitCompound`
- `extractCurrentWord(text) → String` — pure trailing-word extraction; **keeps platform** (uses ASCII-punctuation logic that doesn't fit the engine's `noise_punctuation` Set)

After v3.5.5, `isNoise` and `splitCompoundWord` route through `RustEngineBridge.nextword*` instead of the deleted Kotlin engine; `extractCurrentWord` stays as a smartbar-only utility.

---

## 4. Cross-platform invariant constants

Constants currently maintained by inline `// CROSS-PLATFORM INVARIANT — mirrors <other-file>:<line>` comments. After v3.5.5 the canonical source moves to Rust; the inline comments come down because the constants exist in only one place (Rust crate).

| Constant | iOS source | Android source | Value |
|---|---|---|---|
| `associationTimeoutMs` / `ASSOCIATION_TIMEOUT_MS` | `NextWordEngine.swift:27` | `NextWordEngine.kt:31` | `10_000` (ms; strict-`<` window) |
| `contextTimeoutSeconds` / `CONTEXT_TIMEOUT_MS` | `NextWordEngine.swift:30` (`TimeInterval` 30.0) | `NextWordEngine.kt:38` (`30_000L`) | 30 s (representations differ — see §6) |
| `userWeight` / `USER_WEIGHT` | `NextWordScorer.swift:18` (`50.0` Double) | `NextWordScorer.kt:22` (`50` Int) | 50 |
| `dictWeight` / `DICT_WEIGHT` | `NextWordScorer.swift:19` (`1.0`) | `NextWordScorer.kt:25` (`1`) | 1 |
| `decayHalfLifeHours` | both | both | `168.0` (1 week) |
| `learningBonus` / `LEARNING_BONUS` | both | both | `300.0` |
| `highUsageDecayFloor` / `HIGH_USAGE_DECAY_FLOOR` | both | both | `0.95` |
| `lowUsageDecayFloor` / `LOW_USAGE_DECAY_FLOOR` | both | both | `0.30` |
| `highUsageThreshold` / `HIGH_USAGE_THRESHOLD` | both | both | `3` |
| `ln2` (Android-only constant) | implicit `0.693` literal at `calculateDecay` | `NextWordScorer.kt:46` (`LN_2 = 0.693`) | `0.693` (3-digit literal — preserve precision) |

**Behavioral invariants pinned**: `behavioral-invariants.md` §§7 (ranking weights), §8 (decay half-life). The Rust port preserves both labels via Rust workspace tests; iOS / Android `INVARIANT_*` tests reroute through `RustEngineBridge.nextword*` (parity-test commit).

---

## 4. Behavior table (decide function — locked from G5-design / A5-design)

For `WordSelected(text, roman, requireRomanMode, triggerPrediction)`:

| Condition | Outcome effects | newState mutation |
|---|---|---|
| `requireRomanMode && settings.isTranslateSwapped` | `[]` | unchanged |
| `text` empty | `[]` | unchanged |
| `text` is noise punctuation, NOT sentence-end | `[]` | unchanged |
| `text` is sentence-end punctuation | `[CancelContextTimeout, ClearPredictionsUI(gen) if isShowing]` | reset to defaults + bump generation |
| `settings.isAssociationRecordingEnabled && shouldRecordAssociation && lastSelectedWord != nil` | `[RecordAssociation, RecordCompoundAssociations] (if any)` | — |
| Always (for valid text) | append `[RescheduleContextTimeout(30000ms)]` | `lastSelectedWord/Roman = ...`, `lastSelectionTimeMs = nowMs`, bump generation |
| `triggerPrediction == true` | append `[QueryPredictions(textTl, romanTl, newGen, nowMs)]` | — |

For `Backspace(lastChar)`:

- newState: `lastSelectedWord = lastChar, lastSelectedRoman = nil, lastSelectionTimeMs = nowMs, generation += 1`
- effects: `[QueryPredictions(lastChar, "", newGen, nowMs)]`
- **Never** emits `RecordAssociation` or `RecordCompoundAssociations`.

For `ContextTimeoutFired` / `ResetFull`:
- newState: `NextWordPersistedState.initial` with bumped generation.
- effects: `[CancelContextTimeout, ClearPredictionsUI(newGen) if was-isShowing]`.

For `ClearForNewComposing`:
- newState: `isShowing = false`, generation += 1.
- effects: `[ClearPredictionsUI(newGen) if was-isShowing else nothing]`.

`shouldRecordAssociation(state, nowMs) = state.lastSelectedWord != nil && (nowMs - state.lastSelectionTimeMs) ∈ [0, 10_000)` — strict-`<` upper, non-negative lower.

---

## 5. Intentional cross-platform divergences (preserved by Rust port — DO NOT silently fix)

Documented in `nextword-engine-boundary.md` §13 (Android addendum). The Rust engine must reproduce these branches per platform — settings field on the proto request distinguishes them where needed:

| # | Concern | iOS behavior | Android behavior | Resolution in Rust port |
|---|---|---|---|---|
| 1 | `splitCompound` separator | `-` only | `-` and whitespace | Add per-platform branch via `AppConfig.platform_id` enum (`PLATFORM_IOS = 1`, `PLATFORM_ANDROID = 2`); see plan §3 |
| 2 | `noisePunctuation` set | iOS limits to actual punctuation chars | Android superset includes ASCII space / full-width space / digits | Per-platform branch in `is_noise_text` |
| 3 | `filterPredictions` POJ↔TL dispatch | `inputMode == .poj` → `tlToPoj` | `inputMode == "tl"` (string) → use TL as-is, else `tlToPoj` | Engine reads `inputMode` enum; same outcome (convert iff POJ) |
| 4 | `EnginePrediction.score` | iOS drops score | Android keeps for `TaigiWord.lengthScore` | Proto carries `score: double` always; iOS bridge ignores it |
| 5 | `NextWordHandler.updateLastSelectedWord` (Space path) | Goes through `WordSelected(triggerPrediction=false)` → records, reschedules timeout, bumps gen | Records compound only (no `prev→this`), no timer reschedule, no gen bump | **Explicit Rust intent `UpdateLastSelectedWord`** — Android-only call site routes through Rust engine; intent preserves "compound-only, no timer, no gen bump" semantics. iOS wrappers never emit this intent. Codex P1 v1: cannot stay platform direct mutation once Rust owns `PersistedState` (otherwise platform shadow-state diverges from canonical Rust state). |

Divergences #1–4 are engine-internal and resolved via `AppConfig.platform_id`. #5 is now an Android-only Rust intent (`UpdateLastSelectedWord`) — the Rust engine accepts it generically; iOS bridge never calls it. This was originally proposed as "stays platform direct state mutation" but Codex pre-impl review v1 (P1) flagged that approach: with `NextWordEngine.kt` deleted, Android would have no state to mutate directly without shadowing Rust's canonical `PersistedState`. Adding an explicit intent preserves the divergent behavior cleanly without dual-source-of-truth.

**Optional convergence flag (out of scope this slice)**: a future "remove platform_id divergence" round may unify #1 + #2 once user accepts the behavior change. Tracked as roadmap follow-up, not v3.5.5 work.

---

## 6. Time representation across the boundary

iOS engine uses `TimeInterval` (`Double` seconds) for `RescheduleContextTimeout(after:)`; Android uses `Long` ms. The proto carries `uint64 after_ms` — single representation; iOS bridge converts ms → seconds for `Timer.scheduledTimer(withTimeInterval:)`. The 30-second invariant value is unchanged.

`nowMs` is `Int64` ms on both platforms and on the wire — no conversion needed. The rust engine and the platform executor share ONE `nowMs` per intent: the executor reads `System.currentTimeMillis()` / `Date().timeIntervalSince1970 * 1000` once at intent entry, passes into `Request.nextword.<intent>.now_ms`, and reuses the same value when the `QueryPredictions` effect fires the SQLite query (Android boundary §13.3).

---

## 7. Anti-pattern adjacency audit

Per slice scoping rule (`project_rust_migration_cadence.md`): catalogue per-codepoint scans / static engine-derived tables / engine helper utilities used only by NextWord that must be pulled into this slice.

**Checked locations**: NextWord engine + scorer + outcome + booster + service + handler + classifier (out of slice).

| Pattern | Location | Disposition |
|---|---|---|
| Per-codepoint string scan | None inside NextWord engine code path | N/A |
| Static engine-derived table on platform | None — scoring constants are pure literals; `noisePunctuation` set is a Foundation/Kotlin literal (moves to Rust) | N/A |
| Helper utility scoped to NextWord | `AutocompleteContextBooster.boost` — already in slice | Pulled in |
| Phonetic conversion calls inside NextWord | `RustEngineBridge.pojToTl` / `tlToPoj` (platform side) | Become **internal Rust** calls (`phonetics::api::*`) — no FFI |
| Static engine-derived table on platform | `NextWordHandler.companion.extractCurrentWord` uses ASCII-punctuation literal | **Stays platform** — UI-side trailing-word extraction, not engine concern |

**Conclusion**: no anti-pattern adjacencies leak outside the slice scope. The slice boundary is clean.

---

## 8. Persistence boundary justification (Option A)

Per cadence release map (decided 2026-04-27): v3.5.5 = NextWord predictions; v3.5.6 = Lexicon (SQLite + Trie behind FFI). v3.5.5 ships **engine logic only**; SQLite + assoc.bin mmap reader stay on platform.

**Why**:

1. **Cadence alignment** — v3.5.6 is dedicated to SQLite/Trie/`fst`/`rusqlite` integration. Pulling it forward into v3.5.5 would either collapse two slices into one mega-PR or duplicate the SQLite work. Both violate "one slice per release".
2. **Cross-platform DB I/O divergence** — iOS uses `SQLite3` C API + custom `SQLiteConnectionManager`; Android uses `SQLiteDatabase` + `compileStatement`; both have their own migration policies (iOS schema in `NextWordSchema.swift`, Android multi-version in `NextWordService.migrateUserDb`). Unifying on `rusqlite` is a big jump that deserves dedicated review.
3. **Effect-level interface is sufficient** — engine emits `QueryPredictions(word, roman, gen, nowMs)` and `RecordAssociation(pair)` / `RecordCompoundAssociations(pairs)`. Platform executor interprets these against existing `NextWordService` query / record APIs — exact pre-v3.5.5 IO behavior preserved.
4. **`scoreDict` / `calculateUserScore` swap is in scope** — these are pure-math primitives currently called from `NextWordService.predict`. The slice folds them into the Rust `FilterPredictions` path which scores raw rows engine-side (plan §3.4). SQLite query stays platform; scoring math becomes Rust.

**Boundary contract for v3.5.5** (DB ownership preserved; row DTO + ranking boundary changes):
- Engine: pure decision + scoring math + merge + sort + limit + filter; emits effects describing IO intent.
- Platform: interprets effects → SQLite query / record / mmap lookup → returns un-merged un-scored rows tagged by `Source` (DICT / USER) with `count` + `last_used_ms`.
- Engine: filter step takes `[RawNextWordPrediction]` + `query_generation` + `now_ms` + `limit` + settings → groups by `(hanzi, tl)`, scores dict/user separately, sums scores on conflict, sorts desc by score, applies limit → `[EnginePrediction]` (or empty if stale).

**Note on row DTO change**: pre-v3.5.5 `NextWordService.predict` returned merged-scored rows. Post-v3.5.5 returns un-merged un-scored rows (`hanzi`, `tl`, `count`, `last_used_ms`, `source`). This shrinks platform `NextWordService` responsibility — SQL query stays SQLite-on-platform, but ranking/scoring math lives in Rust. The platform DB CRUD ownership is unchanged.

**v3.5.6 will subsume**: `NextWordService` → `lexicon::NextWordPersistence` Rust crate; `NextWordSchema` migrations → `rusqlite` schema; `AssociationBinaryReader` mmap → `fst`-or-mmap-equivalent in Rust; `NextWordRepository` CRUD → Rust query layer.

---

## 9. Slice scope decisions (locked under auto-mode)

| Q | Decision | Rationale |
|---|---|---|
| Q1 — DB boundary | **Option A**: engine logic Rust, SQLite + mmap stay platform | Cadence map (v3.5.6 = Lexicon) + DB-driver divergence |
| Q2 — `AutocompleteInputClassifier` | **OUT** | Different concern (autocomplete input-type triage), already calls `RustEngineBridge` for TPS work; future autocomplete slice |
| Q3 — `AutocompleteContextBooster` | **IN** | Single consumer (NextWord-derived `predictedFirstChars`); pure boost reorder; small (33 LOC iOS + 40 LOC Android) |
| Q4 — `EnginePrediction` | **Rust** (proto type) | Filter logic ports together; `iOS` will pass it through, `Android` will read `score` field |
| Q5 — Engine handle | **New `engine/nextword` crate with own `EngineHandle`** | Mirrors composing crate pattern (separate `Mutex<Engine>` per stateful slice; dispatch routes via `nextword::EngineHandle::instance()`). Reusing composing's handle would couple two unrelated state machines under one mutex — rejected after re-reading `engine/composing/src/handle.rs`. **Surfaced as scope delta § A.** |
| Q6 — Proto pattern | Sealed shape mirrors `composing.proto` — `oneof method` + `oneof effect` | Codex sandwich already approved this shape in v3.5.4 |
| Q7 — Branch / PR | `phase4b/v3.5.5-nextword-slice`, single PR, 14 commits (split per Codex v1 P3) | Same pattern as v3.5.4 |
| Q8 — `AutocompleteService` / `Providers` / `English` / `TaigiAutocompleteService` | **OUT** | Belong to a future autocomplete slice |

### Scope delta surfaced to user before Codex review (per round playbook)

**§ A — Engine handle pattern revision**: the auto-mode pre-decision said "reuse Composing EngineHandle". After reading `engine/composing/src/handle.rs` and `engine/dispatch/src/lib.rs`, the established codebase pattern is **one EngineHandle singleton per stateful slice crate** (composing has its own; ranking is stateless and uses a free function; phonetics is stateless). NextWord introduces its own `engine/nextword::EngineHandle` instead of bolting NextWord state into the composing crate. This is the more surgical pattern; ASKED USER ABOVE.

**§ B — `AppConfig` deltas**: NextWord engine reads three settings fields not yet on `AppConfig`:
- `is_translate_swapped: bool` (existing — used by composing slice already, present in current AppConfig)
- `is_association_recording_enabled: bool` (NEW — gates association recording inside `decideWordSelected`)
- `platform_id: enum { IOS, ANDROID }` (NEW — branches divergences §5 #1, #2)
The first is already there. The two new fields are additive; no breaking change.

---

## 10. Cross-references

- iOS boundary contract: `nextword-engine-boundary.md` §§1–12.
- Android binding addendum: `nextword-engine-boundary.md` §13.
- Cadence release map: `project_rust_migration_cadence.md`.
- Rust workspace rules: `rules/rust-best-practices.md`.
- Cross-platform alignment: `rules/cross-platform-alignment.md` §1c (shared-core-candidate constraint), §3a (invariant comments), §5.1 (Rust shared-core non-goals).
- Behavioral invariants: `docs/architecture/behavioral-invariants.md` §§7, 8.
- Composing slice as reference: `docs/engine/composing-slice-{audit,plan}.md`; `engine/composing/src/{lib,handle,dispatch,api,transition,derived}.rs`.
- Codex sandwich pattern: `feedback_codex_review_sandwich.md`.
