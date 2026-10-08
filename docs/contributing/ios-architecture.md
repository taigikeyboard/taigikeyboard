# iOS Architecture Rules

Mandatory architectural contract for the iOS target. Read before any non-trivial structural change.

iOS is the architectural exemplar; Android matches the shape documented at `docs/architecture/ios-exemplar.md`. The per-slice Rust inventory lives in `docs/engine/migration-inventory.csv`.

**Split note**: settings-injection wiring lives in `docs/contributing/ios-settings-injection.md`.

---

## 1. Module Layers

iOS code is organized into four layers. Dependencies flow **top-down only** — a lower layer never imports a higher one.

```
┌──────────────────────────────────────────────────────────────┐
│  App Layer            (App/*)                                │
│  SwiftUI host app, Tabs, settings UI, dictionary browser     │
│  Imports: SwiftUI, UIKit, Combine, Platform, Adapter, Engine │
└──────────────────────────────────────────────────────────────┘
                             │
┌──────────────────────────────────────────────────────────────┐
│  Platform Layer       (KeyboardExtension/, Actions/,         │
│                        Callouts/, Emojis/, Layout/,          │
│                        Overlays/, Styling/, Theme/)          │
│  KeyboardKit-aware extension runtime: view controller,       │
│  action handler, keyboard views, overlays, styling           │
│  Imports: KeyboardKit, SwiftUI, UIKit, Adapter, Engine       │
└──────────────────────────────────────────────────────────────┘
                             │
┌──────────────────────────────────────────────────────────────┐
│  Adapter Layer        (explicit *Adapter.swift files that    │
│                        translate between Engine value types  │
│                        and KeyboardKit / Platform types)     │
│  Example:                                                    │
│   - KeyboardCaseAdapter (Actions/: Keyboard.KeyboardCase     │
│     ↔ RustEngineBridge.CaseTransformLetterCase)              │
│  Imports: KeyboardKit, Engine                                │
└──────────────────────────────────────────────────────────────┘
                             │
┌──────────────────────────────────────────────────────────────┐
│  Engine Layer         (Engine/, Input/, Lexicon/,            │
│                        NextWord/, ServiceGraph/, Logging/,   │
│                        Candidates/Services,                  │
│                        Settings/ pure parts)                 │
│  Rust FFI bridge (RustEngineBridge + generated protos),      │
│  input pipeline, next-word glue, DB repositories, engine     │
│  settings protocol, service graph wiring, logging backend    │
│  Imports: Foundation ONLY (+ SwiftProtobuf in Engine/)       │
└──────────────────────────────────────────────────────────────┘
```

### Folder → Layer map

