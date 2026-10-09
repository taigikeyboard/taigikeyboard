# Build artifacts — what is committed, what is generated, and why

> **Type**: Reference
> **Keywords**: `bootstrap`, `make build`, `make dict`, `dictionaries`, `fonts`, `stale-artifact gate`
> **Related**: ../../AGENTS.md § Build & Test, data-artifacts-portability.md, system-overview.md §3

---

## Summary

- Engine binaries (xcframeworks, `jniLibs/*.so`, platform protos) are **generated** by `make build` and gitignored.
- The dictionary artifacts (`assets/dictionaries/`) and the typefaces (`assets/fonts/font/`) are **committed once** and packaged natively by all five platforms.
- `AGENTS.md` keeps only the bootstrap table and the stale-artifact gate; the rationale and the timings live here.

## Typefaces

The typefaces are committed, once, at `assets/fonts/font/` — all five platforms package that directory (Android through a `res` source dir in `android/app/build.gradle.kts`, the other four by copying it — Linux at `make -C linux install`), so nothing has to be staged before a build.

## Submodule

Clone with `--recurse-submodules`, or run `git submodule update --init --recursive` before `make dict`. `taigi-converter` is a submodule and the dictionary pipeline converts every reading through it; `make dict` and `dictionary/common/taigi_bridge.py` both refuse to start without it. Without the submodule `make dict` used to die in `cleanup.py:201` with a misleading `TypeError` (every conversion returned an error string that the pipeline ingested as data).

## Dictionary artifacts

The dictionary artifacts are committed, once, at `assets/dictionaries/` — all five platforms package that directory the same way the typefaces are (Android through an `assets` source dir, iOS through an Xcode synchronized folder, macOS and Windows by copying it, Linux at `make -C linux install`), so nothing has to be staged before a build.

They stay committed at all because that is the USER's standing instruction (2026-09-07: "do not touch the dictionary/ folder at all"), not a technical limit; `make dict` does reproduce them from a clean checkout: `dictionary/build.sh` writes them into `dictionary/output/`, where the four shipped files are untracked scratch (`dictionary.csv` and `walker_lm_stats.txt` there stay tracked), and `dictionary/build/deploy.sh` then copies them to `assets/dictionaries/`.

## One pass per machine

Ignored files survive `git checkout`, so bootstrapping is one pass per machine, not per build. After that, re-run only what a change invalidates — the stale-artifact gate table in `AGENTS.md` § Build & Test.

## Timings (measured 2026-09-07, warm machine, fresh clone with submodules)

| | Wall | Where it goes |
| --- | --- | --- |
| `make dict` | **~2 min** | `run.sh` 67 s across the nine per-source pipelines (`kautian` 26 s, `taihoa` 10 s; `extract` 15 s and `merge` 8 s dominate), then `build.sh` 53 s (`merge_csv` 16 s, `create_association_bin` + verify 13 s, `create_syllables_fst` 11 s, `create_fst` 7 s, `create_dictionary_bin` + verify 5 s) |
| `make build` | **5.1 s warm** | sequential, all six steps; every cargo invocation reports `Finished` in 0.03–0.08 s against a warm target dir. A cold build compiles the engine for five targets and takes minutes. |

`make dict` must finish before `make build` when dictionary sources moved.

## Five Cargo workspaces (deliberate)

`engine/`, `desktop/`, `windows/`, `linux/` and `macos/` are five workspaces, each with its own `Cargo.lock`, `target/` and `rust-toolchain.toml`; the four desktop ones reach the engine crates by path, so each compiles its own copy. A root workspace was spiked on 2026-10-01 and not adopted:

- The lockfiles do not conflict: of 272 external crates, 106 appear in two or more workspaces, and the only version differences are multi-version sets every workspace already carries (`syn` 1/2/3, `hashbrown`, `getrandom`), plus the Windows windows-rs two-island lock, which one resolver would keep. Merging is possible.
- What it would cost: ~140 `cargo` call sites across Makefiles, CI, the release scripts, the Windows box gate and the Linux VM sync; one `rust-toolchain.toml` carrying every platform's targets; the desktop version moving out of `[workspace.package]` (`tools/release_notes.py`); `windows/.cargo/config.toml` (static CRT) silently skipped by a root invocation; and one feature graph across all crates, which can change what `dispatch` and SQLite are built with.
- A shared `build.target-dir` instead saves less than it seems (each platform target compiles separately anyway) and makes one workspace's `cargo clean` or build lock everyone's.

`macos/` (2026-10-02, `macos-desktop-core-roadmap.md` D1 / D7) was re-checked against this decision and stays separate for three things a root workspace would take away: its release profile governs the shipped archive (`lto`, `strip`, `panic = "unwind"`) independently of whichever workspace it is built from; its engine edges set their own features (`default-features = false` + `user-data`), which one root feature graph would merge with the other shells'; and its toolchain lists only the two darwin targets the archive is built for.

Each workspace declares its own `rust-version`, and CI checks the floor itself, not only stable: engine 1.86 (`msrv` in `.github/workflows/engine.yml`, every engine change); desktop 1.93 (`msrv` in `desktop.yml`, every desktop or engine change) and linux 1.93 (`msrv` in `linux-build.yml`, nightly), kept at the rustc the latest Ubuntu ships so a distribution build can use it (#283); macos 1.93, the floor of the desktop crates it links (`msrv` in `macos.yml`, every macOS, desktop or engine change). Windows stays at 1.95, windows-reactor's floor, and has no floor check: it builds only on the Windows box and the release runner.

Revisit when a shared dependency has to be bumped by hand in more than one lockfile.

## A release rebuilds first

`/release-mobile` and `/release-desktop` run `make i18n` + `make build` themselves (and `make dict` when dictionary sources moved): the engine binaries a platform links are generated and gitignored, so nothing else can prove the shipped artifact was built from the commit being released.
