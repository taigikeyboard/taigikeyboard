---
name: clean-comments
description: Clean, standardize, and improve code comments across iOS and Android source files. Removes outdated/redundant comments, translates non-English comments, and ensures comments aid AI searchability. Use when you want to clean up comments in a specific file, module, or the full project.
disable-model-invocation: true
---

# Clean Comments

Audit and clean code comments in source files. Remove stale/redundant comments, standardize to English, and ensure remaining comments add value for both humans and AI tools.

## Arguments

The user may provide:
- A **file path** (e.g., `ios/Sources/.../ToneConverter.swift`) to scope to a single file
- A **module name** (e.g., `Tone`, `Composing`, `Autocomplete`) to scope to that module's files on both platforms
- `--ios` or `--android` to limit to one platform
- `--translate-only` to only find and translate non-English comments (skip other checks)
- No argument = analyze all source files on both platforms

## Steps

### 1. Determine scope

**If a file path is provided:**
- Analyze that file only.

**If a module name is provided:**
- Read `docs/file-structure.md` and extract the iOS + Android files for that module.

**If no argument:**
- Collect all source files under:
  - `ios/Sources/TaigiKeyboard/` (`.swift`)
  - `android/app/src/main/java/com/siansiansu/taigikeyboard/` (`.kt`)

Apply `--ios` / `--android` filters if provided.

### 2. Read files

Read all in-scope files in parallel. For each file, understand the code context around every comment — a comment's relevance depends on the code it describes.

### 3. Analyze each comment

For each comment in every file, classify it into one of the following categories:

#### 3a. Outdated Comments

Comments that describe behavior or code that no longer exists or has changed:
- References to removed functions, variables, or classes
- Describes old logic that has been refactored
- Contains version-specific notes for versions no longer supported
- TODO/FIXME for issues that have already been resolved (no issue number, and the described problem is fixed)

#### 3b. Redundant Comments

Comments that add no value beyond what the code itself communicates:
- Restates the function/variable name (e.g., `// Set the value` above `setValue()`)
- Describes obvious syntax (e.g., `// Loop through items` above `for item in items`)
- Empty or placeholder comments (e.g., `// TODO`, `// ...`, `// MARK: -` with no label)
- Commented-out code blocks (not documentation — actual dead code)

**Exceptions — do NOT flag these as redundant:**
- `// MARK:` section headers with meaningful labels
- Comments explaining **why** (business logic, workarounds, non-obvious decisions)
- Comments linking to external resources, bug reports, or specs
- Comments on regex patterns, bitwise operations, or complex algorithms
- Protocol conformance / framework requirement explanations

#### 3c. Non-English Comments

Comments written in Chinese (Traditional/Simplified), Japanese, or any non-English language.

**Translation rules:**
- Translate the comment to concise, natural English
- Preserve the comment style (`//`, `///`, `/* */`, `/** */`)
- Preserve any code references within the comment (function names, variable names, etc.)
- **Domain terms to preserve as-is** (do not translate these):
  - Taiwanese linguistic terms: POJ, TL, TPS, Hanji (漢字)
  - Input method terms: 方音符號 (TPS layout name)
  - Proper nouns: 教育部 (MOE), 台語 (Taigi/Taiwanese)
- For ambiguous terms, add the original in parentheses: `// Tone sandhi rules (變調規則)`

#### 3d. Inconsistent Style

Comments that don't follow the project's conventions:
- Mixed `//` and `/* */` for single-line comments within the same file
- Inconsistent spacing after `//` (e.g., `//comment` vs `// comment`)
- Doc comments (`///`) on private/internal helpers that don't need them
- Missing doc comments (`///`) on public API that should have them

#### 3e. AI Searchability Improvements

Comments that could be improved to help AI tools (Grep, Explore) find relevant code:
- Functions with non-obvious names that lack a brief description of what they do
- Complex conditional blocks with no summary comment
- Entry points and key integration points lacking a brief role description
- **Add keyword-rich comments** at module boundaries, e.g.:
  ```swift
  // MARK: - Tone Conversion (POJ/TL tone marks, numeric tone → diacritic)
  ```

