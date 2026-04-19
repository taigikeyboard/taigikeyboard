# Android Keyboard Keystroke Latency Baseline — 2026-04

**Status** (authored 2026-04-19 as Phase II A0 sibling artifact to `keyboard-baseline-2026-04.md`): methodology frozen, **quantitative capture deferred**. Phase II A0 adopts a qualitative dogfooding gate (see `docs/architecture/android-state-audit.md` §A0 and Phase II gating signal #6) instead of P50/P95 numbers. This document is the Android equivalent of the iOS template — same three sequences (S1 / S2 / S3), same deferred-quantitative policy, Android-specific tooling surface.

**Why deferred**: solo-dev IME cadence — the user is the QA, perceptible regression on S1 / S2 / S3 is the acceptance criterion. Perfetto capture + per-refactor re-measurement cost exceeds the benefit of catching microsecond-level regressions the user cannot feel.

**When to revive this doc**: CI perf runner is added, OR team grows past 1 developer, OR a specific regression is suspected and needs a number to confirm (e.g. A4-impl or A5-impl flags an unexpected latency shift during dogfooding).

**Original gating signal** (kept for reference; currently inactive): P95 keystroke latency on every sequence below must be ≤ the baseline. A refactor that regresses any P95 by more than the documented noise band fails the gate.

**Device contract**: baseline is captured on a **real Android device** (not emulator — emulator latency is not representative for IME extension cost on Android, where `InputConnection` round-trips and SurfaceFlinger composition interact with real hardware schedulers). Record device model, Android version + API level, build variant, host app, refresh-rate mode, battery saver / thermal state, keyboard subtype settings, and branch SHA. Re-measure only on the same device class when comparing.

---

## 1. Input sequences

Three sequences, same intent as iOS — exercise orthogonal engine paths. All are typed manually in the same target app (see §4) with the Taigi Keyboard active. The same sequence is repeated N=30 times per run; discard the first 3 (warm-up).

### S1 · POJ diacritic composition

Target a short phrase that forces tone-mark rendering on every keystroke.

```
Input (MOE2 POJ layout): g u a 2  s i 7  s o o
Expected display progression: g → gu → gua → guá → guá s → ... → guá sī sòo
```

Exercises: Android `TaigiPhonetics.convertSyllable`, tone-mark placement via `ToneConverter`, `ComposingManager.deriveDisplay`, `ic.setComposingText` round-trip through `InputConnection`, Compose recomposition of the candidate bar.

### S2 · TPS composition (3-key sequence)

Target a TPS initial + medial + final + tone sequence.

```
Input (MOE2 TPS layout): ㄍ ㄨ ㄚ ˋ (tone-2 key)
Expected display progression: ㄍ → ㄍㄨ → ㄍㄨㄚ → guá (converted)
```

Exercises: Android `TPSConverter` (tables + adjuster + TPSToTL folded into one file), `InputNormalizer` through the converted path, `CandidateUpdateCoordinator` debounced dispatch on `Dispatchers.Default`.

### S3 · Hanji candidate scroll (smartbar / candidate strip)

Type `gua` then scroll the candidate bar / smartbar **horizontally** through 3 full visible-pages worth of candidates (≈ 36 candidates total on a ~6.1" device at default text size — ≈ 12 candidates visible per page).

```
Type: g u a
Then: wait ≤ 200 ms for candidate bar to populate, then horizontal scroll
      (swipe-left drag on the candidate strip). Scroll through 3 full
      visible-pages — stop after the 3rd page boundary has scrolled off
      the leading edge (≈ 36 candidates have passed the viewport origin).
```

Exercises: `LexiconService.search` path, Android `CandidateProcessor.sortByScore` + `removeDuplicates` + `removeDisplayDuplicates`, `RecyclerView` + `CandidateAdapter` scroll dispatch in `SmartbarManager` candidate strip (Android smartbar is not Compose — `CandidateAdapter` + `CandidateOverlayAdapter` back the `RecyclerView`).

---

## 2. Methodology (manual testing on real Android device)

The IME extension does not currently have `android.os.Trace` sections around the keystroke pipeline (audit 2026-04-19 — grep for `Trace.beginSection` inside `ime/text/composing/` and `ime/text/smartbar/` returned zero hits). Two measurement paths are acceptable; pick **one** per run and record which was used.

> **Instrumentation policy** — any `Trace.beginSection` / `System.nanoTime()` additions required for a measurement run are **temporary local instrumentation only**. They live on the measurement branch and are NOT committed to `main` / `develop`. A0 ships this document with zero `.kt` changes; instrumentation is introduced at measurement time, reverted at capture close. A future round may decide to land a small permanent instrumentation set — out of scope here.

### Option A — Perfetto app-tracing (preferred)

Requires temporary `android.os.Trace` sections around the keystroke pipeline:

```kotlin
import android.os.Trace

// At the start of the keystroke handler (TextInputManager.onKeyDown / handleKey):
Trace.beginSection("keystroke")
try {
    // existing keystroke pipeline — ComposingManager, InputConnection calls,
    // candidate dispatch through CandidateUpdateCoordinator
} finally {
    Trace.endSection()
}
```

Capture via Perfetto system tracing:

```bash
# 1. Install perfetto config for app tracing (30-second capture).
#    Capture filename is computed once and reused for capture + pull.
TS=$(date +%s)
TRACE_FILE="taigi-${TS}.perfetto-trace"

adb shell perfetto \
  --config-file /data/misc/perfetto-configs/taigi-keystroke.pbtx \
  --txt \
  -o "/data/misc/perfetto-traces/${TRACE_FILE}"

# 2. Pull the trace (same filename as capture).
adb pull "/data/misc/perfetto-traces/${TRACE_FILE}" /tmp/

# 3. Open at https://ui.perfetto.dev and filter on the "keystroke" slice name.
#    Export per-slice durations; compute P50 and P95 over 27 post-warm-up samples.
```

Minimal Perfetto config (atrace "view" + app-tagged slices):

```
buffers { size_kb: 63488 fill_policy: RING_BUFFER }
data_sources {
  config {
    name: "linux.ftrace"
    ftrace_config {
      atrace_categories: "view"
      atrace_apps: "com.siansiansu.taigikeyboard"
    }
  }
}
duration_ms: 30000
```

### Option B — logcat-paired timestamps

If Perfetto attach is flaky (keyboard extensions can be finicky — the IME service may run under a different UID namespace for tracing), use paired `System.nanoTime()` log lines from temporary local instrumentation:

```kotlin
import android.util.Log

// Temporary (not committed):
private val tag = "TaigiPerf"
Log.d(tag, "keystroke_begin ${System.nanoTime()}")
// ... existing pipeline ...
Log.d(tag, "keystroke_end ${System.nanoTime()}")
```

Capture with logcat filter and manually pair lines in a spreadsheet:

```bash
adb logcat -s TaigiPerf:D -v time > /tmp/taigi-perf.log
# Pair begin/end in spreadsheet; compute P50 / P95 over post-warm-up samples.
```

### Warm-up protocol

1. Launch target app (§4), open the input field, activate the Taigi Keyboard subtype.
2. Type 10 throwaway characters, delete.
3. Start the Perfetto capture / logcat recording.
4. Type S1 thirty times back-to-back with normal human cadence (no forced pauses).
5. Stop recording; repeat for S2 and S3.

### Noise band

Report the standard deviation alongside P50 / P95. Treat a future run as a regression when the new P95 exceeds (baseline P95 + 2σ).

---

## 3. Baseline captures

> **Fill in after each manual measurement session. Do not delete previous captures — each row is a historical baseline.**

### Run 1 — TBD (first capture)

| Field | Value |
|---|---|
| Date | `YYYY-MM-DD` |
| Device model | e.g. `Pixel 7`, `Samsung Galaxy S22`, `Xiaomi 13` |
| Android version / API | e.g. `Android 14 / API 34` |
| Build variant | `debug` / `release` (prefer `release` for baseline — R8 + resource shrink enabled) |
| Host app | see §4 (record name + input-field type) |
| Refresh rate mode | `60 Hz` / `90 Hz` / `120 Hz` (fixed, not adaptive) — record the sustained rate during capture |
| Battery saver | `off` / `on` (prefer `off` for baseline) |
| Thermal state | `nominal` / `throttling` (check `/sys/class/thermal/.../temp` before capture) |
| Measurement option | `A (Perfetto / android.os.Trace)` / `B (logcat paired)` |
| Keyboard subtype | `POJ` / `TL` / `TPS` (record default subtype at capture start) |
| Layout mode | `MOE2` (only one supported today — record in case it diverges) |
| Double-tap toggles | `enableDoubleTapOO` state / `enableDoubleTapNN` state |
| Branch / commit SHA | `main @ <sha>` |

| Sequence | Samples (N) | P50 (ms) | P95 (ms) | σ (ms) | Notes |
|---|---|---|---|---|---|
| S1 — POJ diacritics | 27 | TBD | TBD | TBD | |
| S2 — TPS composition | 27 | TBD | TBD | TBD | |
| S3 — Hanji candidate scroll | 27 | TBD | TBD | TBD | For S3, measure per-scroll-frame time, not per-keystroke time. |

---

## 4. Target app for testing

The IME extension runs inside whatever app hosts the text field. For baseline consistency, always type into the same host app on Android. Recommended options:

- **Google Keep** — simple `EditText`, minimal host overhead, widely installed.
- **Messages (Google Messages)** — realistic user scenario but has extra UI layout cost (the message composer re-renders on every text change).

Pick one for run 1 and record it. Subsequent runs should use the same host on the same real device — emulator / AVD capture is explicitly disallowed by the device contract in the status block, even for convenience or noise-band exploration.

---

## 5. What's NOT measured here

- Cold-start latency of the IME service (first keystroke after subtype switch or after Android force-kills the IME) — different measurement, different baseline. Track separately if it becomes a concern.
- Dictionary binary load time — one-shot at `LexiconService.init(context)`, covered by memory baseline (Android-side addendum TBD).
- DataStore flush latency for settings writes — not a keystroke path. `PrefHelper.cachedPrefs` serves reads synchronously per §4 of `android-state-audit.md`.
- JNI trie lookup overhead (`trie_jni.cpp`) — embedded inside `LexiconService.search`, captured indirectly via S3 scroll latency. If isolating is needed, add a dedicated `Trace.beginSection("trie_lookup")` inside the JNI boundary.

---

## 6. Cross-references

- iOS sibling baseline: `docs/perf/keyboard-baseline-2026-04.md`.
- Phase II plan (A0 defines the qualitative gate): `docs/architecture/android-state-audit.md` §A0 and §9 gating signal #6.
- Android invariants coverage (sibling A0 artifact): `docs/architecture/android-g9-coverage-matrix.md`.
- Behavioral invariants exercised per sequence:
  - S1 · POJ diacritic composition — `behavioral-invariants.md` §1 (TL ↔ POJ roundtrip), §4 (input normalization), §9 (case transformation).
  - S2 · TPS composition — `behavioral-invariants.md` §3 (TPS roundtrip), §4 (input normalization through the converted path).
  - S3 · Hanji candidate scroll — `behavioral-invariants.md` §5 (dedup), §6 (scoring determinism). §7 (next-word decay) is not on the S3 path — it activates on candidate commit, not on candidate-strip scroll.
- Composing boundary target shape: `docs/architecture/composing-state-boundary.md` §2 (platform-neutral target). The Android-binding appendix is a future A4-design deliverable and does not yet exist in that file; cross-reference to update once A4-design lands.
