---
name: layout
description: Run the layout test (版面測試) — type real corpus sentences word by word, on the staged web page (viewports × candidate layouts × candidate text sizes) or on the Android keyboard in an emulator (devices × orientations × fonts × font scales), check the candidate window / strip, expanded grid, symbol picker and key faces after every word, screenshot each failure, and publish the findings. Use when asked to "run the layout test", 「跑 Web 的版面測試」, 「跑 Android 的版面測試」, 「版面測試」, or to check that candidates and key labels never overlap, clip or overflow. Args: a platform (`web` or `android`) and optionally its matrix filters. Reports findings; never opens a fix round itself.
disable-model-invocation: false
---

# Layout test

Born from the 2026-11-24 cross-platform layout audit. Web: an automated harness, `web/layout-sim/` (§1–§4). Android: an agent-driven emulator procedure plus the layout instrumented tests (§5). Another platform joins this skill after its own layout fix round.

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

- A candidate wider than the whole list less the row's index and padding, cut with "…" — the check already skips those (at `zoom200` a five-Hanji word does not fit a `horizontal` / `expandable` cell). The romanization is truncated with "…" by design (the candidate stays whole, `web/e2e/look.spec.mjs`) and is never checked for cuts.
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
| `fullscreen-extract` | landscape shows the framework's extract editor over the app |

The thresholds are the ones `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/keyboard/KeyLabelGeometry.kt` draws to; measure in pixels from the crop when a case is close. Design, not defects: a candidate wider than the whole row ellipsized; tofu for a character no bundled font has (a font coverage report, not layout).

### 5.7 Report

As §4, with the table per device × orientation instead of per browser (rows: font × font scale), the failed instrumented tests listed with their message, and each defect traced to its file under `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/` (`candidates/CandidateStrip.kt`, `overlays/`, `keyboard/KeyContent.kt`).
