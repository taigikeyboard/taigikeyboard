---
paths: ["engine/**/*.rs", "engine/**/Cargo.toml"]
---

# Rust Best Practices

Mandatory rules for Rust shared-core development. Read before any Rust code lands in the engine. FFI / proto-boundary / `unsafe` / opaque-handle / enforcement rules are split into `.claude/rules/rust-ffi-safety.md`.

**Three goals** every rule below serves at least one of:

- **R — Rust-idiomatic**: follows community conventions; catches bugs at compile time
- **S — Safety**: no undefined behavior, no unwind across FFI, no data races
- **A — Anti-regression**: predictable builds, reproducible CI, supply-chain hygiene

Each rule is tagged with one or more of `[R]`, `[S]`, `[A]`.

## 1. Workspace layout `[R]` `[A]`

Cargo workspace with one crate per concern. Models khiin-rs (`references/khiin-rs/Cargo.toml`) with deliberate deviations. Full crate list lives in `docs/architecture/system-overview.md`; the runtime dependency graph + layering invariant in §1a below.

- **`phonetics`** — pure POJ/TL/TPS / Unicode / tone, no I/O, zero platform dependencies, portable to any Rust target. Corresponds to `knowledge/` reference content + `taigi-converter/` behavior.
- **Domain crates** (`composing`, `lexicon`, `ranking`, `nextword`) own the state machines, candidate scoring, and next-word prediction. Dependency direction per §1a.
- **FFI crates** (`android-jni`, `swift-ffi`) are **thin** — protobuf in / protobuf out / `catch_unwind`. No domain logic. Each crate's `lib.rs` should be < 300 LOC.
- Depend via `workspace.dependencies` in root `Cargo.toml` with pinned versions. Workspace-internal deps use relative paths (`phonetics = { path = "./phonetics" }`).

## 1a. Crate layering & dependency direction `[R]` `[A]`

The §1 sketch is the original khiin-rs-modeled target. The workspace has since split the stateful `engine/` crate into discrete domain crates. Current runtime crates — dependency edges flow **one way, top → bottom** (the offline `build-helpers/fst-builder` producer sits outside this runtime graph):

```
┌─ adapters ─────────────────────────────────────────────────┐
│  swift-ffi · android-jni   thin: bytes in/out, catch_unwind │
└───────────────────────────┬─────────────────────────────────┘
                            │ depends ↓
┌─ use-case ────────────────┴─────────────────────────────────┐
│  dispatch                  only crate that sees all domains  │
└───────────────────────────┬─────────────────────────────────┘
                            │ depends ↓
┌─ domain ──────────────────┴─────────────────────────────────┐
│  composing → lexicon, ranking, phonetics                     │
│  lexicon   → ranking, phonetics, mmap-host                   │
│  ranking   → phonetics                                       │
│  nextword  → phonetics                                       │
└───────────────────────────┬─────────────────────────────────┘
                            │ depends ↓
┌─ leaf / shared kernel ────┴─────────────────────────────────┐
│  phonetics  pure fns (POJ/TL/TPS, tone, normalize)           │
│  protos     prost-generated message types (shared by all)    │
│  mmap-host  unsafe mmap carve-out (infra)                     │
└──────────────────────────────────────────────────────────────┘
```

**Dependency-direction invariant** — a crate may depend only on crates in its own layer or below:

- **Forbidden upward edges**: no domain crate (`phonetics` / `ranking` / `lexicon` / `nextword` / `composing`) may depend on `dispatch` or an FFI crate; the leaf layer (`phonetics` / `protos` / `mmap-host`) may depend on nothing above itself.
- **`dispatch` is the only orchestrator** — the single crate allowed to reference every domain. FFI crates (`swift-ffi` / `android-jni`) see only `dispatch` + `protos`.
- **Cargo enforces acyclicity at build time** (a cycle fails to compile) — that is the hard backstop. This layering rule is the *soft* guide that stops the graph degrading into flat all-depends-on-all while still technically acyclic.
- **New crate / new edge**: place it so the arrow still points down. If a domain crate appears to need something currently in `dispatch`, that is an inversion — push the shared piece **down** into `phonetics` / `protos`, never add an upward edge (mirrors `~/.claude/rules/planning.md` § No redundant fallback — keep data flow one-direction).

A visual copy of this graph plus the per-keystroke request lane lives in `docs/architecture/system-overview.md` § 2 Engine crate dependency graph.

## 2. Error handling `[R]` `[S]`

- **`thiserror` for library errors**. Every public engine error derives `thiserror::Error`:
  ```rust
  #[derive(thiserror::Error, Debug)]
  pub enum EngineError {
      #[error("invalid protobuf: {0}")]
      InvalidProto(#[from] prost::DecodeError),
      #[error("database locked")]
      DatabaseLocked,
      #[error("internal panic: {0}")]
      InternalPanic(String),
  }
  ```
