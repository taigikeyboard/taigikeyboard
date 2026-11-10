# Long-press Callouts & Tone-variation Menus

> **Type**: Feature
> **Keywords**: `callout`, `long-press`, `tone-variation`, `popup`, `GetToneVariations`
> **Related**: ../engine/tone.md, ../engine/tps.md, ../architecture/behavioral-invariants.md (popup invariants), ../architecture/keyboard-body-invariants-android.md

---

## Summary

- Long-pressing a key surfaces a callout menu of variants: layout-specific punctuation (TPS / MOE), symbol-page callouts, and **POJ/TL tone variations** (`a → á à â ǎ ā a̍ a̋`, `n → … ⁿ`, `ng → ńg …`).
- The tone-variation map is built once in the Rust engine (`phonetics::tone_variations`) and pulled in a single init-time call (`GetToneVariations`).
- Both platforms read that one engine map. **Architectural divergence** in how it reaches a key: iOS indexes it live at long-press through a runtime lookup chain; Android bakes it into each key's popup at layout-load time. See [Divergence](#cross-platform-divergence).

---

## Files

### Engine

| File | Responsibility |
|------|----------------|
| `engine/phonetics/src/tone_variations.rs` | `build()` returns POJ + TL tone-variation maps in one `ToneVariationsResult` (init-bulk-pull builder) |
| `engine/phonetics/src/requests.rs` | Routes `Method::GetToneVariations` → `tone_variations::build()` |
| `engine/protos/proto/phonetics.proto` | `GetToneVariations` request + `ToneVariationsResult` / `ToneVariationList` response |

### iOS (engine-driven, runtime chain)

| File | Responsibility |
|------|----------------|
| `Callouts/TaigiCallouts+Builder.swift` | `taigiCalloutActions` — long-press entry point; decides the lookup chain |
| `Callouts/TaigiCallouts+Maps.swift` | `TaigiToneMaps` (proxies the engine cache), `TPSCallouts`, `MOE1Callouts`, `MOE2Callouts`, `SymbolCallouts` |
| `Callouts/KeyboardCalloutStyle+Taigi.swift` | `KeyboardCalloutStyle.taigi(for:)` font sizing |
| `Engine/RustEngineBridge+Phonetics.swift` | `static let toneVariations` lazy cache `{ poj, tl: [String:[String]] }` |

### Android (engine-driven, pre-baked)

| File | Responsibility |
|------|----------------|
| `engine/PhoneticsBridge.kt` | `RustEngineBridge.toneVariations` lazy cache (`ToneVariations { tl, poj }`, DTO in `RustEngineBridge.kt`); `CompositionRoot.toneVariations` hands it to the two readers below (JVM tests pass a stand-in) |
| `ime/text/layout/LayoutManager.kt` | Adds the input mode's tone marks (`LayoutScript.toneMarks`) to each matching key's `keyData.popup` as fresh `KeyData.ofEntry` keys; then `CustomLayoutTables.applyCallouts` puts a custom layout's own lists in |
| `ime/text/layout/CustomLayoutTables.kt` | The custom-layout editor's view of the same table: `defaultCallouts`, tray `toneRows` |
| `ime/popup/KeyPopupManager.kt` | `PopupHost` impl: preview + extended `PopupWindow`, motion hit-test, `activeKeyData()` |
| `ime/popup/PopupCells.kt` | `buildPopupCells` maps `KeyData.popup` → visual cells |

---

## Engine: the tone-variation map

`tone_variations::build()` returns a single `ToneVariationsResult { poj_variations, tl_variations }` — **both maps in one call** (init-bulk-pull). Each map is `base char → [toned variants]`.

- **Bases**: vowels `a e i o u` (+ uppercase), syllabic `n m`, `ng`, the dialectal vowels `er ir or` (mark on the vowel, same spelling in TL and engine POJ; the custom-layout tray offers them as keys), plus a mode-specific `o` variant (TL `oo`, POJ `o͘` = `o`+U+0358). The `n` entry also appends nasal `ⁿ` (U+207F). Two-letter bases have Shift (`Er`) and Caps Lock (`ER`) keys.
- **Tones**: `2 3 5 6 7 8 9`. Tone 9 differs by mode — TL double-acute (U+030B), POJ breve (via `TONE_NUM_TO_COMBINING`). Output is NFC-recomposed.
- **No engine-side caching** — the engine is stateless. Each platform fetches once at first access and caches platform-side (proto comment: "Caller caches once at engine init").
- **Proto**: request `GetToneVariations {}` (empty); response `map<string, ToneVariationList>` for `poj_variations` + `tl_variations`.

