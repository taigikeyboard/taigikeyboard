# Code signing policy

Who may release a signed Taigi Keyboard binary, what gets signed, and what a
user can verify for themselves.

## Current signing status

| Platform | Artifact | Status |
| --- | --- | --- |
| Windows | `TaigiKeyboard-<version>.exe` (Inno Setup installer), `taigi_windows_tsf.dll`, `TaigiKeyboardSettings.exe` | **Unsigned.** Integrity rests on the SHA-256 digest published in the update manifest. |
| macOS | `TaigiKeyboard-<version>.pkg` | Signed and notarized with an Apple Developer ID. |
| iOS / Android | App Store / Google Play builds | Signed by the respective store pipeline. |

Windows releases being unsigned is the gap this policy exists to close.
`docs/architecture/windows-release.md` § Signing status owns what the digest
is and is not worth in the meantime.

## Roles

Taigi Keyboard is maintained by one person, who therefore holds every role:

| Role | Who |
| --- | --- |
| Author (requests a signing operation) | Soo Bîn-hiân 蘇民弦 — <https://github.com/siansiansu> |
| Reviewer (reviews the artifact and its provenance) | Soo Bîn-hiân 蘇民弦 |
| Approver (approves the signing request) | Soo Bîn-hiân 蘇民弦 |

The team that develops and maintains this software is the same team that
requests signing, and owns <https://github.com/taigikeyboard/taigikeyboard>.
Only artifacts built from that repository are ever submitted for signing.

Multi-factor authentication is required on the GitHub account and on any code
signing service account used for this project.

If a second maintainer joins, this table is updated in the same commit that
grants them access.

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
  downloading the package it names. Neither request carries an identifier.
- Learned data — word frequency, word association, custom dictionary — stays
  in app-private storage and is excluded from OS automatic backup.

It contains no vulnerability scanning, no exploitation capability, and no
remote administration feature. It changes no system configuration beyond
registering itself as a keyboard or text service, which is what the user
installed it to do.

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

Once Windows releases are signed, the in-app updater additionally verifies the
downloaded package's Authenticode signature before offering to install it —
that path already exists in `windows/crates/taigi-windows-update/src/verify.rs`
and activates as soon as a running copy carries a signature of its own.

## Attribution

This section is filled in when a signing certificate is in place; it is stated
here so the policy is complete rather than to imply a relationship that does
not yet exist.

> Free code signing provided by [SignPath.io](https://signpath.io), certificate
> by [SignPath Foundation](https://signpath.org).

## Licence

Source code: Apache License, Version 2.0 — see `LICENSE`.
Third-party components: `THIRD_PARTY_LICENSES.md`.
Dictionary data: `dictionary/LICENSE` — **not** Apache-2.0, and several
sources are unresolved.
