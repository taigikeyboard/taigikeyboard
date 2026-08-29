# Manual Release Notes

## Goal

Maintain one concise English What's New source per mobile platform, show the same content inside each app, and let the release owner paste it into App Store Connect or Google Play manually. No store API credentials or publishing automation are involved.

## Canonical sources

```text
changelog/store/vMAJOR.MINOR.PATCH/ios.txt
changelog/store/vMAJOR.MINOR.PATCH/android.txt
```

Each non-empty line is one entry without a bullet marker. `tools/release_notes.py` validates the entries, adds bullets for store output, and mirrors them into the newest version-history entry on each platform.

Platform notes may differ when shipped behavior differs. For a given platform, its canonical notes, in-app version history, and manually pasted store text must contain the same entries.

## Two release trains

Two version numbers, moving independently (USER 2026-08-29):

| Train | Platforms | Version files | Detailed record | Announces itself through |
| --- | --- | --- | --- | --- |
| **mobile** | iOS + Android | Android `versionName`, iOS shipping targets' `MARKETING_VERSION` | `changelog/vMAJOR.MINOR.PATCH.md` | App Store / Google Play What's New (`changelog/store/vMAJOR.MINOR.PATCH/{ios,android}.txt`) |
| **desktop** | macOS + Windows | both macOS plist keys, `windows/Cargo.toml` `[workspace.package] version` | `changelog/desktop-vMAJOR.MINOR.PATCH.md` | its own GitHub release per platform: `macos/scripts/publish-release.sh` extracts the `### macOS` section and `windows/scripts/publish-release.sh` the `### Windows` section as the release body, each falling back to a one-line `TaigiKeyboard for <platform> <version>` note when its section is missing |

Within a train the platforms cannot drift apart — `set-versions` writes both files or neither, and `check-versions` holds both to one number. Across trains nothing is compared: a mobile 3.6.7 and a desktop 3.7.0 are two unrelated facts, and the same number appearing in both is a coincidence, not a link. The repository's `vMAJOR.MINOR.PATCH` git tags are the mobile train's (`dictionary/build/version_snapshot.py` reads them); desktop releases are tagged `macos-v…` / `windows-v…` on the website repository.

The store notes cover the mobile train only. Shared-engine work that ships on every platform is a mobile change too, so it belongs in the mobile notes on its own merits — described by what a phone user sees, not by the platforms it happened to land on. Desktop-only work never enters `changelog/vMAJOR.MINOR.PATCH.md`: it waits for the desktop train's own file.

`validate_notes` enforces the exclusion: `macOS` and `Mac` are forbidden terms in both `ios.txt` and `android.txt`, matched as whole words so ordinary release-note words such as "machine" and "match" still pass.

`check-versions --train desktop` holds `macos/App/Info.plist` to the desktop version — `CFBundleShortVersionString` equals it, and `CFBundleVersion` equals `MAJOR*10000 + MINOR*100 + PATCH`, the same derivation `macos/scripts/release-app.sh` enforces at package time — and `windows/Cargo.toml` to the same number. That build number is what Installer compares between packages, so the desktop train only ever moves upward; the per-train downgrade guard in `set-versions` is what enforces it.

## Set the version

One command per train writes that train's version into both of its project files, or into neither:

```bash
make version-mobile MAJOR.MINOR.PATCH    # iOS + Android
make version-desktop MAJOR.MINOR.PATCH   # macOS + Windows
```

One train per invocation. Neither build number is a maintainer's problem: the iOS `CURRENT_PROJECT_VERSION` is pinned to 1 because App Store Connect numbers a marketing version's uploads itself, Android's `versionCode` is epoch minutes, and the macOS `CFBundleVersion` derives from the desktop version.

## Prepare notes

Create both canonical files, then synchronize the apps:

```bash
python3 tools/release_notes.py sync --version vMAJOR.MINOR.PATCH --date YYYY/MM/DD
```

Validate the canonical files, generated app histories, and the mobile train's marketing versions:

```bash
python3 tools/release_notes.py check --version vMAJOR.MINOR.PATCH
python3 tools/release_notes.py check-versions --train mobile --version vMAJOR.MINOR.PATCH
python3 tools/release_notes_test.py
```

The validator requires one to five factual user-visible entries, approved prefixes, terminal punctuation, platform-safe wording, and at most 500 rendered Unicode characters. The 500-character ceiling satisfies Google Play and keeps the Apple text concise.

## Copy for manual paste

iOS:

```bash
python3 tools/release_notes.py print --version vMAJOR.MINOR.PATCH --platform ios | pbcopy
```

Paste into the English **What's New in This Version** field in App Store Connect.

Android:

```bash
python3 tools/release_notes.py print --version vMAJOR.MINOR.PATCH --platform android | pbcopy
```

Paste into the English release-notes field for the intended Google Play release.

Always review the clipboard content before saving store metadata. The script checks deterministic hazards but cannot verify that every claim accurately describes the shipped build.

## Manual release boundary

The tooling does not authenticate with either store, upload binaries, select builds, create tags, promote tracks, submit review, or release production. Those actions remain entirely manual.
