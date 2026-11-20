# Android Guidelines

Project facts for `android/` that the code alone does not make obvious. Cross-platform contract: `cross-platform-alignment.md`; where each concept lives on both phones: its §2.

## 1. Shared-core candidates

Criteria: `cross-platform-alignment.md` §1c. A qualifying file begins with:

```kotlin
// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
```

Logging goes through the `LoggerBackend` interface (`ime/core/logging/LoggerBackend.kt`); `AndroidLoggerBackend` is the only file that touches `android.util.Log` (`LoggerBackendNeutralityTest` pins the import ban). `LoggerBackend` has members `d` / `i` / `w` / `e`, so its lazy extensions take other names (`debug`, `tdebug`): Kotlin resolves a member before a same-name extension, which would make the extension unreachable.

## 2. Naming and tests

- New types follow `ios-architecture.md` §4 (no new `*Manager` / `*Helper`); existing names stay.
- Unit tests: JUnit 4 + `kotlinx-coroutines-test`, mirrored under `app/src/test/` by package, named `component_scenario()` or a backtick sentence. JVM tests cannot load the engine `.so` — use the fakes (`StubEngineSettings`, `StubUserDataClient`); engine round-trips go to `androidTest`.

## 3. DI and lifecycle

- Manual DI: `ime/core/CompositionRoot.kt`; no Hilt.
- Three scopes: app (`TaigiKeyboardApplication`), IME service (`TaigiKeyboard`, whose `serviceScope` is cancelled in `onDestroy`), input view (`onStartInputView` / `onFinishInputView` — reset per-field state there).
- Never keep an Activity context in an `object` or a long-lived class; use `applicationContext`.
- Layering: `ime/` may use the shared look (`ui/theme`, `ui/components`) but never an app tab screen (`ui/tabs`) — something both need moves out first, as the option lists did (`ime/settings/SettingOptions.kt`); `ImeLayeringTest` pins it. A tab screen takes its services as parameters from its Activity, like `prefs`, rather than calling `CompositionRoot.shared` itself.

## 5. IME host facts

- `InputConnection.finishComposingText()` **commits** the composing region. To clear without committing, `setComposingText("", 1)` first (`ComposingDelegate.kt`; `docs/architecture/composing-state-boundary.md` §2.2).
- Deleting committed text: raw `KEYCODE_DEL` only for `TYPE_NULL` editors (terminals, some games). Rich editors — WebView / contenteditable hosts such as Google Chat — silently drop synthetic key events, so everything else uses `deleteSurroundingText` with a `BreakIterator` grapheme length, or `commitText("", 1)` for a selection (`TextInputKeyHandler.kt`).
- `InputConnection` calls run on the main thread. Async candidate work hops back with `withContext(Dispatchers.Main.immediate)` and drops a result whose input-context generation is stale (`CandidateUpdateCoordinator.kt`); cancel the old `Job` before launching its replacement.
- The IME window is `MATCH_PARENT × WRAP_CONTENT`; the keyboard body takes Compose `WindowInsets`, and the only custom inset is the 0.9× navigation-bar padding on `media_input` (`ime/core/InputView.kt`, `INVARIANT_keyboard_navbar_inset_padding_factor`). Reference: `references/florisboard`, `references/aiongtaigi-sushi`.
- FlorisBoard-derived code (keyboard views, layout JSON loaders) is platform code; do not purify it.

## 6. Settings

- `PrefHelper` implements `EngineSettings` and re-reads its cached DataStore preferences on every `get()`. Never copy a setting into a field at construction — later changes would be invisible (`EngineSettingsLiveReadTest`).
- App → IME changes travel through DataStore `Flow`s collected in `TaigiKeyboard.onCreate`, not broadcasts.
- SharedPreferences is read once by `PrefHelper.migrateFromSharedPreferences`; nothing new writes it.

## 8a. User-data SQLite is the engine's

The four user-data stores (`user_frequency.db`, `user_association.db`, `custom_dictionary.db`, `learned_phrases.db`) are opened, migrated and written by the engine's bundled SQLite (`engine/userdata`); Android reaches them only through `engine/UserDataBridge.kt` / `ime/dictionary/UserDataClient.kt`, opened in `TaigiKeyboardApplication.onCreate`. Never open them with `android.database.sqlite` or any other SQLite: two SQLite copies in one process lock independently and can corrupt a file both hold (`docs/architecture/user-data-engine-roadmap.md` U2 / U6). A new store or query belongs in the engine. The app runs no SQL of its own, so `minSdk`'s OS SQLite does not constrain these stores.

## 9. Gradle

Android Gradle files are editable (Xcode project files are not). After an edit, say what changed — the maintainer needs a Gradle sync.
