## v3.5.2

### iOS + Android

#### Refactoring: Phase IV-B Lexicon Ranking on Rust

Second slice of the cross-platform Rust shared core. The lexicon ranking pipeline — candidate dedup, score computation, sort by score — moves from per-platform Swift / Kotlin into a single Rust crate (`engine/ranking/`) called from both platforms via `RustEngineBridge.processCandidates`. Behavior-preserving: the score formula constants (`USER_FREQ_CAP=100`, `USER_FREQ_WEIGHT=100`, `RECENCY_WINDOW_MS=3_600_000`, `RECENCY_BONUS=200`, `EXACT_BONUS=100`, `COMPLETION_PENALTY=-1000`, `CLOSENESS_WEIGHT=500`, `BASE_FREQ_DIVISOR=10`, `SOURCE_TIERS`, `TIER_DENOMINATOR=10`) and the dedup / sort semantics are pinned byte-identical and verified by parity tests on both platforms.

**v3.5.2 — Lexicon Ranking slice (#189):**
- New `engine/ranking/` crate with five modules: `lib`, `dedup`, `score`, `sort`, `nfd`. Pure-CPU, stateless, no I/O, no time reads inside the crate (caller supplies `now_ms`). Implementation modules are crate-private; `process_candidates` is the single named entry point.
- New `engine/dispatch/` crate as the top-level FFI router — single `process_request(&[u8]) -> Vec<u8>` decodes once, routes by `Request.payload` variant (Phonetics or Lexicon) to the matching domain crate, encodes once. `swift-ffi` and `android-jni` are now thin pass-throughs that call `dispatch::process_request` directly; the phonetics crate no longer hosts the FFI seam.
- New `lexicon.proto` schema — `TaigiWord`, `FrequencyEntry`, `ScoreBreakdown`, `ProcessCandidatesRequest`, `ProcessCandidatesResponse`. `envelope.proto` adds `CMD_LEXICON = 3` + payload tag 12. Generated Swift (`lexicon.pb.swift`) and Java protobuf-javalite builders (15 new classes under `engine/proto/`) committed alongside the Rust prost output.
- iOS `Lexicon/Utils/CandidateProcessor.swift` and Android `ime/dictionary/CandidateProcessor.kt` collapse to thin wrappers — production paths route the `removeDuplicates` / `removeDisplayDuplicates` / `calculateScore` / `sortByScore` quartet through `RustEngineBridge.processCandidates`. JVM unit tests retain platform copies of the math (`removeDuplicates`, `calculateScore`, `sortByScore`, `romanToBase`, `inputToBase`) under `src/test/` so they exercise without `librust_taigi.so`; canonical CROSS-PLATFORM INVARIANT comments mark each retained helper.
- iOS `Lexicon/Services/LexiconService.swift` updated to pass `RustEngineBridge` ranked candidates straight through; Android `LexiconService.composeRanked` mirrors. `UserFrequencyRepository` + `FrequencyData` shaped to match the new `FrequencyEntry` proto.
- 154 cargo workspace tests pass; clippy `-D warnings` clean. Codex pre-impl + post-impl reviews APPROVE-WITH-FIXES (all 5 P1 + 3 P2 incorporated). Two Codex sandwiches closed during the slice; one in-PR P2 (FFI fallback graceful degradation) addressed before merge.

**Phonetics-API cleanup (#190):**
- `engine/phonetics/src/derivation.rs` split into `derivation` (CustomDictionaryDerivation port — `derive_notone`, `derive_abbrev`) and a new `normalization` module (InputNormalizer + ToneRestoration ports — `normalize_input`, `has_tone_marks`, `restore_tone`). The two groups had been bundled but operate on different substrates; splitting them gives the upcoming Composing slice a cleaner template.
- `engine/phonetics/src/lib.rs` `pub mod` surface collapses — only `api` and `dispatch` stay public façades; ten implementation modules (`case_adjust`, `derivation`, `normalization`, `parser`, `poj`, `tables`, `tl`, `tone_variations`, `tps`, `tps_adjust`) flip to `mod`-private. The redundant top-level `process_request` re-export drops; `engine/dispatch` is the canonical FFI entry.
- Stale dead-code suppressors (`_keep_imports_alive`, `_alias_to_tone_marks`) deleted.

### Documentation

- `docs/engine/ranking-slice-audit.md` — pre-flight audit for the v3.5.2 slice: public-surface inventory, constants parity check, caller inventory, behavior parity decisions, JVM unit-test JNI compat audit, locked Rust crate layout, locked proto contract, 8-commit sequence, risks + mitigations.
- `rules/rust-best-practices.md` §3a — new domain↔proto boundary rule (Pattern A): the dispatch / RPC façade of each domain crate accepts and returns protobuf-generated types directly; no parallel native-Rust mirror tier; native-Rust convenience APIs (CLI helpers, integration-test fixtures) may coexist on the side but never grow into a proto-mirroring tier the dispatcher routes through. Codifies the surface shape both `phonetics` and `ranking` already follow so future Composing (D9.3) and beyond inherit a consistent template.
