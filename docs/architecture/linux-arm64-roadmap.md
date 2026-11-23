# Linux on Arm64 — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `linux`, `arm64`, `aarch64`, `deb`, `rpm`, `ubuntu-24.04-arm`, `desktop train`
> **Status**: Accepted 2026-11-24 (maintainer: "do it"). Phase 0 (this doc) done; Phases 1–4 pending.
> **Siblings**: `linux-release.md` (packages, layout, CI half of a desktop release), `windows-arm64-roadmap.md` (the Windows precedent), `desktop-release.md`.

---

## Summary

- **Goal**: Arm64 Linux machines (Ubuntu / Debian / Fedora on aarch64) install a released Taigi Keyboard package instead of building from source.
- **Packages**: `taigikeyboard_<version>_arm64.deb` and `taigikeyboard-<version>-1.aarch64.rpm` beside the existing x86_64 three. No Arch arm64 package: the official `archlinux` container image is x86_64-only and Arch Linux ARM is a separate distribution (recommended default, to be confirmed by the maintainer before Phase 3).
- **Cost**: ~150–250 hand-written lines over three PRs plus one website PR; no engine or platform-code change expected.

## Current state (grounded in code, 2026-11-24)

The code is architecture-neutral; only packaging and release name x86_64.

- CI builds natively without `--target` (`.github/workflows/linux-build.yml` "cargo build --release" step), so an Arm runner builds the same commands unchanged.
- An arm64 build already ran: the UTM arm64 Ubuntu dogfood VM builds from source with `make install`, and the GNOME + IBus dogfood pass ran there. Fcitx5 on arm64 has never run.

| Location | Hardcoded |
|---|---|
| `linux/packaging/control.in:3`, `linux/packaging/shlibdeps-control:7` | `Architecture: amd64` |
| `linux/packaging/taigikeyboard.spec.in:16` | `ExclusiveArch: x86_64` |
| `linux/packaging/PKGBUILD.in:12` | `arch=('x86_64')` (stays; no Arch arm64) |
| `linux/Makefile:20,267,299,304,311` | `LINUX_TARGET`, the three package file names, `RPMS/x86_64/` |
| `.github/actions/setup-protoc/action.yml:27` | protoc digest for Linux/X64 only — an Arm runner fails here |
| `.github/workflows/linux-build.yml` | every job on `ubuntu-24.04`; amd64 control check; `x86_64-linux-gnu` addon path in install-check |
| `tools/release/lib/desktop-release.sh:50-52,233` | asset names, expected-asset list |
| `tools/release/announce-release.sh:196`, `tools/release/nightly-release.sh:53-55` | announce and nightly asset tables |
| `docs/INSTALL.md`, website repository | download buttons |

## Phases

One PR per phase.

| Phase | Change | Size |
|---|---|---|
| 0 | This roadmap + project memory (admin) | docs |
| 1 | Packaging parameterised by host architecture: `DEB_ARCH` from `dpkg --print-architecture`, RPM arch from `uname -m`, `@ARCH@` in `control.in` / `shlibdeps-control`, `ExclusiveArch: x86_64 aarch64`, package file names derived; `check-target` also cross-builds `aarch64-unknown-linux-gnu` | ~40–60 |
| 2 | CI: `ubuntu-24.04-arm` matrix leg for build + `.deb`, `fedora:44` on the Arm runner for `.rpm`, install-check on arm64; protoc Linux/ARM64 digest in `setup-protoc` | ~60–100 |
| 3 | Release tooling: arm64 assets in `desktop-release.sh` (names + expected list), announce, nightly; `docs/INSTALL.md`; website download buttons (separate repository PR) | ~50–80 |
| 4 | Dogfood on the UTM arm64 VM: install the CI arm64 `.deb`, check IBus and Fcitx5 | — |

Phase 3 touches release tooling → High tier (full review sandwich).

## Open questions (maintainer)

1. Package set: `.deb` + `.rpm` arm64 (recommended), `.deb` only, or also Arch arm64 on an unofficial image.
2. Website buttons: two more buttons, or an architecture picker.

## Unverified

- GitHub-hosted Arm runner availability and quota for this public repository — check current GitHub docs before Phase 2.
- `fedora:44` aarch64 image building the package on the Arm runner.
- Fcitx5 on arm64.

## Best practices alignment

- Package names follow each format's own architecture field (Debian `arm64`, RPM `aarch64`), as `linux-release.md` already does for x86_64.
- Deliberately not adopted: cross-compiling arm64 packages on the x86_64 runner — `dpkg-shlibdeps` and rpm's ELF scan need target-architecture system libraries; a native Arm runner keeps one build path.
