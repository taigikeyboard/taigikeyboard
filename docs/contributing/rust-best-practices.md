# Rust Best Practices

Rules for Rust shared-core development. Read before any Rust code lands in the engine. FFI / proto-boundary / `unsafe` / opaque-handle / enforcement rules are split into `docs/contributing/rust-ffi-safety.md`.

**Three goals** every rule below serves at least one of:

- **R — Rust-idiomatic**: follows community conventions; catches bugs at compile time
- **S — Safety**: no undefined behavior, no unwind across FFI, no data races
- **A — Anti-regression**: predictable builds, reproducible CI, supply-chain hygiene

Each rule is tagged with one or more of `[R]`, `[S]`, `[A]`.

## 1. Workspace layout `[R]` `[A]`

Cargo workspace with one crate per concern. Models khiin-rs (`references/khiin-rs/Cargo.toml`) with deliberate deviations. Crate list and dependency graph: `docs/architecture/system-overview.md` §2; layering rule: §1a below.

- **`phonetics`** — pure POJ/TL/TPS / Unicode / tone, no I/O, zero platform dependencies, portable to any Rust target. Corresponds to `docs/phonetics/` reference content + `taigi-converter/` behavior.
- **Domain crates** (`composing`, `lexicon`, `ranking`, `nextword`, `userdata`) own the state machines, candidate scoring, next-word prediction and the user's stores.
- **FFI crates** (`android-jni`, `swift-ffi`) are **thin** — protobuf in / protobuf out / `catch_unwind`. No domain logic. Each crate's `lib.rs` should be < 300 LOC.
- Depend via `workspace.dependencies` in root `Cargo.toml` with pinned versions. Workspace-internal deps use relative paths (`phonetics = { path = "./phonetics" }`).

## 1a. Crate layering & dependency direction `[R]` `[A]`

Runtime crates sit in four layers, and dependency edges flow **one way, top → bottom**: **adapters** (`swift-ffi`, `android-jni`, and outside this workspace `taigi-desktop-core`; thin — bytes in / out, `catch_unwind`) → **use-case** (`dispatch`, the only crate that sees all domains) → **domain** (`composing`, `lexicon`, `ranking`, `nextword`, `userdata`) → **leaf / shared kernel** (`phonetics`, `protos`, `mmap-host`). The offline `build-helpers/fst-builder` sits outside this runtime graph. The drawn graph is canonical in `docs/architecture/system-overview.md` §2.

**Dependency-direction invariant** — a crate may depend only on crates in its own layer or below:

- **Forbidden upward edges**: no domain crate (`phonetics` / `ranking` / `lexicon` / `nextword` / `composing` / `userdata`) may depend on `dispatch` or an FFI crate; the leaf layer (`phonetics` / `protos` / `mmap-host`) may depend on nothing above itself.
- **`dispatch` is the only orchestrator** — the single crate allowed to reference every domain. FFI crates (`swift-ffi` / `android-jni`) see only `dispatch` + `protos`. Outside this workspace, `taigi-desktop-core` sees only `dispatch` + `protos` too.
- **Cargo enforces acyclicity at build time** (a cycle fails to compile) — that is the hard backstop. This layering rule is the *soft* guide that stops the graph degrading into flat all-depends-on-all while still technically acyclic.
- **New crate / new edge**: place it so the arrow still points down. If a domain crate appears to need something currently in `dispatch`, that is an inversion — push the shared piece **down** into `phonetics` / `protos`, never add an upward edge (mirrors `AGENTS.md` § Design principles § No redundant fallback — keep data flow one-direction).

## 2. Error handling `[R]` `[S]`

- **`thiserror` for library errors**. Each domain crate owns its error enum deriving `thiserror::Error` (e.g. `lexicon::LexiconError` in `lexicon/src/error.rs`, `UserDataDatabaseError` in `userdata/src/database.rs`); there is no workspace-wide `EngineError` type.
- **`anyhow` is forbidden in every workspace library crate**. Allowed in build scripts only.
- **`Result<T, <CrateError>>` throughout internal APIs.** `dispatch` maps each crate error to the wire `ErrorCode` (`envelope.proto` `Response.error`) — e.g. `lexicon_error_code` in `dispatch/src/lib.rs` — and the FFI seams build error-only responses with `dispatch::encode_error` (see `docs/contributing/rust-ffi-safety.md` § FFI boundary discipline).
- **No `panic!` / `unwrap()` / `expect()` on unvalidated input.** `unwrap()` on a `Mutex::lock()` result is acceptable (poison is a programmer error, not a data path); briefly explain with `// JUSTIFICATION:` when non-obvious. `SAFETY:` comments are reserved for `unsafe` blocks per `docs/contributing/rust-ffi-safety.md` §3 — a safe `Mutex::lock().unwrap()` does not take one.
- **`?` is allowed and idiomatic inside `dispatch`'s handlers**, which turn every `Err` into an error `Response`. What is banned is propagating a `Result` out of the FFI function itself — the outer `extern fn` must return protobuf bytes or a null sentinel, never a Rust `Result` or `Option`. The seam's `catch_unwind` maps a panic to `encode_error(ErrorCode::FailInternal)` (`swift-ffi/src/lib.rs::process_request_bytes`).

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
| Async runtime | **None — sync only** | IME latency budget (< 50ms keystroke roundtrip) does not require async. Rejects `tokio` / `async-std` complexity. |

