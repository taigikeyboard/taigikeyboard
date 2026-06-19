# i18n Tier-1 — String Inventory, Scope Classification, Divergence Audit, POJ Validation

> **Type**: Report (frozen dated snapshot). Pure analysis — zero code, zero scope/version commitment.
> **Date**: 2026-06-19.
> **Feeds**: P0 of `docs/architecture/i18n-multilang-plan.md` (string inventory & scope classification) + de-risks D4 (POJ derive).
> **Doc language**: English prose; CJK only for domain terms (漢字/TL/POJ) and verbatim source values.
> **Method**: full read of all 6 `*Texts` files per platform + extension overlays + `SymbolData` + `content/*.json` + `FeatureContent` model; `grep` of extension-layer dirs for `*Texts.*` references to fix scope; node probes of `taigi-converter` for POJ.

---

## 0. Executive summary

- **Localizable surface (excl. changelog)** ≈ **212 iOS keys / ~242 Android keys** across 6 namespaces + **5 hardcoded `SymbolData` labels/platform** + **~80 `content/*.json` strings** (13 features + 3 FAQ). Changelog version-history = **26 entries/platform, EXCLUDED** (USER 2026-06-19), already English.
- **Extension scope is real but small and bounded**: only **SettingsTexts (12 toggle labels) + LayoutTexts (7 layout names) + SymbolData (5 category labels)** cross the host↔extension process boundary. Everything else is host-app-only. This answers the open extension-scope question: extension localization is ~24 strings, not the whole surface.
- **Wording divergence is tiny** — hand-mirroring kept ~98% of shared values byte-identical. Only **2 real value divergences** need a USER canonical pick (`appHeaderTitle`, `devSupplementCredit`); changelog divergences are excluded.
- **Key-NAME divergences (~11) are cosmetic** — the central-`i18n/` semantic key (D5) dissolves them at codegen; they are NOT reconciliation work, only a mapping table.
- **POJ derive (D4) is viable**: corruption surface is narrow, deterministic, statically detectable. Override burden ≈ **~10% of TL strings that embed Latin brand/English tokens**, ~0% of pure-Hanji-plus-clean-TL strings. Build step must add placeholder masking + a brand/English protected-span lint.

---

## 1. Namespace inventory + scope classification

Scope tags (from D5): `host` = settings app only; `extension` = also consumed by keyboard-extension overlay (separate process). All current strings are 漢字-only.

| Namespace | iOS keys | Android keys | Scope | Format args | Notes |
|---|---|---|---|---|---|
| **Common** | 15 | 14 | host | — | dict names (9) + font names (iOS 4) + cancel/viewWebsite. Android adds `ok`/`exportFailed`/`importFailed`; iOS keeps font names here, Android in `LayoutTexts`+`HomeTexts`. |
| **Settings** | 36 | 38 | **host + extension** | — | 12 toggle labels cross to extension overlay (`SettingsSelectionOverlay.swift` / `SettingsOverlayContent.kt`). Info strings (`*Info`) are host-only. Android adds `resetSuccess`/`resetFailed`/`noEmailApp` + `confirmKeyLabel()` fn. |
| **Layout** | 11 | 18 | **host + extension** | — | 7 layout names cross to extension (`LayoutSelectionOverlay`). Android adds 6 color-picker labels (`colorPickerGrid`…`colorBlue`) — iOS uses native `ColorPicker`, no labels. |
| **Theme** | 34 | 34 | host | — | near-identical; Android adds `themeMenu`="主題選項" (iOS lacks). |
| **Dictionary** | ~68 | ~74 | host | **iOS 2 / Android 5** | largest namespace. Format strings below. Android distinguishes 3 import-result toasts; iOS uses 1 generic. |
| **Home** | ~48 | ~50 | host | — | **+ version-history (26 entries, EXCLUDED)**. Credits/licenses (~24) mostly brand names + license IDs (POJ-derive hazards — see §4). |
| `SymbolData` labels | 5 | 5 | **extension** | — | `全形/半形/平仮名/片仮名/顏文字` — **hardcoded in `SymbolData.swift:16-21` / `SymbolData.kt:11-15`, NOT in any `*Texts`**. Per-platform duplicated. In scope; must be lifted into the schema. |
| `content/tab1-features.json` | ~67 strings (13 blocks) | (shared symlink) | host | — | `{"hanji": …}` schema **already multilang-shaped**. |
| `content/tab1-faq.json` | ~13 strings (3 blocks) | (shared symlink) | host | — | same shape. |

