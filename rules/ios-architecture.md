# iOS Architecture Rules

Mandatory architectural contract for the iOS target. Read before any non-trivial structural change.

**Status**: iOS structure refactor Phase 0–11 closed 2026-04-19 (PRs #131–#133). iOS Phase I exemplar plan closed same day (PR #141). This doc is now a stable reference — Phase I tactical TODO blocks removed 2026-04-19.

**Phase context**: shared-core extraction roadmap is tracked in Claude auto-memory (`project_shared_core_roadmap.md`, not in-repo); the per-slice Rust inventory lives in `docs/engine/migration-inventory.csv`. iOS is the architectural exemplar; Android matches the shape documented at `docs/architecture/ios-exemplar.md`. Phase I and Phase II audit docs (ios-exemplar-plan, android-state-audit) have been retired post-completion.

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
│  Engine Layer         (Phonetics/, Input/, Lexicon/,         │
│                        NextWord/, Autocomplete/Services,     │
│                        Settings/ pure parts)                 │
│  Pure-logic: phonetics, trie, input pipeline, next-word      │
│  scoring, DB repositories, engine settings protocol          │
│  Imports: Foundation ONLY                                    │
└──────────────────────────────────────────────────────────────┘
```

### Folder → Layer map (post-Phase 1)

| Folder                     | Layer    | Notes                                             |
|----------------------------|----------|---------------------------------------------------|
| `Phonetics/`               | Engine   | All files Foundation-only                         |
| `Input/`                   | Engine   | Incl. `Composing/` — must be KK-free (see §3)     |
| `Lexicon/`                 | Engine   | DB repos allowed (Foundation + SQLite3 C API). `LexiconService`, `NextWordService`, `DictionaryRepository` all inject `EngineSettingsProvider`. |
| `NextWord/`                | Engine   | After Phase 5 restructure                         |
| `Autocomplete/Services/`   | Mixed    | `AutocompleteService.swift` and `EnglishAutocompleteService.swift` inherit `KeyboardKit.AutocompleteService` — unavoidable KK adapter boundary. `AutocompleteProviders.swift` and `AutocompleteContextBooster` (moved to `NextWord/`) are engine-pure. Treat subclass files as Platform-in-Engine-folder. |
| `Settings/` (non-UI parts) | Engine   | `EngineSettings`, `EngineSettingsProvider`, etc.  |
| `Settings/` (UI parts)     | Platform | `KeyboardColorSettings` (UIColor), `CodableColor` |
| `Actions/`                 | Platform | Hosts KK adapters: `ActionHandler*`, `KeyboardCaseAdapter`, `KeyboardContext+Composing`, `KeyboardContext+Translate` |
| `KeyboardExtension/`       | Platform | (renamed from `_Keyboard/` in Phase 1; extension target host: `KeyboardViewController`, `Info.plist`, `FontRegistration`, `zh-Hant.lproj`) |
| `Callouts/`                | Platform |                                                   |
| `Emojis/`, `Layout/`       | Platform |                                                   |
| `Overlays/`                | Platform | Except `CandidateRowLayoutEngine` (Engine)        |
| `Styling/`                 | Platform |                                                   |
| `App/`                     | App      | Host app, tabs, settings UI                       |
| `Strings/`                 | App      | (renamed from `Localization/` in Phase 1)        |

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

## 4. Shared-Core Candidates

"Shared-core candidate" = a file eligible for future cross-platform extraction (iOS ↔ Android). Marking a file is a **contract** about its dependencies, not a promise to extract it.

### Criteria — ALL must hold

1. Only `import Foundation` (no `UIKit`, `SwiftUI`, `KeyboardKit`, `Combine`, `OSLog`).
2. No global singleton dependency (no `SharedSettings.shared`, no `KeyboardSettings.store`, no `*.shared` access).
3. No DB / App Group container / `FileManager` / file-system access — data is injected.
4. No app-specific URL generation (e.g., `iTaigi://...`, `moedict://...`) or external service integration.
5. No platform side effects — no `NotificationCenter` observers, no `Timer`, no `DispatchQueue.main`, no `OperationQueue`.
6. No `@Published`, no `ObservableObject`, no `@MainActor` on type declarations.

### Marking

Every file that satisfies the criteria begins with:

```swift
// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
```

Files that are engine-layer but **do not** qualify should begin with a one-line `// NOTE: Not shared-core — <reason>` comment so the audit state stays visible at the top of the file.

### Candidate roster

**Authoritative inventory: `docs/engine/migration-inventory.csv`** (148 rows, 13-col schema). Filter `status=rust_shipped` for already-migrated items; `native_pending` / `native_keep` for residual platform candidates; `wont_migrate` for explicit exclusions (UI / SQLite user-data / KeyboardKit wrappers / etc.). Do not re-enumerate here — update the CSV and point back.

### Exclusions, soft dependencies, verification

`migration-inventory.csv` rows with `status=wont_migrate` enumerate the exclusions (Lexicon Database/* SQLite, Services/* glue, KeyboardKit wrappers, URL builders, App-Group / FileManager paths). Per-criterion enforcement is documented in §4 above; running the original verification greps against `Foundation`-only candidate files still applies but the candidate set is the live `native_pending` / `native_keep` filter on the CSV.

Matches inside `///` doc comments of a candidate file are informational, not violations (e.g., `CandidateProcessor` documents that it does *not* use `SharedSettings.shared`; `EnginePrediction` documents that it does *not* `import KeyboardKit`).

---

## 5. Settings Injection

### Rule

Engine-layer code **MUST NOT** read `SharedSettings.shared` directly. It reads from an injected `EngineSettingsProvider` instead.

### Definition

```swift
// Settings/EngineSettings.swift (Foundation-only)
public protocol EngineSettings {
    var inputMode: InputMode { get }
    var isAutoSpaceEnabled: Bool { get }
    var isOutputBothScripts: Bool { get }
    var isFrequencyRecordingEnabled: Bool { get }
    var isAutoCap: Bool { get }
    // ... add as needed, never everything
}

// Settings/EngineSettingsProvider.swift (Foundation-only)
public protocol EngineSettingsProvider: AnyObject {
    var current: EngineSettings { get }
    func addChangeListener(_ listener: @escaping () -> Void) -> AnyObject  // token for removal
}
```

`SharedSettings.shared` conforms to `EngineSettingsProvider`. Its `current` returns a `SettingsSnapshot` (value type) captured at the moment of access. `addChangeListener` hooks into the existing `UserDefaults.didChangeNotification` plumbing.

### Why provider, not snapshot

`KeyboardViewController`, `AutocompleteService`, `ComposingManager`, and several views rely on `UserDefaults.didChangeNotification` + lazy re-read of `SharedSettings.shared` to implement **live updates** — when the user changes a setting in the host app, the keyboard extension reflects it without being relaunched.

A one-shot snapshot injection (inject once at init, store the value) **breaks this invariant**: input mode switches, TPS toggles, enabled-dictionary toggles would stop propagating. `TaigiKeyboardView` today uses a per-render snapshot (it re-reads on every render cycle), which is a different pattern and remains safe.

### Usage

| Context                                   | Access pattern                                     |
|-------------------------------------------|----------------------------------------------------|
| Engine service (`LexiconService`, etc.)   | `provider.current` at each call site               |
| Engine value-type operation               | Receive `EngineSettings` as a parameter            |
| UI view                                   | `@ObservedObject var settings = SharedSettings.shared` (app / platform layer only) |
| Keyboard extension entry (`KeyboardViewController`) | Constructs the provider, injects into engine     |
| Unit test                                 | Inject a stub `EngineSettingsProvider`             |

### Change sync regression test (mandatory)

Every phase that touches settings wiring must verify:

1. Launch app, open keyboard extension, confirm current setting value is used.
2. Without relaunching the keyboard, change the setting in the host app.
3. Interact with the keyboard again — the new setting must be applied.

Settings requiring this check: `inputMode`, `isAutoSpaceEnabled`, `isOutputBothScripts`, `isFrequencyRecordingEnabled`, `isAutoCap`, enabled-dictionaries set, TPS layout toggle.

---

## 6. Naming Conventions

### Files

- File name matches the primary type it defines (`LexiconService.swift` → `public final class LexiconService`).
- One public type per file when reasonable; nested helper types OK if they support the primary type.
- Avoid generic suffixes: `Manager`, `Helper`, `Util`, `Utils`, `Handler` are discouraged unless they map to a concrete, well-scoped responsibility (e.g., `SQLiteConnectionManager` is a literal connection owner — OK).

### Folders

- Group by feature / layer, not by type kind.
  - **Good**: `NextWord/Services/`, `NextWord/Repository/`, `Phonetics/Parser/`
  - **Bad**: `Models/` at the root mixing unrelated value types, `Utils/` as a dumping ground
- No ordering hacks (no leading `_`, no numeric prefixes like `Tab1-4`).
- Folder names describe what is inside, not when it was added or its position in the UI.

### Renames applied in Phase 1

Folder names align with `TabType` enum cases and UI-visible titles, not sub-file names.

| Before          | After                | Why                                                                 |
|-----------------|----------------------|---------------------------------------------------------------------|
| `_Keyboard/`    | `KeyboardExtension/` | `_` was a sort hack; folder hosts extension target bootstrap (`KeyboardViewController`, `Info.plist`, `FontRegistration`, `zh-Hant.lproj`). Not `Keyboard/` because the whole project is TaigiKeyboard (too vague) and that name collides with KeyboardKit's `Keyboard` namespace. Symmetric with `App/` (host app target). |
| `Tab1/`         | `Home/`              | Matches `TabType.home` + title「頭頁」                              |
| `Tab2/`         | `Layout/`            | Matches `TabType.layout` + title「齒盤佈局」(layout + font + color) |
| `Tab3/`         | `Dictionary/`        | Matches `TabType.dictionary` + title「詞庫管理」                    |
| `Tab4/`         | `Settings/`          | Matches `TabType.settings` + title「齒盤設定」                      |
| `Localization/` | `Strings/`           | No `.strings` catalog, just inline text                             |

**Important**:
- Do **not** introduce a type named `Keyboard` or `KeyboardExtension` — KeyboardKit already owns `Keyboard` namespace.
- Tab struct / Texts enum renames follow folder names: `Tab1 → HomeTab` (`Tab1Texts → HomeTexts`), `Tab2 → LayoutTab` (`LayoutTexts`), `Tab3 → DictionaryTab` (`DictionaryTexts`), `Tab4 → SettingsTab` (`SettingsTexts`).
- `TabType` enum cases (`.home`, `.layout`, `.dictionary`, `.settings`) are already semantic — no rename needed.

---

## 7. Per-Change Audit Checklist

Apply to any non-trivial structural change:

- [ ] Any new or moved file in Engine/ passes shared-core criteria (if it claims the marker)
- [ ] No Engine-layer file imports `KeyboardKit` / `UIKit` / `SwiftUI` / `Combine`
- [ ] No Engine-layer file reads `SharedSettings.shared` / `KeyboardSettings.store`
- [ ] Settings change sync regression test passes (live-read propagation from app to keyboard)
- [ ] File names match primary types; no `_` prefix, no numeric folders

---

## 8. References

- `rules/ios-guidelines.md` — day-to-day iOS rules (SourceKit, KeyboardKit, memory mgmt, naming, tests)
- `rules/android-guidelines.md` — Android counterpart with shared-core / Kotlin best-practice rules
- `rules/cross-platform-alignment.md` — refactor-freeze contract both platforms follow
- `rules/ai-friendly-code.md` — naming, comments, function design (cross-platform)
- `rules/code-review-rules.md` — review checklist
- `docs/architecture/ios-exemplar.md` — Phase II alignment target for Android
- `docs/engine/migration-inventory.csv` — authoritative Rust slice inventory + native pending / keep / wont-migrate roster
