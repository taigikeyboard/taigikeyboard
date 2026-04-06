# Case Handling (Uppercase/Lowercase)

> **Type**: Feature
> **Keywords**: `Case`, `Shift`, `CapsLock`, `CaseTransformer`
> **Related**: ../engine/composing.md

---

## Summary

- Case state managed by KeyboardKit
- This project controls behavior through settings synchronization
- Supports proper tone letter conversion (`á` → `Á`)

---

## Core Mechanism

```
User action (Shift / sentence start)
    ↓
KeyboardKit manages keyboardCase internally
    ↓
CaseTransformer.transformForInput()
    ↓
Output character
```

---

## Auto-Capitalization Setting

### isAutoCapitalizationEnabled

| Setting | Keyboard display | Sentence start output | Manual Shift | Caps Lock |
|---------|-----------------|----------------------|--------------|-----------|
| `true` | Uppercase at sentence start | Uppercase | Uppercase | Uppercase |
| `false` | Always lowercase | Lowercase | Uppercase | Uppercase |

---

## KeyboardKit Settings Sync

```swift
func syncToKeyboardContext(_ context: KeyboardContext) {
    context.settings.isAutocapitalizationEnabled = isAutoCapitalizationEnabled

    if !isAutoCapitalizationEnabled {
        context.autocapitalizationTypeOverride = .none
        if context.keyboardCase == .uppercased {
            context.keyboardCase = .lowercased
        }
    }
}
```

---

## Core Components

| Component | File | Description |
|-----------|------|-------------|
| `CaseTransformer` | `CaseTransformationService.swift` | Unified conversion entry |
| `ToneUtilities` | `ToneUtilities.swift` | Tone letter conversion |
| `TaigiPhonetics` | `TaigiPhonetics.swift` | POJ/TL phonetics engine |
| `SharedSettings` | `SharedSettings.swift` | Settings and sync |

---

## KeyboardKit 10 Known Issues

### Issue 1: keyboardCase overwritten when switching keyboards

- **Symptom**: Switching back from symbol keyboard may result in uppercase
- **Workaround**: Monitor keyboardCase changes, restore to lowercase

### Issue 2: Candidate case doesn't follow

- **Symptom**: Mixed case input shows incorrect candidate display
- **Solution**: `SuggestionCaseTransformer` handles at View layer

---

## Platform Differences

| Item | iOS | Android |
|------|-----|---------|
| Framework | KeyboardKit | Self-managed |
| State tracking | KeyboardKit managed | `CapsStateManager.kt` (extracted from TextInputManager in v3.4.6) |
| Control method | Settings sync | `updateCapsState()` |
| Real-time update | NotificationCenter | DataStore Flow |
| Suggestion case | `SuggestionCaseTransformer.swift` | `SuggestionCaseTransformer.kt` |
