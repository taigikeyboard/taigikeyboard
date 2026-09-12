# Build artifacts — what is committed, what is generated, and why

> **Type**: Reference
> **Keywords**: `bootstrap`, `make build`, `make dict`, `dictionaries`, `fonts`, `stale-artifact gate`
> **Related**: ../../CLAUDE.md § Build & Test, data-artifacts-portability.md, ../roadmap.md § Repository size

---

## Summary

- Engine binaries (xcframeworks, `jniLibs/*.so`, platform protos) are **generated** by `make build` and gitignored.
- The dictionary artifacts (`dictionaries/`) and the typefaces (`fonts/font/`) are **committed once** and packaged natively by all four platforms.
- `CLAUDE.md` keeps only the bootstrap table and the stale-artifact gate; the rationale and the timings live here.

## Typefaces

The typefaces are committed, once, at `fonts/font/` — all four platforms package that directory (Android through a `res` source dir in `android/app/build.gradle.kts`, the other three by copying it), so nothing has to be staged before a build.

## Submodule

Clone with `--recurse-submodules`, or run `git submodule update --init --recursive` before `make dict`. `taigi-converter` is a submodule and the dictionary pipeline converts every reading through it; `make dict` and `dictionary/common/taigi_bridge.py` both refuse to start without it. Without the submodule `make dict` used to die in `cleanup.py:201` with a misleading `TypeError` (every conversion returned an error string that the pipeline ingested as data).

## Dictionary artifacts

The dictionary artifacts are committed, once, at `dictionaries/` — all four platforms package that directory the same way the typefaces are (Android through an `assets` source dir, iOS through an Xcode synchronized folder, macOS and Windows by copying it), so nothing has to be staged before a build.

They stay committed at all because that is the USER's standing instruction (2026-09-07: 「dictionary/ folder 都不要碰」), not a technical limit; `make dict` does reproduce them from a clean checkout: `dictionary/build.sh` writes them into `dictionary/output/`, where the four shipped files are untracked scratch (`dictionary.csv`, `corpus_total_freq.txt` and `.build_ts` there stay tracked), and `dictionary/build/deploy.sh` then copies them to `dictionaries/`.

## One pass per machine

Ignored files survive `git checkout`, so bootstrapping is one pass per machine, not per build. After that, re-run only what a change invalidates — the stale-artifact gate table in `CLAUDE.md` § Build & Test.

## Timings (measured 2026-09-07, warm machine, fresh clone with submodules)

| | Wall | Where it goes |
| --- | --- | --- |
| `make dict` | **~2 min** | `run.sh` 67 s across the nine per-source pipelines (`kautian` 26 s, `taihoa` 10 s; `extract` 15 s and `merge` 8 s dominate), then `build.sh` 53 s (`merge_csv` 16 s, `create_association_bin` + verify 13 s, `create_syllables_fst` 11 s, `create_fst` 7 s, `create_dictionary_bin` + verify 5 s) |
| `make build` | **5.1 s warm** | sequential, all six steps; every cargo invocation reports `Finished` in 0.03–0.08 s against a warm target dir. A cold build compiles the engine for five targets and takes minutes. |

`make dict` must finish before `make build` when dictionary sources moved.

## A release rebuilds first

`/release-mobile` and `/release-desktop` run `make i18n` + `make build` themselves (and `make dict` when dictionary sources moved): the engine binaries a platform links are generated and gitignored, so nothing else can prove the shipped artifact was built from the commit being released.
