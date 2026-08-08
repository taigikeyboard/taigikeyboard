# Manual Release Notes

## Goal

Maintain one concise English What's New source per platform, show the same content inside each app, and let the release owner paste it into App Store Connect or Google Play manually. No store API credentials or publishing automation are involved.

## Canonical sources

```text
changelog/store/vMAJOR.MINOR.PATCH/ios.txt
changelog/store/vMAJOR.MINOR.PATCH/android.txt
```

Each non-empty line is one entry without a bullet marker. `tools/release_notes.py` validates the entries, adds bullets for store output, and mirrors them into the newest version-history entry on each platform.

Platform notes may differ when shipped behavior differs. For a given platform, its canonical notes, in-app version history, and manually pasted store text must contain the same entries.

## Prepare notes

Create both canonical files, then synchronize the apps:

```bash
python3 tools/release_notes.py sync --version vMAJOR.MINOR.PATCH --date YYYY/MM/DD
```

Validate the canonical files, generated app histories, and project marketing versions:

```bash
python3 tools/release_notes.py check --version vMAJOR.MINOR.PATCH
python3 tools/release_notes.py check-versions --version vMAJOR.MINOR.PATCH
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
