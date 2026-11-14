# Browser Extension

> **Type**: Design record + roadmap
> **Keywords**: `browser extension`, `manifest v3`, `content script`, `optional_host_permissions`, `execCommand`, `quill`, `lexical`, `codemirror`, `editcontext`, `hidden field`, `wasm`, `wasm2js`, `content security policy`, `learning`
> **Status**: Chrome Phases 1–2 done 2026-11-14 (issue #864): typing, per-site access, per-tab mode, status, shared learning, CodeMirror 5 / 6 support and canvas / hidden-field detection pass `make -C web extension-e2e` in Playwright's Chromium and in Helium. Firefox not started (§ Phases). Not published.
> **Siblings**: `web-roadmap.md` (the page and the wasm engine this reuses), `chromeos-roadmap.md` (the ChromeOS system input method, a separate product).

---

## Summary

- **Goal**: type Taigi into the fields of existing websites, with the composition and the candidates beside the caret, from a browser extension alone: no native install, no server, no download after install (issue #864).
- **Approach**: a Manifest V3 extension in `web/extension/`. A content script runs the web version's wasm engine in the tab, draws the page's own candidate panel in a shadow tree, and writes commits into the site's field with `execCommand`.
- **Scope**: desktop Chrome (Firefox in Phase 3). Visible text inputs, textareas, CodeMirror 5 (`inputStyle: "textarea"`), and `contenteditable` editors, held to Quill's Snow editor, Lexical's rich-text editor and CodeMirror 6. Canvas and hidden-field editors that draw their own text (Google Docs, Google Sheets, Monaco) are detected as unsupported with their keys untouched. The top frame only.

## Decisions

### D1 — The engine runs in the tab

`preventDefault` must run during `keydown`, so "did the engine take this key?" cannot wait for a message (`web-roadmap.md` § Why the main thread). An engine in the service worker or an offscreen document would mean swallowing every key and re-creating the browser's default behaviour in someone else's editor. So each tab that types runs its own instance, as the page does.

Cost: the wasm (4 MB) and the dictionaries (31 MB) are read into every tab that types. A tab loads nothing until typing is turned on in it (`content.js` is the only script a page gets before that), and typing starts off in every tab.

### D2 — The composition never enters the site's document

As on the page, the composition draws in an overlay and the text reaches the field only at commit. A site's editor therefore never sees a half-written word, its undo history holds only committed words, and no `composition*` event is faked.

### D3 — One write: `execCommand`

`document.execCommand("insertText")` is the one call that keeps an input's own undo history and that an editor hears as typing (`beforeinput` / `input`, `insertText`). `web-roadmap.md` § Text surface chose it for the page's first textarea for the same reason. Commits within one reply (such as a romanized word and its auto space) are joined before `execCommand`, and the auto-space swap's delete selects backwards and lets the commit replace the selection, so one engine reply is one undo step and one `beforeinput`; a delete with no commit is `execCommand("delete")`. Because `beforeinput`-driven editors like Lexical reconcile the DOM in a microtask after `execCommand`, `EditableWrites` adopts the selection after that microtask so its own write is not mistaken for a foreign caret move.

A write the field refuses (the call answers false, or an input's value did not change) makes that field unsupported until the focus leaves it: its keys are the site's again, and the refused text's length goes to the console. Never the text itself: the console belongs to the site's page, and the text is the user's (`security-rules.md`).

### D4 — Per-site access, asked from the popup

The manifest holds no host permission, only `optional_host_permissions`. Ungranted sites and restricted pages keep the manifest's `default_popup` (`popup.html`), whose **Use on This Site** asks the browser for `*://<host>/*`; the browser remembers the answer. Once granted, the service worker clears that tab's popup (`popup: ""`) so left-clicking the toolbar icon toggles typing directly without leaving the page's focused field, and enables **Stop Using on This Site** in the toolbar icon's right-click context menu. The service worker registers one content script whose `matches` are the granted sites (`chrome.scripting.registerContentScripts`, kept across sessions) and follows `permissions.onAdded` / `onRemoved`. A page already open when its site is enabled gets the script by `executeScript` and receives the enabled toolbar state.

### D5 — The typing mode belongs to the tab

`chrome.storage.session` holds `{ site, mode, state, label }` per tab id. `mode` is `off`, `taigi` or `english`. The content script says hello with its host on every load; a host other than the stored one answers `off`, and `tabs.onUpdated` forgets a tab that leaves its site, including for a site the extension cannot see. So a reload or another page of the same host keeps the mode (English included, through `Session.toggleLanguageMode`), another site starts off, and so does the way back. A page restored from the back-forward cache says hello again.

### D6 — One learned history, numbered

Every tab's engine has its own SQLite stores in memory, and an open store cannot be handed in again (`learning.js`). The service worker keeps the profile's one copy in IndexedDB with a generation number (`learning-store.js`). A tab hands in the files that changed together with the generation it started from; the check and the write are one transaction, and a write from an older generation is refused. A refused tab, or one that finds a newer generation when it comes back to the front (`focus`, `visibilitychange`), starts a new instance over the latest copy once no composition is open. One keyboard types into one tab at a time, so in practice a tab is always current before it learns.

A private window's content script (`chrome.extension.inIncognitoContext`) neither reads nor sends, and the service worker refuses a private tab's learning messages as a second lock: a private tab learns for its own lifetime only.

### D7 — A JS fallback when the site's CSP forbids WebAssembly

Chromium applies the page's Content Security Policy to `WebAssembly.compile` in a content script: a `script-src` without `'wasm-unsafe-eval'` blocks it (measured 2026-11-13 on GitHub, Reddit and Calmly Writer; X, Facebook, Threads, Wikipedia, Gmail, YouTube, Notion, Dcard and PTT allow it). Dynamic `import()`, `fetch` of the package's files, `DOMParser` under Trusted Types and constructed stylesheets all pass a `default-src 'self'; require-trusted-types-for 'script'` page (`strict.html`).

When `WebAssembly.compileStreaming` throws `WebAssembly.CompileError`, `engine.js` imports `pkg/taigi_web_asm.js` once for the tab and instantiates each session through `pkg/taigi_web_js.js` (`scripts/build-wasm2js.mjs`). The build clears `+multivalue` and `+reference-types` in `taigi_web.wasm`'s `target_features` custom section so `wasm-bindgen` emits one function table and stack out-pointers, lowers bulk-memory copies with `wasm-opt --llvm-memory-copy-fill-lowering`, translates with Binaryen's `wasm2js`, and minifies with `esbuild` — no second `cargo build`.

This is a capability fallback, not a redundant one (`AGENTS.md` § Design principles): on such a site the WebAssembly path cannot run at all, so there is no "A" to push the capability into. The two paths run the same compiled engine, built from one `taigi_web.wasm`, so they cannot drift. The switch is one `instanceof WebAssembly.CompileError` check in `engine.js`. Any other error, such as a missing file or a failed read, still fails as **Could not load**. A damaged `.wasm` also throws `CompileError`, so it falls back too, and the fallback then types from its own intact copy. The fallback stays as long as sites can set a `script-src` without `'wasm-unsafe-eval'`.

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

`field.js` `classify` decides per focused element, using `visibleBox` (non-empty after ancestor overflow clipping, `visibility: visible`, `opacity > 0`, unclipped `clip` / `clip-path`, visible text or caret on inputs, and intersecting the viewport):

| Element | Kind |
|---|---|
| visible `<textarea>` outside CodeMirror 5, visible `<input type="text">`, visible `<input type="search">`, neither read-only nor disabled | `input` |
| CodeMirror 5's zero-height-clipped `<textarea>` inside a visible `.CodeMirror`, neither read-only nor disabled | `codemirror5` |
| visible `contenteditable` host in the document (outside Google Sheets' grid / formula singleton) | `editable` |
| `<input type="password">`, `email`, `number`, `tel`, `url`; an `<iframe>`; an `EditContext` host (`element.editContext`); Google Sheets' `#waffle-rich-text-editor` / `[docs-unhandledkeys]`; a `contenteditable` host inside a shadow tree; any `<input>`, `<textarea>` or `contenteditable` host whose `visibleBox` is `null` | `unsupported` |
| anything else (including read-only and disabled fields, or `aria-readonly="true"` on `EditContext` / Sheets hosts) | none |

An unsupported field's keys never reach the engine, a Shift tap included, and the chip and the toolbar say **Field not supported**. A password field is in this list so that its keys are never read. When an unsupported field has no visible box of its own, `Overlay.placeChip` walks out to its nearest visible container (such as `.monaco-editor`) or places the chip in the viewport's bottom-right corner when none exists before `<body>` (such as an off-screen `<iframe>`), so the chip never clamps to `(0, 0)`.

### Canvas and hidden-field editors

Measured against the Phase 1 extension (2026-11-14), then resolved per editor:

| Editor | Phase 1 behaviour before Phase 2 | Phase 2 choice |
|---|---|---|
| **CodeMirror 6** (`@codemirror/view`) | `.cm-content` is a visible `contenteditable="true"` host on desktop (`EditContext` in CM6 is Android-only). Commits, selection replacement and `history()` undo / redo already worked via `EditableWrites`. **Bugs**: `.cm-content` shrinks or grows to document height inside `.cm-scroller`, misplacing `#chip` when short or overflowing; and on macOS `DOMObserver` rewrote a `. ` auto-space swap into two spaces (`"tâi-uân  "`) because `selectionchange` had not fired before `execCommand` | **Supported** (`editable`, `codemirror6.spec.mjs`). `EditableWrites.deleteBefore` dispatches `selectionchange` so `DOMObserver` sees a selection replacement, `classify` anchors `#chip` to `.cm-editor`, and `Overlay.placeChip` clips by `visibleBox` |
| **CodeMirror 5** (`codemirror` 5, default `inputStyle: "textarea"`) | Focuses a `1000 px × 1 em` `<textarea>` clipped inside a `3 px × 0 px` `overflow: hidden` wrapper under `.CodeMirror`. `TextareaInput.poll()` diffs `textarea.value` against `prevInput`, so `execCommand("insertText")`, selection replacement, auto-space swap and CM5 undo / redo all worked. **Bugs**: `InputSurface` measured the hidden `<textarea>` in its browser-default font, and `#chip` anchored at `x = 1200` off `.CodeMirror`'s right | **Supported** (`codemirror5` / `CodeMirror5Surface`, `codemirror5.spec.mjs`). Reads `.CodeMirror-cursor` / `.CodeMirror-selected` clamped to `.CodeMirror-scroll`, takes `.CodeMirror-code`'s font, includes cursor style in `snapshot()`, and anchors `#chip` to `.CodeMirror`. Visible `<input>`s inside `.CodeMirror` (such as `addon/dialog`) stay `"input"` |
| **Monaco** (`monaco-editor`) | In Chromium's default `editContext: true` mode, focuses a `0 px`-wide `div.native-edit-context` (`element.editContext` set, `isContentEditable === false`): `classify` answered `null`, leaving the toolbar on `台` with no chip while keys bypassed the engine. With `editContext: false` (and in Firefox), focuses a `1 px` transparent `textarea.inputarea`: `classify` answered `"input"`, drawing the panel at the parked `.inputarea` while `.inputarea` resets `value = ""` on Windows / Linux so `deleteBefore` has no text | **Detected and unsupported** (`hidden.spec.mjs`). `EditContext` violates D3, splits auto-space swap undo steps in Monaco's `NativeEditContext`, and does not exist in Firefox (§ Phases). Both `element.editContext` and `visibleBox === null` mark Monaco **Field not supported** (`×`), anchoring `#chip` under `.monaco-editor` |
| **Google Docs** (`docs.google.com/document/`) | Body renders on `<canvas class="kix-canvas-tile-content">` and focuses `iframe.docs-texteventtarget-iframe` parked at `top: -10000px` under `<body>` (its inner `contenteditable` rejects `execCommand("insertText")` and holds no preceding text). `classify` marked the `<iframe>` unsupported, **but** `placeChip` clamped `top: -10000px` to `(0, 0)` over the menu bar | **Detected and unsupported** for the canvas body (`hidden.spec.mjs`): keys untouched, `#chip` placed in the viewport's bottom-right corner. Top-frame fields (document title, Find & Replace, comment `contenteditable`) remain supported |
| **Google Sheets** (`docs.google.com/spreadsheets/`) | Cells render on `<canvas>`; focus rests on singleton `div#waffle-rich-text-editor` (`[docs-unhandledkeys]`) inside `.input-box` parked at `top: -10000px` with `contenteditable="false"` while navigating. Phase 1 answered `null` when `"false"` (toolbar `台`; typing on a cell sent key 1 raw to open edit mode and intercepted key 2+, corrupting `"taiuan"` into `"t"` + `"aiuan"`) and clamped `#chip` to `(0, 0)` when `"true"` off screen | **Detected and unsupported** (`hidden.spec.mjs`): `#waffle-rich-text-editor` / `[docs-unhandledkeys]` is unsupported in both navigation and cell-edit states, anchoring `#chip` beside the cell while editing and returning it to the viewport corner on `Enter`. Top-frame `<input>`s remain supported |

The composition ends with its field:

| Event | What happens |
|---|---|
| `pointerdown` outside the panel | `commitComposition`, then the click moves the caret or the focus |
| Focus moved by the site or by a key the engine did not take | The composition is dropped (`endSession`): the old field can no longer be written to, and it never follows to the new one |
| The site removes, disables or hides the focused field | Found on the next key, on blur, or by a 500 ms check that runs only while a composition is open; dropped |
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

The tooltip on an enabled tab is the extension's name, the same words, and the active shortcut (`chrome.commands`) when assigned; on an unenabled or restricted page it keeps the manifest's title. The popup shows them too, with the buttons for the state (Use on This Site, Start Typing, Stop Typing, Stop Using on This Site, with the live shortcut on the toggle/enable buttons) and the settings. Stop Using on This Site is also on the toolbar button's right-click context menu when the active tab's site is enabled. The chip takes no pointer events.

## Known limits

- **Frames**: the content script runs in the top frame. A field inside an `<iframe>` shows as unsupported.
- **Canvas and custom-input editors**: Google Docs' document canvas, Google Sheets' grid and formula bar, Monaco (`EditContext` and hidden `<textarea>`), Ace and xterm.js show **Field not supported** and keep their keys (§ Fields).
- **Modal dialogs**: the overlay is a fixed element with the highest `z-index`, not in the top layer. A field inside `<dialog>.showModal()` composes, but the panel draws under the dialog and takes no clicks there.
- **Fonts**: the panel uses the system's fonts; the page's Taigi fonts are not in the package, and the Font row is hidden in the options page.
- **Not drawn**: the TPS key panel and the Telex guide. Their chords change nothing visible.
- **`web_accessible_resources`**: the engine, the dictionaries and the modules are readable by any site, which can therefore tell that the extension is installed. Fix tracked in #902 (`use_dynamic_url`).
- **The extension's shortcut** (Alt+Shift+T) toggles typing on an enabled site (matching a left-click on the toolbar button) and opens the popup on one that is not.

## Package

`make -C web extension` stages `web/target/extension/` and zips it: the extension's own files, the page's `panel.js`, `surface.js`, `i18n.js`, `settings.js` and `style.css` as they are, the wasm and its `wasm2js` fallback (`scripts/build-wasm2js.mjs`), the dictionaries uncompressed, and three things `scripts/stage-extension.mjs` cuts from the page so both keep one source: the options form (`chromeos/options-form.mjs` with the extension's list), the panel's markup (`generated/panel-markup.js`), and `overlay.css`, which is `style.css` with `:root` turned into `:host` followed by `extension/overlay.css`.

## Tests

`make -C web extension-e2e` loads the staged extension into Chromium with Playwright (`--load-extension`, a new profile per test) and sends real key events. `EXTENSION_BROWSER=<executable>` runs another Chromium (such as Helium; `launch` waits for any bundled MV2 background page like Helium's uBlock Origin to finish compiling its filter lists before tests navigate); `EXTENSION_PUBLIC=1` adds the smoke test on quilljs.com.

The browser's permission question is a native dialog no harness can answer. The copy under test is therefore given `a.test` and `b.test` in its manifest, `c.test` stays not enabled, and all three resolve to the fixture server. **The grant itself (popup → browser dialog → typing starts) is checked by hand.**

| Issue #864 criterion | Where |
|---|---|
| Real key events through the extension, inputs and textareas | `typing.spec.mjs` |
| Pinned local Quill, Lexical, CodeMirror 5 and CodeMirror 6: document state, caret / chip placement, selection replacement, auto-space swap, undo / redo | `quill.spec.mjs`, `lexical.spec.mjs`, `codemirror5.spec.mjs`, `codemirror6.spec.mjs` |
| Canvas and hidden-field editors (`EditContext`, 1 px transparent / zero-opacity / clipped `<textarea>`, off-screen `<iframe>`, Google Sheets singleton, dynamic visibility) | `hidden.spec.mjs` |
| Candidates, selection, committed text | `typing.spec.mjs` |
| Caret, selection replacement, switching fields, composition boundaries | `typing.spec.mjs`, `boundaries.spec.mjs` |
| Public Quill smoke | `quill-public.spec.mjs` |
| No network but the fixture | `offline.spec.mjs` |
| Initially off, per-tab mode, 1-click toolbar toggle preserving field focus, reload and same-site retention, cross-site reset, a site not enabled, `removeSiteAccess` | `tabs.spec.mjs` |
| Toolbar and field status, shortcut tooltip and popup badge, per-tab `getPopup`, action context menu state | `status.spec.mjs` |
| Shared learning across tabs and sites, persistence, separate profiles, private-tab client isolation | `learning.spec.mjs`, `tests/extension-learning.test.mjs` |
| Password exclusion, unsupported and disabled fields | `boundaries.spec.mjs` |
| Strict CSP + Trusted Types (`wasm2js` fallback, Lexical, expandable layout, cross-tab learning, mode retention) | `strict.spec.mjs`, `tests/wasm2js.test.mjs` |

Not automated in the browser: the native permission grant dialog and its removal (E2E verifies `removeSiteAccess` stops typing before `permissions.remove`), the keyboard shortcut, an actual incognito window (the extension must be allowed in incognito by hand; `SharedLearning`'s `isPrivate` path is unit-tested in Node), and a modal dialog.

## Phases

One PR per phase. Issue #864 makes Chrome and Firefox one release: neither ships until both pass every row of § Tests. No version is assigned.

| # | Phase | Content | Status |
|---|---|---|---|
| 0 | Design | Issue #864: decisions and acceptance criteria | Done |
| 1 | Chrome | This document's D1–D7; `make -C web extension`, `extension-e2e` in `web.yml` | Merged (#901) |
| 2 | Canvas and hidden-field editors | Recorded Google Docs, Google Sheets, Monaco and CodeMirror 5 / 6 (§ Fields): CodeMirror 5 (`CodeMirror5Surface`) and CodeMirror 6 supported with E2E rows in § Tests; `EditContext`, zero-size / clipped / transparent / off-screen fields, Google Docs' body `<iframe>` and Google Sheets' grid singleton detected as unsupported with container or corner chip placement (`hidden.spec.mjs`) | Done |
| 3 | Firefox | A Firefox manifest from the same sources: `browser_specific_settings`, `background.scripts` in place of the service worker. Its own harness, because Playwright cannot load a Firefox extension (`web-ext` or geckodriver), and its own pass of every row of § Tests. Check in Firefox, not by analogy with Chromium, whether a page's CSP blocks `WebAssembly.compile` in a content script (D7). Firefox keeps its own learned history (#864) | Not started; blocks the release |
| 4 | Other Chromium browsers | Edge, Brave, Vivaldi, Opera, Arc: the hand-checked rows (permission grant, shortcut, private window) once per browser. Whether to list in the Edge Add-ons store is the maintainer's call | Not started |
| 5 | Packaging + store | A release doc in the shape of `chromeos-release.md`: version on the desktop train, the zip built in CI, store listing, privacy fields, screenshots. Developer accounts and submission are the maintainer's | Not started |

### Open questions for the maintainer

- **Safari**: not in #864. A Safari web extension ships inside a Mac / iOS app made in Xcode, and only the maintainer edits Xcode projects (`AGENTS.md` Core Principle 1). Whether to start it, and when, is the maintainer's call.
- **Detection by sites**: issue #902, `"use_dynamic_url": true` (§ Known limits, `web_accessible_resources`). Needs a higher `minimum_chrome_version` and a full extension e2e pass.