Neither platform builds a tone map: iOS `TaigiToneMaps.poj` / `.tl` proxy `RustEngineBridge.toneVariations` (a thread-safe `static let` loaded once via FFI); Android `RustEngineBridge.toneVariations` is a synchronized `lazy` (layouts compute on the main thread and `Dispatchers.IO`). On failure both keep empty maps. The engine tests pin every base's full list.

---

## Callout lookup order

**iOS** — single runtime chain in `taigiCalloutActions`, first match wins:

1. Non-character action → KeyboardKit default English callouts.
2. **Layout-specific** by `keyboardLayoutType`: `.tps` → `TPSCallouts`, `.moe1` → `MOE1Callouts`, `.moe2` → `MOE2Callouts`.
3. **Symbol callouts** → `SymbolCallouts[char]`.
4. **Tone variations** → `inputMode == .poj ? TaigiToneMaps.poj : TaigiToneMaps.tl`, indexed by char.

So `layout → symbol → POJ/TL tone`. TPS/MOE punctuation overrides symbols; symbol-only chars always get callouts regardless of layout.

**Android** — no runtime chain. Popups are pre-resolved into `KeyData.popup` at layout-build time: `mergeLayouts` picks the engine map by input mode (`poj`/`tl` → the tone tables; `english`/`tps` → empty) and adds a key's list by label match. An entry of one code point carries it as `code`; longer text has code 0 and types its label (`TextInputKeyHandler` commits the label whenever it differs from the code's character). At long-press, `KeyPopupManager` renders `anchor.popupCells` and `activeKeyData()` returns the selected variant or falls back to `anchor.data`. No engine call at press time, no symbol/MOE branching in the popup layer — whatever the layout build baked in is what shows, except that an active custom layout's own list (TL / POJ letter page) replaces a key's whole list (`CustomLayoutTables.applyCallouts`, after the extended popups).

---

## Cross-platform divergence

| Aspect | iOS | Android |
|---|---|---|
| Tone-variation source | **Engine** `GetToneVariations`, indexed live at long-press | **Engine** `GetToneVariations`, baked into `KeyData.popup` at layout load |
| Engine cache used by UI? | Yes (`TaigiToneMaps` proxies it) | Yes (`CompositionRoot.toneVariations` → `LayoutManager`, `CustomLayoutTables`) |
| Lookup chain | Runtime `layout → symbol → tone` | None at press time; pre-resolved into `KeyData.popup` |
| Selection on commit | KeyboardKit `ActionsBuilder` | `KeyPopupManager.activeKeyData()` (motion hit-test) → `KeyTouchCoordinator` |

---

## Constants

- `TONE_NUMBERS = ["2","3","5","6","7","8","9"]`; tone-9 mark TL U+030B / POJ breve; POJ `o͘` = U+0358; nasal `ⁿ` = U+207F.
- iOS callout font: action 20 pt / input 32 pt (custom fonts only).
- Android tone popups: loaded only for Taigi layouts in POJ/TL mode (TPS/English get none). Android matches lowercase labels only (letter-page labels are lowercase; `transformInputCase` handles case), so the map's capitalised keys (`A`, `Oo`, `Ng`, `OO`, `NG` …) serve iOS.
- Android `KeyPopupManager.ANCHOR_EXCEPTIONS = {ENTER, LANGUAGE_SWITCH, SWITCH_TO_TEXT_CONTEXT, SWITCH_TO_MEDIA_CONTEXT}`.

---

## See also

- `engine/tone.md` (tone normalize/restore), `engine/tps.md` (TPS/MOE layout callouts source).
- `architecture/keyboard-body-invariants-android.md` (popup touch + geometry invariants, `INVARIANT_keyboard_popup_hide_*`).
