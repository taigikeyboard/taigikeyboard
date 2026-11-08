# Desktop TPS mode — roadmap

TPS (方音符號, the i18n `en` label "Phonetic Symbols", key `tpsMode`) as a third input mode on macOS, Windows and Linux, typed on a physical keyboard, with an on-screen key panel. iOS and Android have typed TPS since v3.5.x.

Status: shipped — every phase merged.

## Maintainer decisions

| # | Decision |
|---|---|
| U2 | TPS is not a romanization. Its switch shortcut is its own action and does not join the TL ↔ POJ toggle. |
| U3 | An on-screen key panel the user can look at or click. |
| U4 | Physical layout = the system Zhuyin (Dachen) position for every symbol Mandarin Zhuyin shares with TPS; the TPS-only glyphs on the keys Dachen leaves free and on Shift. |
| U5 | Candidate picking under TPS = arrows / Tab + Enter, plus the numeric keypad `1`–`9`. **Revised 2026-10-04 by U8.** |
| U8 | (2026-10-04, after P4 on device: no digit picking without a keypad.) TPS follows the Zhuyin input methods, not the romanizations: the candidate window is hidden while typing and opened on demand, and the number row picks inside it (D7). The preedit stays glyphs (arm A); inline Hanji conversion (arm B) is to be weighed once the TPS feature is complete. TL and POJ keep their always-on list. **Revised 2026-10-05 by arm B** (maintainer decision B3, [`desktop-tps-hanji-conversion-roadmap.md`](desktop-tps-hanji-conversion-roadmap.md), merged #398–#403): the preedit shows the Hanji conversion and arm A is gone; the window on demand and the number row stay. |
| U6 | Panel scope: macOS and Windows show it and take clicks; Linux shows it only. |
| U9 | (2026-10-04, P6.) A click on a cap types that cap's glyph on both platforms — never "the physical key pressed": with the window up, a click on `1` types ㄅ rather than picking slot 1. |

O1 (what Space does once the syllable is closed) — decided 2026-10-03, the recommended arm: see D3. Revised by D7: with no window up, Space opens it.

## Design

### D0 — One engine intent: `TpsKey`

A new composing intent `TpsKey { key }`, the TPS counterpart of `TelexKey`. In one transition the engine runs `tps_adjust` against the pending text before its own caret, applies the replacement the adjuster asks for, and inserts the glyph. `key = " "` is the separator: inserted when the character before the caret is neither a tone mark nor a space; otherwise the transition is a no-op that answers with no effects, which is how the caller knows the Space was not taken. With the caret inside the tail, a tone mark or separator after it refuses the Space too, so a syllable is never parted from its own tone mark.

Why in the engine rather than three wrappers in the desktop core: the desktop has a movable composing caret and only the engine knows where it sits in the raw buffer; one mutation renders one preedit instead of two; and the rule is then written once for five platforms. Mobile keeps its three-call path; moving it onto `TpsKey` is outside this plan.

### D1 — `InputMode::Tps`

One new variant, wire and stored spelling `"tps"`, label `SettingsTpsMode` (`tpsMode` gains the three desktop platforms). The five exhaustive matches (`settings/engine_settings.rs:27-30,38-41`, `policies/auto_space.rs:45-47`, `engine/dictionary_search.rs:100-103`, `engine/lexicon.rs:286-289`) are compile-forced decisions; the wildcard matches above become exhaustive in the same PR. A document that already stores `"tps"` (a restored mobile backup) starts reading as TPS on all three desktops — on macOS too, since its importer does not filter — so the test at `document.rs:376-386` changes its unknown value to one that stays unknown, and the variant lands together with the classifier branch, never before it.

### D2 — Physical layout (U4)

A table in the core, new file `keys/tps_layout.rs`, keyed on the character the key types on a US layout — the same assumption the slot keys make. Letters are read lowercased with the Shift modifier deciding the layer, so Caps Lock does not select it; digits and punctuation are read as typed (`!` `#` `*` `(` `)` `:` `^`), since Windows and Linux hand over the shifted character.

Base rows — the Dachen positions: every symbol Mandarin Zhuyin shares with TPS keeps its Dachen key, so a Zhuyin typist's finger memory never lands on a different Taiwanese phoneme (2026-10-07; the `rime-moetaigi` positions this replaced put ㆦ on `i`, so a hand trained on ㄛ there typed `oo`, a different word the engine does not fold). The TPS-only glyphs take the keys Dachen leaves free under TPS (ㄈ `z`, ㄓ `5`, ㄔ `t`, ㄕ `g`, ㄖ `b`, ㄩ `m`, ㄟ `o`, ㄡ `.`, ㄦ `-`) and the Shift layer, where Shift is the key's sister sound: the voiced initial (ㄅ ㆠ, ㄍ ㆣ, ㄐ ㆢ, ㄗ ㆡ), the sister nasal coda (ㄣ ㆬ n / m, ㄥ ㆭ ng), the nasalized vowel (ㄚ ㆩ, ㆤ ㆥ, ㄧ ㆪ, ㆦ ㆧ, ㄨ ㆫ, ㄞ ㆮ, ㄠ ㆯ) or the labial twin of a final (ㄢ ㆰ, ㆲ ㆱ). The TPS-only keys follow a TPS typist's request (v3.9.0 row 10c, Discord 2026-10-19, maintainer 2026-11-08): ㄧ ㄨ ㆨ in one column (`u` `j` `m`), ㆦ on Dachen's ㄡ key (the sound closest to it, as ㆤ replaces ㄟ), ㆲ bare on the free ㄦ key (5.9% of syllables against ㆱ's 0.02%), the stop codas p t k down one column:

| Key | Glyph | Shift | | Key | Glyph | Shift |
|---|---|---|---|---|---|---|
| `1` | ㄅ | ㆠ | | `8` | ㄚ | ㆩ |
| `q` | ㄆ | | | `i` | ㄛ | |
| `a` | ㄇ | | | `k` | ㄜ | |
| `2` | ㄉ | | | `o` | ㆤ | ㆥ |
| `w` | ㄊ | | | `.` | ㆦ | ㆧ |
| `s` | ㄋ | | | `9` | ㄞ | ㆮ |
| `x` | ㄌ | | | `l` | ㄠ | ㆯ |
| `e` | ㄍ | ㆣ | | `0` | ㄢ | ㆰ |
| `d` | ㄎ | ㄫ | | `;` | ㄤ | |
| `c` | ㄏ | | | `u` | ㄧ | ㆪ |
| `r` | ㄐ | ㆢ | | `j` | ㄨ | ㆫ |
| `f` | ㄑ | | | `p` | ㄣ | ㆬ |
| `v` | ㄒ | | | `/` | ㄥ | ㆭ |
| `y` | ㄗ | ㆡ | | `,` | ㄝ | |
| `h` | ㄘ | | | `m` | ㆨ † | |
| `n` | ㄙ | | | `-` | ㆲ | ㆱ † |

Stop codas `t` ㆴ, `g` ㆵ, `b` ㆻ, `z` ㆷ. Tones `4` ˋ (2), `3` ˪ (3) ‡, `6` ˊ (5), Shift+`3` ˇ (6) ‡, `5` ˫ (7) †, `7` ˙ (8, typed as U+02D9 as on mobile; the engine folds it for lookup, `engine/phonetics/src/tps.rs:262`), Shift+`6` ˆ (9) †; tone 1 and the unmarked tone 4 are Space. `=` types the hyphen, how 輕聲 `--` is written (`engine/phonetics/src/tps.rs:132-134`).

A key or Shift layer the table leaves out (`'`, `:`, `@`, `+`, a capital) and every keypad key are not TPS keys: they are document text — passed through when idle, committing the glyphs ahead of themselves while composing (D3) — so punctuation with no glyph still reaches the document, full width. `?` types ㆭ; full-width `？` stays on Ctrl+Shift+`/` (D3's Ctrl punctuation chord).

† = no Dachen position to inherit (Mandarin Zhuyin has no ㆨ ㆱ ˫ ˆ): ㆨ ˫ take free keys, ㆱ the Shift cell of ㆲ, ˆ the Shift cell of a tone key. Mobile types ㆨ ˇ from its grid and tone 9 as the digit `9`, which the adjuster turns into ˆ (`ios/Sources/TaigiKeyboard/Layout/TaigiLayouts.swift:83-98`; `engine/phonetics/src/tps_adjust.rs:268-302`); here ˆ has its own key. ㄝ is `ee`, a phoneme of its own, not a spelling of ㆤ `e` (`engine/phonetics/src/tps_ambiguity.rs:23-27`).

‡ = the one deliberate Dachen deviation: `3` types ˪, tone 3 (11% of syllables, `dictionary/output/dictionary.csv` weighted by frequency), not Dachen's ˇ, which is tone 6 (0.4%); ˇ moves to Shift+`3`, the cell beside its Dachen key. ㄛ and ㄜ are both bare keys as on Dachen: the engine spells TL `o` as ㄛ and keeps ㄜ for `er` / `or` (`engine/phonetics/src/tps.rs:53-56,67`), so a typed ㄛ finds both and a typed ㄜ only `er` / `or` words.

ㆳ (U+31B3) gets no key: it is ㆪ encoded a second time, and the engine folds it onto ㆪ before any lookup (`phonetics::fold_tps_glyph_alias`), so the ㆪ key already types the symbol.

The table was checked glyph by glyph against `engine/phonetics/src/tps.rs:17-130` and `taigi-converter/src/tables.js:75-98` (2026-10-03, again after the Dachen realignment 2026-10-07; `engine_roundtrip.rs::every_layout_glyph_begins_a_composition_the_engine_takes` keeps it so): every glyph the engine spells has a key, and every key types a glyph the engine accepts. The palatal keys `r` `f` `v` are optional — ㄗ ㄘ ㄙ ㆡ before ㄧ or ㆪ are rewritten by `tps_adjust.rs:319-332`. The explicit coda keys do not force a final reading: §35 lookup reads every member of a key's family in both directions, and a user who never learns `t g b z`, `p` `/` or the Shift nasals types the same words through the fold (§32, §33). Only a tone mark or Space pins the boundary.

### D3 — Classifier under TPS

One branch at the top of `ComposingKeyIntent::intent`, taken when the bindings say the mode is TPS (`ComposingKeyBindings::from_document` carries it — the classifier's signature and its four callers do not change), ahead of the slot tier, the composing bindings and the typing tier, so a bound Space (or a chord rebound onto a layout key) cannot intercept:

| Key | Idle | Composing |
|---|---|---|
| A layout key | `TpsKey(glyph)` — begins the composition | `TpsKey(glyph)` |
| Space | pass through | `TpsKey(" ")`; when it answers with no effects the syllable was already closed → O1 |
| Keypad `1`–`9`, no modifier, list showing | — | pick that slot |
| Keypad digit otherwise | pass through | `CommitThenInsert` |
| Enter / Shift+Enter / Tab / arrows / `[` `]` / Esc / Backspace / caret chord | unchanged | unchanged |
| Ctrl + a mapped punctuation key (`,` `.` `;` `[` …) | types the full-width mark (`ComposingKeyIntent::tps_punctuation_chord`) — the bare `,` `.` `;` type glyphs, so under TPS the chord is the punctuation key. TPS is full width only (USER 2026-10-07); an attaching mark swaps an armed auto space in full width. Outside TPS the same chord is the host's (no width flip since 2026-10-07) | commits, then types the mark |
| Any other printable | pass through | `CommitThenInsert` |

`ComposingManager::tps_key` answers `Taken`, `Refused { is_caret_at_end }` or `Failed`. Only a refused Space with the caret at the end of the composition is O1's; a Space refused inside it (beside a tone mark or separator, or at the start) and a failed round trip do nothing. Shift+Space is not the separator: it is document text — the glyphs committed as typed, then a space. The symbol picker picks with the keypad too, and confirms with Space / Enter as in every mode.

Classifying a key never asks the engine — the Windows Test phase returns before the runtime is prepared (`windows/crates/taigi-windows-tsf/src/session.rs:205-242`) — so "Space is consumed while composing" is the whole test-phase answer.

**O1 — Space on a closed syllable.** Mobile commits the glyphs as typed and writes a document space; nobody types TPS that way on mobile, where a candidate is tapped. On a desktop, Space is the key every Zhuyin-family input method confirms with. Recommended: with a list showing, Space confirms the highlighted candidate (Enter's action); with none, it commits the glyphs as typed, no space after. The alternative was mobile's rule verbatim. **Decided 2026-10-03 (maintainer: "follow your recommendation"): the recommended arm** — one arm in the executor, built in P2b.

The keypad is read from `key_code`, which Windows already fills; the Linux and macOS key-code tables gain the nine keypad codes. No snapshot field and no proto change. With Num Lock off the keypad arrives as navigation keys and behaves as those do today.

Output the Other Script is inert under TPS wherever it is bound. Under TPS these do nothing, and write no setting: the Toggle Hanji / Romanization and Cycle Candidate Display shortcuts (`windows/crates/taigi-windows-tsf/src/session.rs:888-925` and their counterparts in `taigi-desktop-core` `session/global.rs`), the Telex guide shortcut, Telex keys, the letter and digit slot sets, the Shift + slot script flip, Output the Other Script (its row in the Shortcuts pane stays; its chord never fires under TPS), No Hyphens. Full-width punctuation is on regardless of the stored swap, as on mobile (`ios/.../Settings/SharedSettings.swift:737-741`).

### D4 — What the list and the commit show

One cell per candidate: Hanji when the row has one; otherwise the row's `roman` through the phonetics op `TlDisplayToTps`. Candidate Display has no effect, as in the engine's commit (`commit_text.rs:83-91`). Commits use `CommitScript::Lead`; the raw commit writes the glyphs as typed.

Sites in `desktop/crates/taigi-desktop-core/src/` that assume a romanization and get a TPS arm:

| Site | Under TPS |
|---|---|
| `composing/cell_content.rs:55-66` | Hanji-less cell shows glyphs; Romanization Only does not hide Hanji; no roman annotation |
| `composing/presentation.rs:40-74` | One cell per candidate — no Alternate cell under Combined |
| `composing/presentation.rs:96-103` | A Hanji-less first cell is a candidate, not the typed-literal lead |
| `composing/intent_executor.rs:149-159` | No `Other` commit, no Shift-slot flip |
| `settings/document.rs:267-278` | Full-width punctuation effective value is on |
| `keys/bindings.rs:96-99` | Slot labels are `1`–`9` |
| engine `composing/src/commit_text.rs` `finalized_composition_resolution` | `CommitAsShown` under the TPS layout earns no auto space |
| `engine/bridge.rs:115-123` | `tps_or_maps_to_er = true`, the mobile default (`ios/.../Settings/SharedSettings.swift:88`); the cell's `TlDisplayToTps` call passes the same value, so a cell and its commit agree |

### D5 — Shortcuts and mode changes (U2)

| Action | Windows / Linux | macOS | Does |
|---|---|---|---|
| `ToggleTps` (new) — Switch TPS | `Ctrl+Alt+P` | `⌃⌘P` | TPS ↔ the romanization last used |
| `ShowTpsKeyboard` (new) — Show Phonetic Symbols Mini Keyboard | `Ctrl+Alt+J` | `⌃⌘J` | Shows / hides the panel (D6); inert outside TPS |
| `ToggleRomanization` (existing) | `Ctrl+Alt+C` | `⌃⌘C` | Unchanged between TL and POJ. Under TPS it leaves TPS for the *other* romanization than the one last used — it never enters TPS |

`Ctrl+Alt+T` is avoided (GNOME's terminal). Both chords pass `global_rejection` (`keys/shortcut_actions.rs:245-279`) and collide with nothing in the default roster; they are rebindable and go through the existing conflict rules. No list of system shortcuts reserves either, which is not a promise for every desktop environment or app. The panel's chord was first planned as ⌃⌘K; Apple Notes binds it (show / hide the shared-note activity list), so it moved to ⌃⌘J / Ctrl+Alt+J (maintainer, 2026-10-04). ⌃⌘P is confirmed on device in P4 (S90).

One function in the core, `settings::next_input_mode(current, last_romanization, request)`, answers every mode change the user asks for — the picker and both shortcuts — through the one writer `SettingsDocument::switch_input_mode`, which every writer in the five places listed above calls. Reset and restore need no transition: reset removes the keys, restore replaces the document. The last-used romanization is one new stored key, `lastRomanizationMode`, written only when a romanization is left for TPS, so a POJ user's round trip returns to POJ. It is typed as the two-value `Romanization`, so TPS cannot be stored in it; absent or unreadable reads as TL (a document that reached TPS without a switch, e.g. a restored mobile backup). Both new stored keys join the General reset list (`settings/keys.rs` `GENERAL_KEYS`); `ShowTpsKeyboard` joins `fires_once_per_press` (`keys/shortcut_actions.rs`). macOS keeps its settings in `UserDefaults`, so its picker and chords ask the same writer through a pure seam request, `SwitchInputMode` (`taigi-macos-ffi/src/key_rules.rs`), carrying the two stored values, and store what it answers (`SettingsStore.switchInputMode`); the inert rule is mirrored in Swift (`ShortcutAction.isInert(under:)`), and a mode written from outside the session takes down the list, the guide and the picker (P4).

**A composition across a switch** (P3). A switch that enters or leaves TPS changes the raw buffer's alphabet, so the composition cannot carry over (a glyph buffer must not take Latin keys). The switch itself leaves the composition on screen and closes its list; the next key commits it as shown and is then read as the new mode's first key (`ComposingManager::is_left_by_mode_change`; each shell's key path sends a keyless `Commit` before the classifier). `perform_intent` opens with the same commit, so the commits that run with no key — the symbol picker's commit-first, a host's Finalize, the Windows Shift-tap — write the composition as shown, with no auto space and no candidate picked. One place covers both chords, the input-method menu, the settings window (focus loss does not commit on Windows, `text_service.rs` `OnSetFocus`) and a restore, and Windows requests no edit session outside a key. The raw commit writes the preedit whatever the mode now (read by running, 2026-10-03), so no mode is pinned for it. TL ↔ POJ crosses no TPS and keeps today's behaviour: the composition carries on. A switch keeps the auto-space arm: it names a space in the document, whatever the mode. The input-method menu keeps the same rows under every mode — Fcitx5 registers its actions once and re-titles them by position, and IBus updates only the root property on a mode change — so Switch Candidate Display stays under TPS and does nothing, as its chord does (P3 cloud review).

### D7 — The candidate window on demand (U8)

**Revised by arm B (2026-10-05).** The rows below are arm A as P4b built it. Arm B (`desktop-tps-hanji-conversion-roadmap.md` § H6, merged #403) keeps the window on demand, the number row and the slot set, and changes these rows: plain ← → step the caret by word instead of opening the window; ↓ and the other opening keys list the word before the caret, not the whole buffer; a pick replaces that word and closes the window, never writing; Enter, a key that commits first and Space or a navigation key with the window off commit the composition as shown (Hanji); Commit as Typed writes the glyphs of the whole composition. The table is kept as the record of arm A.

The model McBopomofo and vChewing use: Space or ↓ opens the window, and while it is up the number row picks (`references/McBopomofo/Source/KeyHandler.mm:589-625` opens it, `:1948-1962` reads the selection keys; `references/vChewing-macOS/Packages/vChewing_Typewriter/Sources/Typewriter/InputHandler/InputHandler_HandleStates.swift:1175-1193`). Rejected: Ctrl + digit over an always-on list, the `rime-moetaigi` answer (`moetaigi-tsuim.schema.yaml:275-283`), and a ↓ latch over an always-on list (a hidden state, and the macOS ↓ latch was retired 2026-08-28).

Under TPS only; TL and POJ are unchanged. All of it is the core's classifier and executor, so the three shells change only where a test pins the old list.

| State | Key | Does |
|---|---|---|
| Typing (no window) | A layout key, Backspace, the caret chord | Edits the glyphs; no fetch, and a window still up closes |
| Typing | A navigation key (arrows, Page Up / Down) or a key bound to a navigation or paging row (Tab, Shift+Tab, `[` `]` by default) | Opens the window on the first candidate (`OpenCandidates`) |
| Typing | Space after a closed syllable (the engine refuses the separator, D0) | Opens the window — revises O1's no-list arm |
| Typing | The keys bound to Confirm and to Commit as Typed (Enter, Shift+Enter) | Commits the glyphs as typed — the user has seen no candidate |
| Window up | Number row `1`–`9`, keypad `1`–`9` | Picks that slot |
| Window up | Space, Enter | Confirms the highlighted candidate; Shift+Enter still commits the glyphs as typed |
| Window up | Arrows, Tab, `[` `]` | Navigate, as today |
| Window up | Escape | Closes the window; the glyphs stay (`CloseCandidates`). A second Escape cancels, as today |
| Window up | Backspace, the caret chord | Closes the window, then edits as while typing |
| Window up | A layout key other than `1`–`9` | Closes the window and types the glyph — composition goes on |
| Either | Any other printable | As today: commits the glyphs as typed, then the key |

With Show Candidate Window off nothing opens: Space after a closed syllable commits the glyphs as typed (O1's old no-list arm), and a navigation key commits them and goes on to the host (`CommitThenPassThrough`). A fetch that finds nothing leaves the composition up and the window closed.

Slot set: `Keypad` is renamed `TpsDigits` and also matches the bare number row by key code (`NUMBER_ROW_KEY_CODES`), no modifier — Shift+`1` stays ㆠ. The keypad keeps its typed-digit check, so a keypad key with Num Lock off navigates rather than picks. Labels stay `1`–`9`; the Shortcuts pane's slot row under TPS reads `1–9, Num 1–9`. The symbol picker reads the same set, so under TPS its number row picks too. No new state: the list being empty is "typing", non-empty is "window up".

### D6 — On-screen key panel (U3, U6)

Shared in the core, new file `keys/tps_keyboard_rows.rs`: four rows of key caps built from the D2 table — the physical key's label, its glyph, its Shift glyph — so the panel cannot drift from the layout.

| Platform | Window | Shown | Click |
|---|---|---|---|
| macOS | `Panels/TpsKeyboardPanel.swift`: a non-activating `NSPanel` on the HUD chrome, owner-token guarded like the Telex guide; it takes clicks and never becomes key (`becomesKeyOnlyIfNeeded`, each cap's `needsPanelToBecomeKey` false) — vChewing's candidate window takes clicks the same way (`CtlCandidateTDK4AppKit.swift:23`, `VwrCandidateTDK4AppKit.swift:169`). Rows from the core through the pure seam request `TpsKeyboardRows` (`taigi-macos-ffi/src/key_rules.rs`) — no Swift copy of the glyphs | Bottom centre of the visible frame of the screen the mouse is on — best effort, since activation may not query the caret (`TaigiInputController.swift` `activateServer`) and `NSScreen.main` in the agent can be the primary display. Follows the session that holds the engine (`syncTpsKeyboard`, guarded by `backend.owns`) | `TpsKeyboardPress` (`taigi-macos-ffi` `session.rs`), an owned session request beside `Key` that runs the same `TpsKey` path — not `InsertSymbol`, which writes external text. The core checks the mode and the glyph (`ComposingKeyIntent::tps_keyboard_press`) and the engine's owner; the controller (`typeTpsKeyboardGlyph`) checks the tenure the panel was shown in, ownership and a live client (`macos-roadmap.md` D11), before and again after taking the picker down |
| Windows | `ui/tps_keyboard.rs`: a `WindowHandler` on `PopupWindow`, one per service activation, created hidden at activation (a focus callback may not create a window, W3), synced once when activation ends | Bottom centre of the work area of the foreground window's monitor (`MonitorFromWindow`); stays up across keys. Follows keyboard focus: `ITfKeyEventSink::OnSetFocus`, the thread-focus sink and a document focus of none record whether this activation is where the user types. Every move — focus, the chord, a mode switch — POSTS a sync (`WM_APP + 2`); the sync reads that flag and the settings as they stand when it runs, so a quick lose-and-regain or a focus change re-entering a show settles right. A key in the activation also marks it focused, for hosts that send no focus event. Moving between documents of one application keeps it up | `SendInput` of one key that names the glyph — the unassigned virtual key 0xE8, the glyph's number on the panel as its scan code (`taigi-windows-platform` `tps_keyboard_click.rs`) — which re-enters through the key sink (`TextService_Impl::tps_keyboard_click`), so W3 holds. Both panels type on the button's release over the cap the press began on (Windows since P7: the press takes the mouse, `SetCapture`). Not the cap's own virtual key (the P5 spike's): that would pick with the window up (U9), type `&` for `1` on AZERTY, and type a letter into whatever app took focus between the click and the delivery. The key carries its glyph, so nothing waits for its delivery; a refused click (read-only, English mode, no longer TPS) reaches the host as a key that types nothing |
| Linux | `taigikeyboard-settings/src/tps_keyboard.rs`: a window of the GTK settings app. The chord and the menu row launch the app with `--tps-keyboard`; its one instance opens the panel, or closes the one that is up, without building the settings window | An ordinary window, placed by the compositor | None — clicking it takes focus, which ends the session (`taigikeyboard-ibus/src/engine.rs:302-326`) |

Windows spike acceptance, on the Windows box, before P6 is scoped: a base key and a Shift-layer key each type their glyph; a click with a physical Shift / Ctrl / Alt / Win held; the synthetic Shift does not trip the Shift-tap English toggle (`taigi-windows-platform/src/key_translation.rs:68-93`); the cold first key (Test, then Deliver with `prepare_for_first_key`); focus moving between click and delivery; an English-mode and a read-only context; a click while the symbol picker or the Telex guide is up; a non-US keyboard layout (the injected key's character depends on the live layout); `SendInput`'s return count, and an elevated host (UIPI refuses injection upward).

The panel is shown only under TPS. On macOS and Windows the user's wish is one stored key, `tpsKeyboardShown` (in the General reset list): the chord flips it, leaving TPS hides the panel and keeps the key, so the panel comes back with TPS; focus moving away hides it for the moment only. Linux keeps no key: the IME process cannot show or hide the settings app's window, and a key saying "shown" with no settings process running would turn the next chord press into a no-op. There the window closes when the mode leaves TPS (its 1 s poll of the settings) and does not reopen with TPS — opening a window takes focus from the document, so it opens only when asked. Under TL / POJ the chord does nothing but stays the input method's (registered on every mode, like the Telex guide's under TPS), so a host does not see Ctrl+Alt+J / ⌃⌘J while Taigi is active. The Windows Shift-tap English mode leaves the panel up: it is a moment's detour, and the panel is wanted again on the way back.

**Flash (P7, maintainer 2026-10-07).** A key the engine takes under TPS lights its cap for 150 ms, as the system on-screen keyboards light a pressed key — the panel's job is learning the layout, so it answers "which key was that". One flashed cap at a time; a new key moves the light (a cap held under the mouse stays lit beside it). The signal is the executor's: when `ComposingManager::tps_key` answers `Taken`, `perform_intent` finds the cap (`keys::tps_keyboard_cap_of`; the separator has none) and calls `IntentSurface::tps_keyboard_cap_typed` (`composing/intent_executor.rs`), so a slot key picking, a refused Space, the separator or a key the window takes lights nothing — nor the TPS punctuation chord (Ctrl + `,` `.` `;` `[` `'`, S110): it is document text (`PassThrough` / `CommitThenInsert`), not a `TpsKey`, and its cap shows a glyph, not the mark. Both layers light the whole cap in the accent, its texts in the selected-text colour (Windows: the candidate window's highlight roles, which keep a high-contrast theme readable; macOS: the system accent, since the panel belongs to no app). A click runs the same `TpsKey`, so the clicked cap lights too; while the button is down the pressed cap is lit as well, as a button is (maintainer 2026-10-07) — macOS follows the pointer on and off the cap, Windows keeps it lit until the release; a release off the cap types nothing on either. macOS: an effect, `TpsKeyboardKeyTyped` (row, cap), replayed after the composition's writes and under the replay's tenure and owner checks; the panel lights only while it is up for that session, and a hide or a handover puts the light out. Windows: the session keeps the key and the panel lights it once the edit session and the engine lock are released (`run_key`), as the list is shown — a repaint and a timer, no window shown (W3); a document write that failed lights nothing; a hidden panel lights nothing, and a hide puts the light out; a timer tick that arrives early (queued before a newer key restarted it) is let pass, and the repeating timer's next tick ends the flash. Linux: nothing — its panel is a window of another process, with no channel from the input method.

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| Zhuyin-family layouts keep the Dachen positions and put the language's extra glyphs on Shift | `references/rime-moetaigi/rime-moetaigi/moetaigi-tsuim.schema.yaml:70,87` — keeps most Dachen positions but puts ㆦ on Dachen's ㄛ key, which D2 does not copy | D2 |
| Space collides with tone 1, so candidate picking moves off the main block | same file, `:254-268` (`Num_Lock: toggle_selection`) | D3, U5 |
| A closed composition is confirmed with Space | same file, `:266-268` (`when: composing, accept: space, send: Return`) | O1, recommended arm |
| Shared behaviour is written once in Rust; an edit relative to the caret belongs to the owner of the caret | `AGENTS.md` § Design principles; `engine/composing/src/transition.rs:127-140` (`TelexKey`) | D0 |
| Align on intended behaviour, not API calls | `docs/contributing/cross-platform-alignment.md` | D3 keeps the mobile separator rule; D6 differs per platform and says why |
| No redundant fallback | `AGENTS.md` § Design principles | The classifier has one TPS branch, not a romanization path that catches what TPS misses |
| A mouse path into the composition needs owner + generation validation | `macos-roadmap.md` D11 | D6 macOS |
| Never request an edit session from a window procedure | `windows-roadmap.md` W3 | D6 Windows uses the key sink |

Deliberately not adopted:

- **`TpsInputAdjust` + `ReplaceLast` + `Append` wrappers in the desktop core** — three calls, two preedits, and a raw caret the desktop does not have.
- **A second layout following the mobile 10 × 5 grid** — offered, not chosen (U4); one table, one panel.
- **Shift + digit as slot keys** — the Shift layer of the number row types glyphs.
- **A copy of the tone-mark set in the core** — the engine's separator test is the only one.
- **An IME-owned GTK window on Linux, or Fcitx5's virtual-keyboard interface** — `linux-roadmap.md` § Best practices alignment (not adopted); a click path there is outside this plan (U6).
- **An asynchronous edit session from the Windows panel** — breaks W3.
- **A physical-position (scan code) layout table** — only Windows carries full key codes today; the character table needs nine keypad codes and nothing else.
- **A new `is_keypad` snapshot field** — `key_code` already says it, and a new field touches every snapshot literal and the macOS proto.
- **The Linux panel as a framework lookup table**, as the Telex guide is drawn — the table is shared with the candidates, which the panel must stay up beside, so it would vanish while composing.
