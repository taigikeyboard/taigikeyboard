# engine — Rust shared-core workspace

Cross-platform shared core for the Taigi Keyboard. iOS and Android route their phonetics + lexicon-ranking call sites through the same Rust implementation via a single proto-encoded byte buffer crossing the FFI seam.

## Crates

| Crate | Role |
|---|---|
| `phonetics` | Domain crate: TL ↔ POJ ↔ TPS conversion, tone diacritics, Unicode preprocessing, custom-dictionary derivation. `forbid(unsafe_code)`. |
| `ranking` | Domain crate: candidate dedup + score + sort for the lexicon pipeline. `forbid(unsafe_code)`. |
| `protos` | `prost`-generated wire types. Single envelope shared across all domain crates. |
| `dispatch` | Top-level FFI router. Single `process_request(&[u8]) -> Vec<u8>` entry; decodes the envelope, routes by `Request.payload` variant to the matching domain crate, encodes the response. Owns `MAX_REQUEST_BYTES`. |
| `swift-ffi` | `staticlib` — `swift-bridge` entry points consumed by the iOS extension via `RustTaigi.xcframework`. |
| `android-jni` | `cdylib` — JNI entry points consumed by `RustEngineBridge.kt`. |
| `cli` | Developer parity tool mirroring `taigi-converter`'s `tai` command. Not shipped to platforms. |

Dependency direction: `swift-ffi` / `android-jni` → `dispatch` → `phonetics` / `ranking` → `protos`.

## Authoritative contracts

- `../docs/engine/ffi-safety.md` — FFI seam discipline (panic isolation, Mutex, Drop, error sentinels, logging bridge).
- `../docs/engine/rust-core-proto.md` — Wire shape per slice.
- `../rules/rust-best-practices.md` — Workspace conventions, §3a domain↔proto boundary rule, MSRV, crate choices.

## Toolchain

Rust 1.86 (pinned in `rust-toolchain.toml` and `Cargo.toml` `[workspace.package].rust-version`). `prost-build` shells out to `protoc` at compile time; `swift-bridge-build` is invoked from `engine/scripts/build-xcframework.sh`. `cargo-ndk` 4.x drives the Android cross-compile from `engine/scripts/build-android-libs.sh`.

## MSRV

Rust 1.86. Bumping MSRV is a PR-level decision per `../rules/rust-best-practices.md` §8. Two prior bumps documented: 1.75 → 1.85 in D9.1 (`edition2024` ecosystem catch-up), 1.85 → 1.86 in D9.2 (`cargo-ndk` 4.x requirement).
