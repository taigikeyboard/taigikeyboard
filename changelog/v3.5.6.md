## v3.5.6

### iOS + Android

#### Refactoring: Phase IV-B Lexicon read-path on Rust

Fifth slice of the cross-platform Rust shared core. The dictionary read path — prefix index + bundled binary record reader + candidate row assembly + bigram association lookup + multi-source search + hanzi-prefix search — moves from per-platform Swift / Kotlin into a new Rust crate (`engine/lexicon/`) called from both platforms via `RustEngineBridge.lexicon*`. Persistence for user-data SQLite (`user_association.db`, `user_frequency.db`, `custom_dictionary.db`) stays platform-side by design (`feedback_user_data_sqlite_stays_native`). Behavior-preserving on both platforms; dogfood-verified against the v3.5.5 baseline.

**v3.5.6 part 1 — Lexicon read-path slice (#199):**
- New `engine/lexicon/` crate (11 modules, ~615 LOC) with `lib`, `api`, `dispatch`, `prefix_index` (fst-backed), `dictionary_reader` (TKDB binary record archive), `association_reader` (bundled bigram mmap), `search` (5-stage pipeline), `error`, plus internal helpers. Pure logic; SQLite I/O stays platform-side. `#![forbid(unsafe_code)]`. Singleton `LexiconHandle` (mmap-owned `Arc<Inner>`) routes intents through `dispatch::handle`; install reuses the same method with atomic-on-success swap.
- New prefix index format: `dictionary.fst` replaces the MARISA trie. Wire format `key_bytes (UTF-8) || 0xFF || rowid_le_4`; range scan on `[prefix, prefix_succ)` decodes trailing rowid per hit. Insertion order preserved through deterministic byte-sorted iteration (D-12 parity with Android `distinct()`).
- New record archive format: `dictionary.bin` (TKDB) — `Header(16B): "TKDB" || version || count || build_ts`, `Offset table: count × u32`, `Records: bitmask(u16) || frequency(u32) || hanzi_len(u8) || tl_len(u8) || hanzi || tl`. 4.4 MB on disk; rowid-keyed via offset table lookup.
- Extended `lexicon.proto` schema — 6 oneof methods: `process_candidates` (existing — ranking), `install`, `search`, `search_with_sources`, `search_by_hanzi`, `assoc_lookup` (NEW). `envelope.proto` adds `CMD_LEXICON = 5` + payload tag 14. Generated Swift (`lexicon.pb.swift`) and Java protobuf-javalite builders committed.
- iOS bridge surface: new `RustEngineBridge+Lexicon.swift` extension exposes `lexiconInstall(triePath:dictionaryBinPath:associationBinPath:enabledMask:)`, `lexiconSearch(searchKey:inputType:enabledMask:limit:)`, `lexiconSearchWithSources(...)`, `lexiconSearchByHanzi(...)`, `lexiconAssocLookup(...)` + synth `EnabledDictionaries` value type.
- Android bridge surface: matching region inside `RustEngineBridge.kt` adds the same 5 methods with Kotlin data class `EnabledDictionaries` mirror.
- iOS swap: `DictionarySearchService` + `LexiconService` swap MARISA trie + native C++ bridge to `lexiconSearch*` calls. Deletes `Lexicon/Database/{AssociationBinaryReader,DictionaryBinaryReader,DictionaryRepository}.swift`, `Lexicon/Trie/{InputNormalizer,TrieService,marisa_bridge.cpp,marisa_bridge.h}`, `InputNormalizerTests.swift` (768 LOC). New `LexiconBitmask.swift` + `EnabledDictionaries.swift` value types stay platform-side as bridge inputs.
- Android swap: equivalent removal of platform Lexicon read-path source + JVM unit tests per `feedback_path_g_delete_mirrors`. Platform-stays code retained: `LexiconBitmask`, source enum (bitmask ordering authoritative), settings injection.
- iOS: `Engine/RustEngineBridge.swift` access promotion (3 helpers `internal`) so the new `+Lexicon.swift` extension can share the diagnostics ring-buffer + request-id sequence + RustVec helper, mirroring the v3.5.5 NextWord pattern.
- 271 cargo workspace tests pass (218 existing + 53 new in `lexicon`). Codex pre-impl review APPROVED (joint Claude+Codex auto-mode 4-round audit + plan); Codex post-impl APPROVED. Build pipeline: `dictionary/build/create_fst.py` shells exclusively to `engine/build-helpers/fst-builder` Rust binary — no PyPI MARISA / Python-C binding remains.

**v3.5.6 part 2 — build-pipeline SQLite intermediates removed (#200):**
- Drop `dictionary/output/{dictionary.db, trie.db, word_association.csv}` — three intermediates from the legacy MARISA + SQLite trie pipeline that are no longer consumed by any downstream step. The Rust crate reads directly from `dictionary.fst` + `dictionary.bin`. `word_association.csv` (177,577 lines) was build-time scaffolding for `association.bin` and not needed once the fst-builder produces the bundled bigram artifact.
- Pipeline cleanup: `compare_baseline.py` manifest entries pruned to live artifacts; `query_fst.py` flags adjusted for the streamlined pipeline; new test `tests/test_associations_sparse_inputs.py` covers boundary cases.
- Doc sync: `docs/engine/binary-format.md` updated to reflect TKDB + fst as canonical formats; `docs/architecture/data-artifacts-portability.md` reference fixed.

**v3.5.6 cleanup — comments (#201):**
- Drop transition-narrative comments referencing pre-Rust class names (`MARISATrieService`, `Native*Bridge`, `*Migration`) from iOS Lexicon source. Six files touched, ~10 net LOC. No behavior change; aligns with `feedback_round_hygiene` once the Rust swap is permanent.

### Build / Tooling

- `engine/Cargo.toml` workspace member added: `engine/build-helpers/fst-builder` (Rust binary used by `dictionary/build/create_fst.py`).
- Workspace dependency added: `fst = "0.4"` (BurntSushi finite-state transducer crate).
- 271 cargo workspace tests, 0 warnings on `make build`.

### Documentation

- `docs/engine/lexicon-slice-audit.md` — pre-flight audit: file inventory, divergences, fst-vs-marisa-rs spike, install/init contract, risk matrix.
- `docs/engine/lexicon-slice-plan.md` — Rust crate layout, proto schema, bridge surface, commit slicing, INVARIANT_LEX_* parity tests, platform deletion list.
- `docs/engine/binary-format.md` — TKDB record format spec.
- `docs/engine/rust-core-proto.md` — Lexicon section now AS-IMPLEMENTED post-#199.
