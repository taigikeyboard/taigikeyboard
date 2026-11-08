# Rust FFI Safety

Rules for the Rust ↔ platform boundary: the seam as built, panic / thread / error / logging discipline, domain↔proto layering, `unsafe` discipline, the opaque-handle pattern, enforcement hooks and the test contract. General Rust hygiene (workspace, errors, crates, tests, versions) stays in `docs/contributing/rust-best-practices.md`.

**Active window**: every Rust PR touching `engine/swift-ffi/`, `engine/android-jni/`, `engine/dispatch`, `engine/protos/`, any domain crate's RPC façade, `linux/crates/taigi-linux-ffi/`, `macos/crates/taigi-macos-ffi/`, `web/crates/taigi-web/` or a `windows/crates/taigi-windows-tsf/` COM entry.

## 1. FFI boundary discipline `[S]`

### 1.1 The seam as built

- **Engine adapters (iOS, macOS, Android)** — a process-singleton, bytes-in / bytes-out seam. The entry takes only `bytes: &[u8]` and reaches the engine through `EngineHandle::instance()` (`engine/composing/src/handle.rs`); no handle crosses the boundary and there is no shutdown call — the singleton owns its lifetime, and the user-data stores are process-wide in `userdata` (`UserDataHandle`, opened once by `OpenUserData`). Live entry points: `process_request_bytes` / `install_logger_sink` / `set_log_level` / `panic_for_test` / `e2e_trace_open` (`engine/swift-ffi`) and `processRequestBytes` / `registerLogger` / `setLogLevel` / `panicForTest` / `e2eTraceOpen` (`engine/android-jni`). Both call `dispatch::process_request` directly.
- **macOS desktop shell** — a second bytes seam in the same archive: `desktop_request_bytes` (`macos/crates/taigi-macos-ffi/src/lib.rs`), which links `engine/swift-ffi` as an rlib so macOS ships ONE static library and the engine's statics exist once (`docs/architecture/macos-desktop-core-roadmap.md` D1, D2). Envelope `macos/crates/taigi-macos-ffi/proto/desktop_shell.proto`; its `DesktopResponse.error` is the engine's `ErrorCode` (§1.5). Same rules as the engine adapters: no handle crosses, `catch_unwind`, the §1.4 cap, no `unsafe`.
- **Linux Fcitx5 addon** — a C ABI over `taigi-linux-core` (`linux/crates/taigi-linux-ffi`, contract `include/taigikeyboard.h`): opaque handles (`TaigiRuntime`, `TaigiEngine`, `TaigiReply`, `TaigiMenu`), each with a matching `*_free`, accessors instead of shared structs. This is the §4 pattern.
- **Windows TSF DLL** — every COM entry runs through `windows/crates/taigi-windows-tsf/src/com_guard.rs` (`guarded` / `guarded_hresult`), which turns a panic into `E_FAIL` plus one log line.

### 1.2 Panic discipline — `catch_unwind` mandatory

Every exported function (`#[no_mangle]` / `extern "system"` / `#[swift_bridge::bridge]` method / C ABI entry / COM method) wraps its body in `std::panic::catch_unwind`. Unwinding across a language boundary is undefined behavior on JNI, the C ABI and COM.

- Engine adapters: a caught panic → `dispatch::encode_error(ErrorCode::FailInternal)` → protobuf `Response.error`. There is no `EngineError` type.
- Linux C ABI: a panic answers the null / false / zero the header documents and is logged. Windows COM: `E_FAIL` and a log line.
- `?` is fine inside the closure; the outer `extern fn` never returns a Rust `Result` or `Option`.
- Unwinding stays on in every crate that hosts a boundary (`panic = "unwind"` in `linux/Cargo.toml`; `windows/Cargo.toml` explains why `abort` would take the host process down).
- Named divergence: `wasm32-unknown-unknown` has no unwinding, so `catch_unwind` catches nothing in `taigi-web`. A panic traps the instance; the page starts a new one and loses the composition, never the text already in the editor (`docs/architecture/web-roadmap.md` § What must change).
- Failure mode blocked: khiin-rs parses JNI request bytes with `.expect(...)` (`references/khiin-rs/android/rust/src/lib.rs:51-56`), so a malformed payload aborts the whole IME process.