- **`anyhow` is forbidden in library crates** (`phonetics`, `engine`, `protos`). Allowed in build scripts only.
- **`Result<T, EngineError>` throughout internal APIs.** Encode into `Response.ErrorCode` only at the FFI edge (see `.claude/rules/rust-ffi-safety.md` § FFI boundary discipline).
- **No `panic!` / `unwrap()` / `expect()` on unvalidated input.** `unwrap()` on a `Mutex::lock()` result is acceptable (poison is a programmer error, not a data path); briefly explain with `// JUSTIFICATION:` when non-obvious. `SAFETY:` comments are reserved for `unsafe` blocks per `.claude/rules/rust-ffi-safety.md` §3 — a safe `Mutex::lock().unwrap()` does not take one.
- **`?` is allowed and idiomatic inside the `catch_unwind` closure** (which returns `Result<Vec<u8>, EngineError>`). What is banned is propagating a `Result` out of the FFI function itself — the outer `extern fn` must return protobuf bytes or a null sentinel, never a Rust `Result` or `Option`. Encode errors into `Response.ErrorCode` at the seam between closure and extern fn.

## 3. Crate + type choices `[R]` `[A]`

Pinned choices (deviations require written justification):

| Concern | Choice | Rationale |
|---|---|---|
| Protobuf codegen | **`prost` + `prost-build`** | Modern, zero-copy-friendly, integrates with tonic. Rejects older `rust-protobuf` that khiin-rs uses. |
| SQLite driver | **`rusqlite` with `bundled` feature** | Cross-platform consistent; matches khiin-rs (validated). |
| Trie / prefix lookup | **`fst` crate** | Pure-Rust, mmap-friendly (iOS extension memory budget), replaces JNI-bound MARISA. Decision pinned in Phase IV-B Lexicon slice. |
| JNI wrapper | **`jni` crate** | Idiomatic safe wrapper; rejects hand-rolled `extern` fns per khiin-rs style. |
| Swift-bridge | **`swift-bridge` crate** | Matches khiin-rs; proven on iOS + macOS. |
| Logging | **`log` crate** + per-platform adapter | Standard Rust idiom. |
| Error types | **`thiserror`** (libs) + `anyhow` (bins/scripts only) | Community standard. |
| Async runtime | **None — sync only** | IME latency budget (< 50ms keystroke roundtrip) does not require async. Rejects `tokio` / `async-std` complexity. |

Type-shape preferences that cross FFI:

- `#[repr(transparent)]` newtype for opaque handles (see `.claude/rules/rust-ffi-safety.md` § Opaque handle pattern).
- **Explicit numeric widths**: `i64` / `f64` at the boundary (not `isize` / `usize`). Mirrors `.claude/rules/android-guidelines.md` §1 Kotlin→Rust shape rules.
- **UTF-8 strings only** — protobuf `string` already enforces; never use `&[u8]` for text data.

## 4. Cross-compile + build tooling `[A]`

- **`cargo-make` + `Makefile.toml`** as the build orchestrator. Matches khiin-rs pattern (validated across 4 platforms). Tasks at minimum: `build-db`, `build-droid`, `build-swift`.
- **Android**: `cargo-ndk` for multi-ABI builds (`arm64-v8a` mandatory; `x86_64` for emulator testing only — khiin-rs documents a known `x86_64` NDK crash in `android/README.md`, acceptable for emulator work).
- **iOS / macOS**: `cargo build --target aarch64-apple-ios` + simulator targets; packaged as xcframework via `swift-bridge` generator.
- **Rustup targets** pinned in `rust-toolchain.toml`:
  ```toml
  [toolchain]
  channel = "stable"
  components = ["rustfmt", "clippy"]
  targets = [
      "aarch64-linux-android", "armv7-linux-androideabi", "x86_64-linux-android",
      "aarch64-apple-ios", "aarch64-apple-ios-sim", "x86_64-apple-ios",
  ]
  ```

## 5. Testing strategy `[R]` `[A]`

- **Unit tests**: `#[cfg(test)] mod tests` alongside source files. Pure logic (`phonetics`, `engine`) targets ≥ 80% line coverage.
- **Integration tests**: `engine/tests/` exercises the engine through its public API with protobuf messages, no FFI.
- **FFI roundtrip tests**: `android-jni/tests/` + `swift-ffi/tests/` exercise FFI entry points for panic safety, Drop, thread serialization, malformed-bytes handling. Required for D9 POC acceptance.
- **Invariant tests**: every `INVARIANT_*` label from `docs/architecture/behavioral-invariants.md` has a matching Rust test. Drift between platform tests and Rust tests = regression.
- **`cargo test --workspace`** must pass in CI before any PR merges.
- **Property tests** (`proptest`) for phonetics round-trip invariants (TL↔POJ↔TPS) — complements hand-written invariant tests.

## 6. Rust version policy `[A]`

