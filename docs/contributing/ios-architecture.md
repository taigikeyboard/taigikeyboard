# iOS Architecture

Layering, settings injection and shared-core markers for `ios/Sources/TaigiKeyboard/`. Day-to-day iOS facts: `ios-guidelines.md`. Cross-platform contract: `cross-platform-alignment.md`.

## 1. Layers

Dependencies flow down only: **App → Platform → Adapter → Engine**. A lower layer never imports a higher one.

| Folder | Layer | Notes |
|---|---|---|
| `App/` (`Tabs/`, `Components/`) | App | host app; view models take services by injection, defaulting to `CompositionRoot` |
| `Strings/` | App | `Generated/` from `make i18n` + `DisplayLanguage` / `DisplayLanguageStore` / `StringResolver` |
| `KeyboardExtension/` | Platform | extension entry (`KeyboardViewController`), `Info.plist`, `FontRegistration`, `ResourceBundleResolver` (`.appex` bundle lookup) |
| `Actions/`, `Callouts/`, `Emojis/`, `Layout/`, `Overlays/`, `Styling/` (+ `Providers/`), `Theme/`, `Candidates/Views/`, `Candidates/Models/` | Platform | KeyboardKit / SwiftUI / UIKit allowed |
| `Settings/` UI parts — `SharedSettings`, `KeyboardEnvironment`, `DeviceCapabilities`, `SettingsResetCoordinator`, `SettingsSnapshot` | Platform | |
| `*Adapter.swift` (e.g. `Actions/KeyboardCaseAdapter.swift`) | Adapter | translation between engine values and KeyboardKit types only |
| `Engine/` (+ `Generated/`), `Input/` (+ `Composing/`), `Lexicon/`, `NextWord/`, `ServiceGraph/`, `Logging/`, `Candidates/Services/AutocompleteProviders.swift`, `Settings/` pure parts (`EngineSettings`, `EngineSettingsProvider`, `InputMode`, `PojMarkerOptions`) | Engine | see §2 |

## 2. Engine-layer imports

Engine-layer files import Foundation, plus SwiftProtobuf in `Engine/` and Observation for the `@Observable` `ComposingManager`. No KeyboardKit, UIKit, SwiftUI or Combine. Named exceptions, each the one place for its concern:

- `Candidates/Services/EngineFreeAutocompleteService.swift` and the `{Taigi,English}AutocompleteService.swift` that adopt it conform to `KeyboardKit.AutocompleteService` — the KeyboardKit boundary sits there.
- `Logging/DebugLogger.swift` — the OSLog sink (`#if DEBUG`).
- `Engine/UserDataOpening.swift` — the one App Group / filesystem site; it opens the engine's user-data stores (`engine/userdata`). No Swift code opens those files with SQLite.

A new Engine-layer file that seems to need KeyboardKit belongs in Platform or Adapter instead.

## 3. Settings injection

Engine-layer code reads settings through `EngineSettingsProvider` (`Settings/EngineSettingsProvider.swift`, `var current: EngineSettings`), never through `SharedSettings.shared` in its body. Services take the provider as an init parameter whose default is `SharedSettings.shared`; tests pass `StubEngineSettings`.

Read `provider.current` at each use, never store a snapshot: `SharedSettings` re-reads `UserDefaults` on each access, which is how a change made in the host app reaches a running keyboard. `KeyboardViewController` resyncs on `UserDefaults.didChangeNotification`; `TaigiKeyboardView` takes a per-render `SettingsSnapshot`, which is safe because it is rebuilt every render.

After touching this wiring, check on a device that changing input mode, output-both-scripts or a dictionary toggle in the app takes effect in an already-open keyboard.

## 4. Naming

- File name = primary type. New types avoid `Manager` / `Helper` / `Utils` suffixes and folders.
- No type named `Keyboard` or `KeyboardExtension` — KeyboardKit owns that namespace.

## 5. Shared-core candidates

Criteria: `cross-platform-alignment.md` §1c. A qualifying file carries, before its first declaration:

```swift
// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.
```

Roster: `docs/engine/migration-inventory.csv` (`native_keep`); update the CSV, not this file.
