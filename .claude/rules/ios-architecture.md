---
paths: ["ios/**/*.swift"]
---

# iOS Architecture Rules

Mandatory architectural contract for the iOS target. Read before any non-trivial structural change.

iOS is the architectural exemplar; Android matches the shape documented at `docs/architecture/ios-exemplar.md`. The per-slice Rust inventory lives in `docs/engine/migration-inventory.csv`.

**Split note**: Shared-core candidate criteria + marker live in `.claude/rules/ios-shared-core-candidates.md`. Settings-injection wiring lives in `.claude/rules/ios-settings-injection.md`.

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
│                        Overlays/, Styling/)                  │
│  KeyboardKit-aware extension runtime: view controller,       │
│  action handler, keyboard views, overlays, styling           │
│  Imports: KeyboardKit, SwiftUI, UIKit, Adapter, Engine       │
└──────────────────────────────────────────────────────────────┘
                             │
┌──────────────────────────────────────────────────────────────┐
│  Adapter Layer        (explicit *Adapter.swift files that    │
│                        translate between Engine value types  │
│                        and KeyboardKit / Platform types)     │
│  Examples (post-refactor):                                   │
│   - KeyboardKitSuggestionAdapter (EnginePrediction ↔         │
│     Autocomplete.Suggestion)                                 │
│   - LetterCaseAdapter (Keyboard.KeyboardCase ↔ LetterCase)   │
│  Imports: KeyboardKit, Engine                                │
└──────────────────────────────────────────────────────────────┘
                             │
