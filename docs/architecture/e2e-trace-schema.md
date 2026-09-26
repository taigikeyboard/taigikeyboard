# E2E Trace Schema

> **Type**: Reference (contract between the test-build trace writers, the scenarios, the drivers and `tools/e2e/analyze.py`)
> **Keywords**: `e2e`, `trace`, `JSON Lines`, `test mode`, `schema_version`, `TAIGI_E2E_TRACE_V1`
> **Status**: Active — schema version 1 (engine layer PR1, scenarios + analyzer PR2 of `e2e-testing-roadmap.md`); platform-layer events are added by the platform PRs
> **Siblings**: `e2e-testing-roadmap.md` (design D1/D2, PR table)

---

## Test mode only

The trace exists only in builds compiled with the test switch: cargo feature `e2e-trace` on `engine/dispatch` (forwarded by `swift-ffi`, `android-jni` and, in their PRs, the desktop crates). A build without the feature holds no trace code, opens no file and writes nothing (USER 2026-09-24: the production IME emits no logs).

Proof that a shipped library is clean:

| Check | Where |
|---|---|
| Marker `TAIGI_E2E_TRACE_V1` (`dispatch::trace::MARKER`, `#[used]`) absent | `engine/scripts/lib/e2e-trace-guard.sh`, called by `build-xcframework.sh`, `build-macos-xcframework.sh`, `build-android-libs.sh` on every library they stage |
| `dispatch feature "e2e-trace"` absent from the shipped crates' feature graph | `.github/workflows/engine.yml` job `e2e-trace-guard` |
| Positive control: a traced release build DOES carry the marker | same job |

## Opening the trace

The platform resolves a path inside its own sandbox and calls, from test-build code only:

| Platform | Call |
|---|---|
| iOS / macOS | `e2e_trace_open(path)` (swift-bridge; returns false in a build without the feature) |
| Android | `RustEngineBridge.e2eTraceOpen(path)` (JNI symbol exported only with the feature) |
| Linux | `taigi-linux-core` opens `$XDG_DATA_HOME/taigikeyboard/e2e-trace.jsonl` in `Runtime::probe` (features `taigi-linux-ffi/e2e-trace`, `taigikeyboard-ibus/e2e-trace`; `make -C linux build E2E=1`) |
| Windows | `dispatch::trace::open(path)` through the desktop crates (its PR) |

The file is append-created; each `open` writes a `trace_open` line and restarts `t_us` at 0.

## Line format

JSON Lines, UTF-8, one object per line. Every line starts with the common fields:

| Field | Type | Meaning |
|---|---|---|
| `t_us` | integer | microseconds since the latest `trace_open` of this process (monotonic clock); for an `engine_request`, the moment the request entered `process_request` |
| `pid` | integer | OS process id |
| `tid` | integer | Rust thread id number (process-local) |
| `event` | string | event kind, below |

Readers ignore fields they do not know, so adding a field is not a version bump; changing a field's meaning is.

### `trace_open`

| Field | Meaning |
|---|---|
| `schema_version` | `1` |
| `marker` | `TAIGI_E2E_TRACE_V1` |
| `layer` | `engine` |
| `engine_version` | `dispatch` crate version |
| `wall_ms` | Unix time in milliseconds — anchors this process's `t_us` for cross-process alignment |

### `engine_request` — one per `dispatch::process_request`

| Field | Meaning |
|---|---|
| `req_id`, `generation` | from the `Response` (`envelope.proto` `Request.id` / `generation`); 0 when the request did not decode |
| `domain` | `phonetics` / `composing` / `lexicon` / `nextword` / `case` / `userdata` (the `Request.payload` field, tags 10–15), `none` when absent or undecodable |
| `method_tag` | field number of the sub-request's `oneof method` (every sub-request holds only that oneof); the analyzer names it from `engine/protos/proto/*.proto` |
| `error` | `ErrorCode` value (0 = OK) |
| `dur_us` | decode + dispatch + encode time; excludes the trace write |
| `req_bytes`, `resp_bytes` | encoded sizes |

### `engine_panic`

A panic caught at the dispatch boundary: `domain`, `method_tag`, `req_bytes`. The caller still receives a `FAIL_INTERNAL` response.

