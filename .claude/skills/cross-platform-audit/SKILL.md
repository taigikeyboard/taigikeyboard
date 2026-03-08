---
name: cross-platform-audit
description: Compare iOS and Android implementations to find gaps, divergences, and redundant logic. Use when you want to check cross-platform alignment for a specific module (IME engine or main app), recent changes, or the full project.
disable-model-invocation: true
---

# Cross-Platform Audit

Identify implementation gaps between iOS and Android, detect redundant logic, and produce a refactoring plan aligned to a chosen source of truth.

## Arguments

The user may provide:
- An **IME module name** (e.g., `Tone`, `Lexicon`, `Autocomplete`, `Composing`) to scope the audit to IME core
- An **App module name** (e.g., `App`, `Tab1`, `Tab2`, `Tab3`, `Tab4`, `Theme`, `Localization`) to scope the audit to the main app
- `--changes` to audit only files changed on the current branch vs main
- `--app` to audit only the main app (all tabs, theme, localization, shared components)
- `--engine` to audit only the IME engine (composing, autocomplete, lexicon, tone, etc.)
- No argument = full audit (both IME engine AND main app)

## Steps

### 1. Determine scope

**If a module name is provided:**
- Read `docs/file-structure.md` and extract the iOS ↔ Android file pairs for that module only.
- For App modules (`Tab1`–`Tab4`), include both the tab fragment/view files AND the corresponding `TabNTexts` localization files.

**If `--changes` is provided:**
- Run `git diff main..HEAD --name-only` to get changed files.
- Read `docs/file-structure.md` and match changed files to their cross-platform counterparts.
- If a changed file has no counterpart listed, flag it as a potential gap.

**If `--app` is provided:**
- Read `docs/file-structure.md` and extract all App Page file pairs:
  - Tab Structure (Tab1–Tab4, ContentView ↔ SettingsMainActivity)
  - Tab1 Sub-pages (SetupGuide, FeatureDetail, etc.)
  - Localization (Tab1Texts–Tab4Texts, LocalizedText, DisplayLanguage, LanguageManager)
  - Shared Components (ImageSlideshowView, LocalizedTextView, ListCard)
  - Theme files
  - Other App Files (Onboarding, Debug, DictionarySettings)

**If `--engine` is provided:**
- Read `docs/file-structure.md` and extract all IME Core file pairs (Composing, Autocomplete, Lexicon, Tone, UserFrequency, NextWord sections).

**If no argument:**
- Read `docs/file-structure.md` and extract ALL file pairs — both IME Core AND App Page sections.

### 2. Read file pairs thoroughly

For each iOS ↔ Android pair in scope, read both files in parallel.

**Deep-read directive** — Do not skim. For each file:
- Read the entire file, not just function signatures
- Trace the full call chain from each public function to its internal helpers
- Note every guard clause, early return, and edge-case branch
- Identify all constants, thresholds, and configuration values
- Understand the data flow: what comes in, how it transforms, what goes out

Also read:
- `docs/keywords.md` — to use consistent terminology in the report
- Relevant specs under `docs/engine/` if they exist for the module (e.g., `docs/engine/tone.md` for the Tone module)
- Relevant specs under `docs/ui/` if they exist for app modules (e.g., `docs/ui/app-ui.md`, `docs/ui/theme.md`)

### 3. Compare implementations

For each file pair, perform three levels of comparison:

#### 3a. API Surface Comparison

Extract and compare public/internal function signatures side by side:

```
| iOS (Swift)                          | Android (Kotlin)                     | Match? |
|--------------------------------------|--------------------------------------|--------|
| func normalizeInput(_ raw: String)   | fun normalizeInput(raw: String)      | YES    |
| func applyToneMark(...)              | fun applyToneMarks(...)              | NAME   |
| func restoreTone(...)                | —                                    | GAP    |
```

Check for:
- **Function name mismatches** — same logic but different method names (e.g., `applyToneMark` vs `applyToneMarks`)
- **Parameter name mismatches** — same parameter with different names (e.g., `rawInput` vs `input`)
- **Property/variable name mismatches** — same concept with different names (e.g., `trieKey` vs `searchKey`)
- **Missing functions** — function exists on one platform but not the other
- **Signature differences** — different parameter types, return types, or parameter order for the same function

#### 3b. Logic Comparison

For each matched function pair, perform these concrete checks:

**Call-graph audit** — List every internal function/method call inside the function body on each platform. Flag any call present on one side but missing on the other.

