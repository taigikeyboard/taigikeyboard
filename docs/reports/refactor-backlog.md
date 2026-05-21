# Refactor Backlog

> **Type**: Report
> **Created**: 2026-03-21
> **Last re-validated**: 2026-05-22 (post-v3.5.8 ship `61df3028`; audit folded from v3.5.9 refactor plan §1B/§1C/§1D)
> **Active tracking**: [`docs/reports/2026-05-18-v3.5.9-refactor-plan-draft.md`](2026-05-18-v3.5.9-refactor-plan-draft.md) §4 Tier B (slice tags **B1–B12**, except **B2** which is roadmap/`flow.md` doc work tracked only in the plan, not in this backlog)
> **Supersedes** (post 2026-05-22 re-validation): the plan §4 row "B8a `cached()` fallback hardening" framing is **superseded** by this backlog — the `PrefHelper.cached():137` ANR fix is owned by **B11**; **B8** covers `SharedSettings`/`PrefHelper` typed-key wrapper + TPS sync transition only. When the two docs disagree, this backlog is the canonical slice-ownership map.
> **Constraint**: All items below carry refactoring risk — they touch core logic, state management, or high-traffic code paths. Each slice ships as its own branch + PR with Codex sandwich (pre + post) and `Skill(simplify)` per `~/.claude/rules/round-workflow.md § Codex review sandwich`.

---

## iOS

### HIGH — Core Logic Risk

#### SharedSettings: typed-key cleanup + bidirectional TPS ↔ InputMode sync
- **Tracked as**: **B8** (cross-platform Settings slice, iOS half)
- **File**: `ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift`
- **Problems** (two related issues handled in one slice — spec §1B iOS-6 + §8 disposition row "iOS 2"):
  - **Raw-key boilerplate**: 40 raw-string keys at `:25-67` + manual per-property getter/setter throughout (e.g. `inputMode` at `:83-104`, `keyboardLayoutType` at `:173-193`). App-Group + KK-store-proxy constraint blocks a naive `@AppStorage` swap; needs a typed-key wrapper instead.
  - **Bidirectional sync**: Setting `inputMode` (`:83-104`) triggers `keyboardLayoutType` setter (`:173-193`), which may trigger `inputMode` again. Hidden cascading state dependencies; settings are read on the continuous-input hot path (`ComposingManager.swift:110-113`), so any logic change requires platform dogfood.
- **Fix direction**: Typed-key wrapper for the 40 keys; explicit state-transition methods (`enterTPSMode()` / `exitTPSMode()`) replacing the cross-setter side effects.
- **Caution**: continuous-input hot path; dogfood required.

#### RustEngineBridge.swift: God-file split
- **Tracked as**: **B3**
- **File**: `ios/Sources/TaigiKeyboard/Engine/RustEngineBridge.swift` (**1493 LOC**); existing extensions `RustEngineBridge+CaseTransform.swift` (144) + `RustEngineBridge+Lexicon.swift` (471) already follow the pattern.
- **Problem**: Single file mixes Phonetics / Composing / NextWord / Lexicon / CaseTransform FFI surfaces.
- **Fix direction**: Domain split by FFI domain into per-extension files (Phonetics / Composing / NextWord). Each new file needs maintainer Xcode-target add per `feedback_xcode_manual`.

### MEDIUM — State & Structure

#### TaigiButtonContent: Deep Nesting
- **Status**: LOW (no slice) — current ≈ 2–3 levels with helpers extracted; not worth a dedicated PR.
- **File**: `ios/Sources/TaigiKeyboard/Styling/TaigiButtonContent.swift` (171 LOC)

#### ActionHandler: NextWord State Coupling + Long Functions
- **Status**: LOW (downgraded) — `NextWordController.swift` extracted; `ActionHandler.swift` now 213 LOC + 4 cohesive extensions (`+Suggestions` 231, `+KeyActions` 278, `+Utilities` 21, `+CustomActions` 34). Original "4 loose properties" + "71-line gesture handler" largely mitigated.
- **File**: `ios/Sources/TaigiKeyboard/Actions/ActionHandler.swift`

---

## Android

### HIGH — Boilerplate & Safety

#### PrefHelper: ~755 LOC, 46-Property Boilerplate
- **Tracked as**: **B6**
- **File**: `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/PrefHelper.kt` (**755 LOC**)
- **Problem**: 46 properties repeat identical get/set boilerplate against DataStore (~600 LOC). Adding a new preference requires copy-pasting 12 lines.
- **Risk**: Read on every keystroke and UI update. Incorrect delegate = settings not persisted or wrong defaults. Must preserve persisted keys.
- **Fix direction**: Kotlin `PreferenceProperty<T>` property delegate encapsulating get/set/cache pattern; target ≈ 400 LOC after collapse.

#### PrefHelper: TPS ↔ InputMode Sync (mirrors iOS)
- **Tracked as**: **B8** (cross-platform Settings slice, Android half)
- **File**: `android/.../ime/core/PrefHelper.kt:201-249,325-370`
- **Problem**: Same bidirectional sync issue as iOS `SharedSettings`; duplicated across two setters.
- **Fix direction**: Same shape as iOS — explicit transition methods. (Note: `PrefHelper.cached():137` ANR locus is owned by **B11**, not by B8 — they are related Settings work but separate slices. See header "Supersedes" note.)

