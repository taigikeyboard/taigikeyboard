---
name: log-audit
description: Audit logging across iOS and Android for format consistency, debug-only visibility, and convention compliance. Use when you want to check log hygiene for a specific module, recent changes, or the full project.
disable-model-invocation: true
---

# Log Audit

Scan logging statements across iOS and Android to check format consistency, debug-only visibility, and adherence to project conventions. Produce a report with actionable findings.

## Arguments

The user may provide:
- A **module name** (e.g., `Tone`, `Lexicon`, `Autocomplete`, `Composing`) to scope to that module's files
- A **file path** to scope to a single file
- `--changes` to audit only files changed on the current branch vs main
- `--ios` or `--android` to limit to one platform
- No argument = audit all source files on both platforms

## Project Logging Conventions

These are the expected conventions. The audit checks compliance against them.

### iOS

| Rule | Convention |
|------|-----------|
| **Framework** | `os.Logger` only — no `print()`, `NSLog`, or `debugPrint` |
| **Log level** | Use `.debug()` for development traces so they are stripped in release builds. Use `.error()` / `.fault()` only for genuine errors |
| **Declaration** | One `Logger` per class (instance property) or per file (file-level `private let`). Never create a `Logger` inside a function body |
| **Subsystem** | `"com.taigikeyboard"` |
| **Category** | Match the class or file name (e.g., `category: "ToneConverter"`) |
| **Message prefix** | `[UPPER_TAG]` matching the module (e.g., `[TONE]`, `[LAYOUT]`, `[NORMALIZE]`) |
| **Privacy annotation** | All `String` interpolations in `os.Logger` calls MUST include `privacy: .public` to avoid showing `<private>` in Console.app. Scalar types (Int, Bool, Double) are public by default and do not need the annotation |

### Android

| Rule | Convention |
|------|-----------|
| **Framework** | `android.util.Log` — no `println`, `print()`, or `System.out` |
| **Debug guard** | Every `Log.d()` and `Log.i()` call MUST be wrapped in `if (BuildConfig.DEBUG)`. `Log.w()` and `Log.e()` may omit the guard when logging genuine runtime errors |
| **Tag** | Use a `companion object` `TAG` constant (`private const val TAG = "ClassName"`) or `this::class.simpleName`. Do not use string literals inline |
| **Message prefix** | `[UPPER_TAG]` matching the module (e.g., `[TONE]`, `[LAYOUT]`, `[NORMALIZE]`), consistent with the iOS counterpart |

### Cross-platform

| Rule | Convention |
|------|-----------|
| **Matching prefixes** | iOS and Android log messages for the same operation should use the same `[TAG]` prefix |
| **No commented-out logs** | Remove commented-out log lines; do not leave them in the codebase |

## Steps

### 1. Determine scope

**If a file path is provided:**
- Audit that file only.

**If a module name is provided:**
- Read `docs/file-structure.md` and extract the iOS + Android files for that module.

**If `--changes` is provided:**
- Run `git diff main..HEAD --name-only` to get changed files.
- Filter to source files (`.swift`, `.kt`).

**If no argument:**
- Collect all source files under:
  - `ios/Sources/TaigiKeyboard/` (`.swift`)
  - `android/app/src/main/java/com/siansiansu/taigikeyboard/` (`.kt`)

Apply `--ios` / `--android` filters if provided.

### 2. Scan log statements

Read all in-scope files in parallel. For each file, extract every line that matches a logging call:

- **iOS**: `logger.debug(`, `logger.info(`, `logger.error(`, `logger.fault(`, `Logger(`, `print(`, `NSLog(`, `debugPrint(`
- **Android**: `Log.d(`, `Log.i(`, `Log.w(`, `Log.e(`, `Log.v(`, `println(`, `print(`, `System.out`

Record for each log statement:
- File path and line number
- Log level (debug / info / warning / error)
- Tag or category used
- Message prefix (if any)
- Whether it has a debug guard (Android)
- Whether it is commented out

### 3. Check each rule

#### 3a. Framework compliance

- **iOS**: Flag any `print()`, `NSLog()`, or `debugPrint()` call.
- **Android**: Flag any `println()`, `print()`, or `System.out` usage.

#### 3b. Debug-only visibility

- **iOS**: Flag any `Logger` call using a level higher than `.debug()` (i.e., `.info()`, `.notice()`, `.log()`) for development trace messages. `.error()` and `.fault()` are acceptable for genuine errors only — flag if used for trace/diagnostic messages.
- **Android**: Flag any `Log.d()` or `Log.i()` call NOT wrapped in `if (BuildConfig.DEBUG)`. Check the immediate enclosing statement, not a distant outer scope.

