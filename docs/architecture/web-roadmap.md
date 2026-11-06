# Web Version — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `web`, `browser`, `wasm`, `wasm-bindgen`, `PWA`, `desktop-core reuse`, `McBopomofoWeb`, `kautian-mirror`
> **Status**: Accepted 2026-11-06. Phases 0 and 1 done. Decisions D1–D6 settled by the maintainer as recommended (§ Decisions). Reviewed once by a fresh-context same-model agent, findings applied; Codex skipped (no subscription, maintainer 2026-11-04).
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

**Not measured**: a real browser, a low-end Chromebook, DOM cost, or the full `process_key` path with its JSON reply, which carries the whole candidate list on every key (`linux/crates/taigi-linux-core/src/session.rs:603`). All numbers come from Node on an Apple Silicon Mac. Phase 4 measures through `keyDown` in a browser on the slowest machine at hand.

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
| `ModeChanged` / `AnnounceMode` | Update the status control |
| New in Phase 2: `OpenSettings`, `ToggleTpsKeyboard` | Open the sidebar, show the TPS key panel |

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
| Caret moved or text changed by anything but this page (arrows, mouse, paste, undo) | `session.caretMoved()` |
| Textarea `scroll` | Re-anchor the panel at the caret |

The `caretMoved` row prevents real text loss. The auto-space swap deletes the character before the caret without looking at it (`Emit::DeleteSurrounding { offset: -1, count: 1 }`, `session/executor.rs:97-100`). Its armed state clears only on the next key or at session end, so click elsewhere, type `.`, and a real character goes.

`EditContext` is left out: MDN lists it as limited availability and experimental.

### Browser key rules

From McBopomofoWeb:

- Skip while `event.isComposing` or `keyCode === 229`, and show a notice that another input method is active (`index.js:1245-1282`).
- Shift tap switches to English on `keyup` (`index.js:1261-1271`). Ours uses the shared tracker in `keys/shift_tap.rs`.

Different from McBopomofoWeb, which resets on `Alt` (`index.js:1278-1281`): **only `Meta` chords stay with the browser.** Everything else goes to the classifier, because seven of the eight default shortcuts are Ctrl+Alt (`keys/shortcut_actions.rs:224-232`) and ⌥ is the Mac caret chord (`keys/intent.rs:64`).

`KeyEventSnapshot` (`keys/snapshot.rs:88`) is filled in `taigi-web` from `event.key`, `event.code` and the modifiers:

- `characters_ignoring_modifiers` has no browser equivalent, and chord matching reads it. Derive it from `event.code`.
- `key_code` is read by `chord.rs:72,179` and `tps_layout.rs:100` for the keypad and shifted digits. Map `event.code` into the code space those read.
- Phase 3 tests each mapped entry, and lists which default chords survive the browser. Ctrl+T, Ctrl+W and Ctrl+N never reach a page.

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
| Sidebar | `src/styles.css:125-167`, `src/sidebar.js` | 18.75rem, docked above 50rem, `popover` drawer below, collapsed by default |
| Settings to `data-*` | `src/settings.js`, `src/index.html:23-36` | One form, restored before first paint so a reload does not flash |
| Fonts | `src/styles.css:3-38`, `scripts/subset-fonts.sh` | Same roster, CJK-only `unicode-range` |
| Tool chain | `esbuild.config.mjs`, `node --test` | No framework |

The `unicode-range` is required, not cosmetic: those fonts misplace combining tone marks (`tsi̍t` renders as `tsit`), so romanization must fall through to the system font.

Sidebar content: Appearance, Font, Input Mode (Tâi-lô, Pe̍h-ōe-jī, TPS), Output (Candidate Display), Display Language, and links to the apps, the converter and kautian-mirror. Labels come from `i18n/` by their existing keys.

Copy follows the project's plain-prose rules, and stays neutral between romanization systems.

## Behaviour against the desktops

