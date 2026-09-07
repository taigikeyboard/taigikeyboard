# Code signing policy

Who may release a signed Taigi Keyboard binary, what gets signed, and what a
user can verify for themselves.

## Current signing status

| Platform | Artifact | Status |
| --- | --- | --- |
| Windows | `TaigiKeyboard-<version>.exe` (Inno Setup installer), `taigi_windows_tsf.dll`, `TaigiKeyboardSettings.exe` | **Unsigned.** Integrity rests on the SHA-256 digest published in the update manifest. |
| macOS | `TaigiKeyboard-<version>.pkg` | Signed and notarized with an Apple Developer ID. |
| iOS / Android | App Store / Google Play builds | Signed by the respective store pipeline. |

Windows releases being unsigned is the gap this policy exists to close. Until
it is, the published SHA-256 is the only integrity check a user has, and it
proves only that the file downloaded is the file published — not who built it.

## Roles

Releasing a binary splits into three roles: the **Author** who writes the code
and requests a release, the **Reviewer** who reviews what goes into the source
tree and the artifact built from it, and the **Approver** who approves the
release itself.

Taigi Keyboard is maintained by one person, who therefore holds all three:

| Role | Who |
| --- | --- |
| Author — commits source and build scripts, requests a release | Soo Bîn-hiân 蘇民弦 — <https://github.com/siansiansu> |
| Reviewer — reviews changes into `main` and the release artifact's provenance | Soo Bîn-hiân 蘇民弦 |
| Approver — approves each release | Soo Bîn-hiân 蘇民弦 |

Every maintainer of a source file or a build script in this repository is
listed above. There are no other committers, and no contributor's change
reaches a signed binary without passing through the review below.

One person cannot separate these roles, and this policy does not claim
otherwise. What stands in for that separation is that every step is recorded
where a third party can read it:

- Every change lands on `main` through a pull request from a branch, so the
  diff that entered a release is public and dated.
- The release script refuses to run on a dirty working tree, and the engine and
  dictionary it links are pre-built artifacts committed to the repository, so a
  release ships exactly what the released commit contains.
- Each release's artifact digest is published in the update manifest and read
  back from the served file, so it attests what the download URL actually
  serves rather than what was built locally.

Stated plainly because it bears on provenance: Windows releases are built by the
maintainer on their own Windows machine by `make windows-release`, not on a
hosted build server, and the source repository is not public.
`docs/architecture/windows-release.md` owns that procedure. Nothing here asks the reader to take that on
trust — the published SHA-256 below is what a user can actually check, and it
proves only that the file downloaded is the file published.

If a second maintainer joins, this table is updated in the same commit that
grants them access.

## Approval

Every release is approved by hand by the Approver above. No publish is
triggered automatically — not by a push, not by a tag, not by a
scheduled job, and not by a successful build. A build that completes produces
an unsigned artifact and waits.

Multi-factor authentication is required on the GitHub account and on every
account that can publish a release.

## What is signed

Only first-party binaries produced by this project's own release scripts:

- `windows/scripts/release-app.sh` — builds the DLL, the settings executable,
  and stages the installer payload
- `windows/scripts/publish-release.sh` — publishes the installer and records
  its digest

Third-party dependencies are statically linked into those binaries and are not
separately signed. Bundled data files — fonts, the compiled dictionary — are
not executable and are not signed.

Nothing built from another project's source is ever signed with this project's
certificate.

## What the software does

Taigi Keyboard is an input method. It converts what the user types into
Taiwanese text, entirely on-device.

- No keystroke, and no text, leaves the device.
- No analytics, no telemetry, no advertising identifier, no crash reporter.
- No account, and no network permission on the mobile builds.
- The desktop builds make one kind of network request: fetching a static JSON
  update manifest from `taigikeyboard.tw`, and, only when the user asks,
  downloading the package it names. Neither request carries an identifier, a
  cookie, or any content derived from what the user typed. Both are ordinary
  HTTPS requests, so the server necessarily sees the client's IP address and
  user agent, as it would for any web page; nothing in the application adds to
  that or correlates it across requests.
- Learned data — word frequency, word association, custom dictionary — stays
  in app-private storage and is excluded from OS automatic backup.

It contains no vulnerability scanning, no exploitation capability, and no
remote administration feature.

What the Windows installer changes on the system, all of it required for an
input method to work or to be updated, and all of it reversed on uninstall:

| Change | Why |
| --- | --- |
| Registers `taigi_windows_tsf.dll` as a TSF text service (`regsvr32`, x64 and x86) | This is what makes the keyboard selectable at all. |
| Creates the scheduled task `TaigiKeyboard Update Check` | Fetches the update manifest so the user is told a new version exists. Deleted by the uninstaller. |
| Writes a Start-menu shortcut and the usual uninstall registry entry | Standard for an installed application. |

The installer requires administrator rights because `regsvr32` writes to
`HKLM`, and because the text service must load into every user's process. It
installs no driver, no service, and no browser extension, and changes no
system-wide setting outside its own registration.

`SECURITY.md` carries the full privacy statement and the vulnerability
reporting address.

## Verifying a release yourself

Every Windows release publishes its SHA-256 in the update manifest at
<https://taigikeyboard.tw/appcast/windows.json>, alongside the download URL.
The digest is read back from the *published* asset, not merely computed
locally, so it attests the file that URL actually serves.

```powershell
Get-FileHash .\TaigiKeyboard-<version>.exe -Algorithm SHA256
```

If Windows releases are signed in future, the in-app updater additionally
verifies the downloaded package's Authenticode signature before offering to
install it — that path already exists and activates as soon as a running copy
carries a signature of its own.

## Why Windows releases are unsigned

An Authenticode certificate costs a few hundred US dollars a year, and the free
programme for open source projects requires the whole product — including the
data it bundles — to be open source. Taigi Keyboard bundles Taiwanese-language
dictionary data whose sources carry their own, non-open-source terms, so that
route is closed. Signing may be revisited; until then this page describes what
a user can verify instead.

## Licence

Source code: Apache License, Version 2.0 — see `LICENSE`.
Third-party components: `THIRD_PARTY_LICENSES.md`.
Dictionary data: `dictionary/LICENSE` — **not** Apache-2.0, and several sources
carry non-commercial terms.
