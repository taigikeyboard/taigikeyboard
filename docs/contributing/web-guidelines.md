# Web Project Guidelines

Rules for the web version (`web/`: the `taigi-web` crate compiled to `wasm32-unknown-unknown`, over `desktop/` + the shared engine). Design record and phase plan: `docs/architecture/web-roadmap.md`.

## Build

- Run `cargo` from `web/` (`make -C web …`). Rustup reads `web/rust-toolchain.toml` from the working directory, and that file is what supplies the target.
- `make web-check` is the gate: release build, `wasm-bindgen`, `wasm-opt`, Node smoke test, clippy, biome, fmt. It builds from source, so it has no stale-artifact step. `make web-e2e` is the browser gate, separate because it downloads three browsers. `make web-serve` runs the page on `http://127.0.0.1:8000`.
- `make -C web site` stages `web/src/`, the bindings, the dictionaries and the fonts into `web/target/site/`, the tree the browser sees. `scripts/subset-fonts.sh` cuts the fonts to `web/target/fonts/`, and reruns only when a font or `fonts.css` changes. It needs `pyftsubset` (`mise install`). The output is 19 MB, so it is never committed.
- `wasm-bindgen` is pinned twice, exactly: the crate in `web/Cargo.toml`, the CLI in `mise.toml`. Move both in one change; `make -C web bindgen-version` compares them. `biome.json`'s `$schema` follows the `biome` pin.
- The release profile keeps `strip` off. `strip = true` removes the `target_features` section that `wasm-bindgen` and `wasm-opt` read; `wasm-opt` drops the debug names instead. `wasm-opt --all-features` emits an encoding Node 26 rejects (`unknown import kind 0x7f`).

## Rust

- No `#[cfg(target_family = "wasm")]` outside `engine/mmap-host`, `engine/lexicon/src/paths.rs` and the `mod bindings` line of `taigi-web`'s `lib.rs`. A difference the browser needs is a value the shell passes in (`RuntimeParts`), as the clock is.
- `bindings.rs` is the only module that names the browser: `wasm-bindgen` imports and `JsError` panic off this target, and `mmap_host::register` exists only on it. Everything else in `taigi-web` is plain Rust with native tests (`make -C web test`).
- No hand-written `unsafe`. `unsafe_code = "forbid"` holds with `#[wasm_bindgen]`.
- Anything on the key path that calls `SystemTime::now`, `Instant::now`, `std::fs` or `std::thread` compiles here and fails at run time. `cargo check` for this target proves nothing about those; the smoke test does.
- This target has no unwinding: a panic traps the instance (`rust-ffi-safety.md` §1.2).

## Page

- One write path into the textarea: `surface.js`, through `execCommand`. Assigning `.value` wipes the undo history.
- Every engine call after startup goes through `call` in `ime.js`, which turns a trap into a new instance; `start` handles its own, because a trap there would only repeat. A new instance needs a new copy of the glue module (`loader.js`): the glue keeps the instance it has.
- A caret or text change the page did not make is found by comparing at the next key (`Surface.takeForeignChange`), not by `selectionchange`, which is asynchronous and fires on a different target per engine.
- One settings document, `taigi.settings`, in desktop's shape and key spellings (`settings.js`). A form control is named after its key and its markup default is that key's default; the session is the judge of a stored document. A chord's write comes back in the reply and goes through `Settings.adopt`, so the form and storage follow it.
- `prepaint.js` restores the theme, the font and the sidebar before first paint. It is the first child of `<body>`: as an external script in `<head>`, on either side of the stylesheet, WebKit left `body` on stale or missing rules in up to 4 of 10 reloads, which a first load does not show and parallel load brings out. Found by bisecting in Playwright's WebKit (`--repeat-each`); the mechanism is not known.
- UI text comes from `i18n/*.json` through `web/src/generated/strings.js` (`make i18n`). An element names its key in `data-i18n` or `data-i18n-<attribute>`, `namespace.key`. The markup's English holds the layout and stays unseen until `setDisplayLanguage` has run, which `Settings` does before anything else draws. Text from the session arrives already translated, by the same `displayLanguage` setting and locale. `effectiveLanguage` in `i18n.js` is a twin of `DisplayLanguage::from_tag` + `effective`, because the page draws before the engine has loaded.
- Fonts are CJK-only by `unicode-range`, which also keeps an installed copy off romanization: these fonts misplace combining tone marks. Every face repeats one range, and `subset-fonts.sh` reads it from `fonts.css`. `data-font` picks the face through `--font`, so a face the setting does not name is never fetched.
- Every state has a slot in the layout (`#message`, `#notice` with `visibility`), so nothing that appears shifts the page. The panel flips above the caret's line rather than pass the editor's bottom, so it never covers Copy or Clear.
- The card is `#editor`, not the textarea: `border-radius` would take the textarea's corners out of its click area.
- Hit by a real browser, not by Node: WebKit folds consecutive edits into one undo step, typed ones included, so undo can take back more than the last word.

## Tests

- Native tests cover the session and the key translation, with no lexicon. The engine is one per process, so a test that asserts what it composed takes the `ENGINE` turn and leaves it idle (`taigi-web` `session.rs`, `desktop/crates/taigi-desktop-core/tests/session_language_mode.rs`).
- The smoke test (`web/tests/`) drives the real engine and real dictionaries through the bindings the page loads, one key at a time, with `wasm-bindgen --target web` + `initSync`.
- Expected values come from `engine/composing/tests/` fixtures, whose traces are recorded there. Never from `corpus/`, which feeds no test (`corpus/README.md`).
- Passing in Node says nothing about a browser, a low-end machine or DOM cost. Playwright (`web/e2e/`) sends real keystrokes to the staged site in Chromium, Firefox and WebKit, one test per row of the roadmap's § Text surface.
- Playwright types on a US layout, so Option and AltGr rewrites never reach it; `key.rs`'s table tests cover those.