**Format-arg constants** (named placeholders `{x}` at codegen per D6; today raw `%d`):
- iOS: `DictionaryTexts.importResultFormat` (`%d %d`), `importBackupResult` (`%d %d %d`). = **2**.
- Android: `importResult` (`%d %d`), `frequencyImportResult` (`%d %d`), `associationImportResult` (`%d %d`), `totalEntries` (`%d / %d`), `importBackupResult` (`%d %d %d`). = **5**.
- Confirms D6 ("≥5, not 2"). English likely adds plurals (`1 item`/`2 items`).

**Oversized / excluded**: `HomeTexts.versionHistoryEntries` (iOS `:64-235`, Android `:69-354`) — 26 versions, already English, platform-specific by design (iOS "73%/48→13MB" vs Android "75%"; different per-platform fixes). **EXCLUDED from multi-language** (USER 2026-06-19) — do not codegen/translate; stays 漢字-chrome + English changes as today.

**content/ multilang readiness (grounded)**: `FeatureContent.swift:24,44,63` decodes a `HanjiText{ hanji }` wrapper; comment `:21-23` says "JSON stores localized text as {"hanji":…} for cross-platform compatibility. iOS only uses the hanji value." → adding `tailo`/`ja`/`en` keys to the same JSON + a language-param decode is the integration path. Android `FeatureContentLoader.kt` reads the same file (symlink). Direct precedent for the i18n design; no schema redesign needed for content.

---

## 2. Extension-scope key list (the cross-process subset — bounded)

Grep of extension-layer dirs (iOS `Overlays/Layout/Actions/Callouts/KeyboardExtension/Styling/Emojis`; Android `ime/`):

**SettingsTexts → extension** (toggle labels in the in-keyboard settings overlay):
`isOutputBothScripts`/`outputBothScripts`, `literalRomanCandidate`, `autoCapitalization`, `autoSpace`, `toolbarAutoCollapse`, `globeKey`, `soundFeedback`, `vibrationFeedback`, `doubleTapOO`, `doubleTapNN`, `isTpsOrMappedToER`/`tpsOrMapsToER`, `openApp`. (Info strings NOT used in extension — host-only.)

**LayoutTexts → extension** (layout-selection overlay):
`romanizationKeyboard`, `taigiPhonetic`, `standardLayout`, `phahTaigiLayout`, `tpsLayout`, `moe1Layout`/`moe2Layout`, `comingSoon` (iOS).

**SymbolData → extension** (symbol overlay, hardcoded): 5 category labels.

**Android-only extension special case**: `SettingsTexts.confirmKeyLabel(inputMode, isTranslateSwapped)` — a **function**, not a constant; returns `選`/`soán`/`suán` by input mode (`SettingsTexts.kt:18-26`), used by `KeyView`. This is localization *logic*, not a static string — D6 typed-function codegen territory; iOS produces this keycap label elsewhere (verify at P0).

→ **Extension localization total ≈ 24 strings + 1 dynamic fn.** Small, well-bounded. The hard part is not the count but the live-switch contract across the process boundary (D7 / §6 risk), not addressed by this Tier-1 pass.

---

## 3. iOS↔Android divergence audit (USER constraint 2)

### 3a. VALUE divergences — need a canonical pick (the actual reconciliation work)

Only **2** real cases (changelog excluded). Hand-mirroring kept everything else byte-identical.

| Concept | iOS | Android | **Canonical (USER 2026-06-19)** |
|---|---|---|---|
| App header title | `台語齒盤` | `Android 台語齒盤` | **`台語齒盤`** both — drop Android's "Android " prefix. (iOS already canonical; Android `HomeTexts.kt:15` edit.) |
| 詞庫增補檔案 credit | `「建中」整理、提供` (brackets) | `建中整理、提供` (no brackets) | **`建中整理、提供`** both — no brackets (matches `accentDictCredit` = `實齋整理、提供`). iOS `HomeTexts.swift:288` drops `「」`. ⚠ 致謝「實齋」勿改寶齋 = a *spelling* guard, unrelated. |

Edits deferred to the P0 wording-reconcile round (app source → branch+PR), not made in this analysis pass.

Changelog (`versionHistoryEntries`): heavily platform-specific (app-size %, platform-specific fixes, KeyboardKit vs Compose internals). **EXCLUDED** — no reconciliation.

