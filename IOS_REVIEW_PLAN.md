# iOS Localization Refactor Plan

Target: `ios/Sources/TaigiKeyboard/Localization/`

> **Review sources**: Initial review + Codex reuse review + quality review + efficiency review (2026-04-14)

## Stage 1: Remove Dead Code
**Goal**: Delete ~18 unused properties across all Tab*Texts files
**Success Criteria**: All remaining properties have at least one reference outside their definition file
**Tests**: Build succeeds, no compiler errors

### Properties to remove

**KeyboardTexts.swift**
- `confirmKey` — this is the only property; **delete the entire file** after removal

**Tab1Texts.swift**
- `setupGuideStartSetup`
- ~~`viewWebsite`~~ — **FALSE POSITIVE**: used in `CopyrightView.swift:147`

**Tab2Texts.swift**
- `appearanceDescription`
- `layoutDescription`
- `colorSettings`
- `colorCandidateSection`

**Tab3Texts.swift**
- `dictionarySettings`
- `entriesCount`
- `importExportHelpTitle`
- `importExportHelp`
- `viewWebsite`
- `customDictionarySource`
- `frequentWordsManagement`
- `frequencyTab`
- `associationTab`
- `totalEntries`
- `backupHelpTitle`
- `backupHelp`

**Tab4Texts.swift**
- `resetSuccess`

**Status**: Not Started

---

## Stage 2: Extract Shared Strings
**Goal**: Consolidate 14 cross-file duplicate strings into a shared enum
**Success Criteria**: The 13 listed duplicate strings are each defined in exactly one place (CommonTexts)
**Tests**: Build succeeds, UI displays identical text as before

### Create `CommonTexts.swift`

> **Naming**: Android already has `CommonTexts.kt` with a comment `對應 iOS CommonTexts.swift`. Use `CommonTexts` (not `SharedTexts`) for cross-platform alignment.

Extract dictionary names used by both Tab1 (copyright) and Tab3 (dictionary management):

| Shared property | Value | Currently in |
|---|---|---|
| `moeDict` | 教育部臺灣台語常用詞辭典 | Tab1, Tab3 |
| `newwordDict` | 公視台語台台語新詞辭庫 | Tab1, Tab3 |
| `kunggeDict` | 工藝中心臺灣台語工藝詞庫 | Tab1, Tab3 |
| `iTaigiDict` | iTaigi愛台語 | Tab1, Tab3 |
| `taiwanJapanDict` | 臺日大辭典台語譯本 | Tab1, Tab3 |
| `taiHuaDict` | 台華線頂對照典 | Tab1, Tab3 |
| `taiwanPlantDict` | 台灣植物名彙 | Tab1, Tab3 |
| `sttiDict` | 教育部學科術語臺灣台語對譯 | Tab1, Tab3 |
| `accentDict` | 腔口差 | Tab1 (`accentDict`), Tab3 (`khpooDict`) |
| `viewWebsite` | 官方網站 | Tab1, Tab3 |
| `cancel` | 取消 | Tab3, Tab4 |
| `fontOpenHuninn` | 粉圓 | Tab1 (`openFontTitle`), Tab2 |
| `fontIansui` | 芫荽 | Tab1 (`iansuiFontTitle`), Tab2 |

Update Tab1/Tab2/Tab3/Tab4 to reference `CommonTexts.*` instead of local duplicates.

**Status**: Not Started

---

## Stage 3: Consolidate Same-File Duplicates
**Goal**: Reduce identical strings within the same file
**Success Criteria**: No two properties in the same enum have identical hanji values (unless intentionally separate)
**Tests**: Build succeeds, UI unchanged

### Tab3Texts.swift
- `importResult` / `frequencyImportResult` / `associationImportResult` all = "匯入 %d 項成功，%d 項重複"
  - Consolidate to single `importResultFormat`
  - Update all call sites

### Tab2Texts.swift
- `colorSettings` / `colorKeyboardBackground` both = "齒盤色水"
  - Keep `colorKeyboardBackground`, remove `colorSettings` (already in Stage 1 unused list)

### Intentionally kept separate (same value now, may diverge):
- Tab4: `pojMode` / `pojSettingsSectionTitle` (label vs section header)
- Tab1: `moeCopyright` / `sttiCopyright` (different copyright holders happen to share publisher)

> Note: Tab3 `dictionarySettings` ("詞庫管理") duplicates `tabTitle` but is already in the Stage 1 removal list (unused).

**Status**: Not Started

---

## Stage 4: Separate Icon Constants from Tab4Texts
**Goal**: Move SF Symbol icon names out of the "Texts" enum
**Success Criteria**: Tab4Texts contains only `LocalizedText` properties
**Tests**: Build succeeds, icons render correctly

### Create `SettingsIcons.swift`