Same as desktop, because it is the same code: the key table, continuous input, candidates and paging, full-width punctuation, auto-space, dictionary toggles, Hanji and romanization output.

Named divergences (`cross-platform-alignment.md` §3), all intentional unless marked:

| Behaviour | Web | Why |
|---|---|---|
| Next-word prediction | None, same as desktop | Physical keyboard; maintainer 2026-10-03 |
| Target of typing | The page's textarea only | A tab cannot reach other apps |
| Global shortcuts | The Ctrl+Alt set of Windows and Linux on every OS, minus what the browser keeps. On a Mac this is not the app's ⌃⌘ set, whose rows live in Swift (`keys/shortcut_actions.rs:219`) | Browser-reserved chords; one set of defaults in Rust |
| English mode by Shift tap | Yes, as on Windows and macOS. **Linux has none** (`chrome.rs:110-113`), so the gate is opt-in per shell | A page has no OS input-method switcher |
| Candidate click | Selects, never commits, as on Linux and macOS (`session.rs:494-496`) | Same code |
| Keyboard Layout setting | No row | `event.key` already follows the OS layout |
| Panic | Trap, then a new instance | No unwinding on this target |
| Learning, Custom Dictionary | **Pending** until Phase 8 | § Decisions D5 |

## Phases

Each phase is one PR of 200–500 LOC unless noted. No phase is assigned to a release.

| # | Phase | Gate | Status |
|---|---|---|---|
| 0 | This roadmap + project memory | admin lane | Done |
| 1 | Engine runs on wasm32 | `make web-check`, native suites unchanged | Done |
| 2 | Shared session module | behaviour freeze, `make linux-check` | Pending |
| 3 | `taigi-web` crate | native + Node tests | Pending |
| 4 | Plain page, end to end | Playwright in 3 browser engines | Pending |
| 5 | Look, then i18n + fonts, then panels + a11y (3 PRs) | Playwright, i18n gates | Pending |
| 6 | Loading and offline | Playwright offline run | Pending |
| 7 | Deploy | manual run on the real URL | Pending |
| 8 | User data | spike first, then its own plan | Pending |

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

`taigi-linux-core` has the session logic the web needs, about 1,460 non-test lines across `session.rs`, `chrome.rs`, `executor.rs` and `selection.rs`. It cannot be linked as it stands: it depends on `taigi-desktop-storage`, `taigi-linux-platform` and `dispatch/user-data`. The coupling is thin, though. `Runtime` only derefs to `DesktopRuntime`, and the platform ties are a few imports.

Move the framework-free part into `taigi-desktop-core` as a `session` module. No new crate: its dependencies already fit, and a crate would add a seam with one owner.

- `Emit`, `Recorder`, `LookupSelection`, `process_key`, `perform_global`, `click_from_panel`, over `&DesktopRuntime`.
- `DESKTOP_PLATFORM` (`session.rs:29`) gives way to a `platform()` getter on `DesktopRuntime` (`runtime.rs:57`).
- `open_settings` (`chrome.rs:133,171`) and `toggle_tps_keyboard` (`chrome.rs:253`) become `Emit`s.
- Linux keeps `RawKeyEvent`, `Runtime`, menus and the D-Bus and C ABI layers.

Windows and macOS are untouched. Two PRs of about 730 moved lines each (`Emit` + `Recorder` + `LookupSelection`, then `process_key` + `perform_global`), a named exception to the LOC guide because a pure move is reviewed differently. Refactor round: behaviour freeze, every caller listed, `refactor-reviewer` agent, `/code-review`, Linux e2e.

### Phase 3 — `taigi-web` crate

The browser-facing API, no hand-written `unsafe`:

```text
new Session(settingsJson, platform)      // "macos" | "windows" | "linux"
session.addDictionary(name, bytes)       // one call per file
session.install() -> { records }
session.keyDown(keyJson) -> { handled, emits }
session.keyUp(keyJson)   -> { handled, emits }
session.clickCandidate(position) -> { emits }   // click_from_panel
session.commitComposition() -> { emits }        // § Text surface
session.caretMoved()                            // disarms the auto-space swap
session.setSettings(settingsJson)
```

