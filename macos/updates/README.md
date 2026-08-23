# macOS update manifest

`latest.json` is the release manifest the macOS input method's update check
reads, served over HTTPS straight from this repository:

    https://raw.githubusercontent.com/taigikeyboard/taigikeyboard/main/macos/updates/latest.json

The checker (`macos/Sources/TaigiInputMethodCore/Settings/UpdateChecker.swift`)
compares `version` against the installed `CFBundleShortVersionString` and, when
the manifest is strictly newer, offers to open `downloadPageURL` in the
browser. Notify-only: nothing here is downloaded and executed, which is why the
file needs no signing — HTTPS plus a version comparison is the whole contract.

## Wire format

| Field | Meaning |
|---|---|
| `version` | Newest downloadable version, dotted integers only (`3.6.5`). No suffixes — the checker rejects them. |
| `downloadPageURL` | Page the user lands on, `https` only. A page, not a file: the user downloads and runs the pkg themselves. |

Unknown extra fields are ignored by old installs, so the format can grow.

## Publishing a release

The manifest is a release indicator, not a build artifact — it is edited by
hand, after the release actually exists, in this order:

1. Bump `CFBundleShortVersionString` AND `CFBundleVersion` in
   `macos/App/Info.plist` (build version must strictly increase — Installer
   compares it), then `make macos-release` and verify the notarized pkg.
2. Upload the pkg and confirm its download page is live.
3. Only then update `latest.json` to the new version and the real page,
   review, commit, push. `release-app.sh` prints the suggested JSON.

Updating the manifest before the pkg is reachable points every checker at a
download that is not there.

When the download website exists, both this file's serving location and
`downloadPageURL` move there together (`UpdateChecker.manifestURL` is the one
constant to change).
