# iOS Guidelines

Day-to-day facts for `ios/`. Layering and settings injection: `ios-architecture.md`.

## KeyboardKit

KeyboardKit 10+ is closed source: look its API up per `docs/contributing/doc-lookup.md` (Context7 first, then `references/KeyboardKit-Documentation/`), never from memory.

## Keyboard-extension memory

The extension runs under a ~64 MB cap and is re-shown many times per process, so a leak accumulates.

- Views never hold `KeyboardViewController`; `setupKeyboardView { [unowned self] _ in … }` ignores the controller parameter and uses `self.state` / `self.services`.
- Service delegates are `weak`.
- `viewWillDisappear` unparents the KeyboardKit hosting child (`KeyboardViewController+Cleanup.swift`); a child left parented keeps the dismissed controller alive (#480).
- A PR that changes ownership or teardown says what it risks.

## User data and Full Access

The four user-data stores live in `engine/userdata`; the app and the keyboard each open them once through `Engine/UserDataOpening.swift` in the App Group directory. The keyboard opens them only with Full Access (`RequestsOpenAccess`), because without it the App Group container is read-only to the extension; the app never opens them under XCTest. Pages go through `UserDataClient`; the engine counts every pick itself (`CommitContinuous` / `CommitPrediction`).

## Tests

XCTest; name tests `test{Component}_{scenario}`; stubs are `StubEngineSettings` / `UserDataClientStub`. Conversion goldens (TL / POJ / TPS) live in the engine, not in iOS tests.

## Xcode project files — maintainer only

`*.xcodeproj`, `*.xcworkspace` and `*.pbxproj` are never edited by agents (`.claude/hooks/block-project-config.sh` enforces it). List any Xcode step as a maintainer action item.

### Synchronized groups

`project.pbxproj` registers these `Sources/TaigiKeyboard/*` folders as `PBXFileSystemSynchronizedRootGroup`: `Actions`, `App`, `Callouts`, `Candidates`, `Emojis`, `Engine`, `Input`, `KeyboardExtension`, `Layout`, `Lexicon`, `Logging`, `NextWord`, `Overlays`, `ServiceGraph`, `Settings`, `Strings`, `Styling`, `Theme` (and `TaigiKeyboardTests`). A Swift file added to or deleted from any of them needs no Xcode step.

The maintainer still acts for:

- a file at the `Sources/TaigiKeyboard/` root, or a new top-level folder (Xcode does not promote it to a synced group);
- `KeyboardExtension/` per-target exception sets (`Info.plist`, `InfoPlist.strings`);
- binary references (`ios/RustEngine/RustTaigi.xcframework`), `Info.plist`, entitlements, signing, build settings, schemes;
- adding a file to a second target;
- versions: `make version-mobile x.y.z` sets `MARKETING_VERSION` / `CURRENT_PROJECT_VERSION` on iOS and Android together — name the command as the action item, never ask for a hand edit.

### Folder renames break registration

A renamed or moved top-level folder is not re-registered. Before approving one, audit:

```bash
# Orphan groups (registered path no longer exists on disk)
for path in $(grep -E 'PBXFileSystemSynchronizedRootGroup;' -A8 \
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

Drift → the maintainer drags the folder into the Project Navigator and deletes the orphan group. Symptom when missed: "Cannot find '<Type>' in scope" for types under the renamed folder.