Example of what to catch:
```
| iOS ToneConverter.convertToToneMarks()       | Android ToneConverter.convertToToneMarks()  |
|----------------------------------------------|---------------------------------------------|
| calls preprocessPojInput()                   | calls preprocessPojInput()             ✅   |
| calls TaigiPhonetics.convertToToneMarks()    | calls TaigiPhonetics.convertToToneMarks() ✅|
| calls adjustNasalMarkerCase()                | — (MISSING CALL)                       ❌   |
```

**Parameter forwarding audit** — For each downstream call, verify the same parameters are passed. Flag cases where a parameter exists but is not forwarded.

Example of what to catch:
```
| iOS buildSearchKey()                         | Android buildSearchKey()                    |
|----------------------------------------------|---------------------------------------------|
| SyllableSegmenter.segment(input, mode: mode) | SyllableSegmenter.segment(input)       ❌   |
|                                              | ↳ `mode` exists but not passed              |
```

**Guard/validation audit** — Compare early-return conditions, validation checks, and guard clauses line by line. Flag any validation present on one platform but absent on the other.

Example of what to catch:
```
| iOS normalize()                              | Android normalize()                         |
|----------------------------------------------|---------------------------------------------|
| validates via isValidPrefix(base, mode)      | — (NO VALIDATION)                      ❌   |
| returns "" for invalid mode input            | returns segments without mode check    ❌   |
```

**Platform API semantics check** — When both platforms call similarly-named APIs, verify they produce the **same observable behavior**. iOS and Android platform APIs with similar names can have different semantics. Flag cases where code was copied across platforms without accounting for this.

Example of what to catch:
```
| Behavior: "remove composing text"         | iOS                              | Android                            |
|-------------------------------------------|----------------------------------|------------------------------------|
| API used                                  | setMarkedText("") + unmarkText() | setComposingText("", 1)            |
| Effect on committed text                  | marked text is committed first   | composing region removed directly  |
| Extra deleteBackward() needed?            | YES (to remove committed text)   | NO (text already gone)             |
| ↳ Having deleteSurroundingText here       | correct                          | BUG — deletes extra character      |
```

For each matched function that touches platform text APIs (InputConnection, TextDocumentProxy, marked/composing text), explicitly verify: does the same sequence of API calls produce the same user-visible result on both platforms?

**Algorithm and constants check**:
- Are the algorithms equivalent?
- Are edge cases handled the same way?
- Are constants and thresholds the same? (e.g., max candidates, score weights)
- Is the control flow equivalent? (early returns, guard clauses, error handling)

#### 3c. Test Coverage Comparison

Check iOS test files under `ios/TaigiKeyboardTests/` against Android test files (if any) under `android/app/src/test/` or `android/app/src/androidTest/`:

- **Missing test files** — iOS has tests but Android doesn't (or vice versa)
- **Missing test cases** — specific scenarios tested on one platform but not the other
- **Different test data** — same test scenario but using different input/expected values
- **Test naming** — compare naming patterns (iOS: `test{Component}_{scenario}`, Android should match)

List test coverage as:

```
| Test Scenario                        | iOS                  | Android              |
|--------------------------------------|----------------------|----------------------|
| normalizeInput with numeric tone     | InputNormalizerTests | —  (MISSING)         |
| toneConverter basic POJ              | ToneConverterTests   | ToneConverterTest    |
```

#### 3e. App UI Comparison (for App modules only)

When comparing main app files (tabs, theme, localization, shared components), apply these additional checks beyond 3a–3b:

**Section structure audit** — Compare the sections/cards displayed on each tab. Both platforms should show the same sections in the same order.

```
| Section          | iOS Tab1.swift | Android Tab1Fragment.kt | Status |
|------------------|----------------|-------------------------|--------|
| Setup Guide      | ✅ section 1   | ✅ section 1            | MATCH  |
| Feature List     | ✅ section 2   | ✅ section 2            | MATCH  |
| Version History  | ✅ section 5   | —                       | GAP    |
| FAQ              | ✅ section 3   | ✅ section 4            | ORDER  |
```

**Navigation and content audit** — Compare list items, navigation destinations, and interactive elements:
- Same list items / cards on both platforms?
- Same navigation targets (sub-pages, detail views)?
- Same interactive elements (toggles, pickers, buttons)?
- Any feature present on one platform but missing on the other?

**Localization content audit** — Compare `TabNTexts.swift` ↔ `TabNTexts.kt` pairs:
- Same number of entries / text keys?
- Same content for each corresponding entry? (Compare `hanji` field values)
- Same version history entries in `versionHistoryEntries`?
- Any entry present on one platform but missing on the other?

**Theme consistency audit** — Compare theme definitions:
- Same color tokens / palette across both platforms?
- Same dark mode / light mode logic?
- Same font settings and sizes?
- Same corner radius, spacing, and other design tokens?
- Flag visual inconsistencies, not implementation differences (e.g., SwiftUI Color vs Android ColorRes is structural, but different hex values is a divergence)

