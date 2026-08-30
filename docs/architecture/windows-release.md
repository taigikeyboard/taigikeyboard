# Windows release — signed, installed by Inno Setup, web-distributed

The operator procedure for cutting a Windows release (roadmap
`windows-roadmap.md` W8 / W9). Mirror of `macos-release.md`: the same order,
the same website repository, the same manifest contract — with Authenticode
in place of Developer ID + notarization, and an Inno Setup installer in place
of a product archive.

## Architectures

x64 only for v1. The 32-bit DLL (WOW64 hosts) and ARM64 are **prepared**
targets — listed in `rust-toolchain.toml`, not built by the release script —
and ship once x64 registration + uninstall are proven on a real machine (ARM64
also needs a native smoke test). The installer refuses a 32-bit Windows
(`ArchitecturesAllowed=x64compatible`) and, when an
`x86\` DLL is staged by hand, registers it with the 32-bit `regsvr32`
(`SysWOW64`).

## One-time machine setup

A Windows machine (or VM) with Git Bash (`cygpath`, `sha256sum`, `base64`,
`curl`), PowerShell (reads the built files' VERSIONINFO and signature back),
Python 3, GNU **make** (`winget install ezwinports.make` — Git for Windows does
not ship it, and every `make` target below needs it), and:

1. **Rust** with the `x86_64-pc-windows-msvc` target (`rustup target add`).
2. **Inno Setup 6.5 or newer** (`x64compatible` is 6.3 syntax; the official
   `ChineseTraditional.isl` ships from 6.5) — `ISCC.exe` on `PATH`, in its
   default folder, or named by `ISCC=<path>`.
3. **Windows SDK + Visual Studio Build Tools** — `rc.exe` (the resource
   compiler the crates' build scripts use for the icon + VERSIONINFO),
   `signtool.exe`, and `dumpbin.exe` (beside `link.exe`; the import-table
   check) on `PATH`; a Developer Command Prompt puts all three there.
4. **A code-signing certificate** in the current user's certificate store,
   named by its SHA-1 thumbprint: `export WINDOWS_SIGNING_THUMBPRINT=<40 hex>`.
   The updater pins THIS certificate: every installed copy accepts an in-app
   update only from an installer signed with the same leaf (see *Notes*).
   `TIMESTAMP_URL` overrides the RFC 3161 server (default DigiCert).
5. **`gh`** authenticated to an account that can write the website repository
   (`taigikeyboard/taigikeyboard.github.io`).

## Cutting a release

```sh
make build              # only when engine/ or dictionary/ sources moved
make version-desktop 3.7.0   # macOS + Windows together, at the repository root
make windows-release    # from Git Bash, on the Windows machine
```

`make windows-release` does not rebuild the engine or the dictionary: both are
pre-built artifacts committed to the repository (`CLAUDE.md` § stale-binary
gate), and a release from a clean tree ships exactly what is committed.

What `windows/scripts/release-app.sh` does, in order:

1. **Preflight** — `MAJOR.MINOR.PATCH` in `windows/Cargo.toml`, clean working
   tree, the tools above present, a thumbprint named (or `--skip-sign`).
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
   (`+crt-static`, roadmap W14).
4. **Stage** — `windows/.build/staging/`: the DLL, the exe, `Dictionaries\`
   (the four artifacts from `ios/Resources/Dictionaries`, each required
   non-empty), `Fonts\` (every face in `ios/Resources/Fonts`), and the
   scheduled task's definition.
5. **Sign the binaries** — `signtool sign /fd SHA256 /td SHA256 /tr <timestamp>
   /sha1 <thumbprint>`, then `signtool verify /pa`.
6. **Package** — `iscc /DAppVersion /DDist /O … windows/installer/TaigiKeyboard.iss`
   → `TaigiKeyboard-<version>-Setup.exe`, with the same `ProductName` /
   `ProductVersion` in its own VERSIONINFO (`VersionInfo*` directives) —
   read back and compared the same way.
7. **Sign the installer** — same certificate, then verify.
8. **Name the output** — `windows/.build/distribution/TaigiKeyboard-<version>-Setup.exe`
   and its SHA-256. A dirty tree or `--skip-sign` stamps `-dirty` /
   `-unsigned` into the name, so a throwaway cannot be mistaken for a release.
9. **Publish** — `windows/scripts/publish-release.sh` (below).

Flags: `make windows-release` passes `--force --publish`. `--publish` refuses to
run with `--skip-sign` or `--allow-dirty`; those builds call the script directly:

```sh
bash windows/scripts/release-app.sh --skip-sign      # packaging only, unshippable
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
  (`installerDllLocked` / `installerStepFailed` / `installerSignOutNote`) are
  `desktop.installer*` keys in `i18n/desktop.json`, emitted by `make i18n`
  into `windows/installer/Messages.iss` — the installer is never edited for
  wording. Tâi-lô / POJ have no Inno base language (the macOS Installer
  cannot show them either); the app itself offers all five.
