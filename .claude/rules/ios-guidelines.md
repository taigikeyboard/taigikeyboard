---
paths: ["ios/**/*.swift"]
---

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

## Xcode / pbxproj — user-only, with synced-group exceptions

`*.xcodeproj`, `*.xcworkspace`, and `*.pbxproj` are **user-only**. AI never edits them. The hook at `.claude/hooks/block-project-config.sh` enforces this. Do not work around with code-level hacks; list any Xcode-side step as an action item for the user.

### Synchronized groups auto-include new files

`ios/TaigiKeyboard.xcodeproj/project.pbxproj` uses Xcode 16's `PBXFileSystemSynchronizedRootGroup` for nearly all `Sources/TaigiKeyboard/*` subdirectories (`App/`, `Actions/`, `Autocomplete/`, `Callouts/`, `Composition/`, `Emojis/`, `Engine/`, `Input/`, `KeyboardExtension/`, `Layout/`, `Lexicon/`, `Logging/`, `NextWord/`, `Overlays/`, `Settings/`, `Strings/`, `Styling/`). Files dropped under any of these paths are **auto-included** on next build. Do NOT add "user adds X to target" steps when X lives under a synced group. The list above can drift — when in doubt, audit pbxproj live (see "Folder renames" below).

### File DELETION within a synced group is auto-handled — no reminder

Refactors that delete Swift files under a synced-group directory: Xcode auto-removes them on next open/build. Do NOT list "user removes deleted files in Xcode" as a release blocker. (User directive 2026-05-15.)

### When manual action IS still required

- New file at `Sources/TaigiKeyboard/` ROOT level (siblings to synced groups need manual add).
- New top-level directory under `Sources/TaigiKeyboard/` — Xcode does NOT auto-promote a new dir to a synced group; user must "Add Files…" or "Convert to Synchronized Group".
- Binary references (e.g. `ios/RustEngine/RustTaigi.xcframework`) — not synced.
- `Info.plist`, entitlements, signing, build settings, scheme — always pbxproj-level, always user. **One exception, and it is still user-run**: `MARKETING_VERSION` and `CURRENT_PROJECT_VERSION` are written by `make version-mobile x.y.z`, which sets the same version on Android in the same pass (the mobile train; macOS + Windows are the separately numbered desktop train) and pins the build number to 1 (App Store Connect numbers a version's uploads itself) (`tools/release_notes.py set-versions --train mobile`). AI still never edits pbxproj — name that command as the user's action item instead of asking for a hand edit in Xcode, because a hand edit desyncs the two mobile platforms until `check-versions` catches it.
- Adding the same file to a SECOND target — synced group governs the primary target only.

### Folder renames break synced-group registration — audit drift

When a refactor renames an iOS source folder (`git mv ios/Sources/TaigiKeyboard/Common ios/Sources/TaigiKeyboard/Logging`) or moves files across folders, the synced-group registration in pbxproj does NOT auto-update. The build silently breaks for files that move into the new path.

Before approving any iOS folder-level refactor, audit:

```bash
# Orphan groups (registered path no longer exists on disk)
for path in $(grep -E 'PBXFileSystemSynchronizedRootGroup;' -A4 \
  ios/TaigiKeyboard.xcodeproj/project.pbxproj \
  | grep -oE 'Sources/TaigiKeyboard/[A-Za-z]+' | sort -u); do
  [ ! -d "ios/$path" ] && echo "ORPHAN: $path"
done
# Missing registrations (on-disk dir not in pbxproj)
for d in ios/Sources/TaigiKeyboard/*/; do
  name="${d#ios/Sources/TaigiKeyboard/}"; name="${name%/}"
  grep -qE "Sources/TaigiKeyboard/$name\b" \
    ios/TaigiKeyboard.xcodeproj/project.pbxproj \
    || echo "MISSING: $name"
done
```

If drift exists: alert the user. They drag the new folder into Project Navigator as *Create folder references* and delete the orphan group. AI never edits pbxproj directly. (Incident: PR #213 renamed `Common/ → Logging/` on disk without updating pbxproj; build silently broke at `Engine/RustEngineBridge.swift:281` "Cannot find 'LoggerFactory' in scope".)