Type-shape preferences that cross FFI:

- `#[repr(transparent)]` newtype for opaque handles (see `docs/contributing/rust-ffi-safety.md` § Opaque handle pattern).
- **Explicit numeric widths**: `i64` / `f64` at the boundary (not `isize` / `usize`).
- **UTF-8 strings only** — protobuf `string` already enforces; never use `&[u8]` for text data.

## 4. Cross-compile + build tooling `[A]`

- **Root `Makefile`** orchestrates (`make build` / `dict` / `fmt` / `lint`); the per-platform native builds are `engine/scripts/build-*.sh`. Contributor steps: `docs/BUILDING.md`.
- **Android**: `cargo-ndk` for multi-ABI builds (ships `arm64-v8a` + `armeabi-v7a`; add another ABI to `engine/rust-toolchain.toml` only when it ships).
- **iOS**: `engine/scripts/build-xcframework.sh` (`aarch64-apple-ios` + simulator targets, xcframework via the `swift-bridge` generator). **macOS**: `engine/scripts/build-macos-xcframework.sh` builds the `macos/` workspace archive (`taigi-macos-ffi`; targets in `macos/rust-toolchain.toml`).
- **Rustup targets** pinned in `engine/rust-toolchain.toml` (shipped ABIs only; reasons in its comments).

## 5. Testing strategy `[R]` `[A]`

- **Unit tests**: `#[cfg(test)] mod tests` alongside source files.
- **Integration tests**: `engine/<crate>/tests/` exercise each crate through its public API with protobuf messages, no FFI.
- **One test binary per concern, not per file** (`composing` / `lexicon` / `phonetics` / `userdata`): `autotests = false` + explicit `[[test]]` roots (`tests/it.rs`, `tests/prod.rs`, composing `tests/no_lexicon.rs`) that declare each `tests/<file>.rs` as `mod <file>;` and `mod common;` once (`use crate::common::…` in the file). A new test file joins a root by process-wide state: hermetic fixtures → `it` (install under `test_support::engine_install_lock`), production lexicon / shipped CSVs → `prod`, lexicon never installed → `no_lexicon`. The `it` guard test fails on a file no root declares. Filter by module: `cargo test -p composing --test prod candidate_dump -- --ignored --nocapture`. `dispatch` keeps one binary per file (process-isolated user-data handle, logger, trace sink).
- **Crate-neutral test helpers live in `engine/test-support`** (dev-dependency; temp files, TKDB / TKWA / FST serializers, install lock, production artifacts, the once-per-process `dictionary.csv` loader). It depends on no engine crate, so `lexicon`'s own tests can use it; helpers that need an engine type stay in that crate's `tests/common/mod.rs`.
- **Invariant tests**: every `INVARIANT_*` label for behaviour a Rust slice owns has a matching test in `engine/<crate>/tests/`; platform-only labels are pinned by platform tests (`docs/architecture/behavioral-invariants.md` § Test references). Drift between platform tests and Rust tests = regression.
- **`cargo test --workspace`** must pass in CI before any PR merges.
- **Property tests** (`proptest`) for phonetics round-trip invariants (TL↔POJ↔TPS) — complements hand-written invariant tests.

## 6. Rust version policy `[A]`

- **Stable channel only.** No nightly features, no `#![feature(...)]`.
- **MSRV per workspace** — engine 1.89, desktop + linux + macos + web 1.93, windows 1.95 (`rust-version` in each workspace `Cargo.toml`; why and which CI job checks each: `docs/architecture/build-artifacts.md` § Six Cargo workspaces). Bumping is a PR-level decision with CI verification.
- **`rustfmt` default config**, no deviations. Apply with `make fmt`; check without writing via `cd engine && cargo fmt --all -- --check` (CI gates it per §7).
- **`clippy` with `-D warnings`** — CI gates it (`engine.yml` engine workspace, `desktop.yml` desktop workspace); run locally with `make lint` (rustfmt check on all six workspaces, clippy on engine / desktop / macOS / web, biome, SwiftFormat lint, Kotlin spotlessCheck). Project-wide allow list lives in workspace `Cargo.toml` `[workspace.lints]`.

