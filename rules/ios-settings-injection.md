# iOS Settings Injection

`EngineSettingsProvider` wiring contract for live-read settings on the iOS target. Split out from `rules/ios-architecture.md` for focus.

## 1. Rule

Engine-layer code **MUST NOT** read `SharedSettings.shared` directly. It reads from an injected `EngineSettingsProvider` instead.

## 2. Definition

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

## 3. Why provider, not snapshot

`KeyboardViewController`, `AutocompleteService`, `ComposingManager`, and several views rely on `UserDefaults.didChangeNotification` + lazy re-read of `SharedSettings.shared` to implement **live updates** — when the user changes a setting in the host app, the keyboard extension reflects it without being relaunched.

A one-shot snapshot injection (inject once at init, store the value) **breaks this invariant**: input mode switches, TPS toggles, enabled-dictionary toggles would stop propagating. `TaigiKeyboardView` today uses a per-render snapshot (it re-reads on every render cycle), which is a different pattern and remains safe.

## 4. Usage

| Context                                   | Access pattern                                     |
|-------------------------------------------|----------------------------------------------------|
| Engine service (`LexiconService`, etc.)   | `provider.current` at each call site               |
| Engine value-type operation               | Receive `EngineSettings` as a parameter            |
| UI view                                   | `@ObservedObject var settings = SharedSettings.shared` (app / platform layer only) |
| Keyboard extension entry (`KeyboardViewController`) | Constructs the provider, injects into engine     |
| Unit test                                 | Inject a stub `EngineSettingsProvider`             |

## 5. Change sync regression test (mandatory)

Every phase that touches settings wiring must verify:

1. Launch app, open keyboard extension, confirm current setting value is used.
2. Without relaunching the keyboard, change the setting in the host app.
3. Interact with the keyboard again — the new setting must be applied.

Settings requiring this check: `inputMode`, `isAutoSpaceEnabled`, `isOutputBothScripts`, `isFrequencyRecordingEnabled`, `isAutoCap`, enabled-dictionaries set, TPS layout toggle.

## 6. References

- `rules/ios-architecture.md` — parent file (layer dependencies, audit checklist)
- `rules/ios-shared-core-candidates.md` §1 (Criteria #2) — engine-layer files must not read `*.shared` singletons; this file's `EngineSettingsProvider` is the injection mechanism
- `rules/android-guidelines.md` §6 — Android DataStore + settings access (live-read counterpart)
- `docs/architecture/ios-exemplar.md` §3 — live-read warning + executor contract
