# Windows release — installed by Inno Setup, web-distributed

The operator procedure for cutting a Windows release (roadmap
`windows-roadmap.md` W8 / W9). Mirror of `macos-release.md`: the same order,
the same GitHub release — one per desktop version, holding both platforms'
installers — the same manifest contract, with Authenticode in place of
Developer ID + notarization and an Inno Setup installer in place of a product
archive.

## Signing status — UNSIGNED is the current channel

There is no code-signing certificate for this project and none is expected for
the next year or two (owner's decision, 2026-09-04). Releases are cut and
published with `--skip-sign`, and the published artifact carries the plain
release name — `TaigiKeyboard-<version>.exe`, the same shape macOS publishes.

What that costs, and what it does not (SmartScreen behaviour per
[Microsoft's own account](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation),
read 2026-09-04):

| | Unsigned (today) | Signed (when a certificate exists) |
|---|---|---|
| Download | "Windows protected your PC" — 其他資訊 → 仍要執行. Reputation restarts from zero **every version**: an unsigned file cannot inherit any. Enterprise policy can remove the "run anyway" choice | Also warns while the file is new — an OV or EV certificate does NOT buy a bypass, and has not for years — but reputation can then carry across versions on the same certificate |
| Win11 Smart App Control | Blocks execution outright unless the file has positive reputation | Same rule, but signing is how reputation accrues |
| Install | Works; a TSF text service needs no signature | Same |
| Update check, toast, 一般 pane row | Work | Work |
| Update action | 下載安裝 → 安裝, in-app — admitted by the manifest's `packageSHA256` | Same, and the package's Authenticode signature is checked as well |

The in-app install is NOT gated by the running copy's signature. What admits
a package is `taigi-windows-update::verify::admit`: the published SHA-256
always, plus — only when `running_identity` answers `Some` — the signer
thumbprint and VERSIONINFO checks. `can_install_in_app` therefore wants a
`packageURL`, a `packageSHA256` and a `%LOCALAPPDATA%` staging folder, and
nothing else.

The digest is a publishing check, not a defence: the manifest and the asset
are published from one account, so it catches a truncated download, a wrong
or re-uploaded asset, or a URL pointing at another valid executable — not an
adversary holding that account. That is the same security position as the
user downloading the installer in a browser, which is what the alternative
would be; what it buys is that nothing hands somebody an executable it never
checked. Sole maintainer with 2FA, owner's judgement, 2026-09-04.

⚠ A file this app downloads carries no Mark-of-the-Web (browsers write that;
we do not), so launching it should not raise the browser-download SmartScreen
prompt. Win11 Smart App Control still applies to any unsigned executable.
Dogfood item, not an assertion.

Going unsigned → signed is seamless: an unsigned copy installs the first
SIGNED release in-app too, because the digest is what it checks and the digest
is published either way. What that release's own copies gain is the extra
Authenticode check on everything after it — which is also why the reverse is
NOT seamless: a signed copy offered an unsigned package rejects it
(`Untrusted`) and falls back to the download page.

Returning to signed: set `WINDOWS_SIGNING_THUMBPRINT`, stop passing
`--skip-sign`. No code changes; the two flags are the whole switch.

**Named divergence from macOS**: `macos/scripts/release-app.sh` still refuses
`--publish --skip-notarize` and stamps `-unnotarized` into a name, because
macOS HAS a Developer ID certificate. Windows deliberately no longer mirrors
that half.

## Architectures

**x64 only.** A text service is loaded into every process that takes text
input and the DLL must match THAT process's architecture, so this is two
separate questions — which machines can install, and which applications the
input method works in.

`ArchitecturesAllowed=x64os`, so an **Arm64 machine is refused**.
`x64compatible` — what this used to say — also matches Arm64 Windows 11,
which runs x64 binaries under emulation: the installer would succeed and the
input method would then do nothing in every Arm64-native application while
working in emulated ones. For an input method that is indistinguishable from
broken.

**32-bit applications have no input method**, and that is a deliberate gap
(USER 2026-09-01: revisit when a user reports it). Everywhere Taiwanese is
typically typed is 64-bit today — browsers, Notepad, the chat clients. The
known exception is Office 2016 and earlier, and any Microsoft 365 installed
before January 2019, which defaulted to 32-bit and stays 32-bit until it is
reinstalled.

### What a 32-bit or Arm64 round would have to know

Both were investigated on 2026-09-01; this is the measured record, so it does
not have to be re-derived.

**32-bit** is a working build, not a research problem. `cargo build --release
--target i686-pc-windows-msvc -p taigi-windows-tsf` succeeds after one fix:
on 32-bit the `windows` crate maps `SetWindowLongPtrW` onto `SetWindowLongW`,
whose value parameter is an `i32`, not an `isize` (the only error in the whole
graph, `ui/window.rs`). The resulting DLL is machine `14C`, exports the four
entry points undecorated, and imports no C runtime. It must be installed
BESIDE the 64-bit one under its own name, not in an `x86\` subdirectory: a
service resolves `Dictionaries\`, `Fonts\` and the settings exe from its OWN
directory (`module::install_directory`).

Registration, measured with `regsvr32` and a registry dump:

- `CLSID\{…}\InprocServer32` is per-architecture — the 64-bit view and the
  WOW6432Node view hold different paths, and registering one does not disturb
  the other. Both services use the SAME CLSID, which is what Microsoft's TSF
  guidance asks for: one logical input method, one entry in the language list.
- `HKLM\SOFTWARE\Microsoft\CTF\TIP\{…}` — the profile, its description and
  icon, and the four categories — is SHARED, and mirrored into both views.
  The **last** registration wins: registering the 32-bit service after the
  64-bit one left the profile's description naming the 32-bit DLL. So the
  order has to be 32-bit first, 64-bit last.
- Unregistering EITHER architecture removes that shared profile and all four
  categories from BOTH views while the other's `InprocServer32` stays — an
  input method registered with COM and absent from the language list. The
  pair is one transaction, and a rollback has to back up and restore both.

**Arm64** is a research problem. A single CLSID's `InprocServer32` is one
path, and Arm64-native and x64-emulated processes read the SAME 64-bit
registry view, so there is no registry-level way to serve both: Microsoft's
answer for a 64-bit in-process COM server is an **Arm64X pure forwarder** — a
code-less Arm64X DLL that the loader redirects to an Arm64 or an x64 DLL
(`link /dll /noentry /machine:arm64x /defArm64Native:…`). It also needs the VS
Arm64/Arm64EC build tools, which the release machine does not have
(`bin/Hostx64` holds `x64` and `x86` only), and a machine to test on.

## Building on a GitHub-hosted runner

`.github/workflows/windows-build.yml` builds the same installer on a
GitHub-hosted `windows-2025` runner — `bash windows/scripts/release-app.sh
--skip-sign`, the same entry point, with Inno Setup and `protoc` installed from
version- and digest-pinned downloads and the MSVC developer environment applied.

It exists for provenance, not convenience. SignPath Foundation signs only "a
valid, automated build resulting from the source code at the noted source code
repository", and for open-source projects requires every job leading up to the
signing request to have run on a GitHub-hosted agent, with the artifact handed
to its action from inside that workflow. A build cut by hand on this machine can
never satisfy that, however carefully it is done.

The workflow runs on `release: published` — the manual publish of the draft,
which is also what creates the tag. It does not publish anything itself, and it
does **not** vouch for the installer that was staged from the maintainer's box:
that is the point SignPath changes. When a certificate exists the order has to
invert — hosted build and sign first, then stage the signed installer on the
draft — because signing after the release is published is too late.

This section's manual procedure is still how releases are cut. What has to happen before that changes is a certificate; the
repository went public on 2026-09-07, which is what let the releases move back
here from the website repository. Note also that SignPath signs an Inno Setup installer as a plain PE
file, not as a composite — signing the binaries *inside* it is a separate
signing operation before packaging, which is the shape
`windows/scripts/release-app.sh` already has.

## One-time machine setup

A Windows machine (or VM) with Git Bash (`cygpath`, `sha256sum`, and — on the
publishing path only — `base64` and `curl`), PowerShell (reads the built files'
VERSIONINFO and signature back), Python 3 (publishing only: it validates the
manifest JSON), GNU **make** (`winget install ezwinports.make` — Git for Windows
does not ship it, and every `make` target below needs it), `protoc` (the engine's
`protos` crate runs prost-build), and:

1. **Rust** with the `x86_64-pc-windows-msvc` target (`rustup target add`).
2. **Inno Setup 6.5 or newer** (`x64compatible` is 6.3 syntax) — `ISCC.exe` on
   `PATH`, in its default folder, or named by `ISCC=<path>`. Note that
   `release-app.sh` resolves `iscc` from `PATH` **before** it reads `ISCC`.
   Nothing has to be added to the Inno installation: `ChineseTraditional.isl` is
   still an unofficial translation as of 6.7.3 and a stock install does not
   contain it, so it is vendored at `windows/installer/Languages/`.
3. **Windows SDK + Visual Studio Build Tools** — `rc.exe` (the resource
   compiler the crates' build scripts use for the icon + VERSIONINFO) and
   `dumpbin.exe` (beside `link.exe`; the import-table check) on `PATH`, plus
   `signtool.exe` **on the signed path only** (neither script asks for it
   under `--skip-sign` / `--allow-unsigned`); a Developer Command Prompt puts
   all three there.
4. **A code-signing certificate** — *not held today; skip this item and pass
   `--skip-sign`* (§ Signing status). When one exists: in the current user's
   certificate store, named by its SHA-1 thumbprint,
   `export WINDOWS_SIGNING_THUMBPRINT=<40 hex>`. The updater pins THIS
   certificate: every installed copy accepts an in-app update only from an
   installer signed with the same leaf (see *Notes*). `TIMESTAMP_URL`
   overrides the RFC 3161 server (default DigiCert). `signtool.exe` in item 3
   is needed only on this path.
5. **`gh`** authenticated to an account that can write this repository (the
   release) and the website repository, `taigikeyboard/taigikeyboard.github.io`
   (the download button's data file).
6. **A clone whose `origin` is this repository, with HEAD committed clean and
   already pushed.** Publishing tags the commit the box is sitting on, so it
   refuses a dirty tree and a HEAD that is not an ancestor of `origin/main` —
   the release clone on the box is what § Cutting a release below assumes, and it must have fetched the commit being
   released rather than a local-only one.

## Cutting a release

```sh
make build              # only when engine/ or dictionary/ sources moved
make version-desktop 3.7.0   # macOS + Windows together, at the repository root
make windows-release RELEASE_FLAGS=--skip-sign   # from Git Bash, on the Windows machine
```

That stages the installer on a draft release nobody can reach. Testing,
publishing and announcing are in § Staging and publishing the installer below.

`make windows-release` does not rebuild the engine or the dictionary: both are
pre-built artifacts committed to the repository (`CLAUDE.md` § stale-binary
gate), and a release from a clean tree ships exactly what is committed.

What `windows/scripts/release-app.sh` does, in order:

1. **Preflight** — `MAJOR.MINOR.PATCH` in `windows/Cargo.toml`, clean working
   tree, the tools above present, a thumbprint named (or `--skip-sign`, which
   is what today's releases pass).
2. **Build** — `TAIGI_REQUIRE_RESOURCES=1 cargo build --release --target
   x86_64-pc-windows-msvc` for the DLL and the settings exe; each crate's
   `build.rs` compiles its icon and VERSIONINFO (`ProductName` = `Taigi
   Keyboard`, `ProductVersion` = the workspace version) into the binary, and
   under that variable a missing or failing resource compiler fails the
   build (no `windres` fallback on the MSVC target).
3. **Read back** — PowerShell reads each binary's `ProductName` /
   `ProductVersion` and the script compares them to the checkout (the
   updater's package check reads the same block); `dumpbin /dependents`
   proves neither binary imports `vcruntime*.dll` / `msvcp*.dll`
   (`+crt-static`, roadmap W14), and that the DLL imports nothing of WinUI /
   the Windows App Runtime (W17 — only the settings exe may).
4. **Stage** — `windows/.build/staging/`: the DLL, the exe, `Runtime\` (the
   Windows App Runtime files the settings exe's build script staged in the
   target directory — every name in
   `windows/build-support/windows-app-runtime-files.txt`, W17), `Dictionaries\`
   (the four artifacts from the repo-root `dictionaries`, each required
   non-empty), `Fonts\` (every face in the repo-root `fonts/font`), and the
   scheduled task's definition.
5. **Sign the binaries** — `signtool sign /fd SHA256 /td SHA256 /tr <timestamp>
   /sha1 <thumbprint>`, then `signtool verify /pa`. Skipped entirely under
   `--skip-sign`.
6. **Package** — `iscc /DAppVersion /DDist /O … windows/installer/TaigiKeyboard.iss`
   → `TaigiKeyboard-<version>.exe`, with the same `ProductName` /
   `ProductVersion` in its own VERSIONINFO (`VersionInfo*` directives) —
   read back and compared the same way.
7. **Sign the installer** — same certificate, then verify. Skipped entirely
   under `--skip-sign`.
8. **Name the output** — `windows/.build/distribution/TaigiKeyboard-<version>.exe`
   and its SHA-256. Only a DIRTY tree stamps a qualifier (`-dirty`) into the
   name, so a throwaway cannot be mistaken for a release; an unsigned
   installer is publishable and keeps the plain name.
9. **Publish** — `windows/scripts/publish-release.sh` (below), with
   `--allow-unsigned` passed down when the build was `--skip-sign`.

Flags: `make windows-release` passes `--force --publish`. `--publish` still
refuses `--allow-dirty` (a published installer must be reproducible from a
commit). Today's release adds `--skip-sign`:

```sh
make windows-release RELEASE_FLAGS=--skip-sign       # the unsigned release channel
bash windows/scripts/release-app.sh --skip-sign      # package only, no publish
bash windows/scripts/release-app.sh --allow-dirty    # build from a dirty tree
```

## Versioning

One number for the desktop train — macOS and Windows share it; iOS and
Android are the separately numbered mobile train — written by
`make version-desktop x.y.z` (`tools/release_notes.py set-versions --train
desktop`): `windows/Cargo.toml`
`[workspace.package] version` is the Windows source of truth; every crate
inherits it, the VERSIONINFO blocks and the installer read it, and the
update manifest announces it. The build number the macOS package carries
(`MAJOR*10000 + MINOR*100 + PATCH`) is also the dictionary stamp the Windows
engine caches under (`dictionary_artifacts::dictionary_version`).

## What the installer does

Administrator, `%ProgramFiles%\TaigiKeyboard`: a text service is loaded into
every process of every user and `regsvr32` writes HKLM. In order:

- **Before copying** — stops `TaigiKeyboardSettings.exe`, unregisters the
  installed DLL (so new processes stop loading it), and probes whether the
  old DLL can be renamed. A host still holding it stops the install with the
  one recipe that works: switch input method, sign out, sign in, run the
  installer again (no restart).
- **Languages** — the macOS bundle's system localizations, no more and no
  fewer: Traditional Chinese (Hanji, listed first = the fallback when the
  user's UI language is none of them, the app's own default), English,
  Japanese. Inno picks the UI language automatically. Its own strings
  (`installerDllLocked` / `installerStepFailed` / `installerSignOutNote` /
  `installerUpdateTaskSkippedNote`) are
  `desktop.installer*` keys in `i18n/desktop.json`, emitted by `make i18n`
  into `windows/installer/Messages.iss` — the installer is never edited for
  wording. Tâi-lô / POJ have no Inno base language (the macOS Installer
  cannot show them either); the app itself offers all five.
- **Files** — `TaigiKeyboard.dll` (+ `x86\TaigiKeyboard.dll` when staged),
  `TaigiKeyboardSettings.exe` with the Windows App Runtime files beside it
  (W17), `Dictionaries\`, `Fonts\`, `update-check-task.xml`.
- **Registration** — `regsvr32 /s` on the DLL (the DLL's `DllRegisterServer`
  registers the CLSID, the `0x0404` profile, the categories and the
  display-attribute provider — roadmap W7). Run from `[Code]`, not `[Run]`:
  Inno ignores a `[Run]` entry's exit code, so a registration that fails is
  checked explicitly and treated as the install failing — the previous DLL
  (backed up before it was unregistered) is put back and re-registered, and
  Setup raises, which reports the error and rolls back its file installation.
  The scheduled task is deliberately NOT in that tier: see below.
- **Start menu** — a shortcut to the settings exe carrying
  `System.AppUserModel.ID = TaigiKeyboard.Settings`, which is what lets the
  updater's toast be shown at all.
- **Scheduled task** — `TaigiKeyboard Update Check`, created from
  `update-check-task.xml` with the exe's path filled in: five minutes after
  logon, then daily, running `TaigiKeyboardSettings.exe --check-updates`;
  `IgnoreNew` for parallel instances. Created from Setup's own ELEVATED
  context — the Task Scheduler root folder admits no other integrity level,
  and an unelevated `schtasks /Create` answers `ERROR: Access is denied.`,
  which is what failed every install before 2026-09-04. The definition names
  no principal user, so the task binds to the account Setup runs as (ordinary
  UAC consent keeps that the user's own) and `RunLevel` keeps it
  least-privileged. Its creation failing is NOT an install failure: the input
  method works without it and the settings window checks on demand, so the
  failure is logged and the finished page carries
  `installerUpdateTaskSkippedNote`.
- **Uninstall** — the reverse, item by item; `%APPDATA%\TaigiKeyboard` (the
  settings, the learning data, the custom dictionary) stays. The scheduled
  task is deleted by the elevated uninstaller, which reaches the root folder
  whoever registered it.

The installer never launches the settings window itself: the window removes
stale staged update packages at launch, and the installer may still be
reading its own payload.

## Staging and publishing the installer

A desktop release happens in two halves with a manual test between them, and
nothing reaches a user until a person publishes it. The full table is in
`macos-release.md` § Publishing the package; the Windows-side steps are:

```sh
make windows-release RELEASE_FLAGS=--skip-sign   # stages the .exe on the draft
```

Then, once the Mac has staged its package too:

```sh
gh release download desktop-<version> --repo taigikeyboard/taigikeyboard --dir ~/Downloads
# install it, use it, check SmartScreen behaviour on a machine that has never seen it
gh release edit desktop-<version> --repo taigikeyboard/taigikeyboard --draft=false
make desktop-announce
```

`windows/scripts/publish-release.sh` (run by `--publish`):

1. Checks the shared preconditions before touching the installer — `gh` present
   and authenticated, a dotted-integer version, a clean tree, HEAD pushed, and
   `changelog/desktop-v<version>.md` committed — so a mistake costs a second
   rather than a signtool round trip.
2. Refuses a `-dirty` name, an installer whose VERSIONINFO is not
   `TaigiKeyboard` / the checkout's version, and — when
   `WINDOWS_SIGNING_THUMBPRINT` is set — a signer other than that certificate
   (what a signed installed copy pins). The Authenticode gate (`signtool verify`
   must trust the installer) stands unless `--allow-unsigned` is passed, which
   is what `release-app.sh --skip-sign --publish` passes down; naming a
   certificate AND `--allow-unsigned` is a contradiction and fails. A direct
   invocation without the flag therefore cannot publish unsigned by accident.
3. Creates (or attaches to) the **draft** release `desktop-<version>` in this
   repository — the same draft the macOS package goes on — recording the commit
   this checkout is at as what publishing will tag, with the whole
   `changelog/desktop-v<version>.md` as the notes, read out of that commit
   rather than the working tree. When macOS staged first the draft already
   exists and the installer is added to it; either way an existing tag must
   dereference to this same commit and an existing draft must already target it,
   and a release that has already been published is refused (an asset added to
   it would be public immediately). A staged asset is never replaced: an
   identical one is verified in place, and one whose bytes differ stops the run.
4. Uploads `TaigiKeyboard-<version>.exe.sha256` beside the installer and reads
   the installer back — authenticated, since a draft has no anonymous URL —
   requiring its SHA-256 to equal the local file's. That receipt is what the
   announcement holds the published bytes against, and it is also what a user
   can check a manual download with, which matters on an unsigned channel.

`scripts/announce-release.sh` (`make desktop-announce`), after the manual
publish, is what writes `_data/windows_release.json` — now carrying `sha256` —
and waits until `https://taigikeyboard.tw/appcast/windows.json` serves the new
version, its installer URL **and** that digest. The manifest every installed
copy polls is rendered from that data file by the site's own build
(`windows/updates/README.md` § One published fact, one committed file), so the
poll is also what proves the site built what was committed. It runs on either
machine — everything it needs is on the release — and both platforms' data files
go in one website commit.

Re-running either half after a failure adds to what is there rather than tearing
it down; the site data is written only after the download is provably reachable.
The site advertises the download only once its `enable_windows_download` flag is
on — the data file alone does not.

## Notes

- **Certificate rotation.** The updater pins the signer's LEAF thumbprint
  (roadmap W9, PR9 Codex): the first release signed with a renewed
  certificate is not accepted in-app by the copies signed with the old one.
  Plan that release as a download-page release (the manifest's
  `downloadPageURL` route, which every copy has); the copies it installs pin
  the new certificate and in-app updates resume.
- **Unsigned releases** (`--skip-sign`, today's channel — § Signing status)
  run and install — SmartScreen warns — and DO offer the in-app install: the
  running settings exe has no signature identity, so the manifest's
  `packageSHA256` is what admits the download instead. This is where Windows
  parts from the Mac, which has no digest field and sends an ad-hoc build to
  the download page.
- **`rc.exe` absent** — a development build compiles, with a warning, without
  its icon and VERSIONINFO; the release script sets `TAIGI_REQUIRE_RESOURCES=1`,
  under which the build fails instead, and reads the VERSIONINFO back
  afterwards regardless — the updater's package check depends on it.
- **First real-Windows run (PR11 smoke)** — `schtasks /Create /XML` from the
  elevated install, then `schtasks /Query` for the task's "Run As User"; an
  install with a host still holding the DLL (expect the sign-out recipe, not
  a partial install); the rollback path (make `regsvr32` fail on purpose:
  expect the old DLL back and Setup reporting failure); the toast under the
  Start-menu AUMID; `dumpbin` and PowerShell present in the Git Bash `PATH`
  on the release machine. The `schtasks` and PATH items ran 2026-09-01 through
  2026-09-04; the rest is still unexercised.