- **Stable channel only.** No nightly features, no `#![feature(...)]`.
- **MSRV pinned at Rust 1.86** in root `Cargo.toml` (`rust-version = "1.86"`). Rationale: 1.85 (Feb 2025) stabilises edition2024 and 1.86 (Apr 2025) is the next stable. Bumped during Phase III D9.2 because `cargo-ndk` 4.x requires 1.86 and the dev-tool gap is not worth carrying a 3.5.x sidegrade for. Earlier pins (1.75 → 1.85 in D9.1) similarly bumped to clear active-tooling gaps. Bumping MSRV further remains a PR-level decision with CI verification.
- **No experimental features** (`async fn` in traits — stable since 1.75 — OK; GATs in traits OK; const generics full — OK; edition2024 — OK on 1.85+).
- **`rustfmt` default config**, no deviations. Apply with `make fmt`; check without writing via `cd engine && cargo fmt --all -- --check` (§7 is judgment-gated, not mandatory).
- **`clippy` with `-D warnings`** available via `make lint` (runs clippy + Kotlin spotlessCheck). Project-wide allow list lives in workspace `Cargo.toml` `[workspace.lints]`.

## 7. Pre-commit gate + supply chain `[A]`

This project runs the Rust gate **locally**, not via GitHub Actions. Mirrors `~/.claude/rules/round-workflow.md` "Build & changelog" for `./gradlew` / `xcodebuild`: the user runs build/test, AI does not. The user decides per-round whether the full canonical gate runs, a subset runs, or no gate runs.

- **No mandatory canonical 4-cmd pre-PR gate.** USER explicitly retains judgment per-round on whether to run `cargo fmt --all -- --check` / `cargo check --workspace --locked` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace`, in part or in whole. AI does not run these as a fixed gate; AI may surface findings if it noticed something concrete, but never as "you must run the gate now". Rationale: the gate as a mandatory step adds significant wall-clock cost that is wasted on most rounds (docs / single-crate refactor / mechanical rename). Judgment-based use catches what matters; reflexive use does not.
- The full four commands above remain the **canonical recipe** when USER does decide to run a full check — kept as a documented bare-cargo form so it is reproducible.
- `make`-target shortcuts available for round-internal iteration (fast paths) AND canonical form (full paths). See root `Makefile help` for the current target list.
- **`cargo-audit`** scans against the RustSec advisory DB on demand — optional, install via `cargo install cargo-audit --locked`.
- **`cargo-deny check`** enforces dependency policy via `engine/deny.toml`: license allow-list (MIT / Apache-2.0 / BSD / ISC / Unicode-DFS-2016 / Unicode-3.0 / Zlib), `multiple-versions = warn`, `unknown-git = deny`, `unknown-registry = deny`. Optional, install via `cargo install cargo-deny --locked`.
- **FFI integration tests** are run on representative Android emulator + iOS simulator targets when relevant to the round (D9 gate onward) — user-gated, no CI matrix, no fixed schedule.
- **Supply chain**: no git dependencies in `Cargo.toml`. Patches go through explicit `[patch.crates-io]` with version pins and written justification.

## 8. Explicit non-goals

Codifying `.claude/rules/cross-platform-alignment.md` §4.1 in Rust terms:

- **No async runtime** (`tokio`, `async-std`, `smol`). Engine is synchronous. Platform wrappers handle threading.
- **No global state in Rust.** No `lazy_static!` / `once_cell::sync::Lazy` in engine or phonetics crates. Engine lifetime is platform-managed.
- **No Rust-side UI.** No GTK / Tauri / egui. The Rust workspace ships engine + phonetics + CLI only — platform frontends stay in Swift / Kotlin.
- **No web-target builds** (`wasm32-*`). Future consideration, not a current deliverable.
- **No FFI-crossing types from `std::sync` beyond `Arc<Mutex<...>>`.** Channels, condvars, parking_lot stay Rust-internal.
- **No `Send`-ing `Rc<...>` / `RefCell<...>`.** Interior mutability across FFI is rejected — use `Mutex` if shared, plain ownership if not.

## 9. References

- `.claude/rules/rust-ffi-safety.md` — companion: FFI boundary discipline, domain↔proto boundary, `unsafe`, opaque-handle pattern, enforcement
- `.claude/rules/rust-migration-policy.md` — when to start a slice migration, design goals, no toggles, mirror deletion
- khiin-rs reference study (2026-04-22): lessons to adopt + avoid, captured in `references/khiin-rs/`.
- `.claude/rules/cross-platform-alignment.md` §4.1 — Rust shared-core non-goals.
- `.claude/rules/android-guidelines.md` §1 Kotlin→Rust shape preferences — mirror of the type-shape rules here.
- `.claude/rules/ios-shared-core-candidates.md` — the iOS-side equivalent of what counts as a candidate for Rust extraction.
- `docs/architecture/behavioral-invariants.md` — invariant contracts the Rust implementation must preserve.
- Rust API Guidelines (https://rust-lang.github.io/api-guidelines/) — adopted as the naming + docs baseline.
- Rustonomicon (https://doc.rust-lang.org/nomicon/) — authoritative `unsafe` reference.
