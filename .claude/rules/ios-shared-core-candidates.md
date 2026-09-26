---
paths: ["ios/**/*.swift"]
---

# iOS Shared-Core Candidates

Marking criteria + roster for iOS files eligible for cross-platform extraction (iOS ↔ Android). Marking a file is a **contract** about its dependencies, not a promise to extract it. Split out from `.claude/rules/ios-architecture.md` for focus.

The criteria below apply to any new candidate marking and to the `native_keep` roster in `docs/engine/migration-inventory.csv`.

## 1. Criteria — ALL must hold

1. Only `import Foundation` (no `UIKit`, `SwiftUI`, `KeyboardKit`, `Combine`, `OSLog`).
2. No global singleton dependency (no `SharedSettings.shared`, no `KeyboardSettings.store`, no `*.shared` access).
3. No DB / App Group container / `FileManager` / file-system access — data is injected.
4. No app-specific URL generation (e.g., `iTaigi://...`, `moedict://...`) or external service integration.
5. No platform side effects — no `NotificationCenter` observers, no `Timer`, no `DispatchQueue.main`, no `OperationQueue`.
6. No `@Published`, no `ObservableObject`, no `@MainActor` on type declarations.

## 2. Marking

Every file that satisfies the criteria begins with:

```swift
// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
```

Files that are engine-layer but **do not** qualify should begin with a one-line `// NOTE: Not shared-core — <reason>` comment so the audit state stays visible at the top of the file.

## 3. Candidate roster

**Authoritative inventory: `docs/engine/migration-inventory.csv`**. Filter `status=rust_shipped` for already-migrated items, `native_keep` for platform-stays candidates, `wont_migrate` for explicit exclusions (UI / KeyboardKit wrappers / etc.). User-data SQLite is `rust_shipped` (`engine/userdata`, user-data engine P7b / P8b). Do not re-enumerate or count here — update the CSV and point back.

## 4. Exclusions, soft dependencies, verification

`migration-inventory.csv` rows with `status=wont_migrate` enumerate the exclusions (Services/* glue, KeyboardKit wrappers, URL builders, UI). Enforcement = the §1 criteria checked against the files in the live `native_keep` filter on the CSV.

Matches inside `///` doc comments of a candidate file are informational, not violations (e.g., a doc comment stating the file does *not* use `SharedSettings.shared`).

## 5. References

- `.claude/rules/ios-architecture.md` — parent file (layer dependencies, KeyboardKit isolation, naming conventions)
- `.claude/rules/ios-settings-injection.md` — companion: `EngineSettingsProvider` is the criterion-2 injection mechanism
- `.claude/rules/android-guidelines.md` §1 — Android-side shared-core candidate rules (criteria mirror)
- `.claude/rules/cross-platform-alignment.md` §1c — shared-core-candidate bug-fix constraint
- `docs/engine/migration-inventory.csv` — authoritative live roster
