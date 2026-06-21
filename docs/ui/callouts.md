# Long-press Callouts & Tone-variation Menus

> **Type**: Feature
> **Keywords**: `callout`, `long-press`, `tone-variation`, `popup`, `GetToneVariations`
> **Related**: ../engine/tone.md, ../engine/tps.md, ../architecture/behavioral-invariants.md (popup invariants), ../architecture/keyboard-body-invariants-android.md

---

## Summary

- Long-pressing a key surfaces a callout menu of variants: layout-specific punctuation (TPS / MOE), symbol-page callouts, and **POJ/TL tone variations** (`a → á à â ǎ ā a̍ a̋`, `n → … ⁿ`, `ng → ńg …`).
- The tone-variation map is built once in the Rust engine (`phonetics::tone_variations`) and pulled in a single init-time call (`GetToneVariations`).
- **Architectural divergence**: iOS consumes the engine map live at long-press through a runtime lookup chain; Android pre-bakes static JSON popup tables into each key at layout-load time. See [Divergence](#cross-platform-divergence) — the two are NOT wired the same way.

---

## Files

### Engine

| File | Responsibility |
|------|----------------|
| `engine/phonetics/src/tone_variations.rs` | `build()` returns POJ + TL tone-variation maps in one `ToneVariationsResult` (init-bulk-pull builder) |
| `engine/phonetics/src/dispatch.rs` | Routes `Method::GetToneVariations` → `tone_variations::build()` |
| `engine/protos/proto/phonetics.proto` | `GetToneVariations` request + `ToneVariationsResult` / `ToneVariationList` response |

### iOS (engine-driven, runtime chain)

| File | Responsibility |
|------|----------------|
| `Callouts/Callouts+TaigiCalloutBuilder.swift` | `taigiCalloutActions` — long-press entry point; decides the lookup chain |
| `Callouts/Callouts+TaigiCalloutMaps.swift` | `TaigiToneMaps` (proxies the engine cache), `TPSCallouts`, `MOE1Callouts`, `MOE2Callouts`, `SymbolCallouts` |
| `Callouts/Callouts+TaigiCalloutStyle.swift` | `CalloutStyle.taigi(for:)` font sizing |
| `Engine/RustEngineBridge+Phonetics.swift` | `static let toneVariations` lazy cache `{ poj, tl: [String:[String]] }` |

### Android (JSON-driven, pre-baked)

| File | Responsibility |
|------|----------------|
| `ime/text/layout/LayoutManager.kt` | `loadExtendedPopups` loads `taigi_{poj,tl}.json` by input mode; merges into `keyData.popup` |
| `assets/ime/text/characters/extended_popups/taigi_{poj,tl}.json` | Static tone-variation popup tables (18 keys each) |
| `ime/popup/KeyPopupManager.kt` | `PopupHost` impl: preview + extended `PopupWindow`, motion hit-test, `activeKeyData()` |
| `ime/popup/PopupCellResolver.kt` | `buildPopupCells` maps `KeyData.popup` → visual cells |

---

## Engine: the tone-variation map

`tone_variations::build()` returns a single `ToneVariationsResult { poj_variations, tl_variations }` — **both maps in one call** (init-bulk-pull). Each map is `base char → [toned variants]`.

- **Bases**: vowels `a e i o u` (+ uppercase), syllabic `n m`, `ng`, plus a mode-specific `o` variant (TL `oo`, POJ `o͘` = `o`+U+0358). The `n` entry also appends nasal `ⁿ` (U+207F).
- **Tones**: `2 3 5 6 7 8 9`. Tone 9 differs by mode — TL double-acute (U+030B), POJ breve (via `TONE_NUM_TO_COMBINING`). Output is NFC-recomposed.
- **No engine-side caching** — the engine is stateless. Each platform fetches once at first access and caches platform-side (proto comment: "Caller caches once at engine init").
- **Proto**: request `GetToneVariations {}` (empty); response `map<string, ToneVariationList>` for `poj_variations` + `tl_variations`.

On iOS the platform-side `buildToneMap` helpers were deleted in D9.4; `TaigiToneMaps.poj` / `.tl` now proxy `RustEngineBridge.toneVariations` (a thread-safe `static let` loaded once via FFI).

---

## Callout lookup order

**iOS** — single runtime chain in `taigiCalloutActions`, first match wins:

1. Non-character action → KeyboardKit default English callouts.
2. **Layout-specific** by `keyboardLayoutType`: `.tps` → `TPSCallouts`, `.moe1` → `MOE1Callouts`, `.moe2` → `MOE2Callouts`.
3. **Symbol callouts** → `SymbolCallouts[char]`.
4. **Tone variations** → `inputMode == .poj ? TaigiToneMaps.poj : TaigiToneMaps.tl`, indexed by char.

So `layout → symbol → POJ/TL tone`. TPS/MOE punctuation overrides symbols; symbol-only chars always get callouts regardless of layout.

**Android** — no runtime chain. Popups are pre-resolved into `KeyData.popup` at layout-build time: `loadExtendedPopups` selects JSON by input mode (`poj`/`tl` → the tone tables; `english`/`tps` → empty), merged by label match during `mergeLayouts`. At long-press, `KeyPopupManager` renders `anchor.popupCells` and `activeKeyData()` returns the selected variant or falls back to `anchor.data`. No engine call, no symbol/MOE branching in the popup layer — whatever the layout JSON baked in is what shows.

---

## Cross-platform divergence

| Aspect | iOS | Android |
|---|---|---|
| Tone-variation source | **Engine** `GetToneVariations`, indexed live at long-press | **Static JSON** `taigi_{poj,tl}.json`, baked into `KeyData.popup` at layout load |
| Engine cache used by UI? | Yes (`TaigiToneMaps` proxies it) | **No** — `PhoneticsBridge.toneVariations` exists but has no UI consumer (dead cache) |
| Lookup chain | Runtime `layout → symbol → tone` | None at press time; pre-resolved into `KeyData.popup` |
| Selection on commit | KeyboardKit `ActionsBuilder` | `KeyPopupManager.activeKeyData()` (motion hit-test) → `KeyTouchCoordinator` |

> ⚠ **Known duplication risk.** The Android `taigi_tl.json` content is a manual static duplicate of the engine's `build_mode_map` output (verified identical: `a → á à â ǎ ā a̍ a̋`, `n` ends in `ⁿ`, `ng → ńg …`, 18 keys). Changing the engine tone tables updates iOS automatically but leaves Android's JSON stale, with no `// CROSS-PLATFORM INVARIANT` comment linking the two. The dead `PhoneticsBridge.toneVariations` cache suggests an intended-but-incomplete migration to engine-sourced Android popups. Treat the JSON as a mirror that must be hand-synced until that migration lands; see `.claude/rules/cross-platform-alignment.md` §3a.

---

## Constants

- `TONE_NUMBERS = ["2","3","5","6","7","8","9"]`; tone-9 mark TL U+030B / POJ breve; POJ `o͘` = U+0358; nasal `ⁿ` = U+207F.
- iOS callout font: action 20 pt / input 32 pt (custom fonts only).
- Android tone-popup JSON: 18 keys each, loaded only for Taigi layouts in POJ/TL mode (TPS/English get no tone popups).
- Android `KeyPopupManager.ANCHOR_EXCEPTIONS = {ENTER, LANGUAGE_SWITCH, SWITCH_TO_TEXT_CONTEXT, SWITCH_TO_MEDIA_CONTEXT}`.

---

## See also

- `engine/tone.md` (tone normalize/restore), `engine/tps.md` (TPS/MOE layout callouts source).
- `architecture/keyboard-body-invariants-android.md` (popup touch + geometry invariants).
- `architecture/behavioral-invariants.md` (`INVARIANT_keyboard_popup_hide_*`).