| Folder                     | Layer    | Notes                                             |
|----------------------------|----------|---------------------------------------------------|
| `Engine/`                  | Engine   | `RustEngineBridge*.swift` FFI facade + `Generated/*.pb.swift` protos; Foundation + SwiftProtobuf only. Phonetics / tone logic lives in the Rust engine, not in Swift. |
| `ServiceGraph/`            | Engine   | `CompositionRoot` — production service graph shared by app + extension; Foundation-only |
| `Logging/`                 | Engine   | `LoggerBackend` (Shared-Core Candidate) + `DebugLogger`, the extension-facing wrapper |
| `Input/`                   | Engine   | Incl. `Composing/` — must be KK-free (see §3)     |
| `Lexicon/`                 | Engine   | Engine clients only, no SQLite: `LexiconClient`, `UserDataClient`, `DictionarySearchService` (injects `EngineSettingsProvider`). |
| `NextWord/`                | Engine   | `NextWordController` (injects `EngineSettingsProvider`) |
| `Candidates/Services/`     | Mixed    | `TaigiAutocompleteService.swift` and `EnglishAutocompleteService.swift` inherit `KeyboardKit.AutocompleteService` — unavoidable KK adapter boundary. `AutocompleteProviders.swift` is engine-pure. Treat subclass files as Platform-in-Engine-folder. |
| `Settings/` (non-UI parts) | Engine   | `EngineSettings`, `EngineSettingsProvider`, etc.  |
| `Settings/` (UI parts)     | Platform | `SharedSettings`, `KeyboardEnvironment`, `DeviceCapabilities` |
| `Theme/`                   | Platform | Keyboard themes: `KeyboardColorSettings` (UIColor), `CodableColor`, built-in + user themes, theme images |
| `Actions/`                 | Platform | Hosts KK adapters: `ActionHandler*`, `KeyboardCaseAdapter`, `KeyboardContext+Composing`, `KeyboardContext+Translate` |
| `KeyboardExtension/`       | Platform | Extension target host: `KeyboardViewController`, `Info.plist`, `FontRegistration`, `zh-Hant.lproj` |
| `Callouts/`                | Platform |                                                   |
| `Emojis/`, `Layout/`       | Platform |                                                   |
| `Overlays/`                | Platform |                                                   |
| `Styling/`                 | Platform |                                                   |
| `App/`                     | App      | Host app, tabs, settings UI                       |
| `Strings/`                 | App      | `Strings/Generated/` (codegen from `i18n/*.json` via `make i18n`) + `DisplayLanguageStore` / `StringResolver` / `DisplayLanguage` runtime resolvers |

---

## 2. Dependency Rules

### Engine layer

Engine-layer files import Foundation only (plus SwiftProtobuf in `Engine/`) so the layer stays KeyboardKit-free and unit-testable:

- No `KeyboardKit`, `UIKit`, `SwiftUI`, `Combine` imports.
- No global singletons from outside the layer (no `SharedSettings.shared`, `KeyboardSettings.store` in pure-logic code) — dependencies are injected so tests can stub them.
- No direct `FileManager`, App Group container paths, or network — callers inject the data.
- No SQLite: the user's data is the engine's (`engine/userdata`, `docs/architecture/user-data-engine-roadmap.md` P7b), reached through `RustEngineBridge+UserData.swift` / `UserDataClient`.

### Adapter layer

- Adapters live in **named files** whose sole responsibility is translation between Engine value types and Platform/KK types.
- File name pattern: `<Domain>Adapter.swift` (e.g., `Actions/KeyboardCaseAdapter.swift`) or `<FromType>To<ToType>.swift`.
- Adapters may `import KeyboardKit` and hold translation only — business logic in an adapter would couple it to KeyboardKit.

### Platform layer

- May `import KeyboardKit`, `UIKit`, `SwiftUI`.
- Receives Engine dependencies via constructor injection (never by global singleton, except at the single entry point `KeyboardViewController`).

### App layer

- Free to import anything the host app needs.
- Reaches Engine-layer types through the service layer only (e.g., Dictionary tab view models take an injected `DictionarySearchService` / `UserDataClient`, defaulting to `CompositionRoot`).

---

## 3. KeyboardKit Isolation

### Rule

KeyboardKit types (`Autocomplete.Suggestion`, `Keyboard.KeyboardCase`, `AutocompleteContext`, etc.) appear **only** in:

- Platform layer files (where KK is the runtime contract)
- Adapter layer files (purpose-built for translation)

They do **not** appear in Engine-layer files — no exceptions.

### Conflict resolution

If an Engine-layer file appears to need KeyboardKit, the file is in the **wrong layer**. Move it to Adapter or Platform — do not add `import KeyboardKit` to an Engine file as an exception.

---

## 4. Naming Conventions

### Files

- File name matches the primary type it defines (`DictionarySearchService.swift` → `final class DictionarySearchService`).
- One public type per file when reasonable; nested helper types OK if they support the primary type.
- Avoid generic suffixes: `Manager`, `Helper`, `Util`, `Utils`, `Handler` are discouraged unless they map to a concrete, well-scoped responsibility (e.g., a `…Manager` that is a literal owner of one resource — OK).

### Folders

