# IME Window/Inset Architecture Fix Plan

**Status:** approved (Codex + Claude joint, 2026-04-25), awaiting implementation in a clean branch from `main`
**Replaces:** PR #179 `fix/android-reactive-layout-pipeline` (closed as failed investigation)
**Branch (next):** `fix/ime-default-window-sizing`

## 1. Problem

Android IME intermittently dismisses itself on first show across multiple host
apps (Discord, Chrome, others). User-visible symptom: keyboard appears for
<300 ms then disappears, sometimes recovers on second tap. Reported reproducible
in v3.5.0 (post-`v3.4.9`); user-confirmed v3.4.9 did not exhibit the bug at this
rate.

Stack trace at `onFinishInputView` confirms system-initiated hide:
```
hideSoftInputWithToken                       ← IPC entry from system_server
hideSoftInput → hideWindow → finishViews → onFinishInputView
```

`finishingInput=false` in every failure case → "hide window but keep session"
state; system, not user, decided to hide.

## 2. Failed investigation (PR #179, eight commits)

All eight commits targeted the wrong layer:

| # | Commit | Layer | Outcome |
|---|---|---|---|
| 1 | `45233c3` reactive layout pipeline | layout state mgmt | Architecture cleanup, kept value, did not fix dismiss |
| 2 | `181924d` `registeredInputView` idempotent guard | lifecycle | Tangent |
| 3 | `f51301b` drop `distinctUntilChanged` on `StateFlow` | build | Cosmetic |
| 4 | `25e55eb` skip composing pre-zero on session start | InputConnection layer | Wrong theory |
| 5 | `bd83e2f` `isGlobeKeyEnabled` reactive | layout state | Tangent |
| 6 | `5b67c56` layout-before-flipper.addView ordering | layout state | Partial help (empty-view race) |
| 7 | `51a8043` Chrome `TYPE_NULL` flicker `requestShowSelf` mitigation | symptom recovery | Recovered some Chrome cases |
| 8 | `af780f8` widen mitigation timing windows | symptom tuning | Tuning |

Net effect: ~100% failure → ~5% failure across multiple host apps. New host
scenarios (Discord without `TYPE_NULL`) appearing as the bug surfaced through
different code paths. No commit reduced failure to zero.

Lesson recorded in `feedback_workaround_circuit_breaker` (auto-memory): when
3+ fixes for the same bug fail to fully resolve it, stop coding and compare
to working reference implementations.

## 3. Root cause (architectural)

The hazard is a **three-element combination**, not any single piece:

1. `onConfigureWindow` overridden to force `MATCH_PARENT × MATCH_PARENT`
2. Custom `onComputeInsets` reporting based on live child-view position
   (`inner_input_view_container.getLocationInWindow()`)
3. `outInsets.touchableInsets = TOUCHABLE_INSETS_VISIBLE`

In a default `WRAP_CONTENT` IME window, `topInset = 0` means "my own small
window is touchable". In our `MATCH_PARENT` window, `topInset = 0` means
**"the entire screen is touchable by the IME"**.

When the inner container is mid-layout — host focus dance (Chrome WebView
`TYPE_NULL` flicker, Discord rapid input transitions), view tree relayout,
fresh attach pre-measure-pass — `getLocationInWindow()` returns `(0,0)` →
`topInset = 0` → IME claims full-screen touchable area → system_server
hides via `hideSoftInputWithToken`.

The intermittent nature explains why incidental fixes (e.g., commit 6's
layout-before-flipper ordering) reduced but did not eliminate the failure
rate — they shrank one of several windows in which the inner container is
not yet positioned, but did not address the underlying inset-misreport
hazard.

### Why v3.4.9 didn't show this clearly

The `MATCH_PARENT` overrides date back to `Initial Commit (3b5644c, 2025-11-21)`
under the comment "Follow fcitx5-android approach: set window to MATCH_PARENT".
The bug existed in v3.4.9 too but the rate was lower: pre-Phase-II
`TextInputManager` was a `companion getInstance()` singleton, so
`keyboardViews` survived IME service teardown and `onComputeInsets` rarely
saw an empty inner container. PR #156 (Phase II A7) replaced the singleton
with a per-IME-service ctor, exposing `inner_input_view_container.height = 0`
windows on every fresh service start — pushing the failure rate from ~rare
to ~visible.

## 4. Reference patterns

### Aiongtaigi (Option A target — no overrides)

`references/aiongtaigi-sushi/sources/com/aiongtaigi/sushi/ime/TaiwaneseIME.java`:
- No `onConfigureWindow` override
- No `setInputView` height-forcing
- No `onComputeInsets` override
- UI: Compose; state: uniffi-bridged Rust engine
- `setFlags(0, 16)` clears `FLAG_NOT_TOUCHABLE` — defensive/no-op, do not copy

