# FFI Safety Rules

> **Type**: Reference
> **Keywords**: `FFI`, `panic`, `Drop`, `Mutex`, `EngineHandle`, `ErrorCode`, `logging-bridge`, `catch_unwind`, `shutdown`
> **Related**: `rust-core-proto.md`, `../architecture/behavioral-invariants.md`, `../../.claude/rules/rust-best-practices.md`, `../../.claude/rules/rust-ffi-safety.md`
> **Audience**: anyone authoring Rust core code or platform FFI glue.

---

## 1. Scope and audience

- Applies to every Rust function exposed via `jni` (Android) or `swift-bridge` (iOS / macOS).
- **Does NOT apply** to internal Rust code that never crosses the FFI boundary. Pure-engine and phonetics crates (`engine`, `phonetics`) follow the broader Rust idioms in `.claude/rules/rust-best-practices.md`; this spec only governs the FFI seam.
- **Authoritative companion**: `.claude/rules/rust-ffi-safety.md` §1 (FFI boundary discipline) and §4 (opaque handle pattern). Any deviation from this spec or that rules file requires inline `// JUSTIFICATION:` prose at the deviation site, per `.claude/rules/rust-ffi-safety.md` §5.

> **AS-BUILT divergence (post-D9)**: the engine implemented this contract as a **process-singleton, bytes-in / bytes-out** seam — NOT the per-session opaque-handle design that §3/§4 below pre-authored. Concretely: the FFI entry takes only `bytes: &[u8]` and reaches the engine via `EngineHandle::instance()` (`engine/composing/src/handle.rs`); **no opaque `*mut` handle crosses the boundary and there is no `engine_shutdown` FFI** (the singleton owns lifetime). There is **no `EngineError` type** — a caught panic maps directly to `ErrorCode::FAIL_INTERNAL` via `catch_unwind`. Live entrypoints: `process_request_bytes` / `install_logger_sink` / `set_log_level` / `panic_for_test` (swift-ffi) and `processRequestBytes` / `registerLogger` / `setLogLevel` / `panicForTest` (android-jni). §2 (catch_unwind mandatory) holds as-built; §3 `Mutex<Engine>` exists but is locked inside the singleton, not at the seam; §3/§4 opaque-handle + shutdown remain as pre-impl design record.

---

## 2. Panic discipline — `catch_unwind` mandatory

Every `#[no_mangle]` extern function and every `#[swift_bridge::bridge]` exported method wraps its body in `std::panic::catch_unwind`. Unwinding across a language boundary is undefined behavior on both JNI and the C ABI.

**Rule**:

- The catch-unwind closure returns `Result<Vec<u8>, EngineError>`.
- A caught panic encodes as `EngineError::InternalPanic(msg)` and converts to `Response.error = ErrorCode::FAIL_INTERNAL` in the returned protobuf bytes.
- The outer `extern fn` never returns a Rust `Result` or `Option` — it returns either a valid byte buffer or the sick-engine sentinel from §5.
- `?` is allowed and idiomatic **inside** the closure (where `?` propagates `EngineError`); it is forbidden as the closure's outer return shape.

**Failure mode this rule blocks** — khiin-rs's JNI glue propagates `.expect(...)` panics on bad input. `references/khiin-rs/android/rust/src/lib.rs:51-56` parses the request bytes with `Request::parse_from_bytes(&bytes).expect("Could not parse Request bytes")`; a malformed payload from the platform aborts the entire IME process.

**D9 POC test contract**: malformed protobuf bytes, invalid UTF-8, and oversized payloads each return an `ErrorCode` value — never a panic, never an abort, never an unwind.

---

## 3. Thread safety — `Mutex<Engine>`, never raw pointer + `&mut`

Engine state is `Send + !Sync`. The platform may deliver concurrent calls (IME thread + settings push from main thread + background dictionary refresh). The engine MUST be guarded by a `Mutex<Engine>` that the FFI entry function locks before touching engine state.

**Rule**:

- The opaque `EngineHandle` type from §4 wraps `*mut Mutex<Engine>` (or, when shared across platform threads, `*mut Arc<Mutex<Engine>>`).
- The platform MUST NOT receive a raw `*mut Engine`. Casting back via `&mut *(ptr as *mut Engine)` is forbidden.
- `Arc<Mutex<Engine>>` is reserved for the case where the same handle is shared across platform threads. One engine per IME session is the common case and uses plain `Mutex<Engine>`.
- `RwLock` is NOT part of this spec. The mutex serializes; performance tuning lives post-D9.