### 3b. KEY-NAME divergences — dissolved by codegen, NOT reconciliation work

Same concept + same value, different constant name. Under D5 the central `i18n/` semantic key (e.g. `settings.outputBothScripts.title`) is the single source; both platforms generate their accessor from it, so these vanish. Listed only as the migration mapping table:

| Concept | iOS const | Android const |
|---|---|---|
| 括號標註 | `isOutputBothScripts` | `outputBothScripts` |
| TPS or→ㄜ toggle | `isTpsOrMappedToER` | `tpsOrMapsToER` |
| TPS or→ㄜ info | `isTpsOrMappedToERInfo` | `tpsOrMapsToERInfo` |
| 啟用自訂詞庫 | `isCustomDictEnabled` | `customDictEnabled` |
| (+ info) | `isCustomDictEnabledInfo` | `customDictEnabledInfo` |
| 開啟詞頻紀錄 | `isFrequencyRecordingEnabled` | `frequencyRecordingEnabled` |
| (+ info) | `isFrequencyRecordingEnabledInfo` | `frequencyRecordingEnabledInfo` |
| 開啟詞關聯紀錄 | `isAssociationRecordingEnabled` | `associationRecordingEnabled` |
| (+ info) | `isAssociationRecordingEnabledInfo` | `associationRecordingEnabledInfo` |
| 匯入結果 toast | `importResultFormat` | `importResult` |
| 腔口差 dict name | `CommonTexts.accentDict` | `CommonTexts.khpooDict` |

Pattern: iOS uses Swift `is`-prefix on boolean-toggle labels; Android drops it. Naming-analyzer note: the central schema key should be domain-named (`outputBothScripts.title`), neither platform's convention.

### 3c. Platform-only keys — legitimate (D5 scope-aware schema preserves these)

**iOS-only**: `tabBarTitle` on all 6 namespaces (SwiftUI TabView item vs nav title; value duplicates `tabTitle`); `CommonTexts.font*` (4 font names — Android keeps them in `LayoutTexts`+`HomeTexts`); `LayoutTexts.comingSoon`.

**Android-only**: `resetSuccess`/`resetFailed`/`noEmailApp` (`SettingsTexts`); `confirmKeyLabel()` fn; `colorPickerGrid/Spectrum/Sliders` + `colorRed/Green/Blue` (native iOS picker has no labels); `themeMenu`; `delete`/`ok`/`exportFailed`/`importFailed` (`CommonTexts`); `dictionarySettings`, `entriesCount`, `importExportHelpTitle`/`importExportHelp`, `frequencyImportResult`, `associationImportResult`, `customDictionarySource`, `frequentWordsManagement`, `frequencyTab`, `associationTab`, `totalEntries`, `backupHelpTitle`/`backupHelp`; `setupGuideStartSetup`; font-title consts (`openFontTitle`/`iansuiFontTitle`/`genYoMinFontTitle`/`genYoGothicFontTitle`).

**Structural divergence worth flagging (behavior, not wording)**: Android distinguishes **3 import-result toasts** (`importResult` / `frequencyImportResult` / `associationImportResult`) where iOS reuses **1** (`importResultFormat`). Not a wording fix — a UX-granularity difference. Note for P0; do not silently unify.

→ For D5, none of §3c forces a fake string; the scope-aware key check ("referenced key exists + matches scope") accommodates all of them.

---

## 4. POJ converter validation (de-risk D4)

Probed `taigi-converter` (`convert(text,"tl","poj")`, entry `converter.js:11`) against UI-string-shaped inputs. Tokenizer = `SYLLABLE_RE` (`converter.js:9`): a maximal Latin+combining run with one optional trailing digit; each run must parse as exactly one `TL_INITIAL`+`TL_FINAL` or it's returned verbatim (`converter.js:46`).

**Failure mode (single, deterministic)**: a single-syllable-shaped English/brand token that ALSO triggers a POJ substitution (`tables.js`: `ts→ch`, `tsh→chh`, `nn→ⁿ`, `oo→o͘`, `ua→oa`, `ue→oe`, `ing→eng`, `ik→ek`). Examples that corrupt: `Sing→Seng`, `King→Keng`, `Sue→Soe`, `Sua→Soa`, `Sann→Saⁿ`. Multi-syllable English (`Settings`, `Cancel`, `Keyboard`), URLs, `{placeholders}`, digits, Hanji, punctuation all pass verbatim.

