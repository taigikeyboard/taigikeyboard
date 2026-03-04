---
name: code-simplifier
description: Find and remove unused code, variables, functions, and simplify redundant implementations without affecting functionality. Use when you want to clean up a specific module, changed files, or the full project.
disable-model-invocation: true
---

# Code Simplifier

Identify unused code, dead variables, redundant logic, and unnecessary complexity. Produce a report, then apply changes after user approval.

## Arguments

The user may provide:
- A **file path** (e.g., `ios/Sources/.../ToneConverter.swift`) to scope to a single file
- A **module name** (e.g., `Tone`, `Composing`, `Autocomplete`) to scope to that module's files on both platforms
- `--dead-code` to only find and remove unused functions, variables, imports, and files (skip redundant logic, simplifications, and commented-out code analysis)
- `--ios` or `--android` to limit to one platform
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

Read all in-scope files in parallel. For each file, also identify its test file (if one exists) to understand which symbols are tested.

### 3. Analyze each file

**If `--dead-code` is provided:** Only check categories 3a, 3b, 3c, and 3g-i (orphan tests). Skip 3d, 3e, 3f, 3g-ii, 3g-iii, 3g-iv. Also search for entire files that are unused (not imported/referenced by any other in-scope file).

For each file, check for the following categories:

#### 3a. Unused Imports

- Imports that are not referenced anywhere in the file body.
- For Swift: `import` statements. For Kotlin: `import` statements.

#### 3b. Unused Variables and Properties

- Local variables assigned but never read.
- Properties declared but never accessed (within the file or by other in-scope files).
- Parameters that are accepted but never used in the function body.
- **Caution**: Properties may be accessed via reflection, protocol conformance, or framework callbacks. Flag these as "likely unused" rather than "definitely unused" unless you can confirm no external reference.

#### 3c. Unused Functions and Methods

- Private/internal functions that are never called within the file or module.
- For `public`/`open` functions, search across all in-scope files before flagging.
- **Caution**: Functions may be called by the framework (e.g., `override` methods, protocol conformance, `@objc` selectors, Android lifecycle methods). Never flag these.

#### 3d. Dead Code Paths

- Code after unconditional `return`, `throw`, `break`, or `continue`.
- Conditions that are always true/false based on constant values.
- `else` branches that duplicate the `if` branch.
- Empty `catch`/`else`/`default` blocks that could be removed.

#### 3e. Redundant Logic

- Duplicate code blocks within the same file (3+ lines of identical or near-identical logic).
- Overly verbose patterns that have idiomatic simplifications:
  - Swift: `if x != nil { return x! }` → `return x`, verbose `guard` chains, etc.
  - Kotlin: unnecessary `?.let { it }`, redundant `.toString()` on String, etc.
- Boolean expressions that can be simplified (e.g., `if condition { return true } else { return false }` → `return condition`).
- Unnecessary intermediate variables (assigned once, used once on the next line, with no clarity benefit).
- Wrapper functions that just forward to another function with no added logic.

#### 3f. Commented-Out Code

- Blocks of commented-out source code (not documentation comments).
- Stale TODO/FIXME comments with no associated issue number.

#### 3g. Redundant Tests

Analyze all test files (`*Tests.swift`, `*Test.kt`) in scope. This project uses parametric arrays `[(input, expected)]` with loops — pay special attention to duplicate entries within and across arrays.

##### 3g-i. Orphan Tests

- Test methods that reference functions, types, or symbols that no longer exist in production code.
- Test files whose corresponding production file has been deleted.
- Flag as HIGH confidence — these tests are definitively dead.

##### 3g-ii. Duplicate Test Cases

- **Within a single parametric array**: identical `(input, expected)` pairs appearing more than once.
- **Across test methods in the same file**: different test methods that assert the exact same input → output behavior on the same function.
- **Across test files**: different test files covering the same function with overlapping `(input, expected)` pairs. Cross-reference by matching the production function under test.

##### 3g-iii. Subset Tests

- A test method whose assertions are a strict subset of another test method (every case in test A also appears in test B).
- A parametric array that is a strict subset of another parametric array testing the same function.
- The smaller/subset test is the one flagged for removal.

##### 3g-iv. Trivially Redundant Assertions

- Multiple `XCTAssertEqual` / `assertEquals` calls in the same test method that assert the same expression with the same input (copy-paste errors).
- Assertions that test default/identity behavior already guaranteed by the type system (e.g., asserting an empty string maps to an empty string when the function signature makes this obvious).

### 4. Cross-reference check

Before finalizing findings:
- For any symbol flagged as "unused", search all in-scope files (not just the current file) to confirm it's truly unreferenced.
- Check test files — a function only called in tests is still "used".
- Check protocol/interface conformance — required methods must stay.
- Check framework entry points — `override` methods, `@IBAction`, lifecycle callbacks are NOT unused.

