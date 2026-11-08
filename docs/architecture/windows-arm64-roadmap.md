# Windows on Arm64 — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `windows`, `arm64`, `arm64x`, `forwarder`, `tsf`, `inno setup`, `universal installer`, `mozc`
> **Status**: Done 2026-11-08. Phases 0–4 merged (spike on a Windows 11 Arm VM; TSF + forwarder #760; release tooling + CI smoke #764; universal installer + installer smoke #768). Issue #721. The aarch64 `check-box.sh` step followed once the box had the MSVC ARM64/ARM64EC build tools.
> **Siblings**: `windows-release.md` (§ Architectures, § What an Arm64 round would have to know), `windows-roadmap.md`.

---

## Summary

- **Goal**: Windows 11 on Arm64 machines can install Taigi Keyboard, and the input method works in Arm64-native apps (Notepad, Edge, Office), x64-emulated apps and x86-emulated apps.
- **Why it was refused until Phase 4**: a text service loads in the host process's own architecture. An x64-only install would work in emulated apps and do nothing in native ones, so the installer refused Arm64 (`ArchitecturesAllowed=x64os`).
- **Approach**: the mainstream one — an Arm64X pure forwarder named by `InProcServer32`, a native Arm64 TSF DLL beside the x64 one, a native Arm64 settings exe, all in one universal installer. mozc ships its TSF TIP the same way.

## Decisions (maintainer, 2026-11-07)

| # | Question | Decision |
|---|---|---|
| D1 | Test environment | A Windows 11 Arm VM on an Apple Silicon Mac for dogfood, plus a `windows-11-arm` CI smoke job |
| D2 | Settings exe on Arm64 | Native Arm64 build (Microsoft recommends native builds for production apps) |
| D3 | Installer shape | One universal installer; the updater and download links stay single-file |

## Design (grounded in code)

### Install layout on an Arm64 machine

| File in `{app}` | Machine | Role |
|---|---|---|
| `TaigiKeyboard.dll` | x64 | Name unchanged, so an x64 machine's layout is identical to today. Loaded by x64-emulated hosts through the forwarder |
| `TaigiKeyboardArm64.dll` | Arm64 | Loaded by Arm64-native hosts |
| `TaigiKeyboard64X.dll` | Arm64X pure forwarder | Named by `InProcServer32`: Arm64-native and x64-emulated processes read the same 64-bit registry view, so one path has to serve both |
| `TaigiKeyboard32.dll` | x86 | Unchanged; WOW64 hosts read the 32-bit registry view |
| `TaigiKeyboardSettings.exe` + runtime | Arm64 | Same install name, so the shortcut, uninstall icon, update task XML and the updater's `current_exe()` check stay unchanged |

Every service DLL resolves `Dictionaries\`, `Fonts\` and the settings exe from its own directory (`windows/crates/taigi-windows-tsf/src/module.rs:34-36`), so all of them sit flat in `{app}`.

### Forwarder

- mozc's recipe (`references/mozc/src/win32/tip/build_tip_forwarder_dll.py:246-340`), in an Arm64-target developer environment (`windows/scripts/build-forwarder.ps1`: `Enter-VsDevShell -DevCmdArguments '-arch=arm64 -host_arch=<arm64|amd64>'` — its `-HostArch` accepts only x86 / amd64):
  1. `cl /c empty.cc` → `empty_arm64.obj`, and `cl /c /arm64EC empty.cc` → `empty_x64.obj`.
  2. `link /lib /machine:arm64EC /def:<x64>.def` and `link /lib /machine:arm64 /def:<arm64>.def` → one import library per view.
  3. `link /dll /noentry /machine:arm64x /dynamicbase /highentropyva /largeaddressaware /ignore:4104 /defArm64Native:<arm64>.def /def:<x64>.def` over both objects, both libraries and a VERSIONINFO `.res`.
- Each `.def` forwards `DllGetClassObject` and `DllCanUnloadNow` only. Registration is never run through the forwarder.
- **Measured traps (spike 2026-11-07)**:
  - Without the two empty objects the forwarder links with `/nodefaultlib` but warns `LNK4294` (no EC / native load config). An x64 process then reads it as a plain Arm64 DLL, follows the native forward to `TaigiKeyboardArm64.dll` and fails with error 193 — the service works in Arm64 apps and silently not in x64 ones. `dumpbin /headers` must print `AA64 machine (ARM64) (ARM64X)`; that marker is the release read-back check.
  - `PRIVATE` on the `.def` exports removes them from the import libraries and the final link fails `LNK2001`; mozc keeps them public and passes `/ignore:4104`.
  - The service DLLs need the Windows SDK `rc.exe` on `PATH` (`bin\10.0.*\arm64`), or they build without icon and VERSIONINFO (`build-support/resource.rs` warns).

### Registration

- Today `register_server` writes its own `module_path()` as `InProcServer32` and as the profile icon / display-name resource (`windows/crates/taigi-windows-tsf/src/registration.rs:64-75, 242-248, 253-308`).
- The aarch64 build (`cfg(target_arch = "aarch64")`) writes the sibling forwarder's path as `InProcServer32` instead, and fails registration when that file is missing rather than registering a path that cannot load.
- The profile icon and display name keep the Arm64 DLL's own path: resource loading does not depend on the reader's architecture. mozc likewise points its profile at a non-forwarder DLL (`references/mozc/src/win32/custom_action/custom_action.cc:489-493`).
- Installer order on Arm64: the x86 DLL first, the Arm64 DLL (`{sys}\regsvr32`, native) last, so the last write wins on the shared profile — the existing native-64-bit-last rule (`TaigiKeyboard.iss` `RegisterEverything`, through `NativeDll`). The x64 DLL is never self-registered on Arm64; that would overwrite `InProcServer32` with an x64-only path.

### Build

- `windows/crates/taigi-windows-tsf/build.rs:12-15` gains an `aarch64` arm: OriginalFilename `TaigiKeyboardArm64.dll`.
- The settings exe is built per target; `windows-reactor-setup` already picks the `win10-arm64` Windows App Runtime payload from the same pinned nupkg.
- **Hazard**: `windows-reactor-setup`'s `.msix_extract` cache is not keyed by architecture (windows-rs rev `dc720b36`, `crates/libs/reactor-setup/src/lib.rs:155-164`). Built after x64, the Arm64 build would silently stage x64 runtime DLLs. `release-app.sh` clears that cache before each architecture's build and reads back the PE machine of every staged runtime DLL.
- Bundled SQLite (`engine/Cargo.toml:56`, via `cc`) needs an Arm64 `cl.exe`: the VS "MSVC ARM64/ARM64EC build tools" component.
- Staging adds `arm64\TaigiKeyboardSettings.exe` and `RuntimeArm64\`; the installer picks them with `Check: IsArm64` under the same `DestName`.

### Installer

- `ArchitecturesAllowed=x64compatible`, `ArchitecturesInstallIn64BitMode=x64compatible`.
- `IsArm64` branches in `[Files]` and `[UninstallRun]`; the Pascal register / restore / rollback code goes through one `NativeDll` (the x64 DLL on x64, the Arm64 DLL on Arm64) and one `HadNativeDll` flag, so the x64 path is unchanged step for step.
- `MinVersion` stays 10.0.17763 for x64. Arm64 also requires Windows 11 (build 22000): Windows 10 on Arm has no x64 emulation, so the forwarder's x64 half could not load. `x64compatible` already excludes it (it matches x64 Windows and Arm64 Windows 11), so Inno refuses it with its own architecture message.
- Decided in Phase 4 (2026-11-08, pre-impl review): no custom `InitializeSetup` check (it could never run — the case above is refused first); no `HadForwarder` flag (the forwarder is never registered itself, and MakeWay / PutBackMovedFiles move and restore it with every other file); `RemoveAddedX86Dll` is not extended to the Arm64 DLL (a failed fresh install leaves its payload the same way on both machines). Known and accepted: installers before 3.6.8 allowed `x64compatible`, so an Arm64 machine can hold an old x64-only install. Upgrading it registers the Arm64 DLL over the old registration; only a FAILED upgrade of such a machine rolls back to an unregistered old version (`HadNativeDll` is false there). That install never worked in Arm64-native apps.
- CI: `windows-build.yml` runs the installer itself — `installer-x64` upgrades from the last published release on `windows-2025`, `installer-arm64` installs fresh on `windows-11-arm` and activates from an Arm64 and an x64 process — each then uninstalls and checks nothing is left (`windows/scripts/smoke-installer.ps1`).

### Updater

Unchanged: one installer asset per release (`windows/updates/README.md`).

## Phases

One PR per phase; each phase starts after the previous one merges.

| # | Phase | Content | Status |
|---|---|---|---|
| 0 | Roadmap | This document; the issue #721 update | Done 2026-11-07 |
| 1 | Spike (no merge) | Build the Arm64 DLL and the forwarder by hand, register them by hand in the Windows 11 Arm VM, type in Notepad (Arm64), an x64-only app and a 32-bit app. Record the result here. **Hard gate**: if the forwarder does not activate the text service, stop | **Passed 2026-11-07** (§ Spike result) |
| 2 | TSF + forwarder | `build.rs` aarch64 name; registration writes the forwarder path, with tests; `windows/forwarder/` `.def` / `.rc` + `scripts/build-forwarder.ps1`; `install-dev.ps1` Arm64 path; aarch64 in `Makefile check-msvc` | Done 2026-11-07 (#760) |
| 3 | Release tooling + CI | `scripts/lib/identity.sh` Arm64 names; `release-app.sh` aarch64 DLL + settings build, forwarder link, read-back checks (`AA64` / Arm64X machine, forwarded exports, runtime machine), staging, signing list; `windows-build.yml` aarch64 target, `Hostx64\arm64\link.exe`, Arm64 `cl.exe`, and a `windows-11-arm` smoke job (regsvr32, then `CoCreateInstance` of the CLSID from an Arm64 and an x64 process); the aarch64 TSF check in `check-box.sh`, once the box has the MSVC ARM64/ARM64EC build tools (moved from Phase 2: the box was off, and an aarch64 step without them fails every `make windows-check`) | Done 2026-11-08 (#764); the `check-box.sh` step in a follow-up PR |
| 4 | Universal installer + docs | `TaigiKeyboard.iss` as above; rewrite `windows-release.md` § Architectures / § Arm64 / machine setup / installer; `docs/CODE_SIGNING_POLICY.md` file list; `windows-roadmap.md`; installer smoke jobs in CI | Done 2026-11-08 (#768) |

### Verification

- Phases 2–3: `make windows-check`; a branch can be built by dispatching `windows-build.yml` on it (only a `main` run attaches to a draft, after the smoke job passes); `python3 tools/test_select.py --run --platform windows`; a dispatched `windows-build.yml` run whose artifact holds all five binaries with the expected PE machines; the `windows-11-arm` smoke job.
- Phase 4 (VM dogfood): install the CI installer; type Taigi in Notepad and Edge (Arm64), an x64-only app and a 32-bit app; Task Manager shows the settings window as Arm64; uninstall leaves no CLSID or profile. On an x64 machine, upgrade and uninstall behave as before.

### Spike result (2026-11-07)

Windows 11 Arm64 VM (UTM on Apple Silicon, build 26300), `main` @ `ad13d934a`, all three DLLs built natively on the VM (VS 2022 Build Tools 17.14, MSVC 14.44, Rust 1.99.1). Registration: `regsvr32` on the x86 DLL, then on the Arm64 DLL, then `InProcServer32` rewritten by hand to the forwarder. Typed `tai5gi2` + Space through `SendInput` from the interactive session:

| App | Process | Candidate window | Committed | Modules loaded |
|---|---|---|---|---|
| Notepad | Arm64 | yes | 台語 | `TaigiKeyboard64X.dll`, `TaigiKeyboardArm64.dll` |
| Notepad++ | x64, emulated | yes | 台語 | `TaigiKeyboard64X.dll`, `TaigiKeyboard.dll` |
| PowerShell ISE (`SysWOW64`) | x86, emulated | yes | 台語 | `TaigiKeyboard32.dll` (WOW64 modules are not listed by Arm64 `tasklist`) |

`CoGetClassObject(CLSID)` returns `S_OK` from both an Arm64 and an x64 process, each loading only its own architecture's DLL. Explorer loads the Arm64 service through the forwarder too. One observation not yet compared with an x64 machine: in PowerShell ISE (WPF) the preedit and candidate window sit at the screen's top-left instead of the caret.

### Open questions
- Answered 2026-11-07: the `windows-2025` image's Visual Studio 2022 (17.14) carries the `VC.Tools.ARM64` and `VC.Tools.ARM64EC` components (actions/runner-images README); the build job cross-builds Arm64 on it, and `windows-11-arm` only runs the smoke job. If the `windows-2025` label moves to Visual Studio 2026, re-check `build-forwarder.ps1`'s `-DevCmdArguments` host argument.
- Answered 2026-11-07: the Arm64 Windows App Runtime payload stages the same names as `windows/build-support/windows-app-runtime-files.txt`; all 28 top-level DLLs are `AA64` with no CLR header, so `release-app.sh` requires every top-level runtime DLL to match the exe's machine.

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| Arm64X pure forwarder for a 64-bit in-process COM server | learn.microsoft.com/windows/arm/arm64x-build § pure forwarder; `references/mozc/src/win32/tip/build_tip_forwarder_dll.py` | Phase 2 |
| The installer registers the forwarder on Arm64 machines | `references/mozc/src/win32/custom_action/custom_action.cc:457-476` | Phases 2, 4 |
| Profile resources from a real DLL, not the forwarder | `references/mozc/src/win32/custom_action/custom_action.cc:489-493` | Phase 2 |
| One installer for x64 and Arm64 | mozc `win_universal_installer` (`references/mozc/docs/build_mozc_in_windows.md:134-148`) | Phase 4 |
| Native Arm64 builds for production apps | learn.microsoft.com/windows/arm/add-arm-support | Phase 3 |
| Spike an OS capability before building on it | `docs/contributing/known-pitfalls.md` § Tests and diagnosis | Phase 1 |

### Deliberately not adopted

- **Arm64EC service DLL** — one binary for both, but every crate would build as EC; mozc ships separate DLLs plus a forwarder.
- **Separate Arm64 installer** — D3.
- **Runtime architecture detection inside the DLL** — only the aarch64 build ever sits beside a forwarder, so `cfg(target_arch)` decides at compile time.
- **Arm64 on Windows 10** — no x64 emulation there.