### `adapter_reject`

A request an FFI adapter refused before dispatch: `reason` (`oversize`), `req_bytes`.

## Platform-layer events

Written through `dispatch::trace::event` (same file, same clock as the engine events; string values escaped by `dispatch::trace::JsonStr`). Linux writes them from `taigi-linux-core::trace`, the one path both shells share (`process_raw_key`), so Fcitx5 and IBus trace identically.

| Event | Fields | Linux source |
|---|---|---|
| `key` | `keyval` (X keysym), `state` (modifier mask), `handled` | every key the framework hands in |
| `preedit` | `text` (`""` = cleared) | `Emit::Preedit` / `Emit::ClearPreedit` |
| `commit` | `text` | `Emit::Commit` — what the IME asked the client to insert; the host's own read-back is the driver's `observed_text` |
| `candidates` | `items` = `[{"hanji", "tl", "canonical_tl"}]`, display order: `hanji` (`""` for a romanization-only cell), `tl` = the reading as displayed, `canonical_tl` = identity (CLAUDE.md Core Principle #6) | `Emit::LookupTable` for the candidate list (the symbol picker is not traced) |
| `session_end` | `source` | the daemon ended the composition (focus out, reset, disable) |

The analyzer's "first hanji candidate" is the first `candidates` item whose `hanji` holds a CJK character (the §34 literal slot is skipped) and compares its displayed `tl`, plus its `hanji` when the scenario names one (`analyze.matches_cell`).

Still planned, specified by the PR that first writes them: memory sample, key-geometry manifest (mobile).

## Scenarios

`e2e/scenarios/<id>.json`, shared by every driver; the file name is the `id`. Expectations come from an authoritative source (a USER-quoted dogfood item, `knowledge/`), named in `source` — never from a run's output.

| Key | Meaning |
|---|---|
| `id`, `source` | identity; where the expectation comes from |
| `settings` | intent-level settings (`romanization`, `continuous_input`, `output`); each driver maps them to its platform's store |
| `steps` | `text` (type these characters), `key` (a platform-neutral name: `enter`, `space`, `backspace`, `escape`, `capslock`, `0`–`9`; each driver translates it, and a driver leaves Caps Lock off after a scenario that toggled it), `pick` (select the hanji candidate showing this `tl`, and this `hanji` when given). A driver maps each step to real input — hardware keys or taps — and never sets text directly |
| `expect.committed` | the exact text the host field holds at the end |
| `expect.first_hanji_candidate` | `{tl[, hanji]}` of the first hanji candidate in the last `candidates` event — `hanji` only when the source names one (never taken from a run) |

## Run layout (driver → analyzer)

```
<run-dir>/<platform>/<scenario-id>/result.json   {"status": "ran" | "skipped" | "error", "reason": …, "observed_text": …}
<run-dir>/<platform>/<scenario-id>/*.jsonl       every trace file the scenario produced (engine + platform processes)
```

`<platform>` is the driver's name for what it drove: `linux-fcitx5`, `linux-ibus` (Xvfb, one directory per framework), `linux-gnome-ibus`, `linux-kde-fcitx5` (real desktop sessions; each scenario dir also holds `shot-<step>.png`, or `shot-<step>.missing` with the reason, as evidence the analyzer ignores). Entry point: `make e2e PLATFORM=<platform>` (runs `tools/e2e/<platform>/run.sh`, then the analyzer; skill `/e2e`).

`python3 tools/e2e/analyze.py --run <run-dir> [--baseline <earlier report.json>]` writes `report.md` (read by the agent) and `report.json` (baseline for the next run). Scenario status: `passed` / `failed` (expectation mismatch) / `skipped` (driver could not run, e.g. Windows box off — USER 2026-09-24) / `error`. Findings: `bug` (engine error / panic / adapter reject), `perf` (over `e2e/budgets.json` — platform → op, `*` wildcards, most specific wins — or p95 ≥ 1.5× and +2 ms vs baseline), `trace` (missing or wrong header, unparsable line), `unverified` (an expectation whose events the platform does not write yet). Exit 1 on any failed / error scenario or bug / perf / trace finding.
