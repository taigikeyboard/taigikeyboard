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

All case-mapping math lives in Rust `engine/phonetics::case_transform` (since case-transform slice / PR #205). Platform side calls via `RustEngineBridge`.

| Component | Location | Description |
|-----------|----------|-------------|
| Case-letter math (POJ/TL aware upper/lower, full-upper, candidate capitalization, suggestion transform, nasal-marker case adjust) | Rust `engine/phonetics/src/case_transform.rs` | Cross-platform canonical |
| iOS bridge | `Engine/RustEngineBridge+CaseTransform.swift` | Wraps `transformInputCase` / `capitalizeCandidate` / `transformSuggestion` / `uppercaseToneChar` / `fullUppercaseToneString` / `lowercaseToneChar` |
| Android bridge | `engine/CaseTransformBridge.kt` | Same surface, JVM signatures |
| iOS shift / capslock state | KeyboardKit (managed) | Drives `LetterCase` value passed into bridge |
| Android shift / capslock state | `ime/text/CapsStateManager.kt` | Same role, calls bridge per keystroke |
| Settings sync | iOS `SharedSettings.swift` / Android `PrefHelper.kt` | `isAutoCapitalizationEnabled` → `auto_cap_enabled` parameter on bridge |

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
| Suggestion case | `RustEngineBridge.transformSuggestion(...)` (called from `Autocomplete/Services/SuggestionCaseTransformer.swift` thin wrapper) | `RustEngineBridge.transformSuggestion(...)` (called from `ime/dictionary/SuggestionCaseTransformer.kt` thin wrapper that retains platform skip-rule guards) |