- Two shared additions to the `session` module. `commit_composition`: Linux has no twin, because the daemon commits its own preedit and `end_session` cancels (`session.rs:350,360`); macOS's seam has one (`macos/crates/taigi-macos-ffi/src/session.rs:3`). A `LanguageMode` gate for English mode, off for Linux.
- `key.no_lexicon` warns on every key (`session.rs:324`). On the web that is normal during download, so log it once.
- `SettingsProvider` from a JSON snapshot, the `macos/crates/taigi-macos-ffi/src/settings.rs:121` precedent, so settings keep desktop's key spellings.
- `SettingsStore` over the same snapshot; chords that change settings return the new document in the reply so the page persists it.
- Tests run natively, as `taigi-desktop-core`'s do. Key translation has a table test per `event.code`.

### Phase 4 — Plain page, end to end

Type in a browser and get Hanji, unstyled. `web/src/` + `make web-serve`.

- `ime.js`, `surface.js`, `panel.js`. Caret position by the mirror-element technique (`index.js:369-412`) (D4).
- Dictionaries fetched plainly; caching waits for Phase 6. Keys typed before the install finishes type romanization, the state `install_lexicon` already documents (`runtime.rs:168`).
- Trap recovery (§ What must change).
- Playwright in Chromium, Firefox and WebKit, from McBopomofoWeb's pattern (`e2e/feature_input.spec.js:13-16`): type `tsiahpng`, pick, assert the textarea value. Undo after commit. Backspace through a composition. TPS keys. Shift tap. Ctrl+Alt chords.
- Playwright for every row of the § Text surface table: click during a composition, click elsewhere then type `.` with auto-space on, selection replaced at commit, Copy mid-composition, scroll, window blur and return.
- Repeat the latency measure through `keyDown`, JSON reply included, in a browser on the slowest machine at hand.
- A short hardware-keyboard check on iPad Safari. A keyboard extension cannot see hardware keys (`known-pitfalls.md`), so this may be the only route to composing there. UNCONFIRMED until tried on a device.

### Phase 5 — Look, i18n, panels

Three PRs, one round each:

1. **Look**: § UI colours, sidebar, settings form with English labels. Copy and Clear. Empty, loading and error states in fixed slots, nothing that pops in and shifts the page. kautian-mirror's inline pre-paint script (`src/index.html:22-36`) moves to an external blocking file, so Phase 6's CSP allows it.
2. **i18n + fonts**: `tools/i18n/i18n_lib.py:35`: add `web` to `VALID_PLATFORMS`, emit `web/src/generated/strings.json`, add `web` to the scope of reused keys. A round touching `i18n/**` runs the gates of every platform in the touched keys' scope. `web/scripts/subset-fonts.sh` from kautian-mirror's; fonts stay off the first load until selected.
3. **Panels + a11y**: TPS key panel and symbol picker through the same panel. Keyboard focus order, `aria-live` for the candidate list, `prefers-reduced-motion`.

### Phase 6 — Loading and offline

- `web/assets-manifest.json`: file name, SHA-256, size. Names are content-hashed, so a dictionary change is a new URL.
- First visit: progress in a fixed slot. 15 MB gzip. Verify the hash, then `addDictionary`. A failed or corrupt download leaves romanization working and offers retry.
- Compression: ship `.gz` and read through `DecompressionStream`, Baseline since May 2023, so the result does not depend on what a host compresses. Brotli would save 3.5 MB and needs a decoder in the page, so it is left out.
- Cache Storage plus a service worker for the app shell, one dictionary version kept, old ones deleted on activate. The kautian-mirror extension already uses Cache Storage the same way.
- Web app manifest, so the page installs and opens offline.
- CSP by `<meta>` (Pages cannot set headers): `script-src 'self' 'wasm-unsafe-eval'`, no third-party origins.

