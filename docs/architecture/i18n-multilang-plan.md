# App UI i18n — Multi-Language Plan

> **Type**: Planning (forward-looking, multi-PR) + architecture design
> **Status**: Design approved + Codex design co-review folded in (2026-06-19). NO code yet.
> **Scope/timing/version**: USER-gated. USER 2026-06-19: in-repo CONFIRMED; changelog EXCLUDED from multi-language; tentatively targeting **v3.6.4** (USER said 「可能」/possibly — not a firm commitment). Phases below are work ordering + cost.
> **Doc language**: English prose per repo doc-authoring rule (CJK only for verbatim USER quotes + domain terms 漢字/TL/POJ).

---

## Goal

Localize the **app UI chrome** into **5 display languages**:

1. 台語 TL 羅馬字 (Tâi-lô) — `DisplayLanguage.tailo`
2. 台語 POJ 羅馬字 (Pe̍h-ōe-jī) — `DisplayLanguage.poj`
3. 台語漢字 (Taiwanese Hanji) — `DisplayLanguage.hanji` (the app's current single language)
4. 日語 (Japanese) — `DisplayLanguage.japanese`
5. 英語 (English) — `DisplayLanguage.english`

Plus a `DisplayLanguage.system` (Automatic) state.

Behavior:

- **Automatic (default)** resolves from OS locale: `ja* → 日語`, `zh* → 台語漢字`, everything else → `英語`. Automatic is a persisted *state*, not a one-time seed — the user can always return to it after overriding.
- **In-app language picker** (Settings) overrides Automatic and exposes all 5 explicit languages. TL/POJ are only reachable here (not OS-distinguishable).
- Selecting a language switches the display text **live** (no app restart).

This is **app UI localization** — distinct from the keyboard *input* mode (TL/POJ/TPS/English input), a separate existing axis. A user can type in TL while reading the UI in Japanese.

**Surface scope is NOT host-app-only** (Codex correction): the `*Texts` constants are also consumed by the **keyboard-extension overlays** (`SettingsSelectionOverlay.swift:76`, `SettingsOverlayContent.kt:96`) plus hardcoded labels like symbol categories (`SymbolData.swift:14`) and the shared FAQ/features content (`content/*.json`, currently `hanji`-only). All of these are in scope and cross a process/lifecycle boundary (extension ≠ host app) — see live-switch risk below.

USER constraints (2026-06-19):

1. **Follow i18n best practices.**
2. **Two-platform wording must be consistent**; where they diverge today, normalize text to one canonical form (USER pre-authorized text edits).

---

## Current state (grounded in code)

Both platforms use **hand-mirrored, code-constant** localization — no i18n infrastructure, single language (台語漢字).

| | iOS | Android |
|---|---|---|
| Mechanism | 6 enums `static let` — `ios/Sources/TaigiKeyboard/Strings/*Texts.swift` (~220) | 6 `object` `const val` — `android/app/src/main/java/com/siansiansu/taigikeyboard/localization/*Texts.kt` (~770 lines) |
| Languages | 台語漢字 only | 台語漢字 only |
| Sync | **Manual mirror** (`SettingsTexts.kt:6` comments "對應 iOS SettingsTexts.swift") | same |
| Reference | direct `SettingsTexts.foo` | direct `SettingsTexts.foo` (54 sites) + `getString(R.string.*)` for IME key labels |
| Format args | **≥5** format constants, not 2 — e.g. `DictionaryTexts.kt:49`, `:119` (import results + counts); `DictionaryTexts.swift:49`, `:126` | same |
| Platform-specific strings | — | legitimate Android-only keys exist (`SettingsTexts.kt:70` `resetFailed`/`noEmailApp`) |
| Locale detection | none for UI text | none for UI text |
| Extension reuse | `*Texts` used by extension overlays (`SettingsSelectionOverlay.swift:76`) | same (`SettingsOverlayContent.kt:96`) |
| FAQ/features | shared `content/*.json`, `{"hanji": "..."}` only | same (symlink) |

Two facts shape the design:

- **Already hand-mirrored** → a single source of truth eliminates drift AND satisfies USER constraint 2 by construction.
- **iOS deliberately removed its localization indirection** — "iOS Localization refactor" Stage 7 (commit `b5ec70e2`) deleted the `LocalizedText` wrapper + `LanguageManager` singleton (every view held a `@StateObject` — boilerplate). Multi-language needs reactive locale state again, but **must NOT resurrect the singleton/wrapper form** (see Decision 7).

`content/` is already an **in-repo centralized data source** symlinked into both platforms — direct precedent.

---

## Design (revised per Codex co-review)

### 1. Single source of truth — in-repo `i18n/` directory  *(Codex: CONFIRM)*

`i18n/` holds semantic keys → per-`DisplayLanguage` values, namespaced (`settings`, `home`, `dictionary`, `theme`, `layout`, `common`, `extension`, `content`). NOT a separate submodule repo (rationale + flip trigger below).

**Source format (locked, USER 2026-06-19): JSON, ARB-shaped.** One file per namespace (`i18n/settings.json`, …), one entry per key with metadata + per-language values:

```jsonc
// i18n/settings.json
{
  "settings.inputMode.title": {
    "comment": "Settings row label — keyboard input-mode picker",   // translator context
    "scope": "shared-host",                       // shared/ios/android × host/extension (D5)
    "values": {
      "hanji": "輸入模式",
      "tailo": "...",                             // human-authored
      "poj": "...",                               // human-authored alongside tailo (POJ rendering)
      "ja": "入力モード",
      "en": "Input Mode"
    }
  },
  "dictionary.importResult": {
    "comment": "Toast after CSV import",
    "scope": "shared-host",
    "placeholders": { "imported": "int", "duplicates": "int" },     // named, not %d (D6)
    "values": {
      "hanji": "匯入 {imported} 項成功，{duplicates} 項重複",
      "en": "Imported {imported}, {duplicates} duplicates"
    }
  }
}
```

Rationale: JSON (ARB-shaped) is the i18n best-practice choice — machine-safe, TMS-ready (Crowdin/Weblate ingest ARB/XLIFF; JSON↔ARB↔XLIFF convert cleanly), native placeholder/plural metadata, matches repo precedents (`content/*.json`, `taigi-emojis/dist/emoji.json`). Named placeholders `{imported}` (not positional `%d`); plurals via key-suffix entries (`x.count.one` / `x.count.other`) → codegen to native plurals; `poj` is hand-authored alongside `tailo` — see Decision 4 for the history (auto-derive schema, then derive-and-store, now hand-authored).

⚠ Do not make the symlink the only build integration — add a **generated-output freshness check** (archive builds, non-symlink checkouts, Gradle/Xcode packaging must all see current output).

### 2. Hybrid: canonical JSON → codegen NATIVE resources  *(Codex: REFUTE "no platform-native"; corrected)*

"Custom language *selection*" does NOT require a "custom string *runtime*." Original plan over-corrected. Revised:

- `i18n/` JSON stays the canonical source.
- Build **codegen emits platform resources and typed maps**: iOS String Catalog `.xcstrings` for `en` / `ja` plus Swift maps for Hanji / TL / POJ; Android native resources plus generated maps for unsupported product languages.
- **`en` / `ja` map to App-Store-supported iOS locales** → native resource resolution, accessibility, resource lint, and tooling work without invalid archive locale directories.
- **Hanji / TL / POJ product identities are not safe Apple localization-directory identifiers** → resolve them through an app-level `DisplayLanguage` enum and generated Swift maps. App Store Connect rejects the former `nan-Hant-TW` / private-use `.lproj` directories.
- Android picker uses **`AppCompatDelegate.setApplicationLocales()`** (official in-app-language API) for the OS-locale-backed languages — triggers config change + Compose recomposition + native resolution + syncs the Android 13 system per-app language.

Net: the app leans on native resource machinery for 3 of 5 languages; the only custom bit is selecting the TL/POJ orthography resource set.

### 3. `DisplayLanguage` enum is the domain identity; BCP-47 is metadata only  *(Codex: CONFIRM syntax, REFUTE as identity)*

- Persistence key, branch logic, resource selection → **`DisplayLanguage` enum** (`hanji`/`tailo`/`poj`/`japanese`/`english`/`system`).
- BCP-47 is formatting/platform metadata only, never the identity or persisted key. iOS native bundle resolution uses only `ja` and `en`; Android may use `nan-Hant-TW` as a native qualifier where supported. Hanji / TL / POJ still persist as `hanji` / `tailo` / `poj`.
- `system` (Automatic) is a first-class tri-state value with its own persistence.

### 4. POJ = default-derived from TL + explicit override  *(Codex: REFUTE whole-column auto-derive)*

`taigi-converter` converts Latin runs, keeps non-matching text verbatim (`converter.js:9`, `:39`; tests cover mixed Hanji/case/punctuation `converter.test.js:93`) — but it has **no semantics**: brand names / URLs / filenames / abbreviations matching a TL syllable get rewritten; placeholders/markup need protection; some dialectal finals have no standard POJ (reference itself carries `POJ = —`). So:

- ~~Schema: default `poj = derive(tl)`, plus `pojOverride` and `derivePoj: false`, plus protected-span / placeholder handling.~~ **SUPERSEDED** — see the Implemented note below (no override schema; the audit found zero corruption).
- **Generated-diff human review** — "build succeeded" ≠ "language correct". (Still applies — see below.)

**Implemented (P3c R6-1) — derive-and-store, override schema dropped.** POJ was first derived from `tailo` by `tools/i18n/derive_poj.py` (strict `convert_tl_to_poj` bridge) and **stored** as the `poj` value in `i18n/*.json`, exactly like every other language; the codegen reads the stored value (no Node at `make i18n` time). An empirical audit of all 211 strings (Codex-confirmed) showed the converter preserves every brand / acronym / `{placeholder}` token verbatim — zero corruption — so the `pojOverride` / `derivePoj:false` / protected-span schema was **not** built (YAGNI). tailo↔poj are kept in step by `validate_production_completeness` (both are production languages).

**Superseded — POJ is now hand-authored (no derive tooling).** The `derive_poj.py` / `derive_content_poj.py` derive scripts and their `make i18n-derive-poj` / `content-derive-poj` / `poj-check` targets were removed. POJ is the Pe̍h-ōe-jī rendering of the same reading as TL — a mechanical correspondence the maintainer authors and proofreads by hand, alongside `tailo`, like every other language. This drops the Node dependency from the authoring loop. The codegen path is unchanged (it always read the stored `poj` value); `validate_production_completeness` requires the pair to be authored together (both are production languages, so the separate lockstep gate was dropped). After any `tailo` correction, update `poj` by hand in lockstep.

### 5. Codegen + scope-aware key checks  *(Codex: CONFIRM codegen, REFUTE hard iOS==Android equality)*

- Codegen typed accessors (`L10n.foo` / `Strings.foo`) — ~200+ strings is not over-engineering.
- **Scope-aware schema** (NOT a hard 3-way equal key set, which would force fake strings or block platform UX): `shared-host` / `ios-host` / `android-host` / `shared-extension` / `ios-extension` / `android-extension`. Build check = "every *referenced* key exists and matches its scope," not "both platforms reference identical sets." Preserves legitimate platform-only keys (`resetFailed`, `noEmailApp`).

### 6. No ICU runtime; native plural via codegen typed functions  *(Codex: REFUTE "only 2 %d", CONFIRM no ICU dep)*

- There are **≥5** format constants today, and adding English likely needs plurals (`1 item` / `2 items`).
- No cross-platform ICU dependency. Codegen emits **typed functions** (`importResult(imported:duplicates:)`), backed by Apple String Catalog plural/substitution + Android `plurals`/resources. Forbid call sites passing raw `%d`.

### 7. Reactive locale state (NOT singleton/wrapper)  *(Codex: REFUTE singleton, CONFIRM reactive state)* — **biggest gap**

A static `L10n.foo` getter does NOT tell SwiftUI/Compose to refresh → switching language could leave a screen mixing old/new text until a view rebuild / Activity restart / extension restart.

- iOS: app-root **observable locale state** + a hybrid resolver: native bundles for English/Japanese and generated Swift maps for Hanji/TL/POJ. ⚠ `.environment(\.locale,…)` does NOT switch string tables (formatting only) — see *Verified platform mechanisms* below. Do NOT re-add per-view `@StateObject` wrappers (the Stage-7 boilerplate).
- Android: ⚠ NOT `setApplicationLocales()` for TL/POJ (it strips `-x-` private-use subtags). Use the florisboard model — `DisplayLanguage` enum in DataStore → `createConfigurationContext` Context held as Compose state → `LocalResourcesContext` + custom `stringRes()`. See *Verified platform mechanisms* below.
- Keyboard extension: iOS = **separate process** (App Group `UserDefaults` channel); Android IME = **same process** (reuse the existing `onCreateInputView()` rebuild). Each needs its own update contract.
- **Gate**: a root-to-leaf reactive prototype proving live-switch for host + extension — pulled into its own **R1 spike, Android-first, BEFORE P1** (see *Execution rollout*).

---

## Best-practices alignment

| i18n best practice | This plan |
|---|---|
| Externalize strings; no hardcoded text | central `i18n/`; codegen keys only |
| Semantic keys | `settings.inputMode.title`, namespaced + scoped |
| Use native localization machinery where possible | iOS `.xcstrings` for `en` / `ja`; Android resources where supported; generated maps otherwise |
| Domain identity ≠ locale string | `DisplayLanguage` enum; BCP-47 = metadata only |
| Defined fallback; completeness fails build | production missing key → build fail; `→ 漢字` only as anti-crash last resort |
| Native plural/format | String Catalog / Android plurals via codegen typed fns |
| Reactive, not boilerplate | env-injected observable state; no per-view wrapper |
| Single source of truth | one `i18n/` dir |

### Deliberately NOT adopted (with reasoning)

- **Custom JSON string runtime** (the original plan's mistake) — replaced by codegen → native resources. Custom selection ≠ custom runtime; native machinery keeps plural/format/a11y/lint.
- **Separate i18n repo** (taigi-emojis model) — solo maintainer, external-translator isolation is speculative (YAGNI); UI keys are screen-coupled → renames need same-commit atomicity a submodule can't give; emojis are a reusable dataset, UI strings are not. **Flip trigger**: external translation workflow (Crowdin/Weblate) / cross-product reuse → `git filter-repo` extract. Stay extraction-ready; no submodule tax now.
- **ICU MessageFormat runtime lib** — use native plural instead (above).
- **Singleton `LocalizedText`/`LanguageManager`** — use reactive root state (Decision 7).
- **TSV / CSV source format** — considered for spreadsheet manageability; rejected as canonical (USER 2026-06-19 chose JSON on best-practice grounds). Not a standard i18n format (no TMS ingests it), structure-poor (placeholders/plurals need ad-hoc conventions), round-trip-unsafe through spreadsheets. If spreadsheet editing is later wanted, generate a TSV/Sheet *view* from the JSON and import back — JSON stays the committed source.
- **RTL rollout** — none of the 5 languages need it; but shared components must avoid new hardcoded leading/trailing assumptions.

---

## Phases (work ordering + cost — NOT a version assignment)  *(Codex: REFUTE old P2/P3 boundary)*

Old P2 (show 5 picker options all falling back to 漢字) was a visible fake feature. Revised so the picker only ever shows languages that fully pass their gate, and English proves the live-switch end-to-end first.

| Phase | Content | Behavior | Value |
|---|---|---|---|
| **P0** | Wording reconciliation (iOS↔Android diff → one canonical form, USER-authorized edits) + full string **inventory & scope classification** (host/extension, shared/platform-only, FAQ content) | behavior-preserving | kills drift; scopes the work |
| **P1** | Canonical `i18n/` schema + **native-resource codegen** + typed accessors + scope-aware key/completeness checks + **live-switch reactive prototype** (host + extension) | behavior-preserving | single source of truth; mirror dies; proves the hard part |
| **P2** | Locale **state + persistence + live-switch**, done as a complete vertical slice in **English** (incl. native plural, dynamic-type/long-string layout, a11y locale) | adds picker (English only) | first real switchable language, fully gated |
| **P3a** | Japanese (font verified: global Open Huninn keeps full kana coverage — Hiragana/Katakana/halfwidth — only kanji render in Taiwan rounded-gothic forms; kept for a consistent app aesthetic, no per-`ja` font swap) | adds language | — |
| **P3b** | TL authoring (Taigi prose; Core Principle #3 authoritative-source-only; never invent TL) | adds language | — |
| **P3c** | POJ derive-and-store from tailo (`derive_poj.py`, lockstep-gated) + human diff review — derive tooling later removed, POJ now hand-authored (see Decision 4) | adds language | — |

Picker shows a language only after it passes completeness + layout + accessibility gates. P1 alone delivers value (single source of truth) independent of multi-language shipping.

**Promotion status**: P3a (ja, R4-2), P3b (tailo, R5-2), and P3c (poj, R6-2) are all promoted into the production picker — release roster = `[hanji, english, japanese, tailo, poj]` (+ `system`). TL/POJ prose stays review-pending: USER proofreads + re-authors on his own schedule, a data-only edit (`i18n/*.json` tailo + poj in lockstep), not a code or roster gate. Round-by-round status lives in memory; the per-round table below stops at the early P1 rounds and is not the live tracker.

### Delivery status

The canonical JSON sources, native-resource codegen, typed accessors, completeness checks, display-language persistence, live switching, and all five production languages are implemented on both platforms. Current production roster and fallback contracts live in `docs/architecture/behavioral-invariants.md` §37; this plan no longer tracks per-round delivery history.

---

## Design gaps to resolve (from Codex)

- **Automatic mode** = tri-state with persistence; not a one-time OS-locale seed.
- **Keyboard-extension scope** — `*Texts` is used by extension overlays; in scope, crosses process boundary.
- **FAQ/features content** (`content/*.json`) — host-app UI text, currently `hanji`-only; must enter a phase.
- **Fallback policy** — production completeness should fail build; `→ 漢字` is anti-crash only, else mixed-language UI.
- **Accessibility / TTS** — each language needs a speech locale; verify VoiceOver/TalkBack support for `nan-*` on device.
- **Japanese font** — RESOLVED (P3a R4-2): Open Huninn cmap has full kana coverage (Hiragana 88/96, Katakana 93/96, halfwidth 59/59; derived from Kosugi Maru). Kanji render in Taiwan rounded-gothic forms — accepted for a consistent app aesthetic; no per-`ja` font swap. The forced-Open-Huninn global modifier stays.
- **Dynamic Type / long strings** — English is longer; truncation + screenshot tests on tab titles, settings rows, dialogs, overlays.
- **Number/date/plural formatting** — locale policy, not just template translation.
- **App metadata** — InfoPlist, Android app name, permission/setup copy, App Store / Play listing = separate localization scope.
- **Reset/migration semantics** — does "reset settings" clear the display-language override? Define.
- **OS negotiation** — primary locale vs first-supported in the preferred-language list. Define.

---

## Risks

- **Live-switch invalidation has no architectural definition** — the #1 risk. Static `L10n.foo` won't trigger SwiftUI/Compose refresh; extension is a separate process. Mitigation = the P1 reactive prototype gate (Decision 7).
- **TL authoring is a real linguistic task** — never invent TL (Core Principle #3 + taigi-emojis lesson); the long pole, not the engineering.
- ~~**POJ auto-derive can corrupt non-Taigi tokens**~~ — RESOLVED (P3c R6-1): an audit of all 211 strings (Codex-confirmed) found zero corruption (the converter preserves every brand / acronym / `{placeholder}` token), so no override schema was needed (Decision 4 Implemented note). Diff review still applies.
- **`HomeTexts` version-history changelog EXCLUDED from multi-language** (USER 2026-06-19: changelog 不需要納入多語) — stays 漢字-only; do not codegen/translate those keys.
- **Repo location CONFIRMED in-repo** (USER 2026-06-19). Separate-repo flip trigger (external-translator workflow) still documented in Decision 1.
- **Version / timing** — tentatively v3.6.4 (USER said 「可能」, not firm); remains USER-gated.

---

## Codex co-review receipts (2026-06-19, ANALYSIS-ONLY)

Verdicts folded in above: D1 CONFIRM / D2 REFUTE (→ hybrid native resources) / D3 CONFIRM-syntax, REFUTE-as-identity (→ `DisplayLanguage` enum + Automatic) / D4 REFUTE (→ default-derive + override) / D5 CONFIRM codegen, REFUTE hard equality (→ scope-aware) / D6 REFUTE "2 %d", CONFIRM no-ICU (→ native plural via codegen fns) / D7 REFUTE singleton (→ reactive root state) / D8 REFUTE P2/P3 boundary (→ English vertical slice first, gated picker). Biggest risk = live-switch invalidation.

---

## References

- Current strings: `ios/Sources/TaigiKeyboard/Strings/*Texts.swift`, `android/.../localization/*Texts.kt`
- Extension reuse: `SettingsSelectionOverlay.swift:76`, `SettingsOverlayContent.kt:96`, `SymbolData.swift:14`
- In-repo shared-data precedent: `content/tab1-features.json`
- Separate-repo data precedent: `taigi-emojis` submodule
- TL→POJ converter: `taigi-converter/` (`src/converter.js:9`, `:39`)
- Android in-app language: `AppCompatDelegate.setApplicationLocales()` (developer.android.com app-languages)
- Process: `~/.claude/rules/planning.md`, `.claude/rules/cross-platform-alignment.md`

---

## Verified platform mechanisms (grounded in authoritative docs, 2026-06-20)

> Verified via `find-docs` / `ctx7` + Apple & Android official docs + local KeyboardKit 9.9.0 source + `references/florisboard` / `references/azooKey`, per `.claude/rules/doc-lookup.md`. **Supersedes any earlier API speculation in Decisions 2 & 7.** Method = 3 parallel research agents (iOS API / Android API / reference-IME + codegen grounding).

### iOS (Swift / SwiftUI / KeyboardKit 9.9, iOS 26.1)

- ⚠ **CRITICAL CORRECTION (D7)**: `.environment(\.locale, Locale(identifier:))` does **NOT** switch which string table `Text("key")` reads — it drives **formatting only** (number / date / measurement / collation). String-table selection is a separate path. Source: Apple `EnvironmentValues.locale` + `Bundle.localizedString(forKey:value:table:)` (takes no `Locale`). Relying on environment-locale for UI language is the #1 trap.
- **String-table selection is hybrid**: English/Japanese resolve from compiled `.lproj` bundles; Hanji/TL/POJ resolve from `GeneratedTaigiStrings.swift`. This keeps all five product languages while preventing unsupported locale directory names from entering the archive.
- **`.xcstrings` String Catalog** remains the canonical iOS resource for App-Store-supported locales only (`en`, `ja`) and is included in both targets.
- **`CFBundleLocalizations`** declares only `en` and `ja`. Product-language identity stays in `DisplayLanguage`; it is not advertised as an OS locale.
- **App Store correction (2026-08-07):** the previous private-use `.lproj` design compiled successfully but App Store Connect rejected the resulting `nan-Hant-TW`, `nan-Latn-TW-x-tailo`, and `nan-Latn-TW-x-poj` archive directories as unrecognized locales. Generated maps supersede that design.
- **Live switch** = root `@Observable` language store → computed resolver; publishing a change re-renders the tree, no restart. Set `\.locale` additionally for number/date formatting.
- **Extension** (separate process from host) follows the host via **App Group shared `UserDefaults(suiteName:)`** and mirrors the same hybrid resolver; it sets `KeyboardContext.locale` for KeyboardKit's own keycap labels. Add the `.xcstrings` to BOTH targets' Resources phases (each target compiles its own copy).

### Android (Kotlin / Compose / FlorisBoard base; minSdk 28, AppCompat 1.7.1, Compose BOM 2026.01.01)

- ⚠ **`AppCompatDelegate.setApplicationLocales()` is the WRONG tool for TL/POJ**: `LocaleList` normalization strips `-x-` private-use subtags, and the system per-app-language picker only surfaces real OS locales. Verified limitation.
- **Resource qualifier dirs cannot encode `-x-` private-use** → cannot split TL vs POJ as two `values-b+nan+Latn+TW+x+…` dirs.
- **VERIFIED model (florisboard, the app's base)**: identity = app-level `DisplayLanguage` enum in **DataStore** (NOT `setApplicationLocales`) → build a localized Context via `createConfigurationContext(Configuration().apply { setLocale(...) })` held as Compose `mutableStateOf` → expose through a `LocalResourcesContext` CompositionLocal + a custom `stringRes()` reading `LocalResourcesContext.current.resources.getString(id)`. Gives **live switch with zero Activity/IME `recreate()`**. Source: `references/florisboard/.../Resources.kt:40-88`, `FlorisAppActivity.kt:99-104`, `OtherScreen.kt:85-148`.
- **3 real locales** (en/ja/zh-Hant) → native `values/`, `values-ja/`, `values-b+zh+Hant/` (rename legacy `values-zh-rTW/`; `b+` qualifiers are API 24+). **TL/POJ** → an **enum-selected generated string set read by the custom resolver** (not a `values-*` dir, since both map to `nan-Latn-TW` and qualifiers can't split them). This is exactly D2's hybrid.
- **IME runs in the SAME app process** (not a separate process like the iOS extension — verified `TaigiKeyboard : LifecycleInputMethodService`, no `android:process`). Reuse the existing `onConfigurationChanged` → `onCreateInputView()` / `setInputView()` rebuild hook (`android/.../ime/core/TaigiKeyboard.kt:316`) + collect the same DataStore Flow → recompose the input view. No service restart.
- **No pbxproj-equivalent blocker** — `build.gradle.kts`, `res/`, and the manifest are all Claude-editable.

### Cross-platform conclusion

D2 (hybrid) and D7 (reactive root state) **CONFIRMED + sharpened**: supported locales lean on native resources; other product languages resolve through generated maps. On iOS, only `en` / `ja` are emitted as localization bundles because App Store Connect rejects the former Taiwanese product-language directory identifiers. Android continues to use its platform-appropriate resource/map split.

### Codegen (Python, mirrors `dictionary/` convention)

- Tool under `tools/i18n/`; new **`make i18n` target** (mirrors `make dict` — explicit committed output, NOT a per-compile Xcode/Gradle phase, so archive / non-symlink builds stay fresh). POJ is hand-authored alongside tailo (D4 — the early auto-derive design was later dropped), so the codegen has no Node dependency.
- Emits: iOS `Localizable.xcstrings` (`en` / `ja`) + `GeneratedTaigiStrings.swift` (Hanji / TL / POJ) + typed accessors (into a synced group → Swift auto-includes, no pbxproj edit); Android `values-*/strings.xml` + a Kotlin accessor object + generated string maps. `content/*.json` carries the same authored language roster.

### USER project-config hand-offs (iOS only — Claude cannot edit pbxproj)

1. Add generated `Localizable.xcstrings` to **both** the host app and Keyboard extension Resources build phases (String Catalogs are resources, NOT covered by synced-group Swift auto-include).
2. Keep `CFBundleLocalizations` limited to `en` and `ja` in both `Info.plist` files.
3. Remove the obsolete `nan-*` entries from Xcode `knownRegions` manually; they do not drive runtime resolution after the generated-map migration.

Android has **no** equivalent gate.

---

## Execution rollout (risk-first sequencing, USER-gated)

Reorders the Phase table for de-risking: the D7 live-switch prototype (the #1 risk) runs as a throwaway spike **before** the P1 infra investment; Android leads because it has no pbxproj gate and its IME (same-process) + host both prove the cross-surface live-switch with full Claude control. iOS follows once USER does the project-config hand-offs.

| Round | Content | Platform | PR boundary | Gate |
|---|---|---|---|---|
| **R0** (P0 finish) | 2 wording reconciles (`HomeTexts.kt:15` drop `Android ` → `台語齒盤`; iOS `HomeTexts.swift:288` drop `「」` → `建中整理、提供`); optionally commit the Tier-1 report | iOS + Android | branch + PR, <10 LOC | visual dogfood; no test; Codex sandwich skippable (trivial value swap) |
| **R1** live-switch spike | Throwaway: root `DisplayLanguage` DataStore state → host (Compose) + IME (same-process) recompose via `LocalResourcesContext` / `createConfigurationContext`, incl. a TL/POJ enum-selected set. 2 strings × 2 langs, hardcoded | **Android first** | scratch / draft PR (not a feature) | **architecture go/no-go** — proves the unprecedented TL/POJ + live-switch part; pass → R2, fail → rethink D2/D7 before any infra |
| **R1′** iOS spike | Same minimal prototype: `@Observable` store → per-bundle `.lproj` override + `Text(key, bundle:)`; extension reads App Group `UserDefaults` | iOS | scratch | ⛔ BLOCKED on USER pbxproj (`CFBundleLocalizations` + TL/POJ `.lproj`) |
| **R2** (P1 infra) | `i18n/` JSON schema (1-2 namespaces first: `common` + `settings`) + Python codegen + `make i18n` + native resources + typed accessors + scope-aware key check + freshness check; hand-mirror dies incrementally | both (iOS resources USER-gated) | split R2a (codegen + schema + Android) / R2b (iOS); ~300-500 LOC each | spike passed |
| **R3** (P2) | locale state + persistence + picker (English only) + live-switch wired for real + native plural + dynamic-type / long-string layout + a11y locale | both | vertical slice | completeness + layout + a11y |
| **R4a/b/c** (P3) | ja (font verify) / TL authoring (Core Principle #3 — never invent TL) / POJ (derive-and-store from tailo + diff review — derive tooling later removed, POJ now hand-authored, see Decision 4). `content/*.json` per-language authoring rides here | both | one language per round | per-language gate before entering picker |

Only change from the Phase table above: the D7 prototype is pulled out of P1 into its own R1 spike, run **before** P1 infra, Android-first.
