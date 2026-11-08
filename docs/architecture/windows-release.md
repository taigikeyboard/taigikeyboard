# Windows release — installed by Inno Setup, web-distributed

The Windows half of a desktop release. Mirror of `macos-release.md`: the same
GitHub release — one per desktop version, holding every desktop platform's
installers ([`desktop-release.md`](desktop-release.md) owns the flow) — the same
manifest contract, with Authenticode in place of Developer ID + notarization and
an Inno Setup installer in place of a product archive.

## Signing status — UNSIGNED is the current channel

There is no code-signing certificate for this project yet (signing path:
§ Building on a GitHub-hosted runner). Until a certificate is issued, releases are cut and published with
`--skip-sign`, and the published artifact carries the plain
release name — `TaigiKeyboard-<version>.exe`, the same shape macOS publishes.

What that costs, and what it does not (SmartScreen behaviour per
[Microsoft's own account](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation),
read 2026-09-04):

| | Unsigned (today) | Signed (when a certificate exists) |
|---|---|---|
| Download | "Windows protected your PC" — More info → Run anyway. Reputation restarts from zero **every version**: an unsigned file cannot inherit any. Enterprise policy can remove the "run anyway" choice | Also warns while the file is new — an OV or EV certificate does NOT buy a bypass, and has not for years — but reputation can then carry across versions on the same certificate |
| Win11 Smart App Control | Blocks execution outright unless the file has positive reputation | Same rule, but signing is how reputation accrues |
| Install | Works; a TSF text service needs no signature | Same |
| Update check, toast, General pane row | Work | Work |
| Update action | Download and Install → Install, in-app — admitted by the manifest's `packageSHA256` | Same, and the package's Authenticode signature is checked as well |

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
checked. The publishing account is the sole maintainer's, with 2FA.

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

**x64 and Arm64 Windows 11 machines; Arm64, x64 and 32-bit applications.** A
text service is loaded into every process that takes text input and the DLL
must match THAT process's architecture, so this is two separate questions —
which machines can install, and which applications the input method works in.

One installer for both machines: `ArchitecturesAllowed=x64compatible`, which
matches x64 Windows and Arm64 Windows 11 (Arm64 Windows 10 has no x64
emulation, and Inno refuses it). An Arm64 machine gets the Arm64 service, the
x64 one for x64-emulated hosts and the Arm64X forwarder between them, plus the
Arm64 settings exe — § The Arm64 services.

**32-bit applications get `TaigiKeyboard32.dll`**, the same text service
built for `i686-pc-windows-msvc`. Everywhere Taiwanese is typically typed is
64-bit today — browsers, Notepad, the chat clients — except Office: Office
2016 and earlier, and any Microsoft 365 installed before January 2019,
defaulted to 32-bit and stays 32-bit until it is reinstalled.

### The 32-bit service

Measured, so it does not have to be re-derived.

- **Build**: `release-app.sh` builds `--target i686-pc-windows-msvc -p
  taigi-windows-tsf` after the x64 build — the DLL only; the 64-bit settings
  exe serves both. The one source change it needed: the i686 `windows` crate
  maps `SetWindowLongPtrW` onto `SetWindowLongW`, whose value is an `i32`, so
  `ui/window.rs` casts with `as _`. `check-box.sh` type-checks the same target so a
  pointer-width mistake fails the box gate rather than a release. The DLL is
  machine `14C`, exports the four entry points undecorated, imports no C
  runtime, and goes through every read-back check the x64 DLL does.
- **Layout**: BESIDE the 64-bit DLL under its own name (mozc ships
  `mozc_tip32.dll` + `mozc_tip64.dll` the same way,
  `installer_oss_64bit.wxs:273-277`), not in an `x86\` subdirectory: a service
  resolves `Dictionaries\`, `Fonts\` and the settings exe from its OWN
  directory (`module::install_directory`). Staging renames cargo's
  `TaigiKeyboard.dll`; `build.rs` writes the matching VERSIONINFO
  `OriginalFilename`.
- **Not in the dev install**: `install-dev.ps1` registers the 64-bit build
  tree in place. A 32-bit host is tested with an installer from
  `release-app.sh --skip-sign --allow-dirty`.

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
  The installer does: `RegisterEverything` and `RestorePreviousVersion`
  register x86 then the native 64-bit service (`NativeDll`: x64, or Arm64 on
  an Arm64 machine), and every unregister path (`PrepareToInstall`,
  `RollBack`, `[UninstallRun]`) removes both. A restore re-registers only the
  services the replaced version had (`HadNativeDll` / `HadX86Dll`, taken before
  anything moves): a failed fresh install registers nothing, and a failed
  upgrade from a version without the 32-bit service also removes the new
  `TaigiKeyboard32.dll` (`RemoveAddedX86Dll`), so no mixed install is left
  behind.
- **Downgrade to a version without the 32-bit service: uninstall first.** The
  older installer knows nothing of `TaigiKeyboard32.dll`: its `MakeWayForTree`
  moves the file aside with the rest of the payload, but it never unregisters
  it, so WOW6432Node keeps a CLSID pointing at a deleted DLL and 32-bit hosts
  fail to activate it. Uninstalling first (which keeps `%APPDATA%`) leaves
  nothing for it to miss.

### The Arm64 services

Issue #721; design, measurements and the spike record:
`windows-arm64-roadmap.md`.

- **Why a forwarder**: one CLSID has one 64-bit `InprocServer32`, and
  Arm64-native and x64-emulated processes read the same 64-bit registry view.
  `TaigiKeyboard64X.dll` is an Arm64X pure forwarder (`windows/forwarder/`,
  linked by `windows/scripts/build-forwarder.ps1`, mozc's recipe): its Arm64
  view forwards `DllGetClassObject` / `DllCanUnloadNow` to
  `TaigiKeyboardArm64.dll`, its x64 view to `TaigiKeyboard.dll`.
- **Registration**: the Arm64 build of `register_server` writes the sibling
  forwarder as `InprocServer32` and refuses to register when it is missing;
  the profile icon and display name stay on the Arm64 DLL. The x64 DLL is
  never registered on an Arm64 machine. Order as above: x86 first, then the
  Arm64 DLL with the native `{sys}\regsvr32`.
- **Layout**: all flat in `{app}` under fixed names — the installer picks
  `arm64\TaigiKeyboardSettings.exe` and `RuntimeArm64\*` with `Check: IsArm64`
  under the same install names, so the shortcut, uninstall icon, update task
  and updater see one exe either way.
- **Build**: `release-app.sh` cross-builds `aarch64-pc-windows-msvc` (DLL +
  settings exe) after x64 and i686 and links the forwarder; read-back checks
  the Arm64X marker, both views' forwards, that the forwarder imports
  nothing, and the PE machine of every staged binary and runtime DLL
  (windows-reactor-setup's runtime extraction is shared across
  architectures).
- **CI**: `windows-build.yml` runs the installer on a `windows-11-arm` runner
  (install, activation from an Arm64 and an x64 process through the
  forwarder, uninstall) and, as an upgrade from the last release, on
  `windows-2025` (`windows/scripts/smoke-installer.ps1`,
  `smoke-activation.ps1`).
- **Dev install** on an Arm64 machine: `install-dev.ps1` stages the three
  DLLs and the Arm64 settings exe under `windows\target\arm64x-dev` and
  registers that copy (`windows/README.md`).

## Building on a GitHub-hosted runner

`.github/workflows/windows-build.yml` builds the release installer on a
GitHub-hosted `windows-2025` runner — `bash windows/scripts/release-app.sh
--skip-sign`, the same entry point, with Inno Setup, `protoc` and the Windows App
Runtime packages taken from version- and digest-pinned downloads, the compiler
from the version `windows/rust-toolchain.toml` names, and the MSVC developer
environment applied.
`make desktop-release` dispatches it on `main`, and that run attaches its
installer to the draft — in a separate `attach` job, the only one holding a
write token, after the installer smoke jobs passed: a fresh install on a
`windows-11-arm` runner, activated from an Arm64 and an x64 process, and an
upgrade from the last release on `windows-2025`, each uninstalled clean
(`windows/scripts/smoke-installer.ps1`, `smoke-activation.ps1`). A run
dispatched on any other branch
builds and smoke-tests and attaches nothing; a run fired by `release:
published` rebuilds for provenance and publishes nothing.

It exists for provenance, not convenience. SignPath Foundation signs only "a
valid, automated build resulting from the source code at the noted source code
repository", and for open-source projects requires every job leading up to the
signing request to have run on a GitHub-hosted agent, with the artifact handed
to its action from inside that workflow. A build cut by hand on a Windows
machine can never satisfy that.

When a certificate exists, signing goes into the dispatched run before its
attach step — signing after the release is published is too late. SignPath
signs an Inno Setup installer as a plain PE file, not as a composite: signing
the binaries *inside* it is a separate signing operation before packaging,
which is the shape `windows/scripts/release-app.sh` already has.

## One-time machine setup

A Windows machine (or VM) with Git Bash (`cygpath`, `sha256sum`, and — on the
publishing path only — `base64` and `curl`), PowerShell (reads the built files'
VERSIONINFO and signature back), Python 3 (publishing only: it validates the
manifest JSON), GNU **make** (`winget install ezwinports.make` — Git for Windows
does not ship it, and every `make` target below needs it), `protoc` (the engine's
`protos` crate runs prost-build), and:

1. **Rust** with the `x86_64-pc-windows-msvc`, `i686-pc-windows-msvc` and
   `aarch64-pc-windows-msvc` targets (`rustup target add`).
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
   all three there. The Visual Studio component **MSVC ARM64/ARM64EC build
   tools** too: the Arm64 build's bundled SQLite and the Arm64X forwarder
   need it.
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

A release does not build here at all: `make desktop-release` on the Mac
dispatches `.github/workflows/windows-build.yml`, which builds the installer on
a GitHub-hosted runner and attaches it to the same draft the macOS package is
staged on (`desktop-release.md`). `make windows-release` remains what a person
runs on a Windows machine to produce an installer by hand — for a dev loop, or
when CI cannot.

`make windows-release` does not rebuild the engine, and does not need to: the
TSF service and the settings exe compile the engine crates from source through
cargo — the protos included, by prost-build in `engine/protos/build.rs` — so
they cannot link a stale copy the way the Mac's xcframework can.

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
   (the four artifacts from the repo-root `assets/dictionaries`, each required
   non-empty), `Fonts\` (every face in the repo-root `assets/fonts/font`), and the
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

`windows/Cargo.toml` `[workspace.package] version` is the Windows source of
truth: every crate inherits it, the VERSIONINFO blocks and the installer read
it, and the update manifest announces it. The engine caches the dictionary
under the same `MAJOR*10000 + MINOR*100 + PATCH` stamp the macOS package
carries (`dictionary_artifacts::dictionary_version`). Setting it:
[`manual-release-notes.md` § Set the version](manual-release-notes.md#set-the-version).

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
- **Files** — `TaigiKeyboard.dll` and `TaigiKeyboard32.dll` side by side,
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
  and an unelevated `schtasks /Create` answers `ERROR: Access is denied.`. The definition names
  no principal user, so the task binds to the account Setup runs as (ordinary
  UAC consent keeps that the user's own) and `RunLevel` keeps it
  least-privileged. Its creation failing is NOT an install failure: the input
  method works without it and the settings window checks on demand, so the
  failure is logged and the finished page carries
  `installerUpdateTaskSkippedNote`.
- **Uninstall** — the reverse, item by item; `%APPDATA%\TaigiKeyboard` (the
  settings, the learning data, the custom dictionary) stays. The scheduled
  task is deleted by the elevated uninstaller, which reaches the root folder
  whoever registered it. A payload file a running host still has mapped (the
  service DLLs in any app the input method was active in) cannot be deleted;
  the uninstaller renames it to a unique `.stale` name and queues that name for
  the next restart, then each directory after its contents, so `{app}` is gone
  after a reboot. The queued name is never the install name: a reinstall
  before the reboot writes the new DLL there, and a delete queued against the
  path would take it at boot (which is why `uninsrestartdelete` is not used).
  The CI smoke uninstalls while holding the 64-bit service mapped and checks
  exactly that.

The installer never launches the settings window itself: the window removes
stale staged update packages at launch, and the installer may still be
reading its own payload.

## Staging and publishing the installer

The flow — stage on a draft, test, publish, announce — is
[`desktop-release.md` § The flow](desktop-release.md#the-flow). The installer
reaches the draft from CI; when testing it, check SmartScreen behaviour on a
machine that has never seen it.

What `windows/scripts/publish-release.sh` asserts that no other platform can:

1. A `-dirty` name is refused, as is an installer whose VERSIONINFO is not
   `TaigiKeyboard` / the checkout's version, and — when
   `WINDOWS_SIGNING_THUMBPRINT` is set — a signer other than that certificate
   (what a signed installed copy pins). The Authenticode gate (`signtool verify`
   must trust the installer) stands unless `--allow-unsigned` is passed, which is
   what `release-app.sh --skip-sign --publish` passes down; naming a certificate
   AND `--allow-unsigned` is a contradiction and fails. A direct invocation
   without the flag therefore cannot publish unsigned by accident.
2. `TaigiKeyboard-<version>.exe.sha256` goes up beside the installer. On an
   unsigned channel that digest is what a user can check a manual download
   against, and it is what `_data/windows_release.json` publishes as
   `packageSHA256` — the field `taigi-windows-update::verify::admit` requires
   before it will install anything.

The site advertises the download only once its `enable_windows_download` flag is
on — the data file alone does not.

## Notes

- **Certificate rotation.** The updater pins the signer's LEAF thumbprint: the first release signed with a renewed
  certificate is not accepted in-app by the copies signed with the old one.
  Plan that release as a download-page release (the manifest's
  `downloadPageURL` route, which every copy has); the copies it installs pin
  the new certificate and in-app updates resume.
- **`rc.exe` absent** — a development build compiles, with a warning, without
  its icon and VERSIONINFO; the release script sets `TAIGI_REQUIRE_RESOURCES=1`,
  under which the build fails instead, and reads the VERSIONINFO back
  afterwards regardless — the updater's package check depends on it.
