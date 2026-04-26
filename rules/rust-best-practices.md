# Rust Best Practices

Mandatory rules for Rust shared-core development. Read before authoring `docs/engine/ffi-safety.md`, `docs/engine/rust-core-proto.md`, the Phase III D9 FFI POC, or any Rust code landing in Phase IV-A and beyond.

**Active window**: Phase IV-A onward, when the first Rust crate lands. Pre-commitment scope for Phase II.5 and Phase III — authoring of FFI safety spec, proto contract draft, and the D9 POC must comply.

**Three goals** every rule below serves at least one of:

- **R — Rust-idiomatic**: follows community conventions; catches bugs at compile time
- **S — Safety**: no undefined behavior, no unwind across FFI, no data races
- **A — Anti-regression**: predictable builds, reproducible CI, supply-chain hygiene

Each rule is tagged with one or more of `[R]`, `[S]`, `[A]`.

## 1. Workspace layout `[R]` `[A]`

Cargo workspace with one crate per concern. Models khiin-rs (`references/khiin-rs/Cargo.toml`) with deliberate deviations:

```
taigi-keyboard-rs/
├── phonetics/   # Pure functions — POJ/TL/TPS, Unicode, tone. No I/O, no std beyond core+alloc.
├── engine/      # Stateful engine — composing state, candidate ranking, next-word.
├── protos/      # Protobuf definitions (generated via prost-build).
├── android-jni/ # cdylib — JNI entry points + protobuf marshaling.
├── swift-ffi/   # staticlib — swift-bridge entry points + protobuf marshaling.
├── cli/         # Developer TUI (optional, matches khiin-rs `cli/`).
└── Cargo.toml         # Workspace manifest, pinned workspace.dependencies.
```

- **`phonetics/`** corresponds to `knowledge/` reference content + `taigi-converter/` behavior. Zero platform dependencies — portable to any Rust target.
- **`engine/`** owns `BufferMgr`-equivalent state machines, candidate scoring, SQLite access. Depends on `phonetics` + `protos`.
- **FFI crates** (`android-jni`, `swift-ffi`) are **thin** — protobuf in / protobuf out / `catch_unwind`. No domain logic. Each crate's `lib.rs` should be < 300 LOC.
- Depend via `workspace.dependencies` in root `Cargo.toml` with pinned versions. Workspace-internal deps use relative paths (`phonetics = { path = "./phonetics" }`).

## 2. FFI boundary discipline `[S]`

Policy lives here; technical spec is `docs/engine/ffi-safety.md` (Phase II.5 deliverable). Enforcement:

- **`std::panic::catch_unwind` wraps every FFI function body.** A panic → encoded `EngineError::InternalPanic(msg)` → protobuf `Response.error_code` → platform recovers. Unwind across FFI is undefined behavior in both JNI and C ABI.
- **Engine is guarded by `Mutex<Engine>`**. Engine state is `Send + !Sync`; the mutex serializes concurrent native calls. `Arc<Mutex<Engine>>` only if the handle is shared across platform threads (usually not needed — one engine per IME session).
- **Drop discipline**: every opaque handle exposes an explicit `shutdown(handle)` FFI. Rust side implements `Drop` with the same teardown path. Platform side (Kotlin `use {}` / Swift `deinit`) must call `shutdown`. Two khiin-rs failure modes this rule blocks: (a) Kotlin `EngineManager.kt:48` declares `external fun shutdown(enginePtr: Long)` with no matching Rust `extern fn` in `android/rust/src/lib.rs` — Kotlin link succeeds but runtime call panics; (b) `swift/bridge/src/lib.rs:33-35` defines `EngineBridge { engine_ptr: *mut c_void }` with zero `Drop` impl anywhere in the file — the boxed `Engine` leaks on app teardown.
- **Error sentinels travel in protobuf**: every FFI return is either a valid protobuf byte buffer carrying `Response.ErrorCode`, or an out-of-band failure (null bytes / negative length) that means "engine is sick, restart this IME session". Never leak Rust error types across the ABI.
- **Logging bridge**: the `log` crate is the only logging API candidate code sees. Platform adapter (`OSLog` on iOS, `android.util.Log` on Android) is registered once at engine init via `log::set_logger`. Candidate code never imports `OSLog`, `android.util.Log`, or any platform log API.

## 3. Error handling `[R]` `[S]`

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
- **`anyhow` is forbidden in library crates** (`phonetics`, `engine`, `protos`). Allowed in `cli/` and build scripts only.
- **`Result<T, EngineError>` throughout internal APIs.** Encode into `Response.ErrorCode` only at the FFI edge.
- **No `panic!` / `unwrap()` / `expect()` on unvalidated input.** `unwrap()` on a `Mutex::lock()` result is acceptable (poison is a programmer error, not a data path); briefly explain with `// JUSTIFICATION:` when non-obvious. `SAFETY:` comments are reserved for `unsafe` blocks per §4 — a safe `Mutex::lock().unwrap()` does not take one.
- **`?` is allowed and idiomatic inside the `catch_unwind` closure** (which returns `Result<Vec<u8>, EngineError>`). What is banned is propagating a `Result` out of the FFI function itself — the outer `extern fn` must return protobuf bytes or a null sentinel, never a Rust `Result` or `Option`. Encode errors into `Response.ErrorCode` at the seam between closure and extern fn.

