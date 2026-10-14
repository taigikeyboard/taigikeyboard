# Custom Layouts — Roadmap

> **Type**: Planning (feature)
> **Keywords**: `custom layout`, `layout editor`, `romanization keyboard`, `Layout tab`
> **Status**: Active — phase 1 (iOS) merged; phase 2 (Android) in two PRs, 2a (data, storage, keyboard) merged (#506), 2b (editor + UI) merged (#507); device dogfood pending

---

## Summary

The mobile Layout tab's Custom Layouts shelf (a "Coming Soon" create card since #471) becomes real: a user rearranges the keys of a built-in romanization layout and saves it under a name. Two PRs, iOS first, Android follows on the same model.

Decisions (maintainer, 2026-10-14):

- **Edit model** — swap + spare tray. Tap two keys to swap them. Keys not on the board (`q w x z o͘ …`) wait in a spare tray under it; tap a key, then a spare, to replace — the replaced key goes to the tray. The editor never duplicates a key; a key left in the tray is the user's choice and cannot be typed from that layout.
- **Remove + symbols** (maintainer, 2026-10-14 update) — a trash button leads the tray: select a key, tap it, and the key goes to the tray. Its place shows as a dashed outline in the editor (tap it, then a spare, to fill it again); on the keyboard the place is gone and the row's other keys keep their width, centred. Character keys can be removed, and (second update below) the emoji key. The tray adds every key of the common symbol page (the 123 key, `TaigiLayouts.Numeric`) after the letters.
- **More letters + emoji** (maintainer, 2026-10-14 second update) — the tray's letters include every multi-letter key of the romanization layouts (MOE2 `ph th kh ts tsh ng nn oo`, POJ `ch chh`) whatever the base; 3-letter keys shrink to fit on any layout (`ButtonFontProvider`). The emoji key can be removed the same way (select, trash); a 😊 chip then waits in the tray and only goes back to the emoji key's place. Space takes the freed width.
- **Finals** (maintainer, 2026-10-14 fifth update) — the tray letters end with finals spelled the same in TL and POJ: `ai au ang ong ir er` as asked, plus `an ia in ian ann inn iau iong un iu ui` from the dictionary; script-divergent ones (`ing`/`eng`, `ua`/`oa`, `ue`/`oe`, `ik`/`ek`) stay out. Order = share of `dictionary/word-keys.tsv` syllables whose final contains the key, most frequent first. No built-in table has them, so each is a plain key (iOS `.char`, Android code 0) with no callout.
- **Function keys move within their row** (maintainer, 2026-10-14 third update) — tap two function keys of the same row to swap them (`⇧` ↔ `⌫`, `123` ↔ `↵` …). Never across rows and never with a character key: row widths stay the base's, and Android keeps function keys in separate modifier rows. Each key keeps its own width.
- **Drag** (maintainer, 2026-10-14 fourth update) — every edit can also be dragged: a key onto another key swaps them, a spare onto a key replaces it (or fills a removed place), a key onto the tray removes it. The board is fixed above a scrolling tray; board keys drag at once (custom `DragGesture` in the editor's coordinate space, hit-tested against reported key frames), tray chips lift on press-and-hold (system `draggable` / `dropDestination`). The dragged key floats above the finger, the target key lights up, a light haptic marks each drop. Taps stay for VoiceOver and precision.
- **Starting point** — chips at the top of the editor pick the base: PhahTaigi / Lohankha / MOE Layout 1 / MOE Layout 2 (existing names). Switching the base restarts the draft from it.
- **Where it shows** — the Layout tab shelf and the in-keyboard layout picker.
- **Platforms** — iOS (phase 1), then Android (phase 2).

## Model (grounded in code)

```
CustomLayout { id: UUID, name, base: KeyboardLayoutType, script: tl|poj,
               keys: [String?], functionKeys: [FunctionKey], hasEmojiKey: Bool }
// keys: nil = removed; functionKeys missing in a record = base order; hasEmojiKey missing = true
```

- **Base-relative.** `keys` replaces, in reading order, every `.char` key outside the digit row of the base table `base × script` (iPhone variant), taking the whole `KeyDef` of the moved key so full-width forms travel with it (`LayoutConverter.swift:40` reads `fullWidth` per key): the base's own key first (MOE1 `(` stays half-width), else the catalog — the deduplicated `.char` keys of the symbol page and the romanization tables, no second punctuation table. `functionKeys` fills the function keys the same way, each row keeping the base row's set (checked on load). The digit row and each key's width stay the base's. The globe key is not stored: the keyboard puts it right after 123. The email / URL key replaces a bottom-row `,`, else goes right before space — on every built-in romanization / English row that is right after emoji, as before (`applyInputTypeKey`; TPS returns before it). A removed key leaves its row; a row left empty leaves the keyboard.
- **Seam.** `CustomLayoutService.selectAlphabeticLayout` picks `base × script` for an active custom layout (not the live mode — Lohankha TL and POJ differ in key count, `TaigiLayouts.swift:35-55`), substitutes, then runs `applyInputTypeKey` as for built-ins. Globe insertion (`TaigiLayouts.swift:8-12`) and the 文/A drop (`LayoutConverter.swift:17`) touch no `.char` key. `applyInputTypeKey` swaps the bottom-row `,` for `@` / `/` or inserts before space — on a custom layout it follows wherever `,` and space ended up, same rule.
- **Same key positions in TL and POJ.** Only the board is frozen; tone callouts and `nn` display still follow the live mode (`TaigiCallouts+Builder.swift:59`, `ButtonTextProvider.swift:166`). English and TPS keep their own tables (`CustomLayoutService.swift:80-90`).
- **Label-keyed behaviour follows the key.** MOE callouts (`TaigiCallouts+Builder.swift:35-49`), MOE hints (`ButtonTextProvider.swift:51,90`), MOE2 font (`ButtonFontProvider.swift:45`), 文/A width (`LayoutConverter.swift:102-104`) read the base id + label; shift, `-`, `oo`/`nn` handling read the typed string (`ActionHandler+KeyActions.swift:26,70,134`, `engine/phonetics/src/api.rs:95`).
- **Places** — the editor selects a `Place`: a slot of `keys` or of `functionKeys`. The emoji key is whichever function slot holds it; `hasEmojiKey` false hides it.
- **Spare tray** = letters (`a`–`z`, `o͘`, `-`, every multi-letter key of the romanization tables, the finals) then symbols (the symbol page) − keys on the board. Symbols keep their long-press callouts (`SymbolCallouts`, keyed by label).
- **Validation on load** — non-TPS base, `keys.count` = the base table's slot count, every placed key in the catalog, no duplicates. An invalid record is dropped, never rendered.

### Android (phase 2, grounded in code)

Same record and JSON; per-platform tables. Android keeps its layout JSON: `LayoutAssets` (parse + layer merge + file names, shared by the keyboard and the tables) and `CustomLayoutTables` (`ime/text/layout/`).

- **Base table** = the half-width merge of `number_row` + `characters/<base × script>` + `characters/mod/<base>` that `LayoutManager` shows, digit row inert. Character places and function places are numbered in reading order as on iOS; the slot counts match (Lohankha TL 28 / POJ 29, PhahTaigi 27, MOE1 27, MOE2 29). The three 文/A variation copies (email / normal / URL fields) are one function place and move together; globe, `@` (email) and `/` (URL) are not places.
- **Catalog** = half label → (half-width, full-width `KeyData`), zipped from each half / full-width file pair: the symbol page (`symbols/western_*`) first, then every romanization table. Handed out as deep copies (`popup` is mutable).
- **Seam** — `LayoutManager.mergeLayouts` substitutes after the merge and before extended popups (keyed by label, so tone popups follow the key): each place takes the base's own `KeyData` for that label in the current width (MOE1 `(` stays half-width), else the catalog's. Inline popups move with the key. Removed keys and the emoji key (`hasEmojiKey` false) leave their row; an empty row leaves the keyboard. `@` / `/` go right after a bottom-row `,` (which then hides in email / URL fields), else right before space; the existing globe pass puts the globe after 123. The main table follows the record's script, not the live mode.
- **Storage** — DataStore `keyboard__user_layouts` (JSON list, per-record decode, invalid records dropped) and `keyboard__selected_custom_layout_id`; write plans in `CustomLayoutSelection` (pure, beside `TpsCascade`). Settings reset keeps the list. `allowBackup="false"` already keeps DataStore out of OS backup.
- **Refresh** — `TaigiKeyboard` collects `PrefHelper.observeLetterLayout()` (stored layout + active record, from one emission, decoded off the main thread) and reloads the letter layout once per change. The Theme-tab preview keys its layout on the active record.
- **Editor (2b)** — editing ops on `CustomLayout` (pure, return a copy; function-row checks take the `BaseTable`); `CustomLayoutTables` adds `fresh` / `isUnchanged`, the tray pools and the drawn rows (`editorRows` keeps every place, a removed one flagged; `previewRows` is the keyboard's view), numbered from the same pieces `substitute` reads. `LayoutKeyGrid` (Compose, `ime/text/layout/`) draws the board, the cards and the picker cards; widths come from the keyboard's own slot fractions (`KeyboardLayoutSolver.slotFraction`). `LayoutEditorActivity` hosts the editor full screen, as `ThemeEditorActivity`. One drag state covers all three drags (board key after touch slop, tray chip after a long press so the tray still scrolls), hit-tested in window coordinates; only a release commits, a cancelled gesture drops the drag. The card menu and the name dialog are the theme ones, moved to `ui/components/` (`GalleryCardMenu`, `NameDialog`).

## Selection contract

Selection = `keyboardLayoutType = base` + `selectedCustomLayoutId` (App Group defaults). One `SharedSettings` operation applies a custom layout; built-in selection clears it.

| Path | Custom id |
|---|---|
| Built-in card / overlay / TPS card (`setKeyboardLayoutType`) | cleared, even for the same base |
| Apply custom | `setKeyboardLayoutType(base)` (clears), then id set |
| Input-mode TPS enter / leave (`setInputMode`, writes defaults directly, `SharedSettings.swift:238-253`) | kept; `layoutBeforeTps` = base, so leaving resumes it |
| Settings reset | cleared; saved layouts stay |
| Delete the active layout (also while TPS / English) | cleared; base stays |
| Save in the editor | save-and-apply (theme editor pattern, `ThemeEditorViewModel.swift:58-75`), so an edited base never mismatches |

The resolver honours the id only when the stored layout matches `custom.base` (a guard, not a fallback). Built-in checkmarks require no custom id (`LayoutTab.swift:107`, `LayoutSelectionOverlay.swift:83`).

## Keyboard refresh

The layout is built once per keyboard view (`KeyboardViewController.swift:167`) and `syncSettings` only resets autocomplete on a layout-type change (`KeyboardViewController+Setup.swift:178-184`). `syncSettings` compares the active custom layout (`activeCustomLayout`, the whole record): a switch, an edit or a TPS suspend calls `KeyboardContext.notifyDisplayChange()`, the path the Candidate Display change already uses to re-run the layout build (`KeyboardViewController+Setup.swift`), so an open overlay is not torn down and composition is not reset. Built-in switching is unchanged. A host-app edit reaches the extension on its next `viewDidAppear` → `syncSettings` (`KeyboardViewController.swift:142-153`).

## Storage

`UserLayoutStore`, mirroring `UserThemeStore`: `user_layouts.json` in the App Group, written atomically, excluded from OS backup (`behavioral-invariants.md` §29), cap 5, a revision counter in defaults, a revision-keyed cache in `SharedSettings` (as `SharedSettings.swift:617-630`). Add / update / delete return success; the UI applies or closes only on success. Not in `.taigi` backup (themes are not either). No generic store: theme seed / carry-over logic (`UserThemeStore.swift:38-73`) has no layout counterpart.

## UI

- **Editor** (`App/Tabs/Layout/`) — base chips · a drawn key grid (selected key filled; removed places dashed; the digit row inert) · spare tray (trash, the removed emoji key, letters, symbols). A new layout starts from the active custom layout's base / script, else the stored built-in (PhahTaigi while the layout is TPS) in the current romanization script (TL unless POJ). Base switch / reset clear the selection; Back discards the draft. Save asks for a name (alert, as the theme editor). Keys are buttons with selected traits for VoiceOver; function keys are named by their Unicode symbols (⇧ ⌫ ⏎ ␣ 😊), a removed place by `settings.syllableSeparatorNone`. Reset reuses `theme.editorResetAll`.
- **Grid view** (`Layout/`, compiled into app and extension) — takes rows, selection and a tap callback; drawn small as the card preview on the Layout tab and in the in-keyboard picker. No screenshot asset, no KeyboardKit view.
- **Cards** — card menu Apply / Edit / Delete (existing keys). At the cap the create card hides, as on the Theme tab.
- **i18n** — new (iOS scope now, Android joins in phase 2): `layout.editorTitleNew` ("New Layout"; also the blank-name fallback and the name placeholder), `layout.nameHeader` ("Layout Name"). Edit mode titles the editor with the layout's name. `layout.comingSoon` loses its iOS scope in phase 1 and is deleted in phase 2 (Android still reads it, `LayoutScreen.kt:137`).

## Phases

| Phase | Scope | Status |
|---|---|---|
| 1 | iOS — model + store + keyboard + editor (swap / spare tray / remove / symbols / multi-letter spares / emoji / function keys within row / drag) + Layout tab shelf + in-keyboard picker | Merged #505 `c0fb934c` |
| 2a | Android — record + tables + DataStore storage + selection + keyboard substitution + refresh; built-in cards / picker uncheck while a custom layout is active; 3-char keys shrink on any layout | Merged (#506) |
| 2b | Android — editing ops + Compose editor (swap / tray / remove / emoji / function keys / drag) + Layout tab shelf + in-keyboard picker; i18n `layout.editorTitleNew` / `layout.nameHeader` gain Android scope; delete `layout.comingSoon` | Merged (#507) |
| 3 | iOS + Android — tray finals (17 keys, frequency order) | In progress |

Phase 1 checks: same-base custom ↔ built-in switch, custom A → B, active edit with a base change, TPS via mode and via card, delete while suspended, reset, globe, email / URL fields, full-width punctuation, shift, callouts, Theme-tab keyboard preview (`KeyboardPreviewPanel.swift:28` also calls `CustomLayoutService`).

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| A custom layout starts from a preset base | `references/azooKey/MainApp/Customize/EditingGridFitCustardView.swift:379-420` (base Custard selection) | Base chips (phases 1–2) |
| User-made items persisted beside settings, capped, shown on a gallery shelf | `ios/.../Theme/UserThemeStore.swift:17-110`, `ThemePickerView.swift:33-51` | Same store shape and shelf (phase 1) |

Rules: `code-review-rules.md` §2 (reuse theme shelf / card / store pattern), §4 (no generic layout DSL); `AGENTS.md` Core Principle 2 (same model, per-platform tables).

**Deliberately not adopted**

- Moving function keys across rows or onto character places — rows would overflow (`⇧` 1.5, `↵` 1.75 keys wide) and Android builds them from separate modifier files.
- Free per-key text entry, adding keys or rows, key widths, removing function keys other than emoji — needs validation and warning copy (a board with no ⌫ or ↵); swap + tray + remove keeps the editor free of errors.
- Tone-marked vowels (`á à â ā a̍ …`) in the tray — tones are typed as digits or chosen by long-press; a precomposed vowel as composing input is unverified in the engine.
- The rarer symbol page (`#+=`, `TaigiLayouts.Symbolic`) in the tray — keeps the tray short.
- An engine-owned layout model — the engine has no notion of layouts (`engine/composing/src/requests.rs:360` is a comment); built-in tables stay per platform (Swift tables / Android JSON).
- A generic user-item store shared with themes (see Storage).
- Custom TPS layouts — TPS is bound 1:1 to its input mode.
- Layout export / sharing (CustardKit format) and `.taigi` backup.
