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
| `packageURL` | The signed `TaigiKeyboard-<version>-Setup.exe`, `https` only. Optional; an invalid one is dropped on its own. |

Unknown extra fields are ignored. Before the first release the manifest reads
`0.0.0`, which notifies nobody.

## Where it lives

In the website repository (`taigikeyboard/taigikeyboard.github.io`, at
`appcast/windows.json`), written by `windows/scripts/publish-release.sh` only
after the installer is anonymously reachable — never by hand, never from this
(private) repository. The same script writes `_data/windows_release.json` for
the site's download button.

## Who checks, when

Roadmap W9: the installer's per-user scheduled task runs
`TaigiKeyboardSettings.exe --check-updates` (headless: due ⇒ fetch, record,
toast once per version); the settings window checks when overdue at launch;
檢查更新 in the 一般 pane and the lang-bar menu check on demand; the DLL only
reads the recorded pending manifest. `updateNextCheckMs` in `settings.json`
is stamped BEFORE each fetch, so a hanging server is asked once a day, not
once a launch.