## 4. `unsafe` discipline `[S]`

- **Every `unsafe` block carries a `// SAFETY:` comment** explaining the invariant that makes the operation sound. The khiin-rs unsafe deref at `references/khiin-rs/swift/bridge/src/lib.rs:52` has no SAFETY note — this pattern is rejected at review.
- **`unsafe` blocks are confined to FFI marshaling.** No domain logic inside `unsafe`. Target: `unsafe` block contents ≤ 3 lines.
- **No `transmute` unless absolutely required** — prefer `as` casts, `From`/`Into`, or `#[repr(C)]` layout-compatible structs.
- **No raw pointer dereferences outside FFI crates.** `phonetics` and `engine` are `#![forbid(unsafe_code)]` at the crate root; only `android-jni` and `swift-ffi` may contain `unsafe`.
- **Every new `unsafe` block requires Codex pre-implementation review** per `rules/cross-platform-alignment.md` §1c.

## 5. Crate + type choices `[R]` `[A]`

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

- `#[repr(transparent)]` newtype for opaque handles (not raw `*mut c_void` / `jlong`):
  ```rust
  #[repr(transparent)]
  pub struct EngineHandle(*mut Engine);
  ```
- **Explicit numeric widths**: `i64` / `f64` at the boundary (not `isize` / `usize`). Mirrors `rules/android-guidelines.md` §1 Kotlin→Rust shape rules.
- **UTF-8 strings only** — protobuf `string` already enforces; never use `&[u8]` for text data.

## 6. Cross-compile + build tooling `[A]`

- **`cargo-make` + `Makefile.toml`** as the build orchestrator. Matches khiin-rs pattern (validated across 4 platforms). Tasks at minimum: `build-db`, `build-droid`, `build-swift`, `build-cli`.
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

## 7. Testing strategy `[R]` `[A]`

- **Unit tests**: `#[cfg(test)] mod tests` alongside source files. Pure logic (`phonetics`, `engine`) targets ≥ 80% line coverage.
- **Integration tests**: `engine/tests/` exercises the engine through its public API with protobuf messages, no FFI.
- **FFI roundtrip tests**: `android-jni/tests/` + `swift-ffi/tests/` exercise FFI entry points for panic safety, Drop, thread serialization, malformed-bytes handling. Required for D9 POC acceptance.
- **Invariant tests**: every `INVARIANT_*` label from `docs/architecture/behavioral-invariants.md` has a matching Rust test. Drift between platform tests and Rust tests = regression.
- **`cargo test --workspace`** must pass in CI before any PR merges.
- **Property tests** (`proptest`) for phonetics round-trip invariants (TL↔POJ↔TPS) — complements hand-written invariant tests.

## 8. Rust version policy `[A]`

- **Stable channel only.** No nightly features, no `#![feature(...)]`.
- **MSRV pinned at Rust 1.86** in root `Cargo.toml` (`rust-version = "1.86"`). Rationale: 1.85 (Feb 2025) stabilises edition2024 and 1.86 (Apr 2025) is the next stable. Bumped during Phase III D9.2 because `cargo-ndk` 4.x requires 1.86 and the dev-tool gap is not worth carrying a 3.5.x sidegrade for. Earlier pins (1.75 → 1.85 in D9.1) similarly bumped to clear active-tooling gaps. Bumping MSRV further remains a PR-level decision with CI verification.
- **No experimental features** (`async fn` in traits — stable since 1.75 — OK; GATs in traits OK; const generics full — OK; edition2024 — OK on 1.85+).
- **`rustfmt` default config**, no deviations. `cargo fmt --check` in CI.
- **`clippy` with `-D warnings`** in CI. Project-wide allow list lives in workspace `Cargo.toml` `[workspace.lints]`.

## 9. Pre-commit gate + supply chain `[A]`

This project runs the Rust gate **locally**, not via GitHub Actions. Mirrors `feedback_manual_build_test.md` for `./gradlew` / `xcodebuild`: the author runs build/test, AI does not. The author invokes the four-command gate before committing each PR (canonical bare-`cargo` form; `cargo-make` is optional via `engine/Makefile.toml`).