**Shared component audit** — For components listed in `docs/file-structure.md` Shared Components:
- Same props / parameters?
- Same behavior (e.g., slideshow timing, auto-advance)?
- Same visual output?

#### 3f. Classify findings

Categorize all findings from 3a–3e into:

| Category | Definition | Example |
|----------|-----------|---------|
| **Gap** | Feature/logic exists on one platform but not the other | `TPSConverter` is iOS-only |
| **Divergence** | Same feature, different logic or behavior | Different tone normalization rules |
| **Missing call** | Function exists on both platforms but one side omits an internal call | Android `ToneConverter` doesn't call `adjustNasalMarkerCase()` |
| **Missing param** | Downstream function is called but a parameter is not forwarded | Android `buildSearchKey()` doesn't pass `mode` to `segment()` |
| **Missing guard** | Validation/guard clause present on one platform but absent on the other | Android `normalize()` lacks `isValidPrefix()` check |
| **Redundancy** | Duplicate logic that could be simplified on one or both sides | Same helper function reimplemented differently |
| **Naming mismatch** | Same concept, inconsistent naming across functions, params, or variables | `ToneCharacterUtils` (Android) vs `ToneUtilities` (iOS) |
| **Test gap** | Test exists on one platform but not the other | `InputNormalizerTests` iOS-only |
| **Content mismatch** | Same UI section but different text content, list items, or entries | Tab1 version history has different entries |
| **Section gap** | UI section/card exists on one platform but not the other | Tab1 FAQ section missing on Android |
| **Order mismatch** | Same sections exist on both but in different display order | Tab1 sections ordered differently |
| **Theme divergence** | Same design token but different values (color hex, font size, spacing) | Primary color `#3A7BD5` (iOS) vs `#2196F3` (Android) |
| **API semantic mismatch** | Same API pattern copied across platforms but different platform semantics cause different behavior | `deleteSurroundingText` after clearing composing text deletes extra char on Android but not iOS |
| **Structural difference** | Unavoidable platform difference (framework, lifecycle, etc.) | KeyboardKit vs FlorisBoard, SwiftUI vs Fragment/XML |

For each finding, note:
- Which files are involved (full paths)
- Specific function/variable names involved
- Brief description of what differs
- Whether this is an intentional platform difference or an unintended gap

### 4. Produce the report

Output a structured report with this format:

```markdown
## Cross-Platform Audit Report

**Scope**: [module name | changed files | engine | app | full]
**Date**: [today]
**Files compared**: [count]

### Summary

| Category          | Count |
|-------------------|-------|
| Gaps              | N     |
| Divergences       | N     |
| Missing calls     | N     |
| Missing params    | N     |
| Missing guards    | N     |
| Content mismatches| N     |
| Section gaps      | N     |
| Order mismatches  | N     |
| Theme divergences | N     |
| Redundancies      | N     |
| Naming mismatches | N     |
| Test gaps         | N     |
| API semantic       | N     |
| Structural (OK)   | N     |

### API Surface Comparison

#### [Module Name]

**[FileName.swift] ↔ [FileName.kt]**

| iOS Function/Property | Android Function/Property | Status |
|-----------------------|---------------------------|--------|
| `func normalizeInput(_:)` | `fun normalizeInput(...)` | MATCH |
| `func restoreTone(_:)` | — | GAP |
| `var trieKey: String` | `val searchKey: String` | NAME MISMATCH |

### App UI Comparison (if app modules in scope)

#### Tab Structure

| Tab | iOS Sections (in order) | Android Sections (in order) | Status |
|-----|-------------------------|----------------------------|--------|
| Tab1 | Setup, Features, FAQ, Feedback, Version History | Setup, Features, FAQ, Feedback | SECTION GAP (Version History) |
| Tab2 | ... | ... | ... |

#### Localization Content

**Tab1Texts.swift ↔ Tab1Texts.kt**

| Entry Key / Topic | iOS `hanji` | Android `hanji` | Status |
|-------------------|-------------|-----------------|--------|
| setupGuideTitle | "安裝指南" | "安裝指南" | MATCH |
| featureListTitle | "功能介紹" | — | GAP |

#### Theme Tokens

| Token | iOS Value | Android Value | Status |
|-------|-----------|---------------|--------|
| primaryColor | `#3A7BD5` | `#3A7BD5` | MATCH |
| cardCornerRadius | `12` | `8` | DIVERGENCE |

### Naming Mismatches

| # | iOS Name | Android Name | Type | Location |
|---|----------|-------------|------|----------|
| 1 | `ToneUtilities` | `ToneCharacterUtils` | file | Tone module |
| 2 | `applyToneMark()` | `applyToneMarks()` | function | ToneConverter |
| 3 | `rawInput` | `input` | parameter | InputNormalizer.normalizeInput |

