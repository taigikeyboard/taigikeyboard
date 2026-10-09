---
name: previews
description: Regenerate the keyboard screenshots shown on the mobile Theme and Layout pages (and the in-keyboard layout picker) by rendering the real keyboard view on an iOS simulator / Android emulator. Use after a change to built-in themes, layouts, key styling, fonts or the KeyboardKit version, or when a new built-in theme / layout needs its card image. Args: `ios`, `android`, or nothing (both). Overwrites the preview PNGs in the repo; never commits.
disable-model-invocation: false
---

# Preview screenshots

Each platform has a generator test that renders the theme editor's live preview (`KeyboardPreviewPanel`, i.e. the real keyboard view) with the **factory settings**, crops the key area to the card aspect and writes every card image. Ordinary test runs skip both. iPad and tablets reuse the phone-shaped images.

**Stale artifacts first.** Apply `AGENTS.md` § Stale-artifact gate (`engine/` changed → `make build`; `dictionary/` changed → `make dict`, then `make build`).

## iOS

Generator: `ios/TaigiKeyboardTests/PreviewAssetGeneratorTests.swift`. It overwrites every PNG that each imageset's `Contents.json` in `ios/Resources/Assets/LayoutPreviewAssets.xcassets` names, as 720×454 8-bit RGB.

1. **New card?** Add its imageset first: a `<name>.imageset/` folder with `Contents.json` + a placeholder PNG named in it (copy a sibling: light+dark for adaptive, one universal image otherwise). The name must equal the layout's `KeyboardLayoutType.previewImageName` / `pojPreviewImageName` (rendered in POJ mode) or the theme's `previewImageName`; the generator fails on any other.
2. **Run** on an **iPhone** simulator (it fails on iPad), alone, with parallel testing off. It clears the simulator's App Group settings while it runs and restores them afterwards.

   ```sh
   TEST_RUNNER_PREVIEW_ASSETS_DIR="$PWD/ios/Resources/Assets/LayoutPreviewAssets.xcassets" \
     xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboardTests \
     -destination 'platform=iOS Simulator,name=iPhone 17' -parallel-testing-enabled NO \
     -only-testing:TaigiKeyboardTests/PreviewAssetGeneratorTests test 2>&1 | grep -E "preview.generate|error:|TEST "
   ```

   Expect `preview.generate.complete count=<N>` (N = PNGs in the catalog) and `** TEST SUCCEEDED **`.

Note: the Liquid Glass backdrop colours are generator constants sampled from iOS 27 — re-sample them when a new iOS changes the system keyboard backdrop.

## Android

Generator: `android/app/src/androidTest/java/com/siansiansu/taigikeyboard/ui/PreviewAssetGeneratorTest.kt`. It renders every card `KeyboardLayoutOptions` (`previewRes`, plus `pojPreviewRes` in POJ mode) and `builtInThemePreviewRes` (`ThemePickerScreen.kt`) name, as 800×540 PNGs for `drawable-xxxhdpi` (+ `drawable-night-xxxhdpi` for adaptive themes), into the app's `files/previews/`. A new layout needs its `KeyboardLayoutOptions` entry; a new built-in theme is picked up from `BuiltInThemes.all` and needs its `builtInThemePreviewRes` mapping (the generator fails without it). Either way add a placeholder drawable so `R.drawable` compiles.

1. **Device**: an **emulator only** — the run wipes the app's data. A portrait arm64 phone AVD (the APK ships ARM ABIs); boot one headless if none is up, e.g. `ANDROID_AVD_HOME=$HOME/.config/.android/avd ~/Library/Android/sdk/emulator/emulator -avd <phone AVD> -no-window -no-audio -gpu swiftshader_indirect` (background). Pin every command to it with `export ANDROID_SERIAL=emulator-5554`.
2. **Run** (factory settings come from `pm clear`):

   ```sh
   android/gradlew -q -p android :app:installDebug :app:installDebugAndroidTest
   adb shell pm clear com.siansiansu.taigikeyboard
   adb shell am instrument -w -e class com.siansiansu.taigikeyboard.ui.PreviewAssetGeneratorTest -e previews true \
     com.siansiansu.taigikeyboard.test/androidx.test.runner.AndroidJUnitRunner | grep -E "preview.generate|OK|FAIL"
   ```

   Expect `preview.generate.complete count=<N>` and `OK (1 test)`.
3. **Pull into the scratchpad, check, then replace** the card images (the archive holds `drawable-xxxhdpi/` and `drawable-night-xxxhdpi/` at its root). Deleting every card image first makes the run the whole set, so no stale bucket copy survives (`appearance_preview.png` does not match):

   ```sh
   OUT=<session scratchpad directory>/android_previews; rm -rf "$OUT"; mkdir -p "$OUT"
   adb exec-out run-as com.siansiansu.taigikeyboard tar c -C files/previews . | tar x -C "$OUT"
   rm android/app/src/main/res/drawable*/{layout,theme}_*_preview.png
   cp "$OUT"/drawable-xxxhdpi/*.png android/app/src/main/res/drawable-xxxhdpi/
   cp "$OUT"/drawable-night-xxxhdpi/*.png android/app/src/main/res/drawable-night-xxxhdpi/
   ```

## Check and report

1. **Look at the images**, at least: an adaptive Default light + dark, Catppuccin (dark-only palette), the TPS layout, one framed and one clean theme. Each must show the full key area — number row at the top, the bottom row whole with a margin under it, no candidate bar. Tile them for one Read:

   ```sh
   ffmpeg -v error -y -pattern_type glob -i '<image dir>/*/*.png' \
     -vf "scale=360:-1,pad=370:ih+10:5:5:gray,tile=4x9" -frames:v 1 "<scratchpad>/preview_sheet.png"
   ```

2. **Report** `git diff --stat` of the image folders and the absolute paths of the images you checked. Committing is the caller's round. A gradient image can differ between runs by ±1 in a few pixels (dithering); when that is its only change, `git checkout` it rather than commit the churn.