| Class | Verdict |
|---|---|
| Pure Hanji + clean TL phrase | PASS (control, derive clean) |
| Brand multi-syllable (`Gmail`,`iCloud`,`App Store`,`POJ`,`FAQ`) | PASS (verbatim) |
| URLs / `{count}` `{imported}` / digits / punctuation | PASS (delimiters split them out) |
| Single-syllable English hitting a sub (`Sing`,`Sue`,`Sua`,`-nn`) | **CORRUPT** |
| Short Latin parsing as Taigi but no sub today (`Pin`,`Tan`,`Map`,`Go`) | PASS-but-latent (time-bomb if subs grow) |
| Dialectal finals (`ir-`/`er-`/`ee-`) | PASS — no LOSS class observed (POJ==TL or has a form) |

**Measured**: 7/68 synthetic English UI words = **10.3%** corrupt; real rate lower (most labels multi-syllable). **No LOSS class** — derive never yields "no POJ".

**D4 verdict: VIABLE as default + override.** Build step MUST add: (1) mask `{…}` spans before convert, restore after (defense-in-depth); (2) brand/English protected-span or `pojOverride`/`derivePoj:false` for the ~10% Latin-embedding strings — concentrated in §3 credits/licenses (brand names) + any loanword label; (3) a lint flagging any TL token in the sub-trigger set so the author confirms/overrides (closes the latent-corruption gap); (4) generated-diff human review (build-pass ≠ language-correct). Corruption is narrow, deterministic, statically detectable — the D4 mechanism is necessary and sufficient with these guards.

---

## 5. Recommendations + open Tier-0 decisions (USER-gated)

**Tier-1 conclusions (factual, no scope/version commitment):**
1. **Extension scope is cheap** (~24 strings + 1 fn) and unavoidable — `SymbolData` labels + Settings/Layout overlay labels already cross the boundary. Lifting `SymbolData`'s 5 hardcoded labels into the schema is the only net-new extraction; the rest are already in `*Texts`.
2. **FAQ/features content is the largest single block** (~80 strings) but the schema is **already multilang-shaped** — lowest-risk, highest-volume; integration = add language keys + language-param decode.
3. **Wording reconciliation is nearly free** — 2 value picks; key-name divergences dissolve at codegen.
4. **D4 confirmed viable** with the 4 build-step guards above.

**Tier-0 decisions — RESOLVED 2026-06-19:**
- **Scope** (USER delegated → Claude): **all surfaces in scope** — host chrome + extension (~24) + FAQ content (~80). None deferred. Rationale: mixed-language UI is the failure mode (English picker + 漢字-only FAQ/overlay = broken feature); schema/codegen cost is flat across surfaces; extension live-switch (D7) is the #1 risk that must be solved for a credible multilang keyboard regardless. Per-surface TL/ja/en authoring rides each language phase (P2 en → P3 ja/TL/POJ), not a separate deferral. Aligns with the approved P1 "host + extension prototype".
- **`appHeaderTitle`** (USER): canonical `台語齒盤` both — drop "Android " prefix.
- **`devSupplementCredit`** (USER): canonical `建中整理、提供` both — no brackets.
- ⚠ **iOS `pbxproj`/`Info.plist` = user-only** (Core Principle #1): adding `CFBundleLocalizations` / declared `.lproj` for TL/POJ (D2 spike) needs USER project-config edits — Claude cannot.

The two wording edits land in the P0 reconcile round (app source = branch+PR), not this pass.

**Not done in Tier-1 (by design)**: live-switch reactive prototype (Tier-2 #4, the #1 risk) and TL/POJ non-standard-locale resolution spike (Tier-2 #5) — throwaway architecture spikes, separate from this analysis pass.

---

## References
- Plan: `docs/architecture/i18n-multilang-plan.md`; memory `project_i18n_multilang`.
- Sources: `ios/Sources/TaigiKeyboard/Strings/*.swift`, `android/.../localization/*Texts.kt`.
- Extension: `Overlays/SettingsSelectionOverlay.swift`, `ime/text/smartbar/SettingsOverlayContent.kt`, `Overlays/SymbolData.swift` / `ime/text/smartbar/SymbolData.kt`.
- Content: `content/tab1-features.json`, `content/tab1-faq.json`, `App/Tabs/Home/Models/FeatureContent.swift`.
- POJ: `taigi-converter/src/converter.js` (`:9` tokenizer, `:11` entry, `:46` verbatim fallback), `src/poj.js`, `src/tables.js`.
