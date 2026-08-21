# KeyboardKit 10.4.1 → 10.8.1 Upgrade Plan (2026-08-21)

Status: **PLAN ONLY** — no code changed. Doc-only round; upgrade round(s) gated on USER.

## Goal

Upgrade iOS KeyboardKit from pinned **10.4.1** to **10.8.1** to pick up the upstream
auto-capitalization fix and evaluate retiring our multi-layer autocap workaround.

## Motivation (why 10.8.1 specifically)

- **Issue [#967](https://github.com/KeyboardKit/KeyboardKit/issues/967)** ("initial
  first-letter still capitalized when `isAutocapitalizationEnabled = false`") is the bug
  we commented on (siansiansu, 2025-12-28) and worked around locally. Author
  danielsaidi closed it 2026-08-20 (milestone 10.8.1):
  > "This has been fixed in the upcoming 10.8.1, where the initial case is handled in a
  > more reliable way. There is no new settings to consider, just changes to how the
  > controller initializes things and when in the launch cycle."
- 10.8.1 release notes (2026-08-20) also fix:
  - App Group sync now set up **before** any settings access — fixes locales set in the
    main app not syncing to the keyboard extension.
  - Initial keyboard-case logic cleanup — removes launch flickering / flaky initial case;
    keyboard now honors the text field's preferred autocapitalization.

## Current state (grounded in code, 2026-08-21)

| Item | Value |
|---|---|
| Pin | `exactVersion 10.4.1` in `ios/TaigiKeyboard.xcodeproj/project.pbxproj:998-999` (**user-only file** — Claude cannot edit; version bump must be done by USER in Xcode) |
| `Package.resolved` | KeyboardKit 10.4.1 (`16c76394`), LicenseKit 2.1.3, swift-protobuf 1.38.0 |
| KK importers | 49 Swift files `import KeyboardKit` |
| Setup API | Already on current `viewWillSetupKeyboardView()` (`KeyboardViewController.swift:115`, `+Setup.swift:269`) — no setup-API migration needed |
| `hostApplicationBundleId` | Not used — 10.6 deprecation irrelevant |

Note: `.claude/rules/doc-lookup.md` still describes the pin as "range 9.7.2 ..< 11.0.0,
local clone 9.9.0" — **stale**; actual pin is exact 10.4.1. Update that rule when the
upgrade lands (local `references/keyboardkit9.9.0/` clone + DocC archive also need a
matching refresh to stay authoritative).

## Existing autocap workaround inventory (candidate for retirement)

Three defensive layers, all working around the pre-10.8.1 initial-case behavior:

1. **Setup-time sync** — `KeyboardViewController+Setup.swift:273-288`: reads
   `settings.isAutocapitalizationEnabled`, sets/clears
   `autocapitalizationTypeOverride`, forces `keyboardCase = .lowercased`.
2. **Combine guard** — `KeyboardViewController.swift:225-274`: observes
   `keyboardContext.$keyboardCase`, blocks unexpected mutations from KK's internal
   `setKeyboardCase(preferredKeyboardCase)` path (documented at `:98`, `:210`, `:228-232`).
3. **Per-gesture enforcement** — `ActionHandler.swift:170-185`: re-checks case after
   shift/auto-cap transitions.

10.8.1 changes exactly the code path these guard against ("cleans up its initial case
logic"). Per the workaround circuit-breaker rule, the eventual fix here is **removing
code, not keeping both** — but removal is a separate, dogfood-gated round (see Phases).
Risk if kept as-is: layer 2 fights KK's *new* (correct) case logic — e.g. blocking a now-
legitimate case change — so "keep workaround + upgrade" is NOT a safe default either.
Dogfood must cover both states.

### Can the workaround be removed safely? (analysis 2026-08-21)

**Static verification is impossible**: KeyboardKit 10+ is closed-source; the GitHub
10.8.0...10.8.1 compare touches only Demo/, `Package.swift` (binary bump), and
RELEASE_NOTES. Evidence = author's claim + release notes wording only. Mapping each
guarded path against what the notes explicitly claim:

| Layer | Guarded path | Explicitly claimed fixed in 10.8.1? |
|---|---|---|
| 1 — Setup sync (`+Setup.swift:272-290`, forced `.lowercased` + `autocapitalizationTypeOverride = .none`) | Initial case at launch | **Yes** — "cleans up its initial case logic"; author: "initial case handled in a more reliable way… no new settings to consider" |
| 1/2 — `textDidChangeAsync` override (`KeyboardViewController.swift:211-220`, skips `super` entirely when autocap OFF) | Per-text-change `keyboardCase = preferredKeyboardCase` | **Partially** — notes say keyboard now honors the *text field's* preferred autocapitalization (a different axis); per-change respect of `isAutocapitalizationEnabled = false` not explicitly stated |
| 2 — Combine guard (`KeyboardViewController.swift:234-285`) | numeric/symbols → alphabetic switch forcing `.uppercased` via internal path | **No** — notes mention launch cycle only; type-switch path unmentioned |
| 3 — `tryChangeKeyboardCase` (`ActionHandler.swift:166-189`, skips `super` unless shift or autocap ON) | Per-gesture auto-cap (e.g. after `. `) | **No** — unmentioned |

Both directions carry risk: removing layers 2/3 may regress the mid-session #967
symptom if 10.8.1 only fixed launch; keeping the `textDidChangeAsync` override skips
whatever `super` gained between 10.4.1 and 10.8.1 (unknowable, closed source), and
the Combine guard may block now-correct case changes. Hence **staged removal in
Phase 3, ordered by fix-coverage confidence**: layer 1 first (positively covered) →
`textDidChangeAsync` + layer 3 → layer 2 last, with a dedicated type-switch dogfood
step (autocap OFF → type → switch to numeric → back to ABC → keys must stay
lowercase) before touching it. Each step re-runs the full Phase 2 matrix.

## Release-by-release delta (10.5.0 → 10.8.1)

| Version | Date | Impact on us |
|---|---|---|
| 10.5.0 | 2026-05-19 | Additive (accessibility settings, Arabic, emoji). `Keyboard.SettingsScreen` moved → `KeyboardSettings` (we don't use KK settings screens). Low. |
| 10.5.1 | 2026-06-02 | License info only. None. |
| 10.6.0 | 2026-06-22 | **Namespace flattening wave 1** (renames w/ deprecated shims): `KeyboardLayout.Item` → `KeyboardLayoutItem`, `KeyboardLayout.DeviceConfiguration` → `KeyboardLayoutConfiguration`. Fewer on-launch redraws. Spacebar vertical drag added. |
| 10.6.1 | 2026-07-02 | `hostApplicationBundleId` soft-undeprecated. None. |
| 10.7.0 | 2026-07-04 | **Namespace flattening wave 2**: `Autocomplete.*`, `Callouts.*`, `Feedback.*`, `Dictation.*` flattened; `KeyboardAction.StandardActionHandler` → `StandardKeyboardActionHandler`. Old names deprecated, **removed in KK 11**. Undo experiment (opt-in, ignore). |
| 10.7.1–10.7.3 | 2026-07 | Picker additions, emoji button style tweak. None. |
| 10.8.0 | 2026-08-17 | **Settings types → `ObservableObject` classes**, injected into `KeyboardState` + SwiftUI environment. Context initializers deprecated in favor of settings-instance variants. `KeyboardAppView` no longer observes keyboard state. Autocomplete gains user dictionaries/ignored words (additive). |
| 10.8.1 | 2026-08-20 | **The target fixes**: earlier SDK init (App Group before settings reads), initial-case logic cleanup (= #967 family). |

## Our deprecated-symbol exposure (compiles on 10.8.1, breaks on KK 11)

Audit of `ios/Sources` (excluding false positives: our own `Fonts` in
`KeyboardFonts.swift`, `textDocumentProxy.*` matches):

| Old symbol (still used by us) | Uses | New name (since) |
|---|---|---|
| `Autocomplete.Suggestion` | 48 uses / 15 files | `AutocompleteSuggestion` (10.7) |
| `Autocomplete.Result` | 6 | `AutocompleteResult` (10.7) |
| `Autocomplete.ToolbarStyle` | 2 | `AutocompleteToolbarStyle` (10.7) |
| `Callouts` namespace — we **extend** it in 3 files (`Callouts/Callouts+TaigiCallout{Builder,Maps,Style}.swift`) + `Callouts.CalloutStyle` ×5, `Callouts.Actions` | ~10 | `KeyboardCalloutActions` / `KeyboardCalloutStyle` (10.7) |
| `KeyboardLayout.Item` | 2 | `KeyboardLayoutItem` (10.6) |
| `KeyboardLayout.DeviceConfiguration` | 2 | `KeyboardLayoutConfiguration` (10.6) |
| `KeyboardAction.StandardActionHandler` | 1 | `StandardKeyboardActionHandler` (10.7) |

All shims survive through 10.x → upgrade compiles with **deprecation warnings only**.
Renaming now is optional but pays down the KK 11 migration up front.

## Risk assessment

| Risk | Severity | Mitigation |
|---|---|---|
| 10.8.1 new init order vs our `viewWillSetupKeyboardView` / setup-time autocap sync (layer 1) — ordering assumptions may shift | Medium | Compile + full autocap dogfood matrix (below) before touching workaround |
| Combine guard (layer 2) fighting KK's corrected case logic → blocked legitimate case changes (shift stuck / no sentence-cap when autocap ON) | Medium | Dogfood both autocap ON and OFF; expect a follow-up removal round |
| 10.8.0 `ObservableObject` settings conversion vs our `SharedSettings` / `EngineSettingsProvider` wiring (`.claude/rules/ios-settings-injection.md` live-read invariants) | Medium | `context.settings` access path still exists (only initializers deprecated); verify live-read behavior in dogfood |
| 10.6.0 spacebar drag rework (vertical drag, `SpacebarDragGestureHandler` / `SpacebarDragSensitivity` changes) lands on the exact path our #545 fix targets (drag offset not reset → dead spacebar) | **High** | Re-run the full #545 dogfood (long-press drag cursor, release, spacebar must still type) — see `memory/project_ios_spacebar_drag_dead.md` |
| Launch-redraw reduction (10.6) + flicker fix (10.8.1) changing first-frame behavior of our custom `TaigiKeyboardView` | Low | Visual dogfood |
| LicenseKit transitive bump | Low | SPM resolves automatically on pin change |
| Deprecation warnings noise (~70 sites) | Low | Optional rename pass (Phase 3) |

## Phases (each = own round, USER-gated; no release version assigned)

1. **Version bump + compile** — USER changes pbxproj pin `10.4.1` → `10.8.1` in Xcode
   (Claude blocked from `.pbxproj`). Claude fixes any hard compile breaks (expected:
   none — shims cover all our uses). Workarounds stay in place. Build + test gate:
   `xcodebuild test` per CLAUDE.md § Build & Test. iOS-only; no `make build`.
2. **Autocap dogfood (workarounds still in)** — real-device matrix:
   - autocap OFF: fresh keyboard open → first letter lowercase (the #967 symptom);
     new sentence after `. ` → still lowercase; shift + caps-lock still work manually.
   - autocap ON: sentence-start capitalizes; no flicker; no stuck shift (watch for
     layer-2 guard fighting new KK logic).
   - locale/App Group: settings changed in host app reflect in keyboard without
     relaunch (10.8.1 sync fix; also re-check `ios-settings-injection.md` invariants).
   - **spacebar drag (#545 regression check)**: long-press spacebar → drag cursor →
     release → spacebar must still insert space (10.6.0 reworked this gesture path).
3. **Workaround retirement round** (separate PR, only after Phase 2 evidence) —
   remove layers 1-3 one at a time, re-running the same matrix; circuit-breaker rule
   says expected end state = platform/KK default with zero custom case enforcement.
   Codex sandwich applies (design judgment: which layers are provably redundant).
4. **Optional deprecation-rename pass** (mechanical, shim → new names, ~70 sites)
   — pure grep-replace tier, skips Codex sandwich; can ride any later round. Pays
   down KK 11 debt.
5. **Doc/rule refresh** — update `.claude/rules/doc-lookup.md` pin info + refresh
   `references/keyboardkit*` local clone/DocC archive to 10.8.1.

Phase 1-2 are the upgrade; 3-5 are follow-ups the upgrade unlocks. Scheduling is
USER's call.

## Open questions for USER

- Timing: upgrade before or after current dogfood backlog (B1 etc.) clears?
- Phase 3 appetite: retire workaround immediately after Phase 2 passes, or soak longer?
- Phase 4: fold renames into Phase 1 (bigger diff, less warning noise) or defer?
