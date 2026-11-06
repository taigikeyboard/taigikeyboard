# Web Project Guidelines

Rules for the web version (`web/`: the `taigi-web` crate compiled to `wasm32-unknown-unknown`, over `desktop/` + the shared engine). Design record and phase plan: `docs/architecture/web-roadmap.md`.

## Build

- Run `cargo` from `web/` (`make -C web …`). Rustup reads `web/rust-toolchain.toml` from the working directory, and that file is what supplies the target.
- `make web-check` is the gate: release build, `wasm-bindgen`, `wasm-opt`, Node smoke test, clippy, biome, fmt. It builds from source, so it has no stale-artifact step.
- `wasm-bindgen` is pinned twice, exactly: the crate in `web/Cargo.toml`, the CLI in `mise.toml`. Move both in one change; `make -C web bindgen-version` compares them. `biome.json`'s `$schema` follows the `biome` pin.
- The release profile keeps `strip` off. `strip = true` removes the `target_features` section that `wasm-bindgen` and `wasm-opt` read; `wasm-opt` drops the debug names instead. `wasm-opt --all-features` emits an encoding Node 26 rejects (`unknown import kind 0x7f`).

## Rust

- No `#[cfg(target_family = "wasm")]` outside `engine/mmap-host` and `engine/lexicon/src/paths.rs`. A difference the browser needs is a value the shell passes in (`RuntimeParts`), as the clock is.
- No hand-written `unsafe`. `unsafe_code = "forbid"` holds with `#[wasm_bindgen]`.
- Anything on the key path that calls `SystemTime::now`, `Instant::now`, `std::fs` or `std::thread` compiles here and fails at run time. `cargo check` for this target proves nothing about those; the smoke test does.
- This target has no unwinding: a panic traps the instance (`rust-ffi-safety.md` §1.2).

## Tests

- The smoke test (`web/tests/`) drives the real engine and real dictionaries through the bindings the page loads, with `wasm-bindgen --target web` + `initSync`.
- Expected values come from `engine/composing/tests/` fixtures, whose traces are recorded there. Never from `corpus/`, which feeds no test (`corpus/README.md`).
- Passing in Node says nothing about a browser, a low-end machine or DOM cost.