### 1.3 Thread safety — `Mutex<Engine>`, never raw pointer + `&mut`

Engine state is `Send + !Sync`; the platform may call concurrently (IME thread, settings push, background refresh). The singleton holds `Mutex<Engine>` and locks it per request — platforms never receive a raw `*mut Engine`, and casting back via `&mut *(ptr as *mut Engine)` is forbidden. `RwLock` is not part of the contract. Failure mode blocked: khiin-rs casts the raw pointer back to `&mut Engine` with no synchronization (`references/khiin-rs/android/rust/src/lib.rs:64`, `references/khiin-rs/swift/bridge/src/lib.rs:51-52`), so two platform calls can alias one `&mut`.

### 1.4 Request size cap

`dispatch::MAX_REQUEST_BYTES` (2 MB) is checked before the request is copied or decoded; over the cap answers `FAIL_INVARIANT`, exactly the cap is accepted.

### 1.5 Error channel — the protobuf envelope

Every FFI return is either (a) valid protobuf bytes carrying `Response.error: ErrorCode` — the platform parses the envelope first — or (b) an out-of-band failure (null / empty / negative length) meaning "engine is sick, restart this IME session". Rust error types never cross the ABI (`docs/contributing/rust-best-practices.md` §2); they convert to `ErrorCode` at the seam.

| Code | Name | Cause |
|---|---|---|
| 0 | `OK` | Success |
| 1 | `FAIL_PARSE` | Malformed bytes |
| 2 | `FAIL_INTERNAL` | Caught panic |
| 3 | `FAIL_IO` | DB / file error |
| 4 | `FAIL_INVARIANT` | Engine invariant violated or request over the cap — surfaced as data, logged, the engine serves the next request |

Source of truth: `engine/protos/proto/envelope.proto` `enum ErrorCode`.

### 1.6 Logging bridge — `log` crate only

- Rust core imports nothing beyond the `log` crate. Forbidden in engine code: `OSLog`, `os_log`, `android.util.Log`, `__android_log_print`, `println!`, `eprintln!`.
- Each adapter crate defines the `log::Log` implementation that forwards each record into the platform's `LoggerBackend` (`docs/architecture/behavioral-invariants.md` §12). The adapter lives in the adapter crate, never in a domain crate.
- `log::set_logger` succeeds once per process: registration sits behind a `Once` / `OnceLock` so a second IME session start never panics on `SetLoggerError`. The logger outlives every session.

## 2. Domain↔proto boundary rule `[R]` `[A]`

Every domain crate reached by the dispatcher (`engine/phonetics`, `engine/lexicon`, `engine/composing`, `engine/nextword`, `engine/userdata`, any future stateless slice) follows one surface pattern.

**Rule.** The **request / RPC façade** (`src/requests.rs`) of each domain crate (the function the dispatcher routes through — `phonetics::requests::handle`, `lexicon::requests::handle`, `composing::*`, `userdata::UserDataHandle::handle`) accepts and returns **protobuf-generated types** (`protos::engine::*`) directly. There is no parallel native-Rust mirror tier and no proto↔native translation layer between `engine/dispatch` and the domain crate. The protobuf schema is the cross-platform contract; duplicating it doubles maintenance with no consumer.

This rule binds the cross-platform RPC seam, not every public function. CLI helpers, test fixtures, and stable native-Rust convenience APIs (`phonetics::to_tone_marks`, `phonetics::to_tone_number`, `phonetics::normalize_to_tl`, `phonetics::strip_tone_mark`, etc.) may keep native signatures — they were intentionally exposed for in-process Rust callers (CLI, integration tests). What is forbidden is letting those native helpers grow into a **second proto-mirroring type tier** that the dispatcher routes through.

**Module visibility.** Implementation modules are `mod`-private. Only **named façade / entry modules** are `pub mod`, and they expose only the named entry points downstream actually call. Top-level `pub use` re-exports are reserved for stable cross-crate symbols (CLI helpers, integration-test helpers, the public `Error` enum) — never as a redundant alias for an entry already reachable through a façade module.

