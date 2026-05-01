# Lexicon Read-Path Slice — Plan (v3.5.6 / OPT-A)

**Status**: pre-impl plan, authored 2026-05-01 on `main` (HEAD `c6dd1f0`) before opening branch `phase4b/v3.5.6-lexicon-readpath`. Companion to `docs/engine/lexicon-slice-audit.md` (Codex round-3 APPROVED 2026-05-01).

**Scope frozen**: OPT-A read path only (per `project_v3_5_6_lexicon_scope.md`). Locked plan-time gates (auto-mode joint Claude + Codex 2026-05-01):

| Gate | Decision | Codex confidence |
|---|---|---:|
| G1 — Build pipeline | `dictionary/build/create_trie.py` → renamed `create_fst.py` in same commit; emits `output/dictionary.fst`; new Rust binary `engine/build-helpers/fst-builder` shells in from Python (no Python C dependency); `tools/query_trie.py` deletes, `tools/query_fst.py` adds. | 90% |
| G2 — Proto tags | `LexiconRequest.method` oneof tags `10` (preserved) + `11..15` for `install` / `search` / `search_with_sources` / `search_by_hanzi` / `assoc_lookup`. No `shutdown` reservation (YAGNI). Mirror response oneof `10..15`. | 95% |
| G3 — Bridge synth-types | iOS: co-located in `RustEngineBridge+Lexicon.swift` (split to `+LexiconTypes.swift` only if soft cap exceeded). Android: top-level `LexiconBridge.kt` object with nested data classes (intentional divergence from existing `RustEngineBridge.kt` per scope memo — accepts the inconsistency for LOC management). | 85% |
| G4 — mmap unsafe | Option A. New crate `engine/mmap-host` with crate-local `unsafe_code = "allow"` carve-out. `engine/lexicon` + `swift-ffi` + `android-jni` stay `forbid(unsafe_code)`. Uses `memmap2`. | 95% |
| G5 — Commit slicing | 13 commits (compressed from 16; bundles engine impl 5+6 → 5; mirror deletions 14+15+16 → 13). | 90% |
| D-8 — hanzi-input semantics | Locked 2026-05-01 (auto-mode): Option A + Mod 1 (parity test on autocomplete/lexicon-service layer with seeded custom-dict entry) + Mod 2 (`docs/architecture/behavioral-invariants.md` invariant doc + isolated commit). | 91% |

**Cadence ref**: `project_rust_migration_cadence.md` v3.5.6 = Lexicon (DB + Trie behind FFI), pulling forward only the read half per scope memo.

---

## 1. Rust workspace delta

### 1.1 Adds

