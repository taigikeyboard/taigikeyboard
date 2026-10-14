# Custom Layouts — Roadmap

> **Type**: Planning (feature)
> **Keywords**: `custom layout`, `layout editor`, `romanization keyboard`, `Layout tab`
> **Status**: Active — phase 1 (iOS) in progress

---

## Summary

The mobile Layout tab's Custom Layouts shelf (a "Coming Soon" create card since #471) becomes real: a user rearranges the keys of a built-in romanization layout and saves it under a name. Two PRs, iOS first, Android follows on the same model.

Decisions (maintainer, 2026-10-14):

- **Edit model** — swap + spare tray. Tap two keys to swap them. Keys not on the board (`q w x z o͘ …`) wait in a spare tray under it; tap a key, then a spare, to replace — the replaced key goes to the tray. The editor never duplicates a key; a key left in the tray is the user's choice and cannot be typed from that layout.
- **Remove + symbols** (maintainer, 2026-10-14 update) — a trash button leads the tray: select a key, tap it, and the key goes to the tray. Its place shows as a dashed outline in the editor (tap it, then a spare, to fill it again); on the keyboard the place is gone and the row's other keys keep their width, centred. Only character keys can be removed. The tray adds every key of the common symbol page (the 123 key, `TaigiLayouts.Numeric`) after the letters.
- **Starting point** — chips at the top of the editor pick the base: PhahTaigi / Lohankha / MOE Layout 1 / MOE Layout 2 (existing names). Switching the base restarts the draft from it.
- **Where it shows** — the Layout tab shelf and the in-keyboard layout picker.
- **Platforms** — iOS (phase 1), then Android (phase 2).

## Model (grounded in code)

```
CustomLayout { id: UUID, name, base: KeyboardLayoutType, script: tl|poj, keys: [String?] }   // nil = removed
```

- **Base-relative.** `keys` replaces, in reading order, every `.char` key outside the digit row of the base table `base × script` (iPhone variant), taking the whole `KeyDef` of the moved key so full-width forms travel with it (`LayoutConverter.swift:40` reads `fullWidth` per key): the base's own key first (MOE1 `(` stays half-width), else the catalog — the deduplicated `.char` keys of the symbol page and the romanization tables, no second punctuation table. Function keys, the digit row and key widths stay the base's. A removed key leaves its row; a row left empty leaves the keyboard.
- **Seam.** `CustomLayoutService.selectAlphabeticLayout` picks `base × script` for an active custom layout (not the live mode — Lohankha TL and POJ differ in key count, `TaigiLayouts.swift:35-55`), substitutes, then runs `applyInputTypeKey` as for built-ins. Globe insertion (`TaigiLayouts.swift:8-12`) and the 文/A drop (`LayoutConverter.swift:17`) touch no `.char` key. `applyInputTypeKey` swaps the bottom-row `,` for `@` / `/` or inserts after emoji (`CustomLayoutService.swift:124-149`) — on a custom layout it follows wherever `,` ended up, same rule.
- **Same key positions in TL and POJ.** Only the board is frozen; tone callouts and `nn` display still follow the live mode (`TaigiCallouts+Builder.swift:59`, `ButtonTextProvider.swift:166`). English and TPS keep their own tables (`CustomLayoutService.swift:80-90`).
- **Label-keyed behaviour follows the key.** MOE callouts (`TaigiCallouts+Builder.swift:35-49`), MOE hints (`ButtonTextProvider.swift:51,90`), MOE2 font (`ButtonFontProvider.swift:45`), 文/A width (`LayoutConverter.swift:102-104`) read the base id + label; shift, `-`, `oo`/`nn` handling read the typed string (`ActionHandler+KeyActions.swift:26,70,134`, `engine/phonetics/src/api.rs:95`).
- **Spare tray** = letters (`a`–`z`, `o͘`, `-`, the base table's letter keys) then symbols (the symbol page) − keys on the board. Symbols keep their long-press callouts (`SymbolCallouts`, keyed by label).
- **Validation on load** — non-TPS base, `keys.count` = the base table's slot count, every placed key in the catalog, no duplicates. An invalid record is dropped, never rendered.

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

- **Editor** (`App/Tabs/Layout/`) — base chips · a drawn key grid (selected key ringed; function keys shown inert) · spare tray. A new layout starts from the active custom layout's base / script, else the stored built-in (PhahTaigi while the layout is TPS) in the current romanization script (TL unless POJ). Base switch / reset clear the selection; Back discards the draft. Save asks for a name (alert, as the theme editor). Keys are buttons with selected traits for VoiceOver. Reset reuses `theme.editorResetAll`.
- **Grid view** (`Layout/`, compiled into app and extension) — takes rows, selection and a tap callback; drawn small as the card preview on the Layout tab and in the in-keyboard picker. No screenshot asset, no KeyboardKit view.
- **Cards** — card menu Apply / Edit / Delete (existing keys). At the cap the create card hides, as on the Theme tab.
- **i18n** — new (iOS scope now, Android joins in phase 2): `layout.editorTitleNew` ("New Layout"; also the blank-name fallback and the name placeholder), `layout.nameHeader` ("Layout Name"). Edit mode titles the editor with the layout's name. `layout.comingSoon` loses its iOS scope in phase 1 and is deleted in phase 2 (Android still reads it, `LayoutScreen.kt:137`).

## Phases

| Phase | Scope | Status |
|---|---|---|
| 1 | iOS — model + store + keyboard + editor + Layout tab shelf + in-keyboard picker | In progress |
| 2 | Android — same model over `LayoutManager.mergeLayouts` (substitute whole `KeyData` so inline popups move with the key) + DataStore string + editor + picker; delete `layout.comingSoon` | Pending |

Phase 1 checks: same-base custom ↔ built-in switch, custom A → B, active edit with a base change, TPS via mode and via card, delete while suspended, reset, globe, email / URL fields, full-width punctuation, shift, callouts, Theme-tab keyboard preview (`KeyboardPreviewPanel.swift:28` also calls `CustomLayoutService`).

## Best practices alignment

| Mainstream practice | Source | This plan |
|---|---|---|
| A custom layout starts from a preset base | `references/azooKey/MainApp/Customize/EditingGridFitCustardView.swift:379-420` (base Custard selection) | Base chips (phases 1–2) |
| User-made items persisted beside settings, capped, shown on a gallery shelf | `ios/.../Theme/UserThemeStore.swift:17-110`, `ThemePickerView.swift:33-51` | Same store shape and shelf (phase 1) |

Rules: `code-review-rules.md` §2 (reuse theme shelf / card / store pattern), §4 (no generic layout DSL); `AGENTS.md` Core Principle 2 (same model, per-platform tables).

**Deliberately not adopted**

- Free per-key text entry, adding keys or rows, key widths, removing function keys — needs validation and warning copy (a board with no ⌫ or ↵); swap + tray + remove keeps the editor free of errors.
- The rarer symbol page (`#+=`, `TaigiLayouts.Symbolic`) in the tray — keeps the tray short.
- An engine-owned layout model — the engine has no notion of layouts (`engine/composing/src/requests.rs:360` is a comment); built-in tables stay per platform (Swift tables / Android JSON).
- A generic user-item store shared with themes (see Storage).
- Custom TPS layouts — TPS is bound 1:1 to its input mode.
- Layout export / sharing (CustardKit format) and `.taigi` backup.