#### RustEngineBridge.kt: God-file split
- **Tracked as**: **B4**
- **File**: `android/.../engine/RustEngineBridge.kt` (**2230 LOC**)
- **Problem**: Single file mixes all FFI surfaces (Phonetics / Composing / NextWord / Lexicon / CaseTransform).
- **Fix direction**: Domain split by FFI domain; Gradle files editable by Claude per `feedback_gradle_editable`. Target ≈ 5 files, each ≤ 500 LOC.

#### PrefHelper.cached() — blocking-DataStore fallback (ANR locus)
- **Tracked as**: **B11** (canonical owner of the `PrefHelper.kt:137` ANR fix; B8 references this as related Settings work but does not duplicate)
- **File**: `android/.../ime/core/PrefHelper.kt:137`
- **Precise locus** (audit AND-3, Codex-verified reachability): `runBlocking { dataStore.data.map { it[key] ?: default }.first() }` runs per-read on the calling thread when `cachedPrefs == null`. Any property read on Main before the cache is filled hits this blocking fallback. Of the 9 non-IME `PrefHelper` ctors found on main, **7 do not call `warmUp()` and are therefore unsafe paths**:
  - `FrequencyDataViewModel.kt:27`
  - `CustomDictionaryViewModel.kt:21`
  - `AssociationDataViewModel.kt:27`
  - `DictionarySearchViewModel.kt:43` (reads happen later, during search — still on Main if not dispatched)
  - `SettingsMainActivity.kt:60`
  - `DetailActivity.kt:23`
  - `CopyrightActivity.kt:22`
  - The two safe ctors (`TaigiKeyboardApplication.kt:36` and `AppearanceSettingsActivity.kt:16`) do invoke `warmUp()` immediately after construction and are not in scope.
- **Fix direction**: Replace per-read `runBlocking` with a non-blocking default-or-suspend path; ensure non-IME ctors cannot hit a Main-thread block. The `warmUp()` path itself (one-time `runBlocking{…first()}` at IME `onCreate`) is defensible and not the locus.

### MEDIUM — File Size & Architecture

#### TextInputManager.kt Decomposition (worsened 794 → 1275 LOC)
- **Tracked as**: **B5**
- **File**: `android/.../ime/text/TextInputManager.kt` (**1275 LOC**, **42 `fun` declarations** including nested / lambda-internal) — worsened from the 794 LOC originally noted in this backlog.
- **Problem**: Core key-event logic mixed with 6 handler delegations.
- **Fix direction**: Extract `KeyEventDispatcher` + mode/layout coordinator; orchestrator delegates only. Hot path — qualitative S1/S2/S3 dogfood per slice required.

#### SmartbarManager: Boolean State → Enum
- **Tracked as**: **B7**
- **File**: `android/.../ime/text/smartbar/SmartbarManager.kt` (**690 LOC**)
- **Problem**: **5 boolean state vars** (`isComposingEnabled:48`, `isExpanded:71`, `hasCandidates:72`, `cachedIsTranslateSwapped:78`, `cachedOutputBothScripts:79`) interact with complex visibility logic. (Spec §4 B7 quoted "4 flags"; current main has 5 — drift from a post-spec addition. Either count is correct for its snapshot; the slice scope still covers the full enum replacement.)
- **Fix direction**: Replace with a `SmartbarState` enum (e.g. EMPTY / COLLAPSED / EXPANDED) + explicit transition rules. Hot path (user-visible UI state) — dogfood required.

#### Candidate-strip Compose stability
- **Tracked as**: **B10**
- **Files**: `android/.../ime/text/smartbar/CandidateStripState.kt:17,26` (48 LOC; holds `List<TaigiWord>`); `TaigiWord.kt:33` (holds `Map`); Compose stability config (to be added).
- **Problem (audit AND-1, Codex-sharpened)**: `@Stable`/`@Immutable` used only 2× across 86 `@Composable` → recomposition skipping likely suboptimal on the per-keystroke candidate strip. This is **NOT "add annotations"** — the existing models hold `List`/`Map`, so naive `@Immutable` would be aspirational.
- **Fix direction**: (1) Generate Compose-compiler stability reports + add a stability config (or immutable-collection strategy); (2) implement the stability changes the report demands — which may include annotating proven-immutable models, switching to immutable collections, or splitting the state. Step 1 can run any time; step 2 needs the report and is per-keystroke UI (dogfood).

#### Android logging facade consistency
- **Tracked as**: **B12**
- **Problem (audit AND-5)**: **22 files** bypass the existing facade — 20 of them via `import android.util.Log` + 2 via fully-qualified `android.util.Log.*` calls (`engine/LexiconBridge.kt`, `engine/CaseTransformBridge.kt`). The 21st `import android.util.Log` user is `AndroidLoggerBackend.kt`, which IS the facade's backend implementation and is NOT a bypass. iOS `os.Logger` is uniform — Android is the asymmetry. (Spec §1C AND-5 quoted 23 files; current main is 22 actual bypassers; −1 organic drift since 2026-05-18.)
- **Fix direction**: Mechanical routing of the 22 raw-`Log` bypassers through the facade. Treat the 2 fully-qualified callers the same as the 20 raw-import users.