- Group by feature / layer, not by type kind.
  - **Good**: `Lexicon/Services/`, `Candidates/Views/`, `Engine/Generated/`
  - **Bad**: `Models/` at the root mixing unrelated value types, `Utils/` as a dumping ground
- No ordering hacks (no leading `_`, no numeric prefixes like `Tab1-4`).
- Folder names describe what is inside, not when it was added or its position in the UI.

- App tab folders (`App/Tabs/Home/`, `App/Tabs/Theme/`, `App/Tabs/Layout/`, `App/Tabs/Dictionary/`, `App/Tabs/Settings/`) align with `TabType` enum cases and UI-visible titles; tab structs follow the folder name (`HomeTab`, `ThemeTab`). Tab titles come from `i18n/nav.json`, not per-tab `*Texts` enums.
- Do **not** introduce a type named `Keyboard` or `KeyboardExtension` — KeyboardKit already owns the `Keyboard` namespace.

---

## 5. Shared-Core Candidates

Marking a file is a **contract** about its dependencies, not a promise to extract it. The criteria apply to any new candidate marking and to the `native_keep` roster in `docs/engine/migration-inventory.csv`.

### Criteria — ALL must hold

1. Only `import Foundation` (no `UIKit`, `SwiftUI`, `KeyboardKit`, `Combine`, `OSLog`).
2. No global singleton dependency (no `SharedSettings.shared`, no `KeyboardSettings.store`, no `*.shared` access) — settings arrive through `EngineSettingsProvider` (`docs/contributing/ios-settings-injection.md`).
3. No DB / App Group container / `FileManager` / file-system access — data is injected.
4. No app-specific URL generation (e.g., `iTaigi://...`, `moedict://...`) or external service integration.
5. No platform side effects — no `NotificationCenter` observers, no `Timer`, no `DispatchQueue.main`, no `OperationQueue`.
6. No `@Published`, no `ObservableObject`, no `@MainActor` on type declarations.

Matches inside `///` doc comments of a candidate file are informational, not violations (e.g., a doc comment stating the file does *not* use `SharedSettings.shared`). Android mirrors these criteria in `docs/contributing/android-guidelines.md` §1.

### Marking

Every file that satisfies the criteria begins with:

```swift
// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
```

### Roster

Authoritative inventory: `docs/engine/migration-inventory.csv` — `rust_shipped` for migrated items, `native_keep` for platform-stays candidates, `wont_migrate` for explicit exclusions (Services glue, KeyboardKit wrappers, URL builders, UI). Do not re-enumerate or count here — update the CSV.

---

## 6. Per-Change Audit Checklist

Apply to any non-trivial structural change:

- [ ] Any new or moved file in Engine/ passes the shared-core criteria (if it claims the marker) — §5
- [ ] No Engine-layer file imports `KeyboardKit` / `UIKit` / `SwiftUI` / `Combine`
- [ ] No Engine-layer file reads `SharedSettings.shared` / `KeyboardSettings.store` — see `docs/contributing/ios-settings-injection.md`
- [ ] Settings change sync regression test passes (live-read propagation from app to keyboard) — see `docs/contributing/ios-settings-injection.md` § Change sync regression test
- [ ] File names match primary types; no `_` prefix, no numeric folders

---

## 7. References

- `docs/contributing/ios-guidelines.md` — day-to-day iOS rules (SourceKit, KeyboardKit, memory mgmt, naming, tests)
- `docs/contributing/ios-settings-injection.md` — `EngineSettingsProvider` wiring for live-read settings
- `docs/contributing/android-guidelines.md` — Android counterpart with shared-core / Kotlin best-practice rules
- `docs/contributing/cross-platform-alignment.md` — refactor-freeze contract both platforms follow
- `docs/architecture/ios-exemplar.md` — alignment target for Android
- `docs/engine/migration-inventory.csv` — authoritative Rust slice inventory + `native_keep` / `wont_migrate` roster
