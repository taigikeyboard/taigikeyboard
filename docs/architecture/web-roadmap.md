# Web Version — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `web`, `browser`, `wasm`, `wasm-bindgen`, `PWA`, `desktop-core reuse`, `McBopomofoWeb`, `kautian-mirror`
> **Status**: Accepted 2026-11-06. Phases 0 to 7 done; live at `taigikeyboard.tw/web/`. Decisions D1–D6 settled by the maintainer as recommended (§ Decisions). Reviewed once by a fresh-context same-model agent, findings applied; Codex skipped (no subscription, maintainer 2026-11-04).
> **Siblings**: `linux-roadmap.md` (the closest shell), `system-overview.md`, `data-artifacts-portability.md`.

---

## Summary

- **Goal**: TaigiKeyboard in a browser tab, nothing to install, for shared computers, classrooms, Chromebooks and anyone who wants to try it. TL, POJ and TPS with the same candidates as the desktop apps.
- **Approach**: compile the existing Rust (`dispatch` + `taigi-desktop-core`) to `wasm32-unknown-unknown` and write a thin browser shell over it. No TypeScript port of the engine.
- **Proven 2026-11-06** by a throwaway spike (§ Spike): the real engine and real dictionaries run in WASM, top candidates match native on 200 of 200 sentences, and a key costs about 1.6× native.
- **What it is not**: a system input method. Text is typed into the page and copied out. A browser tab cannot type into other apps.
- **UI references**: input flow from McBopomofoWeb, colours and sidebar from kautian-mirror.

## Spike results (grounded in code)

A throwaway copy of `engine/` + `desktop/` outside the repository, no repository file changed. One harness crate drove `ComposingManager::append` + `fetch_candidates` for each key, the same two calls a shell makes per key, with real `assets/dictionaries/`.

Input: 200 real sentences from `corpus/taigi-typing`, tones and hyphens stripped, typed as one continuous composition (9,572 keys, longest 220 characters). That is the worst case the engine sees.

| Measure | WASM (Node 26) | Native | Note |
|---|---|---|---|
| Records installed | 167,986 | 167,986 | |
| Top candidate on all 200 sentences | identical | baseline | byte comparison of both outputs |
| Key p50 / p95 / p99 / max | 1.38 / 8.95 / 17.72 / 24.58 ms | 0.88 / 5.88 / 11.58 / 15.05 ms | about 1.6× |
| Instantiate + copy dictionaries + install | 3 + 12 + 2.6 ms | — | after the bytes are downloaded |
| Linear memory | 32 MB | — | dictionaries are 29.6 MB of it |
| `.wasm` size | 2.3 MB, 726 KB gzip | — | before `wasm-opt`, without `wasm-bindgen` glue |

Download size of the dictionaries:

| File | Raw | gzip -9 | brotli 11 |
|---|---|---|---|
| `dictionary.fst` | 15 MB | 9.67 MB | 7.71 MB |
| `dictionary.bin` | 8.6 MB | 2.97 MB | 2.30 MB |
| `association.bin` | 6.0 MB | 2.55 MB | 1.66 MB |
| `syllables.fst` | 17 KB | 8 KB | — |
| **Total** | **29.6 MB** | **15.2 MB** | **11.7 MB** |

The spike did not measure a real browser, DOM cost, or the full `process_key` path with its JSON reply, which carries the whole candidate list on every key (`desktop/crates/taigi-desktop-core/src/session/table.rs` `table_content`). Phase 4 did, around the page's whole `keydown` listener (engine, JSON, DOM), on an Apple M4. Words from `engine/composing/tests` fixtures, typed as short compositions (392 keys) and as one continuous composition of about 100 characters (633 keys):

| Browser | Short p50 / p99 / max | Long p50 / p99 / max |
|---|---|---|
| Chromium | 0.6 / 3 / 22 ms | 5.7 / 13.4 / 14.8 ms |
| Firefox | 1 / 3 / 20 ms | 7 / 16 / 17 ms |
| WebKit | 1 / 2 / 7 ms | 5 / 13 / 13 ms |
| Chromium, CPU throttled 6× | 3.2 / 7.8 / 90.6 ms | 34.4 / 84.9 / 89 ms |

Firefox and WebKit round their timers to 1 ms. The throttled row stands in for a slow machine; no low-end Chromebook was at hand. A 48 KB document adds about 8 ms a key, the mirror measuring the text before the caret (p50 7 to 9 ms, p99 10 to 19 ms). Nothing here asks for bounding the fetch.

## What must change in shared code (grounded in code)

`cargo check --target wasm32-unknown-unknown -p taigi-desktop-core` passed before Phase 1. It compiled, and these four things failed at run time. The spike hit 1 and 2, and replaced 3 and skipped 4 before running.

| # | Where | What happens on WASM | Fix |
|---|---|---|---|
| 1 | `engine/mmap-host/src/lib.rs:45,56` before the split, now `native.rs` | `File::open` and `Mmap::map` have no file system to read | A `cfg(target_family = "wasm")` backend: `MmapHandle::open_readonly` looks the path up in a registry of `Arc<Vec<u8>>`. Nothing needs `'static`: each reader owns its handle. The `lexicon` readers and `InstallRequest` stay as they are. `memmap2` becomes a non-wasm dependency |
| 2 | `engine/lexicon/src/paths.rs:62` | `Path::is_absolute()` is `false` for every path here, so install failed with 0 records. Passing the check with `has_root()` fixed it | Accept `has_root()` on wasm only; Windows keeps `is_absolute` |
| 3 | `desktop/crates/taigi-desktop-core/src/composing/clock.rs:16`, read on every fetch (`composing/manager.rs:383`) | `SystemTime::now()` is not implemented on this target. The spike replaced the clock rather than letting it panic | `RuntimeParts` takes an `Arc<dyn Clock>` (`runtime.rs:26`, an `Arc` because `coordinator()` builds lazily from `&self`), passed through `coordinator()` (`runtime.rs:140-143`) and `for_desktop` (`composing/coordinator.rs:62-66`) to `ComposingManager::new`. Native shells pass `SystemClock` |
| 4 | `desktop/crates/taigi-desktop-core/src/dictionary_artifacts.rs:41` | `std::fs::metadata` always fails, so `locate` refuses | No change. The shell passes `dictionaries: || None` (`runtime.rs:36`) and calls `engine::lexicon_install` itself once the bytes are in |