- Per-PR gate: `cargo fmt --all -- --check` + `cargo check --workspace --locked` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace`.
- **`cargo-audit`** scans against the RustSec advisory DB before each PR — optional, install via `cargo install cargo-audit --locked`.
- **`cargo-deny check`** enforces dependency policy via `engine/deny.toml`: license allow-list (MIT / Apache-2.0 / BSD / ISC / Unicode-DFS-2016 / Unicode-3.0 / Zlib), `multiple-versions = warn`, `unknown-git = deny`, `unknown-registry = deny`. Optional, install via `cargo install cargo-deny --locked`.
- **FFI integration tests** must pass on a representative Android emulator + iOS simulator target before merge to main (D9 gate onward) — author runs locally; no CI matrix.
- **Supply chain**: no git dependencies in `Cargo.toml`. Patches go through explicit `[patch.crates-io]` with version pins and written justification.

## 10. Opaque handle pattern `[S]` `[R]`

For engines owned on one side of the FFI boundary:

```rust
#[repr(transparent)]
pub struct EngineHandle(*mut Engine);

#[no_mangle]
pub extern "C" fn engine_new(db_path: *const c_char) -> EngineHandle {
    let result = std::panic::catch_unwind(|| {
        // SAFETY: db_path is a C string from the platform caller; validated non-null upstream.
        let path = unsafe { std::ffi::CStr::from_ptr(db_path) }.to_string_lossy();
        Box::into_raw(Box::new(Engine::new(&path)?))
    });
    EngineHandle(result.unwrap_or(std::ptr::null_mut()))
}

#[no_mangle]
pub extern "C" fn engine_shutdown(handle: EngineHandle) {
    let _ = std::panic::catch_unwind(|| {
        if !handle.0.is_null() {
            // SAFETY: handle came from engine_new; platform contract is single-shutdown.
            unsafe { drop(Box::from_raw(handle.0)); }
        }
    });
}
```

Both extern fns wrap their bodies in `catch_unwind` per §2. Every `unsafe` block carries a `// SAFETY:` comment per §4. The handle is `#[repr(transparent)]` so the ABI matches `*mut Engine` exactly.

- **Kotlin side**: wrap `jlong` in `@JvmInline value class EngineHandle(val raw: Long)` — type-safe, zero runtime cost.
- **Swift side**: swift-bridge generates the wrapper; platform holds it via ARC.
- **Never expose the raw pointer to platform code.** The handle is opaque.

## 11. Explicit non-goals

Codifying `rules/cross-platform-alignment.md` §5.1 in Rust terms:

- **No async runtime** (`tokio`, `async-std`, `smol`). Engine is synchronous. Platform wrappers handle threading.
- **No global state in Rust.** No `lazy_static!` / `once_cell::sync::Lazy` in engine or phonetics crates. Engine lifetime is platform-managed.
- **No Rust-side UI.** No GTK / Tauri / egui. The Rust workspace ships engine + phonetics + CLI only — platform frontends stay in Swift / Kotlin.
- **No web-target builds** (`wasm32-*`) in Phase IV-A. Future consideration, not a current deliverable.
- **No FFI-crossing types from `std::sync` beyond `Arc<Mutex<...>>`.** Channels, condvars, parking_lot stay Rust-internal.
- **No `Send`-ing `Rc<...>` / `RefCell<...>`.** Interior mutability across FFI is rejected — use `Mutex` if shared, plain ownership if not.

## 12. Enforcement hooks `[A]`

- **Phase II.5 entry**: `docs/engine/ffi-safety.md` and `docs/engine/rust-core-proto.md` drafts must cite this rules file. Rule deviations in those docs require `// JUSTIFICATION:` prose in-line.
- **Phase III D9 POC**: POC code is reviewed against every rule above. Deviations land only after Codex + `/simplify` pre-review and Codex post-review on the diff, with written rationale.
- **Phase IV-A onward**: every Rust PR runs through the §1c constraint (for shared-core-candidate equivalence), Codex + `/simplify` pre-implementation review on the plan, Codex post-edit review on the diff, plus this file's §§2–10 enforcement. `/simplify` is the Claude Code official skill and catches reuse / quality / dead-code issues Codex does not flag; run both in parallel per `rules/claude-workflow.md` §Subagent Usage.

## 13. References

- khiin-rs reference study (2026-04-22): lessons to adopt + avoid, captured in plan `/Users/alexsu/.claude/plans/cozy-dancing-nova.md` and `references/khiin-rs/`.
- `rules/cross-platform-alignment.md` §4a — Phase II.5 prerequisite docs.
- `rules/cross-platform-alignment.md` §5.1 — Rust shared-core non-goals.
- `rules/android-guidelines.md` §1 Kotlin→Rust shape preferences — mirror of the type-shape rules here.
- `rules/ios-architecture.md` §4 shared-core criteria — the iOS-side equivalent of what counts as a candidate for Rust extraction.
- `docs/architecture/behavioral-invariants.md` — invariant contracts the Rust implementation must preserve.
- Rust API Guidelines (https://rust-lang.github.io/api-guidelines/) — adopted as the naming + docs baseline.
- Rustonomicon (https://doc.rust-lang.org/nomicon/) — authoritative `unsafe` reference.
