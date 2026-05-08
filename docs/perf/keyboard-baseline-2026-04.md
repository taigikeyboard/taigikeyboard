# Keyboard Keystroke Latency Baseline — 2026-04

**Status** (revised 2026-04-19, qualitative-gate philosophy ratified by `feedback_perf_gate.md`): methodology frozen, **quantitative capture deferred**. The project uses a qualitative dogfooding gate (S1/S2/S3 typing feels right, no keyboard dismiss, no leaks) instead of P50/P95 numbers. This document is retained as a template for a future CI perf lane / team workflow where Instruments numbers become worth the capture overhead.

**Why deferred**: solo-dev IME cadence — the user is the QA, perceptible regression on S1/S2/S3 is the acceptance criterion. Instruments signposts + per-refactor re-measurement cost exceeds the benefit of catching microsecond-level regressions the user cannot feel.

**When to revive this doc**: CI perf runner is added, OR team grows past 1 developer, OR a specific regression is suspected and needs a number to confirm.

**Original gating signal** (kept for reference; currently inactive): P95 keystroke latency on every sequence below must be ≤ the baseline. A refactor that regresses any P95 by more than the documented noise band fails the gate.

**Device contract**: baseline is captured on a **real iOS device** (not simulator — simulator latency is not representative for keyboard extension cost). Record device model + iOS version + low-power / performance mode state; re-measure only on the same device class when comparing.

---

## 1. Input sequences

Three sequences chosen to exercise orthogonal engine paths. All are typed manually in the same target app (see §4) with the Taigi keyboard active. The same sequence is repeated N=30 times per run; discard the first 3 (warm-up).

### S1 · POJ diacritic composition

Target a short phrase that forces tone-mark rendering on every keystroke.

```
Input (MOE2 POJ layout): g u a 2  s i 7  s o o
Expected display progression: g → gu → gua → guá → guá s → ... → guá sī sòo
```

Exercises: `SyllableParser.parseSyllable`, `POJFormatter.toPOJ`, tone-mark placement, `ComposingManager` publishes, SwiftUI diff.

### S2 · TPS composition (3-key sequence)

Target a TPS initial + medial + final + tone sequence.

```
Input (MOE2 TPS layout): ㄍ ㄨ ㄚ ˋ (tone-2 key)
Expected display progression: ㄍ → ㄍㄨ → ㄍㄨㄚ → guá (converted)
```

Exercises: `TPSTables`, `TPSInputAdjuster`, `TPSToTL`, `InputNormalizer` through the converted path.

### S3 · Hanji candidate scroll

Type `gua` then scroll the candidate bar through at least 12 candidates (swipe left-right on the autocomplete strip).

```
Type: g u a
Then: swipe candidate bar 3 full pages (≈ 12 candidates visible per page on iPhone 12-class).
```

Exercises: `DictionarySearchService`, Rust `ranking::process_candidates` (sort + dedup + optional TPS display dedup; routed via `RustEngineBridge.processCandidates`. iOS `CandidateProcessor.swift` retains only a 4-LOC residual unrelated to ranking math), SwiftUI `LazyHStack` scroll dispatch on the hottest UI path.

---

## 2. Methodology (manual testing on real device)

The keyboard extension does not currently have signposts (audit 2026-04-19 — grep for `os_signpost` returned zero hits). Two measurement paths are acceptable; pick **one** per run and record which was used.

### Option A — Xcode Instruments "os_signpost" template (preferred)

Requires adding **temporary** signposts around the keystroke pipeline (not committed to main; keep on a local branch during the measurement session):

```swift
import os
private let kbSignpost = OSSignposter(subsystem: "com.siansiansu.taigikeyboard", category: "keystroke")

// At the start of the keystroke handler (KeyboardViewController or ActionHandler):
let state = kbSignpost.beginInterval("keystroke", id: kbSignpost.makeSignpostID())
// After ComposingManager publishes + UI reflects the change (end of the main-thread dispatch):
kbSignpost.endInterval("keystroke", state)
```

Run Instruments → **os_signpost** template → attach to the running host app (NOT the keyboard extension process directly — Instruments attaches to the host). Type sequence S1/S2/S3. Export intervals; compute P50 and P95 over the 27 post-warm-up samples.

### Option B — Signpost log + Console.app

If Instruments attach to the extension is flaky, use `os_log` signposts with `.pointsOfInterest` and read the stream from Console.app filtered by `subsystem == "com.siansiansu.taigikeyboard"`. Manually pair begin/end timestamps; compute P50/P95 in a spreadsheet.

### Warm-up protocol

1. Launch target app, open the input field, activate Taigi keyboard.
2. Type 10 throwaway characters, delete.
3. Start the Instruments recording.
4. Type S1 thirty times back-to-back with normal human cadence (no forced pauses).
5. Stop recording; repeat for S2 and S3.

### Noise band

Report the standard deviation alongside P50/P95. Treat a future run as a regression when the new P95 exceeds (baseline P95 + 2σ).

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
| Build config | `Debug` / `Release` (prefer Release for baseline) |
| Measurement option | `A (Instruments signposts)` / `B (Console.app)` |
| Branch / commit SHA | `main @ <sha>` |

| Sequence | Samples (N) | P50 (ms) | P95 (ms) | σ (ms) | Notes |
|---|---|---|---|---|---|
| S1 — POJ diacritics | 27 | TBD | TBD | TBD | |
| S2 — TPS composition | 27 | TBD | TBD | TBD | |
| S3 — Hanji candidate scroll | 27 | TBD | TBD | TBD | |

---

## 4. Target app for testing

The keyboard extension runs inside whatever app hosts the text field. For baseline consistency, always type into the same host app. Recommended options:

- **Notes.app** — simple `UITextView`, minimal host overhead.
- **Messages.app** — realistic user scenario but has extra UI layout cost.

Pick one for run 1 and record it. Subsequent runs should use the same host.

---

## 5. What's NOT measured here

- Cold-start latency of the extension process (first keystroke after app switch) — different measurement, different baseline. Track separately if it becomes a concern.
- Dictionary binary load time — one-shot at extension launch, covered by memory baseline instead.
- Cross-process Darwin notification latency for settings sync — not a keystroke path.

---

## 6. Cross-references

- Qualitative-gate philosophy: memory `feedback_perf_gate.md` (Claude auto-memory).
- Memory counterpart: `docs/perf/extension-memory-2026-04.md`.
- Candidate scoring / NextWord decay invariants exercised by S1/S3: `docs/architecture/behavioral-invariants.md` §6, §7.
