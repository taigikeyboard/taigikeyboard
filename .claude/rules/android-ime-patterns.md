---
paths: ["android/**/*.kt"]
---

# Android IME / UI / Testing / Refactor Patterns

Compose + IME-specific patterns + testing + refactor-round checklist. Split out from `.claude/rules/android-guidelines.md` for focus. Core architecture / Kotlin idioms / DI / DataStore stay in the parent file.

## 1. Compose patterns `[B]`

- `remember { … }` for UI-only state that survives recomposition but not configuration change.
- `rememberSaveable { … }` when the state should survive process death (e.g. user input in a form).
- `collectAsStateWithLifecycle(initial)` — preferred over `collectAsState` for Flow observation in Compose. Pauses collection when the lifecycle stops, avoiding unnecessary recomposition.
- State hoisting — stateless `@Composable` functions take state + callbacks as parameters. Stateful versions delegate to the stateless version.
- `derivedStateOf { … }` for computed state that depends on multiple observable sources.
- Side-effect APIs choose by scope:
  - `LaunchedEffect(key)` — coroutine bound to composition, restarts on key change.
  - `DisposableEffect(key)` — setup + teardown pair (listeners, subscriptions).
  - `SideEffect` — synchronous side effect on every recomposition.
- ViewModel owns async; View invokes ViewModel methods. No `LaunchedEffect { viewModelScope.launch { … } }` inside a Compose function.

## 2. IME-specific rules `[B]` `[A]`

- `InputConnection.finishComposingText()` **commits the composing region by default.** To honor "clear without commit" semantics, call `setComposingText("", 1)` **before** `finishComposingText()`. Documented in `docs/architecture/ios-exemplar.md` §4.1 and `docs/architecture/composing-state-boundary.md` §2.2 (Android mapping addendum is A4-design deliverable).
- **Committed-text deletion/editing uses the semantic `InputConnection` API, NOT synthetic raw key events.** Raw `KeyEvent` / `sendKeyEvent` / `sendDownUpKeyEvents(KEYCODE_DEL)` is only for **`TYPE_NULL`** editors (terminals, some games). Rich editors (`TYPE_CLASS_TEXT` — including Gmail Google Chat and other WebView/contenteditable hosts) **silently drop** a synthetic `KEYCODE_DEL` — deletes nothing while working in native `EditText` (which acts on key-down). Dispatch by capability: `(inputType and InputType.TYPE_MASK_CLASS) == TYPE_NULL` → raw key; else → `deleteSurroundingText` (grapheme length via `BreakIterator.getCharacterInstance()` to keep emoji/ZWJ/combining whole) / `commitText("", 1)` for a selection. `deleteSurroundingTextInCodePoints` returns `false` on editors that don't implement it (pre-Android-13) → fall back to `deleteSurroundingText`. Incident: PR #446 (Gmail Chat backspace), `memory/project_android_rich_editor_backspace.md`. iOS immune (semantic `textDocumentProxy.deleteBackward()` throughout).
- All `InputConnection` calls on `Dispatchers.Main.immediate` — Android IME requires main-thread for InputConnection.
- `InputMethodService.onStartInput` / `onFinishInput` bracket per-input-session state — reset composing context here, not in `onCreate` / `onDestroy`.
- `LifecycleInputMethodService` provides a `Lifecycle` + `ViewModelStore`. Scoped ViewModels inside the IME use the service's `ViewModelStoreOwner`, not a plain `androidx.lifecycle.ViewModel()` (which has no owner).
- FlorisBoard-derived code (keyboard view hierarchy, layout JSON loaders) is **platform**, not shared-core. Do not try to purify it.

## 3. Testing `[B]` `[A]`

Cross-platform test naming + assertion conventions follow `.claude/rules/ios-guidelines.md` "Test Conventions"; Android-specific additions only here.

- JUnit 4 — project default (see existing `app/src/test/java/.../ime/dictionary/*Test.kt`). Do not mix JUnit 5.
- Turbine (`app.cash.turbine`) for Flow assertions — pattern `flow.test { … }`.
- `kotlinx-coroutines-test` — `runTest { … }` block with injectable `TestDispatcher` for time-controlled tests.
- `INVARIANT_*` function-name prefix for cross-platform-invariant tests; labels match `docs/architecture/behavioral-invariants.md` (policy: `.claude/rules/cross-platform-alignment.md` §3a).
- Tests must be runnable via `./gradlew test` (wired into `testImplementation` in the test source set).

## 4. Refactor-round checklist `[A]`

Durable checklist for every Android refactor PR:

- [ ] Refactor-freeze observed per `.claude/rules/cross-platform-alignment.md` §1. If the PR intentionally changes behavior, it uses the emergency tier (§1a) or parity-correction tier (§1b) and labels accordingly.
- [ ] Codex + `/simplify` pre-review on plan before implementation (per `~/.claude/rules/code-review-rules.md` §8). `/simplify` is the Claude Code official skill — run in parallel with Codex to catch reuse / quality / dead-code issues Codex does not flag.
- [ ] Codex post-review on diff before merge.
- [ ] Qualitative dogfooding pass (S1 / S2 / S3 sequences) on a real Android device for any hot-path round (concrete Taigi sequences in `.claude/rules/taigi-incidents.md` § Qualitative perf gate).
- [ ] Invariant tests stay green.
- [ ] `// CROSS-PLATFORM INVARIANT` comments updated if constants moved (policy in `.claude/rules/cross-platform-alignment.md` §3a).
- [ ] No new `android.util.Log` / `GlobalScope` / `!!` / `object`-with-state introduced.
- [ ] PR touching a shared-core-candidate file honors `.claude/rules/cross-platform-alignment.md` §1c.

## 5. References

- `.claude/rules/android-guidelines.md` — parent file: shared-core criteria, Kotlin idioms, lifecycle/DI, DataStore, null/error handling, Gradle
- `.claude/rules/cross-platform-alignment.md` — refactor-freeze contract + emergency / parity tiers + §3a invariant policy
- `docs/architecture/ios-exemplar.md` — alignment target
- `docs/architecture/composing-state-boundary.md` — composing/finishComposingText contract
- `docs/architecture/behavioral-invariants.md` — `INVARIANT_*` labels
- `~/.claude/rules/code-review-rules.md` — review checklist + §8 review-before-impl + §9 perf gate
- `docs/architecture/dogfood-checklist.md` — concrete Taigi dogfood sequences (S1–S28); `.claude/rules/taigi-incidents.md` — PR #227 refactor-freeze incident