### Internal Call Mismatches

| # | Function | iOS Calls | Android Calls | Status |
|---|----------|-----------|---------------|--------|
| 1 | `ToneConverter.convertToToneMarks()` | `adjustNasalMarkerCase(result)` | — | MISSING CALL |
| 2 | `buildSearchKey()` | `segment(input, mode: mode)` | `segment(input)` | MISSING PARAM |
| 3 | `normalize()` | `isValidPrefix(base, mode)` guard | — | MISSING GUARD |

### Logic Divergences

| # | Function | iOS Behavior | Android Behavior | Suggested SOT |
|---|----------|-------------|-----------------|---------------|
| 1 | `normalizeInput` | strips tone marks then lowercases | lowercases then strips | needs review |

### Test Coverage

| Test Scenario | iOS | Android | Status |
|---------------|-----|---------|--------|
| normalizeInput with numeric tone | InputNormalizerTests | — | GAP |
| toneConverter basic POJ | ToneConverterTests | ToneConverterTest | MATCH |

### Gaps

| # | Component | iOS | Android | Description | Suggested SOT |
|---|-----------|-----|---------|-------------|---------------|
| 1 | TPSConverter | exists | missing | TPS romanization conversion | iOS |

### Platform-Specific (Intentional)

- list items that are correctly platform-specific and need no action
```

**SOT** = Source of Truth. Use one of: `iOS`, `Android`, `needs review`, `both OK` (when both are correct but different).

### 5. Ask user for source-of-truth decisions

For any finding marked `needs review`, present the specific divergence and ask the user which platform should be the source of truth.

Do NOT proceed to planning until the user has confirmed SOT choices.

### 6. Generate refactoring plan

After SOT decisions are confirmed, create `IMPLEMENTATION_PLAN.md` with staged tasks:

```markdown
# Cross-Platform Alignment Plan

**Source**: cross-platform-audit on [date]
**Branch**: [current branch or suggest new branch name]

## Stage 1: [Module — e.g., "Align Tone module naming"]
**Goal**: [specific deliverable]
**SOT**: iOS
**Type**: Naming | Logic | Test | Gap
**Files to change**:
- `path/to/target/file` — align with `path/to/source/file`
**Renames** (if any):
- `oldFunctionName()` → `newFunctionName()` (match iOS)
- `oldParamName` → `newParamName`
**Logic changes** (if any):
- bullet list of specific logic changes
**Tests to add/align** (if any):
- port `testScenarioName` from iOS `TestFile.swift`
**Success Criteria**: [testable outcome]
**Status**: Not Started

## Stage 2: ...
```

Group stages by module. Order by dependency (e.g., InputNormalizer before LexiconService).

### 7. Update docs after implementation

After the refactoring plan has been implemented (all stages completed), update the relevant documentation under `./docs/`:

1. **Read existing docs** — Check which files under `docs/` are affected by the changes:
   - `docs/file-structure.md` — update if files were added, removed, or renamed
   - `docs/keywords.md` — update if new terms were introduced or existing terms renamed
   - `docs/engine/*.md` — update the spec for any module whose logic, API, or behavior changed
   - `docs/ui/*.md` — update if UI-related behavior changed

2. **Update each affected doc** — For each doc that needs changes:
   - Reflect new/renamed functions, parameters, or properties
   - Update code examples and signatures to match the new implementation
   - Add or remove sections for added/removed features
   - Ensure cross-references between docs remain valid

3. **Verify consistency** — After updates:
   - All function/class names in docs match the actual codebase
   - No references to old names or removed features remain
   - `docs/file-structure.md` iOS ↔ Android file pairs are accurate

Do NOT create new doc files unless a wholly new module was added. Prefer updating existing docs.

## Important

- Do NOT modify any source files — this skill produces reports and plans only
- Structural differences are expected — flag them as "Structural (OK)" and move on. Examples:
  - IME: KeyboardKit vs FlorisBoard patterns
  - App: SwiftUI View vs Fragment/XML, NavigationLink vs Intent/Activity
  - Theme: SwiftUI Color vs Android ColorRes/theme.xml (implementation differs, but hex values should match)
  - Localization: iOS in-code localization vs Android strings.xml (mechanism differs, but TabNTexts content should match)
- When comparing logic, focus on **behavior** not syntax — a Swift `guard let` and Kotlin `?.let` may implement the same logic
- Use `docs/keywords.md` terminology in all findings (e.g., "trieKey" not "search string")
- If `docs/file-structure.md` is missing a file pair that clearly corresponds, note it as a gap in the spec itself