Concretely, `engine/phonetics/src/lib.rs` is the canonical shape:

```rust
pub mod api;        // tests + CLI hit phonetics::api::*
pub mod requests;   // engine/dispatch routes through phonetics::requests::handle

pub mod case_transform; // cross-crate case façade (dispatch/src/case.rs)
mod case_tables;
mod derivation;
mod normalization;
mod syllable;
// … all other implementation modules stay private
```

**Layering by crate.**

| Layer | Type vocabulary | Visibility |
|---|---|---|
| `phonetics`, `lexicon`, `composing`, `nextword`, `userdata` (domain) | **Request façade** (`requests.rs`) takes / returns `protos::engine::*` directly. Native-Rust helpers (CLI / test convenience functions) may exist alongside but never grow into a parallel mirror tier. | Implementation modules `mod`-private; one or two `pub mod` façades; `pub use` only for genuine cross-crate symbols. |
| `engine/dispatch` | Single `process_request(&[u8]) -> Vec<u8>`. Decodes once, routes by `Request.payload` variant to the matching domain crate, encodes once. | Pure routing — no proto↔proto translation, except cross-domain composition only `dispatch` can do (`predict.rs` expands nextword `PredictNext` with a lexicon lookup into `FilterPredictions`; `user_data/with_stores.rs` feeds the `userdata` stores' rows to `composing` / `nextword`). |
| `swift-ffi`, `android-jni` | Bytes in, bytes out across the FFI seam. `catch_unwind` per §1.2. | Calls `dispatch::process_request` directly. |

**What this rule excludes.** Native-Rust input/output structs that mirror proto messages, `From<NativeFoo> for protos::engine::Foo` impls, separate per-op entry points in dispatch (`dispatch::process_phonetics`, `dispatch::process_ranking`, …) — all banned. They show up in candidate refactors and they are always extra work for no end-user benefit.

**When this rule may be revisited.** If a future slice needs to expose a domain API that takes / returns Rust-native types because the public Rust crate has consumers outside the IME (e.g. someone embeds `phonetics` in a non-IME tool). Until that happens, this rule holds.

## 3. `unsafe` discipline `[S]`

- **Every `unsafe` block carries a `// SAFETY:` comment** explaining the invariant that makes the operation sound. The khiin-rs unsafe deref at `references/khiin-rs/swift/bridge/src/lib.rs:52` has no SAFETY note — this pattern is rejected at review.
- **`unsafe` blocks are confined to FFI marshaling.** No domain logic inside `unsafe`. Target: `unsafe` block contents ≤ 3 lines.
- **No `transmute` unless absolutely required** — prefer `as` casts, `From`/`Into`, or `#[repr(C)]` layout-compatible structs.
- **No raw pointer dereferences outside FFI crates.** Domain crates inherit the workspace `unsafe_code = "forbid"` lint. Only `android-jni`, `swift-ffi`, the documented `mmap-host` carve-out and the desktop crates whose `[lints.rust]` allows it under a `# JUSTIFICATION:` comment (`taigi-linux-ffi`, `taigi-windows-tsf`, `taigi-windows-platform`, `taigi-windows-update`) may contain `unsafe`.

## 4. Opaque handle pattern `[S]` `[R]`

The engine adapters pass no handle (§1.1). Any boundary that does — today the Linux C ABI — follows this pattern:

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

Both extern fns wrap their bodies in `catch_unwind` per §1.2. Every `unsafe` block carries a `// SAFETY:` comment per §3. The handle is `#[repr(transparent)]` so the ABI matches `*mut Engine` exactly.

- **Drop discipline**: every handle has an explicit free / shutdown entry matched on both sides; the Rust type implements `Drop` with the full teardown; a null handle is a no-op, never a dereference. Failure modes blocked: khiin-rs Kotlin declares `external fun shutdown(enginePtr: Long)` (`references/khiin-rs/android/app/src/main/kotlin/be/chiahpa/khiin/EngineManager.kt:39-48`) with no matching Rust extern (`references/khiin-rs/android/rust/src/lib.rs:11-79`) — the link succeeds and the call fails at runtime; khiin-rs Swift `EngineBridge` (`references/khiin-rs/swift/bridge/src/lib.rs:33-48`) has no `Drop`, so the boxed engine leaks on every teardown.
- **Kotlin / Swift side** (only if a handle ever crosses those seams): Kotlin wraps `jlong` in a `@JvmInline value class`, freed from `close()` / `onDestroy`; Swift holds the swift-bridge wrapper via ARC and frees from `deinit`.
- **C / C++ side**: the header documents ownership of every returned pointer and which `*_free` releases it.
- **Never expose the raw pointer to platform code.** The handle is opaque.

## 5. Enforcement hooks `[A]`

- **Spec docs**: `docs/engine/rust-core-proto.md` cites this rules file. Rule deviations require `// JUSTIFICATION:` prose in-line at the deviation site.
- **Every Rust FFI PR** is reviewed against §§1–4 here plus the `docs/contributing/cross-platform-alignment.md` §1c shared-core-candidate equivalence constraint, with the design reviewed before implementation and the diff after. New `unsafe` blocks (§3) always take both reviews; deviations land only with written rationale.

## 6. Test contract

| ID | Test | Pass condition | Pinned in |
|---|---|---|---|
| T1 | Panic at FFI | Forced panic inside a Rust entry → platform receives encoded `FAIL_INTERNAL`; app does not crash | `panic_for_test` / `panicForTest` entries |
| T1' | Library-side panic | A panicking dispatcher behind the same `catch_unwind` boundary answers `FailInternal` | `engine/dispatch/src/lib.rs` `forced_dispatcher_panic_is_caught_and_returns_fail_internal` |
| T2 | Session lifecycle | 1000 IME session create / destroy cycles → RSS stable, no growing handle table | — |
| T3 | Thread safety | Two concurrent requests from different threads → both valid, TSan clean | — |
| T4 | Malformed protobuf | Invalid bytes → `FAIL_PARSE`, no panic | `engine/dispatch/tests/malformed.rs`; iOS `RustEngineBridgeTests.test_T4_malformedBytes_returnsFailParse`; macOS `EngineFfiSmokeTests` |
| T5 | Oversized payload | Over `MAX_REQUEST_BYTES` → `FAIL_INVARIANT` before copy / decode; exactly the cap is accepted | `engine/dispatch/tests/oversized.rs`, `engine/dispatch/src/lib.rs` `request_cap_accepts_exactly_the_cap_and_refuses_one_byte_over`; each adapter keeps an over-cap test for its mapping |
| T6 | Logging round-trip | Rust `log::warn!` reaches the platform sink with category preserved | `engine/dispatch/tests/logging.rs` |
| T7 | Null handle (handle ABIs only) | Every entry called with a null handle answers its documented sentinel without dereferencing | — |
| T8 | Double free (handle ABIs only) | Freeing twice is bounded (idempotent or documented invalid); no UB | — |
| T9 | Call after free (handle ABIs only) | A call after free answers the sentinel without touching freed memory | — |

## 7. What is NOT shared-core

Rust core never sees platform-only surfaces. The authoritative exclude lists live in:

- `docs/contributing/ios-architecture.md` § Shared-core candidates (criteria + exclusions)
- `docs/contributing/android-guidelines.md` §1
- `docs/contributing/cross-platform-alignment.md` §1c
- `docs/engine/migration-inventory.csv` (filter `status=wont_migrate` for the current exclusion set)

This file does not re-enumerate those symbols; a third copy of the same list would force every expansion to update three places.

## 8. References

- `docs/contributing/rust-best-practices.md` — parent file: workspace, errors, crates, tests, versions, non-goals
- `docs/contributing/rust-migration-policy.md` — slice migration policy
- `docs/contributing/cross-platform-alignment.md` §1c, §4.1 — shared-core-candidate constraint + non-goals
- `docs/engine/rust-core-proto.md` — Request/Response schema
- `references/khiin-rs/khiin/src/engine.rs:57` — `send_command_bytes` single-entry-point shape
- `references/ChiaKey` `ChiaKeyCore` — the opaque-handle C ABI shape `taigi-linux-ffi` follows
- Rustonomicon (https://doc.rust-lang.org/nomicon/) — authoritative `unsafe` reference