**Failure mode this rule blocks** — khiin-rs's JNI glue casts the raw pointer back to `&mut Engine` with no synchronization or null check at `references/khiin-rs/android/rust/src/lib.rs:64`. The Swift bridge does the same at `references/khiin-rs/swift/bridge/src/lib.rs:51-52`. Either pattern allows two concurrent platform calls to alias the same `&mut`, which is undefined behavior in Rust regardless of platform.

**D9 POC test contract**: two threads issuing `send_command_bytes` from different platform threads complete without data race (TSan clean) and produce deterministic serialization.

---

## 4. Drop discipline — explicit `shutdown` matched on both sides

Every opaque handle exposes an explicit `engine_shutdown(handle)` FFI. The Rust engine type implements `Drop` with the full teardown path (close DB, flush user-frequency, drop dictionary handles). The platform side calls shutdown deterministically.

The opaque handle pattern itself is defined in `.claude/rules/rust-ffi-safety.md` §4. This section adds the platform-binding contract and the test surface; it does not redefine the pattern.

**Platform binding**:

- **iOS**: a Swift class (e.g. `EngineBridge`) holds the handle and calls `engine_shutdown` from `deinit`.
- **Android**: a Kotlin class (e.g. `EngineManager`) wraps the handle as `@JvmInline value class EngineHandle(val raw: Long)` and calls `engine_shutdown` from `close()` / `onDestroy`.

**Failure modes this rule blocks**:

- khiin-rs's Kotlin declares `external fun shutdown(enginePtr: Long)` at `references/khiin-rs/android/app/src/main/kotlin/be/chiahpa/khiin/EngineManager.kt:39-48`, but `references/khiin-rs/android/rust/src/lib.rs:11-79` exposes only `Java_..._load` and `Java_..._sendCommand`. There is no matching `Java_..._shutdown` extern. The Kotlin link succeeds at compile time; the runtime call would fail with `UnsatisfiedLinkError` if it ever ran.
- khiin-rs's Swift `EngineBridge` (`references/khiin-rs/swift/bridge/src/lib.rs:33-48`) defines `engine_ptr: *mut c_void` initialized via `Box::into_raw` at line 40, with no `Drop` impl anywhere in the file. The boxed `Engine` leaks on every IME teardown.

**D9 POC test contract** (T2 / T8 / T9 in §7):

- 1000 IME create/destroy cycles → RSS / heap stable per platform tools; no growing handle table.
- Double `engine_shutdown` is bounded — either idempotent or documented invalid; either way no UB.
- Calling `send_command_bytes` after `engine_shutdown` returns the sick-engine sentinel without dereferencing freed memory.

---

## 5. Error sentinels — protobuf envelope is the only error channel

Every FFI return from a stateful operation is one of:

- (a) **Valid protobuf bytes** carrying `Response.error: ErrorCode`. The platform always parses the envelope first.
- (b) **Out-of-band failure signal** — null pointer / negative length. The platform treats this as "engine is sick, restart this IME session."

Rust never propagates a `Result` or `Option` across the ABI (`.claude/rules/rust-best-practices.md` §2). Rust error types stay inside the catch-unwind closure; they convert to `ErrorCode` at the seam.

**Initial ErrorCode shape** (final values land with the Phase III proto):

| Code | Name | Cause |
|---|---|---|
| 0 | `OK` | Success |
| 1 | `FAIL_PARSE` | Malformed bytes |
| 2 | `FAIL_INTERNAL` | Caught panic |
| 3 | `FAIL_IO` | DB / file error |
| 4 | `FAIL_INVARIANT` | Engine invariant violated (should have been caught earlier — log + return for diagnostics) |

`FAIL_INVARIANT` exists so an engine bug surfaces as data rather than a crash. The platform logs it and the engine continues serving the next request.

---

## 6. Logging bridge — Rust uses `log` crate only; platform installs the adapter

Rust core code (`engine`, `phonetics`) imports nothing beyond the `log` crate. Forbidden in candidate code: `OSLog`, `os_log`, `android.util.Log`, `__android_log_print`, `println!`, `eprintln!`.

**Setup contract**:

