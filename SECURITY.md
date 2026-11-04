# Security policy

## Reporting a vulnerability

Report it privately through GitHub —
[Security → Report a vulnerability](https://github.com/taigikeyboard/taigikeyboard/security/advisories/new)
— or email **<info@taigikeyboard.tw>** with `SECURITY` in the subject. Please
do not open a public issue for a vulnerability.

Include what you have: affected platform and version, what an attacker can do,
and the smallest reproduction you can manage. A report without a working
reproduction is still worth sending.

This project has two maintainers. Expect an acknowledgement within a week, and
a fix timeline with it. Please give a reasonable window before disclosing
publicly.

## Supported versions

Only the most recent release of each train receives fixes:

| Train | Platforms |
| --- | --- |
| mobile | iOS, Android |
| desktop | macOS, Windows, Linux |

The two trains carry independent version numbers.

## Scope

In scope: the keyboard extensions and input methods, the settings
applications, the shared Rust engine, the dictionary build pipeline, and the
release and update mechanisms (installer, update manifest, package
verification).

Out of scope: vulnerabilities in third-party dependencies that are already
public and awaiting an upstream fix — report those upstream; findings that
require physical access to an unlocked device; and the content of the
dictionary data itself.

## What this software does with user data

The keyboard runs entirely on-device. It makes no network request while
typing, sends no keystroke anywhere, and has no analytics or telemetry.

Four databases of learned data are stored in app-private storage — word
frequency, word association, learned phrases, and the custom dictionary. They
are deliberately excluded from OS automatic backup (iOS iCloud, Android Auto
Backup and device transfer), so they never leave the device unless the user
exports a `.taigi` backup file by hand; learned phrases are never exported.

The macOS and Windows applications make exactly one kind of network request:
fetching a static JSON update manifest from `taigikeyboard.tw`, and — only if
the user asks for it — downloading the package that manifest names. No
identifier is attached to either request. The Linux input method makes none;
it is updated through its package.
