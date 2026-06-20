# Keyboard Body Invariants — Android Compose

**Status**: extracted from `behavioral-invariants.md` §16 on 2026-05-26 as part of the P3 doc-size split — same authoritative invariants, separated because the section is Android-only (the iOS keyboard is a KeyboardKit-driven SwiftUI tree with its own platform contract). Kotlin source comments referencing `INVARIANT_keyboard_*` labels are unchanged.

**Scope**: Android Compose keyboard body — touch dispatch, `PopupWindow` ownership, `WindowInsets` consumption. Phase D of Roadmap Item 4 (Compose migration) pinned these invariants when the View-subclass keyboard body retired in favour of a Compose tree consuming P2's `KeyboardLayoutData` + `KeyboardLayoutSolver`.

**Why a dedicated file**: the parent `behavioral-invariants.md` is the *cross-platform* behavior contract. Android-only invariants live here so cross-platform readers do not load Android-specific surface area, and Android engineers have a single canonical anchor for keyboard-body invariant lookups.

**Drift policy**: see `behavioral-invariants.md` § Change protocol — same rules apply.

---

## Index

1. [`INVARIANT_keyboard_touch_hit_box_matches_visible_bounds`](#invariant_keyboard_touch_hit_box_matches_visible_bounds)
2. [`INVARIANT_keyboard_long_press_delay_unchanged`](#invariant_keyboard_long_press_delay_unchanged)
3. [`INVARIANT_keyboard_popup_drag_select_tracks_pointer`](#invariant_keyboard_popup_drag_select_tracks_pointer)
4. [`INVARIANT_keyboard_multi_touch_first_pointer_cancels`](#invariant_keyboard_multi_touch_first_pointer_cancels)
5. [`INVARIANT_keyboard_navbar_inset_padding_factor`](#invariant_keyboard_navbar_inset_padding_factor)
6. [`INVARIANT_keyboard_navbar_dismiss_bug_stays_resolved`](#invariant_keyboard_navbar_dismiss_bug_stays_resolved)
7. [`INVARIANT_keyboard_action_cancel_unconditional_cleanup`](#invariant_keyboard_action_cancel_unconditional_cleanup)
8. [`INVARIANT_keyboard_popup_hide_clears_anchor`](#invariant_keyboard_popup_hide_clears_anchor)
9. [`INVARIANT_keyboard_popup_hide_dismisses_preview_window`](#invariant_keyboard_popup_hide_dismisses_preview_window)
10. [`INVARIANT_keyboard_press_starts_only_on_down`](#invariant_keyboard_press_starts_only_on_down)
11. [`INVARIANT_keyboard_register_input_view_main_thread_setup`](#invariant_keyboard_register_input_view_main_thread_setup)

---

## Why these invariants exist

Phase D structurally retires the dismiss-bug hazard documented in `memory/project_ime_window_arch.md`. Any drift on these invariants risks resurrecting that class of bugs (off-by-pixel ghost taps, pinned-popup latency drift, lost multi-touch cancellation, navbar-area inset re-tunes that re-introduce `MATCH_PARENT × MATCH_PARENT` + `TOUCHABLE_INSETS_VISIBLE` pairing).

---

### `INVARIANT_keyboard_touch_hit_box_matches_visible_bounds`

The pointer hit-box of every key matches the visible bounding box of that key, including the `key_marginH` × `key_marginV` margin band that the legacy `KeyView.updateTouchHitBox` extended outward beyond the View bounds for first/last-in-row keys (left edge → 0, right edge → `keyboardView.measuredWidth`). No off-by-pixel ghost taps near key edges; no dead band between keys.

### `INVARIANT_keyboard_long_press_delay_unchanged`

Long-press popup extension fires at `prefs.longPressDelay` ms after `ACTION_DOWN` (default 300 ms; user-configurable). Special long-press paths — SPACE → IME picker, LANGUAGE_SWITCH → IME picker, DELETE repeat (50 ms repeat after 500 ms initial delay) — preserve their pre-D timings.

### `INVARIANT_keyboard_popup_drag_select_tracks_pointer`

While the extended popup is showing, finger movement across the popup glyph row updates `activeIndex` per `KeyPopupManager.propagateMotionEvent` boundary math (`anchorSide` + `row0count` + `row1count` + `anchorOffset` from `KeyboardLayoutSolver.solveExtendedPopupGeometry`). Y-axis bounds: `event.y < -keyPopupHeight` or `event.y > 0.9f * keyPopupHeight` returns `false` (out of bounds). On `ACTION_UP`, `activeKeyData()` returns the popup variant at `activeExtIndex`, falling back to `anchor.data` if no variant is active.

### `INVARIANT_keyboard_multi_touch_first_pointer_cancels`

When a second `ACTION_POINTER_DOWN` arrives while a first pointer is mid-press, the IME synthesizes an `ACTION_UP` for the first pointer's active key BEFORE handing focus to the new pointer's hit-tested key. Two pointers never simultaneously drive two key-press flows. The synthetic `ACTION_UP` commits the first pointer's press unless `shouldBlockNextKeyCode` (set by long-press paths SPACE / LANGUAGE_SWITCH → IME picker) is true — matching the legacy `KeyboardView.onTouchEvent` "send ACTION_UP to current active view and move on" path.

Distinct from this: a same-`pointerId` `ACTION_DOWN` arriving without a clearing UP/CANCEL (stale-state recovery) drops the prior press without committing — mirrors legacy `KeyboardView.onTouchEvent`'s "ACTION_DOWN with stale pointerId" branch.

### `INVARIANT_keyboard_navbar_inset_padding_factor`

The bottom padding of the keyboard body equals `max(navigationBars.bottom, mandatorySystemGestures.bottom, systemGestures.bottom) × 0.90f` (truncated to int / converted to dp). The 0.90 factor is empirical-tuned to keep the keyboard visually close to the navigation-bar boundary without overlapping the gesture region. Any deviation re-introduces the visual shift that necessitated the factor in the first place.

### `INVARIANT_keyboard_navbar_dismiss_bug_stays_resolved`

Opening the IME in Discord, Chrome, and any default `WRAP_CONTENT`-host app does not cause an "open then immediately dismiss" cycle. This is the regression surface for the `project_ime_window_arch.md` 3-element hazard (`onConfigureWindow` overridden to `MATCH_PARENT × MATCH_PARENT` + custom child-position insets + `TOUCHABLE_INSETS_VISIBLE`). Phase D removed the `inner_input_view_container.setPadding` workaround; `InputView.onApplyWindowInsets` now derives the `WindowInsets`-based bottom padding declaratively (see `INVARIANT_keyboard_navbar_inset_padding_factor`), applied through the Compose `KeyboardImeRoot`.

### `INVARIANT_keyboard_action_cancel_unconditional_cleanup`

`MotionEvent.ACTION_CANCEL` aborts the active gesture wholesale: pressed state, popup window, scheduled long-press, and DELETE-repeat runnables are all torn down regardless of which pointer id the framework reported on `actionIndex`. The pointer-id gate used for `ACTION_UP` does NOT apply — the framework does not guarantee `ACTION_CANCEL` reports the active pointer id, and gating cancel on a mismatch leaves the keyboard with a stuck pressed key or a runaway DELETE repeat until an unrelated event clears state. Cancel never commits the press (`committed = false`).

### `INVARIANT_keyboard_popup_hide_clears_anchor`

`PopupHost.hide()` clears `lastAnchor` so the next call to `activeKeyData()` reflects the *current* press lifecycle, not the prior one. `KeyTouchCoordinator.endKeyPress` resolves the dispatched KeyData via `popupHost.activeKeyData() ?: activeKey?.data`; without this clear, a press whose hit-test misses (`activeKey == null`) but whose UP fires before the next press would commit the previous anchor's KeyData. The same clear runs in both teardown paths: `hide()` (per-press, called from `endKeyPress`) and `dismissAllPopups()` (full teardown, called from `KeyTouchCoordinator.reset` and `TextInputManager.onDestroy`).

### `INVARIANT_keyboard_popup_hide_dismisses_preview_window`

`PopupHost.hide()` calls `PopupWindow.dismiss()` on the preview popup (and the extended popup, when showing) — not just toggle the Compose `_previewState` flow. Phase D's switch from `showAsDropDown(keyView, ...)` to `showAtLocation(rootView, ...)` removed the implicit anchor-View lifecycle cleanup, so toggling state alone left the previous Compose frame painted on the popup decor view until the next press redrew it (visible as a lingering callout on tap UP). The Compose state toggle still fires first so `KeyPopupBox` recomposes to empty before the window dismisses, but the explicit `dismiss()` is what guarantees immediate visual removal.

### `INVARIANT_keyboard_press_starts_only_on_down`

A keyboard press is initiated only on `ACTION_DOWN` / `ACTION_POINTER_DOWN`. If the initial DOWN's hit-test misses (the touch starts above the keyboard on the smartbar / overlay, or in a layout gap), subsequent `ACTION_MOVE` events do NOT promote the touch to a press even if the finger drifts onto a key. The touch stays dormant until UP / CANCEL clears it. This deliberately diverges from the legacy `KeyboardView.onTouchEvent` synthetic-DOWN-on-MOVE rescue (now retired) — Phase D §1b parity-correction aligns the contract with AOSP LatinIME `PointerTracker.onMoveEvent`, which has identical "no MOVE-driven press promotion" semantics. Multi-touch handoff stays unaffected because `handleDown` calls `startKeyPress` explicitly when a second pointer arrives.

### `INVARIANT_keyboard_register_input_view_main_thread_setup`

`TextInputManager.onRegisterInputView` resolves `textViewGroup`, synchronously publishes the active mode's layout + appearance + active mode into `KeyboardUiState` (via `ensureLayoutLoadedNow`), mounts the keyboard ComposeView, and registers all four smartbar overlays **synchronously** before returning. The background `ensureLayoutLoaded` coroutine is reserved for runtime reload paths (subtype / input-mode / layout-type / settings changes), not the boot path. This ordering is load-bearing because `InputMethodService.onWindowShown` calls `setActiveInput(R.id.text_input)` immediately after the IME window first becomes visible, and that call reads `textInputManager.textViewGroup` synchronously. If the resolution sat behind a `launch(Dispatchers.Default) { withContext(Main) { ... } }` thread-hop, `onWindowShown` could race ahead, find `textViewGroup == null`, and `ViewGroup.indexOfChild(null) == -1` would feed `ViewAnimator.setDisplayedChild(-1)`, which wraps to `childCount - 1` (= the `media_input` emoji keyboard) — the "first-install opens emoji keyboard" symptom. `TaigiKeyboard.setActiveInput` defends against the same class of bug by clamping the resolved index with `coerceAtLeast(0)`.

### `INVARIANT_keyboard_body_layout_published_before_compose_mount`

`TextInputManager.onRegisterInputView` publishes a non-empty `KeyboardUiState` (layout + appearance + active mode, via the synchronous `ensureLayoutLoadedNow`) **before** calling `mountKeyboardComposeView`. `mountKeyboardComposeView` runs `setContent` on the already-attached `InputView`, which can create the keyboard body's **first composition synchronously**; that composition must read a populated `KeyboardUiState` so `KeyboardImeRoot` renders a non-empty `KeyboardSurface` at the IME window's first measure. If the publish ran after the mount (or asynchronously, as the retired `launch(Dispatchers.Default) { … }` boot path did), the first composition sees `KeyboardUiState.EMPTY`, the `wrap_content` input view measures to smartbar-only height, and the late async publish does not reliably re-grow the IME window — the intermittent cold-open "keyboard body collapses to a smartbar-only sliver" symptom. `onStartInputView` mirrors the guarantee for the register-before-`onStartInputView` lifecycle order by calling `ensureLayoutLoadedNow(keyboardMode)` before `setActiveKeyboardMode`, so the editor-specific mode (NUMERIC / PHONE / …) is published synchronously too. The first `ensureLayoutLoadedNow` per mode does one bounded synchronous layout compute (small `assets/layouts/*.json` read + Moshi parse, ~KB-scale); warm reopens cache-hit and do no I/O. Aligns with FlorisBoard / khiin-rs, which build the input view synchronously before returning it from `onCreateInputView`.

### Tests

- **JVM unit** — `KeyboardLayoutSolverTest` (P2) pins key dimensions, popup dimensions, and extended-popup geometry math (container-width-based, mode-agnostic). The `KeyTouchCoordinator` touch state machine (single-pointer state, multi-pointer first-cancel, popup-drag boundary, long-press scheduling) has no JVM test today — covered by the real-device dogfood below.
- **Real device dogfood (qualitative, per `feedback_perf_gate`)** — S1/S2/S3 sequences plus explicit Discord/Chrome regression check for the dismiss bug.
- **iOS** — N/A (no iOS mirror; KeyboardKit owns equivalent invariants on the iOS side).

---

## Cross-references

- Parent doc (cross-platform invariants §1–§15): `behavioral-invariants.md`.
- Architectural context for the dismiss-bug class: `memory/project_ime_window_arch.md` (auto-memory, local-only).
- Layout solver pinned tests: `KeyboardLayoutSolverTest` (Android JVM unit).
- AOSP LatinIME parity reference: `PointerTracker.onMoveEvent` (no MOVE-driven press promotion).
