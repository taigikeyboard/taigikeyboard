# ChromeOS — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `chromeos`, `chrome.input.ime`, `input_components`, `manifest v3`, `service worker`, `wasm`, `linux-chromeos`, `chromeos flex`, `utm`, `rosetta`
> **Status**: Draft 2026-11-07, decisions D1–D4 open. Phase 0 (this document, issue #759). Phase 1 (test-environment spike) next.
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
| E1 | `linux-chromeos` prebuilt snapshot (`Linux_ChromiumOS_Full`, x86_64, 318 MB zip, refreshed daily) in a new UTM **Apple Virtualization** Debian arm64 VM with **Rosetta for Linux** | Mac | No: ChromeOS UI and IME framework as a Linux app, stub services | Free, ~1 h setup | Daily dev loop, if it runs |
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
| 0 | Roadmap | This document; issue #759 | Draft 2026-11-07 |
| 1 | Spike (no merge) | **1a environment**: E1 on the Mac (fall back to E1b); E2 timeboxed. **1b capability**: a minimal unpacked MV3 IME extension (`input_components` + `onKeyEvent` + `setComposition` / `commitText`) types into the snapshot's browser and a Files rename box. **1c engine**: load `taigi-web` wasm + dictionaries in the worker, type `tai5gi2` + Space → 台語; measure cold first key after the worker stops, warm p50 / p99, worker memory. Record results here. **Hard gate**: if no environment loads an IME extension, stop and ask about E3 / E4 | Pending |
| 2 | Extension shell | New source dir (D2), manifest, worker glue (key mapping, Emit mapping, `keyEventHandled`), dictionaries packaged, build target + lint in `Makefile`, unit tests for the mapping | Pending |
| 3 | Settings + i18n | Options page from the web settings form; `_locales` from `i18n/*.json`, reusing the desktop keys the web page already uses (`docs/contributing/i18n.md`) | Pending |
| 4 | Learning | User data in the worker (IndexedDB), as web Phase 8a | Pending |
| 5 | Packaging + store | CI build of the `.zip`, store listing, privacy fields; Chrome Web Store developer account and submission are the maintainer's | Pending |
| 6 | Real-OS check | E3 (or E4): install the packaged build, type in Chrome, Docs, Files, an Android app, a Linux app | Pending |

### Verification

- Phase 1: results table here (environment, what loaded, cold / warm key times).
- Phases 2–4: `make web-check` stays green (shared crate); new extension target's tests; manual typing in E1 / E1b.
- Phase 6: real ChromeOS, maintainer.

## Decisions (open)

| # | Question | Options | Recommendation |
|---|---|---|---|
| D1 | Test environment | E1 Mac + Rosetta · E1b laptop VM · E2 Flex emulated · E3 Flex USB · E4 Chromebook | E1 for the dev loop (E1b if Rosetta fails), E3 once before store submission; E4 only if tablet mode or long dogfood is wanted |
| D2 | Source location | `web/chromeos/` beside `web/src`, sharing the `web` Cargo workspace · new top-level `chromeos/` | `web/chromeos/`: same crate, same build tools, same `make -C web` lint |
| D3 | Android app on Chromebooks | Leave as-is (tablet mode only) · tune it for ChromeOS | Leave as-is; outside this roadmap |
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
