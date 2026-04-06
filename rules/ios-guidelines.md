# iOS Project Guidelines

Mandatory rules for iOS development. Read before modifying iOS code.

## SourceKit Diagnostics

- SourceKit cannot resolve cross-file types without a full Xcode build — errors like "Cannot find type 'X' in scope" are expected and should be ignored
- Only investigate diagnostics that reference types/functions within the same file

## KeyboardKit

- **Must consult KeyboardKit documentation before implementation**
- **KeyboardKit 10+ is closed-source** — cannot view source code directly
- Local docs: `./references/KeyboardKit-Documentation/`
- Online docs: https://keyboardkit.github.io/KeyboardKitDocs/

## Memory Management

1. **Separate SwiftUI View from Controller** — Views must not directly hold Controller references
2. **setupKeyboardView safe mode** — Ignore controller parameter, use `self.state` and `self.services`
3. **Service class Delegates** — Must use `weak` reference
4. **Any memory-related changes must explicitly document risks**

## Architecture Notes

- **SQLite layer**: `SQLiteConnectionManager` handles connection, queue, and initialization. Repositories use raw `sqlite3_*` C API inside `connectionManager.execute { db in }` closures — this verbosity is inherent to the C API, don't add wrapper abstractions
- **Shared constant**: `SQLiteConnectionManager.sqliteTransient` replaces inline `unsafeBitCast(-1, to: sqlite3_destructor_type.self)` — use it for all `sqlite3_bind_text` calls

## Swift Naming Conventions

- Boolean `@State` and properties: always use `is`/`has`/`can`/`should` prefix (`isPressed`, `isExpanded`, not `pressed`, `expanded`)
- Private nested types: prefix with parent context (`CandidateRowItem` not `RowItem`)
- Extract magic numbers into named local constants with units in the name where applicable

## Test Conventions

- Tests must be simple, effective, and non-redundant — no duplicate coverage across files
- All conversion-related tests (TPS, TL, POJ, tone marks) use `./taigi-converter` (git submodule) as canonical reference implementation
- When tests fail, verify against reference behavior before changing production code
- Assertion messages must be descriptive enough to copy-paste for debugging
- Framework: XCTest; pattern: parametric arrays `[(input, expected)]` with loops + `XCTAssertEqual`
- Naming: `test{Component}_{scenario}`
