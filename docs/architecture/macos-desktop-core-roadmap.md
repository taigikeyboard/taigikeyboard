# macOS over desktop-core — Roadmap

> **Type**: Design record (multi-PR refactor, complete)
> **Keywords**: `macOS`, `desktop-core`, `refactor`, `FFI`, `behaviour oracle`, `parity`
> **Status**: Complete. `AppConfig.platform_id` has since left the wire, so `DesktopPlatform` no longer selects an engine `platform_id` (C1 below is history).
> **Source**: measured inventory `docs/reports/2026-10-02-macos-desktop-core-inventory.md` (frozen on `e330cd30`)

---

## Mandate

The macOS input method links the shared Rust `taigi-desktop-core` instead of re-implementing desktop behaviour in Swift, so a desktop feature is written once for macOS, Windows and Linux.

Carried over from the maintainability mandate (`maintainability-roadmap-2026-10.md` § Mandate): a stored **setting** may change; user **data** — the engine `userdata` stores, copied-in fonts — survives every phase.

## Summary

- **Moved to the core**: the key path — key classification, bindings and chords, the composing manager, the intent executor, auto space, full-width punctuation, next-word, the candidate list model, the engine-settings snapshot and the engine bring-up (about 4,300 Swift lines deleted).
- **Left in Swift**: what is IMKit / AppKit / SwiftUI, plus candidate-window geometry, the settings storage backend, the global-shortcut tier and the update flow (§ Outside this plan).
- **desktop-core is the behaviour oracle** for the three desktops (D8; `docs/contributing/windows-guidelines.md`). A platform difference is a named `DesktopPlatform` value with a test per platform.

## Inventory and scope

Per-file rows with `file:line` on both sides, every divergence and the tests without a Rust equivalent: inventory report §2–§7.

| Group | Swift | Duplicate | In this plan | Why / why not |
|---|---|---|---|---|
| Composing, policies, next-word, controller switch, candidate list model | 3,102 | ~1,580 | **yes** (~1,580) | Faithful port; highest drift cost |
| Keys: classifier, bindings, chords, slot keys, tone scheme, symbol-picker classifier | 1,297 | ~963 | **yes** (~963) | Same; needs the platform rules first |
| Engine bridge: composing, next-word, lexicon, phonetics | 872 | ~872 | **yes** (~872) | Callers are the groups above or dead code |
| Engine settings snapshot, source toggles, dictionary artefacts and stamp | 436 | ~406 | **yes** (~406) | The core builds `AppConfig` and installs the lexicon |
| Dictionary Search (service, page, URL builder) — dead on macOS | 501 | 501 | **yes** (delete) | No production caller (report S21) |
| Shortcuts tier (roster, recorder policy, conflicts, hotkeys, migration) and the menu / Telex-guide / mode-flash panels | 1,834 | ~265 | no | Different design on macOS (`KeyboardShortcuts` + Carbon hotkeys); behaviour change + migration |
| Candidate-window geometry | 5,397 | ~1,328 | no | 71% is AppKit; three pixel divergences; scroll ownership |
| Settings storage, page models, user-data client, strings, symbols, font library, bootstrap | ~5,800 | ~1,700 | no | Persisted-format change, or stateful UI models over the boundary |
| Update check / verify / install | 1,731 | ~132 | no | Three blockers; trust decisions stay in Swift |

In plan: about 4,300 hand-written lines out. The `composing` / `nextword` / `lexicon` / `phonetics` / `case` `*.pb.swift` files (7,757 generated lines) stop being generated for macOS **one by one, as each loses its last reference** — the retained user-data UI keeps `envelope` and `user_data`.

## Design

### D1. One static library (grounded in code)

The engine and desktop-core reach the app in **one** archive: two Rust static libraries that each contain `dispatch` risk either a link failure on duplicate symbols or two copies of the engine's static state over the same `.db` files, depending on what the linker keeps — neither is acceptable.

A new crate `macos/crates/taigi-macos-ffi` (own workspace `macos/Cargo.toml`, the `linux/` and `windows/` shape) depends on `taigi-desktop-core`, on `dispatch` with `user-data`, and on `swift-ffi` — which gains `rlib` beside `staticlib`, the `taigi-linux-ffi` precedent (`linux/crates/taigi-linux-ffi/Cargo.toml:11-13`). `build-macos-xcframework.sh` builds this crate instead of `swift-ffi`; the xcframework and its module keep the name `RustTaigi`, so `Package.swift` does not change (it finds the library through the xcframework, whatever the archive is called). iOS keeps building `engine/swift-ffi` alone and its staging is verified unchanged.

A P2 spike proved it for both slices: the archive exports all five engine bridge functions plus `desktop_request_bytes` with no duplicate global symbol, both bridges' `SwiftBridgeCore.{h,swift}` are byte-identical (staged once), and Swift links and calls both seams. The crate's library is `taigi_macos_ffi`; the xcframework carries `libtaigi_macos_ffi.a`.

### D2. Boundary mechanism — a second bytes seam

The mechanism macOS already uses for the engine. One more bridged function, `desktop_request_bytes(&[u8]) -> Vec<u8>`, carrying protobuf messages defined in `macos/crates/taigi-macos-ffi/proto/desktop_shell.proto` (prost in Rust; `gen-macos-protos.sh` emits the Swift).