### Phase 7 — Deploy

- `docs/architecture/web-release.md`, build and deploy flow per D1.
- A long edge-cache rule for hashed assets, like the one on `dl.taigikeyboard.tw`. Today `taigikeyboard.tw` is Cloudflare in front of GitHub Pages with `max-age=600`. HiNet to GitHub has been slow, so measure from Taiwan before trusting Pages alone.
- Licences page: `NOTICE`, `THIRD_PARTY_LICENSES.md`, OFL, dictionary source credits.
- Privacy statement: text and settings stay in the browser. No analytics.
- Deploy runs by hand, like every other release.

### Phase 8 — User data

Not required for a useful first page. Gate it with a spike, because this is the one part with no working proof.

What the 2026-11-06 check found: `rusqlite` selects `sqlite-wasm-rs` by itself on this target. The build stops at `uuid`, which needs a random source feature. `engine/userdata/src/handle.rs:99` and `database.rs:290` start threads this target does not have. `engine/dispatch/src/user_data/with_stores.rs:131` and `engine/userdata/src/timestamp.rs:15` still call `SystemTime::now`, past the Phase 1 `Clock` seam, so the stores need the clock too.

The storage choice is constrained by the main-thread decision. `sqlite-wasm-rs`'s OPFS `sahpool` VFS needs a Dedicated Worker, and the engine lives on the main thread. That leaves its IndexedDB-backed `relaxed-idb` VFS or an in-memory database saved to OPFS, to be proven by the spike. Stores stay in `engine/userdata` either way (`rust-migration-policy.md` §6). `.taigi` backup import and export lets a user carry data between the web and the apps.

## Outside these phases

Possible later, none planned: a touch keyboard for phones (Android reports `keyCode` 229 with no key, so it needs an on-screen keyboard and `inputmode="none"`; McBopomofoWeb uses `simple-keyboard`, `index.js:1114`), a ChromeOS input method (`output/chromeos`, `src/chromeos_ime.ts`), and a browser extension. The WASM session would serve all three.

## Decisions

Settled by the maintainer 2026-11-06 ("follow your recommendation"). Ranked by impact.

| # | Question | Decision | Rejected alternative | Applies to |
|---|---|---|---|---|
| D1 | Source location and URL | Source in this repository under `web/`, live at `taigikeyboard.tw/web/`. A thin repo named `web` holds only a Pages workflow that checks out a pinned commit here, builds and deploys. No token needed, and no 30 MB binaries in git | Pages on this repository, but the URL becomes `/taigikeyboard/`. Or a separate source repo like kautian-mirror, where every engine change has to be pulled across | Phase 7 |
| D2 | First public scope | Physical keyboard only: desktop browsers, Chromebook, iPad with a keyboard | Add a touch keyboard for phones first. Much larger, and phones have the app | Phase 4 |
| D3 | Session logic | A `session` module in `taigi-desktop-core`, shared by Linux and web (Phase 2). Follows Core Principle #5, and costs a Linux refactor round | A fourth session copy in `taigi-web`, about 1,000 lines to keep aligned by hand | Phase 2 |
| D4 | Candidate placement | At the caret, like the desktop apps and McBopomofoWeb. Costs the mirror element and scroll handling | A fixed slot under the editor. Removes both costs and matches the fixed-slot preference for other UI, but the eye leaves the caret | Phase 4 |
| D5 | Learning and Custom Dictionary | Ship without them, add in Phase 8 | Hold the first release for Phase 8, whose storage is unproven | Phase 8 |
| D6 | Dictionary hosting | Same origin on Pages with a long edge cache. No CORS and one deploy | R2 at `dl.taigikeyboard.tw`: free egress and proven speed from Taiwan, at the cost of CORS and a second upload | Phase 6 |

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
