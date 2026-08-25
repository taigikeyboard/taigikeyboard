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

## macOS is a separate surface

The store notes cover iOS and Android only. macOS announces itself through its own GitHub release: `macos/scripts/publish-release.sh` extracts the `### macOS` section of `changelog/vMAJOR.MINOR.PATCH.md` and passes that to `gh release create` as the release body, falling back to a one-line `TaigiKeyboard for macOS <version>` note when the section is missing (`docs/architecture/macos-release.md`).

That splits the two surfaces cleanly:

| Surface | Audience | macOS content |
| --- | --- | --- |
| `changelog/vMAJOR.MINOR.PATCH.md` | the detailed record; its `### macOS` section is the macOS GitHub release body | That section, headed exactly `### macOS`. Nothing fails without it — the macOS release just ships the one-line fallback note instead |
| `changelog/store/vMAJOR.MINOR.PATCH/{ios,android}.txt` | App Store and Google Play What's New | **Excluded** — mobile users cannot see macOS-only work |

Shared-engine work that ships on every platform is a mobile change too, so it belongs in the mobile notes on its own merits — described by what a phone user sees, not by the platforms it happened to land on.

`validate_notes` enforces the exclusion: `macOS` and `Mac` are forbidden terms in both `ios.txt` and `android.txt`, matched as whole words so ordinary release-note words such as "machine" and "match" still pass.

All three platforms ship one version number. `check-versions` holds `macos/App/Info.plist` to it as well — `CFBundleShortVersionString` equals the release version, and `CFBundleVersion` equals `MAJOR*10000 + MINOR*100 + PATCH`, the same derivation `macos/scripts/release-app.sh` enforces at package time.

## Prepare notes

Create both canonical files, then synchronize the apps:

```bash
python3 tools/release_notes.py sync --version vMAJOR.MINOR.PATCH --date YYYY/MM/DD
```

Validate the canonical files, generated app histories, and the marketing versions of all three platforms:

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
