# Keyboard Extension Memory Baseline — 2026-04

**Status** (revised 2026-04-19, qualitative-gate philosophy ratified by `feedback_perf_gate.md`): methodology frozen, **quantitative capture deferred**. Phase I G0 adopted a qualitative gate (no keyboard dismiss = 64 MB termination signal, plus no progressive memory growth during an extended typing session). Since refactor-only slices should not change steady-state capacity, the gate is leak-free, not a number.

**Why deferred**: solo-dev IME cadence. The iOS platform already enforces a hard 64 MB cap on keyboard extensions — if a refactor breaks capacity headroom, the keyboard visibly dismisses and the user notices immediately. Persistent Instruments capture adds overhead without catching anything the platform cap does not already catch.

**When to revive this doc**: a specific memory regression is suspected, OR the extension approaches the cap from eager loading, OR team workflow justifies the Allocations capture cost.

**Original gating signal** (kept for reference; currently inactive): peak resident memory on every sequence below must be ≤ the baseline. Track both peak RSS and headroom relative to the 64 MB cap.

**Device contract**: baseline is captured on a **real iOS device** (simulator memory accounting is misleading for extensions). Record device + iOS version; re-measure only on the same device class when comparing.

---

## 1. Measurement sequences

Same three sequences as the latency baseline (`keyboard-baseline-2026-04.md` §1). Memory is measured **across the entire session**, not per keystroke — the goal is peak resident set.

| # | Sequence | Memory hot spots exercised |
|---|---|---|
| S1 | POJ diacritic composition (`gua2si7soo`, repeated) | `ComposingManager` state, SwiftUI view tree, tone-mark formatter caches |
| S2 | TPS composition (`ㄍㄨㄚˋ`, repeated) | TPS tables kept resident, converter closures |
| S3 | Hanji candidate scroll (type `gua` + swipe 3 pages) | fst pages faulted in, candidate view recycling, SQLite page cache |

An additional reading is taken **at rest** (keyboard activated, zero keystrokes) to isolate load-time cost from interaction cost.

---

## 2. Methodology (manual testing on real device)

### Option A — Xcode Debug Navigator (preferred, no code changes)

1. Build and run the **host app + keyboard extension** on a real device in **Release** config (Debug skews allocations).
2. In the host app's input field, activate the Taigi keyboard.
3. In Xcode, **Debug Navigator → Memory** — but Xcode only attaches to the host process by default. To attach to the keyboard extension:
   - **Debug → Attach to Process by PID or Name** → pick `TaigiKeyboard` (the extension bundle name). The Memory gauge then tracks the extension.
4. Read peak "Memory (MB)" during S1/S2/S3 from the graph. Record separately per sequence.

### Option B — Instruments "Allocations" template

1. Run Instruments → **Allocations**.
2. **File → Record Options → Choose Target** → attach to the keyboard extension process (same name lookup as Option A).
3. Run each sequence; record **Persistent Bytes** peak and **All Heap & Anonymous VM** peak.
4. Option B is richer but slower to set up; Option A is enough for a Phase I baseline.

### Option C — `task_info()` in-process (future, not required for baseline)

If the extension later wires an internal probe:

```swift
func currentResidentBytes() -> UInt64 {
    var info = mach_task_basic_info()
    var count = mach_msg_type_number_t(MemoryLayout<mach_task_basic_info>.size / MemoryLayout<integer_t>.size)
    let kerr: kern_return_t = withUnsafeMutablePointer(to: &info) {
        $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
            task_info(mach_task_self_, task_flavor_t(MACH_TASK_BASIC_INFO), $0, &count)
        }
    }
    return kerr == KERN_SUCCESS ? info.resident_size : 0
}
```

Log peak via `LoggerBackend`. Do not ship this probe in a release build.

### Protocol

1. Launch host app, activate Taigi keyboard. Wait ~5 s for post-activation settling.
2. **At-rest reading**: snapshot memory.
3. Run S1 for 30 s of typing. Snapshot peak.
4. Pause 10 s. Run S2. Snapshot peak.
5. Pause 10 s. Run S3. Snapshot peak.
6. Final reading: 10 s idle after S3 (to observe any steady leak).

---

## 3. Baseline captures

> **Fill in after each manual measurement session. Do not delete previous captures — each row is a historical baseline.**

### Run 1 — TBD (first capture)

| Field | Value |
|---|---|
| Date | `YYYY-MM-DD` |
| Device | e.g. `iPhone 12`, `iPhone 14 Pro` |
| iOS version | e.g. `17.4.1` |
| Low-power mode | `off` / `on` |
| Thermal state | `nominal` / `fair` / `serious` |
| Build config | `Release` (strongly preferred) |
| Measurement option | `A (Debug Navigator)` / `B (Allocations)` |
| Host app | `Notes.app` / `Messages.app` / other |
| Branch / commit SHA | `main @ <sha>` |

| Reading | Peak RSS (MB) | Headroom vs 64 MB (MB) | Notes |
|---|---|---|---|
| At-rest (post-activation) | TBD | TBD | |
| S1 — POJ diacritics | TBD | TBD | |
| S2 — TPS composition | TBD | TBD | |
| S3 — Hanji candidate scroll | TBD | TBD | |
| Post-S3 idle (10 s) | TBD | TBD | Check against at-rest for leaks |

### Peak across all sequences

| Overall peak | TBD MB |
| Overall headroom | TBD MB |

---

## 4. Expected rough bands (sanity check, not a baseline)

These are not the baseline — they are order-of-magnitude sanity checks so an obviously-wrong number gets flagged during capture.

| Reading | Plausible range | Out-of-band signal |
|---|---|---|
| At-rest | 15–30 MB | > 45 MB = something loading eagerly that shouldn't |
| S3 peak | 25–45 MB | > 55 MB = critical — < 10 MB headroom to termination |
| Post-idle | ≤ at-rest + 2 MB | monotonic growth across runs → leak |

Extension termination at 64 MB is abrupt (no warning, keyboard dismisses). A P95 headroom under 10 MB is a Phase I blocker regardless of gate text.

---

## 5. What's NOT measured here

- Host-app memory — separate concern; the extension cap is isolated.
- Disk cache for `dictionary.bin` / `association.bin` — mapped files, not counted as resident unless faulted.
- `SharedSettings` cross-process reads — too small to matter; noise floor is larger.

---

## 6. Cross-references

- Latency counterpart: `docs/perf/keyboard-baseline-2026-04.md`.
- Qualitative-gate philosophy: memory `feedback_perf_gate.md` (Claude auto-memory).
- Data-artifact load cost (`dictionary.fst`, `dictionary.bin`, `association.bin`): tracked under `docs/architecture/data-artifacts-portability.md`.
