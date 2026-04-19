# AI-Friendly Code Rules (Cross-Platform)

Mandatory rules for writing code that is easy for AI tools to understand while minimizing token consumption.
When adding or modifying code, follow these rules to keep the codebase navigable for both humans and AI.

## File Headers

Every source file must have a **one-line** purpose comment at the top (below imports).

```kotlin
// Manages user frequency data for autocomplete ranking
class UserFrequencyRepository { ... }
```

```swift
// Manages user frequency data for autocomplete ranking
class UserFrequencyRepository { ... }
```

**Do NOT** write multi-line file headers, copyright blocks, or change logs in code — these waste tokens and duplicate git history.

## Naming

### Self-Documenting Names

Names must convey purpose without needing a comment. A good name eliminates the need for explanation.

| Bad | Good | Why |
|-----|------|-----|
| `proc()` | `processCandidate()` | Action is clear |
| `tmp` | `filteredSyllables` | Content is clear |
| `flag` | `isToneMarked` | Boolean intent is clear |
| `data` | `dictionaryEntries` | Type of data is clear |
| `handle()` | `handleToneInput()` | Scope is clear |
| `mgr` | `connectionManager` | No abbreviation guessing |

### Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Boolean vars/props | `is`/`has`/`can`/`should` prefix | `isExpanded`, `hasFrequencyData` |
| Functions | verb + noun, describe what it does | `fetchCandidates()`, `buildSyllableGraph()` |
| Constants | describe the value's role, not its content | `maxCandidateCount` not `TWENTY` |
| Files | match primary class/struct name | `CandidateProcessor.swift` for `CandidateProcessor` |
| Test files | `{Subject}Tests` | `CandidateProcessorTests.swift` |

### Abbreviations

- **Allowed**: Well-known domain terms (`TL`, `POJ`, `TPS`, `DB`, `SQL`, `UI`, `URL`)
- **Forbidden**: Ad-hoc abbreviations (`freq` for frequency, `cand` for candidate, `seg` for segment)
- When in doubt, spell it out

## Comments

### What to Comment

Comment only **non-obvious decisions** — the "why", not the "what".

```swift
// Recency window is strict `<`: a selection exactly one hour old must
// drop out of the boost. Using `<=` would pin a sticky candidate at
// the boundary and regress the 1-hour decay invariant.
guard (currentTime - lastUsedMillis) < oneHourMillis else { return baseScore }
```

### What NOT to Comment

| Avoid | Example of waste |
|-------|-----------------|
| Restating the code | `// increment counter` above `counter += 1` |
| Section banners | `// ===== SECTION: Database =====` |
| Obvious types | `// This is a string` |
| TODO without issue | `// TODO: fix this later` (use `// TODO(#123): ...`) |
| Commented-out code | Delete it; git has history |

### Comment Format

- Single-line `//` preferred over block `/* */`
- Keep comments under 80 characters per line
- Place comments on the line **above** the code, not inline (except brief field annotations)

## Function Design

### AI-Readable Function Signatures

Functions should be understandable from their signature alone:

```swift
// Good: signature tells the full story
func capitalizeCandidate(_ text: String, basedOn input: String, isAutoCapitalizationEnabled: Bool, inputMode: InputMode) -> String

// Bad: requires reading the body to understand
func process(_ s: String, _ m: Int) -> [Any]
```

### Function Size

- Target: **under 40 lines** per function
- If a function exceeds 40 lines, consider extracting a helper with a descriptive name
- Exception: simple switch/when statements with many cases

## Magic Values

Extract all magic numbers and strings into named constants:

```kotlin
// Bad
if (candidates.size > 20) { ... }

// Good
private const val MAX_VISIBLE_CANDIDATES = 20
if (candidates.size > MAX_VISIBLE_CANDIDATES) { ... }
```

## Rules Summary

1. **One-line file header** — purpose, not boilerplate
2. **Self-documenting names** — if you need a comment to explain a name, rename it
3. **Comment the "why"** — never the "what"
4. **No dead code or commented-out code** — git has history
5. **No ad-hoc abbreviations** — spell it out
6. **Descriptive function signatures** — readable without body
7. **Named constants** — no magic numbers or strings
8. **Under 40 lines per function** — extract with descriptive names
