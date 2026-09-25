# End-to-End Test System — Roadmap

> **Type**: Planning (design record + PR table; becomes Reference once shipped)
> **Keywords**: `e2e`, `test mode`, `trace`, `simulator`, `emulator`, `VM`, `container`, `scenario`, `perf`, `Xvfb`, `xdotool`, `UiAutomator`
> **Status**: Linux live (PR1–PR3b merged 2026-09-24: `make e2e PLATFORM=linux`, CI `linux-e2e.yml`, skill `/e2e`); PR3c `make e2e PLATFORM=linux-desktop` (branch `feat/e2e-linux-desktop`), PR4–PR8 pending; real-desktop-session goal added 2026-09-24 (§ Goal 5). Phase 0 plan written 2026-09-24; Codex ANALYSIS-ONLY pre-review 2026-09-24 = REVISE (revisions applied below); spikes S-A–S-D run 2026-09-24 (§ Spikes — results).
> **Session memory**: project memory `project_e2e_test_system.md` (Claude auto-memory)

---

## Goal

USER 2026-09-24: 「我需要一個end-to-end的自動化測試系統,能夠讓AI開啟模擬機測試輸入，並且分析debug log評估bug,效能等issue,debug log只在測試模式可見,因為輸入法在正式環境不適合發送任何log,for全平台,包含linux各個發行版本」

1. An AI agent starts a simulator / emulator / VM / container, installs a **test build** of the IME, types real key sequences into a real text field, and reads back the committed text.
2. The test build writes a **structured trace** (JSON Lines). An analyzer turns trace + expectations into a report: wrong commit, engine errors / panics, latency per operation, memory.
3. **The trace exists only in test builds.** Release artifacts contain no trace code and write no trace file — enforced by build-script and CI checks (marker string + feature graph), not by a runtime switch. The one exception is swift-bridge's always-present `e2e_trace_open` stub, which returns false (bridge items cannot be cfg-gated, same as `panic_for_test`).
4. Every platform: iOS, Android, macOS, Windows, Linux (Fcitx5 + IBus) on every packaged distribution (`.deb` Ubuntu / Debian, `.rpm` Fedora, `.pkg.tar.zst` Arch).
5. **Real desktop sessions beside CI.** USER 2026-09-24: 「除了CI上執行的e2e以外,我希望能夠真實在linux VM,macos,windows開啟模擬器測試」. The headless Xvfb run (CI + `make e2e PLATFORM=linux`) stays; the desktop drivers type into the logged-in session of: both Linux VMs (UTM GNOME Wayland + IBus, VirtualBox KDE X11 + Fcitx5). Windows: USER 2026-09-24 「skip windows」 — no Windows e2e driver. macOS: USER 2026-09-24 「skip mac, macos我自己手動測試就好」 — no macOS e2e driver; macOS stays manual dogfood. USER decisions 2026-09-24: both Linux VMs; a screenshot per checkpoint is kept as evidence in the run dir, never asserted on.

## Today (grounded in code)