┌──────────────────────────────────────────────────────────────┐
│  Engine Layer         (Engine/, Input/, Lexicon/,            │
│                        NextWord/, Composition/, Logging/,    │
│                        Autocomplete/Services,                │
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
| `Composition/`             | Engine   | `CompositionRoot` — production service graph shared by app + extension; Foundation-only |
| `Logging/`                 | Engine   | `LoggerBackend` (Shared-Core Candidate); `DebugLogger.swift` at the root is the extension-facing wrapper |
| `Input/`                   | Engine   | Incl. `Composing/` — must be KK-free (see §3)     |
| `Lexicon/`                 | Engine   | DB repos allowed (Foundation + SQLite3 C API). `LexiconService`, `NextWordService`, `DictionaryRepository` all inject `EngineSettingsProvider`. |
| `NextWord/`                | Engine   |                                                   |
| `Autocomplete/Services/`   | Mixed    | `TaigiAutocompleteService.swift` and `EnglishAutocompleteService.swift` inherit `KeyboardKit.AutocompleteService` — unavoidable KK adapter boundary. `AutocompleteProviders.swift` and `AutocompleteContextBooster` (moved to `NextWord/`) are engine-pure. Treat subclass files as Platform-in-Engine-folder. |
| `Settings/` (non-UI parts) | Engine   | `EngineSettings`, `EngineSettingsProvider`, etc.  |
| `Settings/` (UI parts)     | Platform | `KeyboardColorSettings` (UIColor), `CodableColor` |
| `Actions/`                 | Platform | Hosts KK adapters: `ActionHandler*`, `KeyboardCaseAdapter`, `KeyboardContext+Composing`, `KeyboardContext+Translate` |
| `KeyboardExtension/`       | Platform | Extension target host: `KeyboardViewController`, `Info.plist`, `FontRegistration`, `zh-Hant.lproj` |
| `Callouts/`                | Platform |                                                   |
| `Emojis/`, `Layout/`       | Platform |                                                   |
| `Overlays/`                | Platform | Except `CandidateRowLayoutEngine` (Engine)        |
| `Styling/`                 | Platform |                                                   |
| `App/`                     | App      | Host app, tabs, settings UI                       |
| `Strings/`                 | App      | `Strings/Generated/` (codegen from `i18n/*.json` via `make i18n`) + `DisplayLanguageStore` / `StringResolver` / `DisplayLanguage` runtime resolvers |

---

## 2. Dependency Rules

### Engine layer

- **MUST** `import Foundation` and nothing else from Apple frameworks.
- **MUST NOT** `import KeyboardKit`, `UIKit`, `SwiftUI`, `Combine`.
- **MUST NOT** reference global singletons outside the layer (no `SharedSettings.shared`, no `KeyboardSettings.store`, no `DictionaryRepository.shared` from pure-logic code).
- **MUST NOT** reach into `FileManager`, App Group container paths, or network directly. Data is injected by callers.
- **MAY** use SQLite3 C API through `SQLiteConnectionManager` — SQLite is a build dep, not a platform framework.

### Adapter layer

- Adapters live in **named files** whose sole responsibility is translation between Engine value types and Platform/KK types.
- File name pattern: `<Domain>Adapter.swift` (e.g., `KeyboardKitSuggestionAdapter.swift`) or `<FromType>To<ToType>.swift`.
- Adapters **MAY** `import KeyboardKit`. They **MUST NOT** contain business logic — pure translation only.

### Platform layer

- May `import KeyboardKit`, `UIKit`, `SwiftUI`.
- Receives Engine dependencies via constructor injection (never by global singleton, except at the single entry point `KeyboardViewController`).

### App layer

- Free to import anything the host app needs.
- **MUST NOT** reach into Engine-layer types by bypassing the service layer (e.g., Tab views must not call `DictionaryRepository.shared` directly — go through `LexiconService`).

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

- File name matches the primary type it defines (`LexiconService.swift` → `public final class LexiconService`).
- One public type per file when reasonable; nested helper types OK if they support the primary type.
- Avoid generic suffixes: `Manager`, `Helper`, `Util`, `Utils`, `Handler` are discouraged unless they map to a concrete, well-scoped responsibility (e.g., `SQLiteConnectionManager` is a literal connection owner — OK).

### Folders

- Group by feature / layer, not by type kind.
  - **Good**: `NextWord/Services/`, `NextWord/Repository/`, `Engine/Generated/`
  - **Bad**: `Models/` at the root mixing unrelated value types, `Utils/` as a dumping ground
- No ordering hacks (no leading `_`, no numeric prefixes like `Tab1-4`).
- Folder names describe what is inside, not when it was added or its position in the UI.

- App tab folders (`Home/`, `Theme/`, `Layout/`, `Dictionary/`, `Settings/`) align with `TabType` enum cases and UI-visible titles; tab structs follow the folder name (`HomeTab`, `ThemeTab`). Tab titles come from `i18n/nav.json`, not per-tab `*Texts` enums.
- Do **not** introduce a type named `Keyboard` or `KeyboardExtension` — KeyboardKit already owns the `Keyboard` namespace.

---

## 5. Per-Change Audit Checklist

Apply to any non-trivial structural change:

- [ ] Any new or moved file in Engine/ passes shared-core criteria (if it claims the marker) — see `.claude/rules/ios-shared-core-candidates.md`
- [ ] No Engine-layer file imports `KeyboardKit` / `UIKit` / `SwiftUI` / `Combine`
- [ ] No Engine-layer file reads `SharedSettings.shared` / `KeyboardSettings.store` — see `.claude/rules/ios-settings-injection.md`
- [ ] Settings change sync regression test passes (live-read propagation from app to keyboard) — see `.claude/rules/ios-settings-injection.md` § Change sync regression test
- [ ] File names match primary types; no `_` prefix, no numeric folders

---

## 6. References

- `.claude/rules/ios-guidelines.md` — day-to-day iOS rules (SourceKit, KeyboardKit, memory mgmt, naming, tests)
- `.claude/rules/ios-shared-core-candidates.md` — criteria + marker for files eligible for cross-platform extraction
- `.claude/rules/ios-settings-injection.md` — `EngineSettingsProvider` wiring for live-read settings
- `.claude/rules/android-guidelines.md` — Android counterpart with shared-core / Kotlin best-practice rules
- `.claude/rules/cross-platform-alignment.md` — refactor-freeze contract both platforms follow
- `~/.claude/rules/ai-friendly-code.md` — naming, comments, function design (cross-platform)
- `~/.claude/rules/code-review-rules.md` — review checklist
- `docs/architecture/ios-exemplar.md` — alignment target for Android
- `docs/engine/migration-inventory.csv` — authoritative Rust slice inventory + native pending / keep / wont-migrate roster
