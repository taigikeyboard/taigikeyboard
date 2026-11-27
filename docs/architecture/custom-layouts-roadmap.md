# Custom Layouts — Roadmap

> **Type**: Design record (phases 1–4 done; TPS phases 5–6 open)
> **Keywords**: `custom layout`, `layout editor`, `romanization keyboard`, `Layout tab`
> **Status**: Phases 1–4 and follow-up fixes #511 / #513 / #514 merged; device dogfood passed 2026-10-18. TPS layouts (phases 5–6, maintainer 2026-11-27) in progress

---

## Summary

The mobile Layout tab's Custom Layouts shelf (a "Coming Soon" create card since #471) becomes real: a user rearranges the keys of a built-in romanization layout and saves it under a name. Two PRs, iOS first, Android follows on the same model.

Decisions (maintainer, 2026-10-14):

- **Edit model** — swap + spare tray. Tap two keys to mark them, press the swap button to swap them; long-press slots work the same way. Keys not on the board (`q w x z o͘ …`) wait in a spare tray under it; tap a key, a spare and the swap button to replace — the replaced key goes to the tray. The editor never duplicates a key; a key left in the tray is the user's choice and cannot be typed from that layout.
- **Remove + symbols** (maintainer, 2026-10-14 update) — a trash button leads the tray: select a key, tap it, and the key goes to the tray. Its place shows as a dashed outline in the editor (tap it, then a spare, to fill it again); on the keyboard the place is gone and the row's other keys keep their width, centred. Character keys can be removed, and (second update below) the emoji key. The tray adds every key of the common symbol page (the 123 key, `TaigiLayouts.Numeric`) after the letters.
- **More letters + emoji** (maintainer, 2026-10-14 second update) — the tray's letters include every multi-letter key of the romanization layouts (MOE2 `ph th kh ts tsh ng nn oo`, POJ `ch chh`) whatever the base; 3-letter keys shrink to fit on any layout (`ButtonFontProvider`). The emoji key can be removed the same way (select, trash); a 😊 chip then waits in the tray and only goes back to the emoji key's place. Space takes the freed width.
- **Finals** (maintainer, 2026-10-14 fifth update) — the tray letters end with finals spelled the same in TL and POJ: `ai au ang ong ir er` as asked, plus `an ia in ian ann inn iau un iu ui` from the dictionary (at most 3 letters, maintainer), and `or` (maintainer, 2026-11-08, GH #802); script-divergent ones (`ing`/`eng`, `ua`/`oa`, `ue`/`oe`, `ik`/`ek`) stay out. Order = share of `dictionary/word-keys.tsv` syllables whose final contains the key, most frequent first. No built-in table has them, so each is a plain key (iOS `.char`, Android code 0) with no callout.
- **`ee`** (maintainer, 2026-10-15) — the tray offers `ee` [ɛ] (same spelling in TL and POJ) right after `o͘`, a plain key like the finals. `o̤` / `ṳ` were considered and dropped (maintainer); the engine does not fold them to `er` / `ir`.
- **Function keys move within their row** (maintainer, 2026-10-14 third update) — two function keys of the same row can be marked and swapped (`⇧` ↔ `⌫`, `123` ↔ `↵` …). Never across rows and never with a character key: row widths stay the base's, and Android keeps function keys in separate modifier rows. Each key keeps its own width.
- **Tap only** (maintainer, 2026-10-31; supersedes the 2026-10-14 drag decision) — the editor takes taps alone: two keys and the swap button swap, a key, a spare and the swap button replace it (or fill a removed place), a key then the trash removes it, the emoji chip puts the emoji key back. Drag (board keys, tray chips, the emoji chip) was removed because taps already cover every edit. The board is fixed above a scrolling tray.
- **Swap is explicit** (maintainer, 2026-11-04) — tapping a second key never edits: it marks both keys, and the swap button below the board, coloured only while a pair is marked, swaps them. A third tap starts over; any edit or long-press slot tap clears the pair. The long-press strip follows the last key tapped. Long-press slots follow the same rule, and a tray character marks with the selected slot or key the same way (2026-11-05): nothing but the swap button places it, except the emoji chip, which has one place only. With a long-press slot selected, a character the key's list holds stays dimmed and inert. Undo sits beside swap and steps back one edit at a time, so the bubble spans the full width; it replaced both Reset All and the per-key long-press reset. Base chips start a board over. Delete joins swap and undo in one row of three equal pills. Only what the next action works on is lit: with a long-press slot selected, the key goes plain and the bubble's tail names it.
- **Both symbol pages go on the board** (maintainer, 2026-11-05, GH #672) — every key of the second symbol page can replace a key, with its full-width form: `{` types `｛` under full-width punctuation. With a key selected and no long-press slot, a tray character that cannot go on it (full-width forms, long-press-only symbols such as `﹛`, tone letters, keys already on the board) is dimmed and inert, instead of ignoring the tap.
- **Starting point** — chips at the top of the editor pick the base: PhahTaigi / Lohankha / MOE Layout 1 / MOE Layout 2 (existing names). Switching the base restarts the draft from it.
- **Where it shows** — the Layout tab shelf and the in-keyboard layout picker.
- **Platforms** — iOS (phase 1), then Android (phase 2).
- **TPS layouts** (maintainer, 2026-11-27; supersedes "Custom TPS layouts" under Deliberately not adopted) — the Phonetic Symbols table is a base too: its own selection slot, a fourth use (Phonetic Symbols) in the create dialog, a tray of TPS glyphs (the 16 long-press variants included) and symbols with no Latin letters or digits, and row-1 digit long-presses that follow their key. See § TPS layouts.

## Model (grounded in code)

```
CustomLayout { id: UUID, name, base: KeyboardLayoutType, script: tl|poj,
               keys: [String?], functionKeys: [FunctionKey], hasEmojiKey: Bool }
// keys: nil = removed; functionKeys missing in a record = base order; hasEmojiKey missing = true
```

- **Base-relative.** `keys` replaces, in reading order, every `.char` key outside the digit row of the base table `base × script` (iPhone variant), taking the whole `KeyDef` of the moved key so full-width forms travel with it (`LayoutConverter.swift` reads `fullWidth` per key): the base's own key first (MOE1 `(` stays half-width), else the catalog — the deduplicated `.char` keys of the symbol page and the romanization tables, no second punctuation table. `functionKeys` fills the function keys the same way, each row keeping the base row's set (checked on load). The digit row and each key's width stay the base's. The globe key is not stored: the keyboard puts it right after 123. The email / URL key replaces a bottom-row `,`, else goes right before space — on every built-in romanization / English row that is right after emoji, as before (`applyInputTypeKey`; TPS returns before it). A removed key leaves its row; a row left empty leaves the keyboard.
- **Seam.** `CustomLayoutService.selectAlphabeticLayout` picks `base × script` for an active custom layout (not the live mode — Lohankha TL and POJ differ in key count, `TaigiLayouts.swift`), substitutes, then runs `applyInputTypeKey` as for built-ins. Globe insertion (`TaigiLayouts.swift`) and the 文/A drop (`LayoutConverter.swift`) touch no `.char` key. `applyInputTypeKey` swaps the bottom-row `,` for `@` / `/` or inserts before space — on a custom layout it follows wherever `,` and space ended up, same rule.
- **Same key positions in TL and POJ.** Only the board is frozen; tone callouts and `nn` display still follow the live mode (`TaigiCallouts+Builder.swift`, `ButtonTextProvider.swift`). English keeps its own table; TPS shows a TPS custom layout or its own table (`CustomLayoutService.swift`).
- **Label-keyed behaviour follows the key.** MOE callouts (`TaigiCallouts+Builder.swift`), MOE hints (`ButtonTextProvider.swift`), MOE2 font (`ButtonFontProvider.swift`), 文/A width (`LayoutConverter.swift`) read the base id + label; shift, `-`, `oo`/`nn` handling read the typed string (`ActionHandler+KeyActions.swift`, `engine/phonetics/src/tone_render.rs` `preprocess_for_normalize_tone`).
- **Places** — the editor selects a `Place`: a slot of `keys` or of `functionKeys`. The emoji key is whichever function slot holds it; `hasEmojiKey` false hides it.
- **Spare tray** = letters (`a`–`z`, `o͘`, `ee`, `-`, every multi-letter key of the romanization tables, the finals) − keys on the board, then symbols (both symbol pages, then the built-in long-press entries). Symbols keep their long-press callouts (`SymbolCallouts`, keyed by label).
- **Validation on load** — `keys.count` = the base table's slot count, every placed key placeable on that base (a TPS base: TPS glyphs and symbol-page keys; a romanization base: catalog keys), no duplicates. An invalid record is dropped, never rendered.

### TPS layouts (phase 5, grounded in code)

- **Record** — the same record with `base` `"tps"`, `script` `"tl"` (unused: one table serves TPS), `isUniversal` always false (also when the field is missing). The TPS table has no digit row, so all five rows are places (iOS: 44 character places, function places backspace / 123 / space / emoji / return); a row counts as the digit row only when it is non-empty and holds only the keys `0`–`9`, so a TPS first row with every key removed stays editable.
- **Selection** — `tpsCustomLayoutId` beside `selectedCustomLayoutId`; the active layout reads the TPS slot while the layout is TPS. Applying a TPS layout saves the mode to leave to (`inputModeBeforeTps`) and enters TPS as the input mode does, so the romanization selection stays (suspended) and resumes on leaving.
- **Keyboard** — `CustomLayoutService` returns the TPS layout's rows in the TPS branch (no input-type key, no globe, as the built-in table). Long-press lists and keycap hints are keyed by label and follow the key; a key with its own list shows no built-in hint, as on romanization layouts. TPS always types full-width punctuation, so a symbol key or a list entry types its full-width form (`'` → `、`) while the editor board draws the half-width label.

### Android (phase 2, grounded in code)

Same record and JSON; per-platform tables. Android keeps its layout JSON: `LayoutAssets` (parse + layer merge + file names, shared by the keyboard and the tables) and `CustomLayoutTables` (`ime/text/layout/`).

- **Base table** = the half-width merge of `number_row` + `characters/<base × script>` + `characters/mod/<base>` that `LayoutManager` shows, digit row inert. Character places and function places are numbered in reading order as on iOS; the slot counts match (Lohankha TL 28 / POJ 29, PhahTaigi 27, MOE1 27, MOE2 29). The three 文/A variation copies (email / normal / URL fields) are one function place and move together; globe, `@` (email) and `/` (URL) are not places.
- **Catalog** = half label → (half-width, full-width `KeyData`), zipped from each half / full-width file pair: the symbol page (`symbols/western_*`) first, then every romanization table, then the second symbol page (`symbols2/western_*`). Handed out as deep copies (`popup` is mutable).
- **Seam** — `LayoutManager.mergeLayouts` substitutes after the merge and before extended popups (keyed by label, so tone popups follow the key): each place takes the base's own `KeyData` for that label in the current width (MOE1 `(` stays half-width), else the catalog's. Inline popups move with the key. Removed keys and the emoji key (`hasEmojiKey` false) leave their row; an empty row leaves the keyboard. `@` / `/` go right after a bottom-row `,` (which then hides in email / URL fields), else right before space; the existing globe pass puts the globe after 123. The main table follows the record's script, not the live mode.
- **Storage** — DataStore `keyboard__user_layouts` (JSON list, per-record decode, invalid records dropped) and `keyboard__selected_custom_layout_id`; write plans in `CustomLayoutSelection` (pure, beside `TpsCascade`). Settings reset keeps the list. `allowBackup="false"` already keeps DataStore out of OS backup.
- **Refresh** — `TaigiKeyboard` collects `PrefHelper.observeLetterLayout()` (stored layout + active record, from one emission, decoded off the main thread) and reloads the letter layout once per change. The Theme-tab preview keys its layout on the active record.
- **Editor (2b)** — editing ops on `CustomLayout` (pure, return a copy; function-row checks take the `BaseTable`); `CustomLayoutTables` adds `fresh`, the tray pools and the drawn rows (`editorRows` keeps every place, a removed one flagged; `previewRows` is the keyboard's view), numbered from the same pieces `substitute` reads. `LayoutKeyGrid` (Compose, `ime/text/layout/`) draws the board, the cards and the picker cards; widths come from the keyboard's own slot fractions (`KeyboardLayoutSolver.slotFraction`). `LayoutEditorActivity` hosts the editor full screen, as `ThemeEditorActivity`. The card menu and the name dialog are the theme ones, moved to `ui/components/` (`GalleryCardMenu` in `GalleryCard.kt`, `NameDialog`).

## Selection contract

Selection = `keyboardLayoutType = base` + `selectedCustomLayoutId` (App Group defaults). One `SharedSettings` operation applies a custom layout; built-in selection clears it.

Per-script memory (#650): Tâi-lô and Pe̍h-ōe-jī each keep a selection (`tlLayoutSelection` / `pojLayoutSelection`, `"<base>"` or `"<base>|<id>"`; Android `keyboard__tl_layout_selection` / `keyboard__poj_layout_selection`) and `layoutScriptOwner` names the script the live pair belongs to. Entering the other script through the input mode saves the live pair to the owner's slot and loads the new script's (`LayoutScriptMemory`); a custom id survives only while the layout exists over that base and fits the script (its own script, or it is universal). English and TPS keep no selection. Applying a custom layout enters its script first; a Tâi-lô / Pe̍h-ōe-jī card writes the mode, then the layout. Where a custom layout shows is its use (`LayoutUse`: Universal, Tâi-lô, Pe̍h-ōe-jī), picked in the save dialog and stored as `isUniversal` plus `script` (records without `isUniversal` read as their base's default: Phah Taigi universal). A universal layout fits both scripts and applying it keeps the mode. Picking one script lays the keys as arranged on that script's table row by row (`moveKeys` iOS, `CustomLayoutTables.withUse` Android): QWERTY's Pe̍h-ōe-jī row 2 has one more place (o͘), filled as a removed key; going back drops that place's key and its long-press list.

| Path | Custom id |
|---|---|
| Built-in card / overlay / TPS card (`setKeyboardLayoutType`) | cleared, even for the same base; inside TPS the TPS card clears only the TPS slot |
| Apply a TPS custom layout | `inputModeBeforeTps` = the live mode, `setInputMode(.tps)`, then the TPS slot set; the romanization id is kept |
| Apply custom | `setKeyboardLayoutType(base)` (clears), then id set |
| Input-mode TPS enter / leave (`setInputMode`, writes defaults directly, `SharedSettings.swift`) | kept; `layoutBeforeTps` = base, so leaving resumes it |
| Settings reset | cleared (both slots); saved layouts stay |
| Delete the active layout (also while TPS / English) | cleared (its slot); base stays |
| Save in the editor | save-and-apply (theme editor pattern, `ThemeEditorViewModel.swift`), so an edited base never mismatches |

The resolver honours the id only when the stored layout matches `custom.base` (a guard, not a fallback). Built-in checkmarks require no custom id (`LayoutTab.swift`, `LayoutSelectionOverlay.swift`).

## Keyboard refresh

The layout is built once per keyboard view (`KeyboardViewController.swift`) and `syncSettings` only resets autocomplete on a layout-type change (`KeyboardViewController+Setup.swift`). `syncSettings` compares the active custom layout (`activeCustomLayout`, the whole record): a switch, an edit or a TPS suspend calls `KeyboardContext.notifyDisplayChange()`, the path the Candidate Display change already uses to re-run the layout build (`KeyboardViewController+Setup.swift`), so an open overlay is not torn down and composition is not reset. Built-in switching is unchanged. A host-app edit reaches the extension on its next `viewDidAppear` → `syncSettings` (`KeyboardViewController.swift`).

## Storage

`UserLayoutStore`, mirroring `UserThemeStore`: `user_layouts.json` in the App Group, written atomically, excluded from OS backup (`behavioral-invariants.md` §29), cap 5, a revision counter in defaults, a revision-keyed cache in `SharedSettings` (as `SharedSettings.swift`). Add / update / delete return success; the UI applies or closes only on success. Not in `.taigi` backup (themes are not either). No generic store: theme seed / carry-over logic (`UserThemeStore.swift`) has no layout counterpart.

## UI

- **Editor** (`App/Tabs/Layout/`) — base chips · a drawn key grid (selected key filled; removed places dashed; the digit row inert) · spare tray (the removed emoji key, letters, symbols) · action row (swap, undo, delete). A new layout is named and given its use (Universal / Tâi-lô / Pe̍h-ōe-jī / Phonetic Symbols) in a dialog on the Layout tab before the editor opens; it starts from the active custom layout's base, else the stored built-in (the TPS table while the layout is TPS; a romanization use then starts from PhahTaigi), in the script its use names (Universal: the active layout's script, else the current romanization script, TL unless POJ). Base switch and undo clear the selection and keep the use; Back discards the draft. Save stores a new layout directly; an edit's Save asks for the name and use again (a TPS layout: the name only). A universal layout's built-in tone lists hold both scripts' marks (both tone-9 marks). Keys are buttons with selected traits for VoiceOver; function keys are named by their Unicode symbols (⇧ ⌫ ⏎ ␣ 😊), a removed place by `settings.syllableSeparatorNone`.
- **Grid view** (`Layout/`, compiled into app and extension) — takes rows, selection and a tap callback; drawn small as the card preview on the Layout tab and in the in-keyboard picker. No screenshot asset, no KeyboardKit view.
- **Cards** — card menu Apply / Edit / Delete (existing keys). At the cap the create card hides, as on the Theme tab.
- **i18n** — new (iOS scope now, Android joins in phase 2): `layout.editorTitleNew` ("New Layout"; also the blank-name fallback and the name placeholder), `layout.nameHeader` ("Layout Name"). Edit mode titles the editor with the layout's name. `layout.comingSoon` was deleted in phase 2.

## Phases

| Phase | Scope | Status |
|---|---|---|
| 1 | iOS — model + store + keyboard + editor (swap / spare tray / remove / symbols / multi-letter spares / emoji / function keys within row / drag) + Layout tab shelf + in-keyboard picker | Merged #505 `c0fb934c` |
| 2a | Android — record + tables + DataStore storage + selection + keyboard substitution + refresh; built-in cards / picker uncheck while a custom layout is active; 3-char keys shrink on any layout | Merged (#506) |
| 2b | Android — editing ops + Compose editor (swap / tray / remove / emoji / function keys / drag) + Layout tab shelf + in-keyboard picker; i18n `layout.editorTitleNew` / `layout.nameHeader` gain Android scope; delete `layout.comingSoon` | Merged (#507) |
| 3 | iOS + Android — tray finals (16 keys, frequency order) | Merged (#508) |
| 4 | iOS + Android — tray `ee` after `o͘` | Merged (#510) |
| 5 | iOS — TPS layouts (record, places without a digit row, TPS slot, Phonetic Symbols use, TPS tray) | Merged #1174 `430e9f685` |
| 6 | Android — TPS layouts on the same record | Pending |

The Theme-tab keyboard preview (`KeyboardPreviewPanel.swift`) also calls `CustomLayoutService`.

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| A custom layout starts from a preset base | `references/azooKey/MainApp/Customize/EditingGridFitCustardView.swift` (base Custard selection) | Base chips (phases 1–2) |
| User-made items persisted beside settings, capped, shown on a gallery shelf | `ios/.../Theme/UserThemeStore.swift`, `ThemePickerView.swift` | Same store shape and shelf (phase 1) |

Rules: `code-review-rules.md` §2 (reuse theme shelf / card / store pattern), §4 (no generic layout DSL); `AGENTS.md` Core Principle 2 (same model, per-platform tables).

**Deliberately not adopted**

- Moving function keys across rows or onto character places — rows would overflow (`⇧` 1.5, `↵` 1.75 keys wide) and Android builds them from separate modifier files.
- Free per-key text entry, adding keys or rows, key widths, removing function keys other than emoji — needs validation and warning copy (a board with no ⌫ or ↵); swap + tray + remove keeps the editor free of errors.
- Tone-marked vowels (`á à â ā a̍ …`) in the tray — tones are typed as digits or chosen by long-press; a precomposed vowel as composing input is unverified in the engine.
- An engine-owned layout model — the engine knows only whether the TPS layout is on (`AppConfig::is_tps_layout`); built-in tables stay per platform (Swift tables / Android JSON).
- A generic user-item store shared with themes (see Storage).
- Layout export / sharing (CustardKit format) and `.taigi` backup.