| Piece | Where | State |
|---|---|---|
| Engine choke point | `engine/dispatch/src/lib.rs:34` `process_request` → `run` `:55` | every platform's engine call passes here (iOS/macOS `swift-ffi/src/lib.rs:50`, Android `android-jni/src/lib.rs:35`, Windows/Linux `desktop/crates/taigi-desktop-core/src/engine/bridge.rs:43`) |
| Engine logging | `log` 0.4; sinks `swift-ffi/src/lib.rs:64-106`, `android-jni/src/lib.rs:85-142` (runtime level, default Warn) | free-text, not structured; runtime-gated, not compile-gated |
| iOS / macOS logging | `ios/Sources/TaigiKeyboard/DebugLogger.swift:3`, `macos/Sources/TaigiInputMethodCore/Logging/DebugLogger.swift` | `#if DEBUG` `os.Logger`; no-op in release |
| Android logging | `AndroidLoggerBackend.kt:31-62`, `TraceContext.kt:16-30` | `BuildConfig.DEBUG` gated |
| Windows logging | `windows/crates/taigi-windows-platform/src/lib.rs:268-294` | `cfg(debug_assertions)` → `OutputDebugStringW` |
| Linux logging | `linux/crates/taigikeyboard-ibus/src/main.rs:20` (`env_logger`, `RUST_LOG`), `linux/fcitx5/src/engine.cpp:27-29` (`TAIGI_DEBUG`) | **runtime-gated only — present in release** |
| Desktop composing ops | `desktop/crates/taigi-desktop-core/src/composing/manager.rs` (`append` :142 … `commit_candidate` :377); Linux C API `linux/crates/taigi-linux-ffi/src/lib.rs:413` `taigi_engine_key` | platform-side op boundary for key → commit events |
| UI / e2e tests | none; only the CI ibus-daemon smoke `.github/workflows/linux-build.yml:124-165` (xvfb + dbus, selects the engine, types nothing) | — |
| Distro containers in CI | `linux-build.yml:229-230` (`fedora:44`), `:280-281` (`archlinux:latest`), install-check matrix `:320-340` | x86_64, hosted |
| Dogfood rigs | Windows box `ssh win` (session 0, elevated); VirtualBox Ubuntu KDE VM; UTM arm64 Ubuntu GNOME VM (memory `reference_linux_dogfood_vms.md`) | manual, no scripts in repo |

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
- **Analyzer** (`tools/e2e/analyze.py`, stdlib only): expectation diff, error / panic / invariant events, latency p50 / p95 / max per op against budgets in `e2e/budgets.json`, memory ceiling (iOS extension 64 MB). Output a Markdown report the agent reads, plus JSON for diffing against a stored baseline.

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
- **Sink = test-only init API** (`e2e_trace_open(path)`), present only when the feature is on; the platform resolves the sandbox path. An env var is a desktop-driver convenience at most — an OS-launched extension / IME never sees the driver's environment. `swift-ffi/src/lib.rs:25` rules out per-feature gating of bridge items; PR1 keeps one bridge with an always-present `e2e_trace_open` whose non-feature definition returns false (the `panic_for_test` precedent) — a second codegen'd bridge for one entry point costs more than the stub symbol.
- **Proof of absence** = `cargo tree -e features` on the release graph (no `e2e-trace`) + a string marker (`TAIGI_E2E_TRACE_V1`) grepped in every shipped artifact per ABI, with a positive control on the traced build. Symbol checks alone miss inlined code under LTO + strip (`engine/Cargo.toml:55`).
- **Android** `e2e` build type: `initWith(debug)`, `applicationIdSuffix = ".e2e"`, coverage off, `testBuildType = "e2e"`; `BuildConfig.E2E_TRACE` declared `false` in `debug` / `release`.
- **macOS** builds with `swift build` (`macos/Makefile:44`) → `-Xswiftc -DE2E_TRACE`, not an Xcode setting.

The trace file lives in the platform's app sandbox (iOS extension container or App Group — Full Access dependent, decided in PR iOS; Android app files dir; macOS / Linux `$XDG_STATE_HOME`-style dir; Windows `%LOCALAPPDATA%`) and never leaves the device except when the driver pulls it.

**Production zero-log (in scope, Codex 2026-09-24).** Linux release builds log today: `taigi-linux-ffi/src/lib.rs:177` and `taigikeyboard-ibus/src/main.rs:20` install an `env_logger` at `info` by default (spike S-D saw `runtime.probe`, `lexicon.installed`, `key.claim`, `storage.open` lines in the Fcitx5 log of the released 3.6.9 build). The USER requirement covers them: release Linux builds install no logger, Fcitx `TAIGI_DEBUG` compiles out, panics stay silent. The other platforms already compile logging out of release (`DEBUG` / `debug_assertions`) and are unchanged.

#### Trace schema (Codex revisions)