## 7. CI gate + supply chain `[A]`

CI per workspace (`.github/workflows/`): `engine.yml` — `cargo test --workspace`, `cargo fmt --check`, `cargo clippy -D warnings`; `desktop.yml` — desktop-shared tests + clippy, desktop + Windows fmt; `macos.yml` — `macos/` tests, clippy, fmt, archive build, `make -C macos test`; `linux-build.yml` — nightly, on release and on dispatch, never per PR: `linux/` clippy, tests, fmt, release build; `web.yml` — `make web-check`; `security.yml` — `cargo-audit` over all six workspaces + `cargo-deny` over engine and macOS (`engine/deny.toml`), also weekly; `security-windows.yml` — `cargo-deny` over Windows (`windows/deny.toml`), also weekly. Each workflow's `on.pull_request.paths` is the only record of what triggers it. Post-PR verification follows AGENTS.md § Build & Test.

- `make help` lists the root targets.
- **`cargo-audit`** scans against the RustSec advisory DB (CI `security.yml`; locally, install via `cargo install cargo-audit --locked`).
- **`cargo-deny check`** enforces dependency policy via `engine/deny.toml`: license allow-list (MIT / Apache-2.0 / BSD / ISC / Unicode-DFS-2016 / Unicode-3.0 / Zlib), `multiple-versions = warn`, `unknown-git = deny`, `unknown-registry = deny`. Runs in CI `security.yml`; locally, install via `cargo install cargo-deny --locked`.
- **FFI integration tests** are run on representative Android emulator + iOS simulator targets when relevant to the round — user-gated, no CI matrix, no fixed schedule.
- **Supply chain**: no git dependencies in the engine workspace (`engine/deny.toml` `unknown-git = deny`); the one pinned git dependency elsewhere is `windows-reactor` (`windows-guidelines.md`). Patches go through explicit `[patch.crates-io]` with version pins and written justification.

## 8. Explicit non-goals

What the engine never owns on the platform side: `docs/contributing/cross-platform-alignment.md` §4.1. Rust-side non-goals (no async runtime: §3):

- **No stray global state.** Process-wide engine state lives only in each stateful domain crate's `handle.rs` singleton behind a `Mutex` / `RwLock`. Immutable lookup tables may use `once_cell::sync::Lazy`. One install-time exception: the `wasm32` byte registry in `engine/mmap-host/src/registry.rs`, written before the lexicon installs, where native targets have the file system. No other mutable globals.
- **No Rust-side UI in `engine/`.** No GTK / Tauri / egui. The `engine/` workspace ships engine + phonetics + CLI only. The Windows (`windows/`: TSF DLL + WinUI 3 settings) and Linux (`linux/`: IBus engine + GTK settings in Rust, Fcitx5 addon in C++ over `taigi-linux-ffi`) frontends live in separate Cargo workspaces governed by `windows-guidelines.md` / `linux-guidelines.md`; the `macos/` workspace builds the one archive the macOS app links (`taigi-macos-ffi`: engine seam + `taigi-desktop-core`, governed by `rust-ffi-safety.md`); the `web/` workspace builds the browser module (`taigi-web`, governed by `web-guidelines.md`). iOS / Android UI and the macOS UI stay in Swift / Kotlin.
- **No FFI-crossing types from `std::sync` beyond `Arc<Mutex<...>>`.** Channels, condvars, parking_lot stay Rust-internal.
- **No `Send`-ing `Rc<...>` / `RefCell<...>`.** Interior mutability across FFI is rejected — use `Mutex` if shared, plain ownership if not.

## 9. References

- `docs/contributing/rust-ffi-safety.md` — companion: FFI boundary discipline, domain↔proto boundary, `unsafe`, opaque-handle pattern, enforcement
- `docs/contributing/rust-migration-policy.md` — when to start a slice migration, design goals, no toggles, mirror deletion
- khiin-rs reference study: `docs/references/khiin-reference.md` (clone under `references/khiin-rs/`).
- `docs/contributing/cross-platform-alignment.md` §4.1 — Rust shared-core non-goals.
- `docs/contributing/cross-platform-alignment.md` §1c — what counts as a shared-core candidate on iOS and Android.
- `docs/architecture/behavioral-invariants.md` — invariant contracts the Rust implementation must preserve.
- Rust API Guidelines (https://rust-lang.github.io/api-guidelines/) — adopted as the naming + docs baseline.
- Rustonomicon (https://doc.rust-lang.org/nomicon/) — authoritative `unsafe` reference.