#### 3c. Logger declaration

- **iOS**: Flag any `Logger(subsystem:category:)` constructed inside a function body rather than as a class/file-level property.
- **iOS**: Flag any logger with a subsystem other than `"com.taigikeyboard"`.

#### 3d. Tag convention

- **Android**: Flag any `Log.*` call where the tag is a string literal instead of `TAG` or `this::class.simpleName`.
- **Android**: Flag any file that uses `Log.*` but has no `TAG` companion object constant (if using the `TAG` pattern).
- **iOS**: Flag any `Logger` whose `category` does not match the enclosing class or file name.

#### 3e. Message prefix consistency

- For each log message, check if it starts with a `[UPPER_TAG]` prefix.
- Flag messages missing a prefix.
- Flag inconsistent prefixes within the same file (e.g., mixing `[TONE]` and `[tone]`).

#### 3f. Cross-platform prefix alignment

- For each iOS ↔ Android file pair (from `docs/file-structure.md`), compare the set of `[TAG]` prefixes used. Flag any prefix that exists on one platform but not the other.

#### 3g. Commented-out logs

- Flag any commented-out log statement (`// Log.d(`, `// logger.debug(`, etc.).
- Include the file path, line number, and the commented-out content.

#### 3h. Log level appropriateness

Flag likely misuses:
- `Log.e()` / `logger.error()` used for non-error diagnostic messages
- `Log.i()` / `logger.info()` used for high-frequency per-keystroke traces that should be debug level

#### 3k. Privacy annotations (iOS only)

Apple's `os.Logger` redacts `String` interpolations as `<private>` by default — even during live streaming via Console.app or `log stream`. This makes debug logs unreadable without explicit `privacy: .public` annotations.

**Check every `logger.debug(` / `logger.info(` / `logger.error(` / `logger.warning(` call:**
- Flag any **String** interpolation `\(someString)` that is missing `privacy: .public`
- **Scalar types** (Int, Bool, Double, Float) are public by default — no annotation needed
- `String(describing:)` wrapping an enum/struct also produces a String — needs `privacy: .public`
- `error.localizedDescription` is a String — needs `privacy: .public`

**Severity**:
- Logs NOT inside `#if DEBUG` → **MEDIUM** (affects real-device debugging via Console.app)
- Logs inside `#if DEBUG` → **LOW** (only affects debug builds, but still shows `<private>`)

**Fix**: Add `privacy: .public` to all String interpolations. Example:
```swift
// Before (shows <private>)
logger.debug("[TAG] input='\(input)'")
// After (shows actual value)
logger.debug("[TAG] input='\(input, privacy: .public)'")
```

#### 3i. Duplicate logs

Detect log statements that log the same or nearly identical information:

- **Exact duplicates**: Two or more log calls with identical message strings in the same file or across files in the same module.
- **Semantic duplicates**: Log calls that differ only in minor wording but log the same variable/state at the same code path (e.g., `Log.d(TAG, "input: $input")` and `Log.d(TAG, "input value: $input")` in the same function).
- **Re-logged values**: A value logged at point A, then logged again at point B without any transformation in between (e.g., logging `raw` at the start of a function and logging it again 3 lines later with no change).

For each duplicate group, identify which statement to keep (the one with richer context or at the more useful location) and which to remove.

#### 3j. Redundant logs

Detect log statements that provide little or no diagnostic value:

- **Identity / no-op logs**: Logging a value that is obvious from context and adds no information (e.g., logging a constant, logging a variable immediately after assignment with the same name as the assignment).
- **Excessive granularity**: Multiple consecutive log calls that could be combined into a single, more informative statement. Example: 5 separate `Log.d` calls logging individual fields of the same object — should be one structured log.
- **Lifecycle noise**: Logging every lifecycle entry/exit (`onCreate`, `onDestroy`, etc.) when no diagnostic action depends on it. Flag as LOW — these are sometimes useful but often left from initial development.
- **High-frequency hot-path logs**: Log calls inside tight loops, per-keystroke handlers, or per-frame callbacks that would flood the log output even in debug mode. These degrade performance and obscure useful logs.

### 4. Produce the report

Output a structured report:

```markdown
## Log Audit Report

**Scope**: [file path | module name | changed files | full]
**Date**: [today]
**Files scanned**: [count]
**Total log statements found**: [count]

### Summary

| Check                          | Pass | Fail |
|--------------------------------|------|------|
| Framework compliance (3a)      | N    | N    |
| Debug-only visibility (3b)     | N    | N    |
| Logger declaration (3c)        | N    | N    |
| Tag convention (3d)            | N    | N    |
| Message prefix (3e)            | N    | N    |
| Cross-platform alignment (3f)  | N    | N    |
| Commented-out logs (3g)        | N    | N    |
| Log level appropriateness (3h) | N    | N    |
| Privacy annotations (3k)       | N    | N    |
| Duplicate logs (3i)            | N    | N    |
| Redundant logs (3j)            | N    | N    |

### Findings

#### [FileName.swift / FileName.kt]

| #  | Line | Check | Log Statement (truncated) | Issue | Severity |
|----|------|-------|---------------------------|-------|----------|
| 1  | 39   | 3b    | `Log.d(TAG, "Requested…")` | Missing `BuildConfig.DEBUG` guard | HIGH |
| 2  | 151  | 3c    | `let debugLogger = Logger(…)` | Logger created inside function body | MEDIUM |
| 3  | 398  | 3g    | `// Log.d("KeyboardInsets"…` | Commented-out log | LOW |

#### [Next file...]
...
```

**Severity levels**:
- **HIGH** — Log visible in production release (privacy/performance risk)
- **MEDIUM** — Convention violation that harms consistency or maintainability
- **LOW** — Minor style issue or cleanup opportunity

### 5. Ask user for approval

Present the report and ask the user:
1. Which findings to fix (all HIGH? all HIGH + MEDIUM? specific items?)
2. Any items to skip or exclude
3. For convention violations with no existing project standard, ask which convention to adopt

Do NOT modify any files until the user confirms.

### 6. Apply fixes

After user approval, apply fixes file by file:

**Framework compliance (3a)**:
- Replace `print()` / `NSLog` with `Logger.debug()` (iOS)
- Replace `println()` / `System.out` with `Log.d()` wrapped in `BuildConfig.DEBUG` (Android)

**Debug-only visibility (3b)**:
- Wrap unguarded `Log.d()` / `Log.i()` calls in `if (BuildConfig.DEBUG)` (Android)

**Logger declaration (3c)**:
- Move function-level Logger to class/file-level property

**Tag convention (3d)**:
- Replace inline string literal tags with `TAG` constant
- Add `companion object { private const val TAG = "ClassName" }` if missing

**Message prefix (3e)**:
- Add missing `[TAG]` prefix to messages

**Commented-out logs (3g)**:
- Delete commented-out log lines

**Privacy annotations (3k)**:
- Add `privacy: .public` to all String interpolations in `os.Logger` calls
- Do not add to scalar types (Int, Bool, Double) — they are public by default

**Duplicate logs (3i)**:
- Remove the duplicate with less context; keep the one at the more useful location
- If both are equally informative, remove the later occurrence

**Redundant logs (3j)**:
- Remove identity / no-op logs entirely
- Merge excessive-granularity groups into a single structured log statement
- For lifecycle noise flagged as LOW, only remove if user approves
- For hot-path logs, move outside the loop or guard with a sampling condition (e.g., log once per N iterations), per user preference

### 7. Summary

After applying fixes, output:

```markdown
## Fixes Applied

| File                     | Fixes | Lines Changed |
|--------------------------|-------|---------------|
| InputView.kt             | 2     | 4             |
| ComposingManager.swift   | 1     | 3             |
| **Total**                | 3     | 7             |
```

## Important

- **Never remove a log statement** unless it is commented out, a confirmed duplicate (3i), or confirmed redundant (3j) — only modify the guard, tag, level, or prefix for other checks
- **Never change log message content** — only adjust the wrapping and formatting
- **Preserve log level intent** — do not downgrade `Log.e()` to `Log.d()` without user confirmation (it may be a genuine error log)
- **`Log.w()` and `Log.e()` for genuine errors may remain unguarded** — only flag these if they appear to be diagnostic traces
- **iOS `Logger.debug()` is already release-safe** — do not add extra `#if DEBUG` guards around it
- **When merging redundant logs, preserve all unique information** — the merged log must contain every variable/value from the originals
- **Duplicate detection is per-module, not global** — the same log message in two unrelated modules is not a duplicate
- **Hot-path log removal requires user confirmation** — some may be intentionally placed for performance profiling
- Follow the project's existing code style when making changes
