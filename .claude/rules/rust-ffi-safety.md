---
paths:
  - "engine/android-jni/**"
  - "engine/swift-ffi/**"
  - "engine/dispatch/**"
  - "engine/protos/**"
  - "engine/**/src/api.rs"
  - "engine/**/src/dispatch.rs"
  - "engine/**/src/handle.rs"
  - "engine/**/ffi*.rs"
---

# Rust FFI Safety

Mandatory rules for the Rust ↔ platform boundary: FFI surface, domain↔proto layering, `unsafe` discipline, opaque-handle pattern, enforcement hooks. Split out from `.claude/rules/rust-best-practices.md` for focus. General Rust hygiene (workspace, errors, crates, tests, versions) stays in the parent file.

**Active window**: every Rust PR touching `swift-ffi/`, `android-jni/`, `engine/dispatch`, `protos/`, or any domain crate's RPC façade.

## 1. FFI boundary discipline `[S]`

Policy lives here; the technical spec is `docs/engine/ffi-safety.md`. Enforcement:

- **`std::panic::catch_unwind` wraps every FFI function body.** A panic → encoded `EngineError::InternalPanic(msg)` → protobuf `Response.error_code` → platform recovers. Unwind across FFI is undefined behavior in both JNI and C ABI.
- **Engine is guarded by `Mutex<Engine>`**. Engine state is `Send + !Sync`; the mutex serializes concurrent native calls. `Arc<Mutex<Engine>>` only if the handle is shared across platform threads (usually not needed — one engine per IME session).
- **Drop discipline**: every opaque handle exposes an explicit `shutdown(handle)` FFI. Rust side implements `Drop` with the same teardown path. Platform side (Kotlin `use {}` / Swift `deinit`) must call `shutdown`. Two khiin-rs failure modes this rule blocks: (a) Kotlin `EngineManager.kt:48` declares `external fun shutdown(enginePtr: Long)` with no matching Rust `extern fn` in `android/rust/src/lib.rs` — Kotlin link succeeds but runtime call panics; (b) `swift/bridge/src/lib.rs:33-35` defines `EngineBridge { engine_ptr: *mut c_void }` with zero `Drop` impl anywhere in the file — the boxed `Engine` leaks on app teardown.
- **Error sentinels travel in protobuf**: every FFI return is either a valid protobuf byte buffer carrying `Response.ErrorCode`, or an out-of-band failure (null bytes / negative length) that means "engine is sick, restart this IME session". Never leak Rust error types across the ABI.
- **Logging bridge**: the `log` crate is the only logging API candidate code sees. Platform adapter (`OSLog` on iOS, `android.util.Log` on Android) is registered once at engine init via `log::set_logger`. Candidate code never imports `OSLog`, `android.util.Log`, or any platform log API.

## 2. Domain↔proto boundary rule `[R]` `[A]`

