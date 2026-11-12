# Manual Release Notes

## Goal

Maintain one concise English What's New source per mobile platform and let the release owner paste it into App Store Connect or Google Play manually. The apps show no version history, since users do not read it; the detailed record lives in `changelog/` only. No store API credentials or publishing automation are involved.

## Canonical sources

```text
changelog/store/vMAJOR.MINOR.PATCH/ios.txt
changelog/store/vMAJOR.MINOR.PATCH/android.txt
```

Each non-empty line is one entry without a bullet marker. `tools/release_notes.py` validates the entries and adds bullets for store output.

Platform notes may differ when shipped behavior differs. For a given platform, its canonical notes and manually pasted store text must contain the same entries.

## Two release trains

Two version numbers, moving independently:

| Train | Platforms | Version files | Detailed record | Announces itself through |
| --- | --- | --- | --- | --- |
| **mobile** | iOS + Android | Android `versionName`, iOS shipping targets' `MARKETING_VERSION` | `changelog/mobile-vMAJOR.MINOR.PATCH.md` | App Store / Google Play What's New (`changelog/store/vMAJOR.MINOR.PATCH/{ios,android}.txt`) |
| **desktop** | macOS + Windows + Linux + ChromeOS | both macOS plist keys, the `[workspace.package] version` of `windows/Cargo.toml`, `desktop/Cargo.toml`, `linux/Cargo.toml` and `macos/Cargo.toml`, and `web/chromeos/manifest.json` `version` | `changelog/desktop-vMAJOR.MINOR.PATCH.md` | one GitHub release in this repository, tagged `desktop-MAJOR.MINOR.PATCH`, holding every platform's installer, whose body is that whole file — staged as a draft, published by a person, announced automatically ([`desktop-release.md` § The flow](desktop-release.md#the-flow)) |

Within a train the platforms cannot drift apart — `set-versions` writes every version file of the train or none, and `check-versions` holds them all to one number. Across trains nothing is compared: a mobile 3.6.7 and a desktop 3.7.0 are two unrelated facts, and the same number appearing in both is a coincidence, not a link. A train therefore never skips a number because the other train used it: after desktop 3.6.7 the next mobile release is still whatever follows the last *mobile* number. The mobile train's git tags are `mobile-MAJOR.MINOR.PATCH` (bare `vMAJOR.MINOR.PATCH` up to `v3.6.5`; `dictionary/build/version_snapshot.py` reads both); the desktop train's are `desktop-MAJOR.MINOR.PATCH`, created by publishing the release (desktop releases cut before 2026-09-09 are tagged `macos-v…` / `windows-v…` on the website repository).

The store notes cover the mobile train only. Shared-engine work that ships on every platform is a mobile change too, so it belongs in the mobile notes on its own merits — described by what a phone user sees, not by the platforms it happened to land on. Desktop-only work never enters `changelog/mobile-vMAJOR.MINOR.PATCH.md`: it waits for the desktop train's own file.

`validate_notes` enforces the exclusion: `macOS` and `Mac` are forbidden terms in both `ios.txt` and `android.txt`, matched as whole words so ordinary release-note words such as "machine" and "match" still pass.

`check-versions --train desktop` holds `macos/App/Info.plist` to the desktop version — `CFBundleShortVersionString` equals it, and `CFBundleVersion` equals `MAJOR*10000 + MINOR*100 + PATCH`, the same derivation `macos/scripts/release-app.sh` enforces at package time — and `windows/Cargo.toml`, `desktop/Cargo.toml`, `linux/Cargo.toml`, `macos/Cargo.toml` and `web/chromeos/manifest.json` to the same number. That build number is what Installer compares between packages, so the desktop train only ever moves upward; the per-train downgrade guard in `set-versions` is what enforces it. The formula also caps MINOR and PATCH at 99 each (`MAX_MACOS_VERSION_COMPONENT`, `tools/release_notes.py`); beyond that two versions would collide.

## Set the version

One command per train writes that train's version into all of its project files, or into none:

```bash
make version-mobile MAJOR.MINOR.PATCH    # iOS + Android
make version-desktop MAJOR.MINOR.PATCH   # macOS + Windows + Linux + ChromeOS
```

One train per invocation. Neither build number is a maintainer's problem: the iOS `CURRENT_PROJECT_VERSION` is pinned to 1 because App Store Connect numbers a marketing version's uploads itself, Android's `versionCode` is epoch minutes, and the macOS `CFBundleVersion` derives from the desktop version.

## Prepare notes

Create both canonical files, then validate them and the mobile train's marketing versions:

```bash
python3 tools/release_notes.py check --version vMAJOR.MINOR.PATCH
python3 tools/release_notes.py check-versions --train mobile --version vMAJOR.MINOR.PATCH
python3 tools/release_notes_test.py
```

The validator requires one to eight factual user-visible entries, approved prefixes, terminal punctuation, platform-safe wording, and at most 500 rendered Unicode characters. The 500-character ceiling is Google Play's and is the real limit; the entry count is a shape check.

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

The tooling does not authenticate with either store, upload binaries, select builds, promote tracks, submit review, or release production. Those actions remain entirely manual. The one git-side step it takes is the `release-mobile` skill's: it tags the release commit `mobile-<version>` and pushes the tag (`.claude/skills/release-mobile/SKILL.md` § 6).
