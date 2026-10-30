# Case Handling (Uppercase/Lowercase)

> **Type**: Feature
> **Keywords**: `Case`, `Shift`, `CapsLock`, `transformInputCase`
> **Related**: ../engine/composing.md

---

## Summary

- Shift / caps-lock state: KeyboardKit on iOS, `CapsStateManager` on Android
- All case-letter math lives in the engine, so tone letters convert correctly (`á` → `Á`)

---

## Core Mechanism

```
User action (Shift / sentence start)
    ↓
Shift state (iOS: KeyboardKit keyboardCase; Android: CapsStateManager)
    ↓
transformInputCase (platform bridge → engine case_transform)
    ↓
Output character
```

iOS calls `RustEngineBridge.transformInputCase` from `ActionHandler+KeyActions.swift`; Android calls the `RustEngineBridge.transformInputCase` extension in `engine/CaseTransformBridge.kt`.

---

## Auto-Capitalization Setting

| Setting | Keyboard display | Sentence start output | Manual Shift | Caps Lock |
|---------|-----------------|----------------------|--------------|-----------|
| on | Uppercase at sentence start | Uppercase | Uppercase | Uppercase |
| off | Always lowercase | Lowercase | Uppercase | Uppercase |

iOS stores the setting in KeyboardKit's own `KeyboardSettings` (`isAutocapitalizationEnabled`). When it is off, `KeyboardViewController.setPreferredKeyboardCase()` resolves the preferred case to `.lowercased`, and `KeyboardViewController+Setup.swift` forces `keyboardCase` back to `.lowercased` on a settings sync; caps lock is kept in both. Android applies it in `CapsStateManager.updateCapsState()`.

---

## Core Components

| Component | Location | Description |
|-----------|----------|-------------|
| Case-letter math (POJ/TL aware upper/lower, full-upper, candidate raising, nasal-marker case adjust) | Rust `engine/phonetics/src/case_transform.rs` | Cross-platform canonical |
| iOS bridge | `Engine/RustEngineBridge+CaseTransform.swift` | Wraps `transformInputCase` |
| Android bridge | `engine/CaseTransformBridge.kt` | Superset: `transformInputCase` + the 3 per-char ops (`KeyLabelCaseCache` key labels) |
| iOS shift / capslock state | KeyboardKit (managed) | Drives `LetterCase` value passed into bridge |
| Android shift / capslock state | `ime/text/CapsStateManager.kt` | Same role, calls bridge per keystroke |

Candidate case: the engine cases each candidate from the user's own raw input (`composing::continuous::recase_roman`), so the strip shows what was typed — candidates arrive cased on both platforms.