Already fine:

- `DesktopPlatform` needs no `Web` variant. It is "a value the shell passes in" (`platform.rs:1-3`), so the page passes `MacOS` on a Mac and `Windows` or `Linux` elsewhere, and gets the right caret chord (`keys/intent.rs:61-66`).
- `dispatch` without `user-data` is a designed mode (`dispatch/src/user_data.rs:1-7`), and `data_directory: None` means "nothing is learned" (`runtime.rs:33-34`).
- `Instant::now` in `dispatch` is behind `e2e-trace` only (`dispatch/src/lib.rs:59`).

One thing does not hold and is a named divergence: `process_request` relies on `catch_unwind` (`dispatch/src/lib.rs:61`, `rust-ffi-safety.md` §1.2), and this target aborts on panic. A panic traps the instance. The page catches the trap, starts a new instance and loses the composition, never the text already in the editor.

Written rules changed in Phase 1, in the same PR as the code:

| Rule | Today | Change |
|---|---|---|
| `rust-best-practices.md:101` | "No web-target builds (`wasm32-*`). Future consideration" | Removed; web joins the shell list |
| `rust-best-practices.md:99` | Process-wide state only in a domain crate's `handle.rs` | Names the `mmap-host` wasm registry as an install-time exception |
| `rust-ffi-safety.md` §1.2 | Every boundary unwinds | Names `taigi-web` as the divergence below |

`rust-ffi-safety.md:95` needed no change: `#[wasm_bindgen]` compiles under `unsafe_code = "forbid"`.

## Architecture

```text
Browser tab, main thread
┌───────────────────────────────────────────────────────────┐
│ web/src (ES modules, esbuild)                             │
│   ime.js       keydown / keyup → engine → preventDefault  │
│   surface.js   textarea writes, caret position            │
│   panel.js     preedit + candidate list                   │
│   sidebar.js, settings.js   (kautian-mirror's)            │
│   loader.js    fetch, verify, cache dictionaries          │
└──────────────────────────┬────────────────────────────────┘
                           │ wasm-bindgen, JSON reply per key
┌──────────────────────────▼────────────────────────────────┐
│ web/crates/taigi-web (cdylib, wasm32-unknown-unknown)     │
│   browser key → KeyEventSnapshot, settings from JSON      │
├───────────────────────────────────────────────────────────┤
│ taigi-desktop-core::session      process_key → Emit list  │
│ taigi-desktop-core               classifier, composing    │
│ engine: dispatch · composing · lexicon · phonetics …      │
│ dictionaries: 29.6 MB in linear memory                    │
└───────────────────────────────────────────────────────────┘

Offline: service worker + Cache Storage
```

### Why the main thread, not a Worker

`preventDefault()` must be called during `keydown`. The answer to "did the engine take this key?" cannot wait for a `postMessage`, so a Worker would mean swallowing every key and re-creating the browser's default behaviour by hand. McBopomofoWeb calls its controller synchronously for this reason (`output/example/index.js:1295-1303`).

The spike's p99 of 18 ms says the main thread can afford it. If a slow machine proves otherwise, the fix is bounding the fetch for very long compositions, not moving the engine.

### Why one reply per key, as JSON

The Linux shell already answers a key with a list of signals: `KeyReply { handled, emits }` (`desktop/crates/taigi-desktop-core/src/session/state.rs`, `session/executor.rs`). `Emit` maps onto a browser one to one:

| `Emit` | Browser |
|---|---|
| `Preedit { text, caret }` | Draw the composition in the panel |
| `ClearPreedit` | Clear it |
| `Commit(text)` | Insert at the textarea caret |
| `DeleteSurrounding` | Remove the auto space before the caret |
| `LookupTable` / `HideLookupTable` | Render or hide the candidate list |
| `ModeChanged` / `AnnounceMode` | `AnnounceMode` reaches the hidden live region and draws the mode flash, a card held 0.8 s as on the desktops; the sidebar shows the mode |
| New in Phase 2: `OpenSettings`, `ToggleTpsKeyboard` | The settings chord opens or closes the sidebar, the page's settings (maintainer 2026-11-07), one toggle per press. Show the TPS key panel |
| New in Phase 5: `TpsKeyboardCapTyped` | Light the cap. Linux drops it in `run_launches`: its panel is another process |

`serde_json` is already a dependency of `taigi-desktop-core`. Proto bytes would pull a protobuf runtime into the page for no gain.

### Text surface

A plain `<textarea>`. The composition is drawn in an overlay and nothing reaches the textarea until commit, so the undo stack only holds committed words.

Commit uses `document.execCommand("insertText")`, which MDN marks deprecated and which is the only call that keeps the native undo history. McBopomofoWeb assigns `textArea.value` (`index.js:235-245`), which wipes that history. One write path, no fallback (`AGENTS.md` § No redundant fallback): Phase 4 tests it in Chromium, Firefox and WebKit, and if a browser refuses it, that test decides the single call the page uses.

`execCommand` only works while the textarea has focus, so **every commit happens before focus leaves**:

| Event | What the page does |
|---|---|
| `pointerdown` on the textarea, composition running | `session.commitComposition()`, then the click moves the caret. Without this the composition would land at the new caret |
| `pointerdown` elsewhere on the page | Same commit. `pointerdown` runs before `blur` |
| `mousedown` on the candidate panel, Copy, Clear | `preventDefault`, so focus stays. Copy and Clear commit first |
| `blur` with no pointer involved (another window or tab) | Nothing is written. The composition stays on screen and continues on return |
| Caret moved or text changed by anything but this page (arrows, mouse, paste, undo) | `session.caretMoved()`, found at the next `keydown` by comparing caret and text with what the page's last write left. `selectionchange` is not used: the page's own `execCommand` fires it too, and it arrives asynchronously |
| Window loses the keyboard | `session.focusLost()`: a key held across that is released elsewhere |
| Any `pointerdown` on the page | Also `session.caretMoved()`, so a Shift-click on a control with no caret is not a tap |
| Textarea `scroll` | Re-anchor the panel at the caret |

The `caretMoved` row prevents real text loss. The auto-space swap deletes the character before the caret without looking at it (`Emit::DeleteSurrounding { offset: -1, count: 1 }`, `session/executor.rs:97-100`). Its armed state clears only on the next key or at session end, so click elsewhere, type `.`, and a real character goes.

`EditContext` is left out: MDN lists it as limited availability and experimental.

### Browser key rules

From McBopomofoWeb:

- Skip while `event.isComposing` or `keyCode === 229`, and show a notice that another input method is active (`index.js:1245-1282`).
- Shift tap switches to English on `keyup` (`index.js:1261-1271`). Ours uses the shared tracker in `keys/shift_tap.rs`.

Different from McBopomofoWeb, which resets on `Alt` (`index.js:1278-1281`): **only `Meta` chords stay with the browser.** Everything else goes to the classifier, because seven of the eight default shortcuts are Ctrl+Alt (`keys/shortcut_actions.rs:224-232`) and ⌥ is the Mac caret chord (`keys/intent.rs:64`).

`KeyEventSnapshot` (`keys/snapshot.rs:88`) is filled in `taigi-web` from `event.key`, `event.code` and the modifiers:

- `characters_ignoring_modifiers` is `event.key`, so a Dvorak key reads as what it types. Only where Alt rewrote the key (non-ASCII or `Dead`) does `event.code` decide, and that differs per platform. On a Mac the position gives the character back, because Option reports `ß` for `s`. Elsewhere the rewrite is AltGr text and Control and Alt are dropped, so Polish AltGr+C types `ć` rather than firing Ctrl+Alt+C. Two costs: under a non-Latin layout the Ctrl+Alt chords do not fire off the Mac, and on a Mac the Control+Option chords read by US position, so they miss on Dvorak.
- `key_code` is read by `chord.rs:72,179` and `tps_layout.rs:100` for the keypad and shifted digits. `event.code` maps into the code space those read. `Semicolon` carries a code only when the key types `;` or `:`, because Dvorak's `s` sits there.
- `Meta` is the core's `win`, so the classifier hands the chord to the browser and commits a running composition first, as desktop does.
- A bare `Dead` key is a named key, so it ends a composition before the browser starts its own.
- `web/crates/taigi-web/src/key.rs` tests each mapped entry. Every default chord is Ctrl+Alt or the bare backtick, none of them in the set a page never receives (Ctrl+T, Ctrl+W, Ctrl+N). Phase 4 proves that in real browsers.

## UI (from kautian-mirror)

```text
┌──┐
│≡ │  sidebar toggle, top left                              
└──┘
          台語齒盤                              Tâi-lô ▾
          ┌─────────────────────────────────────────┐
          │ Guá sī Tâi-uân-lâng                     │
          │          ┌──────────────────┐           │
          │          │ tsiahpng         │ composition
          │          │ 1 食飯  2 食 …   │ candidates
          │          └──────────────────┘           │
          └─────────────────────────────────────────┘
                                      Copy     Clear
```

Taken as they are from the kautian-mirror repository (`taigikeyboard/kautian-mirror`, paths below are relative to it):

| Element | Source | Use |
|---|---|---|
| Colour tokens with `light-dark()` | `src/styles.css:45-61` | `--bg`, `--card`, `--text`, `--secondary`, `--border`, `--hairline`, `--hover`, `--accent`, `--ring` unchanged. The candidate highlight uses `--accent` |
| Theme pin | `src/styles.css:41-43` | Automatic, light, dark through `<html data-theme>` |
| Sidebar | `src/styles.css:125-167`, `src/sidebar.js` | 21rem, docked above 50rem, `popover` drawer below, open by default on wide screens, closed on narrow ones |
| Settings to `data-*` | `src/settings.js`, `src/index.html:23-36` | One form, restored before first paint so a reload does not flash |
| Fonts | `src/styles.css:3-38`, `scripts/subset-fonts.sh` | Same roster, CJK-only `unicode-range` |
| Tool chain | `esbuild.config.mjs`, `node --test` | No framework |

The `unicode-range` is required, not cosmetic: those fonts misplace combining tone marks (`tsi̍t` renders as `tsit`), so romanization must fall through to the system font.

Sidebar content follows the desktop settings panes, as sections that fold: General, Appearance and Manage Dictionaries, with the labels and row order of `GeneralSettingsView.swift`, `AppearanceSettingsView.swift` and `settings/dictionary_sources.rs`. Below them, links to the website, the converter and kautian-mirror. Labels come from `i18n/` by their existing keys. The title and Copy, Clear are fixed text.

Around the textarea the page draws Copy and Clear only: no mode control, no standing status text. Loading is a bare progress bar, a failure a Try Again button. `#message` and `#notice` stay as hidden live regions for screen readers. A mode switch, a Shift tap to English included, draws the mode flash inside the editor for 0.8 s (maintainer 2026-11-07).