#### 4a. Transitive dead code analysis (always applied in `--dead-code` mode)

Single-level reference checks miss chains of dead code. Perform transitive analysis:

1. **Identify dead files** — files not imported/referenced by any live file (entry points such as `@main`, `AppDelegate`, `KeyboardViewController`, `Activity`, `Fragment`, `Service` subclasses are always considered live).
2. **Exclude dead file references** — remove all references originating from dead files from the reference graph.
3. **Re-check symbols** — symbols that were previously considered "used" may now have zero live references. Flag them as unused.
4. **Repeat** until no new dead files or symbols are found (fixed-point iteration).

Example: if `PageX.swift` is a dead file and `funcA()` is only called by `PageX`, then `funcA()` is also dead code.

Report dead files as a separate category with the full dependency chain:
```
| #  | Category   | File / Symbol          | Only referenced by (dead) | Confidence |
|----|------------|------------------------|---------------------------|------------|
| 1  | Dead file  | PageX.swift            | (none — no live caller)   | HIGH       |
| 2  | Transitive | funcA() in Utils.swift  | PageX.swift (dead)        | HIGH       |
```

### 5. Produce the report

Output a structured report:

```markdown
## Code Simplifier Report

**Scope**: [file path | module name | full]
**Mode**: [full | dead-code]
**Date**: [today]
**Files analyzed**: [count]

### Summary

| Category             | Count | Safe to Remove |
|----------------------|-------|----------------|
| Unused imports       | N     | N              |
| Unused variables     | N     | N              |
| Unused functions     | N     | N              |
| Dead files           | N     | N              |
| Transitive dead code | N     | N              |
| Dead code paths      | N     | N              |
| Redundant logic      | N     | N              |
| Commented-out code   | N     | N              |
| Redundant tests      | N     | N              |
| **Total**            | N     | N              |

### Findings

#### [FileName.swift / FileName.kt]

| #  | Category          | Line(s) | Symbol / Code                    | Confidence | Action        |
|----|-------------------|---------|----------------------------------|------------|---------------|
| 1  | Unused import     | 3       | `import Foundation`              | HIGH       | Remove        |
| 2  | Unused function   | 45-60   | `private func oldHelper()`       | HIGH       | Remove        |
| 3  | Redundant logic   | 78-82   | verbose nil check → simplify     | MEDIUM     | Simplify      |
| 4  | Unused property   | 12      | `private var cache: [String]`    | LOW        | Verify & remove |

#### [Next file...]
...
```

**Confidence levels**:
- **HIGH** — definitely unused/redundant, safe to remove with no side effects
- **MEDIUM** — very likely unused, but verify no reflection/dynamic access
- **LOW** — possibly unused, but could be accessed by framework or external code

**Actions**:
- **Remove** — delete the code entirely
- **Simplify** — replace with a shorter idiomatic equivalent (show the replacement in the report)
- **Verify & remove** — needs manual confirmation before removal

### 6. Ask for user approval

Present the report and ask the user:
1. Which findings to apply (all HIGH? all HIGH + MEDIUM? specific items?)
2. Any items to skip or exclude

Do NOT modify any files until the user confirms.

### 7. Apply changes

After user approval:

1. Apply approved changes file by file, starting from the bottom of each file (highest line numbers first) to avoid line number shifts.
2. For each file:
   - Remove approved unused imports, variables, functions, dead code, and commented-out code.
   - Apply approved simplifications (show before/after for each).
3. After all changes:
   - Run the project's linter/formatter if available.
   - Verify no new compiler errors were introduced by reviewing changed files.

### 8. Summary

After applying changes, output a summary:

```markdown
## Changes Applied

| File                  | Removals | Simplifications | Lines Removed |
|-----------------------|----------|-----------------|---------------|
| ToneConverter.swift   | 3        | 1               | 25            |
| InputNormalizer.kt    | 1        | 2               | 8             |
| **Total**             | 4        | 3               | 33            |
```

## Important

- **Never remove `override`, `@objc`, protocol-required, or lifecycle methods** — even if they appear unused, the framework calls them
- **Never remove public API** without explicit user approval — it may be used by other modules not in scope
- **Never change behavior** — simplifications must be semantically equivalent
- **Never remove test helpers** — functions used only in test files are still used
- **When removing redundant tests, keep the more comprehensive test** — remove the subset/duplicate, not the superset
- **Cross-file test dedup requires matching the function under test** — do not flag tests as duplicates just because they share similar input values but test different functions
- **When in doubt, flag as LOW confidence** rather than auto-removing
- **Preserve all documentation comments** — only remove commented-out code, not doc comments (`///`, `/** */`, `//MARK:`)
- Follow the project's existing code style when simplifying