Established 2026-04-29 after the second domain crate landed (PR #189 ranking slice). Codifies the surface pattern both `engine/phonetics` and `engine/ranking` already follow, so `engine/composing` (and any future stateless slice) inherits a consistent template.

**Rule.** The **dispatch / RPC façade** of each domain crate (the function the dispatcher routes through — `phonetics::dispatch::handle`, `ranking::process_candidates`, future `composing::*`) accepts and returns **protobuf-generated types** (`protos::engine::*`) directly. There is no parallel native-Rust mirror tier and no proto↔native translation layer between `engine/dispatch` and the domain crate. The protobuf schema is the cross-platform contract; duplicating it doubles maintenance with no consumer.

This rule binds the cross-platform RPC seam, not every public function. CLI helpers, test fixtures, and stable native-Rust convenience APIs (`phonetics::to_tone_marks`, `phonetics::to_tone_number`, `phonetics::normalize_to_tl`, `phonetics::strip_tone_mark`, etc.) may keep native signatures — they were intentionally exposed for in-process Rust callers (CLI, integration tests). What is forbidden is letting those native helpers grow into a **second proto-mirroring type tier** that the dispatcher routes through.

**Module visibility.** Implementation modules are `mod`-private. Only **named façade / entry modules** are `pub mod`, and they expose only the named entry points downstream actually call. Top-level `pub use` re-exports are reserved for stable cross-crate symbols (CLI helpers, integration-test helpers, the public `Error` enum) — never as a redundant alias for an entry already reachable through a façade module.

Concretely, `engine/phonetics/src/lib.rs` is the canonical shape:

```rust
pub mod api;        // tests + CLI hit phonetics::api::*
pub mod dispatch;   // engine/dispatch routes through phonetics::dispatch::handle

mod case_adjust;
mod derivation;
mod normalization;
mod syllable;
// … all other implementation modules stay private
```

**Layering by crate.**

| Layer | Type vocabulary | Visibility |
|---|---|---|
| `phonetics`, `ranking`, `composing` (domain) | **Dispatch façade** takes / returns `protos::engine::*` directly. Native-Rust helpers (CLI / test convenience functions) may exist alongside but never grow into a parallel mirror tier. | Implementation modules `mod`-private; one or two `pub mod` façades; `pub use` only for genuine cross-crate symbols. |
| `engine/dispatch` | Single `process_request(&[u8]) -> Vec<u8>`. Decodes once, routes by `Request.payload` variant to the matching domain crate, encodes once. | Pure routing — no proto↔proto translation. |
| `swift-ffi`, `android-jni` | Bytes in, bytes out across the FFI seam. `catch_unwind` per §1. | Calls `dispatch::process_request` directly. |

**What this rule excludes.** Native-Rust input/output structs that mirror proto messages, `From<NativeFoo> for protos::engine::Foo` impls, separate per-op entry points in dispatch (`dispatch::process_phonetics`, `dispatch::process_ranking`, …) — all banned. They show up in candidate refactors and they are always extra work for no end-user benefit.

**When this rule may be revisited.** If a future slice needs to expose a domain API that takes / returns Rust-native types because the public Rust crate has consumers outside the IME (e.g. someone embeds `phonetics` in a non-IME tool). Until that happens, Pattern A holds.

## 3. `unsafe` discipline `[S]`

- **Every `unsafe` block carries a `// SAFETY:` comment** explaining the invariant that makes the operation sound. The khiin-rs unsafe deref at `references/khiin-rs/swift/bridge/src/lib.rs:52` has no SAFETY note — this pattern is rejected at review.
- **`unsafe` blocks are confined to FFI marshaling.** No domain logic inside `unsafe`. Target: `unsafe` block contents ≤ 3 lines.
- **No `transmute` unless absolutely required** — prefer `as` casts, `From`/`Into`, or `#[repr(C)]` layout-compatible structs.
- **No raw pointer dereferences outside FFI crates.** `phonetics` and `engine` are `#![forbid(unsafe_code)]` at the crate root; only `android-jni` and `swift-ffi` may contain `unsafe`.
- **Every new `unsafe` block requires Codex pre-implementation review** per `.claude/rules/cross-platform-alignment.md` §1c.

## 4. Opaque handle pattern `[S]` `[R]`

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

Both extern fns wrap their bodies in `catch_unwind` per §1. Every `unsafe` block carries a `// SAFETY:` comment per §3. The handle is `#[repr(transparent)]` so the ABI matches `*mut Engine` exactly.

- **Kotlin side**: wrap `jlong` in `@JvmInline value class EngineHandle(val raw: Long)` — type-safe, zero runtime cost.
- **Swift side**: swift-bridge generates the wrapper; platform holds it via ARC.
- **Never expose the raw pointer to platform code.** The handle is opaque.

## 5. Enforcement hooks `[A]`

- **Spec docs**: `docs/engine/ffi-safety.md` and `docs/engine/rust-core-proto.md` cite this rules file. Rule deviations in those docs require `// JUSTIFICATION:` prose in-line.
- **D9 POC and successors**: POC / FFI code is reviewed against every rule above. Deviations land only after Codex + `/simplify` pre-review and Codex post-review on the diff, with written rationale.
- **Every Rust FFI PR** runs through the `.claude/rules/cross-platform-alignment.md` §1c constraint (for shared-core-candidate equivalence), Codex + `/simplify` pre-implementation review on the plan, Codex post-edit review on the diff, plus this file's §§1–4 enforcement. `/simplify` is the Claude Code official skill and catches reuse / quality / dead-code issues Codex does not flag; run both in parallel per `~/.claude/rules/claude-workflow.md` § Subagent Usage.

## 6. References

- `.claude/rules/rust-best-practices.md` — parent file: workspace, errors, crates, tests, versions, non-goals
- `.claude/rules/rust-migration-policy.md` — slice migration policy
- `.claude/rules/cross-platform-alignment.md` §1c, §4a, §5.1 — shared-core-candidate constraint + Phase II.5 prerequisites + non-goals
- `docs/engine/ffi-safety.md` — technical spec (this file is the policy)
- `docs/engine/rust-core-proto.md` — Request/Response schema
- Rustonomicon (https://doc.rust-lang.org/nomicon/) — authoritative `unsafe` reference
