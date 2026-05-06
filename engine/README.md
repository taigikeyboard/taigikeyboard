# engine — Rust shared-core workspace

Cross-platform shared core for the Taigi Keyboard. iOS and Android route their phonetics, composing, lexicon-ranking, and next-word call sites through the same Rust implementation via a single proto-encoded byte buffer crossing the FFI seam.

## Crates

| Crate | Role |
|---|---|
| `phonetics` | Domain crate: TL ↔ POJ ↔ TPS conversion, tone diacritics, Unicode preprocessing, custom-dictionary derivation, case-transform. `forbid(unsafe_code)`. |
| `composing` | Domain crate: IME composing-state machine + `EngineHandle` singleton (`Mutex<…>` + `once_cell::sync::OnceCell`). `forbid(unsafe_code)`. |
| `nextword` | Domain crate: bigram next-word model + generation counter + filter/boost. `forbid(unsafe_code)`. |
| `lexicon` | Domain crate: `fst` + dictionary/association mmap reader + classification. `forbid(unsafe_code)`. |
| `ranking` | Domain crate: candidate dedup → score → sort → TPS dedup for the lexicon pipeline. `forbid(unsafe_code)`. |
| `protos` | `prost`-generated wire types. Single envelope shared across all domain crates. |
| `mmap-host` | Sole loader of mmap-backed assets. One of three crates with `unsafe_code = "allow"`. |
| `dispatch` | Top-level FFI router. Single `process_request(&[u8]) -> Vec<u8>` entry; decodes the envelope, routes by `Request.payload` variant to the matching domain crate, encodes the response. Owns `MAX_REQUEST_BYTES` and the panic-boundary `catch_unwind`. |
| `swift-ffi` | `staticlib` — `swift-bridge` entry points consumed by the iOS extension via `RustTaigi.xcframework`. |
| `android-jni` | `cdylib` — JNI entry points consumed by `RustEngineBridge.kt`. |
| `build-helpers/fst-builder` | Offline CLI that builds and queries the lexicon `.fst` artifacts. Not shipped to platforms. |

Dependency direction: `swift-ffi` / `android-jni` → `dispatch` → `composing` / `nextword` / `lexicon` / `ranking` / `phonetics`. The first four also depend on `phonetics`; `lexicon` additionally depends on `mmap-host`. All domain crates depend on `protos` directly.

## Authoritative contracts

- `../docs/engine/ffi-safety.md` — FFI seam discipline (panic isolation, Mutex, Drop, error sentinels, logging bridge).
- `../docs/engine/rust-core-proto.md` — Wire shape per slice.
- `../rules/rust-best-practices.md` — Workspace conventions, §3a domain↔proto boundary rule, MSRV, crate choices.

## Toolchain

Rust stable channel (`rust-toolchain.toml`). `prost-build` compiles `.proto` files at build time; `swift-bridge-build` is invoked from `engine/swift-ffi/build.rs` and the resulting bridge artifacts are bundled into the xcframework by `engine/scripts/build-xcframework.sh`. `cargo-ndk` drives the Android cross-compile from `engine/scripts/build-android-libs.sh`.

## MSRV

Rust 1.86, set in `Cargo.toml` `[workspace.package].rust-version`. Bumping MSRV is a PR-level decision per `../rules/rust-best-practices.md` §8. Two prior bumps documented: 1.75 → 1.85 in D9.1 (`edition2024` ecosystem catch-up), 1.85 → 1.86 in D9.2 (`cargo-ndk` 4.x requirement).
