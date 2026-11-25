---
name: layout
description: Run the layout test (版面測試) — type real corpus sentences word by word on the staged web page, the Android emulator, the iOS simulator (iPhone + iPad), the Windows box, the Linux VMs (IBus + Fcitx5), the browser extension on its fixture sites or, when named, macOS (ChromeOS is out of scope: the system draws its candidate window); check the candidate window / strip, expanded grid, symbol picker and key faces after every word across each platform's matrix (viewports, devices, orientations, candidate layouts, sizes, fonts, scales), screenshot each failure, and publish the findings. Use when asked to "run the layout test", 「跑 <平台> 的版面測試」, 「版面測試」, or to check that candidates and key labels never overlap, clip or overflow. Args: a platform (`web` `android` `ios` `windows` `linux` `macos` `extension`) and optionally its matrix filters. Reports findings; never opens a fix round itself.
disable-model-invocation: false
---

# Layout test

Born from the 2026-11-24 cross-platform layout audit. Every platform is covered:

| Platform | How | Section |
|---|---|---|
| Web | automated harness `web/layout-sim/` | §1–§4 |
| Android | agent-driven emulator + layout instrumented tests | §5 |
| iOS | agent-driven simulator (iPhone + iPad) + layout unit tests | §6 |
| Windows | agent-driven run on the maintainer's Windows box + `taigi-windows-tsf` tests | §7 |
| Linux | agent-driven run on the Linux VMs (IBus + Fcitx5) + popup tests | §8 |
| macOS | in-process render of the real candidate window from a temporary test — **only when the maintainer names macOS** | §9 |
| Browser extension | agent-driven Playwright run of the staged extension on the extension fixtures | §10 |

ChromeOS is out of scope: `chrome.input.ime.setCandidates` hands the candidate window to the system (vertical or horizontal only, `docs/architecture/chromeos-roadmap.md` D4), and its options page is cut from the web page's settings form (`web/scripts/stage-chromeos.mjs`), so it has no layout of its own.

Every platform uses the §1 sentences, the §3 verdict rules and the §4 report. Accessibility text sizes (Web `ROOT_TEXT_SIZE`, iOS accessibility sizes, Windows Text size, GNOME text scale 2.0) run only when the maintainer names them. **One platform at a time** (maintainer 2026-11-25): finish a platform — its run, its report, its restore — before starting the next, and never start a simulator, emulator, VM or Playwright run beside another; the Mac's load spikes and the emulator ANRs. Within a platform, run its tests and its screenshot pass one after the other too.

## 1. Run

```sh
make -C web layout-sim                                     # 3 browsers × every view × layout × size
BROWSERS=chromium VIEWS=phone LAYOUTS="vertical horizontal" make -C web layout-sim
```

`make site` runs first (the page is staged into `web/target/site/`). A fresh worktree needs `git submodule update --init corpus/taigi-typing` first. A full run takes about six minutes.

| Variable | Values (default: all) |
|---|---|
| `BROWSERS` | `chromium` `firefox` `webkit` |
| `VIEWS` | `desktop` (1280×800), `phone` (360×800), `zoom200` (180×400 = the phone at 200 % page zoom; size 17 only, 7 sentences) |
| `LAYOUTS` | `vertical` `horizontal` `expandable` `verticalExpandable` |
| `ROOT_TEXT_SIZE` | unset. A browser text-size setting (`150%`, `200%`) — **accessibility text sizes run only when the maintainer names them** |

Candidate text sizes are 13 / 17 / 23. Sentences: a fixed pick from `corpus/taigi-typing/exampleSentences.js` (every 600th plus the longest), read at run time — the corpus licence allows quoting, not rewriting, so never copy them into the repo. Each word is typed toneless; the second word of a clause browses the candidates with the arrows; each clause ends with Space until the composition empties, each sentence with Enter.

The port is 4175. "already used" → another run's server; wait for it, never kill a process you did not start. Never start `make -C web e2e` during a run: it deletes `test-results/`, this run's output with it.

## 2. Read the output

`web/test-results/layout-sim/` (gitignored; cleaned at the start of every run and by `make -C web e2e`):

