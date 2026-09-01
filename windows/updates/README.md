# Windows update manifest

The Windows input method checks for updates by fetching one static JSON file:

    https://taigikeyboard.tw/appcast/windows.json

It is the macOS manifest's twin (`macos/updates/README.md`), same wire format,
read by `windows/crates/taigi-windows-update`. `version` is compared against
the running settings exe's version (the workspace's); when the manifest is
strictly newer, the 一般 pane offers the update: a signed copy downloads
`packageURL` itself, verifies it, and — on a second press — opens the
installer; everywhere else (the toast, an unsigned development build, a
manifest without `packageURL`) the browser opens `downloadPageURL`.

The file is unsigned, deliberately: what stops a substituted package being
installed is its own Authenticode signature, pinned against the certificate
the running copy was signed with (the signer's leaf thumbprint), plus its
VERSIONINFO naming this product at the manifest's version.

## Wire format

| Field | Meaning |
|---|---|
| `version` | Newest downloadable version, dotted integers only (`3.7.0`). No suffixes — the checker rejects them. |
| `downloadPageURL` | Page the user lands on, `https` only. Required. |
| `packageURL` | The signed `TaigiKeyboard-<version>.exe`, `https` only. Optional; an invalid one is dropped on its own. |

Unknown extra fields are ignored. Before the first release the manifest reads
`0.0.0`, which notifies nobody.

## Where it lives

In the website repository (`taigikeyboard/taigikeyboard.github.io`), at
`appcast/windows.json` — never by hand, never from this (private)
repository, and never before the installer is anonymously reachable.

### One published fact, one committed file

`windows/scripts/publish-release.sh` writes exactly one file over there,
`_data/windows_release.json`:

```json
{
  "version": "3.7.0",
  "tag": "windows-v3.7.0",
  "downloadURL": "https://github.com/taigikeyboard/taigikeyboard.github.io/releases/download/windows-v3.7.0/TaigiKeyboard-3.7.0.exe",
  "releasePageURL": "https://github.com/taigikeyboard/taigikeyboard.github.io/releases/tag/windows-v3.7.0"
}
```

That file is what the site's Windows download button links at — straight at
the installer so the download starts on one click, which is why its URL
carries the version, and deliberately not `/releases/latest/download/...`,
since `latest` resolves across a repository that is a website rather than
this input method's release channel. Whether the button is shown at all is
the site's `enable_windows_download` flag, not this file: the release flow
fills the file the moment any installer is published, a throwaway test
publish included.

The manifest is **rendered** from it by the site's own build:
`appcast/windows.json` over there is a Jekyll template reading
`site.data.windows_release`, mapping `releasePageURL` → `downloadPageURL`
and `downloadURL` → `packageURL`. Nothing writes it directly.

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
