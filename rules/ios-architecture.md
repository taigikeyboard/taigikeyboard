# iOS Architecture Rules

Mandatory architectural contract for the iOS target. Read before any non-trivial structural change. This document is the judge for the 11-phase refactor in `/Users/alexsu/.claude/plans/tender-zooming-steele.md`.

Last updated: 2026-04-18 (Phase 0, post Codex review).

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
| `Lexicon/`                 | Engine   | DB repos allowed (Foundation + SQLite3 C API)     |
| `NextWord/`                | Engine   | After Phase 5 restructure                         |
| `Autocomplete/Services/`   | Engine   | After Phase 4 KK removal                          |
| `Settings/` (non-UI parts) | Engine   | `EngineSettings`, `EngineSettingsProvider`, etc.  |
| `Settings/` (UI parts)     | Platform | `KeyboardColorSettings` (UIColor), `CodableColor` |
| `Actions/`                 | Platform | Also hosts KK adapter code                        |
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

### Current violations to fix during the refactor

| File                                            | Current state                                    | Phase |
|-------------------------------------------------|--------------------------------------------------|-------|
| `Lexicon/Processor/CandidateProcessor.swift`    | Reads `KeyboardSettings.store` directly          | 4     |
| `Input/CaseTransformer.swift`                   | Public API uses `Keyboard.KeyboardCase`          | 4     |
| `Autocomplete/Services/AutocompleteProviders.swift` | Protocol returns `Autocomplete.Suggestion`   | 4     |
| `Input/Composing/ComposingManager.swift`        | `@Published suggestions: [Autocomplete.Suggestion]` | 4  |
| `NextWord/NextWordController.swift`             | Accepts `Autocomplete.Suggestion`                | 4     |
| `Settings/SharedSettings.swift`                 | Calls `KeyboardSettings.store.resetToDefaults()` | 3     |

### KK boundary call-site inventory (to be filled in Phase 4 pre-step)

Before Phase 4 starts, run grep passes and record call-sites here. Format:

```
Keyboard.KeyboardCase consumers:
  - <file:line>  — ...
AutocompleteContextUpdater callers:
  - <file:line>  — ...
Autocomplete.Suggestion producers:
  - <file:line>  — ...
```

This inventory justifies where each adapter lives.

### Conflict resolution

If an Engine-layer file appears to need KeyboardKit, the file is in the **wrong layer**. Move it to Adapter or Platform — do not add `import KeyboardKit` to an Engine file as an exception.

---

## 4. Shared-Core Candidates

"Shared-core candidate" = a file eligible for future cross-platform extraction (iOS ↔ Android). Marking a file as a candidate is a **contract** about its dependencies, not a promise to extract it.

### Criteria — ALL must hold

1. Only `import Foundation` (no `UIKit`, `SwiftUI`, `KeyboardKit`, `Combine`).
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

### Candidate roster (target state after Phase 11)

File-by-file evaluation is required. Candidates known today:

- `Phonetics/Tables/*`, `Phonetics/Parser/*`, `Phonetics/Formatter/*`, `Phonetics/Converter/*`
- `Phonetics/ToneRestoration.swift`, `Phonetics/ToneUtilities.swift`
- `Input/TPS/TPSConverter.swift`, `TPSTables.swift`, `TPSInputAdjuster.swift`, `TPSToTL.swift`, `TLToTPS.swift`
- `Input/CharacterInputPipeline.swift`
- `Input/CaseTransformer.swift` (after Phase 4 KK removal)
- `Lexicon/Trie/InputNormalizer.swift`, `TrieService.swift`
- `Lexicon/Models/TaigiWord.swift`, `Lexicon/Models/InputType.swift`
- `Lexicon/Processor/CandidateProcessor.swift`, `CandidateCaseTransformer.swift`
- `Lexicon/Utils/TaigiUnicode.swift`
- `NextWord/NextWordScorer.swift`
- `Overlays/CandidateRowLayoutEngine.swift`

### Known exclusions (evaluated, do NOT mark)

- `Lexicon/Models/EnabledDictionaries.swift` — reads `SharedSettings.shared` (violates criterion 2).
- `Lexicon/Models/DictionarySearchResult.swift` — computed property builds app-specific lookup URL (violates criterion 4).
- Any `*Service.swift` — coordinates side effects / DB access.
- Any SQLite-using file — SQLite is a runtime dep per criterion 3 (repositories stay engine-layer but are not shared-core candidates).

### Verification (Phase 11)

```sh
# All candidates must import Foundation, and nothing else of consequence
grep -L "import Foundation" $(cat shared-core-list.txt)                                # -> empty
grep -l "import KeyboardKit\|import UIKit\|import SwiftUI\|import Combine" $(cat shared-core-list.txt)   # -> empty
grep -l "SharedSettings.shared\|KeyboardSettings.store\|\.shared" $(cat shared-core-list.txt)            # -> empty
grep -l "URL(string:\|FileManager\|DispatchQueue\|@Published\|ObservableObject\|NotificationCenter" $(cat shared-core-list.txt)   # -> empty
```

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

## 7. Per-Phase Judgment Criteria

This document is the reference the refactor plan defers to. When a phase completes, audit against:

- [ ] Any new or moved file in Engine/ passes shared-core criteria (if it claims the marker)
- [ ] No Engine-layer file imports `KeyboardKit` / `UIKit` / `SwiftUI` / `Combine`
- [ ] No Engine-layer file reads `SharedSettings.shared` / `KeyboardSettings.store`
- [ ] `git grep` call-site inventory in §3 matches the current KK-boundary file set
- [ ] Settings change sync regression test passes
- [ ] File names match primary types; no `_` prefix, no numeric folders

---

## 8. References

- Refactor plan: `/Users/alexsu/.claude/plans/tender-zooming-steele.md`
- Codex review (2026-04-18): recorded in the project memory at `/Users/alexsu/.claude/projects/-Users-alexsu-Workspace-taigikeyboard/memory/` — findings from the review are baked into §3, §4, §5 of this document
- Related rules:
  - `ios-guidelines.md` — SourceKit, KeyboardKit, memory mgmt, naming, tests
  - `ai-friendly-code.md` — naming, comments, function design
  - `code-review-rules.md` — review checklist