`schema_version`; run / session / step IDs; engine `request.id` + `generation` (`engine/protos/proto/envelope.proto:136`); PID / TID; monotonic clock per process + a wall-clock anchor for cross-process alignment. Separate events for: key injected (driver), key received (IME), engine request / response, preedit / selection / effects, candidate list (`(漢字, canonical-TL)` identity per CLAUDE.md #6), commit call, **text observed by the host** (key-to-commit latency ends here), stale / dropped / cancelled, timeout / crash, trace flush status. Header records build id, dictionary version, settings snapshot, learning-data baseline. Memory samples name metric, PID, unit. Perf runs use the release profile plus the trace feature, and one run measures tracing overhead. Budgets are calibrated per device, not fixed (the iOS 64 MB cap is the one hard ceiling).

### D3 — Drivers

| Platform | Device | Key injection | Read-back | Where it runs |
|---|---|---|---|---|
| Linux Fcitx5 / IBus | distro container: Xvfb + dbus + `fcitx5` or `ibus-daemon` + a small GTK text client | `xdotool key` (X11 keysyms) | client writes its buffer to a file on exit | GitHub-hosted matrix: Ubuntu 24.04, Debian 13, Fedora 44, Arch × {Fcitx5, IBus}, building the test-mode binaries (`make install E2E=1`) inside each distribution's container, against its own toolchain and libraries — never a shipped package, which carries no trace |
| Linux desktop sessions (PR3c) | existing VMs: UTM GNOME (Wayland, IBus), VirtualBox KDE (X11, Fcitx5; gdm autologin needed) | in-guest uinput (python3-evdev, one virtual keyboard — works under Wayland and X11 alike, one driver for both VMs; `ydotool` 0.1.8 misparses key names) | the same GTK host window (`host.py`), launched in the session's environment, signals real keyboard focus through a marker file | local; `gnome-screenshot` after every step as evidence only; containers carry the distro matrix |
| macOS | this Mac, test-build IME installed to `~/Library/Input Methods` | CGEvent from a small Swift driver into a test host window | host writes its text view to a file | local only (needs one-time Accessibility grant; takes keyboard focus for the run); `screencapture` per checkpoint as evidence |
| Windows | `ssh win` box | `SendInput` from a driver launched **in the interactive session** (scheduled task `/IT`), not the ssh session 0 | host writes its edit control to a file | local box; locked / off → `skipped`; screenshot per checkpoint as evidence |
| Android | emulator (dedicated e2e AVD, ≥4 GB RAM) | test build writes a **key-geometry manifest** (each laid-out key's rect) to the trace; driver taps those points with `adb shell input tap` — the real `MotionEvent` path through `KeyTouchCoordinator` | host test activity in the e2e APK writes its `EditText` | local; CI later if cheap |
| iOS | simulator iPhone 17 | per S-A: XCUITest coordinate taps (needs a UI test target the USER adds in Xcode) or `idb`; key points from the same geometry manifest | containing app, `E2E_TRACE`-only text field made first responder on launch | local |

Scenarios share **intent** (text to type, candidate identity to pick, checkpoints); each driver maps intent to its own hardware keys or touches. Platform-specific scenarios stay allowed (desktop double-tap semantics differ, `desktop/crates/taigi-desktop-core/src/engine/bridge.rs:89`). No driver may set text directly in place of input.

### D4 — Agent entry point

`make e2e PLATFORM=<p> [SCENARIO=<glob>]` runs driver + analyzer and prints the report path; a `/e2e` project skill tells the agent how to read the report, rerun one scenario, and turn a failure into a bug report (observed failure → bugfix round per Core Principle #4). The skill never opens a round by itself.

## Spikes (gate the platform PRs — `taigi-incidents.md` § spike a platform capability)

| # | Question | ≤20-line probe | Blocks |
|---|---|---|---|
| S-A | Can an automated driver tap keys of a **third-party keyboard extension** in the iOS simulator, and can the keyboard be enabled without the Settings UI? | `simctl spawn defaults write … AppleKeyboards`; `idb ui describe-all` over Notes with the keyboard up | PR8 |
| S-B | Does UiAutomator see FlorisBoard-derived Compose keys (content descriptions / bounds), or only one opaque view? | `uiautomator dump` with the keyboard up | PR7 |
| S-C | Can a process started from `ssh win` inject keys into the logged-in desktop through a `/IT` scheduled task and read Notepad back? | `schtasks /create /it … ` + a 10-line PowerShell `SendKeys` | PR6 |
| S-D | Does Fcitx5 run headless in a container under Xvfb and commit into a GTK entry via `xdotool`? | `fedora:44` / `archlinux` / `ubuntu:24.04` container, `fcitx5 -d`, `xdotool type` | PR3 |

A failed spike changes the driver row, never the trace / analyzer design.

### Spike results (2026-09-24)

| # | Result | Evidence |
|---|---|---|
| S-D Linux | **PASS (mechanism)** | UTM arm64 Ubuntu 24.04, released 3.6.9 addon: `Xvfb :99` + `dbus-run-session` + `fcitx5` (isolated `HOME`, profile `DefaultIM=taigikeyboard`) + GTK 3 `Gtk.Entry` host (`GTK_IM_MODULE=fcitx NO_AT_BRIDGE=1 GTK_USE_PORTAL=0 GDK_BACKEND=x11`) + `xdotool type` → host saw `preedit 't'→'ta'→'tai'`, Return committed `tai`. Readiness matters: the lexicon loads on first key (~5 s on the VM). **Unconfirmed observation:** Space (`CommitAlternateScript`, `linux/crates/taigi-linux-core/src/session.rs:581`) produced no commit in this harness even after the lexicon loaded — harness artifact or bug; not a round until reproduced on a real session (S74). Containers (distro matrix) not yet run — no container runtime on the Mac; they run in CI. |
| S-C Windows | **PASS with blocker** | `schtasks /create /it /ru minsi` runs in session 1 with focus; `SendKeys` text was dropped because the console was **locked** (`LogonUI` in session 1). Injection needs an unlocked session during runs. Real driver uses `SendInput`, not `SendKeys`. |
| S-B Android | **PARTIAL** | Debug APK installs; `ime enable` / `ime set` over adb work; keyboard shows (`onWindowShown`). Keys are not accessibility nodes (`KeyContent.kt:215` null description; touches go through one `KeyTouchCoordinator`) → geometry-manifest driver. The stock AVD (2.5 GB RAM) hit repeated system ANRs (load average 61) → the dedicated e2e AVD needs more RAM. |
| S-A iOS | **BLOCKED on a USER decision** | App + extension build and install; `defaults write -g AppleKeyboards` over `simctl spawn` is accepted (keyboard appearance unverified — no tap path yet). This Xcode ships no `Simulator.app` (only `SharedFrameworks/SimulatorKit.framework`), so no GUI click path; tap injection needs an XCUITest target (project-file edit = USER-only, Core Principle #1) or `idb` (third-party, Xcode 27 support unknown). |

## PR table

PR1/PR2 of the first draft were over the 500-LOC cap (Codex) — split below.

| Phase | Scope | Status |
|---|---|---|
| 0 | this roadmap + project memory + `docs/README.md` row | this commit |
| PR1 | engine `e2e-trace` feature threaded through every FFI crate, dispatch events, `e2e_trace_open`, marker string, traced-artifact build paths, release-graph absence check in `engine.yml`, `e2e-trace-schema.md` | Merged #162 `18a20c80` 2026-09-24 |
| PR2 | `e2e/scenarios` intent format + 3 seed scenarios, `tools/e2e/analyze.py` (stdlib) + tests, report format | Merged #163 `5fc7b825` 2026-09-24 |
| PR3a | Linux: release zero-log (`taigi_linux_platform::install_debug_logger`, Fcitx5 `NDEBUG` macros), `e2e-trace` through `taigi-linux-core` / `-ffi` / `-ibus`, platform events (`key` / `preedit` / `commit` / `candidates` / `session_end`), `make build E2E=1` + package refusal + release guard | Merged #165 `34ae8d27` 2026-09-24 |
| PR3b | Linux: driver `tools/e2e/linux/driver.py` (Xvfb + D-Bus + Fcitx5 / IBus + GTK 3 host + xdotool, test-mode build in a private prefix), `make e2e PLATFORM=linux` → UTM VM (`tools/e2e/linux/run.sh`), CI `linux-e2e.yml` (Ubuntu 24.04 runner), `/e2e` skill | Merged #166 `26c03a34` 2026-09-24 (VM + CI 6/6 PASS) |
| PR3c | Linux desktop sessions: `make e2e PLATFORM=linux-desktop` (`tools/e2e/linux-desktop/run.sh`, `tools/e2e/linux/desktop.py`, `uinput.py`) drives both VMs' logged-in sessions. Framework swap (spike + Codex pre-review 2026-09-24, CONFIRM with revisions): IBus = `systemctl --user set-environment IBUS_COMPONENT_PATH` (system components minus ours + the test component, whose `<exec>` sets `IBUS_ADDRESS` then the scenario's private XDG dirs) + private `XDG_CACHE_HOME` + unit restart; Fcitx5 = original daemon's argv + environ saved, test daemon `--replace` with the prefix addon dirs and private XDG dirs. A 0600 restore marker is written before anything changes; every run restores a leftover one first. First runs 2026-09-24: GNOME 3/3 + KDE 3/3 PASS, both restored; VBox runs in NEM mode (VBS holds VT-x) → own calibrated budget in `e2e/budgets.json` | Merged #169 `1fde108f` 2026-09-24 |
| PR4 | Linux: CI matrix Ubuntu 24.04 / Debian 13 / Fedora 44 / Arch × {Fcitx5, IBus}, each container building `make install E2E=1` from source against its own libraries (packages are never traced — PR3a; the shipped packages keep their `install-check` job). `linux-e2e.yml` becomes one container matrix (`linux-e2e-<distro>` artifacts; deps `tools/e2e/linux/ci-deps.sh`). USER 2026-09-24: PRs run Ubuntu only, the full matrix runs nightly 19:00 UTC (台灣 03:00) + on dispatch. First full runs: Ubuntu / Debian / Arch × both frameworks and Fedora × Fcitx5 PASS; **Fedora × IBus skipped** — its ibus-daemon lists only the system xkb engines, never the test component (`IBUS_COMPONENT_PATH` replaces the search path, `ibus/src/ibusregistry.c:262-286`; waiting for the daemon's address file did not help). USER 2026-09-24: skip it here, root-cause it in its own round | Merged #173 `6559488c` 2026-09-24 |
| PR5 | macOS: `-DE2E_TRACE` events, CGEvent driver + host app | Dropped — USER 2026-09-24 「skip mac, macos我自己手動測試就好」 (would need a traced xcframework, a renamed E2E bundle beside the dev install, Swift platform events over a new FFI, a TIS + CGEvent driver and an Accessibility grant) |
| PR6 | Windows: TSF events, `/IT` interactive-session `SendInput` driver over `ssh win` | Dropped — USER 2026-09-24 「skip windows」, after PR6a's first box build hit `os error 4551` (Windows application control blocked a build binary). Unmerged branch `feat/e2e-windows-trace` deleted 2026-09-24 (USER) |
| PR7 | Android: `e2e` build type, geometry manifest, adb tap driver, e2e AVD | Not planned — USER 2026-09-24 「PR7,8不需要,我暫時可以手動測試」 |
| PR8 | iOS: `E2E_TRACE` events in the extension, geometry manifest, XCUITest driver | Not planned — USER 2026-09-24 「PR7,8不需要,我暫時可以手動測試」 |
| PR9 | More scenarios (USER 2026-09-25 「ok,plan包含更多情境的測試」): 10 from S68 / S69 / S70 / S73 — multi-pick commits, Enter after a pick, backspace + re-pick, learned phrase keeps the separator kind, POJ, Caps Lock; `capslock` key name (the driver releases the lock after the scenario — the display outlives it). Not reachable with today's steps: S1 (no concrete I/O), S2 (desktop has no TPS mode), S62 (`kikhilai` offers no single 記 cell), settings beyond romanization / output, custom-entry seeding, highlight / paging | Merged #192 `ca4ff0f8` 2026-09-25; UTM VM 26/26 PASS |

Each platform PR adds its row to `/e2e` and its budgets; PR sizes 200–500 LOC.

## Best practices alignment (最佳實踐對齊)

| Mainstream practice | Source | This plan |
|---|---|---|
| Declarative key-sequence scenarios with expectations, one runner | `references/mozc/src/session/session_handler_scenario_test.cc`, `references/mozc/src/data/test/session/scenario/*.txt` | D1 scenario JSON |
| Headless IME test harness driving key events into an input context | `references/fcitx5/test/testquickphrase.cpp:36-57` (`testfrontend` `keyEvent` + `pushCommitExpectation`) | D3 Linux: same idea, but through a real X11 client so packaging + addon loading are exercised too |
| Debug-only logging compiled out of release | existing `DebugLogger.swift:3`, `AndroidLoggerBackend.kt:31`, `taigi-windows-platform/src/lib.rs:268` | D2 extends the same compile-time rule to the trace |

Rules: `~/.claude/rules/planning.md` (roadmap + memory, grounded, PR sizing); `diagnosis-discipline.md` (an e2e failure is an observed failure → normal bugfix pre-gate); `code-review-rules.md` §9 (quantitative perf gate adopted here because the USER asked for 效能 analysis).

### Deliberately not adopted

- **Runtime log switch in release builds** — violates the USER requirement; compile-time only.
- **In-process key injection as the primary driver** (feeding keys past the OS input stack) — skips exactly the layer where platform bugs live (TSF edit sessions, IMKit client, InputConnection). Allowed only if a spike proves the real path unreachable on that platform, recorded in the driver row.
- **Screenshot / OCR read-back** — the host app writes its own text; screenshots only for candidate-window layout questions, on demand.
- **Third-party test frameworks with a service** (Appium server, Detox) — one driver script per platform is smaller than an Appium stack for six key-injection paths.
- **`tracing` crate** — the engine uses `log` everywhere; one JSONL writer behind a feature is smaller than a subscriber stack.

## USER decisions (2026-09-24)

USER 2026-09-24: 「Ios我晚點加、windows如果關機就skip，其他go」

1. iOS: the USER adds an XCUITest target in Xcode later; PR8 waits for it.
2. Windows: the driver probes the box first; powered off, unreachable or locked console → the run reports `skipped` with the reason, never a failure.
3. Linux CI matrix: runs on PRs touching `linux/`, `engine/` or `e2e/`, plus `workflow_dispatch`. Superseded USER 2026-09-25 「全部都改成排程執行」: no PR trigger; full matrix nightly 19:00 UTC + `workflow_dispatch` only.
