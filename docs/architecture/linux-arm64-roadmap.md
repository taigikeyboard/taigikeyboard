# Linux on Arm64 — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `linux`, `arm64`, `aarch64`, `deb`, `rpm`, `ubuntu-24.04-arm`, `desktop train`
> **Status**: Accepted 2026-11-24 (maintainer: "do it"). Phases 0 (this doc), 1 (packaging by architecture), 2 (CI on the Arm runner, #1103) and 3 (release tooling) done; Phase 4 pending.
> **Siblings**: `linux-release.md` (packages, layout, CI half of a desktop release), `windows-arm64-roadmap.md` (the Windows precedent), `desktop-release.md`.

---

## Summary

- **Goal**: Arm64 Linux machines (Ubuntu / Debian / Fedora on aarch64) install a released Taigi Keyboard package instead of building from source.
- **Packages**: `taigikeyboard_<version>_arm64.deb` and `taigikeyboard-<version>-1.aarch64.rpm` beside the existing x86_64 three. No Arch arm64 package: the official `archlinux` container image is x86_64-only and Arch Linux ARM is a separate distribution (maintainer decision 2026-11-24).

## Current state (grounded in code, 2026-11-24)

- Packaging, CI and release tooling name the host architecture rather than x86_64: `DEB_ARCH` / `@ARCH@` (`linux/Makefile:276-291`), `LINUX_TARGETS` with `aarch64-unknown-linux-gnu` (`linux/Makefile:23`), the `ubuntu-24.04-arm` legs and arm64 artifacts in `.github/workflows/linux-build.yml`. `linux/packaging/PKGBUILD.in` keeps `arch=('x86_64')` (no Arch arm64).
- The UTM arm64 Ubuntu dogfood VM has built from source with `make install` and passed a GNOME + IBus dogfood run. Fcitx5 on arm64 has never run on a real desktop.

## Phases

One PR per phase.

| Phase | Change | Status |
|---|---|---|
| 0 | This roadmap + project memory (admin) | Done |
| 1 | Packaging parameterised by host architecture: `DEB_ARCH` from `dpkg --print-architecture`, RPM arch from `uname -m`, `@ARCH@` in `control.in` / `shlibdeps-control`, `ExclusiveArch: x86_64 aarch64`, package file names derived; `check-target` also cross-builds `aarch64-unknown-linux-gnu` | Done |
| 2 | CI: `ubuntu-24.04-arm` matrix leg for build + `.deb`, `fedora:44` on the Arm runner for `.rpm`, install-check on arm64; protoc Linux/ARM64 digest in `setup-protoc` | Done |
| 3 | Release tooling: arm64 assets in `desktop-release.sh` (names + expected list), announce, nightly; `docs/INSTALL.md`; website download buttons (separate repository PR) | Done |
| 4 | Dogfood on the UTM arm64 VM: install the CI arm64 `.deb`, check IBus and Fcitx5 | Pending |

## Decisions (maintainer, 2026-11-24)

1. Package set: `.deb` + `.rpm` arm64, no Arch arm64. The arm64 CI legs gate the release and the nightly (no `continue-on-error`).
2. Website buttons: two more buttons (arm64 `.deb`, aarch64 `.rpm`), no architecture picker.

## Unverified

- Phase 3's release paths have not run: dry runs skip the `attach` job and the nightly publish, so the five-package attach, `check-assets` / `check-publish`, the beta mirror, the arm64 announcement and the arm64 nightly upload first run on the next train cut and the next `main` nightly.
- Fcitx5 and IBus on a real arm64 desktop session (Phase 4). CI already proves the rest on `ubuntu-24.04-arm` (generally available and free for public repositories, GitHub changelog 2025-08-07): run 45317118454 built both arm64 packages, `fedora:44` aarch64 included, and the arm64 install checks had Fcitx5 load the addon and IBus spawn the engine.

## Best practices alignment

- Package names follow each format's own architecture field (Debian `arm64`, RPM `aarch64`), as `linux-release.md` already does for x86_64.
- Deliberately not adopted: cross-compiling arm64 packages on the x86_64 runner — `dpkg-shlibdeps` and rpm's ELF scan need target-architecture system libraries; a native Arm runner keeps one build path.