> **Naming**: Android already has `SettingsIcons.kt`. Use `SettingsIcons` (not `Tab4Icons`) for cross-platform alignment.

Move these constants:
- `isOutputBothScriptsIcon`
- `autoCapitalizationIcon`
- `autoSpaceIcon`
- `toolbarIcon`
- `globeKeyIcon`
- `soundFeedbackIcon`
- `vibrationFeedbackIcon`

Update references in Tab4.swift and SettingsSelectionOverlay.swift.

**Status**: Not Started

---

## Stage 5: Align Naming Inconsistency
**Goal**: Fix property naming inconsistency for the same concept
**Success Criteria**: Same dictionary uses same property name everywhere
**Tests**: Build succeeds

### Fixes
- Tab3 `khpooDict` -> align with Tab1 `accentDict` naming (will be in CommonTexts after Stage 2)
- Tab1 `openFontTitle` / Tab2 `fontOpenHuninn` -> unified in CommonTexts after Stage 2

**Status**: Not Started

---

## Stage 6: Remove Redundant Comments
**Goal**: Delete comments that just restate what the code already says
**Success Criteria**: Only non-obvious "why" comments remain
**Tests**: Build succeeds

### Comments to remove

**LocalizedText.swift**
- Line 12: `/// 便利初始化（直接傳入字串）` — the convenience init is self-evident

**Tab3Texts.swift**
- Lines 84-109: Doc comments like `/// 教育部臺灣台語常用詞辭典` on properties whose value is literally that string
  - `moeDict`, `newwordDict`, `kunggeDict`, `iTaigiDict`, `taiwanJapanDict`, `taiHuaDict`, `taiwanPlantDict`, `sttiDict`, `khpooDict`, `lkkDict`

### Comments to keep
- MARK headers (structural navigation, not explanatory)
- File-level comments explaining scope (e.g., `// 包含：頭頁、啟用方法...`)
- `LocalizedText.swift` line 4: `/// 本地化文字結構（簡化版：僅支援漢字）` — explains design intent

**Status**: Not Started

---

## Stage 7: Simplify LocalizedText + LanguageManager (User Decision Required)
**Goal**: Remove the pass-through abstraction layer if multi-language is not planned
**Success Criteria**: Strings accessed directly as `String` without wrapper, or abstraction justified with concrete plan
**Tests**: Build succeeds, all 20+ View files updated

> **Quality review finding**: `LocalizedText` is a single-field wrapper, `LanguageManager.text()` just returns `.hanji`. This is YAGNI — 20+ Views pay `@StateObject` overhead for a manager that does nothing. Overlay files already bypass it via `.hanji`.
>
> **Efficiency review finding**: The `@StateObject` observation cost is negligible (never publishes), but the cognitive overhead across 100+ call sites is real.

### Option A: Remove wrapper (if multi-language is NOT planned)
- Replace `LocalizedText(hanji: "...")` with plain `String` constants
- Remove `LanguageManager` class and `withLanguageEnvironment()` extension
- Remove all `@StateObject private var languageManager` declarations (~20 View files)
- Replace `languageManager.text(SomeTexts.property)` with `SomeTexts.property`
- Also update `CustomDictionaryService.swift` (lines ~249-253) which uses `LanguageManager.shared.text()`

### Option B: Keep wrapper (if multi-language IS planned)
- Unify access: all consumers use `.hanji` directly (drop `LanguageManager.text()`)
- Remove `LanguageManager` class (it adds nothing over direct access)
- Remove all `@StateObject private var languageManager` declarations (~20 View files)
- Also update `CustomDictionaryService.swift` (lines ~249-253)
- Keep `LocalizedText` struct for future multi-field expansion
- Standardize on one init style (`init(hanji:)` or `init(_:)`, not both)

**Status**: Not Started — awaiting user decision on multi-language plans

---

## Out of Scope (Noted, Not Planned)

1. **`View.withLanguageEnvironment()` extension**: Only used in one place (`TaigiKeyboardApp.swift`). Could be inlined but harmless. Will be removed naturally if Stage 7 proceeds.

---

## Execution Notes

- Each stage is independently committable
- Stage 1 must come first (removes noise before extraction)
- Stages 2-6 can be done in order or selectively
- Stage 7 requires user decision before starting
- User must manually add any new files (e.g., CommonTexts.swift, SettingsIcons.swift) to the Xcode project
- No test changes needed (static string constants, no logic)

## Review Notes (2026-04-14)

### Efficiency review summary
- All `static let` properties are lazily initialized by Swift runtime — no eager loading concern
- Keyboard extension only touches Tab2Texts and Tab4Texts — Tab1/Tab3 never loaded in extension
- `versionHistoryEntries` array is dispatch_once lazy — only allocated when VersionHistoryDetailView opens
- `CommonTexts` extraction introduces no new memory cost (same lazy static pattern)
- No performance-sensitive changes in this plan
