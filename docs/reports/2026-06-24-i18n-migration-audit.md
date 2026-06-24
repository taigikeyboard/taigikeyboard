# i18n Migration Coverage Audit — 2026-06-24

**Method**: 5 parallel investigator agents, one per surface (iOS host / iOS extension / Android host / Android IME / shared+exclusion-ledger). Each hunted every user-facing display string (`Text`, `.accessibilityLabel`/`contentDescription`, `.alert`, Toast, navigation/share metadata) and classified MIGRATED / LEAK / EXCLUSION with `file:line`. This doc is the synthesized ledger. **Read-only audit — no code changed, no scope assigned (release scope is USER-gated).**

LEAK = a hardcoded user-facing string (CJK or English) that a user SEES or a screen-reader HEARS and that does NOT route through the display-language picker resolver, and is not a documented intentional exclusion.

---

## Verdict summary

| Surface | State |
|---|---|
| `i18n/` flat namespaces (common/settings/layout/theme/home/dictionary/symbol/keyboard) | ✅ complete, 5-lang, gated by `test_i18n.py` (68 tests) |
| `i18n/content/` (FAQ + features) | ✅ fully 5-lang (80 strings × 5), gated by `ProductionContentTests` |
| Symbol category labels (全形/半形/平仮名/片仮名/顏文字) | ✅ MIGRATED both platforms (`symbol.json`) |
| Engine-layer errors → UI | ✅ no direct leak; English dev-fallback + App-layer `StringKey` mapping both platforms |
| Host-app chrome (settings/home/dictionary/theme/layout) | ✅ migrated, except the gaps below |
| Leak-closure PR1/PR2/PR3 (alerts, candidate-overlay a11y, smartbar/media a11y) | ✅ confirmed complete — Android smartbar/media a11y fully on `applyAccessibilityStrings` resolver, no static `@string/` left |

**Remaining gaps = 6 categories below.** Biggest = the entire **iOS keyboard-extension a11y** layer was never migrated (the `keyboard.json` namespace is scoped `platforms:["android"]` only).

---

## Remaining gaps (net-new findings)

### G1 — iOS keyboard-extension a11y NEVER migrated (cross-platform a11y parity gap) — LARGEST
`keyboard.json` (12 keys: expandCandidates/pageUp/pageDown/translateToggle/morePopupHint/toggleToolbar/symbolPanel/switchLayout/switchInputMethod/dismissKeyboard/settings/deleteIcon) is `platforms:["android"]` only. iOS `StringKey` has zero `keyboard*` cases. Every iOS extension a11y label is hardcoded; git history confirms these files predate the i18n work.

| file:line | string | issue |
|---|---|---|
| `Autocomplete/Views/ToolShortcutsToolbar.swift:50` | 收合工具列/展開工具列 | hardcoded CJK a11yLabel |
| `ToolShortcutsToolbar.swift:63-64` | 符號面板 / 點擊以開啟符號選擇面板 | hardcoded a11yLabel/Hint |
| `ToolShortcutsToolbar.swift:70-71` | 佈局選擇 / 點擊以開啟佈局選擇面板 | hardcoded |
| `ToolShortcutsToolbar.swift:79-80` | **"Dismiss Keyboard" / "Tap to dismiss keyboard"** | hardcoded **English** |
| `ToolShortcutsToolbar.swift:86-87` | 設定 / 點擊以開啟鍵盤設定 | hardcoded |
| `ToolShortcutsToolbar.swift:114-115` | `\(label) 輸入模式` / 點擊切換至… / 目前選擇 | hardcoded |
| `ToolShortcutsToolbar.swift:124-125` | 切換鍵盤 / 點擊切換下一個鍵盤… | hardcoded |
| `Autocomplete/Views/CandidateSuggestionsRow.swift:120-121` | 收合候選詞/展開候選詞 + hint | hardcoded |
| `Overlays/ExpandedCandidateOverlay.swift:139-146` | (collapse `chevron.up`) | **MISSING a11yLabel entirely** |
| `ExpandedCandidateOverlay.swift:150-157` | (page-up) | MISSING a11yLabel |
| `ExpandedCandidateOverlay.swift:159-166` | (page-down) | MISSING a11yLabel |
| `ExpandedCandidateOverlay.swift:169-176` | (translate toggle) | MISSING a11yLabel |
| `Overlays/ExpandedCandidateControlButton.swift:16-33` | shared control button | no a11y label slot |