- Each platform FFI crate (`android-jni`, `swift-ffi`) defines a `PlatformLoggerAdapter` that implements `log::Log`.
- `log::set_logger` is **process-global** and only succeeds **once** for the lifetime of the process. The platform FFI crate wraps the registration in a `std::sync::Once` (or equivalent `OnceLock`) so the call is idempotent across IME session create/destroy cycles. Per-engine init MUST NOT call `log::set_logger` directly — the second call returns `SetLoggerError`, and an unwrapped panic on that path would crash the IME on the second session start (which would also break the T2 / T8 / T9 lifecycle tests in §7).
- The adapter forwards each `log::Record` into the platform's existing `LoggerBackend` — the same protocol candidate Swift/Kotlin code already uses (`docs/architecture/behavioral-invariants.md:313-323` §12).
- `engine_shutdown` does NOT unregister the logger; the adapter outlives engine instances.

**Adapter ownership rule**: the adapter type lives in the platform FFI crate, not in `engine` or `phonetics`. Pure-engine crates never import platform logging adapters; they import `log` and that is all.

This rule extends the existing platform `LoggerBackend` invariant — Rust core is one more candidate that depends on the logger interface, not on a platform log API.

---

## 7. Test contract for D9 POC acceptance

The POC ships with these tests, run on both iOS and Android (per `.claude/rules/rust-best-practices.md` §5):

| ID | Test | Pass condition |
|---|---|---|
| T1 | Panic at FFI | Force panic inside Rust function → platform receives encoded `ErrorCode::FAIL_INTERNAL`; app does not crash |
| T2 | Drop / cleanup | 1000 IME session create/destroy cycles → RSS stable, no growing handle table |
| T3 | Thread safety | Two concurrent `send_command_bytes` from different threads → both return valid responses, TSan clean |
| T4 | Malformed protobuf | Invalid bytes → `ErrorCode::FAIL_PARSE`, no panic |
| T5 | Oversized payload | >1MB byte buffer → bounded behavior (reject or truncate); behavior documented either way |
| T6 | Logging round-trip | Rust `log::warn!` reaches platform log sink with category preserved |
| T7 | Null handle | Every FFI fn called with a null `EngineHandle` returns the sick-engine sentinel without dereferencing |
| T8 | Double shutdown | Calling `engine_shutdown` twice is bounded (idempotent or documented invalid); no UB |
| T9 | Call after shutdown | `send_command_bytes` after `engine_shutdown` returns the sick-engine sentinel without dereferencing freed memory |

Tests live in `android-jni/tests/` and `swift-ffi/tests/` per `.claude/rules/rust-best-practices.md` §5.

---

## 8. What is NOT shared-core

Rust core never sees platform-only surfaces. The authoritative exclude lists live in:

- `../architecture/ios-exemplar.md` §1 (layer map), §9.2–§9.4 (Android deviations: live-read, coroutines, InputConnection)
- `../../rules/ios-shared-core-candidates.md` §1 (criteria + exclusions)
- `../../rules/android-guidelines.md` §1
- `../../rules/cross-platform-alignment.md` §1c
- `migration-inventory.csv` (live roster — filter `status=wont_migrate` for the current exclusion set)

This document does not re-enumerate those symbols. Adding a third copy of the same blacklist would force every future expansion to update three places — see `.claude/rules/cross-platform-alignment.md` §1c for the authoritative-list pointer rationale.

---

## 9. References

- `.claude/rules/rust-ffi-safety.md` — mandatory companion (§1 FFI discipline, §3 unsafe, §4 opaque handle pattern, §5 enforcement hooks)
- `.claude/rules/rust-best-practices.md` — secondary companion (§2 error handling, §8 non-goals)
- `references/khiin-rs/khiin/src/engine.rs:57` — `send_command_bytes` single-entry-point shape
- `references/khiin-rs/android/rust/src/lib.rs:51-56` — JNI parse panic (failure mode for §2)
- `references/khiin-rs/android/rust/src/lib.rs:64` — unsafe `&mut` from raw pointer with no sync (failure mode for §3)
- `references/khiin-rs/swift/bridge/src/lib.rs:33-48` — no `Drop` impl on bridge struct (failure mode for §4)
- `references/khiin-rs/android/app/src/main/kotlin/be/chiahpa/khiin/EngineManager.kt:39-48` — Kotlin shutdown declaration without matching Rust extern (failure mode for §4)
- `docs/architecture/behavioral-invariants.md` §11 (settings live-read), §12 (logger backend neutrality)
- `docs/architecture/ios-exemplar.md` §1 (layer map), §9.2–§9.4 (Android deviations)
