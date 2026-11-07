# Code signing policy

Who may release a signed TaigiKeyboard binary, what gets signed, and what a
user can verify for themselves.

## Current signing status

| Platform | Artifact | Status |
| --- | --- | --- |
| Windows | `TaigiKeyboard-<version>.exe` (Inno Setup installer), `TaigiKeyboard.dll`, `TaigiKeyboard32.dll`, `TaigiKeyboardArm64.dll`, `TaigiKeyboard64X.dll` (Arm64X forwarder), `TaigiKeyboardSettings.exe` (x64 and Arm64 builds) | **Unsigned.** Integrity rests on the SHA-256 digest published in the update manifest. |
| Linux | `taigikeyboard_<version>_amd64.deb`, `taigikeyboard-<version>-1.x86_64.rpm`, `taigikeyboard-<version>-1-x86_64.pkg.tar.zst` | **Unsigned.** Integrity rests on the `.sha256` attached beside each package on the release (`docs/architecture/linux-release.md`). |
| macOS | `TaigiKeyboard-<version>.pkg` | Signed and notarized with an Apple Developer ID. |
| iOS / Android | App Store / Google Play builds | Signed by the respective store pipeline. |

Windows releases being unsigned is the gap this policy exists to close. What
that costs a user (SmartScreen, Smart App Control) and what the published
SHA-256 does and does not prove: [`docs/architecture/windows-release.md` § Signing status](architecture/windows-release.md#signing-status--unsigned-is-the-current-channel).

## Roles

Releasing a binary splits into three roles: the **Author** who writes the code
and requests a release, the **Reviewer** who reviews what goes into the source
tree and the artifact built from it, and the **Approver** who approves the
release itself.

TaigiKeyboard has two maintainers, who share the first two roles. The Approver
role stays with one of them:

| Role | Who |
| --- | --- |
| Author — commits source and build scripts, requests a release | Soo Bîn-hiân 蘇民弦 — <https://github.com/siansiansu>; madmaxieee — <https://github.com/madmaxieee> |
| Reviewer — reviews changes into `main` and the release artifact's provenance | Soo Bîn-hiân 蘇民弦; madmaxieee |
| Approver — approves each release | Soo Bîn-hiân 蘇民弦 |

Every maintainer of a source file or a build script in this repository is
listed above. There are no other committers, and no other contributor's change
reaches a signed binary without a maintainer's review.

These roles are not strictly separated, and this policy does not claim
otherwise: both maintainers also author changes, and each can merge them without
a second review. What stands in for that separation is that every
step is recorded where a third party can read it, that review stays with the
maintainers, and that release tags and release secrets stay behind the Approver:

- Changes land on `main` through a pull request from a branch, and a maintainer
  who is not its author must approve it as code owner (`.github/CODEOWNERS`).
  GitHub enforces both for everyone except the maintainers: either may merge a
  pull request without that approval, and the repository admin may also commit
  directly; that exception is used for docs, version bumps, release prep, and
  small data or string changes. Either way, the diff that entered a release
  is public and dated.
- Only the repository admin can create, move or delete a release tag, and the
  release secrets open only after the Approver approves the run.
- Releases published from 2026-11-04 on are immutable on GitHub: their tag and
  assets cannot be changed afterwards.
- The release scripts refuse a dirty working tree. The dictionary artifacts are
  committed to the repository; the engine binaries are not (they are generated
  and gitignored), so every release rebuilds them from the released commit.
- Each release's artifact digest is published in the update manifest and read
  back from the served file, so it attests what the download URL actually
  serves rather than what was built locally.

Stated plainly because it bears on provenance: Windows installers are built
from the public source repository on a GitHub-hosted runner
(`.github/workflows/windows-build.yml`), not on a maintainer's machine;
`docs/architecture/windows-release.md` § Building on a GitHub-hosted runner
owns that procedure.

This table changes whenever a maintainer joins or leaves.

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

Third-party Rust dependencies are statically linked into those binaries and are
not separately signed. The installer also carries Microsoft's Windows App Runtime
and WebView2 projection beside the settings executable; they are Microsoft's own
binaries, and this project does not sign them (`THIRD_PARTY_LICENSES.md`).
Bundled data files — fonts, the compiled dictionary — are not executable and are
not signed.

Nothing built from another project's source is ever signed with this project's
certificate.

## What the software does

TaigiKeyboard is an input method. It converts what the user types into
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
- Learned data — word frequency, word association, learned phrases, custom
  dictionary — stays in app-private storage and is excluded from OS automatic
  backup.

It contains no vulnerability scanning, no exploitation capability, and no
remote administration feature.

What the Windows installer changes on the system, all of it required for an
input method to work or to be updated, and all of it reversed on uninstall:

| Change | Why |
| --- | --- |
| Registers `TaigiKeyboard32.dll` and the machine's native 64-bit service (`TaigiKeyboard.dll` on x64; `TaigiKeyboardArm64.dll` on Arm64, which names the `TaigiKeyboard64X.dll` forwarder) as a TSF text service (`regsvr32`, per architecture) | This is what makes the keyboard selectable at all. |
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

## Why Windows releases are unsigned

An Authenticode certificate costs a few hundred US dollars a year, so signing
goes through SignPath Foundation's free programme for open-source projects.
Until a certificate is issued, this page describes what a user can verify
instead.

## Licence

Source code: Apache License, Version 2.0 — see `LICENSE`.
Third-party components: `THIRD_PARTY_LICENSES.md`.
Dictionary data: `dictionary/LICENSE` — **not** Apache-2.0, and several sources
carry non-commercial terms.