The 4 ExpandedCandidateOverlay buttons = the previously-known Codex-flagged parity gap (its `keyboard.json` comment documents it). The full smartbar/candidate scope is BROADER than that one flag. Android already has all of this via `keyboard.json` + `applyAccessibilityStrings`. Fixing G1 = widen `keyboard.json` keys to iOS scope + emit iOS `StringKey` + wire the iOS extension views to the resolver.

### G2 — Built-in theme names hardcoded (BOTH platforms, cross-platform invariant)
Rendered as Theme-picker family headers + card titles. No documented exclusion.

| platform | file:line | strings |
|---|---|---|
| iOS | `Settings/BuiltInThemes.swift:27-29` | 經典 / 框線 / 簡潔 (family) |
| iOS | `Settings/BuiltInThemes.swift:88-94` | 預設 / 櫻花 / 金煌 / 海風 / 翠青 / 藤紫 / 暗眠山貓 (color) |
| Android | `ime/core/BuiltInThemes.kt:55-57` | 經典 / 框線 / 簡潔 |
| Android | `ime/core/BuiltInThemes.kt:52-59` | 預設 / 櫻花 / 金煌 / 海風 / 翠青 / 藤紫 / 暗眠山貓 |

Carry `CROSS-PLATFORM INVARIANT` comments — any fix must align both.

### G3 — Dictionary-source descriptions (長文) + source-name tags (BOTH platforms) — Tier-D SCOPE QUESTION
Memory records a prior decision: **"Tier-D dict-desc 長文 OUT (content, same tier as version-history)"**. These match that. Listed here for completeness + because USER's framing ("除了 version-history 之類固定英文之外") may or may not include dict-desc. **Scope call for USER, not auto-resolved.**

| platform | file:line | kind |
|---|---|---|
| iOS | `App/Tabs/Dictionary/DictionaryTab.swift:109,150,156,162,206,213` | 6 toggle descriptions (長文) |
| iOS | `App/Tabs/Dictionary/Models/DictionaryInfo.swift:15-59` | 12 info-alert descriptions (長文) |
| Android | `ui/tabs/dictionary/DictionarySettingsScreen.kt:175,247,258,269,353,364` | 6 toggle descriptions |
| Android | `ui/tabs/dictionary/DictionarySettingsComponents.kt:45-51` | 7 info descriptions |
| Android | `ime/dictionary/DictionarySource.kt:19-31` | 9 source display-name **tags** (教典/台語新詞/…) rendered in `SearchResultRow` — SHORT labels, arguably distinct from 長文 |

Note: the short source-name tags (教典 etc.) are a different shape from the 長文 descriptions — if Tier-D was specifically about 長文, the tags may warrant separate consideration. iOS likely has a sibling source-name source (verify `DictionaryInfo`/source displayName at fix time).

### G4 — Android IME keycap / emoji a11y leaks (Android-only)
PR3 covered smartbar/media a11y; these are OTHER IME surfaces PR3 didn't touch.

| file:line | string | note |
|---|---|---|
| `res/layout/media_input_layout.xml:31` | "ABC" via `@string/key__view_characters` | **visible button label** via OS-locale, bypasses picker — clearest leak |
| `ime/media/emoji/EmojiPaletteView.kt:181` | `category.toString()` → "SMILEYS & EMOTION"… | **English** emoji-tab `contentDescription`, TalkBack-heard |
| `ime/text/keyboard/KeyContent.kt:452-453` | "Pause" / "Wait" (`key__phone_*`) | phone-keypad keycaps, OS-locale (borderline — English conventional) |
| `ime/text/keyboard/KeyContent.kt:459-460` | "ABC" (`key__view_characters`) | mode-switch keycap, OS-locale |
| `ime/popup/PopupCellResolver.kt:33` | "ABC" | key-popup cell, OS-locale |

