# engine — Rust shared-core workspace

Cross-platform shared core for TaigiKeyboard. All five platforms route their phonetics, composing, lexicon-ranking, next-word and user-data call sites through the same Rust implementation, `dispatch::process_request` over a proto-encoded byte buffer: iOS through `swift-ffi`, Android through `android-jni`; macOS, Windows and Linux key paths through `taigi-desktop-core` (`desktop/`, a path dependency on `dispatch`) — macOS via its `macos/crates/taigi-macos-ffi` archive, which also links `swift-ffi` as an rlib for a side channel (user-data pages, punctuation query, logger sink); Windows / Linux in-process. Diagrams: `../docs/architecture/system-overview.md` §1–2.

## Crates

| Crate | Role |
|---|---|
| `phonetics` | Domain crate: TL ↔ POJ ↔ TPS conversion, tone diacritics, Unicode preprocessing, custom-dictionary derivation, case-transform. `forbid(unsafe_code)`. |
| `composing` | Domain crate: IME composing-state machine + `EngineHandle` singleton (`Mutex<…>` + `once_cell::sync::OnceCell`). `forbid(unsafe_code)`. |
| `nextword` | Domain crate: bigram next-word model + generation counter + filter/boost. `forbid(unsafe_code)`. |
| `lexicon` | Domain crate: `fst` + dictionary/association mmap reader + classification. `forbid(unsafe_code)`. |
| `ranking` | Domain crate: continuous-input ranking primitives — source rank, user-frequency boost + decay, dictionary-derived score, previous-word context (`context.rs`). No internal deps. `forbid(unsafe_code)`. |
| `userdata` | Domain crate: the user's SQLite stores (`user_frequency.db`, `user_association.db`, `custom_dictionary.db`, `learned_phrases.db`) + `.taigi` backup, behind `UserDataHandle`; bundled `rusqlite`, linked only through `dispatch`'s `user-data` feature. |
| `protos` | `prost`-generated wire types. Single envelope shared across all domain crates. |
| `mmap-host` | Sole loader of mmap-backed assets. One of three crates with `unsafe_code = "allow"`. |
| `dispatch` | Top-level FFI router. Single `process_request(&[u8]) -> Vec<u8>` entry; decodes the envelope, routes by `Request.payload` variant to the matching domain crate, encodes the response. Owns `MAX_REQUEST_BYTES` and the panic-boundary `catch_unwind`. |
| `swift-ffi` | `["staticlib", "rlib"]` — `swift-bridge` entry points (`process_request_bytes` and friends, `src/lib.rs`): the iOS staticlib in `RustTaigi.xcframework`, and an rlib inside macOS's `taigi-macos-ffi` archive. |
| `android-jni` | `cdylib` — JNI entry points (`Java_…_RustEngineBridge_processRequestBytes` and friends) consumed by `RustEngineBridge.kt`. |
| `build-helpers/fst-builder` | Offline CLI that builds and queries the lexicon `.fst` artifacts. Not shipped to platforms. |
| `test-support` | Dev-dependency of the `phonetics` / `composing` / `lexicon` / `userdata` / `dispatch` tests: temp files, TKDB / TKWA / FST fixture serializers, the per-binary install lock, production artifacts and a once-per-process `dictionary.csv` loader. Depends on no engine crate. Not shipped. |

Dependency graph: `../docs/architecture/system-overview.md` §2; layering rule: `../docs/contributing/rust-best-practices.md` §1a.

## Authoritative contracts

- `../docs/engine/rust-core-proto.md` — Wire shape per slice.
- `../docs/contributing/rust-best-practices.md` — Workspace conventions, MSRV, crate choices.
- `../docs/contributing/rust-ffi-safety.md` — FFI seam discipline (panic isolation, Mutex, error envelope, logging bridge, opaque handles, test contract); §2 Domain↔proto boundary rule (the dispatch / RPC façade of each domain crate accepts/returns `protos::engine::*` directly).

## Toolchain

Rust stable channel (`rust-toolchain.toml`). `prost-build` compiles `.proto` files at build time; `swift-bridge-build` is invoked from `engine/swift-ffi/build.rs` and the resulting bridge artifacts are bundled into the xcframework by `engine/scripts/build-xcframework.sh`. `cargo-ndk` drives the Android cross-compile from `engine/scripts/build-android-libs.sh`. `engine/scripts/build-macos-xcframework.sh` builds the macOS `taigi-macos-ffi` archive (which links `swift-ffi` as an rlib); `make build` runs it as its last step, so every platform's artefacts come from one command and none can go stale behind another.

## MSRV

Rust 1.89, set in `Cargo.toml` `[workspace.package].rust-version`. Bumping MSRV is a PR-level decision per `../docs/contributing/rust-best-practices.md` §6 (Rust version policy).
