# Web Project Guidelines

Rules for the web version (`web/`: the `taigi-web` crate compiled to `wasm32-unknown-unknown`, over `desktop/` + the shared engine). Design record and phase plan: `docs/architecture/web-roadmap.md`.

## Build

- Run `cargo` from `web/` (`make -C web …`). Rustup reads `web/rust-toolchain.toml` from the working directory, and that file is what supplies the target.
- `make web-check` is the gate: release build, `wasm-bindgen`, `wasm-opt`, Node smoke test, clippy, biome, fmt. It builds from source, so it has no stale-artifact step. `make web-e2e` is the browser gate, separate because it downloads three browsers; on CI it runs Chromium only. `make web-serve` runs the page on `http://127.0.0.1:8000`.
- `make -C web site` stages `web/src/`, the bindings, the dictionaries, the fonts and the icons (the iOS app icon's 180, 196 and 512) into `web/target/site/`, the tree the browser sees. `scripts/stage-assets.mjs` runs last: it gzips the dictionaries under content-hashed names, writes `generated/dictionaries.js` and `assets-manifest.json`, and stamps `sw.js` with the build. `scripts/subset-fonts.sh` cuts the fonts to `web/target/fonts/`, and reruns only when a font or `fonts.css` changes. It needs `pyftsubset` (`mise install`). The output is 19 MB, so it is never committed.
- Deploy: `docs/architecture/web-release.md`. Every path in a page is relative, because the site lives under `/web/` and tests serve it from the root.
- `wasm-bindgen` is pinned twice, exactly: the crate in `web/Cargo.toml`, the CLI in `mise.toml`. Move both in one change; `make -C web bindgen-version` compares them. `biome.json`'s `$schema` follows the `biome` pin.
- The release profile keeps `strip` off. `strip = true` removes the `target_features` section that `wasm-bindgen` and `wasm-opt` read; `wasm-opt` drops the debug names instead. `wasm-opt --all-features` emits an encoding Node 26 rejects (`unknown import kind 0x7f`).

## Rust

- No `#[cfg(target_family = "wasm")]` outside `engine/mmap-host`, `engine/lexicon/src/paths.rs` and the `mod bindings` line of `taigi-web`'s `lib.rs`. A difference the browser needs is a value the shell passes in (`RuntimeParts`), as the clock is.
- `bindings.rs` is the only module that names the browser: `wasm-bindgen` imports and `JsError` panic off this target, and `mmap_host::register` exists only on it. Everything else in `taigi-web` is plain Rust with native tests (`make -C web test`).
- No hand-written `unsafe`. `unsafe_code = "forbid"` holds with `#[wasm_bindgen]`.
- Anything on the key path that calls `SystemTime::now`, `Instant::now`, `std::fs` or `std::thread` compiles here and fails at run time. `cargo check` for this target proves nothing about those; the smoke test does.
- This target has no unwinding: a panic traps the instance (`rust-ffi-safety.md` §1.2).

## Page

- Dictionaries are gzip files read through `DecompressionStream`, so the result does not depend on what a host compresses. `loader.js` takes them one at a time, from Cache Storage or the network, through one SHA-256 check; only checked bytes are stored, and a damaged entry is dropped so that retry downloads it. Network reads use `cache: "reload"`: bad bytes may be what the HTTP cache holds.
- The page's dictionary list is `generated/dictionaries.js`, a module loaded with the page, so that an older page in the HTTP cache never meets a newer list. Only `sw.js` reads `assets-manifest.json`.
- `sw.js` is registered after the dictionaries are in, so a first visit fetches them once. Its install is all or nothing, and checks each file with `integrity`. It has no `skipWaiting` and no `clients.claim()`: a page's controller never changes, because trap recovery imports a new copy of the glue, which must match the wasm that page compiled. So a new version starts once every tab of the old one has closed; a reload does not do it, and neither does anything on screen. The exception is the tab of a first visit, which no worker controls: a deploy can activate past it, and a trap in that tab then pairs new glue with its old wasm and fails until a reload. The text stays readable. To see a rebuild in `make web-serve`, close the tab or use DevTools' "Update on reload".
- Cache Storage is per origin and `taigikeyboard.tw` also hosts other sites, so `sw.js` deletes only `taigi-web-*`.
- The CSP is a `<meta>`, the first thing in `<head>`, because Pages cannot set headers. Nothing inline and no `data:` URLs (the select's chevron is `chevron.svg`), except `style-src-attr 'unsafe-inline'`: WebKit's `<select>` sets a `style` attribute inside its shadow tree, and blocking it loses the ellipsis on a long option.
- One write path into the textarea: `surface.js`, through `execCommand`. Assigning `.value` wipes the undo history.
- Every engine call after startup goes through `call` in `ime.js`, which turns a trap into a new instance; `start` handles its own, because a trap there would only repeat. A new instance needs a new copy of the glue module (`loader.js`): the glue keeps the instance it has.
- A caret or text change the page did not make is found by comparing at the next key (`Surface.takeForeignChange`), not by `selectionchange`, which is asynchronous and fires on a different target per engine.
- One settings document, `taigi.settings`, in desktop's shape and key spellings (`settings.js`). A form control is named after its key and its markup default is that key's default; the session is the judge of a stored document. A chord's write comes back in the reply and goes through `Settings.adopt`, so the form and storage follow it.
- `<html>` carries the first visit's font and open sidebar, which also stand when storage is blocked. `prepaint.js` restores the theme, the font, the key panel's room and the sidebar before first paint. It is the first child of `<body>`: as an external script in `<head>`, on either side of the stylesheet, WebKit left `body` on stale or missing rules in up to 4 of 10 reloads, which a first load does not show and parallel load brings out. Found by bisecting in Playwright's WebKit (`--repeat-each`); the mechanism is not known.
- UI text comes from `i18n/*.json` through `web/src/generated/strings.js` (`make i18n`). An element names its key in `data-i18n` or `data-i18n-<attribute>`, `namespace.key`. The markup's English holds the layout and stays unseen until `setDisplayLanguage` has run, which `Settings` does before anything else draws. Text from the session arrives already translated, by the same `displayLanguage` setting and locale. `effectiveLanguage` in `i18n.js` is a twin of `DisplayLanguage::from_tag` + `effective`, because the page draws before the engine has loaded.
- Fonts are CJK-only by `unicode-range`, which also keeps an installed copy off romanization: these fonts misplace combining tone marks. Every face repeats one range, and `subset-fonts.sh` reads it from `fonts.css`. `data-font` picks the face through `--font`, so a face the setting does not name is never fetched. Iansui, the default, is in the service worker's shell so it works offline from the first visit (2.7 MB on that visit); the others are cached on first use.
- `#message` and `#notice` are read aloud, never drawn (`.unseen`). Every drawn state has a slot in the layout (`#load-slot`), so nothing that appears shifts the page. The panel flips above the caret's line rather than pass the editor's bottom, so it never covers Copy or Clear.
- The TPS key panel's state is the desktop setting `tpsKeyboardShown` in the same document, so a chord and the sidebar row are one value. Its caps are out of the tab order: a keyboard user has the key itself. Clicks and `mousedown` keep focus in the textarea, and a pointer on the panel does not end the composition.
- `#spoken` is the live region for the candidate list, outside `#panel`, whose `hidden` would silence it. Focus never leaves the textarea, so it says one cell rather than the page.
- Copy and Clear are inline Font Awesome Free SVGs (`regular/`, CC BY 4.0), pasted from the npm package with their comment, which is the attribution the licence asks for: keep it. No package and no font: nothing to load, and the CSP stands.
- The card is `#editor`, not the textarea: `border-radius` would take the textarea's corners out of its click area.
- WebKit on a Mac skips buttons on Tab by default, so tab-order tests run in Chromium and Firefox only.
- Hit by a real browser, not by Node: WebKit folds consecutive edits into one undo step, typed ones included, so undo can take back more than the last word.

## Tests

- Native tests cover the session and the key translation, with no lexicon. The engine is one per process, so a test that asserts what it composed takes the `ENGINE` turn and leaves it idle (`taigi-web` `session.rs`, `desktop/crates/taigi-desktop-core/tests/session_language_mode.rs`).
- The smoke test (`web/tests/`) drives the real engine and real dictionaries through the bindings the page loads, one key at a time, with `wasm-bindgen --target web` + `initSync`.
- Expected values come from `engine/composing/tests/` fixtures, whose traces are recorded there. Never from `corpus/`, which feeds no test (`corpus/README.md`).
- Passing in Node says nothing about a browser, a low-end machine or DOM cost. Playwright (`web/e2e/`) sends real keystrokes to the staged site in Chromium, Firefox and WebKit, one test per row of the roadmap's § Text surface.
- A request from a page under a service worker never reaches `page.route`, so a test that routes after a first load uses `test.use({ serviceWorkers: "block" })`. A page in a test is under one only after a reload or a second tab.
- A reload is served by the old worker, so a test of a new version closes every tab first (`offline.spec.mjs`).
- Playwright types on a US layout, so Option and AltGr rewrites never reach it; `key.rs`'s table tests cover those.
- A form control's default is its markup: `selected` or `checked`. `data-boolean` marks a select over a stored boolean, `data-requires` a checkbox that greys, keeping its value, while the one it names is off.
- `#text` keeps a blank placeholder only to drive `:placeholder-shown` for `#text-hint`, which a placeholder could not draw: the `/` keycap, from `data-i18n-keycap`.
- `/` focuses the textarea, without selecting: a selection would let the next key replace the document.
- The session measures the expandable layouts during a key, through `globalThis.taigiMeasureCells` (`Panel.measure`, `layout.rs`; `docs/architecture/web-roadmap.md` § Candidate window width source). `#ruler` sits outside `#panel`: `#panel[hidden]` is `display:none`, and the first key of a word measures before the panel shows. The import is `catch`: an exception through `&mut self` skips its drop and every later call is refused. Nothing it runs may call the session.
- One reply can carry two tables, so `layout` goes on the last one with a highlight. A reader of `reply.rs` should not assume one table per reply.
- The candidate panel's lengths are one formula over `--candidate-size`, so `Settings` sets that one property. `LookupTable` carries `cells`, the two scripts apart; `joined_candidates` is for panels with one text column (IBus, Fcitx5).