Copy follows the project's plain-prose rules, and stays neutral between romanization systems.

### Candidate window width source

`ExpandableListModel` packs from widths, and everything a key reads comes out of that packing: which cell a slot key names, whether there is more to unfold, where an arrow lands. The labels of the same key read it too (`session/table.rs`), so the model must exist before the reply, and the page measures during the key. The session calls a `ListLayout` from `present_table`, which calls the page through one synchronous import.

Measured 2026-11-07 on an Apple M4, 200 real cells:

| Ruler | Chromium | Chromium, CPU throttled 6× | Firefox | WebKit |
|---|---|---|---|---|
| Canvas `measureText`, p50 / max | 0.1 / 6.7 ms | 0.6 / 2.5 ms | <1 / 7 ms | <1 / 8 ms |
| Hidden DOM ruler, p50 / p95 / max | 1.2 / 2.1 / 4.6 ms | 6.9 / 12.8 / 25.4 ms | 2 / 3 / 20 ms | 2 / 4 / 61 ms |
| Canvas against what is drawn | ≤0.01 px | same | 0 | up to 2.76 px short under Iansui, 384 of 1,474 strings after `ceil` |

So the hidden DOM ruler: what it reads is what is drawn, and only the two expandable layouts pay for it. Loading a font moves no width (Hanji is 1 em in every face, Latin falls to the system font by `unicode-range`), so nothing listens for it; the exception is WebKit under Iansui, where the next key measures again. The window's width is read at each list, so a resize with a list open takes effect at the next key.

## Behaviour against the desktops

Same as desktop, because it is the same code: the key table, continuous input, candidates and paging, full-width punctuation, auto-space, dictionary toggles, Hanji and romanization output.

Named divergences (`cross-platform-alignment.md` §3), all intentional unless marked:

| Behaviour | Web | Why |
|---|---|---|
| Next-word prediction | None, same as desktop | Physical keyboard; maintainer 2026-10-03 |
| Target of typing | The page's textarea only | A tab cannot reach other apps |
| Global shortcuts | The Ctrl+Alt set of Windows and Linux, minus what the browser keeps. On a Mac the page takes the app's ⌃⌘ set and lists it (`web/crates/taigi-web/src/key.rs` `core_tier_snapshot`), ⌃⌥ does nothing there; a ⌃⌘ press that is no chord stays the browser's. The settings chord toggles the sidebar | One set of defaults in Rust; the Mac app's rows live in Swift |
| English mode by Shift tap | Yes, as on Windows and macOS. **Linux has none** (`linux/crates/taigi-linux-core/src/chrome.rs` `mode_indicator`), so the gate is opt-in per shell | A page has no OS input-method switcher |
| Bare Hanji/romanization key in English mode | Types a backtick. Windows still fires it, because its global match runs above the English gate (`session.rs:168-183`, `:204`) | English mode means the key is the document's, which is what the Windows comment says too |
| Candidate click | Selects, never commits, as on Linux and macOS (`session/key.rs` `click_from_panel`) | Same code |
| Keyboard Layout setting | No row | `event.key` already follows the OS layout |
| Candidate window | macOS Tahoe's look from `CandidateMetrics.swift`, unrounded lengths. The composition sits in the panel's first row; a column pages by nine rather than scrolling with a half-row peek, so the key labels stay the session's; a row wraps rather than paging by width; the highlight is `--accent`, not the system accent; the expandable grid has no scrollbar | A textarea has no inline marked text; `LookupSelection` owns paging of the two paged layouts, `ExpandableListModel` the two expandable ones, as on Windows |
| Panic | Trap, then a new instance | No unwinding on this target |
| Learning | Phase 8: on by default, as every other platform; one tab learns at a time | § Decisions D7–D9 |
| Custom Dictionary, `.taigi` backup | None yet | Maintainer 2026-11-07: learning first |

## Phases

Each phase is one PR of 200–500 LOC unless noted. No phase is assigned to a release.