**Rules for new comments:**
- Only suggest additions at module/section boundaries, not on every function
- Keep additions to one line
- Use `// MARK:` for Swift section headers
- Use `// region` / `// endregion` for Kotlin section headers
- Must include terms from `docs/keywords.md` where applicable

### 4. Cross-reference check

Before finalizing findings:
- For outdated comments referencing symbols, search all in-scope files to confirm the symbol truly no longer exists
- For redundant comments, verify the comment doesn't contain hidden context (e.g., "Set the value — triggers observer" has useful info after the dash)
- For non-English comments, verify the translation is accurate by understanding the surrounding code

### 5. Produce the report

Output a structured report:

```markdown
## Clean Comments Report

**Scope**: [file path | module name | full]
**Mode**: [full | translate-only]
**Date**: [today]
**Files analyzed**: [count]
**Total comments found**: [count]

### Summary

| Category                  | Count | Action    |
|---------------------------|-------|-----------|
| Outdated comments         | N     | Remove    |
| Redundant comments        | N     | Remove    |
| Commented-out code        | N     | Remove    |
| Non-English comments      | N     | Translate |
| Style inconsistencies     | N     | Fix       |
| Searchability additions   | N     | Add       |
| **Total**                 | N     |           |

### Findings

#### [FileName.swift / FileName.kt]

| #  | Line(s) | Category    | Current Comment              | Proposed Action | New Comment (if applicable) |
|----|---------|-------------|------------------------------|-----------------|----------------------------|
| 1  | 15      | Outdated    | `// Used by OldManager`      | Remove          | —                           |
| 2  | 42      | Non-English | `// 處理聲調轉換`              | Translate       | `// Handle tone conversion` |
| 3  | 78-82   | Dead code   | `// func oldHelper() { ... }`| Remove          | —                           |
| 4  | 100     | Redundant   | `// Returns the count`       | Remove          | —                           |
| 5  | —       | Searchability | (none)                     | Add before L120 | `// MARK: - Tone Conversion (POJ/TL tone marks)` |

#### [Next file...]
...
```

### 6. Ask for user approval

Present the report and ask the user:
1. Which categories to apply (all? only translations? specific items?)
2. Any items to skip or exclude
3. Review proposed translations for accuracy

Do NOT modify any files until the user confirms.

### 7. Apply changes

After user approval:

1. Apply approved changes file by file, starting from the bottom of each file (highest line numbers first) to avoid line number shifts.
2. For each file:
   - Remove approved outdated, redundant, and commented-out code
   - Replace non-English comments with approved translations
   - Fix style inconsistencies
   - Add approved searchability comments
3. After all changes:
   - Verify no code was accidentally removed (only comments should change)
   - Check that remaining comments still align with adjacent code

### 8. Summary

After applying changes, output a summary:

```markdown
## Changes Applied

| File                    | Removed | Translated | Style Fixed | Added | Lines Changed |
|-------------------------|---------|------------|-------------|-------|---------------|
| ToneConverter.swift     | 3       | 2          | 1           | 1     | 7             |
| InputNormalizer.kt      | 1       | 4          | 0           | 0     | 5             |
| **Total**               | 4       | 6          | 1           | 1     | 12            |
```

### 9. Suggest /sync-docs

If comment changes revealed terminology shifts, renamed concepts, or removed features, suggest running `/sync-docs` to update documentation accordingly. List the specific docs that might be affected.

## Important

- **Never modify code** — only comments should change. If a line contains both code and a trailing comment, only modify the comment portion
- **Never remove `// MARK:` section headers** with meaningful labels — only remove empty ones
- **Never remove TODO/FIXME with issue numbers** (e.g., `// TODO(#123)`) — these track real work
- **Never remove license headers or copyright notices**
- **Never remove comments that explain "why"** — only remove comments that restate "what"
- **Preserve domain terms** — 方音符號, 漢字, POJ, TL, TPS should remain as-is or be parenthetically noted in translations
- **When translating, match the technical level** — don't simplify technical comments
- **When in doubt about a comment's relevance, keep it** — false removal is worse than keeping a borderline comment
- **Commented-out code is different from commented documentation** — only remove actual dead code, not prose descriptions that happen to use code-like syntax
- Follow the project's existing code style and comment conventions
