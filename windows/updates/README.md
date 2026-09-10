# Windows update manifest

The Windows input method checks for updates by fetching one static JSON file:

    https://taigikeyboard.tw/appcast/windows.json

It is the macOS manifest's twin (`macos/updates/README.md`), same wire format,
read by `windows/crates/taigi-windows-update`. `version` is compared against
the running settings exe's version (the workspace's); when the manifest is
strictly newer, the 一般 pane offers the update: the copy downloads
`packageURL` itself, verifies it, and — on a second press — opens the
installer. Where it cannot (a manifest without a `packageURL` or without its
`packageSHA256`, nowhere to stage) and from the toast, the browser opens
`downloadPageURL` instead.

"Verifies it" is `packageSHA256` always, plus the package's Authenticode
signature when the running copy carries one of its own. Releases are unsigned
today, so for now the digest is the whole bar — what that is and is not worth
is `docs/architecture/windows-release.md` § Signing status, which owns that
policy.

The manifest file itself carries no signature of its own, and does not need
one: it is served over HTTPS from a domain the project controls, and what a
copy checks is the package it names.

## Wire format

| Field | Meaning |
|---|---|
| `version` | Newest downloadable version, dotted integers only (`3.7.0`). No suffixes — the checker rejects them. |
| `downloadPageURL` | Page the user lands on, `https` only. Required. |
| `packageURL` | The `TaigiKeyboard-<version>.exe` an in-app install fetches, `https` only. Optional, and only usable with the digest below. |
| `packageSHA256` | That file's SHA-256, 64 hex digits (case-insensitive). **`packageURL` and this are one fact**: either half missing or invalid reads as no package at all, so the update is announced with the download page as its action. **Windows only** — the macOS manifest has no such field, because it pins a downloaded package by its Developer ID signature instead. |

Unknown extra fields are ignored. Before the first release the manifest reads
`0.0.0`, which notifies nobody.

## Where it lives

In the website repository (`taigikeyboard/taigikeyboard.github.io`), at
`appcast/windows.json` — never by hand, and never before the installer is
anonymously reachable. It lives there, not here, because `PUBLISHED_URL` is
compiled into every shipped copy: the manifest has to stay at a fixed URL on a
domain the project controls. The installer it announces is a release asset in
this repository.

### One published fact, one committed file

`windows/scripts/publish-release.sh` writes exactly one file over there,
`_data/windows_release.json`:

```json
{
  "version": "3.7.0",
  "tag": "desktop-3.7.0",
  "downloadURL": "https://github.com/taigikeyboard/taigikeyboard/releases/download/desktop-3.7.0/TaigiKeyboard-3.7.0.exe",
  "sha256": "115b6d19c0a2f4e6ab8d7315f0c9e24d5b6a1f8309e7c4d25a0b3f6178e917d2",
  "releasePageURL": "https://github.com/taigikeyboard/taigikeyboard/releases/tag/desktop-3.7.0"
}
```

That file is what the site's Windows download button links at — straight at
the installer so the download starts on one click, which is why its URL
carries the version, and deliberately not `/releases/latest/download/...`,
since `latest` resolves repository-wide and the last release here may be a
macOS one. Whether the button is shown at all is
the site's `enable_windows_download` flag, not this file: the release flow
fills the file the moment any installer is published, a test publish
included.

The manifest is **rendered** from it by the site's own build:
`appcast/windows.json` over there is a Jekyll template reading
`site.data.windows_release`, mapping `releasePageURL` → `downloadPageURL`,
`downloadURL` → `packageURL` and `sha256` → `packageSHA256`. Nothing writes
it directly. `sha256` is read back from the PUBLISHED asset by the release
flow, not just computed locally, so the digest the manifest names is one the
URL was serving.

Same shape as macOS (`macos/updates/README.md` § One published fact, one
committed file), for the reason recorded there: two literal files meant two
commits seconds apart, every GitHub Pages run deploys the tree of *its own*
commit, and on 2026-08-28 the run for the earlier commit finished last and
served the macOS manifest a release behind for a day. One file cannot race
with itself. Before the first Windows release the data file names `0.0.0`
with no download, which notifies nobody.

## Who checks, when

Roadmap W9: the installer's per-user scheduled task runs
`TaigiKeyboardSettings.exe --check-updates` (headless: due ⇒ fetch, record,
toast once per version); the settings window checks when overdue at launch;
檢查更新 in the 一般 pane and the lang-bar menu check on demand; the DLL only
reads the recorded pending manifest. `updateNextCheckMs` in `settings.json`
is stamped BEFORE each fetch, so a hanging server is asked once a day, not
once a launch.
