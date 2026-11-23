# End-to-End Test System — Roadmap

> **Type**: Reference (design record + PR table)
> **Keywords**: `e2e`, `test mode`, `trace`, `simulator`, `emulator`, `VM`, `container`, `scenario`, `perf`, `Xvfb`, `xdotool`, `UiAutomator`
> **Status**: Linux live — `make e2e PLATFORM=linux` (headless) and `PLATFORM=linux-desktop` (real VM sessions), nightly CI matrix `linux-e2e.yml`, skill `/e2e`. Other platforms are tested by hand.

---

## Goal

An automated typing test an AI agent can run: real key sequences into a real text field of a test build, with a debug trace that exists only in test mode, because a shipped input method writes no log.

1. An AI agent starts a simulator / emulator / VM / container, installs a **test build** of the IME, types real key sequences into a real text field, and reads back the committed text.
2. The test build writes a **structured trace** (JSON Lines). An analyzer turns trace + expectations into a report: wrong commit, engine errors / panics, latency per operation, memory.
3. **The trace exists only in test builds.** Release artifacts contain no trace code and write no trace file — enforced by build-script and CI checks (marker string + feature graph), not by a runtime switch.
4. Linux (Fcitx5 + IBus) on every packaged distribution (Ubuntu / Debian, Fedora, Arch); the other platforms stay manual (§ Decisions).
5. **Real desktop sessions beside CI.** The headless Xvfb run (CI + `make e2e PLATFORM=linux`) stays; the desktop drivers type into the logged-in session of both Linux dogfood VMs (GNOME Wayland + IBus, KDE X11 + Fcitx5).

## Design

### D1 — One trace format, one analyzer, per-platform drivers

```
scenario (JSON)  ──►  driver (per platform)  ──►  real host app + test-build IME
                                                        │
                                            trace.jsonl ◄┘  (test build only)
                                                        │
                      analyzer  ◄───────────────────────┘  ──► report (JSON + Markdown)
```

- **Scenario** (`e2e/scenarios/*.json`, shared by all platforms): settings snapshot, key steps, expectations. Modelled on mozc's scenario files (`SEND_KEYS` / `SEND_KEY` / expectation lines). Sentences come from `corpus/taigi-typing` (quoted short, never a build input).
- **Driver**: the only per-platform part. Starts the device, installs the test build, focuses a text field, sends keys, reads the field back, pulls the trace.
- **Trace**: JSON Lines, one schema (`docs/architecture/e2e-trace-schema.md`, PR1). Two layers write it:
  - engine: one event per `process_request` — op, duration µs, error code, response size / candidate count;
  - platform: key received, composing op entered / left, commit text, candidate window shown, memory sample.
- **Analyzer** (`e2e/analyzer/analyze.py`, stdlib only): expectation diff, error / panic / invariant events, latency p50 / p95 / max per op against budgets in `e2e/budgets.json`, memory ceiling (iOS extension 64 MB). Output a Markdown report the agent reads, plus JSON for diffing against a stored baseline.

### D2 — Test mode is a compile-time build variant

| Platform | Switch | Release artifact proof |
|---|---|---|
| Engine (Rust) | cargo feature `e2e-trace` on `dispatch` (+ FFI crates re-export it); off by default | CI: `nm` / `strings` on the release `.a` / `.so` / `.dll` finds no `e2e_trace` symbol |
| iOS / macOS | Swift condition `E2E_TRACE` passed on the `xcodebuild` / `make` command line (`SWIFT_ACTIVE_COMPILATION_CONDITIONS='$(inherited) E2E_TRACE'`) — no project-file edit | release build never sets it |
| Android | Gradle build type `e2e` (`initWith(debug)`, `BuildConfig.E2E_TRACE = true`) | `release` type lacks the field's `true` branch; R8 strips it |
| Windows | cargo feature `e2e-trace` on the TSF crates | same `dumpbin` symbol check |
| Linux | cargo feature + CMake option `TAIGI_E2E_TRACE` | package targets build without it; CI `nm -D` check |

A runtime env var or setting is **deliberately not** the switch: the USER requirement is that the production IME cannot emit logs at all.

Codex revisions (2026-09-24), adopted:

- **The feature threads explicitly** `swift-ffi` / `android-jni` → `dispatch` and `taigi-windows-tsf` / `taigi-linux-ffi` / `taigikeyboard-ibus` → `taigi-desktop-core` → `dispatch`; Cargo does not forward a same-named feature on its own.
- **Traced artifacts never share a path with release ones.** `engine/scripts/build-xcframework.sh:39`, `build-android-libs.sh:33`, `build-macos-xcframework.sh:52` today write one fixed output; the traced build writes to a sibling (`RustTaigi-e2e.xcframework`, Android `src/e2e/jniLibs`), and only the e2e variant links it.
- **Sink = test-only init API** (`e2e_trace_open(path)`), present only when the feature is on; the platform resolves the sandbox path. An env var is a desktop-driver convenience at most — an OS-launched extension / IME never sees the driver's environment. `swift-ffi/src/lib.rs:25` rules out per-feature gating of bridge items; PR1 keeps one bridge with an always-present `e2e_trace_open` whose non-feature definition returns false (the `panic_for_test` precedent) — a second codegen'd bridge for one entry point costs more than the stub symbol. (The mobile openers were removed 2026-11-24 with no caller — `e2e-trace-schema.md` § Opening the trace.)
- **Proof of absence** = `cargo tree -e features` on the release graph (no `e2e-trace`) + a string marker (`TAIGI_E2E_TRACE_V1`) grepped in every shipped artifact per ABI, with a positive control on the traced build. Symbol checks alone miss inlined code under LTO + strip (`engine/Cargo.toml:55`).
- **Android** `e2e` build type: `initWith(debug)`, `applicationIdSuffix = ".e2e"`, coverage off, `testBuildType = "e2e"`; `BuildConfig.E2E_TRACE` declared `false` in `debug` / `release`.
- **macOS** builds with `swift build` (`macos/Makefile:44`) → `-Xswiftc -DE2E_TRACE`, not an Xcode setting.

The trace file lives in the platform's app sandbox (iOS extension container or App Group — Full Access dependent, decided in PR iOS; Android app files dir; macOS / Linux `$XDG_STATE_HOME`-style dir; Windows `%LOCALAPPDATA%`) and never leaves the device except when the driver pulls it.

**Production zero-log.** Linux release builds install no logger (`taigi_linux_platform::install_debug_logger` is debug-only), the Fcitx5 `TAIGI_DEBUG` macros compile out under `NDEBUG`, and panics stay silent. The other platforms compile logging out of release (`DEBUG` / `debug_assertions`).

#### Trace schema (Codex revisions)