Default IMS path: full width, `WRAP_CONTENT` height in non-fullscreen mode;
`setInputView` adds the input view as `MATCH_PARENT × WRAP_CONTENT`;
`onComputeInsets` computes from framework-owned input/candidate frames.

### FlorisBoard (Option B fallback — overridden but defensive)

`references/florisboard/app/src/main/kotlin/dev/patrickgold/florisboard/ime/window/ImeWindowController.kt:204-244`:
- Keeps full-screen `MATCH_PARENT` window
- Maintains cached `activeWindowInsets` updated from `OnApplyWindowInsetsListener`
- **Returns early when cache not yet known**:
  `val windowInsets = activeWindowInsets.value ?: return`
- Uses `TOUCHABLE_INSETS_REGION` + explicit `touchableRegion`, NOT `VISIBLE`
- Never reports bad transient values

## 5. Option A — the redirect

Branch: `fix/ime-default-window-sizing` (from `main`)

Four mechanical changes to
`android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/TaigiKeyboard.kt`:

1. **Remove `onConfigureWindow` override entirely.** Framework default is
   `MATCH_PARENT × WRAP_CONTENT` in non-fullscreen mode — what we want.
2. **Remove the `setInputView` override** that forces `MATCH_PARENT` height
   on `inputArea` and the input view. Default `setInputView` adds the input
   view with `WRAP_CONTENT`.
3. **Remove `onComputeInsets` override entirely.** Framework computes from
   its owned input/candidate frames.
4. **Verify `taigikeyboard.xml` root height.** Change to `wrap_content` if
   currently `match_parent`, OR verify default `setInputView` overrides it
   correctly.

**Do NOT include**:
- Chrome `requestShowSelf` recovery (commit 7 / `51a8043`)
- `TYPE_NULL` flicker tracking (commits 7 / 8)
- Mitigation timing-window tuning (commit 8)

These were band-aids for the misreported insets. With insets correct, they
should not be needed.

The `5b67c56` ordering fix (layout-before-flipper) is also not strictly
required after Option A, but the new ordering is cleaner code regardless —
defer that decision to the implementer based on how clean the diff comes out.

## 6. Validation

User runs on device after install:
- Open keyboard 30+ times in Discord, Chrome, generic text fields, search bars
- Failure rate target: 0/30 (was 1-2/20 on PR #179 last build)
- If 0/30 → Option A confirmed, proceed to merge
- If non-zero on Chrome specifically → consider lightweight Chrome-only
  workaround as a follow-up PR, separate from Option A
- If non-zero broadly → pivot to Option B (FlorisBoard pattern, see §4)

Visual checks (any of these may regress; record what you see):
- Keyboard background extension behind navigation bar (currently relies on
  full-screen window + `setOnApplyWindowInsetsListener` adding bottom padding)
- Bottom padding for gesture / 3-button nav
- Nav bar icon contrast vs current keyboard theme (`NavigationBarManager`)

If a visual regression appears, **selectively reintroduce only the visual
piece needed** (e.g., a focused window flag). Do NOT bring back the full
`MATCH_PARENT` + custom inset stack — that defeats Option A.

## 7. Process

Per `feedback_round_hygiene` and `feedback_codex_review_sandwich`:

1. New session reads memory (`project_ime_window_arch`,
   `project_ime_dismiss_v3_5_0_redirect`, this plan doc) + current `main`
2. Branch `fix/ime-default-window-sizing` from `main`
3. Apply 4-step removal in §5
4. `./gradlew assembleDebug` (verify build)
5. `./gradlew test` (verify unit tests still green)
6. Codex pre-impl review of the diff (architecture-restoring, low risk;
   brief review acceptable)
7. Codex post-impl review of the same diff (sanity)
8. Hand to user for device testing per §6
9. PR open: title `fix(android): use default IMS window sizing to prevent
   spurious dismiss`; body links to this plan + closed PR #179
10. After merge, prune `project_ime_dismiss_v3_5_0_redirect` from MEMORY.md;
    `project_ime_window_arch` carries forward as durable knowledge

## 8. Decision rule for future IME work

| Need | Path |
|---|---|
| Type and show insets correctly (the common case) | Option A — default IMS, no override |
| Keyboard background behind navbar / edge-to-edge / full-screen drawing | Option B — FlorisBoard pattern: cached bounds, `TOUCHABLE_INSETS_REGION`, explicit `touchableRegion`, early-return when bounds unknown |
| Anything else | Default to A; require written justification before B |

**Never** combine `MATCH_PARENT × MATCH_PARENT` window with
`TOUCHABLE_INSETS_VISIBLE` + live-measured insets again. That is the failure
mode this document records.
