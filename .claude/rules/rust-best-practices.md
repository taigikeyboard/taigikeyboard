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

Current runtime crates — dependency edges flow **one way, top → bottom** (the offline `build-helpers/fst-builder` producer sits outside this runtime graph):

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
│  ranking   → (protos only)                                   │
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
- **`anyhow` is forbidden in every workspace library crate**. Allowed in build scripts only.
- **`Result<T, EngineError>` throughout internal APIs.** Encode into `Response.ErrorCode` only at the FFI edge (see `.claude/rules/rust-ffi-safety.md` § FFI boundary discipline).
- **No `panic!` / `unwrap()` / `expect()` on unvalidated input.** `unwrap()` on a `Mutex::lock()` result is acceptable (poison is a programmer error, not a data path); briefly explain with `// JUSTIFICATION:` when non-obvious. `SAFETY:` comments are reserved for `unsafe` blocks per `.claude/rules/rust-ffi-safety.md` §3 — a safe `Mutex::lock().unwrap()` does not take one.
- **`?` is allowed and idiomatic inside the `catch_unwind` closure** (which returns `Result<Vec<u8>, EngineError>`). What is banned is propagating a `Result` out of the FFI function itself — the outer `extern fn` must return protobuf bytes or a null sentinel, never a Rust `Result` or `Option`. Encode errors into `Response.ErrorCode` at the seam between closure and extern fn.

## 3. Crate + type choices `[R]` `[A]`

Pinned choices (deviations require written justification):

| Concern | Choice | Rationale |
|---|---|---|
| Protobuf codegen | **`prost` + `prost-build`** | Modern, zero-copy-friendly, integrates with tonic. Rejects older `rust-protobuf` that khiin-rs uses. |
| SQLite driver | **`rusqlite` with `bundled` feature** | Cross-platform consistent; matches khiin-rs (validated). |
| Trie / prefix lookup | **`fst` crate** | Pure-Rust, mmap-friendly (iOS extension memory budget), replaces JNI-bound MARISA. |
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

- **Root `Makefile`** orchestrates (`make build` / `dict` / `fmt` / `lint`); the per-platform native builds are `engine/scripts/build-*.sh`. Contributor steps: `docs/BUILDING.md`.
- **Android**: `cargo-ndk` for multi-ABI builds (ships `arm64-v8a` + `armeabi-v7a`; add another ABI to `engine/rust-toolchain.toml` only when it ships).
- **iOS / macOS**: `cargo build --target aarch64-apple-ios` + simulator targets; packaged as xcframework via `swift-bridge` generator.
- **Rustup targets** pinned in `engine/rust-toolchain.toml` (shipped ABIs only; reasons in its comments).

## 5. Testing strategy `[R]` `[A]`

- **Unit tests**: `#[cfg(test)] mod tests` alongside source files.
- **Integration tests**: `engine/<crate>/tests/` exercise each crate through its public API with protobuf messages, no FFI.
- **Invariant tests**: every `INVARIANT_*` label from `docs/architecture/behavioral-invariants.md` has a matching Rust test. Drift between platform tests and Rust tests = regression.
- **`cargo test --workspace`** must pass in CI before any PR merges.
- **Property tests** (`proptest`) for phonetics round-trip invariants (TL↔POJ↔TPS) — complements hand-written invariant tests.

## 6. Rust version policy `[A]`

- **Stable channel only.** No nightly features, no `#![feature(...)]`.
- **MSRV 1.86** (`rust-version` in root `Cargo.toml`); bumping is a PR-level decision with CI verification.
- **`rustfmt` default config**, no deviations. Apply with `make fmt`; check without writing via `cd engine && cargo fmt --all -- --check` (CI gates it per §7).
- **`clippy` with `-D warnings`** — CI gates it (`engine.yml`, engine + desktop workspaces); run locally with `make lint` (clippy + Kotlin spotlessCheck). Project-wide allow list lives in workspace `Cargo.toml` `[workspace.lints]`.

## 7. CI gate + supply chain `[A]`

CI (`.github/workflows/engine.yml`) runs `cargo test --workspace`, `cargo fmt --check` and `cargo clippy -D warnings` on every PR touching `engine/`; `.github/workflows/security.yml` runs `cargo-audit` over all four Cargo workspaces + `cargo-deny` over the engine on PRs touching Cargo manifests / lockfiles and weekly. Post-PR verification follows CLAUDE.md § Build & Test.

- `make`-target shortcuts available for round-internal iteration (fast paths) AND canonical form (full paths). See root `Makefile help` for the current target list.
- **`cargo-audit`** scans against the RustSec advisory DB (CI `security.yml`; locally, install via `cargo install cargo-audit --locked`).
- **`cargo-deny check`** enforces dependency policy via `engine/deny.toml`: license allow-list (MIT / Apache-2.0 / BSD / ISC / Unicode-DFS-2016 / Unicode-3.0 / Zlib), `multiple-versions = warn`, `unknown-git = deny`, `unknown-registry = deny`. Runs in CI `security.yml`; locally, install via `cargo install cargo-deny --locked`.
- **FFI integration tests** are run on representative Android emulator + iOS simulator targets when relevant to the round (D9 gate onward) — user-gated, no CI matrix, no fixed schedule.
- **Supply chain**: no git dependencies in `Cargo.toml`. Patches go through explicit `[patch.crates-io]` with version pins and written justification.

## 8. Explicit non-goals

Codifying `.claude/rules/cross-platform-alignment.md` §4.1 in Rust terms:

- **No async runtime** (`tokio`, `async-std`, `smol`). Engine is synchronous. Platform wrappers handle threading.
- **No stray global state.** Process-wide engine state lives only in each stateful domain crate's `handle.rs` singleton behind a `Mutex` / `RwLock`. Immutable lookup tables may use `once_cell::sync::Lazy`. No other mutable globals.
- **No Rust-side UI in `engine/`.** No GTK / Tauri / egui. The `engine/` workspace ships engine + phonetics + CLI only. The Windows (`windows/`: TSF DLL + WinUI 3 settings) and Linux (`linux/`: IBus engine + GTK settings in Rust, Fcitx5 addon in C++ over `taigi-linux-ffi`) frontends live in separate Cargo workspaces governed by `windows-guidelines.md` / `linux-guidelines.md`; iOS / Android / macOS frontends stay in Swift / Kotlin.
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
