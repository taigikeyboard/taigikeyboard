## v3.5.3

### iOS + Android

#### Refactoring: engine workspace cleanup + platform mirror deletion

Two-part follow-up to the v3.5.2 Lexicon Ranking slice. **Behavior-preserving** — the IME's runtime behavior is byte-identical to v3.5.2 on both platforms. The slice consolidates the Rust shared-core architecture and removes ~660 LOC of platform Kotlin / Swift code that mirrored Rust algorithms.

**v3.5.3 — engine workspace cleanup (#191):**
- Delete the duplicate FFI envelope from `engine/phonetics/src/api.rs` (~120 LOC: `process_request`, `process_request_with`, `run_request`, `error_response`, `error_code_for`, `encode_response`). `engine/dispatch::process_request` is now the canonical FFI entry per `rules/rust-best-practices.md` §3a; the phonetics-internal envelope was a stale copy that even rejected non-Phonetics payloads. `PhoneticsError` narrows from four variants to one (only `UnsupportedOp` had remaining producers); `prost` dependency drops from `engine/phonetics/Cargo.toml`.
- Test layering re-aligned: op-level tests now drive `phonetics::dispatch::handle(&PhoneticsRequest, &AppConfig)` directly and assert `Result<PhoneticsResponse>`; envelope concerns (panic, malformed bytes, oversized payload, log-crate contract) move to `engine/dispatch/tests/`. Panic injection becomes a `#[cfg(test)]`-only seam inside `engine/dispatch/src/lib.rs` — no public injection hook in production code.
- Visibility tightening sweep — many `pub fn` items downgraded to `pub(crate)` or plain `fn` to match actual reachability. Phonetics: `case_adjust`, `normalization`, `derivation`, `tone_variations`, `tps`, `tps_adjust` internals tightened. Ranking: `score::FrequencyData` / `calculate_score` / `total` / `dedup` / `nfd` tightened; `roman_to_base` / `input_to_base` / `tier_numerator` collapse to plain `fn` (only used inside `score.rs`).
- File rename `engine/phonetics/src/parser.rs` → `syllable.rs` — the module owns syllable-level operations (`strip_tone_mark`, `normalize_to_tl`, `is_stop_tone`, `split_initial_final`, `parse_syllable`), not generic parsing. Filename now matches responsibility.
- Co-locate Zhuyin / TPS lookup tables: `ZHUYIN_INITIALS` / `ZHUYIN_VOWELS` / `ZHUYIN_TONES` / `ZHUYIN_TONES_ENCODE_SAFE` + the TPS-only `PUNCTUATION_*` tables move from the shared `tables.rs` into `tps.rs` next to their primary consumer. `tables.rs` reverts to its narrower role (TL initials / finals + tone diacritics).
- `MAX_REQUEST_BYTES` SSOT lives in `engine/dispatch`; `swift-ffi` and `android-jni` import via `use dispatch::MAX_REQUEST_BYTES`. Pre-allocation early-reject behavior on Android preserved.
- Two NFD helpers gain purpose names — `ranking::nfd::nfd_preprocessed` → `taigi_unicode_base_form`; `phonetics::normalization::nfd_preprocessed` → `trie_key_unicode_form`. The two were always intentionally distinct algorithms; the shared name caused cross-crate ambiguity.
- Project-history test names retire — `d9_4_cleanup.rs` → `normalize_tone_nasal_case.rs`; `d9_4_ops.rs` → `op_coverage.rs`. File headers rewritten to describe behavior, not project history.
- `engine/protos/src/lib.rs` drops dead `pub use engine::*;` re-export (no caller used the `protos::Foo` short form).
- `engine/README.md` rewritten for the current workspace state — drops D9.x framing, lists all 7 crates, fixes Rust toolchain to 1.86 (was stale at 1.85). `rules/rust-best-practices.md` §3a canonical example block reflects `mod syllable`.

**v3.5.3 follow-up — path G JVM-duplication delete (#192):**
- Retires the `feedback_jvm_test_jni_compat.md` strategy: previously, Kotlin / Swift mirror sources of Rust algorithms were retained so Android `src/test/` JVM unit tests could exercise the math without loading `librust_taigi.so`. The new policy (path G) is "delete the platform mirror + JVM tests rather than building JVM-loadable Rust artifacts" — solo maintainer + real-device dogfood + Rust workspace tests cover algorithm correctness.
- Two new Rust ops absorb the production callsites that previously needed Kotlin / Swift mirrors:
  - `ProcessCandidatesRequest.merge_order_only` (proto field 8) — cold-start dedup-without-scoring; replaces iOS `LexiconService` Swift fallback that ran when the user-frequency DB had not yet warmed up.
  - `Method::NfdPreprocessForLookup` (proto tag 19) — replaces the platform `TaigiUnicode.nfdPreprocessed` call inside `ExternalLookupURLBuilder`. Distinct from `Method::NormalizeInput` per algorithm contrast in `phonetics::normalization` doc.
- Helper consolidation — `taigi_unicode_base_form` moves from `engine/ranking/src/nfd.rs` into `engine/phonetics/src/normalization.rs` as `pub fn`. Single source of truth shared by `ranking::score::roman_to_base` (cross-crate via new `phonetics` workspace dep) and the new op. `engine/ranking/src/nfd.rs` deleted.
- Bridge surface gains `mergeOrderOnly: false` default param on `RustEngineBridge.processCandidates(...)` (both platforms) and `nfdPreprocessForLookup(_:)` wrappers.
- iOS `LexiconService` cold-start fallback routes through Rust with `mergeOrderOnly: true` instead of calling Swift `CandidateProcessor.removeDuplicates` / `removeDisplayDuplicates`.
- iOS + Android `ExternalLookupURLBuilder` swap their `TaigiUnicode.nfdPreprocessed` call to `RustEngineBridge.nfdPreprocessForLookup`.
- iOS + Android `RustEngineBridge.fallbackRanked` simplified to raw-list — drops the Kotlin / Swift `CandidateProcessor` defense-in-depth dedup that previously masked Rust dispatch bugs by silently degrading to platform ranking.
- Platform deletions:
  - Android: `TaigiUnicode.kt`, `CandidateProcessor.kt` (whole files, 253 LOC).
  - iOS: `TaigiUnicode.swift` (whole file, 41 LOC), `CandidateProcessor.swift` cold-start dedup block (36 LOC). The retained `isHanzi` / `capitalize` / `startsWithRomanLetter` helpers stay (production callers in `LexiconService` + `DictionarySearchService` — out of slice).
- JVM unit-test cleanup (Android `src/test/`):
  - Three A-class tests deleted (Kotlin mirrors of Rust algorithms): `CandidateProcessorTest` (12 tests), `TaigiUnicodeTest` (5 tests), `InputNormalizerTest` (8 tests).
  - Four B-class tests deleted that had been silently build-broken since D9.4 (referenced deleted symbols `TaigiPhonetics`, `ToneRestoration`, `TPSConverter`, `ToneConverterModels.InputMode`): `CharacterInputPipelineTest` (32 tests), `CustomDictionaryServiceTest` (4 tests), `EngineIntegrationTest` (7 tests), `DictionaryCoverageTest` (2 tests).
  - Total Android JVM test deletion: 70 tests / 1799 LOC across 7 files.
- iOS test trimming: `CandidateProcessorTests.swift` drops 9 dedup tests; 7 `isHanzi` tests retained. INVARIANT_* dedup invariants now live in `engine/ranking/src/process.rs::tests` via the `merge_order_only` branch coverage.

### Tooling

- New `/migration-residue` Claude Code skill (`.claude/skills/migration-residue/SKILL.md`) — slash-invokable audit across seven dimensions (cross-language duplication, cross-crate duplication, over-public Rust surface, stale doc comments, build-broken JVM tests, bridge surface parity, memory hygiene). Pure measurement; no auto-fix. Supports `--dimensions` subset filter and `--codex` second-opinion pass. Run before each new slice migration to seed the audit, after each merge to verify cleanliness, before each release tag to catch stale doc / broken-test residue.
- First baseline run at `docs/engine/migration-residue-2026-04-29.md` — 0 P1, 0 P2, 5 P3 (cosmetic). Verdict: **PASS**. Migration architecture is in the cleanest state since shared-core extraction began.

### Documentation

- `docs/engine/v3.5.3-cleanup-audit.md` — engine workspace cleanup pre-flight audit (PR #191).
- `docs/engine/v3.5.3-followup-jvm-duplication-audit.md` — path-G JVM-duplication pre-flight audit (PR #192). Twelve refactor-only commits, 0 P1 / 0 P2 / 3 P3 from Codex post-impl review (all folded).
- `engine/README.md` rewritten for current workspace state (7 crates, dep direction, MSRV history).
- `rules/rust-best-practices.md` §3a — canonical example block reflects post-cleanup shape.