`schema_version`; run / session / step IDs; engine `request.id` + `generation` (`engine/protos/proto/envelope.proto:136`); PID / TID; monotonic clock per process + a wall-clock anchor for cross-process alignment. Separate events for: key injected (driver), key received (IME), engine request / response, preedit / selection / effects, candidate list (`(Hanji, canonical-TL)` identity per AGENTS.md #6), commit call, **text observed by the host** (key-to-commit latency ends here), stale / dropped / cancelled, timeout / crash, trace flush status. Header records build id, dictionary version, settings snapshot, learning-data baseline. Memory samples name metric, PID, unit. Perf runs use the release profile plus the trace feature, and one run measures tracing overhead. Budgets are calibrated per device, not fixed (the iOS 64 MB cap is the one hard ceiling).

### D3 — Drivers

| Platform | Device | Key injection | Read-back | Where it runs |
|---|---|---|---|---|
| Linux Fcitx5 / IBus | distro container: Xvfb + dbus + `fcitx5` or `ibus-daemon` + a small GTK text client | `xdotool key` (X11 keysyms) | client writes its buffer to a file on exit | GitHub-hosted matrix: Ubuntu 24.04, Debian 13, Fedora 44, Arch × {Fcitx5, IBus}, building the test-mode binaries (`make install E2E=1`) inside each distribution's container, against its own toolchain and libraries — never a shipped package, which carries no trace |
| Linux desktop sessions (PR3c) | existing VMs: UTM GNOME (Wayland, IBus), VirtualBox KDE (X11, Fcitx5; gdm autologin needed) | in-guest uinput (python3-evdev, one virtual keyboard — works under Wayland and X11 alike, one driver for both VMs; `ydotool` 0.1.8 misparses key names) | the same GTK host window (`host.py`), launched in the session's environment, signals real keyboard focus through a marker file | local; `gnome-screenshot` after every step as evidence only; containers carry the distro matrix |

Scenarios share **intent** (text to type, candidate identity to pick, checkpoints); each driver maps intent to its own hardware keys or touches. Platform-specific scenarios stay allowed (desktop double-tap semantics differ, `desktop/crates/taigi-desktop-core/src/engine/bridge.rs:89`). No driver may set text directly in place of input.

### D4 — Agent entry point

`make e2e PLATFORM=<p> [SCENARIO=<glob>]` runs driver + analyzer and prints the report path; a `/e2e` project skill tells the agent how to read the report, rerun one scenario, and turn a failure into a bug report (observed failure → bugfix round per Core Principle #4). The skill never opens a round by itself.

## Spikes

A ≤20-line probe gated each platform driver (`docs/contributing/known-pitfalls.md` § Tests and diagnosis). Only the Linux one led to a driver:

- **Linux (PASS)**: `Xvfb` + `dbus-run-session` + `fcitx5` (isolated `HOME`, profile `DefaultIM=taigikeyboard`) + a GTK 3 `Gtk.Entry` host (`GTK_IM_MODULE=fcitx NO_AT_BRIDGE=1 GTK_USE_PORTAL=0 GDK_BACKEND=x11`) + `xdotool type` commits through the real addon. Readiness matters: the lexicon loads on the first key (~5 s on the VM).
- **Windows**: a `/IT` scheduled task from `ssh win` runs in the logged-in session, but injection needs an unlocked console. **Android**: keys are not accessibility nodes, so a driver would need a geometry manifest. **iOS**: no tap path into a third-party keyboard extension found. None was pursued (§ PR table).

## PR table

| Phase | Scope |
|---|---|
| PR1 | engine `e2e-trace` feature through every FFI crate, dispatch events, `e2e_trace_open`, marker string, release-graph absence check in `engine.yml`, [`e2e-trace-schema.md`](e2e-trace-schema.md) |
| PR2 | `e2e/scenarios` intent format, `e2e/analyzer/analyze.py` (stdlib) + tests, report format |
| PR3a | Linux: release zero-log (`taigi_linux_platform::install_debug_logger`, Fcitx5 `NDEBUG` macros), `e2e-trace` through `taigi-linux-core` / `-ffi` / `-ibus`, platform events (`key` / `preedit` / `commit` / `candidates` / `session_end`), `make build E2E=1` + package refusal + release guard |
| PR3b | Linux: driver `e2e/drivers/linux/driver.py` (Xvfb + D-Bus + Fcitx5 / IBus + GTK 3 host + xdotool, test-mode build in a private prefix), `make e2e PLATFORM=linux` → the UTM VM (`e2e/drivers/linux/run.sh`), CI `linux-e2e.yml`, `/e2e` skill |
| PR3c | Linux desktop sessions: `make e2e PLATFORM=linux-desktop` (`e2e/drivers/linux-desktop/run.sh`, `e2e/drivers/linux/desktop.py`, `uinput.py`) drives both VMs' logged-in sessions. IBus: `systemctl --user set-environment IBUS_COMPONENT_PATH` (system components minus ours + the test component) + private `XDG_CACHE_HOME` + unit restart; Fcitx5: the original daemon's argv + environ saved, test daemon `--replace` with the prefix addon dirs. A 0600 restore marker is written before anything changes; every run restores a leftover one first. VirtualBox runs in NEM mode → its own budget in `e2e/budgets.json` |
| PR4 | Linux CI matrix Ubuntu 24.04 / Debian 13 / Fedora 44 / Arch × {Fcitx5, IBus}, each container building `make install E2E=1` from source (packages are never traced; they keep their `install-check` job). Fedora × IBus: Fedora's rpm writes a system registry cache that ibus-daemon's default `--cache auto` takes before reading `IBUS_COMPONENT_PATH`, so the driver passes `--cache none`; the engine's socket-file lookup takes the first of `/var/lib/dbus/machine-id`, `/etc/machine-id` that reads, even when empty, as libibus does (`taigikeyboard-ibus` `bus.rs`) |
| PR9 | More scenarios (multi-pick commits, Enter after a pick, backspace + re-pick, separator kind, POJ, Caps Lock) |
| Other platforms | macOS, Windows, Android, iOS: no driver — tested by hand (maintainer decision; not to re-propose) |

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| Declarative key-sequence scenarios with expectations, one runner | `references/mozc/src/session/session_handler_scenario_test.cc`, `references/mozc/src/data/test/session/scenario/*.txt` | D1 scenario JSON |
| Headless IME test harness driving key events into an input context | `references/fcitx5/test/testquickphrase.cpp:36-57` (`testfrontend` `keyEvent` + `pushCommitExpectation`) | D3 Linux: same idea, but through a real X11 client so packaging + addon loading are exercised too |
| Debug-only logging compiled out of release | existing `Logging/DebugLogger.swift:3`, `AndroidLoggerBackend.kt:31`, `taigi-windows-platform/src/lib.rs:268` | D2 extends the same compile-time rule to the trace |

Rules: an e2e failure is an observed failure and goes through the normal bugfix path; a quantitative perf gate is adopted here because the maintainer asked for performance analysis.

### Deliberately not adopted

- **Runtime log switch in release builds** — violates the USER requirement; compile-time only.
- **In-process key injection as the primary driver** (feeding keys past the OS input stack) — skips exactly the layer where platform bugs live (TSF edit sessions, IMKit client, InputConnection). Allowed only if a spike proves the real path unreachable on that platform, recorded in the driver row.
- **Screenshot / OCR read-back** — the host app writes its own text; screenshots only for candidate-window layout questions, on demand.
- **Third-party test frameworks with a service** (Appium server, Detox) — one driver script per platform is smaller than an Appium stack for six key-injection paths.
- **`tracing` crate** — the engine uses `log` everywhere; one JSONL writer behind a feature is smaller than a subscriber stack.

## Decisions

1. A desktop-session run that finds its VM powered off, unreachable or locked reports `skipped` with the reason, never a failure.
2. The Linux CI matrix is scheduled only: nightly 19:00 UTC plus `workflow_dispatch`, no pull-request trigger (`.github/workflows/linux-e2e.yml`).
3. macOS, Windows, Android and iOS are tested by hand; no e2e driver is planned for them.
4. A screenshot per step is kept in the run dir as evidence, never asserted on.