| # | Phase | Gate | Status |
|---|---|---|---|
| 0 | This roadmap + project memory | admin lane | Done |
| 1 | Engine runs on wasm32 | `make web-check`, native suites unchanged | Done |
| 2 | Shared session module | behaviour freeze, `make linux-check` | Done |
| 3 | `taigi-web` crate | native + Node tests | Done |
| 4 | Plain page, end to end | Playwright in 3 browser engines | Done; iPad and AltGr device checks open |
| 5 | Look, then i18n + fonts, then panels + a11y (3 PRs) | Playwright, i18n gates | Done |
| 6 | Loading and offline | Playwright offline run | Done |
| 7 | Deploy | `check-live` on the real URL | Done |
| 8 | User data: learning on (8a); Learning Records section (8b) dropped | spike done 2026-11-07; native suites unchanged, Node + Playwright | 8a merged (#756); 8b dropped 2026-11-07 |

### Phase 1 — Engine runs on wasm32

Table rows 1–3 above, plus the plumbing a new platform needs. Row 3 adds one field at each `RuntimeParts` site: `windows/crates/taigi-windows-tsf/src/runtime.rs:69`, `linux/crates/taigi-linux-core/src/runtime.rs:115`, `macos/crates/taigi-macos-ffi/src/runtime.rs:173`, and the test helper at `desktop/crates/taigi-desktop-core/src/runtime.rs:250`.

- The three rule changes in § What must change.
- `web/` Cargo workspace with its own `rust-toolchain.toml` (target `wasm32-unknown-unknown`), the `linux/` shape. A minimal `taigi-web` crate that links `taigi-desktop-core`.
- Pin `wasm-bindgen` and `wasm-bindgen-cli` to one exact version (`mise.toml`), plus `binaryen` for `wasm-opt`. The smoke test needs bindings, and hand-written alloc exports would need `unsafe`. Verify the current release when this phase starts.
- `make web-check`: the wasm build plus a Node smoke test that installs the dictionaries and asserts top candidates for a fixed input list.
- `tools/test_select.py`: a `web` platform, selected in the `DESKTOP_LINKED_CRATE` branch (line 381). `WIRE_SURFACE_CRATES` is the wrong hook: it leaves out `lexicon` and `composing`, which web links.
- The sixth Cargo workspace everywhere the five are named: `.github/workflows/security.yml`, `build-artifacts.md` § Six Cargo workspaces, `rust-best-practices.md:81,87`.
- `.github/workflows/web.yml`. `AGENTS.md` Build & Test row and project structure. `docs/contributing/web-guidelines.md` with its `.claude/rules/` pointer. JS lint in `make lint`. `REVIEW.md` if it needs a tier.
- A McBopomofoWeb card in `docs/references/mainstream-ime-comparison.md`, which has none today.

The smoke test's input list cannot come from `corpus/taigi-typing` (CC BY-ND, and `corpus/README.md` says nothing there feeds a test). Take it from `engine/composing/tests` fixtures.

Check: `make test`, `make desktop-check`, `make linux-check` unchanged. Native behaviour must not move.

### Phase 2 — Shared session module

`taigi-linux-core` had the session logic the web needs, about 1,460 non-test lines across `session.rs`, `chrome.rs`, `executor.rs` and `selection.rs`. It could not be linked as it stood: it depends on `taigi-desktop-storage`, `taigi-linux-platform` and `dispatch/user-data`. The coupling was thin, though. `Runtime` only derefs to `DesktopRuntime`, and the platform ties are a few imports.

Move the framework-free part into `taigi-desktop-core` as a `session` module. No new crate: its dependencies already fit, and a crate would add a seam with one owner.

- `Emit`, `Recorder`, `LookupSelection`, `EngineState`, table presentation, `process_key`, `perform_global`, `end_session`, the panel entry points, over `&DesktopRuntime`.
- `DESKTOP_PLATFORM` gives way to a `platform()` getter on `DesktopRuntime`.
- `perform_global`'s `open_settings` and `toggle_tps_keyboard` calls become `Emit`s. Linux starts them itself before any replay (`session::run_launches`), so both shells stay as they were and the C ABI does not change. About stays a direct call, because the menu stays in Linux.
- `EngineState` carries `can_delete_surrounding`, not the IBus mask, which stays in Linux.
- Linux keeps `RawKeyEvent`, `Runtime`, menus and the D-Bus and C ABI layers.

Windows and macOS are untouched. Two PRs of about 650 and 800 moved lines (`Emit` + `Recorder` + `LookupSelection` + state + table, then `process_key` + `perform_global`), a named exception to the LOC guide because a pure move is reviewed differently. Refactor round: behaviour freeze, every caller listed, `refactor-reviewer` agent, `/code-review`, Linux e2e.

### Phase 3 — `taigi-web` crate

The browser-facing API, no hand-written `unsafe`:

```text
new Session(settingsJson, platform, locale)  // "macos" | "windows" | "linux", navigator.language
session.addDictionary(name, bytes)           // one call per file
session.install() -> records
session.keyDown(keyJson) -> reply            // reply: JSON { handled, emits, settings? }
session.keyUp(keyJson)   -> reply
session.clickCandidate(position) -> reply    // click_from_panel
session.commitComposition() -> reply         // § Text surface
session.tpsKeyboardRows()                    // JSON, from `keys/tps_keyboard_rows.rs`
session.tpsKeyPress(glyph) -> reply          // a click on a cap
session.caretMoved()                         // disarms the auto-space swap, spends a Shift press
session.focusLost()
session.setSettings(settingsJson)
```

`keyJson` is `{ key, code, shift, ctrl, alt, meta, repeat }`. `locale` is there because the Telex guide and the mode labels read the system language. `Preedit.caret` counts Unicode scalars, as `Emit` does, so the page uses `Array.from(text)`. `ModeChanged` carries the indicator symbol and `AnnounceMode` its label: `reply.rs` is a wire type, not `Serialize` on `Emit`.

- Three shared additions to the `session` module. `commit_composition`: Linux has no twin, because the daemon commits its own preedit and `end_session` cancels (`session/key.rs`); macOS's seam has one (`macos/crates/taigi-macos-ffi/src/session.rs:3`). `toggle_language_mode` plus the English gate in `process_key`: `EngineState.language_mode` stays Taigi on Linux, which never calls it. `mode_label`, moved from Linux's `chrome.rs`.
- A commit that leaves the composition standing skips the switch whole, the Windows `commit_refused` guard, because the session reset would drop text still on screen.
- `key.no_lexicon` needs no change: no logger is installed on the web, so the line goes nowhere.
- `SettingsProvider` and `SettingsStore` over one JSON document the page holds (`page_settings.rs`), so settings keep desktop's key spellings. A chord that writes returns the document in `settings`, once, for the page to persist. Unlike macOS's whitelisted snapshot, the whole document passes: Windows and Linux read it that way, and the page owns it.
- Tests run natively, as `taigi-desktop-core`'s do. Key translation has a table test per `event.code`. `make -C web test` joins the gate.
- `topCandidate` is gone. The smoke test types through `keyDown`.

### Phase 4 — Plain page, end to end

Type in a browser and get Hanji, unstyled. `web/src/` + `make web-serve`.

- `ime.js`, `surface.js`, `panel.js`. Caret position by the mirror-element technique (`index.js:369-412`) (D4).
- Dictionaries fetched plainly; caching waits for Phase 6. Keys typed before the install finishes type romanization, the state `install_lexicon` already documents (`runtime.rs:168`).
- Trap recovery (§ What must change).
- Playwright in Chromium, Firefox and WebKit, from McBopomofoWeb's pattern (`e2e/feature_input.spec.js:13-16`): type `tsiahpng`, pick, assert the textarea value. Undo after commit. Backspace through a composition. TPS keys. Shift tap. Ctrl+Alt chords.
- Playwright for every row of the § Text surface table: click during a composition, click elsewhere then type `.` with auto-space on, selection replaced at commit, Copy mid-composition, scroll, window blur and return.
- Repeat the latency measure through `keyDown`, JSON reply included, in a browser on the slowest machine at hand.
- Open: AltGr flags in Chrome on Windows, which Playwright's US layout cannot produce (`key.rs`).
- Open: a short hardware-keyboard check on iPad Safari. A keyboard extension cannot see hardware keys (`known-pitfalls.md`), so this may be the only route to composing there. UNCONFIRMED until tried on a device.

### Phase 5 — Look, i18n, panels

Three PRs, one round each:

1. **Look**: § UI colours, sidebar, settings form with English labels. Copy and Clear. Empty, loading and error states in fixed slots, nothing that pops in and shifts the page. kautian-mirror's inline pre-paint script (`src/index.html:22-36`) moves to an external blocking file, so Phase 6's CSP allows it. The Font and Display Language rows arrive with slice 2, which brings what they select. The status control shows the mode symbol and opens the sidebar, so Input Mode has one control.
2. **i18n + fonts**: `tools/i18n/i18n_lib.py:35`: add `web` to `VALID_PLATFORMS`, emit `web/src/generated/strings.js`, add `web` to the scope of reused keys. A module rather than the JSON file first planned: it loads in the page's module graph, so the page has no fetch to wait on or fail before it can say anything. A round touching `i18n/**` runs the gates of every platform in the touched keys' scope. `web/scripts/subset-fonts.sh` from kautian-mirror's, run at build time because the output is 19 MB; fonts stay off the first load until selected.
3. **Panels + a11y**: The TPS key panel is `tps_keyboard.js` below Copy and Clear, shown by the existing `tpsKeyboardShown` setting under TPS: Ctrl+Alt+J or a sidebar row that stays in place, disabled, outside TPS. Its room comes out of the textarea. A click types through the shared `tps_keyboard_press`, as on macOS; a key or a click lights its cap for 150 ms, as on macOS and Windows. The symbol picker needed no code: it is a `LookupTable`, so it already draws in the candidate panel. No mouse doorway for it, as the desktop menu has none. Candidate cells are a list with `aria-current`, because focus stays in the textarea and a listbox would never hold it; `#spoken` is the live region and says the one cell Enter would take. `#mode-button` from the keyboard moves focus to Input Mode. `prefers-reduced-motion` stops the transitions.

### Phase 6 — Loading and offline

- `assets-manifest.json`, staged at the site root by `web/scripts/stage-assets.mjs`: file name, SHA-256, size, for the dictionaries and the shell. Names are content-hashed, so a dictionary change is a new URL. The page reads a module staged with it, `generated/dictionaries.js`, so its list never comes from a newer build than its engine; only the service worker reads the JSON.
- First visit: progress in a fixed slot. 15 MB gzip. Verify the hash, then `addDictionary`, one file at a time. A failed or corrupt download leaves romanization working and offers retry.
- Compression: ship `.gz` and read through `DecompressionStream`, Baseline since May 2023, so the result does not depend on what a host compresses. Brotli would save 3.5 MB and needs a decoder in the page, so it is left out.
- Cache Storage plus a service worker for the app shell, one dictionary version kept, old ones deleted on activate. The kautian-mirror extension already uses Cache Storage the same way. Registered after the dictionaries are in, so nothing is fetched twice. A new version downloads in the background and starts once every old tab has closed, with nothing on screen: no `skipWaiting`, because an open tab's trap recovery needs its own glue. A font is cached when first used, not with the shell.
- Web app manifest, so the page installs and opens offline.
- CSP by `<meta>` (Pages cannot set headers): `script-src 'self' 'wasm-unsafe-eval'`, no third-party origins. `style-src-attr 'unsafe-inline'` is the one exception, for WebKit's `<select>` (`web-guidelines.md`).

### Phase 7 — Deploy

Flow: `web-release.md`. Maintainer 2026-11-07: the first public page is settings and typing only, so the planned licences page and privacy statement are dropped, and no new page text or i18n. The web version is a beta with no version number: every push to `main` deploys, unlike every other release, and CI runs Playwright in Chromium only.

- `taigikeyboard/web` holds the Pages workflow; `web.yml` starts it after its checks pass.
- A long edge-cache rule for the hashed dictionaries, like the one on `dl.taigikeyboard.tw`. HiNet to GitHub has been slow, so `make -C web check-live` times the download from Taiwan. Maintainer 2026-11-06: start on GitHub Pages as D1 says and observe. If that proves slow, the option looked at is Cloudflare Pages on `web.taigikeyboard.tw`: every file is under its 25 MiB limit, `_headers` gives a real CSP and cache rules, and a custom domain there takes a subdomain rather than `/web/`.
- `make -C web check-live` hashes the live files against `assets-manifest.json`. The worker's install fails quietly on one mismatch, and Cloudflare can cause it: Email Obfuscation rewrites HTML, and `Content-Encoding: gzip` on a `.gz` would change what the page reads.

### Phase 8 — User data

Maintainer 2026-11-07 ("follow your recommendation"): make the web learn the way the other platforms do, persisted, learning plus the Learning Records section first; Custom Dictionary and `.taigi` backup later. Decisions D7–D9.

Why persisted: every other platform keeps learning in the same SQLite stores, so a web that forgets on reload would not be the same keyboard. McBopomofoWeb learns in memory only (`src/McBopomofo/KeyHandler.ts:274`, `UserOverrideModel.ts` has no save) and keeps only its user phrases, in `localStorage` (`output/example/index.js:1013-1030`); that matches its own native app, not ours.

#### Spike 2026-11-07 (scratchpad, not committed)

The real engine and stores in WASM, driven from Node through the page's bindings. Picking 狗岫 for `kausiu` three times put it first; the four files exported (200 KB, 0.6 ms), imported into a new process before the first key (2 ms), and 狗岫 was still first. The exported files open in the `sqlite3` CLI with the engine's `application_id`, `user_version` 2 and a rollback journal, so they are the same files the apps write.

| | Without learning (main) | With learning |
|---|---|---|
| Key p50 / p99 (1,300 keys, Node, Apple Silicon) | 0.51 / 1.35 ms | 0.70 / 1.85 ms |
| Enter p50 / p99 | 0.02 / 0.12 ms | 0.08 / 0.60 ms |
| First key | 17 ms | 49 ms (opens and shapes the stores) |
| `taigi_web_bg.wasm`, gzip | 854 KB | 1,576 KB (SQLite) |

What it took (grounded in code):

- `uuid` gets its `js` feature on wasm32 (`engine/userdata/Cargo.toml`; one use, `custom_dictionary.rs:100`).
- `database.rs:290`: the spawn fails on this target and `.ok()` drops it, so every write would vanish silently. On wasm32 the queue drains on the caller's thread after each send, same order, same `INSIDE_WORKER_JOB` re-entry guard. `handle.rs:99` already finishes inline when it cannot spawn; the web asks for `in_background: false`.
- `timestamp.rs:15` and `with_stores.rs:131` call `SystemTime::now`, which panics here: one `unix_millis_now` in `timestamp.rs`, `js_sys::Date::now` on wasm32.
- `handle.rs:160` refuses every path, because `Path::is_absolute()` is false on this target (the same trap `LexiconPaths::validate` had in Phase 1). `database.rs:251` `create_dir_all` has no file system to call.
- Storage: `sqlite-wasm-rs` 0.5's default VFS is in memory, files keyed by path, with `MemVfsUtil::import_db` / `export_db`. No `relaxed-idb` and no Worker needed: the page keeps the bytes in IndexedDB.
- Journal: `Delete` on the web; the memory VFS has no shared memory for WAL.
- Build: Apple's `ar` cannot index wasm objects ("not a mach-o file") and leaves `libwsqlite3.a` empty, so the link fails on `sqlite3_*`. `zig ar` (already in `mise.toml`) works; `web/Makefile` sets `AR_wasm32_unknown_unknown`, `web.yml` installs zig, and the `web` repo's deploy must too.

#### 8a — Learning on

- `engine/userdata` + `dispatch`: the wasm32 seams above, native behaviour unchanged (native suites are the freeze gate).
- `taigi-web`: depends on `dispatch` with `user-data`; data directory `/taigi`; exports import / export of the four files.
- Page: holds the Web Lock `taigi-user-data`. The holder imports the IndexedDB snapshot, then opens the stores at load (keeps the 49 ms off the first key), and saves right after each key that commits. A save on `pagehide` or a hidden tab was tried and never landed: all three engines abort an IndexedDB write the unload interrupts. Each save copies the four files whole, about 200 KB after the spike's picks; a large custom dictionary would make that several MB per commit, to look at when Custom Dictionary comes to the web. A second tab types without learning and waits for the lock; when it gets it, it imports the latest snapshot and opens then. The stores open once per instance and an open file cannot be re-imported, so a tab that learned from an old snapshot never overwrites a newer one.
- Tests: Node (learn, export, restore), Playwright (reload keeps a pick; a second tab does not learn until the first closes).

#### 8b — Learning Records section (dropped)

Dropped by the maintainer 2026-11-07, before merge: the web is a sidebar version and needs only learning itself (8a), not a section to list or clear it. Built in #758 and reverted there. Below is the plan as it stood.


The desktop pane in the sidebar: `desktop-core settings/learning_records.rs` model (Frequency, Learned Phrases; delete, clear), same `dictionary.learningRecords*` keys with `web` scope, a reset row as the other sections.

Stores stay in `engine/userdata` (`rust-migration-policy.md` §6). Outside the current slice: Custom Dictionary, `.taigi` import and export.

## Outside these phases

Possible later, none planned: a touch keyboard for phones (Android reports `keyCode` 229 with no key, so it needs an on-screen keyboard and `inputmode="none"`; McBopomofoWeb uses `simple-keyboard`, `index.js:1114`), a ChromeOS input method (`output/chromeos`, `src/chromeos_ime.ts`), and a browser extension. The WASM session would serve all three.

## Decisions

Settled by the maintainer 2026-11-06 ("follow your recommendation"). Ranked by impact.

| # | Question | Decision | Rejected alternative | Applies to |
|---|---|---|---|---|
| D1 | Source location and URL | Source in this repository under `web/`, live at `taigikeyboard.tw/web/`. A thin repo named `web` holds only a Pages workflow that checks out a commit here, builds and deploys, so no 30 MB binaries in git. Maintainer 2026-11-07: each push to `main` starts it, which needs one token here, in place of "no token" and a pinned commit by hand | Pages on this repository, but the URL becomes `/taigikeyboard/`. Or a separate source repo like kautian-mirror, where every engine change has to be pulled across | Phase 7 |
| D2 | First public scope | Physical keyboard only: desktop browsers, Chromebook, iPad with a keyboard | Add a touch keyboard for phones first. Much larger, and phones have the app | Phase 4 |
| D3 | Session logic | A `session` module in `taigi-desktop-core`, shared by Linux and web (Phase 2). Follows Core Principle #5, and costs a Linux refactor round | A fourth session copy in `taigi-web`, about 1,000 lines to keep aligned by hand | Phase 2 |
| D4 | Candidate placement | At the caret, like the desktop apps and McBopomofoWeb. Costs the mirror element and scroll handling | A fixed slot under the editor. Removes both costs and matches the fixed-slot preference for other UI, but the eye leaves the caret | Phase 4 |
| D5 | Learning and Custom Dictionary | Ship without them, add in Phase 8 | Hold the first release for Phase 8, whose storage is unproven | Phase 8 |
| D6 | Dictionary hosting | Same origin on Pages with a long edge cache. No CORS and one deploy | R2 at `dl.taigikeyboard.tw`: free egress and proven speed from Taiwan, at the cost of CORS and a second upload | Phase 6 |

Settled by the maintainer 2026-11-07 ("follow your recommendation"), replacing D5's "add in Phase 8" with what Phase 8 does:

| # | Question | Decision | Rejected alternative | Applies to |
|---|---|---|---|---|
| D7 | Phase 8 scope | Learning, persisted (Learning Records section dropped 2026-11-07) | Custom Dictionary and `.taigi` backup in the same round; or learning in memory only, as McBopomofoWeb | Phase 8 |
| D8 | Shared computers | Learning on by default, as on every other platform; a user clears it in Learning Records (8b dropped 2026-11-07: no clearing on the web) | A "do not learn" switch only the web has | 8a, 8b |
| D9 | Several tabs | One tab learns, by Web Lock; the others type without learning until it closes | Every tab learns and the last save wins, losing the other tabs' picks | 8a |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| 15 MB first load on slow links | Certain, impact moderate | Typing romanization works during download; progress in a fixed slot; cached afterwards |
| Slow machines make long compositions lag | Unknown until Phase 4 | Measured on a low-end device; bound the fetch |
| `execCommand` removed or inconsistent | Low near term | One call site in `surface.js`; swapping to `setRangeText` loses undo only |
| GitHub Pages 100 GB per month soft limit | Low | 6,000+ first loads a month to reach it; D6's R2 is the exit |
| Phase 2 regresses Linux | Medium | Behaviour freeze, pure move, Linux e2e |
| A `wasm-bindgen` CLI and crate version mismatch | Medium | One pin, checked in `make web-check` |
| Auto-space swap deletes real text | Medium without `caretMoved` | § Text surface; Playwright row |

## Best practices alignment

Per phase:

| Phase | Constraining rule |
|---|---|
| 1 | `rust-migration-policy.md` §1 (dependencies flow down), `rust-ffi-safety.md` §1.2 + §3 (named panic divergence, no new hand-written `unsafe`), `rust-best-practices.md` §8 (amended) |
| 2 | `round-workflow.md` Refactor type, `code-review-rules.md` §5, `cross-platform-alignment.md` §1 |
| 3 | `rust-ffi-safety.md` §2 (domain types stay inside), `code-review-rules.md` §2 |
| 4, 6 | `known-pitfalls.md` (spike a platform capability first), `code-review-rules.md` §9 (qualitative performance gate, plus the numbers above because this is a new runtime) |
| 5 | `docs/contributing/i18n.md`, `ui-style-guide.md` |
| 7 | `docs/contributing/security-rules.md`, `CONTRIBUTING.md` § Secrets |

Per choice:

| Mainstream practice | Source | This plan |
|---|---|---|
| Key event → controller → UI interface, called synchronously | `references/McBopomofoWeb/README.COMPONENTS.md`, `output/example/index.js:1273-1303` | `ime.js` → `Session.keyDown` → `Emit` (Phase 4) |
| A UI interface the host implements | `references/McBopomofoWeb/src/McBopomofo/InputUI.ts` | `Emit` list, the Linux shape |
| Caret-anchored panel through a mirror element | `output/example/index.js:369-412` | `surface.js` (Phase 4) |
| Browser tests by real keystrokes | `references/McBopomofoWeb/e2e/feature_input.spec.js:13-16` | Playwright (Phase 4) |
| One engine behind several shells | `docs/references/mainstream-ime-comparison.md` #19 PIME, which runs McBopomofoWeb as a backend | One Rust core behind Linux and web (Phase 2) |
| Static site, Cache Storage for data, no backend | kautian-mirror `planning.md` § Scope | Phase 6 |

Deliberately not adopted:

- **A TypeScript port of the engine** (McBopomofoWeb's way). It wrote its engine for the web. Ours exists in Rust, and a port would be a second engine (`AGENTS.md` § Shared logic lives in the engine).
- **Google Analytics and third-party CDN scripts** (`index.html:11,59,869-870`). Privacy and offline use.
- **`simple-keyboard`** in the first scope. See D2.
- **Worker-hosted engine**. See § Why the main thread.
- **A `Web` variant of `DesktopPlatform`**. The existing values carry the rules; YAGNI.
- **`EditContext`, `contenteditable`**. Support is limited, and a textarea does the job.
- **Text services** (Hanji to Bopomofo, braille, MCP). Not the goal here. taigi-converter already exists.

## Reproducing the spike

The spike was not committed. To repeat it: copy `engine/`, `desktop/` and `assets/symbols/` elsewhere, replace `mmap-host` with a registry of `&'static [u8]` keyed by path, replace `SystemClock` with a constant, and relax `paths.rs:62`. Add a `cdylib` that exports alloc, register, install, one key (`append` + `fetch_candidates`) and cancel. Build with `--target wasm32-unknown-unknown --release` under `opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, and drive it from Node with `WebAssembly.instantiate`.