- No handle crosses the boundary: the runtime is a process singleton inside the crate, a session is addressed by a token — the engine adapters' shape (`docs/contributing/rust-ffi-safety.md` §1.1).
- **No new `unsafe`**: `&[u8]` in, `Vec<u8>` out is what `process_request_bytes` does today.
- Both sides get generated types from one schema — no hand-synchronised constant tables.

What the choice costs, and how the plan pays it:

- **A token is identity, not ownership.** Nothing frees a session when Swift forgets it. The contract (D3) names activation, release, a never-reused token and stale-token rejection, and P9 tests them.
- **The safety rules do not come for free**: `catch_unwind`, the request size cap, the error envelope and the malformed-input tests (`rust-ffi-safety.md` §1.2, §1.4, §1.5, §6 T1 / T4 / T5) are implemented and tested for the new envelope from P2 (the Version request, Codex pre-review), and P8 / P9 extend the tests to each request they add.
- **Every key encodes a request and copies the candidate list** into the reply. Held keys and long lists are part of the device pass.
- **The local schema mirrors native enums**; every effect and action translation has a test, and an unknown request variant is refused.

Alternatives considered, both workable: a hand-written C ABI with opaque handles like `taigi-linux-ffi` (pointer / free contracts and accessor functions on both sides; its consumer is C++, which has no protobuf runtime — Swift already links SwiftProtobuf), or swift-bridge opaque types (automatic release, but a new ownership and error surface, and generated destructors to square with the panic rule).

### D3. Boundary shape — one request in, one reply out

