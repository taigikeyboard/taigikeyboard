# ChromeOS — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `chromeos`, `chrome.input.ime`, `input_components`, `manifest v3`, `service worker`, `wasm`, `linux-chromeos`, `chromeos flex`, `utm`, `rosetta`
> **Status**: Accepted 2026-11-07, decisions D1–D5 settled. Phase 0 done (issue #759). Phase 1 passed 2026-11-07 (台語 typed in `linux-chromeos` on the Mac; offscreen-document keepalive, § Spike result). Phases 2–4 merged (#765, #766, #767); Phase 5 release doc ready, publishing and Phase 6 (real ChromeOS) wait for the maintainer.
> **Siblings**: `web-roadmap.md` (the wasm engine this reuses), `windows-arm64-roadmap.md` (same VM-first pattern).

---

## Summary

- **Goal**: Taigi Keyboard as a system input method on ChromeOS, typed from the Chromebook's physical keyboard, in every app (Chrome, Docs, Android apps, Linux apps through the ChromeOS IME bridge).
- **Approach**: a Manifest V3 Chrome extension that registers an input method (`input_components`) and answers `chrome.input.ime` events from its service worker. The engine is the web version's wasm build (`web/crates/taigi-web`) over `taigi-desktop-core::session`, so ChromeOS gets the desktop behaviour without a new engine port.
- **Constraint**: the maintainer has no ChromeOS device. Phase 1 builds a test environment on the Apple Silicon Mac (UTM) and the x64 Windows laptop, and is a hard gate.

## Why not the Android app (grounded in code)

ChromeOS runs Android IMEs only as the on-screen keyboard in tablet mode. Chromium `chrome/browser/ash/arc/input_method_manager/arc_input_method_manager_service.cc`, `ArcInputMethodStateDelegateImpl::ShouldArcIMEAllowed()`:

```cpp
// It depends on tablet mode state and a11y keyboard option.
const bool is_normal_vk_enabled =
    !profile_->GetPrefs()->GetBoolean(prefs::kAccessibilityVirtualKeyboardEnabled) &&
    display::Screen::Get()->InTabletMode();
return is_command_line_flag_enabled || is_normal_vk_enabled;
```

The descriptor it builds says so too: "the feature is for tablet mode". A key from the physical keyboard never reaches an Android IME. The existing Android app can already be installed on a touchscreen Chromebook from Play (no `uses-feature` limits it, `android/app/src/main/AndroidManifest.xml`) and serves tablet mode as-is; this roadmap does not change it (D3).

## Architecture

```text
ChromeOS (ash) input method framework
   │ onActivate / onFocus / onBlur / onKeyEvent / onCandidateClicked / onReset
   ▼
Extension service worker (MV3, module)
┌───────────────────────────────────────────────────────────┐
│ chromeos glue (JS)                                        │
│   keyData → BrowserKey JSON → session.key_down / key_up   │
│   reply Emits → chrome.input.ime.* calls                  │
│   dictionaries from the extension package (fetch(getURL)) │
├───────────────────────────────────────────────────────────┤
│ taigi-web (wasm, unchanged API: Session::new, key_down …) │
│ taigi-desktop-core::session → engine                      │
└───────────────────────────────────────────────────────────┘
Options page (extension page): the web sidebar's settings form
```

### Emit → `chrome.input.ime` (grounded in code)

`web/crates/taigi-web/src/reply.rs:214-253` turns each `Emit` into a `WireEmit`. The glue maps them:

| `WireEmit` | `chrome.input.ime` | Note |
|---|---|---|
| `preedit { text, caret }` | `setComposition({ contextID, text, cursor })` | |
| `clearPreedit` | `clearComposition` | |
| `commit { text }` | `commitText` | |
| `deleteSurrounding { offset, count }` | `deleteSurroundingText({ engineID, contextID, offset, length })` | Auto-space swap. Verify offset units (UTF-16) in Phase 2 |
| `lookupTable` / `hideLookupTable` | `setCandidates` + `setCandidateWindowProperties({ visible, vertical, pageSize, cursorVisible, currentCandidateIndex, … })` | The system draws the window: vertical or horizontal list only (D4) |
| `announceMode` | `setCandidateWindowProperties` auxiliary text, or the `indicator` | Spike decides |
| `openSettings` | `chrome.runtime.openOptionsPage()` | |
| `toggleTpsKeyboard`, `tpsKeyboardCapTyped` | none at first | No extension surface for a key panel beside a system text field |

`onKeyEvent` must return whether the key was taken. With a cold worker the answer is not ready synchronously, so the listener returns `undefined` and calls `chrome.input.ime.keyEventHandled(requestId, handled)` when the engine replies (asynchronous form of the API).

### Key mapping (grounded in code)

`taigi-web` already reads a browser key as `BrowserKey { key, code, shift, ctrl, alt, meta, repeat }` (`web/crates/taigi-web/src/key.rs:66-74`) and turns it into the core's `KeyEventSnapshot`. `chrome.input.ime.KeyboardEvent` carries `type`, `key`, `code`, `shiftKey`, `ctrlKey`, `altKey`, `altgrKey`, `capsLock`. So the glue fills `BrowserKey` and reuses `key.rs` unchanged. To settle in Phase 1: how the Search (Launcher) key arrives (`meta` has no field), whether key repeat is visible, and which `platform` string the session gets (`Session::new` accepts `macos` / `windows` / `linux`, `bindings.rs:89-98`; ChromeOS keyboards are PC layout, so `linux` is the starting guess).

### Service-worker lifetime

An MV3 worker stops after about 30 s without events. An `onKeyEvent` wakes it, but the first key after that waits for wasm instantiation (2.3 MB) and dictionary install (29.6 MB in linear memory, web spike numbers in `web-roadmap.md` § Spike results). McBopomofoWeb keeps its worker alive with a port opened from a tab content script every 295 s (`references/McBopomofoWeb/src/chromeos_ime.ts:839-881`). That needs `<all_urls>` host permission and `scripting`, which review flags. Phase 1 measures the cold first key on the test environment; the decision (accept cold start, keepalive, or an offscreen document holding the engine) waits for the number.

## Test environments

The maintainer has an M4 Mac (16 GB, 80 GB free, UTM installed; existing `Linux.utm` is QEMU arm64, no Rosetta) and the x64 Windows laptop (`ssh win`, VirtualBox, Linux amd64 VM).

| # | Environment | Host | Real ChromeOS | Cost | Use |
|---|---|---|---|---|---|
| E1 | `linux-chromeos` prebuilt snapshot (`Linux_ChromiumOS_Full`, x86_64, 318 MB zip, refreshed daily) in a new **Apple Virtualization** Debian arm64 VM with **Rosetta for Linux** (Lima or UTM) | Mac | No: ChromeOS UI and IME framework as a Linux app, stub services | Free, ~1 h setup | Daily dev loop; runs (§ Spike result) |
| E1b | Same snapshot in the laptop's VirtualBox amd64 VM, or a GitHub-hosted Ubuntu runner under Xvfb | Windows laptop / CI | No | Free | E1 without Rosetta; later an automated smoke test |
| E2 | ChromeOS Flex in UTM x86_64 **emulation** | Mac | Yes, minus Play Store | Free | Expect too slow; virtual GPUs often fail Flex. Timebox 1 h |
| E3 | ChromeOS Flex **live from USB** ("Try it first"; nothing written to disk) | Windows laptop | Yes, minus Play Store | 8 GB USB stick, maintainer at the machine | Real-OS check before store submission |
| E4 | A used Chromebook | — | Yes | Purchase | Tablet mode, Android app, long dogfood |

Snapshot facts checked 2026-11-07: bucket `chromium-browser-snapshots` lists `Linux_ChromiumOS_Full/`, `LAST_CHANGE` = `1738482`, `chrome-chromeos.zip` 317,882,134 bytes. Chromium's build doc says `chrome://flags` has no effect on `linux-chromeos` (pass switches on the command line) and suggests `--ui-enable-software-compositing --ui-disable-threaded-compositing` when drawing breaks in a VM.

Rosetta for Linux runs dynamically linked x86_64 binaries only when the guest has x86_64 libraries (Debian multiarch has them; Fedora does not). Chromium's sandbox under Rosetta is unverified; `--no-sandbox` is acceptable in a throwaway VM.

## Phases

One PR per phase; each starts after the previous one merges. No version is assigned.

| # | Phase | Content | Status |
|---|---|---|---|
| 0 | Roadmap | This document; issue #759 | Done 2026-11-07 |
| 1 | Spike (no merge) | **1a environment**: E1 on the Mac (fall back to E1b); E2 timeboxed. **1b capability**: a minimal unpacked MV3 IME extension (`input_components` + `onKeyEvent` + `setComposition` / `commitText`) types into the snapshot's browser and a Files rename box. **1c engine**: load `taigi-web` wasm + dictionaries in the worker, type `tai5gi2` + Space → 台語; measure cold first key after the worker stops, warm p50 / p99, worker memory. Record results here. **Hard gate**: if no environment loads an IME extension, stop and ask about E3 / E4 | **Passed 2026-11-07** (§ Spike result) |
| 2 | Extension shell | New source dir (D2), manifest, worker glue (key mapping, Emit mapping, `keyEventHandled`), dictionaries packaged, build target + lint in `Makefile`, unit tests for the mapping; `taigi-web` gains `endSession` / `beginSession` (the field confirms the preedit on blur, Linux's model); offscreen keepalive | Merged #765 `77f99ee8c` 2026-11-08 |
| 3 | Settings | Options page cut at build time from the web page's settings form (`chromeos/options-form.mjs`: rows ChromeOS cannot honour hidden, expandable layouts removed, drift fails the build); same document `taigi.settings` and same i18n keys, in `chrome.storage.sync` so it follows the account; worker reads it at start, follows changes (composition written first), saves chord writes; `openSettings` opens the page. No `_locales`: the name 台語齒盤 is the same in every language | Merged #766 `73a742207` 2026-11-08 |
| 4 | Learning | The worker runs the page's `learning.js` unchanged (IndexedDB + Web Lock both work in a service worker; the worker is the extension's only learner), saving after each commit. Every engine call goes through `run`: a trap ends the session for that worker, which then stops keeping itself alive so the next focus starts a new worker (a worker cannot import a fresh copy of the glue) | Merged #767 `8bd2b7db9` 2026-11-08 |
| 5 | Packaging + store | `chromeos-release.md`: build, version, store listing, privacy fields, screenshots. No CI zip: publishing is by hand and `make web-check` already runs the ChromeOS tests. Developer account, version and submission are the maintainer's | Release doc done; first upload with the desktop v3.9.0 release (maintainer 2026-11-08) |
| 6 | Real-OS check | E3 (or E4): install the packaged build, type in Chrome, Docs, Files, an Android app, a Linux app | Pending |

### Verification

- Phase 1: results table here (environment, what loaded, cold / warm key times).
- Phases 2–4: `make web-check` stays green (shared crate); new extension target's tests; manual typing in E1 / E1b.
- Phase 6: real ChromeOS, maintainer.

### Spike result (2026-11-07)

Environment E1 on the M4 Mac: Lima 2.3.0 (`--vm-type=vz --rosetta`, the same Apple Virtualization framework UTM's Apple backend uses, but scriptable), Debian 13 arm64, 4 CPU / 6 GB. `Linux_ChromiumOS_Full` r1738485 runs through Rosetta under Xvfb with `--no-sandbox`. It needs the amd64 multiarch libraries plus `libgl1:amd64 libegl1:amd64 libgl1-mesa-dri:amd64`; without `libGL.so.1` the GPU process fails and the browser aborts (`GPU process isn't usable`). Driven headless: `xdotool` into the ash host window, Chrome DevTools Protocol (`--remote-debugging-port`) to open a tab and read the textarea, worker `console.log` lines in the `--enable-logging=stderr` log.

The input method is enabled without the settings UI by writing the stub user's prefs while Chrome is stopped (`test-user/Preferences`: `settings.language.enabled_extension_imes` and `preload_engines` = `_ext_ime_<extension id><component id>`), then Ctrl+Space selects it.

| Check | Result |
|---|---|
| MV3 extension with `input_components`, loaded by `--load-extension` | Loads (needs `--disable-features=DisableLoadExtensionCommandLineSwitch`) |
| `taigi-web` wasm (4.0 MB) + 4 dictionaries (30.9 MB) in the worker, static `import` + `WebAssembly.compileStreaming`, CSP `'wasm-unsafe-eval'` | Works, unchanged API. Worker boot 280–560 ms (wasm 130–370 ms, dictionaries 110–220 ms) |
| `tai5gi2` + Space in a web textarea | 台語 committed. Preedit `tâigí` underlined at the caret; system candidate window with our labels (`q w d f z x v y ;`) and TL annotations |
| `lang5` + Space in the omnibox (a views field, focus type `url`) | Committed |
| Engine time per key (Rosetta, so slower than a real Chromebook) | First key after boot 95–234 ms, then 3–30 ms |
| `onKeyEvent` + `keyEventHandled(requestId, handled)` | Works |
| **Worker lifetime** (without keepalive) | The worker stops about 30 s after the last event. A key does not wake it: keys typed after a pause reach the field as raw ASCII (`goa2 `) and the worker logs nothing. A focus change (`onFocus` / `onActivate`) does wake it, so the next field works again |
| Keepalive: `chrome.runtime.getPlatformInfo()` every 20 s | Did not help, worker gone at 30–40 s |
| **Keepalive: offscreen document** (`chrome.offscreen`, reason `WORKERS`) that sends `chrome.runtime.sendMessage` every 20 s | **Works.** Worker alive through 160 s idle, the key after the pause composes (`li2` → 你). Created from the worker's top level and again on `onActivate` / `onFocus` when `chrome.offscreen.hasDocument()` is false. Two traced fresh launches created it in under 1 s; one earlier untraced launch logged neither success nor failure and was not reproduced |

**Chosen for Phase 2: the offscreen-document keepalive.** It needs only the `offscreen` permission, no host permissions. McBopomofoWeb's content-script port (`references/McBopomofoWeb/src/chromeos_ime.ts:839-881`) needs `scripting` + `<all_urls>` and only works while an http(s) tab is open.

Not verified: whether real ChromeOS wakes a stopped IME worker on a key (`linux-chromeos` runs stub services); runs past 5 minutes idle; worker memory; a Files rename box; E2. Phase 6 (E3) re-checks the keepalive on real ChromeOS.

## Decisions (maintainer, 2026-11-07: "follow your recommendations")

| # | Question | Options | Decision |
|---|---|---|---|
| D1 | Test environment | E1 Mac + Rosetta · E1b laptop VM · E2 Flex emulated · E3 Flex USB · E4 Chromebook | E1 for the dev loop (E1b if Rosetta fails), E3 once before store submission; E4 only if tablet mode or long dogfood is wanted |
| D2 | Source location | `web/chromeos/` beside `web/src`, sharing the `web` Cargo workspace · new top-level `chromeos/` | `web/chromeos/`: same crate, same build tools, same `make -C web` lint |
| D3 | Android app on Chromebooks | Leave as-is (tablet mode only) · tune it for ChromeOS | Leave as-is; outside this roadmap |
| D5 | School-managed Chromebooks (admin console allowlist, force-install) | Target · not a target | Not a target: no admin-console work or checks in this roadmap |
| D4 | Candidate window | System window (`setCandidates`) · own window | System window: an extension cannot draw over another app's text field; Horizontal / Vertical only, no Expandable |

## Risks

- **`linux-chromeos` is not ChromeOS.** Stub services, X11 key path (Chromium notes Caps Lock must be set by hand there, `ui/base/ime/ash/input_method_ash.cc`). E3 before release.
- **Cold first key** after the worker stops (above).
- **Shortcut collisions**: ChromeOS owns Ctrl+Space (switch input method) and many Search / Ctrl+Alt chords; the default Ctrl+Alt table (`keys/shortcut_actions.rs`) needs a check in Phase 1.
- **Store review** of the `input` permission and any host permission.
- **Chrome extension #655** was dropped by the maintainer 2026-11-07. That was a browser extension for web pages on all OSes; this is a ChromeOS input method that ships through the same store. Distinct on purpose; the maintainer raised ChromeOS 2026-11-07.

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| ChromeOS third-party IME = MV3 extension, `input_components` + `chrome.input.ime` in the worker | `references/McBopomofoWeb/output/chromeos/manifest.json`; developer.chrome.com/docs/extensions/reference/api/input/ime | Phases 1–2 |
| One engine, a thin host per platform | McBopomofoWeb: one TS engine behind web, ChromeOS, PIME, CLI hosts (`docs/references/mainstream-ime-comparison.md` row 31) | Reuses `taigi-web` |
| Shared logic in the engine | `AGENTS.md` § Design principles | No logic in the glue beyond mapping |
| Spike an OS capability before planning on it | `docs/contributing/known-pitfalls.md` § Tests and diagnosis | Phase 1 hard gate |

### Deliberately not adopted

- **Android IME as the ChromeOS keyboard** — tablet mode only (above).
- **Fcitx5 / IBus in the Linux container (Crostini)** — reaches only Linux apps, needs Linux enabled, and is the existing Linux build.
- **A JS rewrite of the engine** (McBopomofoWeb's route) — we already have the engine in wasm.
- **Keepalive by default** — needs `<all_urls>`; only if Phase 1's cold-start number calls for it.
