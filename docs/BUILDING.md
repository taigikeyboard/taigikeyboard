# Building TaigiKeyboard

How to go from a fresh clone to a build and a test run on one platform. You do
not need every toolchain: pick the row you work on.

| You work on | Host | Minimum to build and test |
| --- | --- | --- |
| Shared engine (`engine/`) | macOS, Linux or Windows | Rust, protoc |
| Android | macOS or Linux | engine + JDK 17, Android SDK, NDK r25+, cargo-ndk |
| iOS | macOS | engine + Xcode, swift-protobuf |
| macOS input method | macOS | engine + Xcode, swift-protobuf |
| Windows input method | Windows for the DLL; macOS or Linux for the compile gate (its `check-box` step needs the maintainer's Windows box) | engine + MSVC (Windows) or mingw-w64 (gate) |
| Linux input method | Linux; macOS for the compile gate | engine + the distribution packages below |
| Dictionary | any | Python 3.13+, Node 22+ |

## 1. Get the source

```sh
git clone --recurse-submodules --filter=blob:none \
  https://github.com/taigikeyboard/taigikeyboard.git
```

- `--filter=blob:none` fetches old file versions on demand. The full history is
  about 800 MB, almost all of it dictionary artifacts that are no longer
  tracked.
- `--recurse-submodules` fills `taigi-converter/` (the canonical TL / POJ / TPS
  converter, needed by `make dict`) and `corpus/taigi-typing/` (real sentences
  for manual testing; never a build input). Cloned without it:
  `git submodule update --init taigi-converter`. The large
  `corpus/taigi-corpus/` submodule stays empty unless fetched explicitly
  (`corpus/README.md`).

## 2. Install the tools

Rust comes from [rustup](https://rustup.rs). Each Rust workspace has its own
`rust-toolchain.toml` (`engine/`, `desktop/`, `macos/`, `windows/`, `linux/`), so rustup
picks the channel, components and cross targets on first use.

`engine/rust-toolchain.toml` lists the iOS, macOS and Android targets, so the
first `cargo` run there downloads all of them. For engine-only work on Linux or
Windows, skip that with `RUSTUP_TOOLCHAIN=stable` — which is what CI does.

Everything else that has a version the build checks is pinned in
[`mise.toml`](../mise.toml):

```sh
mise install        # protoc, gitleaks, swiftformat, uv
```

Without [mise](https://mise.jdx.dev), install the same versions by hand. The
one that matters is **protoc 36.2**: it must match `protobuf-javalite` in
`android/app/build.gradle.kts` (javalite `4.36.2` is emitted by libprotoc
`36.2`). With any other protoc, `make protos` skips the Android Java and iOS
Swift binding regeneration and prints `protoc version drift — SKIPPING platform
proto regeneration` (the macOS Swift bindings are still regenerated). The
committed bindings stay valid, so the rest of the build still runs; you only
need 36.2 to change a `.proto` file.

Per platform, on top of that:

| Platform | Install |
| --- | --- |
| Android | JDK 17; Android SDK with platform 36; NDK r25+ (`sdkmanager`); `cargo install cargo-ndk --locked` |
| iOS / macOS | Xcode (iOS 17 / macOS 14 deployment targets); `brew install swift-protobuf` for `protoc-gen-swift` |
| Windows gate on macOS | `brew install mingw-w64` — see [`windows/README.md`](../windows/README.md) |
| Windows DLL | Windows 10 1809+ x64 with MSVC — see [`architecture/windows-release.md`](architecture/windows-release.md) |
| Linux | the packages in [`linux/README.md`](../linux/README.md) § System packages |
| Dictionary | Python 3.13+ with `python3 -m pip install -r dictionary/requirements.txt pytest`; Node 22+ for `taigi-converter/` |

## 3. One-time setup

```sh
make hooks          # pre-commit: gitleaks + personal-data checks on staged changes
```

`core.hooksPath` is shared by every worktree of a clone. See
[`CONTRIBUTING.md`](../CONTRIBUTING.md) § Secrets.

## 4. Build and test

`make help` lists every root target.

| Platform | Build | Test |
| --- | --- | --- |
| Engine | `cargo build --workspace --manifest-path engine/Cargo.toml` | `make test` (`cargo test --workspace` in `engine/`) |
| Android | native libraries: `make android-libs`; app: `android/gradlew -p android :app:assembleDebug` | `android/gradlew -p android :app:testDebugUnitTest` |
| iOS | `make ios-libs`, then open `ios/TaigiKeyboard.xcodeproj` and build the keyboard extension | `xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboardTests -destination 'platform=iOS Simulator,name=<an installed iPhone simulator>' test` |
| macOS | `make macos-libs`, then `make -C macos build` | `make -C macos test` |
| Windows | on Windows: [`architecture/windows-release.md`](architecture/windows-release.md); elsewhere: `make windows-check` — its `check-box` step runs on the maintainer's Windows box over ssh and fails without it; run the box-free steps in [`windows/README.md`](../windows/README.md) instead | included in `make windows-check` |
| Linux | on Linux: `make -C linux build` (packages: `make -C linux deb`); on macOS: `make linux-check` | included in `make linux-check` |
| Desktop-shared crates | `make desktop-check` | included |
| macOS Rust workspace (`macos/crates`, the archive the app links) | `make macos-rust-check` | included |
| Dictionary | `make dict` | `python3 -m pytest tests`, run inside `dictionary/` (needs the `taigi-converter` submodule) |
| taigi-converter | — | `npm test` in `taigi-converter/` |

`make test-changed` runs only the rows the branch's changes need
(`tools/test_select.py`: engine crates plus their reverse dependencies, and
each touched platform); `BASE=<ref>` diffs against another base,
`PLATFORM=engine,macos` narrows it, and the script without `--run` only prints
the selection. Its iOS command names the maintainer's simulator; use the table's
command with your own.

Before opening a pull request, run `make lint` — rustfmt, clippy, SwiftFormat and
Spotless, the same checks CI runs (`make fmt` applies the formatters).

Notes:

- `make build` runs `make protos`, `ios-libs`, `android-libs` and `macos-libs`
  in that order, so it needs macOS with every tool above. On one platform, run
  only that platform's target from the table.
- The Android JVM unit tests do not load the native library, so
  `:app:testDebugUnitTest` runs without it; installing or running the app does
  need it. The Swift and Java protobuf bindings are committed, so only a
  `.proto` change needs them regenerated (protoc 36.2, § 2). The engine's own
  Rust build always needs protoc.
- iOS signing: the project names the maintainer's development team. Pick your
  own team under *Signing & Capabilities* for a device build, and leave that
  change out of your commits.

## 5. Keep native artifacts in step with the engine

iOS, Android and macOS link pre-built engine binaries. After changing Rust or
dictionary sources, rebuild them before testing a platform, or the tests run
against the old engine and pass for the wrong reason:

| Your change touches | Run before platform tests |
| --- | --- |
| `engine/` (Rust, `Cargo.toml`) | the platform's native build from § 4 (`make ios-libs` / `android-libs` / `macos-libs`; `make build` on macOS does all of them) |
| a `.proto` file | `make protos` first (protoc 36.2), then as above |
| `dictionary/` | `make dict`, then as above |
| `desktop/`, `macos/crates/`, `macos/Cargo.*` | `make macos-libs` (or `make build`) before macOS tests — the archive the app links |
| platform-only Swift / Kotlin, docs | nothing |

Details and timings: [`architecture/build-artifacts.md`](architecture/build-artifacts.md).

## 6. Troubleshooting

### macOS input-method switching

Debug builds retain session handovers and key-delivery transitions in the
macOS unified log. These use Notice (and Error for rejected delivery), which
macOS [stores on disk within its retention limit](https://developer.apple.com/documentation/os/generating-log-messages-from-your-code).
Release builds still emit no application logs.

After a failure, read the recent history without needing a stream running
beforehand:

```sh
/usr/bin/log show --last 30m --style compact --predicate 'process == "TaigiInputMethod" AND (category == "InputController" OR category == "SessionCoordinator" OR category == "ComposingBackend")'
```

`session` is a process-local numeric identity; `tenure` changes on activation
and teardown. Follow `activate begin/end`, `claim`, `end begin/complete` and
`release` to reconstruct ownership. Activation makes a controller eligible
for input; it does not prove exclusive focus. `activation waiting` means an
additional controller left the current composition alone; `key selects`
records a handover confirmed by a key delivered to another active controller.
After the owner ends, a sole waiting controller resumes before the first key;
several waiting controllers require key delivery to select one.
`end superseded` records a teardown that yielded to a reactivation;
`replay discarded` records a stale reply.
The first delivered key after a lifecycle change records `key delivery ready`
or a rejection reason (`inactiveSession` / `missingClient`), and only a change
in that delivery state produces another record. No key contents, composition
text, document data or client queries are added by these diagnostics.

For verbose live output, use `make -C macos log`. Record the approximate
failure time and whether another switch restored input; retention is bounded
by macOS, so inspect the history soon after an incident.

### Build setup

| Message | Cause | Fix |
| --- | --- | --- |
| `protoc version drift — SKIPPING platform proto regeneration` | protoc is not 36.2 | `mise install`, or install protoc 36.2; harmless unless you changed a `.proto` |
| `protoc-gen-swift not found` | swift-protobuf missing | `brew install swift-protobuf` |
| `cargo-ndk not found` | Android native build without cargo-ndk | `cargo install cargo-ndk --locked` |
| `gitleaks is not installed` | `make scan-secrets` / the hook without gitleaks | `mise install`, or install gitleaks 8.30.1 — the version `.gitleaks-scanned` records |
| rustup downloads Apple / Android targets on Linux | `engine/rust-toolchain.toml` target list | `RUSTUP_TOOLCHAIN=stable` for engine-only work |