iOS keycap equivalents not separately flagged (iOS keycaps use phonetic glyph providers). The `media "ABC"` + emoji-category English a11y are the strongest.

### G5 — Small iOS host leaks (low-effort, resolver keys mostly exist)
| file:line | string | note |
|---|---|---|
| `App/Components/SettingInfoButton.swift:22` | `Button("OK")` | hardcoded English; `.commonOk` resolver key EXISTS |
| `App/Tabs/Dictionary/Models/FrequencyDataViewModel.swift:87` | "Cannot read file" | NSError English, shown on import-fail alert; no error-message key yet |
| `App/Tabs/Dictionary/Models/AssociationDataViewModel.swift:71` | "Cannot read file" | same |
| `App/Tabs/Settings/SettingsTab.swift:268` | 台語齒盤 Bug 回報 | ShareLink subject (user sees in share sheet) |
| `App/Tabs/Settings/SettingsTab.swift:276` | 台語齒盤 Bug 回報 (v…) | mailto subject (user sees as email subject) |

### G6 — Android a11y null + export filenames
| file:line | string | note |
|---|---|---|
| `ui/tabs/dictionary/CustomDictionaryScreen.kt:186` | `contentDescription = null` (Add icon) | actionable icon silent to TalkBack — known neighbor flag, CONFIRMED still present |
| `settings/DataManagementActivity.kt:112` | 備份復原_$date.taigi | export filename (system save picker) — borderline |
| `ui/tabs/dictionary/AssociationDataScreen.kt:215` | 詞關聯紀錄_$date.csv | export filename — borderline |
| `ui/tabs/dictionary/CustomDictionaryScreen.kt:256` | 自訂詞庫_$date.csv | export filename — borderline |
| `ui/tabs/dictionary/FrequencyDataScreen.kt:213` | 詞頻紀錄_$date.csv | export filename — borderline |

---

## Intentional-exclusion ledger (confirmed CORRECT — do NOT migrate)

| Item | proof (file:line) | reason |
|---|---|---|
| Version-history / changelog | iOS `App/Tabs/Home/VersionHistory.swift`; Android `content/VersionHistory.kt` | fixed English, synced from `changelog/*.md`, language-invariant (USER decision) |
| License codes (SPDX/CC*/CC0/SIL OFL/OGDL) | iOS `CopyrightView.swift` `private enum License`; Android `content/CopyrightData.kt` const | language-invariant identifiers |
| Nav-chrome tab titles | iOS `TabType.swift:35-43` inline; Android native `R.string.tab_*` | follow OS locale, not in-app picker, by design |
| AndroidManifest `android:label` | `AndroidManifest.xml:27,35,52,62,76,92` | OS system chrome (launcher/activity labels) |
| App display name / InfoPlist | iOS `Info.plist` + `.lproj/InfoPlist.strings`; Android `R.string.app_name` | OS-managed identity, separate scope |
| iOS NavigationStack back button | `common.back` scoped `["android"]` only | iOS back is OS-rendered/OS-locale; intentional platform divergence |
| Romanization acronyms (POJ/TL/EN/TPS), numeric/symbol keycaps, Enter keycap 選/soán/suán | `ToolShortcutsToolbar.swift:56-59`; `smartbar.xml:122-143`; `KeyboardLayout.kt:447-449` | language-invariant / phonetic keycap content |
| Symbol/kaomoji glyph DATA | `SymbolData.swift:46-125` | the characters inserted, not chrome |
| Engine-layer error dev-fallbacks | iOS `CustomDictionaryService.swift:204-221`; Android `DictionaryError.kt` | English dev-only; App layer maps case→StringKey for display |

---

## Scope questions for USER (NOT auto-resolved)

