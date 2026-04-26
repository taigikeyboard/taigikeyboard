# engine — Rust shared-core workspace

Phase III D9 FFI POC. Currently sub-round **D9.1** — Phonetics-only slice.
Platform integration (iOS / Android FFI) lands in D9.2.

## Crates

| Crate | Status (D9.1) | Purpose |
|---|---|---|
| `phonetics` | Active | Pure functions: TL/POJ/TPS conversion, Unicode, tone. `forbid(unsafe_code)`. |
| `protos` | Active | `prost`-generated wire types. Envelope + Phonetics slice only in D9.1. |
| `cli` | Active | Developer parity tool mirroring `taigi-converter/`'s `tai` command. |
| `engine` | D9.3 | Composing state machine, `Mutex<Engine>`, candidate ranking. |
| `android-jni` | D9.2 | `cdylib` — JNI entry points. |
| `swift-ffi` | D9.2 | `staticlib` — `swift-bridge` entry points. |

## Authoritative contracts

- `../docs/engine/ffi-safety.md` — FFI seam discipline (panic, Mutex, Drop, error sentinels, logging bridge)
- `../docs/engine/rust-core-proto.md` — Wire shape (Phonetics + Composing slices)
- `../rules/rust-best-practices.md` — Workspace conventions, MSRV, crate choices

## Toolchain

Pinned via `../mise.toml`: Rust 1.85, protoc 27. `prost-build` shells out to `protoc` at compile time.

## MSRV

Pinned at Rust 1.85 in `Cargo.toml` `[workspace.package]`. `rust-toolchain.toml` pins the channel for local dev parity. 1.85 stabilises edition2024, which the modern Rust ecosystem (clap_lex 1.x, getrandom 0.4.x, etc.) already relies on. Bumping MSRV is a PR-level decision per `../rules/rust-best-practices.md` §8.

## Phonetics scope (D9.1)

Ports `taigi-converter/src/{tables,phonetics,tl,poj,zhuyin,converter}.js` minus the segmenter path. Word-level segmentation (`segmenter.js` + 1.6 MB `dictionary.js` trie) is NOT pulled into the IME shared-core — it is a Lexicon concern that lands with the Lexicon slice in Phase IV-B.
