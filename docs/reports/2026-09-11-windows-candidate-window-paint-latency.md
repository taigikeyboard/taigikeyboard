# Windows candidate window: frame-before-content latency — measurement + architecture options

Status: **evaluation only, nothing decided** (USER 2026-09-11: 「先記錄成文件,以後有時間再來評估」).
No round is open; no release scope is implied.

## Symptom (USER report, 2026-09-11)

> for windows,候選窗的候選字有時候會比候選窗慢出現,感覺有一點delay,但不是每次

The window's rounded frame appears, then its candidates fill in a moment later. Intermittent.
Question asked: performance bug, or the machine? (Box: ASUS Vivobook 15, Core 7 150U / 16 GB.)

## Mechanism (actual code reading)

1. `windows/crates/taigi-windows-tsf/src/ui/window.rs` `WindowRef::show_at` — `SetWindowPos` →
   `ShowWindow(SW_SHOWNA)` → `InvalidateRect`. The window is visible immediately (DWM composites
   the `DWMWCP_ROUND` frame over an unpainted redirection surface); the content is drawn only in
   `WM_PAINT`.
2. The TSF DLL is in-proc: the popup lives on the **host application's UI thread**. `WM_PAINT` is
   the lowest-priority message, dispatched by the host's own loop only once its queue holds no
   input. So the frame→content gap = the host's per-key work + queued input. Heavy hosts and fast
   typing widen it; Notepad hides it.
3. First paint of each list also builds one `IDWriteTextLayout` per drawn cell part
   (`candidate_window.rs` `replace_cells` clears the cache; `cached_layout` rebuilds), and the
   first paint in a process creates the Direct2D `HwndRenderTarget` (`ensure_surface`).

Not the gap, but noted: `CandidateWindow::show` measures every cell twice (`measure_cells` +
`build_layout`) inside the key handler — cap 200 cells. Measured at ~2 ms; not a problem today.

## Measurement (2026-09-11, box, temporary instrumentation — not merged)

Three probes appended to `%TEMP%\taigi-paint-timing.log` per host process: `show_measure`
(measure + layout duration), `show_at` (timestamp after `ShowWindow`), `paint` (`WM_PAINT`
timestamp + duration + whether the handler borrow succeeded). Release build via
`install-dev.ps1 install` (+ `reload` after the dictionary-mmap staging failure), log out / in.
USER typed ~10 lists each in Notepad and Chrome; 430 lines, 142 window shows.

| host | shows | show_at → paint p50 / p90 / max | first paint in process | measure p50 |
|---|---|---|---|---|
| Notepad.exe | 115 | 2 / 4 / 15 ms | 15 ms | 1.7 ms |
| chrome.exe | 27 | 2 / 5 / **171 ms** | **43 ms** | 1.5 ms |

- `painted=false` (handler busy) never happened.
- One real occurrence: Chrome's **first** candidate window. Chrome did not pump `WM_PAINT` for
  126 ms after `show_at` (the next key arrived first and replaced the list), then the first paint
  itself took 43 ms (D2D surface creation) — an empty frame for 171 ms. Notepad's equivalent
  first-show gap was 15 ms.
- USER did not perceive anything during this session (「這次完全沒感受到明顯效能問題」).

**Verdict**: not the hardware (measure 2 ms, paint 1.5 ms). Architectural — show-before-paint on a
borrowed thread. Visible only when *first show* × *heavy host* × *host busy* line up, which is
why it reads as "sometimes".

## What other Windows IMEs do (grounded in `references/`)

| Project | Where the candidate UI runs | Evidence |
|---|---|---|
| mozc / Google Japanese Input | separate `mozc_renderer.exe`; TIP sends `RendererCommand` protos over IPC | `references/mozc/src/renderer/win32/` (7,751 LOC), `renderer/*` client/server/launcher (3,654), `ipc/*` (4,816; Win32 named pipe + `SECURITY_ATTRIBUTES` for low-integrity clients at `ipc/win32_ipc.cc:117-122, 466-485`), `protocol/renderer_command.proto` (160); TIP side `win32/tip/tip_ui_handler_conventional.cc:46` |
| Microsoft's own IMEs | separate `TextInputHost.exe` | system |
| Weasel (RIME) | `WeaselServer.exe` draws the UI; TSF is a client | public architecture, not in `references/` |
| PIME | engine out-of-proc (`PIMELauncher` + backends) but the candidate window **in-proc** (`libIME2` `Ime::CandidateWindow`) | `references/PIME/PIMETextService/PIMETextService.cpp:229-231, 316-328` |
| khiin-rs | in-proc; updates paint synchronously with `RedrawWindow(RDW_INVALIDATE \| RDW_UPDATENOW)` | `references/khiin-rs/windows/ime/src/ui/candidates/candidate_window.rs:164-178` |
| **Taigi Keyboard** | in-proc; `ShowWindow` then wait for the host's `WM_PAINT` | `window.rs` `show_at` |

Why the large projects pay for a renderer process (all follow from "a TSF DLL runs inside someone
else's process"): integrity-level / AppContainer hosts cannot draw or read settings; host-crash
isolation; the UI thread is the host's; 32/64-bit hosts each load their own copy; heavy UI
frameworks cannot be loaded in-proc; one shared D2D/DWrite/font instance; anti-cheat / DLL
allowlists; the UI becomes a testable exe. macOS already has this model (IMKit is out-of-proc).

## Options (cost, no scheduling)

### A. Leave as is

Measured gap is 2-5 ms in steady state; one 171 ms first-show outlier in Chrome.

### B. In-proc synchronous paint (khiin's route) — ~50 LOC, one PR

1. After `Surface::apply` (`session.rs` ~`1331-1350`) releases `presenter.borrow_mut()`, force the
   paint synchronously (`UpdateWindow` / `RedrawWindow(RDW_UPDATENOW)`). Today the borrow is still
   held when `show_at` returns, so a synchronous `WM_PAINT` would hit `try_borrow_mut` failure in
   `window.rs` `WM_PAINT` and merely re-invalidate — the borrow order must change first.
2. Create the Direct2D surface at `PopupWindow::create` instead of on first paint (removes the
   43 ms first-show cost per host process).
3. Residual: host busy time (Chrome's 126 ms) cannot be removed in-proc; with (1) the window and
   its content arrive together, so it reads as "window a little late", not "empty frame".

### C. Separate renderer process (mozc's route) — est. 2-3k LOC, 4-6 PRs

mozc answers every design question (launcher / restart, window ownership across hosts, pipe ACL
for low-integrity hosts, message shape, candidate-click round trip). Still to build in Rust:
IPC client/server (~800-1,200), launcher (~300), proto (~100), move `CandidateWindow` into the
new exe and wire it (~500), installer / signing / update / `install-dev.ps1` for one more exe,
all dogfood-only. Runtime cost is light (one idle process, one IPC per update, resources shared
instead of per-host); complexity is the cost.

Worth it only when in-proc hits a wall B cannot fix: an AppContainer host that cannot show the
window at all, or a host that stalls and drags the candidate window with it. Not for this 171 ms.

## Loose end

The box's dev install (`C:\Users\minsi\Workspace\taigikeyboard`, `main` = `7efa0b56` + uncommitted
timing patch in `window.rs` / `candidate_window.rs`) is still the **instrumented** build, writing
`%TEMP%\taigi-paint-timing.log`. Revert with `git checkout -- windows/` + `install-dev.ps1 install`
(+ log out / in) before the next Windows dogfood, or fold it into the next Windows round's install.
The patch itself is at the Mac scratchpad only; nothing of it is committed.
