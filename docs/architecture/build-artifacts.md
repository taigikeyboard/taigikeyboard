# Build artifacts — what is committed, what is generated, and why

> **Type**: Reference
> **Keywords**: `bootstrap`, `make build`, `make dict`, `dictionaries`, `fonts`, `stale-artifact gate`
> **Related**: ../BUILDING.md §4–5, data-artifacts-portability.md, system-overview.md §3

---

## Summary

- Engine binaries (the iOS / macOS xcframeworks with their swift-bridge wrappers, `jniLibs/*.so`) are **generated** by `make build` and gitignored (`.gitignore` § Rust shared-core platform artifacts).
- The platform proto bindings `make build` regenerates (`make protos`: Java under `android/app/src/main/java/…/engine/proto/`, `.pb.swift` under `ios/…/Engine/Generated/` and `macos/…/Generated/`) are **committed**, so only a `.proto` change needs protoc 36.2.
- The dictionary artifacts (`assets/dictionaries/`) and the typefaces (`assets/fonts/font/`) are **committed** and packaged natively by all five platforms.
- Commands per platform and the stale-artifact gate: [`../BUILDING.md`](../BUILDING.md) §4–5 (condensed in `AGENTS.md` § Build & Test). This file holds the rationale and the timings.

## Typefaces

All five platforms package `assets/fonts/font/` (Android through a `res` source dir in `android/app/build.gradle.kts`, the other four by copying it — Linux at `make -C linux install`), so nothing has to be staged before a build.

## Submodule

Clone with `--recurse-submodules`, or run `git submodule update --init --recursive` before `make dict`. The dictionary pipeline converts every reading through `taigi-converter`; `make dict` and `dictionary/common/taigi_bridge.py` both refuse to start without it, because without it every conversion returns an error string the pipeline would ingest as data.

## Dictionary artifacts

All five platforms package `assets/dictionaries/` the same way as the typefaces (Android through an `assets` source dir, iOS through an Xcode synchronized folder, macOS and Windows by copying it, Linux at `make -C linux install`).

They stay committed by maintainer decision (the `dictionary/` tree and its outputs are left as they are), not because of a technical limit: `make dict` reproduces them from a clean checkout. `dictionary/build.sh` writes them into `dictionary/output/`, where the four shipped files are untracked scratch (`dictionary.csv` and `walker_lm_stats.txt` there stay tracked), and its deploy step copies them to `assets/dictionaries/`.

## One pass per machine

Ignored files survive `git checkout`, so bootstrapping is one pass per machine, not per build. After that, re-run only what a change invalidates ([`../BUILDING.md`](../BUILDING.md) §5).

## Timings (measured 2026-09-07, warm machine, fresh clone with submodules)

| | Wall | Where it goes |
| --- | --- | --- |
| `make dict` | **~2 min** | `run.sh` 67 s across the nine per-source pipelines (`kautian` 26 s, `taihoa` 10 s; `extract` 15 s and `merge` 8 s dominate), then `build.sh` 53 s (`merge_csv` 16 s, `create_association_bin` + verify 13 s, `create_syllables_fst` 11 s, `create_fst` 7 s, `create_dictionary_bin` + verify 5 s) |
| `make build` | **5.1 s warm** | sequential (`protos` → `ios-libs` → `android-libs` → `macos-libs`); every cargo invocation reports `Finished` in 0.03–0.08 s against a warm target dir. A cold build compiles the engine for every shipped target and takes minutes. |

`make dict` must finish before `make build` when dictionary sources moved.

## Six Cargo workspaces (deliberate)

`engine/`, `desktop/`, `windows/`, `linux/`, `macos/` and `web/` are six workspaces, each with its own `Cargo.lock`, `target/` and `rust-toolchain.toml`; the five that are not the engine reach its crates by path, so each compiles its own copy. A single root workspace was evaluated and not adopted:

- The lockfiles do not conflict: of 272 external crates, 106 appear in two or more workspaces, and the only version differences are multi-version sets every workspace already carries (`syn` 1/2/3, `hashbrown`, `getrandom`), plus the Windows windows-rs two-island lock, which one resolver would keep. Merging is possible.
- What it would cost: ~140 `cargo` call sites across Makefiles, CI, the release scripts, the Windows box gate and the Linux VM sync; one `rust-toolchain.toml` carrying every platform's targets; the desktop version moving out of `[workspace.package]` (`tools/release_notes.py`); `windows/.cargo/config.toml` (static CRT) silently skipped by a root invocation; and one feature graph across all crates, which can change what `dispatch` and SQLite are built with.
- A shared `build.target-dir` instead saves less than it seems (each platform target compiles separately anyway) and makes one workspace's `cargo clean` or build lock everyone's.

`macos/` stays separate for three things a root workspace would take away: its release profile governs the shipped archive (`lto`, `strip`, `panic = "unwind"`) independently of whichever workspace it is built from; its engine edges set their own features (`default-features = false` + `user-data`), which one root feature graph would merge with the other shells'; and its toolchain lists only the two darwin targets the archive is built for.

Each workspace declares its own `rust-version`, and CI checks the floor itself, not only stable: engine 1.89 (`msrv` in `.github/workflows/engine.yml`, every engine change); desktop 1.93 (`msrv` in `desktop.yml`, every desktop or engine change) and linux 1.93 (`msrv` in `linux-build.yml`, nightly), kept at the rustc the latest Ubuntu ships so a distribution build can use it; macos 1.93, the floor of the desktop crates it links (`msrv` in `macos.yml`, every macOS, desktop or engine change). Windows stays at 1.95, windows-reactor's floor, and has no floor check: it builds only on the Windows box and the release runner. Web declares 1.93, the floor of the desktop crates it links, and has no floor check either: no distribution builds it.

Revisit when a shared dependency has to be bumped by hand in more than one lockfile.

## A release rebuilds first

`/release-mobile` and `/release-desktop` run `make i18n` + `make build` themselves (and `make dict` when dictionary sources moved): the engine binaries a platform links are generated and gitignored, so nothing else can prove the shipped artifact was built from the commit being released.