- `summary.json` — browser → `"<view> <layout> <size>"` → issue kind → count; `{}` = clean; `incomplete: 1` = a step threw before the last sentence (Playwright's `layout-*/error-context.md` says where). The run also prints every combination with issues.
- `report-<browser>-<view>-<layout>-<size>.json` — every issue with the sentence / clause index and the word typed (`<word>+browse` = after the arrows).
- `FAIL-<browser>-<combination>-<kind>.png` — the first occurrence of each kind; `ok-<combination>-s<n>.png` — up to three Chromium shots per combination of clauses without issues.

A combination with any issue fails its test, so `make` exits non-zero; read `summary.json` either way.

| Kind | Means |
|---|---|
| `panel-covers-composition` | the candidate window overlaps the composition text |
| `panel-past-editor-bottom` / `panel-past-viewport` | the window extends past the editor's bottom / the viewport's sides |
| `page-scrolls-sideways` / `list-scrolls-sideways` / `candidates-scroll-sideways` | horizontal overflow of the page / list / candidate row |
| `arrows-past-panel` | the page arrows extend past the window |
| `row-taller-than-box` | a row's content is taller than the row |
| `candidate-cut` | a candidate that would fit in its row, were the row alone in the list with the romanization giving way, is still cut |
| `script-past-cell` / `script-past-row-height` | a candidate or its romanization extends past its cell / its row |

## 3. Real defect or design?

Open the `FAIL-*.png` for every kind before calling it either. Design behavior, not defects:

- A candidate wider than the whole list less the row's index and padding, cut with "…" — the check already skips those (at `zoom200` a five-Hanji word does not fit a `horizontal` / `expandable` cell, nor at size 17 the `verticalExpandable` column, 83 px for 85). The romanization is truncated with "…" by design (the candidate stays whole, `web/e2e/look.spec.mjs`) and is never checked for cuts.
- A kind in one browser only, once, that a rerun of that combination (`BROWSERS=<b> VIEWS=<v> LAYOUTS=<l>`) does not repeat — a timing flake; report it as such.

Everything else is a candidate defect. Trace it to the CSS / layout code (`web/src/style.css`, `web/crates/taigi-web/src/layout.rs`, `desktop/crates/taigi-desktop-core/src/candidates/expandable.rs`) before naming a cause, and say whether the same kind also shows in the other browsers.

## 4. Report, do not fix

Publish an Artifact page: per browser a table of combinations with issues (kind × count), each issue kind with its `FAIL-*.png` beside the words that triggered it and your verdict (defect + traced `file:line` / design / flake), and two or three `ok-*.png` for comparison. Give the maintainer the link, plus one line per candidate defect in the reply.

A candidate defect is an observed failure: a fix is its own round, opened by the maintainer (`AGENTS.md` Core Principle #4).

## 5. Android

No harness script: you drive one emulator with `adb`, read every screenshot, and run the layout instrumented tests per device. Same sentences, same verdict rules (§3) and same report (§4) as the web.

### 5.1 Safety

- A real phone may be connected over adb. Address the emulator only: `adb -s emulator-5554 …` for every command, `ANDROID_SERIAL=emulator-5554` for every Gradle run (`connectedDebugAndroidTest` otherwise installs on every device).
- One emulator at a time; never alongside a running iOS simulator session (the Mac's load spikes, the emulator ANRs).
- The run changes the emulator, not the repo: screenshots go to the scratchpad. Restore what you changed at the end: `wm size reset`, `wm density reset`, `settings put system font_scale 1.0`, `settings put system user_rotation 0`.

### 5.2 Setup

1. Build and install the debug app (`android/gradlew -p android :app:assembleDebug`, then `adb install -r`); the stale-artifact gate in `AGENTS.md` applies. Debug only: settings are written with `run-as`.
2. One AVD with an `android-36` `google_apis` image (`sdkmanager`, `avdmanager create avd`); boot it headless: `emulator -avd <name> -no-window -no-audio -gpu swiftshader_indirect -no-snapshot-save`. Every device in the matrix is that AVD resized (§5.3).
3. Settings for a combination, without the UI: `am force-stop` + `pm clear com.siansiansu.taigikeyboard`, write a DataStore preferences file — `PreferenceMap { 1: map<string, Value> }`, each entry `{1: key, 2: Value}`, `Value` field `1` bool, `2` float, `3` int, `5` string — to `files/datastore/taigi_keyboard_prefs.preferences_pb` through `run-as … sh -c 'cat /data/local/tmp/p.pb > …'`, then `ime enable` + `ime set com.siansiansu.taigikeyboard/.ime.core.TaigiKeyboard`. Keys: `taigi__font_type`, `keyboard__input_mode` (`tl`), `keyboard__layout_type`, `app__display_language` (`en` for the longest tab labels), `keyboard__candidate_display_mode` (`sideBySide`). Add `internal__launcher_alias_restored` = true so first-run work does not interfere.
4. Host field: `am start -a android.intent.action.INSERT -t vnd.android.cursor.dir/contact`, tap the first `EditText` (`uiautomator dump` gives its bounds). `uiautomator` does not see the keyboard window: everything about the keyboard comes from `adb exec-out screencap -p` (crop with `ffmpeg`).
5. Typing: `input text` bypasses the IME, so tap the keys. Find each layout's key rows once per device × orientation from a screenshot and tap the key centres. TL on `phahTaigi` has no `q w x z`, which toneless TL never needs.

### 5.3 Matrix

| Axis | Values |
|---|---|
| Device (`wm size` / `wm density`) | small phone `720x1280` / 320 (360 dp) · phone `1080x1920` / 420 · fold unfolded `2208x1840` / 420 · 7″ tablet `1200x1920` / 320 · 10″ tablet `1600x2560` / 320 |
| Orientation | portrait, landscape (`settings put system accelerometer_rotation 0`, `user_rotation 0` / `1`) |
| Font (`taigi__font_type`) | `system` · `openHuninn` · `iansui` · `genYoMin` · `genYoGothic` |
| Font scale (`settings put system font_scale`) | `1.0` · `2.0` |
| Keyboard layout (`keyboard__layout_type`, key faces only) | `phahTaigi` · `moe1` · `moe2` · `tps` |

100 combinations of device × orientation × font × font scale; the maintainer may name filters (`android phone,fold landscape system`). Display size large (density raised one step) and other font scales run only when named. Changing density or font scale restarts the keyboard: re-tap the field.

### 5.4 Per combination

- **Typing** (layout `phahTaigi`): the §1 sentences, each word toneless. After each word, screenshot the candidate strip. Once per clause, open the expanded grid (the strip's expand button), screenshot it, scroll it to the end and screenshot again, collapse, tap the first candidate. Clear the field after each sentence.
- **Symbol picker**: open it once from the toolbar (`+`, then the symbol button), screenshot the tabs, scroll the grid to the end, screenshot.
- **Key faces**: one screenshot of the keyboard per keyboard layout (plus the number row, where tone marks sit).
- **Layers**: on `phahTaigi`, one screenshot each of Shift, `SYMBOLS`, `SYMBOLS2` (`ime/text/keyboard/KeyboardMode.kt`), and the number / phone pads from the contact form's Phone field.
- **Long-press popups**: per keyboard layout, hold the key with the longest popup list (up to 9 entries, `CustomLayout.MAX_CALLOUTS`), the leftmost and the rightmost key that have one, and a top-row key, and screenshot while held: `adb -s emulator-5554 shell input swipe x y x y 3000 &`, `screencap` about 1.5 s later. Popup code: `ime/popup/` (`KeyPopupManager.kt`, `KeyPopupBox.kt`).
- **Custom layout**: create one in the app's Layout editor (driven with taps or Maestro) that packs the longest keys the editor offers (multi-letter finals such as `ng`, `nn`) into one row and a 9-entry popup on an edge key; screenshot its key faces and that popup. Up to 5 custom layouts exist (`CustomLayout.MAX_CUSTOM_LAYOUTS`); delete the test layout at the end.
- **Fullscreen**: in landscape, confirm the app's own field stays visible above the keyboard (no extract editor).

### 5.5 Instrumented tests

Once per device × orientation × font scale (they render with their own font, so not per font):

```sh
ANDROID_SERIAL=emulator-5554 android/gradlew -p android :app:connectedDebugAndroidTest \
  -Pandroid.testInstrumentationRunnerArguments.class=com.siansiansu.taigikeyboard.ime.text.candidates.CandidateLineStyleTest,com.siansiansu.taigikeyboard.ime.text.overlays.CandidateOverlayLayoutTest,com.siansiansu.taigikeyboard.ime.text.overlays.SymbolOverlayLayoutTest,com.siansiansu.taigikeyboard.ime.core.TaigiKeyboardFullscreenTest
```

Never run the whole `connectedDebugAndroidTest`: `RustEngineBridgeTest` needs the dev `.so` with the panic injector. Results: `android/app/build/outputs/androidTest-results/connected/debug/TEST-*.xml`.

### 5.6 What to check in each screenshot

| Kind | Means |
|---|---|
| `strip-line-cut` | a strip candidate's title or romanization is cut by the strip (System font CJK fallback is the usual trigger) |
| `grid-row-under-navbar` / `symbol-row-under-navbar` | scrolled to the end, the last row sits in the navigation-bar band |
| `grid-cell-cut` | a grid candidate is ellipsized though it is narrower than the whole row |
| `grid-cells-misaligned` | the lines of cells in one row do not share a baseline |
| `grid-divider-short` | a row divider stops short of the row's last cell |
| `grid-controls-squeezed` | a control-panel button (collapse, page, 文/A) is smaller than its neighbours or loses its icon |
| `symbol-tab-wrapped` | a tab label breaks onto a second line or is cut |
| `key-label-near-edge` | a key label's ink is closer to a side than 6 % of the key height (one glyph) or 12 % (several characters) |
| `key-hint-near-top` / `key-hint-touches-label` | a hint's ink is within 12 % of the key height of the key top, or touches the label below |
| `key-label-off-centre` | a label's ink is visibly off the key centre (full-width ？，。 are centred by ink) |
| `popup-past-screen` | a long-press popup extends past a screen side or the top of the keyboard window |
| `popup-entry-cut` / `popup-entries-overlap` | a popup entry is cut by its cell, or two entries touch |
| `popup-off-key` | the popup is not above (or beside, at an edge) the key held |
| `fullscreen-extract` | landscape shows the framework's extract editor over the app |

The thresholds are the ones `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/keyboard/KeyLabelGeometry.kt` draws to; measure in pixels from the crop when a case is close. Design, not defects: a candidate wider than the whole row ellipsized; tofu for a character no bundled font has (a font coverage report, not layout).

### 5.7 Report

As §4, with the table per device × orientation instead of per browser (rows: font × font scale), the failed instrumented tests listed with their message, and each defect traced to its file under `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/` (`candidates/CandidateStrip.kt`, `overlays/`, `keyboard/KeyContent.kt`).

## 6. iOS

No harness script and no UI-test target: you drive the simulator, read every screenshot, and run the layout unit tests per device.

### 6.1 Safety

- Simulators only; a real iPhone / iPad is out of scope. Never alongside a running Android emulator session (§ intro).
- Screenshots go to the scratchpad. Restore what you changed at the end: `xcrun simctl ui <udid> content_size large`, portrait, the keyboard settings back to their defaults (§6.2).

### 6.2 Setup

1. Build and install: `xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboard -destination id=<udid> -derivedDataPath <scratchpad>/ios-dd build`, then `xcrun simctl install <udid> <app>`; the stale-artifact gate in `AGENTS.md` applies.
2. Enable the keyboard on every new simulator: Settings → General → Keyboard → Keyboards → Add New Keyboard → TaigiKeyboard, then Allow Full Access — without Full Access the extension cannot read the App Group settings. Drive Settings and the app with Maestro (`maestro --device <udid> test f.yaml`, `maestro hierarchy` for bounds).
3. Settings live in the App Group suite `group.com.siansiansu.TaigiKeyboard` (`ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift`): `fontType` (`system` `openHuninn` `iansui` `genYoMin` `genYoGothic`), `keyboardLayoutType` (`phahTaigi` `qwerty` `moe1` `moe2` `tps`), `inputMode` (`tl`), `candidateTextSizeScale` / `keyFontSizeScale` (Double, 1.0), `candidateDisplayMode` (`sideBySide`), `displayLanguage` (`en` for the longest labels), `oneHandedMode`. Set them through the app's own Settings tab: `simctl spawn … defaults write` writes a file the extension does not read. Writing the group-container plist directly is unverified — spike it on one key (≤20 lines, relaunch the keyboard, screenshot) before relying on it.
4. Host field: a tiny SwiftUI app with a `TextField` focused from `.task` (built with `swiftc -target arm64-apple-ios17.0-simulator -parse-as-library`, an `Info.plist` with `UIApplicationSceneManifest`, `codesign -s -`) brings the keyboard up without taps; any app with a text field works. After a reinstall a stale extension process can show the old layout: relaunch the host before trusting a shot.
5. Typing: tap the key centres (`maestro` `tapOn: point:`), found once per device × orientation × layout from a screenshot; the extension's keys are not in the hierarchy. Screenshot with `xcrun simctl io <udid> screenshot`.

### 6.3 Matrix

| Axis | Values |
|---|---|
| Device | iPhone SE · iPhone 17 · iPhone 17 Pro Max · iPad (11″) · iPad Pro 13″ (create missing ones with `xcrun simctl create`; names from `xcrun simctl list devicetypes`) |
| Orientation | portrait, landscape (Maestro `setOrientation`) |
| Font (`fontType`) | `system` · `openHuninn` · `iansui` · `genYoMin` · `genYoGothic` |
| System text size (`xcrun simctl ui <udid> content_size`) | `large` · `extra-extra-extra-large` |
| Keyboard layout (`keyboardLayoutType`, key faces only) | `phahTaigi` · `qwerty` (POJ) · `moe1` · `moe2` · `tps` |

100 combinations of device × orientation × font × text size; the maintainer may name filters (`ios se,ipad landscape system`). Candidate / key size scales other than 1.0 and one-handed mode run only when named.

### 6.4 Per combination

As §5.4: the sentences on `phahTaigi` with a candidate-bar screenshot after each word; once per clause the expanded candidate grid (top and scrolled to the end); the symbol picker once (tabs, grid end); one key-face screenshot per keyboard layout plus the number row. Layers: Shift and the `123` / `#+=` pages (`ios/Sources/TaigiKeyboard/Layout/TaigiLayouts.swift`). Long-press popups: the same keys as §5.4 — Maestro `swipe` with start = end for 12 s, screenshot about 9 s in; popup maps `ios/Sources/TaigiKeyboard/Callouts/`. Custom layout: as §5.4, in the app's Layout tab (`App/Tabs/Layout/LayoutEditorView.swift`, limit `CustomLayout.maxCallouts` = 9); delete it at the end.

### 6.5 Unit tests

Once per run (they render with fixed sizes, not per device):

```sh
xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboardTests -destination 'platform=iOS Simulator,id=<udid>' test \
  -only-testing:TaigiKeyboardTests/ExpandedCandidateRowLayoutTests -only-testing:TaigiKeyboardTests/CandidateButtonViewLayoutTests \
  -only-testing:TaigiKeyboardTests/KeyboardFontsTests -only-testing:TaigiKeyboardTests/ButtonFontProviderTests \
  -only-testing:TaigiKeyboardTests/KeyLabelLayoutTests -only-testing:TaigiKeyboardTests/KeyHintStackRenderTests \
  -only-testing:TaigiKeyboardTests/OneHandedModeTests
```

### 6.6 What to check in each screenshot

| Kind | Means |
|---|---|
| `bar-candidate-cut` | a bar candidate's Hanji or romanization is cut by the bar, or its highlight crosses the bar edge |
| `bar-row-off-centre` | the candidate row (text, +, chevron) is visibly off the bar's vertical centre |
| `grid-row-past-screen` | an expanded-grid row or the control column extends past the screen side |
| `grid-cell-cut` / `grid-row-under-home-indicator` | as §5.6 `grid-cell-cut` / `grid-row-under-navbar`, with the home-indicator band |
| `symbol-tab-wrapped` | as §5.6 |
| `key-label-near-edge` / `key-hint-near-top` / `key-hint-touches-label` / `key-label-off-centre` | as §5.6 |
| `key-text-follows-system-size` | a key label or candidate grows with the system text size (every font is fixed-size) |
| `popup-past-screen` / `popup-entry-cut` / `popup-entries-overlap` / `popup-off-key` | as §5.6 |

Owning files, under `ios/Sources/TaigiKeyboard/`: `Candidates/Views/CandidateView.swift`, `Candidates/Views/CandidateButtonView.swift`, `Overlays/ExpandedCandidateOverlay.swift`, `Overlays/ExpandedCandidateRowLayout.swift`, `Overlays/SymbolSelectionOverlay.swift`, `Styling/KeyLabelLayout.swift`, `Styling/KeyHintStack.swift`, `Styling/KeyboardFonts.swift`.

### 6.7 Report

As §4, a table per device × orientation (rows: font × text size), failed unit tests with their message.

## 7. Windows

The candidate window is a TSF window drawn by the DLL, so the run happens on the maintainer's Windows box (`ssh win`); the Mac only orchestrates. The box is a test device: its user data may be cleared and the dev registration may stay.

### 7.1 Safety

- Input must come from the logged-in session, never session 0 (ssh): run the driver as a scheduled task (`schtasks /Create … /IT`, or `Register-ScheduledTask` with an `Interactive` principal) under **pythonw.exe** (python.exe opens a console that steals focus).
- The driver clicks its own host window and aborts when the foreground window is not its own, **before every key** — keys once landed in the maintainer's own Notepad tab. Never type into a window you did not open.
- Binary gate: a host that mapped the old DLL never sees a rebuild. After an install, test only in a host started after it; check with `Get-Process | Where-Object { $_.Modules.ModuleName -contains 'TaigiKeyboard.dll' }`.
- Scripts sent over ssh are ASCII-only `.ps1` / `.py` files run by path; `ssh win "…"` returns 0 even when the command failed — read the output.

### 7.2 Setup

1. On the box's main clone (check its branch first): `git pull`, then `powershell -NoProfile -ExecutionPolicy Bypass -File windows\scripts\install-dev.ps1 install` (cold build 5–8 min; `reload` when only the DLL changed and `dictionary/` did not).
2. Settings: `%APPDATA%\TaigiKeyboard\settings.json`, live-reloaded. Merge the keys, raise top-level `revision` by one, replace the file atomically; pass the changes in a file, never quoted over ssh. Keys (`desktop/crates/taigi-desktop-core/src/settings/keys.rs`, `choices.rs`): `candidateLayout` (`vertical` `horizontal` `expandable` `verticalExpandable`), `candidateTextSize` (`"13"` `"15"` `"17"` `"20"` `"23"`), `fontType` (`system` `openHuninn` `iansui` `genYoMin` `genYoGothic`), `candidateDisplayMode` (`sideBySide`), `inputMode` (`tl`).
3. Driver (Python on the box): DPI-aware (`SetProcessDpiAwareness(2)`, else `GetWindowRect` and the grab miss), `SendInput` with VK + `MapVirtualKeyW` scan codes, Pillow `ImageGrab`. Activate the TIP with `ITfInputProcessorProfileMgr::ActivateProfile` — `TF_IPPMF_FORSESSION` for another process's window. Find the candidate window by class `TaigiKeyboardCandidates` (`windows/crates/taigi-windows-tsf/src/ui/presenter.rs`) **and** `GetWindowThreadProcessId` = the host's pid: every TIP host owns one.
4. Hosts: a WPF `TextBox` (a TSF host) and a WinForms `RichTextBox` (IMM32 via CUAS) — both a few lines of PowerShell. Tk is an IMM32 host whose output is not TSF's: never judge there.

### 7.3 Matrix

| Axis | Values |
|---|---|
| Host | WPF `TextBox` · WinForms `RichTextBox` |
| Candidate layout | `vertical` · `horizontal` · `expandable` (folded and opened) · `verticalExpandable` (folded and opened) |
| Size (`candidateTextSize`) | `13` · `17` · `23` |
| Font (`fontType`) | `system` · `openHuninn` · `iansui` · `genYoMin` · `genYoGothic` |
| Caret position | middle · bottom (window flips above) · right edge · bottom-right |

**Screen axes, only when named** (not yet run on the box — spike each once before relying on it): display scale `150%` · `200%` (Settings → System → Display → Scale, then sign out and in, then the §7.1 binary gate); a second monitor at another scale (the box has one built-in panel, so the maintainer plugs one in) — caret on each monitor, and the host window dragged across (`WM_DPICHANGED`, `windows/crates/taigi-windows-tsf/src/ui/window.rs`). Extra kinds: `panel-wrong-scale` (text drawn at the other monitor's scale), `panel-on-wrong-monitor` (the window is not on the caret's monitor).

### 7.4 Per combination and tests

The sentences, a window capture after each word plus a capture of the host around the caret (the window against the composition); a clause long enough to wrap the preedit near the bottom. Unit tests once per run, on the box: `cargo test -p taigi-windows-tsf` (in no CI gate — this run is their check).

| Kind | Means |
|---|---|
| `panel-covers-composition` | the window covers any line of the composition, wrapped lines included |
| `panel-past-work-area` / `panel-at-screen-edge` | the window extends past the work area, or sits closer than 12 DIP to a screen side |
| `candidate-cut` / `script-past-cell` / `row-taller-than-box` | as §2 |
| `poj-dot-touches-tone` | POJ o͘'s dot collides with a tone mark |

Known, not a new finding: a WinForms `RichTextBox` answers every range with the caret line, so a wrapped preedit there can still be covered (host limitation); the System face's row height ratio (WIN4, deferred by the maintainer 2026-11-25). Owning files: `windows/crates/taigi-windows-tsf/src/ui/caret.rs`, `candidate_window.rs`, plus `desktop/crates/taigi-desktop-core/src/candidates/`. Report as §4, a table per host × layout (rows: font × size × caret position).

## 8. Linux

The candidate window is `taigikeyboard-popup` (ours, X11 / XWayland) for both IBus and Fcitx5. The run happens on the Linux VMs.

### 8.1 Safety

- VMs only. `utm-linux` (Ubuntu GNOME, Wayland, IBus) on this Mac; the VirtualBox KDE VM (X11, Fcitx5) on the Windows box, `ssh -J win -p 2222 taigi@127.0.0.1` — it does not boot while the box's Memory Integrity is on (maintainer's decision to turn it off).
- Never `fcitx5 --replace` on the GNOME VM: it kills ibus-daemon and GNOME resets the input sources. Fcitx5 beside IBus = `fcitx5 -d --disable=ibusfrontend,xim,waylandim,kimpanel` and the app started with `GDK_BACKEND=x11 GTK_IM_MODULE=fcitx` + `XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.*`.
- `pkill -f <pattern>` over ssh matches the ssh shell itself: `pkill -f "[t]aigikeyboard-popup"`.
- Restore at the end: `gsettings set org.gnome.desktop.interface text-scaling-factor 1.0`, the settings file as found.

### 8.2 Setup

1. On the VM's checkout: build as the user, then `sudo env PATH=$PATH HOME=$HOME RUSTUP_HOME=$HOME/.rustup CARGO_HOME=$HOME/.cargo make -C linux install`, then `ibus restart` (GNOME then routes keys nowhere until **Super+Space twice**) or `fcitx5 -r`.
2. Settings: `~/.config/taigikeyboard/settings.json` (same merge + `revision` rule as §7.2). Linux sizes the window in px, not `candidateTextSize`: `hanjiFontSize` / `romanFontSize` (12–48), typefaces `hanjiFontFamily` / `romanFontFamily` (fontconfig family, empty = system sans); `candidateLayout`, `candidateDisplayMode`, `inputMode` as §7.2.
3. Typing and shots: the e2e drivers (`e2e/drivers/linux/desktop.py`, uinput) or the VM's `~/kb.py` + `~/vt.sh` (`t:` text, `k:` chord, `s:<name>` → `gnome-screenshot`); read committed text with `zenity --entry`. After a text-scale change restart the popup (`pkill -f "[t]aigikeyboard-popup"`).

### 8.3 Matrix

| Axis | Values |
|---|---|
| Frontend | IBus (GNOME VM) · Fcitx5 (KDE VM, or beside IBus on the GNOME VM, X11 client) |
| Candidate layout | `vertical` · `horizontal` · `expandable` · `verticalExpandable` |
| Size (`hanjiFontSize` = `romanFontSize`) | `13` · `23` · `48` |
| Typeface (`hanjiFontFamily`) | empty (system sans) · each bundled family `fc-list` shows |
| Text scale (`text-scaling-factor`) | `1.0` (`2.0` only when named — accessibility range) |
| Caret position | middle · bottom · right edge |

Plus the settings window once (at text scale 2.0 when named: no page wider than the screen, headings ellipsized).

**Screen axes, only when named** (spike each once first): GNOME display scale `200%` (Settings → Displays → Scale; the popup reads it through `scale_factor()`, `linux/crates/taigikeyboard-popup/src/main.rs` `scale`); a second display (UTM or VirtualBox adds a virtual one) — caret on each; the popup fits to the caret's monitor ∩ work area. Extra kinds as §7.3 (`panel-wrong-scale`, `panel-on-wrong-monitor`). Known, not a new finding: the screen width the popup reports to the session lags one list after a monitor switch.

### 8.4 Tests and kinds

Once per run, on the Mac: `cargo test -p taigikeyboard-popup -p taigi-linux-core`, `cargo test -p taigikeyboard-settings --test panes`. Check kinds: §7.4's, with `panel-past-work-area` = past monitor ∩ `_NET_WORKAREA` (GNOME top bar and dock). Design, not defects: an over-wide cell ending in "…"; a vertical list scrolling around the cursor when taller than the screen. Known, not a new finding: Fcitx5 on GNOME Wayland (GTK native client) gets a window-relative rect and can cover the field — documented in `linux/README.md`. Owning files: `linux/crates/taigikeyboard-popup/src/main.rs` (placement, `Fit`), `layout.rs`. Report as §4, a table per frontend × layout (rows: size × typeface × caret position).

## 9. macOS

Only when the maintainer names macOS. No IME install: installing the dev build replaces the input method the maintainer is typing with. Instead render the real candidate window in-process, the way `macos/Tests/TaigiInputMethodCoreTests/ManualScreenshotTests.swift` does (real controller, real engine, a stand-in document window, `screencapture` of its region).

### 9.1 Run

1. Copy `ManualScreenshotTests.swift` to a temporary `macos/Tests/TaigiInputMethodCoreTests/LayoutSimTests.swift` (never committed; delete it at the end), class `LayoutSimTests`, env `TAIGI_LAYOUT_SHOTS`. Replace its scenes with the matrix below; each scene types one §1 sentence word by word through `controller.handle(TestFixtures.keyDownEvent…)`, `syncDocument()` after every key, captures after every word, and presses the arrows once per clause.
2. `TAIGI_LAYOUT_SHOTS=<scratchpad>/macos-shots swift test --package-path macos --filter LayoutSimTests`. The terminal needs Screen Recording permission; shots come out at the main display's scale. `make macos-libs` first when `desktop/` / `macos/crates/` changed (stale-artifact gate).
3. Also `swift test --package-path macos --filter 'CandidatePanelPositioningTests|CandidateMetricsTests|ExpandedGridLayoutTests|HorizontalPageLayoutTests|CandidateElasticWidthTests|VerticalExpandableCandidatePanelTests'`.

Settings are `UserDefaults.standard` keys with the desktop spellings (`SettingsStore.Keys`): `candidateLayout`, `candidateTextSize` (Swift `Keys.candidateSize`; `"13"` … `"23"`), `fontType`, `candidateDisplayMode`, `inputMode`. The test restores every key it touches (`clearSettingRestoredAtTeardown`).

### 9.2 Matrix and kinds

Layout (4, expandables folded and opened) × size `13` / `17` / `23` × font (5) × document position (middle, near the bottom of the visible frame, near the right edge — move the stand-in window). Kinds as §7.4; design: an over-wide cell truncated with "…" (`byTruncatingTail`). Known divergence, not a new finding: `CandidatePanelPositioning.swift` clamps to the full `visibleFrame` with no 12-point edge margin (Windows has one). Owning files: `macos/Sources/TaigiInputMethodCore/Candidates/`. Report as §4, a table per layout (rows: font × size × position).

**Screen axes, only when named**: a 1× (non-Retina) display and a second display — move the stand-in document into the other screen's `visibleFrame` (`NSScreen.screens`); the panel stays on the caret's screen. Needs a display the maintainer connects. Extra kinds as §7.3 (`panel-wrong-scale`, `panel-on-wrong-monitor`).

## 10. Browser extension

The extension draws the page's own candidate panel (`panel.js`, `style.css` with `:root` → `:host`, plus `web/extension/overlay.css`) in a shadow tree over a website (`docs/architecture/browser-extension.md`). §1 covers the panel's CSS; this run covers what only the extension adds: placement against a site's field (`web/extension/field.js` `visibleBox`), system fonts only, the editors' DOM, and the chip. No harness script: you write a temporary spec and read every screenshot.

### 10.1 Run

1. `make -C web extension extension-fixtures` (stages `web/target/extension/` and `web/target/extension-fixtures/`; `make -C web build` first on a clean tree).
2. Write a temporary `web/extension/e2e/layout-sim.spec.mjs` (never committed; delete it at the end) on `web/extension/e2e/helpers.mjs`: `test` (the extension in a persistent context, its `worker`), `openTyping(context, worker, page, url, selector)`, `overlay` / `preedit` / `candidates` / `chip`. Settings go in before `openTyping`: `worker.evaluate(() => chrome.storage.sync.set({ "taigi.settings": JSON.stringify({ revision: 1, values: { candidateLayout, candidateTextSize } }) }))` (pattern: `strict.spec.mjs`). Type the §1 sentences as §1 does (toneless words, arrows on the second word of a clause, Space, Enter), read the sentences from the corpus at run time the way `web/layout-sim/sentences.mjs` does, and screenshot the page after every word.
3. `npx playwright test --config web/extension/playwright.config.mjs layout-sim` — port 4174, output `web/test-results/extension/`. Same port and `test-results/` cautions as §1: never run `make -C web e2e` or `extension-e2e` alongside.

### 10.2 Matrix

| Axis | Values |
|---|---|
| Fixture (field) | `fields.html` (textarea, input, contenteditable) · `codemirror5.html` · `codemirror6.html` · `lexical.html` · `quill.html` · `slate.html` · `strict.html` (wasm2js fallback) |
| Viewport | desktop 1280×800 · phone 360×800 |
| Candidate layout | `vertical` · `horizontal` · `expandable` · `verticalExpandable` |
| Size (`candidateTextSize`) | `13` · `17` · `23` |
| Field position | top of the viewport · scrolled so the field sits at the bottom · right edge |

Chromium only (the extension's gate). Other browsers run only when named.

### 10.3 Kinds

§2's kinds, measured against the site's field instead of the page's editor, plus:

| Kind | Means |
|---|---|
| `panel-covers-field-text` | the panel covers the composition or the field's text around the caret |
| `chip-covers-field-text` / `chip-off-field` | the mode chip covers the field's text, or is not beside the field it names |
| `panel-font-tofu` | a candidate draws as tofu in the system fonts (font coverage, report apart from layout) |

Design, not defects: inside a `<dialog>.showModal()` the panel draws under the dialog (top layer, known limitation); no TPS key panel or Telex guide. Owning files: `web/extension/overlay.js`, `overlay.css`, `field.js`, plus §3's web files. Report as §4, a table per fixture (rows: viewport × layout × size).