---

## Modern-but-stable component adoption (audit-folded from v3.5.9 plan §1D)

| Modern-stable component | iOS state | Android state | Slice |
|---|---|---|---|
| Fine-grained reactive state | `@Observable` × **0**; `ObservableObject` × **15**; `@Published` × **49** | `@Volatile` × **17 total** in `android/app/src/main`; B9 targets the **5-field `ComposingManager` hot cluster** (`ComposingManager.kt:74,77,80,83,101`) | **B9** (cross-platform VM-state pair) |
| Compose stability | n/a | `@Stable`/`@Immutable` × **2** across 86 `@Composable` | **B10** |
| Typed settings store | `SharedSettings` raw keys `:25-67` + manual getters | `PrefHelper` raw-key (B6) + blocking `cached():137` fallback (B11) + TPS sync (B8) | **B6** / **B8** / **B11** (Android side) ; **B8** (iOS side, paired) |
| Unified logging | ✅ `os.Logger` uniform | ⚠️ Facade + **22** files bypass (20 raw `import` + 2 fully-qualified) | **B12** |
| Strict concurrency | Swift 5 language mode; Swift 6 essentially unstarted | Healthy (no `GlobalScope`, `StateFlow` + `collectAsStateWithLifecycle`) | iOS = **maintainer build-setting track**, NOT a Claude slice; Android = no action |

---

## Cross-platform mirror pairs (one slice / two sub-PRs)

Per v3.5.9 plan §1D Codex Q6: mirror pairs ship as a single cross-platform slice with platform sub-PRs (parity acceptance shared; implementation order split by risk).

| Slice | iOS half | Android half |
|---|---|---|
| **B8** Settings | `SharedSettings` typed-key wrapper (`:25-67`) + sync-transition methods (`:83-104`, `:173-193`) | `PrefHelper` TPS↔InputMode sync transition (`:201-249,325-370`); `PrefHelper.cached():137` ANR fix is owned by **B11**, related but separate |
| **B9** VM-state | `ObservableObject` / `@Published` cluster → `@Observable` view-model layer (NOT the KeyboardKit-10 Combine case-guard at `KeyboardViewController.swift:217`) | `ComposingManager` 5-field `@Volatile` cluster → single immutable `StateFlow<UiState>` snapshot |

---

## Documented hazards — NOT a forced refactor

These are documented in the v3.5.9 plan but deliberately excluded from Tier B; capturing here so future readers don't re-propose them as backlog items.

- **iOS — sync FFI architecture** (plan §1B iOS-4): `RustEngineBridge` static `enum` + sync dispatch on the keyboard thread is acceptable for a sub-ms engine. A rewrite to an async engine daemon re-opens architecture out of scope.
- **iOS — KeyboardKit-10 auto-cap FIXME** (plan §1B iOS-5): 3-layer Combine guard at `KeyboardViewController.swift:202,217` + `ActionHandler.swift:146`. Continuous-input adjacent (case handling). Document, do not force.
- **iOS — Swift 6 strict concurrency** (plan §1B iOS-2): pbxproj user-only per `feedback_xcode_manual`; this is a maintainer build-setting track, not a Claude code PR.
- **Android — View + Compose hybrid IME** (plan §1C AND-6): 63 `findViewById`/`AppCompatActivity`/`Fragment` overlay Views alongside 86 `@Composable`. Deliberate (`KeyPopupManager.kt:143` popup-Compose workaround; Compose-in-IME-window has known quirks). Document only.

---

## Recommended approach

1. **One slice per round** per `~/.claude/rules/round-workflow.md § Branching & rounds`. Each slice = own branch + PR.
2. **Codex sandwich pre + post + `Skill(simplify)`** per `~/.claude/rules/round-workflow.md § Codex review sandwich` and `~/.claude/rules/code-review-rules.md §8`. `/codex-pr-review` user-trigger before merge per `~/.claude/rules/round-workflow.md § Codex review sandwich step 6`.
3. **PR sizing 200–500 LOC** per `~/.claude/rules/planning.md § Persistent hand-off`. Bridge splits (B3, B4) and `TextInputManager` decompose (B5) likely 2–3 sub-PRs each.
4. **Cross-platform mirror pairs** (B8, B9) = one slice, two sub-PRs per spec §1D Codex Q6.
5. **Hot-path slices** (B5, B7, B8, B9, B10 step 2) require qualitative S1/S2/S3 dogfood per `~/.claude/rules/code-review-rules.md §9 Performance Gate Philosophy`.
6. **Slice scope, ordering, and release tag are user-gated** per `~/.claude/rules/diagnosis-discipline.md § No unilateral release scope`. Do not pre-commit which Tier B slices land in which release.
7. **Behavior-changing refactor** (scorer unification, single cost model, etc.) is **Tier C → v3.6+**, NOT v3.5.9. Do not bundle into Tier B.
