## v3.5.1

### iOS + Android

#### Refactoring: Phase III D9 — Phonetics Engine on Rust

First slice of the cross-platform Rust shared core lands. The entire Phonetics module — TL ↔ POJ ↔ Zhuyin/TPS conversion, tone-mark / tone-number normalization, NFD preprocessing, nasal-marker case agreement — now runs through a single Rust crate (`engine/phonetics/`) called from both platforms via `RustEngineBridge`. Behavior-preserving: parity verified by dogfood across S1/S2/S3 modes, no keyboard dismiss regressions, no leaks.

**D9.1 — Rust workspace + Phonetics POC (#183):**
- New `engine/` Cargo workspace under MSRV 1.86 with `phonetics` + `protos` crates.
- `protos` crate hosts the wire protocol (`phonetics.proto` + `envelope.proto`); generated code is checked in for both Swift (`*.pb.swift`) and Android (Java) consumers so no build-time codegen is required on either platform.
- Phonetics tables (`TL_INITIALS` / `TL_FINALS` / `TONE_NUM_TO_COMBINING` / Zhuyin maps / punctuation pairs) ported byte-exact from the Swift / Kotlin originals; `// SOURCE:` provenance comments preserved.

**D9.2 — FFI POC (#185):**
- iOS: `swift-bridge` consumed via a vendored `RustTaigi.xcframework` (universal slice for arm64 device + arm64/x86_64 simulator) committed under `ios/RustEngine/`.
- Android: JNI wrapper crate produces `librust_taigi.so` for `arm64-v8a` + `armeabi-v7a` + `x86_64` and ships under `android/app/src/main/jniLibs/`.
- Single `RustEngineBridge` facade on each platform — opaque handle pattern, all calls go through one entry point so the FFI surface is centralized and future intents bolt on without new symbols.

**D9.4 — Phonetics fully on Rust (#186):**
- 8 Swift files deleted (`Phonetics/Converter/PhoneticsConverter.swift`, `Phonetics/Converter/RomanizationConverter.swift`, `Phonetics/Formatter/POJFormatter.swift`, `Phonetics/Formatter/TLFormatter.swift`, `Phonetics/Parser/SyllableParser.swift`, `Phonetics/Tables/PhoneticsTables.swift`, `Phonetics/TaigiPhonetics.swift`, `Phonetics/ToneConverter.swift`, `Phonetics/ToneRestoration.swift`, plus 5 corresponding test files).
- Equivalent Kotlin helpers removed under `android/.../ime/phonetics/`.
- Composing display path now invokes one Rust call per keystroke for the full normalize-tone pipeline (parse → tone resolution → nasal-marker case agreement → format) instead of three platform-side stages.
- `OP_NORMALIZE_TONE` folds `adjust_nasal_marker_case` inline so platform callers no longer post-process.
- iOS `RustEngineBridge` (425 LOC) + Android equivalent expose `convert`, `toToneMarks`, `toToneNumber`, `stripToneMark`, `normalizeToTL`, `adjustNasalMarkerCase` (where applicable), `processRequest`.
- New unit tests (`RustEngineBridgeTests.swift`, `RustEngineBridgeTest.kt`) cover the FFI surface; 126 Rust tests cover the engine itself (`cargo test -p phonetics`).

**D9.4-cleanup — final platform helpers internalized (#187):**
- `adjustNasalMarkerCase` now lives in Rust (`engine/phonetics/src/case_adjust.rs`) and is exposed via `OP_NORMALIZE_TONE` fold-in for the composing display path. A platform-side mirror is retained on both iOS and Android for callers reachable from JVM unit tests (where `System.loadLibrary("rust_taigi")` would throw `UnsatisfiedLinkError`); both sides carry a `CROSS-PLATFORM INVARIANT` note pointing at the Rust mirror.
- `nfdPreprocessed` stays platform-side for the same JVM-test compatibility reason; `engine/phonetics/src/nfd.rs` was removed after consolidation since no Rust call site remained.
- Internal helper visibility narrowed (`pub` → `pub(crate)`) for `parse_syllable`, `split_initial_final`, `is_stop_tone`, all `TL_INITIALS` / `TL_FINALS` / `TONE_NUM_TO_COMBINING` / Zhuyin tables — `taigi-converter` shared-core unification is deferred, so a smaller public surface keeps options open.
- Proto regen sync — Swift `.pb.swift` + Android Java protobuf builders refreshed to match `phonetics.proto` simplify edits; `oneof intent` renamed to `oneof method` to align with standard RPC vocabulary (wire-format compatible).

### Dictionary

#### `taigi-converter` Submodule Bump (#184)

`taigi-converter` submodule bumped to bring in upstream fixes; legacy `// DRIFT:` comments dropped from the build pipeline now that the migration off `kesi` has fully settled (see v3.5.0 changelog).

### Documentation

- `docs/engine/ffi-safety.md` — full FFI safety contract (panic catching, UTF-8 boundary handling, opaque handle ownership, MSRV / target triple matrix). Phase II.5 Round C output.
- `docs/engine/rust-core-proto.md` — wire protocol design draft. Phonetics section now reflects the as-implemented surface (post #186 / #187); Composing section remains design-only pending D9.3.
- `/shared-core-confidence` skill + first run report (#182) — automated readiness scoring across the shared-core extraction roster, used as the gate for entering each migration slice.
- `rules/rust-best-practices.md` revised for the active Phase IV-A+ phase.
- Sweep of stale Phonetics references in tier-1 docs (#188).