| Request | Carries | Reply |
|---|---|---|
| `Configure` | data directory, dictionaries directory (each optional), dictionary stamp, system locale | the settings whitelist: name, kind, default (D5) |
| `Prepare` | — | lexicon install stats |
| `Activate` | token | — (claims the coordinator; `activateServer`) |
| `Key` | token, the `NSEvent` fields — `keyCode`, `characters`, `charactersIgnoringModifiers`, `modifierFlags`, `specialKey` — panel state | `handled` + effects |
| `CommitComposition` / `Cancel` | token, panel state | effects |
| `CommitForSymbolPicker` | token, panel state | effects — the highlighted cell, else the composition as typed with its auto space; chosen by the highlight, as the legacy back end (P9) (P7: replaces `PickCandidate`, which no macOS path sends — a click only selects, `CandidateItemView.swift:54`; the Linux shell's `commit_for_picker`, `session.rs:392-417`) |
| `InsertSymbol` | token, symbol, panel state | effects (the Swift picker writes through the core's `insert_symbol`, `intent_executor.rs:179-189`) |
| `Represent` | token, refetch flag, panel state | the list unchanged, repainted in place, or closed (after a Swift global action switched a mode; `represent_list`, `intent_executor.rs:239-254`) — as effects: none, one `CandidatesChanged` (repaint in place, not anchored again), or one `CandidatesClosed` (P9) |
| `Release` | token | — |
| `Press` / `Chord` / `ComposingShortcuts` / `SymbolPickerKey` | a recorder press; a key and modifiers; nothing; a picker key — the last two with the settings snapshot, required | the key rules of the Shortcuts pane and the symbol picker (P14): what a press records or why not, the chord a key makes, the resolved composing rows with their defaults, groups, labels and the pane's fixed rows, the picker's reading of a key. Pure — no token, no session, answered before `Configure` and outside the session lock; the snapshot is read for the one answer and never installed (`key_rules.rs`) |
| `DictionarySearch` | the query, with the settings snapshot, required | the Manage Dictionaries search (#792): the first five hits of desktop-core's one lookup (`engine::dictionary_search`, the Windows and Linux one), each with its romanization, Hanji, badge i18n keys and lookup URLs. Refused before `Configure`; answered outside the session lock and sent from a background task; the snapshot is read for the one answer and never installed (`dictionary_search.rs`) |

Every request may carry the key-path settings snapshot beside it (D5). The session requests share one reply, `SessionReply { ignored, handled, effects, is_composing }` (P8). Effects (replayed by Swift in order, after the call returns): `SetMarkedText { text, caret_utf16 }`, `ClearMarkedText`, `InsertText`, `SwapPrecedingSpace { replacement }`, `ArmSwap`, `CandidatesChanged { cells, leads_with_literal_roman, marked_text_length_utf16 }` (the length anchors the caret walk), `CandidatesClosed`, `Navigate { direction }`. A cell carries what Swift's `CandidateCellContent` holds — the leading script's text and the other script as its annotation; which candidate and script a cell commits stays in Rust, where the commit resolves it. Every reply also carries `is_composing` (P7: the Swift protocol asks it after each key and around the picker's commit; the core back end answers from the last reply, and `owns` from its own `Activate` / `Release` record).

- **Record, then replay.** The Rust surface that implements `IntentSurface` only records; Swift talks to the IMK client after the call returns, so the coordinator lock is never held across a client call and a re-entrant client callback cannot deadlock on it — the Linux shell's rule (`desktop/crates/taigi-desktop-core/src/session/executor.rs:1-5`). The order among client calls is preserved. What changes is that the engine calls of one key now all happen before its first client call (before P7: append → `setMarkedText` → fetch, `TaigiInputController.swift:788-790` @ `96930b1c`); no client can observe that.
- **Panel state travels in** with every request that can act on the list: the selected index, the slot → index map, whether a list is on screen, whether a swap is available. `IntentSurface::selected_index` / `index_for_key_slot` answer from those values; each arm of `perform_intent` reads them once, before it produces any effect (`intent_executor.rs:144-161,304-313`). The swap's client check (`TaigiInputController.canSwapAutoSpace`, P7) runs in Swift before the call; Swift keeps its replacement range and its re-arm arithmetic.
- **The panel is the truth about what is shown.** macOS drops a list it has no caret rectangle for (`TaigiInputController.swift:1065-1078`). When a request says "no list on screen" and the session still holds one, the session clears it before it classifies anything, so Space and the slot keys never reach candidates the user cannot see. Selection after a refetch stays the panel's (macOS keeps the absolute index; the Linux reset at `desktop/crates/taigi-desktop-core/src/session/executor.rs` `list_changed` is a Linux-shell policy).
- **One request at a time.** The shell holds one lock across a request's snapshot, its dispatch and its reply, so a request runs under the settings it carried; the list lock is taken before the coordinator's, never after. The key rules and `DictionarySearch` take no shell lock: they install no snapshot and touch no session. A poisoned lock is recovered.
- **`has_write_failed` answers `false`.** The executor asks it after writing the auto space (`intent_executor.rs:325-335`); IMKit's `insertText` reports no failure (`Composing/ClientEffectExecutor.swift:27-57`), so macOS has no failure to report today either. This is a macOS assumption, not a property of the trait.
- **Tokens.** Swift mints one per controller and never reuses it — `ComposingSessionToken.value`, from a process-wide counter starting at 1 (P10; a `UUID` before); 0 is what an unset field decodes to and is refused. `Activate` claims; `Release` is sent from `endSession` — `deactivateServer` and `inputControllerWillClose` (`TaigiInputController.swift:288-291,313-316,1194-1200`) — so a released token is not a dead one: the field takes the focus back and the same controller claims again with the same token. **Stale-token rejection (P9) is the ownership rule and nothing more**: a request for a token that is not the current owner — released, or superseded by another `Activate` — is answered OK with `ignored` set and no effects, and reconciles nothing — the non-owner rule the controller has today (`TaigiInputController.swift:716-718`); `Activate` always claims. No retired-token table: Release cannot tell a deactivation from a close on the wire, a closed controller sends nothing more, and a table would grow with every focus switch (`rust-ffi-safety.md` §6 T2). (P9 pre-review, Codex + cloud, refuted a retire-on-Release rule; USER 2026-10-02 chose the ownership rule.) A flag rather than an error code: the error vocabulary is the engine's `ErrorCode` (§1.5), whose new value would regenerate the iOS and Android bindings for a reply only macOS reads.
- **Stale replies.** Swift replays an effect only while the controller still owns the session and is in the tenure the reply was made in — a counter bumped by `activateServer` and `endSession`, so a callback that deactivates and activates again with the same token still ends the old reply; a client callback that deactivates the controller mid-replay drops the remaining effects. The replay is the controller's, so this holds for the legacy back end too (P10, an accepted change: before it, the rest of such a reply was written into the released session). Tested in `AutoSpaceControllerTests`.
- **Lifecycle has its own requests**: `deactivateServer` / `commitComposition` need `ComposingManager::commit_composition`, not `perform_intent(Commit)`, which would add an auto space (`intent_executor.rs:76-86`).
- **Key translation lives in Rust** (`taigi-macos-ffi/src/key_translation.rs`): `NSEvent` fields → `KeyEventSnapshot`, Carbon key codes → the core's key space — the `taigi-linux-platform` precedent (`linux/crates/taigi-linux-platform/src/key_translation.rs:195-202`), table-tested without an event. Modifiers map by role (⌘ → `win`, ⌥ → `alt`); Caps Lock, the numeric-pad, Help and Fn bits are dropped; any `specialKey` makes a named key (Return and Delete included, as AppKit names them), six of them the navigation keys; only `1`–`9` and `;` get a code, the ten keys the core reads one for. A bare forward Delete (`F728`) stays what it is on the key path — mapping it to `\u{7F}` there would make it Delete Backward; the recorder's blanking of it is translated where the recorder is wired (P14).
- **What stays in the controller**: IMK overrides, menus, panels, the caret rectangle, the Carbon hotkey handlers and what each global action does, the symbol picker's window, chord and placeholder. The shortcut-target registry too: Swift's `claim` / `release` clear it (`ComposingSessionCoordinator.swift`), and the Rust coordinator has no counterpart, so the core back end keeps that call in Swift around `Activate` and `Release` — and it must outlive the legacy back end's deletion (P13). Since P10 the coordinator holds only the owner and the registry; the Swift `ComposingManager` belongs to the legacy back end, which starts a fresh engine session when `claim` / `release` answers that ownership moved — a coordinator that drove it would hand the engine a Swift generation under the core's composition. The core back end answers `owns` from the coordinator, which records the same `Activate` / `Release` the core does.
- **The Swift side (P7)**: protocol `ComposingBackend` (`macos/Sources/TaigiInputMethodCore/Composing/ComposingBackend.swift`) — `owns`, `isComposing`, `activate`, `release`, `key`, `commitComposition`, `commitForSymbolPicker`, `insertSymbol`, `represent`. Each request carries the controller's settings store (what P8 encodes as the snapshot) and the panel state, whose selection, slot and swap answers are closures: the legacy back end asks them where the old key path did, the core back end before it encodes. `Cancel` is not on it — only the core back end sends it, after `FAIL_INTERNAL` (D4). `TAIGI_COMPOSING_BACKEND` picks the back end per process (unset or `core` since the cut-over, P12; `legacy` until P13 deletes it; an unknown name stops the process). P13 deleted the protocol, the legacy back end and the variable: the controller holds `CoreComposingBackend.shared`, and the request and effect values live in `Composing/ComposingExchange.swift`.

### D4. Threading and errors

- Every call is made on the main thread — IMKit delivers `handle(_:client:)` there and the controller is main-actor bound (`TaigiInputController.swift:1543-1582`). The Rust runtime still locks per call (`runtime.rs:128-132`), so a stray call from another thread is serialised, not undefined. One exception: `DictionarySearch` is sent from the settings pane's background task (`Lexicon/DictionarySearch.swift`); it takes no shell lock, and the engine's own per-request locks order it against the key path (`rust-ffi-safety.md` §6 T3, `session.rs` `a_dictionary_search_runs_beside_the_key_path`).
- The engine finishes opening the user data on a thread of its own, as today (`runtime.rs:153-155`).
- Malformed bytes answer `FAIL_PARSE`; an oversized request `FAIL_INVARIANT`; a panic `FAIL_INTERNAL`. No Rust error type crosses.
- **After `FAIL_INTERNAL` the key is consumed, not handed to the host**: the engine may already have advanced, and a host that processed the key again would type it twice. Swift clears the marked text, closes the list, sends `Cancel`, and logs.
- **Classified by whether the engine may have run** (P10): a response that does not decode, an OK without a session reply and an effect Swift cannot replay are recovered from as `FAIL_INTERNAL` is; a request refused before dispatch (`FAIL_PARSE`, `FAIL_INVARIANT`, an encode failure) changed nothing and is answered like a non-owner — the key goes to the host, the composition stays, and a refusal that recurred cannot eat every keystroke. A `Represent` that fails internally only closes the list: it has no client to clear marked text from, and a refetch never changes the composition. Logs name the request and the error, never what was typed.

### D5. Settings stay in `UserDefaults`

The 43 `@AppStorage` bindings and the KVO observer stay. Swift sends the **key-path settings only** — a fixed whitelist, each as a typed entry (boolean or text), an absent key meaning "default" — and `taigi-macos-ffi` writes them into a `SettingsDocument` behind a `SettingsProvider` (`settings/mod.rs:60-64`). Not the raw defaults domain: the document's JSON form nests values under `values` (`settings/document.rs:44-51`), so a flat dictionary would parse into an empty document, and the domain also holds `Date` and `Data` values the document has no type for.

- The snapshot is synchronous and ordered: Swift rebuilds it from `UserDefaults` for **every** request after `Configure` (as built in P6 — no KVO, no dirty flag: nothing can go stale between a write and the next request, so a mode toggled by a global action is in force for the `Represent` that follows). Each snapshot replaces the whole last one. **Since P8** (USER 2026-10-02) the snapshot travels inside each request — an optional field of `DesktopRequest`, applied before dispatch, as the engine envelope carries `config_snapshot` — so a key crosses the seam once and its settings and its request apply in one step; the separate `Settings` request and the Swift pre-push are gone. A refused snapshot (unknown, repeated or value-less name, wrong kind) refuses its request with `FAIL_INVARIANT`, and a request refused for any reason leaves the previous snapshot in force.
- The macOS spellings: the chord letters `d/o` (report K2) are read by the core in the `MacOS` grammar; a number where a boolean is expected (report S26) is normalised by the Swift side, which reads every whitelisted name with the expression `SettingsStore` reads it with (`object(forKey:) as? Bool`, `string(forKey:)`) — so no Rust replica of Foundation's number bridging exists — and `taigi-macos-ffi` checks the wire kind.
- `settings_store` is `None`. Nothing on the key path writes settings (`perform_intent` and `ComposingManager` only read; `runtime.rs:94-108` is called from shell chrome). What `None` switches off — launch shortcut reconciliation (`runtime.rs:193-199`), mode toggles, the recent-symbol list — stays in Swift, where it is today.
- The whitelist is the core's (`taigi-desktop-core/src/settings/key_path.rs`, next to the readers it lists), answered by `Configure` with each name's kind and default; Swift keeps no second list. A Swift test compares each whitelisted default with `SettingsStore`'s and the name sets both ways.
- The system locale is the one `Configure` carried — a launch snapshot; no macOS caller reads the core's strings today.

### D6. `DesktopPlatform` in desktop-core

A copyable value `{ Windows, Linux, MacOS }` — not `cfg(target_os)`: the desktop crates' tests run on a Mac host and on Linux CI, and each platform's rules must be testable on both.

- **Pure rules take it as an argument** (or a small rules value derived from it): `ComposingKeyIntent::intent` (the width-flip / document-text helpers take none — the width flip is ⌃ / Ctrl on all three, `ComposingKeyIntent.swift:418`), the chord codec — `from_raw`, `translate_raw`, `raw_value`, `make`, `matches` must agree on one spelling and one normalisation (`keys/chord.rs:72,134,145,194,226`) — and the Shortcuts labels. None of them has a runtime in hand (`keys/intent.rs:136-141`).
- **`RuntimeParts` carries it** for the runtime's own consumers; `app_config` and `user_data::open` receive it, since `EngineSettings` alone does not reach them (`engine/bridge.rs:113-125`, `engine/user_data.rs:31-45`).
- It selects: the engine `platform_id`, the user-data journal, the caret-chord modifier, the chord-letter spelling, the reserved chord keys, the case fold of a chord key, the Shortcuts labels.
- Windows and Linux pass their own value and keep every rule they have today. Call sites that change: `windows/crates/taigi-windows-tsf/src/session.rs:194`, `runtime.rs:69`; `desktop/crates/taigi-desktop-core/src/session/key.rs`, `linux/crates/taigi-linux-core/src/runtime.rs:115`; both settings apps' `user_data::open` (`windows/crates/taigi-windows-settings/src/user_data.rs:22`, `linux/crates/taigikeyboard-settings/src/user_data.rs:27`) and Shortcuts pages (`windows/.../winui/pages/shortcuts.rs:76,102`, `linux/.../pages/shortcuts.rs:55,80`); inside the core `keys/bindings.rs:85`, `keys/shortcut_actions.rs:196`, `keys/shortcut_labels.rs:23,38`, `engine/composing.rs` (eight `app_config` calls), `engine/nextword.rs:93`.

### D7. Build, bundle, CI

- **Archive**: `make build` / `make macos-libs` build `taigi-macos-ffi` for both darwin triples with `MACOSX_DEPLOYMENT_TARGET=14.0` (matching `LSMinimumSystemVersion`, `macos/App/Info.plist:78-79`) and `lipo` them — the script's existing shape. The **new workspace's** release profile governs the archive, so it restates the engine's (`lto`, `codegen-units = 1`, `strip`, the unstripped `build-override`, `panic = "unwind"`).
- **Staging** (P2): `engine/scripts/lib/swift-bridge-artifacts.sh` finds each bridge's build-script output by package name and stages extra `(out_dir, bridge_name)` pairs into the one `RustTaigi` module, `SwiftBridgeCore.swift` once after checking every copy is identical. The iOS path through the same helper was re-run (release and `--dev`) and its staged files compared unchanged.
- **Features**: `e2e-trace` and `panic-injector` are forwarded through the new crate; the release script keeps `e2e_trace_assert_absent` (`build-macos-xcframework.sh:85`). The injector's symbol is present in every build by design — only its body is gated (`engine/swift-ffi/src/lib.rs:118-132`).
- **Bundle**: `bundle-app.sh` keeps its architecture, deployment-target and symbol assertions and, from P6 (the first production caller), asserts both entry points in the executable; `build-macos-xcframework.sh` asserts both in every archive slice (P2).
- **Caller's view**: `make -C macos build / test / install` are unchanged; `macos/Makefile` still never rebuilds Rust.
- **Stale-artifact gate** (`AGENTS.md` § Build & Test): a diff touching `desktop/` or `macos/crates/` needs `make build` before a macOS build or test.
- **Tooling**: `macos/rust-toolchain.toml` (both darwin targets); `tools/test_select.py` maps `desktop/**` and `macos/crates/**` to **two** macOS gates — the native Rust tests and, after a rebuild, the Swift suite; `.github/workflows/macos.yml` adds the paths, caches the new workspace and runs the native tests, clippy, fmt and the MSRV check; `.github/workflows/security.yml` and the advisory checks cover the new lockfile; `make lint` and `tools/release_notes.py` (which implements `make version-desktop`) learn the workspace.

### D8. The oracle rule

`windows-guidelines.md` § "macOS is the behaviour oracle" becomes: **desktop-core and its tests are the behaviour oracle for the three desktops.** A rule lives there once; a platform difference is a `DesktopPlatform` branch with a test per platform, or a named shell divergence. The rule is swapped in the cut-over PR. The 275 comments in 90 Rust files that cite a `*.swift` file as a rule's origin are rewritten to state the rule in a separate, last PR.

### D9. Proving behaviour is frozen

No single check proves it; the package is:

1. **One suite, two back ends.** The controller's key path goes behind a Swift protocol (`ComposingBackend`) with the existing code as its legacy implementation; the core-backed implementation is added beside it and the existing controller suites run against each. **Each back end runs in its own `swift test` process**, selected by an environment variable: both would otherwise drive the one process-wide engine with separate generation counters (`Composing/ComposingSessionCoordinator.swift:46`; `composing/coordinator.rs:61-66`) and contaminate each other. The suites' settings fixtures write `.standard` (`AutoSpaceControllerTests.swift:13-23`); the core back end gets the equivalent injection through the snapshot each request carries — built from the request's own store, so a suite whose engine settings live in `.standard` gives its controllers `.standard` too (one domain, as the shipped app has; the whole domain put back at teardown). `make -C macos test` runs the suite once per back end; in the core run the cases that drive the engine through the Swift `ComposingManager` or the bridge's composing ops skip themselves (legacy machinery, run by the legacy process — the second since P12, which runs the shipped core first; `TestFixtures.skipUnderTheCoreBackEnd`), so the two generation counters never meet in one process. P13 deleted the legacy back end, and with it the second process and the skips.
2. **Ported unit tests.** Tests of deleted Swift types are ported to desktop-core first (report §2, §3 list them), under the `MacOS` platform value.
3. **Native tests of the boundary**: key translation tables, every effect and request translation, token rules, the envelope errors.
4. **Integration tests the suites do not reach**: launch order and once-only preparation, journal and dictionary stamp, the settings snapshot (change, reset, removal), lifecycle requests, a deactivation during replay.
5. **A device pass** at the cut-over, before any legacy code is deleted: IMK re-entrancy, caret geometry, focus changes, held keys, long lists.

## Parity decisions

**Preserved on every platform, no decision needed** — the core gains the macOS value as a `DesktopPlatform` rule, and Windows and Linux keep theirs: modifier roles (K1), chord letters (K2), key codes (K3), `platform_id` (C1), journal (S8), engine bring-up at launch (C6), dictionary stamp (S24), labels (K9), **reserved chord keys (K4)** and **case fold of a chord key (K6)**. The last two were candidates for one shared rule, but the Windows recorder manufactures private-use scalars for F1–F24, Insert, Home and End (`windows/crates/taigi-windows-platform/src/key_translation.rs:232-252,263-268`), so narrowing the reserved range would change what it accepts.

**Rare inputs where macOS and the core differ in the classifier itself** — macOS takes the core's rule.

| # | Input | macOS today | desktop-core today | Status |
|---|---|---|---|---|
| E2 (C3) | One key event carrying several characters (`a.`, `a5`) | every character must qualify (before P11c: classified by the first character) | every character must qualify — a deliberate rule (`keys/intent.rs`) | **Settled P11c #356** — core rule on macOS (USER 2026-10-02): romanization / tone tier asks every character (`a.` mid-composition commits with the event as text; `a5` is input only where a digit is a tone — composing under Standard); a Telex key is a one-character event (`fx` / idle `vx` are letters, `f.` is text); a digit slot reads the first scalar (`1⃣` picks, as `direct_selection_slot`). Escape / Backspace tier, bare-key slots, chord match, width flip already agreed. Dead-key / custom-layout / input-source output only. Pinned: `intent.rs` `e2_a_multi_character_event_is_classified_by_every_character`, `under_telex_a_digit_slot_reads_the_first_scalar` (ported from the Swift classifier's tests in P13); `ParityDecisionTests` E2 cases (through the controller) |
| E2b (K8) | A plain Escape inside a multi-character event | whole string (before P11c: first character) | whole string | **Settled P11c #356** — core rule on macOS (USER 2026-10-02): `isPlainEscape` compares the whole string like `is_bare_escape`, so `"\u{1B}x"` no longer closes and swallows — the symbol picker closes and passes it through, and with the Telex guide up the guide hides and the event goes on (Cancel when composing, host when idle). The composition's Escape tier reads the first character on both (Cancel), unchanged. No single key press carries it. Pinned: `symbol_picker.rs` `e2b_an_escape_inside_a_longer_event_closes_and_passes_through`, `TaigiInputControllerTelexGuideTests` `testAnEscapeInsideALongerEvent_*` |
| E4 (K7) | Format characters (Cf) as "not text" | no — Cc only (before P11d: Cc + Cf, `CharacterSet.controlCharacters`) | no (Cc only) | **Settled P11d #358** — core rule on macOS (USER 2026-10-02): `isTextScalar` refuses Cc and the F700–F8FF function-key scalars only. A Cf key, isolated (`U+200B`, `U+00AD`, `U+FEFF`) or embedded (`x‍y`, `👩‍💻`), typed mid-composition commits with the composition in one write instead of after it (with Auto-Space on, the space leads it: `taigi`, space, U+200B), and that commit-then-insert drops the next-word context where the raw commit reported the composition as the selected word — as for `.` typed mid-composition; idle it is still the host's, and is now document text, so the next-word gate sees it — an isolated Cf or an emoji ZWJ sequence is forwarded and read by the engine as noise that keeps the context, `x‍y` stops on its letter. Cc (C0, DEL, C1) unchanged. Layouts / input sources that emit Cf only. Pinned: `intent.rs` `e4_a_format_character_is_text`, `tests/composing_manager.rs` `e4_a_format_character_reaches_the_next_word_gate`, `KeyEventSnapshotTests` `testFormatCharacters_areDocumentText_andControlCharactersAreNot` (the Swift `documentText` predicate), `ParityDecisionTests` E4 cases (through the controller) |
| E5 (C2) | "Is a letter / is whitespace" in the manager — it gates the next-word notification | per scalar (before P11e: per grapheme, `Character`, first scalar of each) | per scalar (`char`) | **Settled P11e #359** — core rule on macOS (USER 2026-10-02): the Swift gate asks every scalar for Alphabetic / White_Space (`unicodeScalars`, `properties`). A grapheme led by a non-letter, non-whitespace scalar and carrying an Alphabetic or White_Space one — an Other_Alphabetic mark (`。` + U+0345, `,` + U+093E; 1,353 marks after `。`) or a Prepend scalar before a letter / space (`\u{600}a`, `\u{600} `) — is no longer forwarded to next-word: no notification is sent, so the existing context is left unchanged. A non-Alphabetic mark (U+0301), whitespace-led graphemes, single scalars (0 of 1,112,064 differ), the shipped symbol table, full-width and attaching punctuation: unchanged. Only the idle pass-through of a key event the host types reaches the difference — layouts / input sources that emit such a sequence outside a composition. Probed over every scalar (Swift 6.4 on macOS 27 against rustc 1.97 — each side's Unicode tables can move on their own), Swift and the core agree on every scalar except U+F882–U+F8D6 (Apple's ICU tags 35 private-use scalars Alphabetic), which the AppKit function-key range keeps from the gate. iOS / Android keep their own gates. Pinned: `tests/composing_manager.rs` `e5_a_mark_that_is_alphabetic_keeps_the_character_from_next_word`, `e5_every_bundled_symbol_reaches_next_word` (the Swift manager's cases went with it in P13) |
| E6 | Chord gate's typing-key test (`is_typing_key`) on a key whose Unicode fold adds a combining mark (`İ` → `i̇`) | first scalar `i` — a typing key, so refused bare (before P11b: first grapheme, not ASCII, so it recorded) | first scalar `i` — a typing key, so refused bare | **Settled P11b #355** — core rule on macOS (USER 2026-10-02): the Swift gate asks the first scalar of the folded key, so bare and ⇧`İ` are refused by the recorder and a stored `s\|0069,0307` reads as a cleared row on both back ends; ⌃`İ` binds; `Ñ` (binds bare) and the Kelvin sign (refused) unchanged. The same holds for any typing key followed by a combining scalar (a decomposed `á`, a keycap `1⃣`). The resolver rules are E7's: the row stays empty, `confirmHighlighted` / `commitLiteral` take back a free commit default, the stored string is left as is. The global tier shares the gate: its recorder refuses the same presses, and a stored global shortcut whose bridged key now reads as a typing key is cleared at launch by that tier's existing typing-key sweep (`ShortcutConflicts.resolveAcrossRegistries`), as a bare `z` row is. Pinned: `chord.rs` `e6_a_fold_that_adds_a_combining_mark_is_a_typing_key_on_the_mac`, shared parse and resolve rows in `chord_rules.tsv` (the Swift gate went in P14) |
| E7 | A stored chord with an empty hex field (`c\|0041,,0042`) | whole value refused (before P11a: field skipped, chord parsed) | whole value refused (`translate_raw`) | **Settled P11a #354** — core rule on macOS (USER 2026-10-02): the Swift codec keeps empty fields, so `c\|0041,,0042`, `c\|,005D`, `c\|005D,` are refused whole on both back ends. A refused value reads as an explicit clear and then goes through the existing resolver: the row stays empty (its default is not restored), except `confirmHighlighted` / `commitLiteral`, which take back a free commit default; the stored string is left as is. Hand-edited values only (`rawValue` / `raw_value` never write an empty field). Pinned: `chord.rs` `e7_a_stored_chord_with_an_empty_hex_field_does_not_parse`, shared parse and resolve rows in `chord_rules.tsv` (the Swift codec went in P14) |
| E8 | A recorded chord matched against an event whose key is a canonically equivalent spelling (`c\|00E9` stored, `e` + U+0301 typed) | scalar by scalar (before E8: `matches` compared `String`s, so the picker read the decomposed `é` as the stored chord — Swift's canonical equivalence) | scalar by scalar (`matches`, derived `Eq` over the string) | **Settled #363 `fd40fa0e` (P14 pre-gate)** — core rule on macOS (USER 2026-10-02 rule for rare inputs): found by the P14 Codex pre-review. Only the symbol picker reached it — the composition's classifier is the core's since P12. Chord identity (`==` / hash, the resolver's duplicates, the cross-registry scans) was already the raw value's (`RawRepresentable`), so scalar identity there is unchanged; the reserved set and the slot row hold no canonically equivalent key. Layouts / input sources that emit decomposed letters, or a hand-edited value, only. Pinned: `chord_rules.tsv` `match` rows `c\|00E9` × `00E9` / `0065,0301` and the `resolve` row keeping both spellings (the Swift matcher and its two tests went in P14) |

Each change lands in its own `parity:` PR with both sides described and a test pinning the result (`cross-platform-alignment.md` §1b), never inside a refactor PR. A divergence of another kind that a ported test uncovers — anything that is not a rare-input classifier rule — stops the phase and goes to the USER.

## Phases

One line per phase (labels cited from code); all merged, complete.

| Phase | Content |
|---|---|
| P1 | Dead Dictionary Search code deleted |
| P2 | `taigi-macos-ffi` crate, one archive (D1) |
| P3 | Tooling: test selection, `macos.yml` native job (D7) |
| P4 | `DesktopPlatform` through the pure rules, runtime, journal, chord codec, labels (D6) |
| P5 | macOS key-path tests ported to desktop-core under `MacOS` |
| P6 | Runtime over the boundary: `Configure` / `Settings` / `Prepare`; launch bring-up in the core |
| P7 | `ComposingBackend` seam with the legacy key path as one implementation |
| P8 | Session I: `Activate` / `Key` / `CommitComposition` / `Cancel` / `Release`; settings snapshot per request (D5) |
| P9 | Session II: `CommitForSymbolPicker`, `InsertSymbol`, `Represent` |
| P10 | Core back end in Swift; controller suites per back end (D9) |
| P11 | Parity items (§ Parity decisions): P11a E7, P11b E6, P11c E2 / E2b, P11d E4, P11e E5 |
| P12 | Cut-over to the core back end; oracle rule (D8) |
| P13 / P13b | Legacy key path and its settings snapshot deleted |
| P14 | Shortcuts pane reads chord grammar, validation, defaults and slot keys from the core |
| P15 | `*.swift` cites in Rust comments and docs updated (D8) |

## Best practices alignment

Rules per phase: P1, P7, P13, P14 — `cross-platform-alignment.md` §1 (behaviour-frozen refactor). P2, P6, P8, P9 — `rust-ffi-safety.md` §1 (seam, panic, size cap, error envelope), §5 (two reviews), §6 (test contract). P4 — §1c (no new platform reads inside shared code). P11 — §1b (a parity correction is isolated, described on both sides, pinned by a test). P5 — `known-pitfalls.md` § Trace before assert. All — no redundant fallback (`AGENTS.md` § Design principles): the legacy back end is a temporary compatibility path, **removed in P13**.

| Mainstream practice | Source | This plan |
|---|---|---|
| One host-neutral core, thin per-OS front ends | librime under Squirrel / Weasel / ibus-rime (`mainstream-ime-comparison.md:221-223`); azooKey-Desktop shares `Core/Sources/` with iOS (`:171-178`) | The whole plan |
| One runtime per process, one engine per text field, one runtime lock | ChiaKey `Frameworks/ChiaKeyCore/Headers/ChiaKeyCore/ChiaKeyCore.h` (`chiakey-reference.md:75-78`) | D3: singleton runtime, token per controller |
| One key in → one reply with commit, composition and candidates | KeyKey `Loaders/Windows-IMM/BaseIMERPC/BIServerRPCInterface.idl` `BISHandleKey` (`mainstream-ime-comparison.md:115`); hazkey `protocol/base.proto` request / response envelope (`:117`) | D3: `Key` request, effect list |
| A single bytes entry point into the core | khiin-rs `khiin/src/engine.rs:57` `send_command_bytes` | D2 |
| Config is host-owned and re-applied by the host; the core never reads a platform preference store | ChiaKeyCore (`chiakey-reference.md:81`) | D5: `UserDefaults` pushed as a snapshot |
| No raw pointer cast back to `&mut`; no panic across the seam | khiin-rs counter-examples `android/rust/src/lib.rs:51-56,64` (`rust-ffi-safety.md` §1.2, §1.3) | D2, D4 |

### Deliberately not adopted

- **A hand-written C ABI with opaque handles** (the `taigi-linux-ffi` shape, ChiaKeyCore's `ChiaKeyCoreC.h`): right for a C++ consumer; for Swift it adds a header, accessor functions and `unsafe` on both sides that the bytes seam does not need.
- **ChiaKeyCore's commit acknowledgement** (`chiakey-reference.md:79`): it guards a host whose insertion can fail; IMKit reports none (D3).
- **An out-of-process engine host** (rakukan, PIME, KeyKey's server, hazkey — `mainstream-ime-comparison.md:112-117`): the engine stays in-process on every platform.
- **`cfg(target_os)` for platform differences**: untestable across hosts (D6).
- **Leaf modules first over throw-away exports** (full-width map, auto space) and **key rules for the Shortcuts pane before the session moves**: either would create a request surface, or a second authority, that exists only for the interim.
- **A `taigi-macos-core` session crate mirroring `taigi-linux-core`**: a third copy of the per-key preamble and chrome. The preamble stays in the controller; hoisting the Windows / Linux copies into desktop-core is a separate refactor of those shells.

### Outside this plan (YAGNI, with the measurement)

- **Candidate-window geometry** (~1,328 duplicate of 5,397): 71% is AppKit; metrics differ in three places that change pixels on one platform (report W1–W3); the vertical and expandable models fight `NSScrollView` for the scroll offset.
- **Settings in `settings.json`** (~450 more lines): a persisted-format change needing an importer and the rewiring of 43 bindings, for no user-visible gain.
- **Global shortcuts as core chords** (~420 more lines): drops `KeyboardShortcuts` and the Carbon hotkeys; changes recording policy and needs a migration (report K5).
- **Settings page models, strings, symbols, font library**: stateful UI models or OS-bound halves; the i18n key sets differ by design (report S19).
- **Update flow** (~132 of 1,731): three blockers and a proxy regression (report §7).