- **Files** — `TaigiKeyboard.dll` (+ `x86\TaigiKeyboard.dll` when staged),
  `TaigiKeyboardSettings.exe`, `Dictionaries\`, `Fonts\`,
  `update-check-task.xml`.
- **Registration** — `regsvr32 /s` on the DLL (the DLL's `DllRegisterServer`
  registers the CLSID, the `0x0404` profile, the categories and the
  display-attribute provider — roadmap W7). Run from `[Code]`, not `[Run]`:
  Inno ignores a `[Run]` entry's exit code, so a registration or task
  creation that fails is checked explicitly and treated as the install
  failing — the previous DLL (backed up before it was unregistered) is put
  back and re-registered, the task deleted, and Setup raises, which reports
  the error and rolls back its file installation.
- **Start menu** — a shortcut to the settings exe carrying
  `System.AppUserModel.ID = TaigiKeyboard.Settings`, which is what lets the
  updater's toast be shown at all.
- **Scheduled task** — `TaigiKeyboard Update Check`, per-user (created as the
  original, non-elevated user from `update-check-task.xml` with the exe's
  path filled in): five minutes after logon, then daily, running
  `TaigiKeyboardSettings.exe --check-updates`; `IgnoreNew` for parallel
  instances.
- **Uninstall** — the reverse, item by item; `%APPDATA%\TaigiKeyboard` (the
  settings, the learning data, the custom dictionary) stays. The scheduled
  task is deleted as the original user of the *uninstall*: a machine where a
  different administrator installed keeps that user's task until they remove
  it (PR11 smoke item).

The installer never launches the settings window itself: the window removes
stale staged update packages at launch, and the installer may still be
reading its own payload.

## Publishing the installer

`windows/scripts/publish-release.sh` (run by `--publish`):

1. Refuses a `-dirty` / `-unsigned` name, an installer `signtool verify`
   does not trust, an installer whose VERSIONINFO is not
   `Taigi Keyboard` / the checkout's version, and — when
   `WINDOWS_SIGNING_THUMBPRINT` is set — a signer other than that certificate
   (what every installed copy pins).
2. Creates (or reuses) the GitHub release `windows-v<version>` on the
   website repository with the `### Windows` section of
   `changelog/desktop-v<version>.md` (the desktop train's record), and
   uploads the installer as its asset.
3. Fetches the release page and one byte of the asset **anonymously** — the
   page must answer `200`, the asset `206` (or `200`).
4. Writes `_data/windows_release.json` — one file, one commit — then waits
   until `https://taigikeyboard.tw/appcast/windows.json` serves the new
   version **and** its installer URL. The manifest every installed copy
   polls is rendered from that data file by the site's own build
   (`windows/updates/README.md` § One published fact, one committed file),
   so the poll is also what proves the site built what was committed.

Re-running after a failure adds to the existing release rather than tearing
it down; the data file is written only after the download is provably
reachable. The site advertises the download only once its
`enable_windows_download` flag is on — the data file alone does not.

## Notes

- **Certificate rotation.** The updater pins the signer's LEAF thumbprint
  (roadmap W9, PR9 Codex): the first release signed with a renewed
  certificate is not accepted in-app by the copies signed with the old one.
  Plan that release as a download-page release (the manifest's
  `downloadPageURL` route, which every copy has); the copies it installs pin
  the new certificate and in-app updates resume.
- **Unsigned builds** (`--skip-sign`) run — SmartScreen warns — but the
  running settings exe has no signature identity, so it never offers an
  in-app install: the 一般 pane offers the download page instead, as an
  ad-hoc macOS build does.
- **`rc.exe` absent** — a development build compiles, with a warning, without
  its icon and VERSIONINFO; the release script sets `TAIGI_REQUIRE_RESOURCES=1`,
  under which the build fails instead, and reads the VERSIONINFO back
  afterwards regardless — the updater's package check depends on it.
- **First real-Windows run (PR11 smoke, none of this has executed yet)** —
  `schtasks /Create /XML` as the original user without `/RU` on a
  UAC-elevated install; an install with a host still holding the DLL
  (expect the sign-out recipe, not a partial install); the rollback path
  (make `regsvr32` fail on purpose: expect the old DLL back and Setup
  reporting failure); the toast under the Start-menu AUMID; `dumpbin` and
  PowerShell present in the Git Bash `PATH` on the release machine.
