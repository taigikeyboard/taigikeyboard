# Browser Extension

> **Type**: Design record
> **Keywords**: `browser extension`, `manifest v3`, `content script`, `optional_host_permissions`, `execCommand`, `quill`, `wasm`, `content security policy`, `learning`
> **Status**: Chrome first cut 2026-11-13 (issue #864): typing, per-site access, per-tab mode, status and shared learning pass `make -C web extension-e2e` in Playwright's Chromium and in Helium. Firefox not started. Not published.
> **Siblings**: `web-roadmap.md` (the page and the wasm engine this reuses), `chromeos-roadmap.md` (the ChromeOS system input method, a separate product).

---

## Summary

- **Goal**: type Taigi into the fields of existing websites, with the composition and the candidates beside the caret, from a browser extension alone: no native install, no server, no download after install (issue #864).
- **Approach**: a Manifest V3 extension in `web/extension/`. A content script runs the web version's wasm engine in the tab, draws the page's own candidate panel in a shadow tree, and writes commits into the site's field with `execCommand`.
- **Scope of the first cut**: desktop Chrome. Text inputs, textareas, and `contenteditable` editors, held to Quill's Snow editor. The top frame only.

## Decisions

### D1 — The engine runs in the tab

`preventDefault` must run during `keydown`, so "did the engine take this key?" cannot wait for a message (`web-roadmap.md` § Why the main thread). An engine in the service worker or an offscreen document would mean swallowing every key and re-creating the browser's default behaviour in someone else's editor. So each tab that types runs its own instance, as the page does.

Cost: the wasm (4 MB) and the dictionaries (31 MB) are read into every tab that types. A tab loads nothing until typing is turned on in it (`content.js` is the only script a page gets before that), and typing starts off in every tab.

### D2 — The composition never enters the site's document

As on the page, the composition draws in an overlay and the text reaches the field only at commit. A site's editor therefore never sees a half-written word, its undo history holds only committed words, and no `composition*` event is faked.

### D3 — One write: `execCommand`

`document.execCommand("insertText")` is the one call that keeps an input's own undo history and that an editor hears as typing (`beforeinput` / `input`, `insertText`). `web-roadmap.md` § Text surface chose it for the page's first textarea for the same reason. The auto-space swap's delete selects backwards and lets the commit replace the selection, so one engine reply is one undo step; a delete with no commit is `execCommand("delete")`.

A write the field refuses (the call answers false, or an input's value did not change) makes that field unsupported until the focus leaves it: its keys are the site's again, and the refused text goes to the console.

### D4 — Per-site access, asked from the popup

The manifest holds no host permission, only `optional_host_permissions`. The popup's **Use on This Site** asks the browser for `*://<host>/*`; the browser remembers the answer. The service worker registers one content script whose `matches` are the granted sites (`chrome.scripting.registerContentScripts`, kept across sessions) and follows `permissions.onAdded` / `onRemoved`. A page already open when its site is enabled gets the script by `executeScript`.

### D5 — The typing mode belongs to the tab

`chrome.storage.session` holds `{ site, mode, state, label }` per tab id. `mode` is `off`, `taigi` or `english`. The content script says hello with its host on every load; a host other than the stored one answers `off`, and `tabs.onUpdated` forgets a tab that leaves its site, including for a site the extension cannot see. So a reload or another page of the same host keeps the mode (English included, through `Session.toggleLanguageMode`), another site starts off, and so does the way back. A page restored from the back-forward cache says hello again.

### D6 — One learned history, numbered

Every tab's engine has its own SQLite stores in memory, and an open store cannot be handed in again (`learning.js`). The service worker keeps the profile's one copy in IndexedDB with a generation number (`learning-store.js`). A tab hands in the files that changed together with the generation it started from; the check and the write are one transaction, and a write from an older generation is refused. A refused tab, or one that finds a newer generation when it comes back to the front (`focus`, `visibilitychange`), starts a new instance over the latest copy once no composition is open. One keyboard types into one tab at a time, so in practice a tab is always current before it learns.

A private window's content script (`chrome.extension.inIncognitoContext`) neither reads nor sends, and the service worker refuses a private tab's learning messages as a second lock: a private tab learns for its own lifetime only.

### D7 — A JS fallback when the site's CSP forbids WebAssembly

Chromium applies the page's Content Security Policy to `WebAssembly.compile` in a content script: a `script-src` without `'wasm-unsafe-eval'` blocks it (measured 2026-11-13 on GitHub, Reddit and Calmly Writer; X, Facebook, Threads, Wikipedia, Gmail, YouTube, Notion, Dcard and PTT allow it). Dynamic `import()`, `fetch` of the package's files, `DOMParser` under Trusted Types and constructed stylesheets all pass a `default-src 'self'; require-trusted-types-for 'script'` page (`strict.html`).

When `WebAssembly.compileStreaming` throws `WebAssembly.CompileError`, `engine.js` imports `pkg/taigi_web_asm.js` once for the tab and instantiates each session through `pkg/taigi_web_js.js` (`scripts/build-wasm2js.mjs`). The build clears `+multivalue` and `+reference-types` in `taigi_web.wasm`'s `target_features` custom section so `wasm-bindgen` emits one function table and stack out-pointers, lowers bulk-memory copies with `wasm-opt --llvm-memory-copy-fill-lowering`, translates with Binaryen's `wasm2js`, and minifies with `esbuild` — no second `cargo build`.

Measured cost (Apple Silicon, V8, real dictionaries and four SQLite stores):

| Cost | WebAssembly | `wasm2js` fallback |
|---|---|---|
| Normal sites (WASM allowed) | — | zero runtime cost (`taigi_web_asm.js` is never fetched or parsed) |
| Package size | 4.0 MB wasm | +10.8 MB unpacked (+2.85 MB in the store zip) |
| Cold load + first session | ~11 ms | ~430 ms (~200 ms for a later session in the same tab) |
| Dictionary `install()` (31 MB) | ~13 ms | ~16–40 ms |
| Per-keystroke `keyDown` | ~1.3 ms avg (~3.3 ms max) | ~6.9–9.1 ms avg (~11–21 ms max) |
| Four-store learning open + export | ~113 ms | ~500–765 ms (attach and post-commit export run off the key path) |
| Tab memory while loaded | wasm + linear memory | +~35–50 MB of parsed JS in that tab |

## Fields

`field.js` `classify` decides per focused element:

| Element | Kind |
|---|---|
| `<textarea>`, `<input type="text">`, `<input type="search">`, neither read-only nor disabled | `input` |
| a `contenteditable` host in the document | `editable` |
| `<input type="password">`, `email`, `number`, `tel`, `url`; an `<iframe>`; a `contenteditable` host inside a shadow tree | `unsupported` |
| anything else | none |

An unsupported field's keys never reach the engine, a Shift tap included, and the chip and the toolbar say **Field not supported**. A password field is in this list so that its keys are never read.

The composition ends with its field:

| Event | What happens |
|---|---|
| `pointerdown` outside the panel | `commitComposition`, then the click moves the caret or the focus |
| Focus moved by the site or by a key the engine did not take | The composition is dropped (`endSession`): the old field can no longer be written to, and it never follows to the new one |
| The site removes or disables the focused field | Found on blur or by a 500 ms check that runs only while a composition is open; dropped |
| The window loses focus | Nothing is written; the composition continues on return |
| Typing turned off | `commitComposition` |
| Navigation | The instance goes with the page |

## Status

| State | Toolbar badge | Chip beside the field |
|---|---|---|
| Site not enabled, or a page no extension may enter | none, grey icon, the manifest's title | none |
| Off | none, colour icon | none |
| Loading | `…` | Loading |
| Taigi | `台` | the mode and the input script |
| English | `A` | the mode, solid |
| Field not supported | `×` | Field not supported, dashed |
| Could not load | `!` | Could not load, dashed |

The tooltip is the extension's name and the same words. The popup shows them too, with the buttons for the state: Use on This Site, Start Typing, Stop Typing, Stop Using on This Site, and the settings. The chip takes no pointer events.

## Known limits

- **Frames**: the content script runs in the top frame. A field inside an `<iframe>` shows as unsupported.
- **Modal dialogs**: the overlay is a fixed element with the highest `z-index`, not in the top layer. A field inside `<dialog>.showModal()` composes, but the panel draws under the dialog and takes no clicks there.
- **Fonts**: the panel uses the system's fonts; the page's Taigi fonts are not in the package, and the Font row is hidden in the options page.
- **Not drawn**: the TPS key panel and the Telex guide. Their chords change nothing visible.
- **`web_accessible_resources`**: the engine, the dictionaries and the modules are readable by any site, which can therefore tell that the extension is installed.
- **The extension's shortcut** (Alt+Shift+T) toggles typing on an enabled site and opens the popup on one that is not.

## Package

`make -C web extension` stages `web/target/extension/` and zips it: the extension's own files, the page's `panel.js`, `surface.js`, `i18n.js`, `settings.js` and `style.css` as they are, the wasm and its `wasm2js` fallback (`scripts/build-wasm2js.mjs`), the dictionaries uncompressed, and three things `scripts/stage-extension.mjs` cuts from the page so both keep one source: the options form (`chromeos/options-form.mjs` with the extension's list), the panel's markup (`generated/panel-markup.js`), and `overlay.css`, which is `style.css` with `:root` turned into `:host` followed by `extension/overlay.css`.

## Tests

`make -C web extension-e2e` loads the staged extension into Chromium with Playwright (`--load-extension`, a new profile per test) and sends real key events. `EXTENSION_BROWSER=<executable>` runs another Chromium (such as Helium; `launch` waits for any bundled MV2 background page like Helium's uBlock Origin to finish compiling its filter lists before tests navigate); `EXTENSION_PUBLIC=1` adds the smoke test on quilljs.com.

The browser's permission question is a native dialog no harness can answer. The copy under test is therefore given `a.test` and `b.test` in its manifest, `c.test` stays not enabled, and all three resolve to the fixture server. **The grant itself (popup → browser dialog → typing starts) is checked by hand.**

| Issue #864 criterion | Where |
|---|---|
| Real key events through the extension, inputs and textareas | `typing.spec.mjs` |
| Pinned local Quill: document state, formatting, undo / redo | `quill.spec.mjs` |
| Candidates, selection, committed text | `typing.spec.mjs` |
| Caret, selection replacement, switching fields, composition boundaries | `typing.spec.mjs`, `boundaries.spec.mjs` |
| Public Quill smoke | `quill-public.spec.mjs` |
| No network but the fixture | `offline.spec.mjs` |
| Initially off, per-tab mode, reload and same-site retention, cross-site reset, a site not enabled | `tabs.spec.mjs` |
| Toolbar and field status | `status.spec.mjs` |
| Shared learning across tabs and sites, persistence, separate profiles, private-tab client isolation | `learning.spec.mjs`, `tests/extension-learning.test.mjs` |
| Password exclusion, unsupported and disabled fields | `boundaries.spec.mjs` |

Not automated in the browser: the native permission grant dialog and its removal, the keyboard shortcut, an actual incognito window (the extension must be allowed in incognito by hand; `SharedLearning`'s `isPrivate` path is unit-tested in Node), and a modal dialog.

## Open

- Firefox: the same sources should load with a Firefox manifest (`browser_specific_settings`, a background script in place of the service worker); it needs its own harness (Selenium / geckodriver or `web-ext`) and its own pass of every row above.
- Store listing, privacy fields, and the release step.