1. **G1 iOS extension a11y parity** — widen `keyboard.json` to iOS + wire iOS extension a11y to resolver? (largest gap; Android already done; pure a11y, no visible-text change)
2. **G2 theme names** — migrate 經典/櫻花/… to i18n on both platforms? (cross-platform invariant)
3. **G3 dict-source 長文 + tags** — confirm Tier-D "OUT, content-tier like version-history" still holds, OR pull the SHORT source-name tags (教典…) in separately?
4. **G4/G5/G6** — small leaks (media "ABC", emoji a11y English, iOS OK/Cannot-read-file/bug-subjects, Android Add-icon a11y null, export filenames). Fix opportunistically or leave?

No version assigned to any of the above — release scope is USER's call.

---

## Migration plan — decisions LOCKED 2026-06-24 (USER)

USER chose to migrate **G1, G2, G3, G5, G6** (G4 Android keycaps/emoji NOT migrated). Decisions:

- **i18n folder stays FLAT** — no new `content/` subfolder. The existing `i18n/content/` (FAQ/features) is a nested-tree document system (multi-paragraph, structurally cannot be flat) and stays as-is; G3 does NOT join it.
- **G3 長文 IN** — but routed to FLAT `dictionary.json` keys (each dict description is a single string → fits flat key-value; no nested content needed).
- **G2 theme names** — migrate fully, all 5 langs (the completeness gate forbids half-authoring; poetic color names get en/ja localization, USER proofreads).
- **G5 bug-report subject → FIXED ENGLISH** (not i18n; developer-facing email metadata). Change `台語齒盤 Bug 回報` → e.g. `Taigi Keyboard Bug Report`.
- **G6 export filenames → FIXED ENGLISH** (not i18n; persisted-artifact stability). Change `備份復原_/詞關聯紀錄_/自訂詞庫_/詞頻紀錄_` → ASCII English; align iOS+Android.

### Phase plan (risk/value ordered, NO version assigned — USER-gated per round)

| Round | Scope | New keys (TL/POJ authoring) | Platform | Tier |
|---|---|---|---|---|
| **R-G1a** | widen 12 `keyboard.json` keys `["android"]`→`["android","ios"]` + emit iOS `StringKey`/xcstrings + wire label-only iOS ext a11y (toggleToolbar/symbolPanel/switchLayout/dismissKeyboard/settings/switchInputMethod/expandCandidates) via `@Environment(DisplayLanguageStore.self)` | **0 new** (12 already 5-lang) | iOS | quick parity win |
| **R-small (G5+G6)** | G5: `Button("OK")`→`commonOk`; "Cannot read file"→1 new key; bug-subject→fixed English. G6: CustomDictionary Add-icon a11y→1 new key; export filenames→fixed English (iOS+Android align) | ~2 small | iOS+Android | small |
| **R-G1b** | iOS ext a11y NEW keys: ~6 `accessibilityHint` sentences + 3 input-mode a11y (format arg `\(label)`) + 4 `ExpandedCandidateOverlay` missing labels (reuse expandCandidates/pageUp/pageDown/translateToggle) | ~9 (hints=iOS-scope, full Taigi sentences) | iOS | medium (authoring) |
| **R-G2** | theme names → `theme.json`: 3 family + 7 color = 10 keys; both platforms data-struct → `labelKey` (LayoutOption/ColorPickerTarget precedent) | 10 (incl poetic en/ja) | iOS+Android | small-medium |
| **R-G3** | dict-source descriptions (長文, iOS 6+12 / Android 6+7) + source-name tags (~9, 教典/…) → FLAT `dictionary.json`; both platforms | ~18 長文 + ~9 tags = HEAVIEST | iOS+Android | large (authoring) |

Authoring rounds (G1b/G2/G3) need: hanji (move) + en/ja (Claude drafts, USER reviews PR) + TL/POJ (grounded in kautian.csv/corpus/taigi-converter, USER proofreads) per Core Principle #3. Each round = branch+PR+Codex sandwich (design pre-impl for authoring rounds).