- `engine/mmap-host/` — sibling crate, ~30 LOC (hard cap <300). The **only new** `unsafe_code = "allow"` carve-out introduced by this slice; no mmap unsafe is pushed into `swift-ffi` / `android-jni` (those crates' existing FFI carve-outs are unchanged).
- `engine/lexicon/` — sibling crate, ~1120 LOC across 11 files (per scope memo, all under 300 LOC soft cap).
- `engine/build-helpers/fst-builder/` — sibling crate, binary target, ~150 LOC budget (hard cap <300). Reads `(key, rowid)` pairs from stdin (line-protocol), emits `dictionary.fst`. Replaces the Python-binding option from G1.

### 1.2 Cargo.toml workspace member edits

```toml
# engine/Cargo.toml — additions to [workspace.members]
members = [
  "phonetics", "composing", "nextword", "protos", "ranking",
  "dispatch", "swift-ffi", "android-jni",
  "lexicon",              # NEW
  "mmap-host",            # NEW
  "build-helpers/fst-builder",  # NEW (binary target)
]

# [workspace.dependencies] additions
fst = "0.4"
memmap2 = "0.9"
indexmap = "2"            # already present — confirmed
lexicon = { path = "./lexicon" }
mmap-host = { path = "./mmap-host" }
```

`fst` 0.4 is the latest stable as of 2026-05-01 (verified via the audit §5 spike at `/tmp/lex-spice/fst-spike` using the same version).

### 1.3 Deletes (post-merge native-bridge cleanup)

None at the Rust workspace level. All deletions are platform-side (§12).

---

## 2. Build pipeline migration — `dictionary/`

### 2.1 File-level changes (Commit 1)

| File | Action | Notes |
|---|---|---|
| `dictionary/build/create_trie.py` | **Rename** to `create_fst.py` + rewrite body | Replaces `marisa_trie.RecordTrie("<I", pairs)` with stdin protocol → `engine/build-helpers/fst-builder`. Output renames to `output/dictionary.fst`. |
| `dictionary/build/__init__.py` | Update | Module exports if any (currently none — file is empty). |
| `dictionary/tools/query_trie.py` | **Delete** | Replaced by `query_fst.py`. |
| `dictionary/tools/query_fst.py` | **Create** | Mirrors `query_trie.py` interface; shells exclusively to `engine/build-helpers/fst-builder` query subcommand (`fst-builder query <fst_path> <prefix>`). No Python `fst` dependency — strict no-Python-C-dep policy from G1. |
| `dictionary/build/create_trie_db.sh` | **Update comments** | Header `MARISA trie` → `fst prefix index`; logic unchanged (still produces `trie.db` SQLite intermediate). |
| `dictionary/build.sh` | **Update** | Step `5. create_trie` → `5. create_fst` + `python3 -m build.create_fst`. |
| `dictionary/README.md` | **Update** | Replace mentions of `dictionary.trie` / `MARISA` with `dictionary.fst` / `fst::Set`. |
| `dictionary/docs/PIPELINE.md` | **Update** | Diagram + table reference. |
| `dictionary/tools/compare_baseline.py` | **Update** | `trie_path = OUTPUT_DIR / "dictionary.fst"`; sha256 manifest key becomes `dictionary.fst`. |

### 2.2 Encoding contract (audit §5 → wire format)

The fst output stores **byte-sorted entries**. For a `(key_str, rowid_u32)` pair, the wire-format byte sequence inserted into `fst::SetBuilder` is:

```
key_bytes (UTF-8)  ||  0xFF  ||  rowid_le_4
```

- `0xFF` is a separator byte chosen because it never appears mid-key in any UTF-8 string (UTF-8 byte-class invariant) and never overlaps with the `key + 0xFF` prefix used by `lookup_prefix` range scans.
- `rowid_le_4` is the 32-bit rowid in little-endian 4 bytes. `dictionary.db` IDs fit in u32 today (≤ ~150k rows); enforce in `fst-builder` (assert + abort on overflow).
- Multiple rowids per key naturally appear as multiple entries; consumer iterates the range `[key+0xFF, key+0x100)` and decodes the trailing 4 bytes per hit, preserving insertion order via fst's deterministic byte-sorted iteration.

### 2.3 `fst-builder` Rust binary contract

Stdin protocol (line-protocol, simplest):

```
TL\tkey1\trowid1\n
TL\tkey2\trowid2\n
...
EOF
```

- Tab-separated. First field is the literal `TL`; any other marker is rejected (builder exits 1 with diagnostic). Second = key string. Third = decimal rowid.
- Builder accumulates pairs in memory, sorts by `key + 0xFF + rowid_le` byte order (required for `fst::SetBuilder`), then writes to the path passed as `argv[1]`.
- Exit code 0 on success, 1 on error (with diagnostic to stderr).

The Python `create_fst.py` invokes:

```
proc = subprocess.run(
  [str(BUILDER_BIN), str(OUTPUT_FILE)],
  input=stdin_bytes, capture_output=True, check=True,
)
```

`BUILDER_BIN` resolves to `engine/target/release/fst-builder` (built once via `cargo build --release -p fst-builder` in `dictionary/build.sh` before invoking `create_fst`).

### 2.4 dictionary.fst rowid parity test

`engine/lexicon/tests/parity.rs` — `INVARIANT_LEX_FST_ROWID_PAYLOAD` — opens a sample `dictionary.fst` shipped with the test fixtures (committed via Git LFS or a tiny generated fixture), iterates 6 representative prefixes (`tl:gua`, `tl:hoo`, `poj:goa`, `hanzi:好`, `tl:kau`, `tl:sing`), and compares hit count + rowid set parity against a known baseline JSON checked in alongside.

---

## 3. Proto schema — `engine/protos/proto/lexicon.proto`

### 3.1 Tag allocation (G2 locked)

```protobuf
message LexiconRequest {
  oneof method {
    ProcessCandidatesRequest  process_candidates    = 10;  // existing
    InstallRequest            install               = 11;
    SearchRequest             search                = 12;
    SearchWithSourcesRequest  search_with_sources   = 13;
    SearchByHanziRequest      search_by_hanzi       = 14;
    AssocLookupRequest        assoc_lookup          = 15;
  }
}

message LexiconResponse {
  oneof result {
    ProcessCandidatesResponse  process_candidates_result   = 10;
    InstallResponse            install_result              = 11;
    SearchResponse             search_result               = 12;
    SearchWithSourcesResponse  search_with_sources_result  = 13;
    SearchByHanziResponse      search_by_hanzi_result      = 14;
    AssocLookupResponse        assoc_lookup_result         = 15;
  }
}
```

### 3.2 Request payloads

```protobuf
// Tag 11 — install (idempotent; replaces existing readers atomically;
// supports Android asset-version-bump reinstall per audit D-9).
message InstallRequest {
  string trie_path             = 1;  // absolute path to dictionary.fst
  string dictionary_bin_path   = 2;  // absolute path to dictionary.bin (TKDB)
  string association_bin_path  = 3;  // absolute path to association.bin (TKWA, bundled)
  uint32 dictionary_version    = 4;  // platform-supplied stamp (Android: dictionary_app_version.txt; iOS: bundle build number)
}

// Tag 12 — autocomplete entry point.
// `input` = pre-normalized raw input (segmentedInput on iOS, post-buildSearchKey
// drop on Android per audit D-1 resolution). Engine runs Rust normalize_input
// internally as part of search.
message SearchRequest {
  string input                       = 1;
  InputType input_type               = 2;
  InputMode input_mode               = 3;
  uint32 limit                       = 4;
  bool tps_or_mapped_to_er           = 5;
  uint32 enabled_sources_bitmask     = 6;
}

// Tag 13 — Tab3 all-source lookup; matches iOS DictionaryRepository.searchWithSources.
message SearchWithSourcesRequest {
  string input               = 1;
  InputMode input_mode       = 2;
  uint32 limit               = 3;
}

// Tag 14 — Tab3 hanzi-prefix lookup; matches iOS DictionaryRepository.searchByHanzi.
message SearchByHanziRequest {
  string query               = 1;
  InputMode input_mode       = 2;
  uint32 limit               = 3;
}

// Tag 15 — bundled-bigram lookup. Called by the platform NextWord services
// (iOS NextWordService.swift / Android NextWordService.kt) through the
// platform bridges (`RustEngineBridge.lexiconAssocLookup` /
// `LexiconBridge.assocLookup`). The Rust `engine/nextword` crate stays
// independent of `engine/lexicon` — no cross-crate import (preserves
// audit § cross-module isolation goal 1).
message AssocLookupRequest {
  string previous_word       = 1;
  uint32 limit               = 2;
}
```

### 3.3 Response payloads

```protobuf
message InstallResponse {
  uint64 dictionary_record_count   = 1;  // diagnostic
  uint64 prefix_index_entry_count  = 2;  // diagnostic
}

message SearchResponse {
  repeated LexiconRow rows = 1;
}

message SearchWithSourcesResponse {
  repeated LexiconRow rows = 1;
}

message SearchByHanziResponse {
  repeated LexiconRow rows = 1;
}

message AssocLookupResponse {
  repeated LexiconAssocEntry entries = 1;
}
```

### 3.4 Shared value types (new in lexicon.proto)

```protobuf
// Mirrors iOS Lexicon/Models/TaigiWord (without the App-side fields like
// `displayText`). Bridges synthesize the platform-side `TaigiWord` from this.
message LexiconRow {
  int64 id                        = 1;
  string roman                    = 2;
  optional string hanji           = 3;
  optional int32 length_score     = 4;
  optional uint32 source_bitmask  = 5;
}

// Bundled association entry (read-only). The user_association.db SQLite
// half is OUT OF SCOPE for v3.5.6 per scope memo D-Boundary.
message LexiconAssocEntry {
  string previous_word     = 1;
  string candidate_word    = 2;
  uint32 count             = 3;
}

enum InputType {
  INPUT_TYPE_UNSPECIFIED      = 0;
  INPUT_TYPE_ROMAN_NO_TONE    = 1;
  INPUT_TYPE_ROMAN_WITH_TONE  = 2;
  INPUT_TYPE_HANZI            = 3;
}

enum InputMode {
  INPUT_MODE_UNSPECIFIED   = 0;
  INPUT_MODE_TL            = 1;
  INPUT_MODE_POJ           = 2;
  INPUT_MODE_TPS           = 3;
}
```

`InputType` / `InputMode` are intentionally lexicon-local (NOT promoted to `envelope.proto`) — only lexicon search consumes them.

---

## 4. Rust impl — `engine/mmap-host/`

### 4.1 Cargo.toml

```toml
[package]
name = "mmap-host"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
memmap2 = { workspace = true }
thiserror = { workspace = true }

[lints.rust]
# Carve-out: this crate IS the unsafe boundary for mmap.
# All other engine crates remain forbid(unsafe_code).
unsafe_code = "allow"
```

### 4.2 src/lib.rs (~30 LOC)

Public surface:

```rust
pub struct MmapHandle { /* opaque, owns Mmap */ }
impl MmapHandle {
    pub fn open_readonly(path: &Path) -> Result<Self, MmapError>;
    pub fn as_slice(&self) -> &[u8];
    pub fn len(&self) -> usize;
}

#[derive(thiserror::Error, Debug)]
pub enum MmapError { /* Io variants */ }
```

Internally: `unsafe { memmap2::Mmap::map(&File::open(path)?)? }` — single `unsafe` block, documented.

`MmapHandle::as_slice()` returns a `&[u8]` whose lifetime is tied to `&self`, so all engine crate consumers stay safe.

---

## 5. Rust impl — `engine/lexicon/`

### 5.1 Cargo.toml

```toml
[package]
name = "lexicon"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
prost = { workspace = true }
fst = { workspace = true }
indexmap = { workspace = true }
log = { workspace = true }
thiserror = { workspace = true }
once_cell = { workspace = true }
mmap-host = { workspace = true }
phonetics = { workspace = true }
protos = { workspace = true }

[lints.rust]
unsafe_code = "forbid"   # workspace default; reaffirm explicitly
```

### 5.2 File layout (11 files, all <300 LOC)

| # | File | Role | LOC budget |
|---|---|---|---:|
| 1 | `lib.rs` | Re-exports + `static_assert::<dyn Send>` on `Engine` | ~30 |
| 2 | `error.rs` | `LexiconError` `thiserror`-typed; `FAIL_*` mapping for envelope `ErrorCode` | ~30 |
| 3 | `paths.rs` | `LexiconPaths` value type built from `InstallRequest` | ~50 |
| 4 | `handle.rs` | `EngineHandle` singleton; `Mutex<Option<EngineState>>` lifecycle; `install` / `with_state` | ~100 |
| 5 | `key_normalizer.rs` | Trie key construction (`tl:` / `poj:` / `hanzi:` prefixes); calls `phonetics::api::normalize_input` for romanization paths | ~80 |
| 6 | `prefix_index.rs` | fst::Set wrapper. Owns the `MmapHandle` for `dictionary.fst`. `lookup_prefix(prefix: &str) -> Vec<u32>` decodes trailing 4-byte rowids | ~150 |
| 7 | `dictionary_reader.rs` | TKDB mmap reader. `lookup_row(rowid: u32, filter: &Filter) -> Option<LexiconRow>`. `passesFilter` 3-layer (variant excl → khiin excl → source-OR-with-dev) | ~150 |
| 8 | `association_reader.rs` | TKWA mmap reader. `lookup(prev_word: &str, limit: usize) -> Vec<LexiconAssocEntry>`. binary search by raw UTF-8 prev_word; `passesFilter` 1-layer | ~150 |
| 9 | `search.rs` | Orchestration: `search_segmented` / `search_with_sources` / `search_by_hanzi` / `assoc_lookup`. Calls normalize → prefix_index → dictionary_reader → optional TPS er↔or expansion. Stays thin; if ranking creep, split immediately | ~200 |
| 10 | `api.rs` | Public bridge entry points keyed by proto `Method` enum | ~100 |
| 11 | `dispatch.rs` | `Method` → handler dispatch; `handle(req: LexiconRequest, ctx: &AppConfig) -> LexiconResponse` | ~80 |

**Total**: ~1120 LOC. Verified against scope memo file roster.

**Companion crate LOC budgets** (also <300 LOC hard cap each, per `feedback_rust_extraction_goals.md`):

| Crate | File | LOC budget |
|---|---|---:|
| `engine/mmap-host` | `src/lib.rs` | ~30 (~50 hard) |
| `engine/build-helpers/fst-builder` | `src/main.rs` | ~150 |
| `engine/build-helpers/fst-builder` | `src/builder.rs` (sort + emit) | ~80 |
| `engine/build-helpers/fst-builder` | `src/query.rs` (debug subcommand) | ~50 |

If `fst-builder/src/main.rs` itself exceeds 200 LOC during impl, split CLI parsing into `src/cli.rs`.

### 5.3 `EngineHandle` lifecycle

- Singleton via `once_cell::sync::Lazy<Mutex<Option<EngineState>>>`.
- `install(paths: LexiconPaths) -> Result<InstallStats>`:
  1. **Validate paths** (per audit R12 / Codex pre-impl gate):
     - Reject any path containing a NUL byte (`\0`) → `LexiconError::InvalidPath`.
     - Reject non-absolute paths → `LexiconError::PathNotAbsolute`. (Both platforms always pass absolute paths from `LexiconPaths` snapshot; defensive check.)
     - Do **not** call `canonicalize` (avoids symlink-following surprises + extra syscall + Bundle-path edge cases on iOS).
     - All errors return through proto `LexiconResponse.error` mapping; **never panic**, never `unwrap` on user-supplied paths.
  2. Open `MmapHandle` for `dictionary.fst` + `dictionary.bin` + `association.bin`.
  3. Validate magic bytes (TKDB / TKWA) + version.
  4. Build `EngineState { fst_set, dict_reader, assoc_reader, version }`.
  5. `*lock = Some(new_state)` — atomic swap; old state drops (closes mmaps).
  6. If any step 1-4 fails, the lock is never assigned and old state stays intact (atomic-swap-on-success semantics).
- `with_state<F, R>(f: F) -> Result<R>` where `F: FnOnce(&EngineState) -> Result<R>`: holds the mutex, calls `f`, returns. All search methods route through this.
- **Concurrency contract**: `install` and `with_state` share the same mutex. Concurrent `search` calls block on `install` (and vice versa); no read/write split until profiling proves contention. Test `INVARIANT_LEX_INSTALL_SEARCH_SERIALIZATION` (§7) pins this.

D-9 reinstall path: platform calls `install` again with same or new paths. The `*lock = Some(new_state)` happens after the new readers successfully open; if the new install fails, old state stays intact.

### 5.4 `search.rs` orchestration

```
search_segmented(req: SearchRequest) -> SearchResponse:
  1. inputType == HANZI: return SearchResponse { rows: [] }   ← D-8 hard guard
  2. normalize_key = key_normalizer::build(input, input_type, input_mode)
  3. rowids = prefix_index.lookup_prefix(normalize_key)
  4. (TPS er↔or) if input_mode == TPS && tps_or_mapped_to_er && key.contains("er"):
       rowids2 = prefix_index.lookup_prefix(key.replace("er", "or"))
       rowids = indexset_extend(rowids, rowids2)   ← IndexSet preserves insertion (D-12)
  5. rows = rowids.iter().filter_map(|id| dict_reader.lookup_row(id, &filter)).take(limit).collect()
  6. SearchResponse { rows }
```

D-8 step 1 — Rust hard rejects HANZI before custom-dict / system-dict; both platforms wrap so iOS deletes the pre-engine custom-dict call when `inputType == .hanzi`. Verified by `INVARIANT_LEX_HANZI_GUARD` parity test (Commit 12).

D-12 — `IndexSet<u32>` insertion-order preservation; matches Android `(exactRowIds + prefixRowIds).distinct()` semantics. iOS-side `Array(Set(...))` non-deterministic order discarded (parity-correction toward Android per `cross-platform-alignment.md` §1b).

---

## 6. Engine dispatch plumbing — `engine/dispatch/src/lib.rs`

The top-level dispatch matches on the envelope `Request.payload`, then for the `Lexicon` arm matches a second time on `LexiconRequest.method` — tag 10 routes to the existing `ranking` crate, tags 11-15 route to the new `lexicon` crate. No cross-crate coupling between `lexicon` and `ranking`; both handle disjoint slices of the same `LexiconRequest` oneof.

```rust
match req.payload {
    Some(Phonetics(r))  => phonetics::dispatch::handle(r, &config),
    Some(Composing(r))  => composing::dispatch::handle(r, &config),
    Some(Lexicon(lex_req)) => match lex_req.method {
        Some(LexiconMethod::ProcessCandidates(r))  => ranking::process_candidates(r, &config),     // tag 10 — unchanged
        Some(LexiconMethod::Install(r))            => lexicon::dispatch::handle_install(r),         // tag 11 — NEW
        Some(LexiconMethod::Search(r))             => lexicon::dispatch::handle_search(r, &config), // tag 12 — NEW
        Some(LexiconMethod::SearchWithSources(r))  => lexicon::dispatch::handle_search_with_sources(r, &config), // tag 13 — NEW
        Some(LexiconMethod::SearchByHanzi(r))      => lexicon::dispatch::handle_search_by_hanzi(r, &config),     // tag 14 — NEW
        Some(LexiconMethod::AssocLookup(r))        => lexicon::dispatch::handle_assoc_lookup(r),    // tag 15 — NEW
        None => Err(FAIL_INVARIANT),
    },
    Some(Nextword(r))   => nextword::dispatch::handle(r, &config),
    None => Err(FAIL_INVARIANT),
}
```

`lexicon::dispatch::handle_*` are the per-method entry points exported from `engine/lexicon/src/dispatch.rs`. Each marshals the proto request → calls `EngineHandle::install` or `with_state`, then marshals the result → `LexiconResponse`.

---

## 7. Parity tests — `engine/lexicon/tests/parity.rs`

INVARIANT_LEX_* parity tests (per audit §1b parity correction tier requirements):

| Invariant | What it pins | Run on |
|---|---|---|
| `INVARIANT_LEX_FILTER_BITMASK` | 12-bit `bitToSource` map; 3-layer `passesFilter` order; association `passesFilter` | Rust unit test |
| `INVARIANT_LEX_BINARY_FORMAT` | TKDB / TKWA magic bytes; version `1`; little-endian; strict UTF-8 reject | Rust unit test |
| `INVARIANT_LEX_PREFIX_KEYS` | `tl:` / `poj:` / `hanzi:` prefix construction parity vs Swift/Kotlin output | Rust integration |
| `INVARIANT_LEX_FST_ROWID_PAYLOAD` | fst rowid encoding `key + 0xFF + rowid_le_4` round-trip; 6-prefix hit-count baseline (per audit §5) | Rust integration |
| `INVARIANT_LEX_LOOKUP_ROWIDS_ORDER` | IndexSet insertion-order dedup (D-12 parity correction) | Rust integration |
| `INVARIANT_LEX_HANZI_GUARD` | D-8 — hanzi `inputType` returns `[]` BEFORE custom-dict | **Three-tier**: Rust unit (engine), iOS XCTest (autocomplete/lexicon-service layer with seeded custom-dict), Android JUnit (autocomplete service layer with seeded custom-dict) per Codex Mod 1 |
| `INVARIANT_LEX_TPS_ER_OR` | TPS er↔or expansion runs only when `inputMode == TPS && tps_or_mapped_to_er && key.contains("er")` | Rust integration |
| `INVARIANT_LEX_INSTALL_IDEMPOTENT` | install can be called twice with same/different paths; mmap atomically swaps; no leaks | Rust unit test |
| `INVARIANT_LEX_INSTALL_SEARCH_SERIALIZATION` | Concurrent install + search calls (8-thread fan-out, 1k iterations each) — no panic, no UB, all `search` calls return either pre-install or post-install state coherently (never half-swapped). Pins the `EngineHandle` Mutex contract from §5.3. | Rust integration test (proptest where appropriate) |
| `INVARIANT_LEX_INSTALL_PATH_VALIDATION` | install rejects NUL bytes in any path / non-absolute paths; returns `LexiconError::InvalidPath` / `PathNotAbsolute` via proto; never panics | Rust unit test |

`INVARIANT_LEX_HANZI_GUARD` is the only invariant that **must** also run on the platform side, because D-8's regression surface is custom-dict-on-hanzi which only exists outside the Rust engine. See §11.

---

## 8. iOS bridge — `RustEngineBridge+Lexicon.swift`

### 8.1 Synth types (co-located inside extension)

```swift
public extension RustEngineBridge {
    struct LexiconRow: Equatable {
        public let id: Int64
        public let roman: String
        public let hanji: String?
        public let lengthScore: Int32?
        public let sourceBitmask: UInt32?
    }

    struct LexiconAssocEntry: Equatable {
        public let previousWord: String
        public let candidateWord: String
        public let count: UInt32
    }

    struct LexiconInstallStats: Equatable {
        public let dictionaryRecordCount: UInt64
        public let prefixIndexEntryCount: UInt64
    }
}
```

### 8.2 Methods

```swift
public extension RustEngineBridge {
    func lexiconInstall(
        triePath: String,
        dictionaryBinPath: String,
        associationBinPath: String,
        dictionaryVersion: UInt32
    ) throws -> LexiconInstallStats

    func lexiconSearch(
        input: String,
        inputType: InputType,
        inputMode: InputMode,
        limit: UInt32,
        tpsOrMappedToER: Bool,
        enabledSourcesBitmask: UInt32
    ) throws -> [LexiconRow]

    func lexiconSearchWithSources(input: String, inputMode: InputMode, limit: UInt32) throws -> [LexiconRow]

    func lexiconSearchByHanzi(query: String, inputMode: InputMode, limit: UInt32) throws -> [LexiconRow]

    func lexiconAssocLookup(previousWord: String, limit: UInt32) throws -> [LexiconAssocEntry]
}
```

**Soft cap reminder**: synth types + 5 methods budget ~250 LOC; if we break 300, split synth types into `RustEngineBridge+LexiconTypes.swift` per G3 lock.

### 8.3 Error mapping

`RustEngineBridge.diagnostics().recentErrors` already pipes envelope `ErrorCode` to platform; lexicon throws map through the same path. No new diagnostic surface.

---

## 9. Android bridge — `LexiconBridge.kt`

### 9.1 Top-level object (NOT extension on `RustEngineBridge`)

```kotlin
package com.siansiansu.taigikeyboard.engine

object LexiconBridge {
    data class Row(
        val id: Long,
        val roman: String,
        val hanji: String?,
        val lengthScore: Int?,
        val sourceBitmask: UInt?,
    )

    data class AssocEntry(
        val previousWord: String,
        val candidateWord: String,
        val count: UInt,
    )

    data class InstallStats(
        val dictionaryRecordCount: ULong,
        val prefixIndexEntryCount: ULong,
    )

    fun install(...): InstallStats
    fun search(...): List<Row>
    fun searchWithSources(...): List<Row>
    fun searchByHanzi(...): List<Row>
    fun assocLookup(...): List<AssocEntry>
}
```

`LexiconBridge` calls into the same JNI surface (`engine_dispatch`) used by `RustEngineBridge`, just from a different Kotlin facade. No new JNI entry points beyond the existing `engine_dispatch` byte-protobuf bidirectional channel.

### 9.2 Why divergence from `RustEngineBridge.kt` (G3 trade-off)

- LOC argument — bundling lexicon onto `RustEngineBridge.kt` would add ~200 LOC to a class already at scale.
- Functional cohesion — lexicon is its own domain (read-path resolution + reader install); keeping its facade isolated is cleaner than mixing into the multi-domain `RustEngineBridge.kt`.
- The cross-class call inconsistency (`RustEngineBridge.composingDispatch(...)` vs `LexiconBridge.search(...)`) is annotated in the bridge KDoc.

---

## 10. Platform call-site rewires

### 10.1 iOS (Commit 10)

| Caller | Old call | New call |
|---|---|---|
| `Autocomplete/Services/AutocompleteService.swift::searchLexicon` | `lexiconService.search(...)` | unchanged (LexiconService internally rewires) |
| `Lexicon/Services/LexiconService.swift::search` | composes via `repository.query` | system-dict path → `bridge.lexiconSearch(...)`; **custom-dict, cold-start `mergeOrderOnly`, and `applyCaseProcessing` (engine-adjacent auto-cap) all STAY platform-side** per audit D-3/D-4 |
| `Lexicon/Services/DictionarySearchService.swift::search` | `repository.searchWithSources` / `searchByHanzi` | `bridge.lexiconSearchWithSources(...)` / `bridge.lexiconSearchByHanzi(...)` |
| `Lexicon/Services/NextWordService.swift` | `AssociationBinaryReader.lookup(prevWord:)` | `bridge.lexiconAssocLookup(previousWord: prevWord, limit:)` (rewires association-bin consumer to the lexicon bridge before the reader file is deleted in Commit 13) |
| `App/CompositionRoot.swift` (or equivalent) | constructs `DictionaryRepository` via `TrieService` + `DictionaryBinaryReader` | constructs nothing for read path; the bridge handles state via `lexiconInstall` at engine bootstrap |
| `KeyboardExtension/KeyboardViewController+Setup.swift` | early init for trie + custom dict | platform passes absolute paths from `LexiconPaths` snapshot to `RustEngineBridge.lexiconInstall(...)` once at extension launch |

### 10.2 Android (Commit 11)

| Caller | Old call | New call |
|---|---|---|
| `ime/text/composing/TaigiAutocompleteService.kt::autocomplete` | `lexicon.search(input, ...)` | unchanged (LexiconService internally rewires) |
| `ime/dictionary/LexiconService.kt::search` | composes via `searchWithTrie` / `lookupRowIds` | system-dict path → `LexiconBridge.search(...)`; **custom-dict STAYS platform-side; smartbar-layer auto-cap (`SuggestionCaseTransformer.transform` in `SmartbarManager`) STAYS outside `LexiconService` per audit D-3**. (Android has no cold-start branch — `UserFrequencyService.frequencyDataBatch` is always callable, audit D-4.) |
| `ime/dictionary/LexiconService.kt::searchWithSources` / `searchByHanzi` | direct calls | `LexiconBridge.searchWithSources(...)` / `searchByHanzi(...)` |
| `ime/text/composing/NextWordService.kt` | `AssociationBinaryReader.lookup(previousWord = ...)` | `LexiconBridge.assocLookup(previousWord = ..., limit = ...)` (rewires association-bin consumer before the reader file is deleted in Commit 13) |
| `ui/tabs/tab3/DictionarySearchViewModel.kt` | `root.lexicon.searchWithSources` | unchanged (hits `LexiconService` which now rewires through bridge) |
| `ime/AppInitializer.kt` (or wherever asset-copy calls `copyAssetsIfNeeded`) | post-copy `binaryReader.open(...)` | `LexiconBridge.install(paths)` after asset-copy completes; on Android version-bump path, this triggers the atomic-swap reinstall (audit D-9) |

---

## 11. D-8 isolated commit (Commit 12) — per Codex Mod 2

This commit lands AFTER both platforms are wired through the bridge (Commits 10 + 11) and BEFORE platform mirror deletion (Commit 13). Both impls coexist in HEAD at this point — old Swift/Kotlin `LexiconService.search` still callable, new Rust bridge wired in. The D-8 parity test runs against the bridge call path (which is what survives post-deletion) and asserts the new behavior.

### 11.1 Behavioral invariant doc edit

`docs/architecture/behavioral-invariants.md` — add new invariant section:

```markdown
## INVARIANT_LEX_HANZI_GUARD (v3.5.6)

When `LexiconService.search` is called with `inputType == .hanzi` (iOS) or
`InputType.Hanzi` (Android), the function returns an empty result list `[]`
WITHOUT consulting the custom-dictionary, system-dictionary, or association
binaries.

**Rationale**: pre-v3.5.6, iOS hit `lookupCustomDictionary` BEFORE the engine
hanzi guard, so a custom-dict entry whose key matched the hanzi composing
buffer would surface as a suggestion. Android short-circuited at the top of
`search()`, returning `[]` immediately. v3.5.6 normalizes both platforms to
Android's behavior (per `rules/cross-platform-alignment.md` §1b parity
correction). User-visible regression on iOS: hanzi composing input no longer
surfaces custom-dict matches. Affected scenario is edge-case (paste / long-press
hanzi into composing buffer); normal IME typing flow never triggers this branch.

**Test pinning**: `INVARIANT_LEX_HANZI_GUARD` exists at three layers:
1. Rust engine unit test — `engine/lexicon/tests/parity.rs::hanzi_guard`
2. iOS — `LexiconServiceHanziGuardTests.swift` — seeds a `CustomDictionaryEntry`
   whose key matches a hanzi input, invokes `LexiconService.search(input: "我",
   inputType: .hanzi, ...)`, asserts `[]`.
3. Android — `LexiconServiceHanziGuardTest.kt` — same shape: seed CustomDictionary
   entry, invoke `lexicon.search(input = "我", inputType = InputType.Hanzi, ...)`,
   assert empty.
```

### 11.2 Parity test files (new)

- `ios/TaigiKeyboardTests/LexiconServiceHanziGuardTests.swift` — seeds CustomDictionary entry via `CustomDictionaryService` test double + asserts `LexiconService.search(...)` returns `[]`. Coordinates with whatever in-memory SQLite test plumbing already exists for custom-dict tests. Pure XCTest unit test (no Rust runtime; bridge is mocked via protocol or the existing `RustEngineBridge` test seam).
- `android/app/src/test/java/.../LexiconServiceHanziGuardTest.kt` — mirror. **Pure JVM unit test** (kept under `src/test/java`, NOT `androidTest`); the test injects a fake `LexiconBridge` test double rather than loading the Rust `librust_taigi.so` artifact, so the JVM-loaded-Rust-artifact ban from `feedback_path_g_delete_mirrors.md` is respected. The test exercises only the platform `LexiconService.search` orchestration: hanzi `inputType` → early `[]` return BEFORE the (mocked) bridge / custom-dict are touched.

### 11.3 Why isolated commit (Codex Mod 2)

Per `rules/cross-platform-alignment.md` §1b, parity corrections must be:
- (a) packaged with their own invariant test (✅ — parity test added in this commit)
- (b) NOT bundled with unrelated refactor work (✅ — no other code changes in this commit)
- (c) reviewable as a distinct unit with before/after behavior diff (✅ — commit message documents the iOS behavior change)

The commit message body explicitly enumerates: "iOS hanzi composing input → previously surfaced custom-dict entries; now returns `[]`. Android unchanged. v3.5.6 parity correction toward Android per `cross-platform-alignment.md` §1b."

---

## 12. Platform mirror + native bridge deletions (Commit 13)

Bundled into a single deletion commit (G5 modification). The commit DELETES the dedicated mirror files and MODIFIES the per-platform `LexiconService` (partial reduction).

### 12.1 iOS — 5 deleted Swift files + 1 modified

**Deleted entirely (5 files)**:

- `ios/Sources/TaigiKeyboard/Lexicon/Trie/InputNormalizer.swift` (25 LOC)
- `ios/Sources/TaigiKeyboard/Lexicon/Trie/TrieService.swift` (153)
- `ios/Sources/TaigiKeyboard/Lexicon/Database/AssociationBinaryReader.swift` (217)
- `ios/Sources/TaigiKeyboard/Lexicon/Database/DictionaryBinaryReader.swift` (197)
- `ios/Sources/TaigiKeyboard/Lexicon/Database/DictionaryRepository.swift` (223)

**Modified (1 file)**:

- `ios/Sources/TaigiKeyboard/Lexicon/Services/LexiconService.swift` — system-dict + TPS er↔or paths removed (now go through bridge per Commit 10); **custom-dict + cold-start `mergeOrderOnly` + `applyCaseProcessing` branches STAY** (per audit D-3 / D-4).

### 12.2 iOS native bridge (3 items deleted)

- `ios/Sources/TaigiKeyboard/Lexicon/Trie/marisa_bridge.cpp` (delete entire file)
- `ios/Sources/TaigiKeyboard/Lexicon/Trie/marisa_bridge.h` (delete entire file)
- `ios/Resources/Dictionaries/dictionary.trie` (delete bundled MARISA asset; replaced by `dictionary.fst` from build pipeline in Commit 1)

### 12.3 iOS test cleanup (in same Commit 13)

The deleted Swift sources have unit tests that must also go (per `feedback_path_g_delete_mirrors.md`):

- `ios/TaigiKeyboardTests/InputNormalizerTests.swift` — delete entire file (was testing the now-deleted Swift `InputNormalizer`; equivalent coverage exists in `engine/phonetics` Rust tests since D9.4-Phonetics).
- `ios/TaigiKeyboardTests/EngineIntegrationTests.swift` — delete or update: lines 11/18/24/30/35 directly call deleted `InputNormalizer.normalize`. If remaining lines have unique coverage, prune those calls; otherwise delete.
- `ios/TaigiKeyboardTests/CustomDictionaryDerivationTests.swift` — lines 64-100 reference `InputNormalizer.normalize` for parity. Replace with `RustEngineBridge.normalizeInput(...)` calls (same behavior, different identifier post-D9.4); the test logic stays.

### 12.4 Android — 4 deleted Kotlin files + 1 modified

**Deleted entirely (4 files)**:

- `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/InputNormalizer.kt` (40 LOC)
- `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/TrieService.kt` (169)
- `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/AssociationBinaryReader.kt` (259)
- `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/DictionaryBinaryReader.kt` (230)

**Modified (1 file)**:

- `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/LexiconService.kt` — system-dict + `searchWithTrie` + `lookupRowIds` + `buildSearchResults` removed (rewired through bridge in Commit 11); **custom-dict STAYS platform-side; smartbar-layer auto-cap stays out of LexiconService entirely**.

### 12.5 Android native bridge + asset

- `android/app/src/main/cpp/trie_jni.cpp` (delete entire file)
- `android/app/src/main/assets/dictionary.trie` (delete bundled MARISA asset; replaced by `dictionary.fst` from build pipeline)

### 12.6 Android test cleanup (in same Commit 13)

- Any JUnit test under `android/app/src/test/java/.../dictionary/` that targeted the deleted Kotlin sources (e.g. `InputNormalizerTest.kt`, `TrieServiceTest.kt`, `DictionaryBinaryReaderTest.kt`, `AssociationBinaryReaderTest.kt` — verify presence at impl time) — delete or update to call `LexiconBridge` instead. Pure JVM tests with fake bridge are fine; tests that load Rust artifacts are not (per `feedback_path_g_delete_mirrors.md`).

### 12.7 USER-ONLY manual edits flagged in PR description

Per `feedback_xcode_manual.md` and `feedback_manual_build_test.md`, the following are manual:

**iOS (Xcode pbxproj)**:
1. Remove file references for the **5 deleted Swift files** (§12.1) from the keyboard extension target. (DO NOT remove `LexiconService.swift` — it is modified, not deleted.)
2. Remove `marisa_bridge.cpp` / `.h` from compile sources + headers (§12.2).
3. Remove iOS test references for the deleted/modified test files in §12.3.
4. Remove `dictionary.trie` from "Copy Bundle Resources" → keyboard extension.
5. Add `dictionary.fst` to "Copy Bundle Resources" → keyboard extension (file exists in `ios/Resources/Dictionaries/dictionary.fst` after the build-pipeline commit).
6. Verify keyboard extension still links (no remaining MARISA symbol references).

**Android (Gradle + CMake)**:
1. `android/app/build.gradle` — remove `taigi_trie` from `externalNativeBuild.cmake.targets` (or whatever the target list looks like).
2. `android/app/src/main/cpp/CMakeLists.txt` — remove the `taigi_trie` library + its source + the prebuilt MARISA static library reference.
3. `android/app/src/main/assets/dictionary.trie` is deleted by the commit; verify Gradle assets dir packaging aligns.

---

## 13. Commit slicing (G5 LOCKED — 13 commits)

| # | Title | Files touched | Codex-reviewable? |
|---|---|---|---:|
| 1 | `build(dictionary): migrate trie pipeline to fst` | `dictionary/build/create_trie.py` (rename → `create_fst.py`), `tools/query_trie.py` → `query_fst.py`, `build.sh`, `create_trie_db.sh`, `README.md`, `docs/PIPELINE.md`, `tools/compare_baseline.py`, NEW `engine/build-helpers/fst-builder/` (Cargo.toml + main.rs), `engine/Cargo.toml` (workspace member) | ✅ |
| 2 | `feat(protos): extend lexicon.proto with read-path methods` | `engine/protos/proto/lexicon.proto`, `engine/protos/build.rs` if needed | ✅ |
| 3 | `feat(engine): add mmap-host crate` | NEW `engine/mmap-host/{Cargo.toml,src/lib.rs}`, `engine/Cargo.toml` | ✅ |
| 4 | `feat(engine): scaffold lexicon crate (lib/error/paths/handle)` | NEW `engine/lexicon/{Cargo.toml,src/lib.rs,error.rs,paths.rs,handle.rs}`, `engine/Cargo.toml` | ✅ |
| 5 | `feat(engine): lexicon body (normalizer/index/readers/search/api/dispatch)` | NEW `engine/lexicon/src/{key_normalizer.rs,prefix_index.rs,dictionary_reader.rs,association_reader.rs,search.rs,api.rs,dispatch.rs}` | ✅ |
| 6 | `feat(engine): wire lexicon into top-level dispatch` | `engine/dispatch/src/lib.rs` | ✅ |
| 7 | `test(engine): INVARIANT_LEX_* parity tests` | NEW `engine/lexicon/tests/parity.rs` + fixture data | ✅ |
| 8 | `feat(ios): RustEngineBridge+Lexicon` | NEW `ios/Sources/TaigiKeyboard/Engine/RustEngineBridge+Lexicon.swift` | ✅ |
| 9 | `feat(android): LexiconBridge` | NEW `android/app/src/main/java/.../engine/LexiconBridge.kt` | ✅ |
| 10 | `refactor(ios): rewire LexiconService + DictionarySearchService + NextWordService through bridge` | `ios/Sources/TaigiKeyboard/Lexicon/Services/LexiconService.swift`, `DictionarySearchService.swift`, `Lexicon/Services/NextWordService.swift` (assoc lookup rewire), `Autocomplete/Services/AutocompleteService.swift`, `App/CompositionRoot.swift`, `KeyboardExtension/KeyboardViewController+Setup.swift` | ✅ |
| 11 | `refactor(android): rewire LexiconService + Tab3 search + NextWordService through bridge` | `android/app/src/main/java/.../ime/dictionary/LexiconService.kt`, `ime/text/composing/TaigiAutocompleteService.kt`, `ime/text/composing/NextWordService.kt` (assoc lookup rewire), `ui/tabs/tab3/DictionarySearchViewModel.kt`, `ime/AppInitializer.kt` (or wherever post-asset-copy install hook lives) | ✅ |
| 12 | `fix(lexicon): D-8 hanzi guard parity correction toward Android` | `docs/architecture/behavioral-invariants.md`, NEW `ios/TaigiKeyboardTests/LexiconServiceHanziGuardTests.swift`, NEW `android/app/src/test/java/.../LexiconServiceHanziGuardTest.kt` (pure JVM, fake bridge) | ✅ ISOLATED |
| 13 | `chore: delete legacy mirror + native trie` | DELETE iOS 5 Swift mirrors (§12.1) + 3 native bridge/asset items (§12.2); MODIFY iOS `LexiconService.swift` (partial reduction); update/delete iOS tests in §12.3. DELETE Android 4 Kotlin mirrors (§12.4) + 2 native bridge/asset items (§12.5); MODIFY Android `LexiconService.kt` (partial reduction); update/delete Android tests in §12.6. PR description flags pbxproj + CMake/build.gradle for user. | ✅ |

---

## 14. Risks + mitigations

| # | Risk | Likelihood | Mitigation |
|---|---|---:|---|
| R1 | fst lookup latency regression vs MARISA | Low | Audit §5 spike showed 28-150 µs across 6 prefixes (within budget). Parity test asserts < 200 µs p95. |
| R2 | Bundle size +3.93 MB | Confirmed (audit §5) | Acceptable. iOS keyboard extension under 60 MB hard cap; Android APK assets +3.93 MB also acceptable. |
| R3 | mmap lifetime escapes `MmapHandle` | Low | `engine/mmap-host` is the only unsafe crate; `&[u8]` lifetime tied to `&self`. Review Commit 3 isolation carefully. |
| R4 | iOS App Group path injection | None — read-only assets stay in Bundle (audit D-11) | — |
| R5 | Android asset-version-bump race during `LexiconBridge.install` reinstall | Medium | `EngineHandle` mutex ensures atomic swap; if `install` fails new state is rejected, old state stays. Manually exercise during dogfood (S2). |
| R6 | iOS hanzi-input regression (D-8) catches a real user | Low (Codex 91% confidence; 9% reserve) | INVARIANT_LEX_HANZI_GUARD parity test on autocomplete/lexicon-service layer with seeded custom-dict. R-list dogfood R3.6 below. |
| R7 | Codex post-impl finds new P0 / P1 | Likely (every prior slice surfaced 2-4 fixes) | Commit slicing accommodates fix commits between Commit 13 and PR open. Each fix gets its own commit per `feedback_codex_review_sandwich.md`. |
| R8 | Logger fires before `lexicon::install` completes (search call lands while lock holds `None`) | Low-Medium | `EngineHandle::with_state` returns `Err(LexiconError::NotInitialized)` when state is `None`; platform bridges surface this through diagnostics, not as a panic. Bridge call sites guard with a "wait until install" flag set by the install completion callback (iOS extension launch + Android `AppInitializer` already gate composing/nextword similarly). Test: `INVARIANT_LEX_INSTALL_SEARCH_SERIALIZATION` exercises a search-before-install case in addition to the concurrent fan-out. |
| R9 | Malformed install paths (NUL bytes, non-absolute) | Low (defensive) | §5.3 install validation; `INVARIANT_LEX_INSTALL_PATH_VALIDATION` test pins reject behavior; never panics. |
| R10 | Concurrent install/search racing leads to half-swapped state | Low | `EngineHandle::Mutex<Option<EngineState>>` plus atomic-swap-on-success semantics (§5.3 step 6). `INVARIANT_LEX_INSTALL_SEARCH_SERIALIZATION` (§7) fans out 8 threads × 1k iterations to verify. |

---

## 15. Codex sandwich gates (per `feedback_codex_review_sandwich.md`)

### 15.1 Pre-impl gate (against this plan, NOT the audit)

- Plan goes to Codex via `codex exec` with this file as input.
- Codex returns AGREE / NEEDS-FIXES per section.
- Iterate until APPROVED before opening `phase4b/v3.5.6-lexicon-readpath`.
- Pre-impl gate output recorded in `project_v3_5_6_audit_progress.md` extension or new `project_v3_5_6_plan_progress.md`.

### 15.2 Post-impl gate (against the diff)

- After Commit 13 lands locally, push branch + run Codex on the full diff.
- Codex returns P0 / P1 / P2 by file.
- Iterate fixes (each as its own commit) until APPROVED.
- Then open PR for user dogfood + manual pbxproj/CMake actions + merge.

### 15.3 `/simplify` round (per `feedback_review_before_impl.md`)

After Codex post-impl APPROVES, run `/simplify` on the post-impl HEAD; address any low-priority cleanups before PR open.

---

## 16. Per-slice cadence template (per `project_rust_migration_cadence.md`)

1. ✅ INVARIANT_LEX_* parity tests on Rust workspace + both platforms (§7 + §11).
2. ✅ ~~Per-slice Settings toggle~~ — DROPPED per `feedback_no_slice_toggles.md`.
3. ✅ Production swap: every site routes through `LexiconBridge.search` directly. Old impl deleted in same PR (path-G per `feedback_path_g_delete_mirrors.md`).
4. v3.5.6 R1-R4 dogfood matrix — designed at release time per `project_v3_5_5_dogfood_matrix.md` template.
5. ✅ Codex sandwich pre + post per `feedback_codex_review_sandwich.md`.

---

## 17. Cross-references

- `docs/engine/lexicon-slice-audit.md` — the audit (468 lines, Codex round-3 APPROVED 2026-05-01)
- `docs/architecture/behavioral-invariants.md` — INVARIANT_LEX_HANZI_GUARD lands here in Commit 12
- `project_v3_5_6_lexicon_scope.md` — OPT-A scope memo (locked 2026-05-01)
- `project_v3_5_6_audit_progress.md` — D-8 sign-off record (auto-mode joint A+mods)
- `project_v3_5_6_next_round_prompts.md` — Prompt 2 (this plan) → 3 → 4 hand-off
- `project_rust_migration_cadence.md` — release map + per-slice cadence
- `feedback_codex_review_sandwich.md` — pre + post gates (§15)
- `feedback_path_g_delete_mirrors.md` — mirror deletion philosophy (§12)
- `feedback_xcode_manual.md` — pbxproj edits stay user-only (§12.5)
- `feedback_manual_build_test.md` — xcodebuild / gradlew runs stay user-only
- `feedback_rust_extraction_goals.md` — 4 design goals applied throughout
- `rules/cross-platform-alignment.md` §1b — parity correction tier (D-8 + D-12)
- `rules/rust-best-practices.md` — workspace layout, FFI safety, `unsafe_code = "forbid"` discipline

---

**Plan closed at draft. Awaits Codex pre-impl review (§15.1).**
