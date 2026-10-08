---
name: previews
description: Regenerate the keyboard screenshots shown on the mobile Theme and Layout pages (and the in-keyboard layout picker) by rendering the real keyboard view on a simulator. Use after a change to built-in themes, layouts, key styling, fonts or the KeyboardKit version, or when a new built-in theme / layout needs its card image. iOS only for now. Overwrites the PNGs in ios/Resources/Assets/LayoutPreviewAssets.xcassets; never commits.
disable-model-invocation: false
---

# Preview screenshots

The generator is `ios/TaigiKeyboardTests/PreviewAssetGeneratorTests.swift`. It renders `KeyboardPreviewPanel` (the theme editor's live preview, i.e. the real `TaigiKeyboardView`) with the **factory settings**, crops the key area, and overwrites every PNG that each imageset's `Contents.json` names, as 720×454 8-bit RGB. Ordinary test runs skip it.

1. **Stale artifacts.** Apply `AGENTS.md` § Stale-artifact gate first (`engine/` changed → `make build`; `dictionary/` changed → `make dict`, then `make build`).
2. **New card?** A new built-in theme or layout needs its imageset first: a `<name>.imageset/` folder with `Contents.json` + a placeholder PNG named in it (copy a sibling: light+dark for adaptive, one universal image otherwise). The imageset name must equal the layout's `KeyboardLayoutType.previewImageName` or the theme's `previewImageName`; the generator fails on any other.
3. **Run** on an **iPhone** simulator (it fails on iPad), alone, with parallel testing off. While it runs, it clears the simulator's App Group settings and restores them afterwards.

   ```sh
   TEST_RUNNER_PREVIEW_ASSETS_DIR="$PWD/ios/Resources/Assets/LayoutPreviewAssets.xcassets" \
     xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboardTests \
     -destination 'platform=iOS Simulator,name=iPhone 17' -parallel-testing-enabled NO \
     -only-testing:TaigiKeyboardTests/PreviewAssetGeneratorTests test 2>&1 | grep -E "preview.generate|error:|TEST "
   ```

   Expect `preview.generate.complete count=<N>` (N = PNGs in the catalog) and `** TEST SUCCEEDED **`.
4. **Look at the images**, at least: an adaptive Default light + dark (`theme_standard_preview`), Catppuccin (dark-only palette), `layout_tps_preview`, one framed and one clean theme. Each must show the full key area — the number row at the top, the `123` row whole at the bottom, no candidate bar. For a one-image overview, tile them (4 per row) into the session scratchpad and Read the result:

   ```sh
   SCRATCH=<session scratchpad directory>
   ffmpeg -v error -y -pattern_type glob -i 'ios/Resources/Assets/LayoutPreviewAssets.xcassets/*/*.png' \
     -vf "scale=360:227,pad=370:237:5:5:gray,tile=4x9" -frames:v 1 "$SCRATCH/preview_sheet.png"
   ```

5. **Report** `git diff --stat -- ios/Resources/Assets/LayoutPreviewAssets.xcassets` and the absolute paths of the images you checked. Committing is the caller's round. A gradient image can differ between runs by ±1 in a few pixels (dithering); when that is its only change, `git checkout` it rather than commit the churn.

Note: the Liquid Glass backdrop colours are generator constants sampled from iOS 27 — re-sample them when a new iOS changes the system keyboard backdrop.
