# KeyboardKit auto-capitalization (issue #967) — verification finding

> **Type**: Report (dated snapshot, frozen)
> **Keywords**: `KeyboardKit`, `autocapitalization`, `#967`, `setupKeyboardKit`, `KeyboardApp`, `v3.6.6`
> **Date**: 2026-08-28
> **Verified on**: `~/Workspace/keyboard-benchmark` (iOS keyboard extension, KeyboardKit 10.9.0, real device)
> **Scope**: USER 2026-08-28 — 「這個問題會安排在 3.6.6 fix」

---

## Summary

`state.keyboardContext.settings.isAutocapitalizationEnabled = false` failing to suppress
sentence-initial uppercase is **not a KeyboardKit bug**. It is a consequence of initializing
the keyboard through the legacy settings-store API. KeyboardKit 10.8.1 fixed the reported
behavior ([issue #967](https://github.com/KeyboardKit/KeyboardKit/issues/967)), but the fix is
only reachable when the controller is set up through `setupKeyboardKit(for: KeyboardApp)`.

Confirmed empirically on device: the standard setup path produces correct lowercase behavior
with **zero** workaround code.

## What was verified

Benchmark extension, KeyboardKit `10.1.4` → `10.9.0`. Five attempts failed before the root
cause was found; each was ruled out on device, not by code reading.

| # | Attempt | Result |
|---|---|---|
| 1 | `settings.isAutocapitalizationEnabled = false` in `viewDidLoad`, after `KeyboardSettings.store = .standard` | uppercase |
| 2 | Same, plus `keyboardContext.autocapitalizationTypeOverride = .none` | uppercase |
| 3 | Same state moved to `viewWillSetupKeyboardKit()` | uppercase |
| 4 | `override viewWillSetupInitialKeyboardCase() { setKeyboardCase(.lowercased) }` | uppercase |
| 5 | TaigiKeyboard's `textDidChangeAsync` workaround ported over | uppercase |
| **6** | **`setupKeyboardKit(for: KeyboardApp)` in `viewWillSetupKeyboardKit()`, settings written in its completion — all workarounds removed** | **lowercase (correct)** |

Attempts 1-5 all wrote settings against a store that had not been initialized through the
supported entry point, so the value never reached the case logic. Attempt 4's and 5's overrides
were then defeated further downstream.

The working setup:

```swift
extension KeyboardApp {
    static var benchmark: KeyboardApp { .init(name: "Keyboard Benchmark") }
}

class KeyboardViewController: KeyboardInputViewController {

    override func viewWillSetupKeyboardKit() {
        setupKeyboardKit(for: .benchmark) { [weak self] result in
            if case .failure(let error) = result { print(error) }
            self?.setupBenchmarkState()
        }
    }

    private func setupBenchmarkState() {
        state.keyboardContext.keyboardType = .alphabetic
        state.keyboardContext.settings.isAutocapitalizationEnabled = false
        services.autocompleteService = AppleAutocompleteService(language: "en_US")
    }
}
```

## Why the legacy path fails

KeyboardKit 10.8.1 release notes:

> The input controller will now make set up App Group syncing **before accessing any settings**.
> […] The input controller will also clean up its keyboard case logic, which caused some initial
> flickering and sometimes some flaky behavior.

`setupKeyboardKit(for:)` is what guarantees that ordering. Assigning `KeyboardSettings.store`
(or calling `KeyboardSettings.setupStore(...)`) from `viewDidLoad` and then writing a setting
does not: the case logic has already read its inputs by then.

Two corrections to earlier hypotheses raised during this investigation, recorded so they are not
re-derived:

- `textDidChangeAsync` calling `super` → `setKeyboardCase(preferredKeyboardCase)` is **not** the
  root cause. It does overwrite the case, but only because the setting was never read correctly;
  once setup is standard, that path behaves.
- `viewWillSetupInitialKeyboardCase()` (new in 10.8.1, absent from the 9.9.0 DocC archive in
  `references/KeyboardKit-Documentation/`) is **not** the required override point. Overriding it
  alone does not fix the symptom.

## Impact on TaigiKeyboard (v3.6.6)

TaigiKeyboard is on the legacy path and carries two workaround layers for this exact symptom,
both marked `FIXME`:

| Item | Location |
|---|---|
| Store init via legacy API in `viewDidLoad` → `setupServices()` | `ios/Sources/TaigiKeyboard/KeyboardExtension/KeyboardViewController.swift:93` → `KeyboardViewController+Setup.swift:16` (`KeyboardSettings.setupStore(forAppGroup:)`) |
| Workaround layer 1/2 — `textDidChangeAsync` skips `super` when auto-cap is off | `KeyboardViewController.swift:209` |
| Workaround layer 2/2 — Combine guard on `$keyboardCase` restoring after `keyboardType → .alphabetic` | `KeyboardViewController.swift:232` (`setupKeyboardCaseProtection()`) |

The benchmark result indicates both layers become unnecessary once setup migrates to
`setupKeyboardKit(for:)`. **Not yet verified in TaigiKeyboard itself** — the benchmark extension
has no App Group, so the migration there was trivial.

Open questions for the v3.6.6 round (each needs its own verification before any code lands):

1. TaigiKeyboard's KeyboardKit is pinned at **10.4.1**; the fix requires ≥ 10.8.1. The upgrade
   plan already exists: [`docs/reports/2026-08-21-keyboardkit-10.8.1-upgrade-plan.md`](2026-08-21-keyboardkit-10.8.1-upgrade-plan.md).
2. `KeyboardApp(appGroupId:)` vs. today's `KeyboardSettings.setupStore(forAppGroup:)` — whether
   the resulting settings keys are identical. `KeyboardApp` also takes
   `keyboardSettingsKeyPrefix`, so a prefix mismatch would strand existing users' settings.
3. Whether removing both workaround layers regresses the cases they were written for, in
   particular the `keyboardType → .alphabetic` transition that layer 2 guards.

## References

- Issue: <https://github.com/KeyboardKit/KeyboardKit/issues/967> — maintainer comment
  [5354125771](https://github.com/KeyboardKit/KeyboardKit/issues/967#issuecomment-5354125771)
  (2026-08-20): "This has been fixed in the upcoming 10.8.1 […] There is no new settings to
  consider, just changes to how the controller initializes things and when in the launch cycle."
- KeyboardKit 10.8.1 release notes: <https://github.com/KeyboardKit/KeyboardKit/releases/tag/10.8.1>
- KeyboardKit 10.9.0 release notes: <https://github.com/KeyboardKit/KeyboardKit/releases/tag/10.9.0> —
  no auto-capitalization changes; 10.8.1 is the relevant version.
- KeyboardKit demo's own setup, for comparison:
  `Demo/Keyboard/KeyboardViewController.swift` in the KeyboardKit checkout (uses
  `viewWillSetupKeyboardKit()` + `setupKeyboardKit(for:)`).
- From 10.9.0 onward KeyboardKit ships as a binary target — implementation source is no longer
  readable; the API surface is in the `.swiftinterface` inside the built framework.
