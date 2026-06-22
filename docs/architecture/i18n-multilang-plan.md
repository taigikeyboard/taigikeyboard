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
      "poj": "...",                               // derived-and-stored from tailo (`make i18n-derive-poj`)
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

Rationale: JSON (ARB-shaped) is the i18n best-practice choice — machine-safe, TMS-ready (Crowdin/Weblate ingest ARB/XLIFF; JSON↔ARB↔XLIFF convert cleanly), native placeholder/plural metadata, matches repo precedents (`content/*.json`, `taigi-emojis/dist/emoji.json`). Named placeholders `{imported}` (not positional `%d`); plurals via key-suffix entries (`x.count.one` / `x.count.other`) → codegen to native plurals; `poj` is derived-and-stored from `tailo` (`make i18n-derive-poj`) — see Decision 4 for the superseded blank/override/skip schema.

⚠ Do not make the symlink the only build integration — add a **generated-output freshness check** (archive builds, non-symlink checkouts, Gradle/Xcode packaging must all see current output).

### 2. Hybrid: canonical JSON → codegen NATIVE resources  *(Codex: REFUTE "no platform-native"; corrected)*

"Custom language *selection*" does NOT require a "custom string *runtime*." Original plan over-corrected. Revised:

- `i18n/` JSON stays the canonical source.
- Build **codegen emits native resources**: iOS String Catalog `.xcstrings` (per locale) + Android `values-<locale>/strings.xml`.
- **`en` / `ja` / 漢字 (zh-Hant) map to real OS locales** → native resource resolution, native plural/format, accessibility, resource lint, tooling all work.
- **TL / POJ have no OS locale** → resolved via an app-level `DisplayLanguage` enum selecting an explicit resource set: Android custom BCP-47 qualifier dir (`values-b+nan+Latn+TW+...`) forced via a Configuration context; iOS a declared non-standard `CFBundleLocalizations` `.lproj` (+ a Bundle-override if needed). This pair is the **implementation spike**, tied to the live-switch prototype (Decision 7).
- Android picker uses **`AppCompatDelegate.setApplicationLocales()`** (official in-app-language API) for the OS-locale-backed languages — triggers config change + Compose recomposition + native resolution + syncs the Android 13 system per-app language.

Net: the app leans on native resource machinery for 3 of 5 languages; the only custom bit is selecting the TL/POJ orthography resource set.

### 3. `DisplayLanguage` enum is the domain identity; BCP-47 is metadata only  *(Codex: CONFIRM syntax, REFUTE as identity)*

- Persistence key, branch logic, resource selection → **`DisplayLanguage` enum** (`hanji`/`tailo`/`poj`/`japanese`/`english`/`system`).
- BCP-47 (`nan-Hant-TW`, `nan-Latn-TW-x-tailo`, `nan-Latn-TW-x-poj`, `ja`, `en`) used ONLY as the formatting/metadata locale + resource-dir qualifier. Per RFC 5646 §2.2.7 private-use subtags are not understood by external systems — never the identity/persisted key.
- `system` (Automatic) is a first-class tri-state value with its own persistence.

### 4. POJ = default-derived from TL + explicit override  *(Codex: REFUTE whole-column auto-derive)*

`taigi-converter` converts Latin runs, keeps non-matching text verbatim (`converter.js:9`, `:39`; tests cover mixed Hanji/case/punctuation `converter.test.js:93`) — but it has **no semantics**: brand names / URLs / filenames / abbreviations matching a TL syllable get rewritten; placeholders/markup need protection; some dialectal finals have no standard POJ (reference itself carries `POJ = —`). So:

- ~~Schema: default `poj = derive(tl)`, plus `pojOverride` and `derivePoj: false`, plus protected-span / placeholder handling.~~ **SUPERSEDED** — see the Implemented note below (no override schema; the audit found zero corruption).
- **Generated-diff human review** — "build succeeded" ≠ "language correct". (Still applies — see below.)

**Implemented (P3c R6-1) — derive-and-store, override schema dropped.** POJ is derived from `tailo` by `tools/i18n/derive_poj.py` (`make i18n-derive-poj`, strict `convert_tl_to_poj` bridge) and **stored** as the `poj` value in `i18n/*.json`, exactly like every other language; the codegen reads it (no Node at `make i18n` / `make i18n-check` time — a graceful bridge inside the freshness gate would be a false-green hazard). An empirical audit of all 211 strings (Codex-confirmed) showed the converter preserves every brand / acronym / `{placeholder}` token verbatim — zero corruption — so the `pojOverride` / `derivePoj:false` / protected-span schema was **not** built (YAGNI; add a minimal override only if a real exception surfaces in review). tailo↔poj lockstep is enforced by `validate_generated_map_completeness`. Re-run the derive after any `tailo` correction; the generated-diff human review still applies (POJ correctness = TL correctness × the canonical converter).

### 5. Codegen + scope-aware key checks  *(Codex: CONFIRM codegen, REFUTE hard iOS==Android equality)*

- Codegen typed accessors (`L10n.foo` / `Strings.foo`) — ~200+ strings is not over-engineering.
- **Scope-aware schema** (NOT a hard 3-way equal key set, which would force fake strings or block platform UX): `shared-host` / `ios-host` / `android-host` / `shared-extension` / `ios-extension` / `android-extension`. Build check = "every *referenced* key exists and matches its scope," not "both platforms reference identical sets." Preserves legitimate platform-only keys (`resetFailed`, `noEmailApp`).

### 6. No ICU runtime; native plural via codegen typed functions  *(Codex: REFUTE "only 2 %d", CONFIRM no ICU dep)*

- There are **≥5** format constants today, and adding English likely needs plurals (`1 item` / `2 items`).
- No cross-platform ICU dependency. Codegen emits **typed functions** (`importResult(imported:duplicates:)`), backed by Apple String Catalog plural/substitution + Android `plurals`/resources. Forbid call sites passing raw `%d`.

### 7. Reactive locale state (NOT singleton/wrapper)  *(Codex: REFUTE singleton, CONFIRM reactive state)* — **biggest gap**

A static `L10n.foo` getter does NOT tell SwiftUI/Compose to refresh → switching language could leave a screen mixing old/new text until a view rebuild / Activity restart / extension restart.

- iOS: app-root **observable locale state** + a **per-bundle `.lproj` override** (`Text(key, bundle:)`). ⚠ `.environment(\.locale,…)` does NOT switch string tables (formatting only) — see *Verified platform mechanisms* below. Do NOT re-add per-view `@StateObject` wrappers (the Stage-7 boilerplate).
- Android: ⚠ NOT `setApplicationLocales()` for TL/POJ (it strips `-x-` private-use subtags). Use the florisboard model — `DisplayLanguage` enum in DataStore → `createConfigurationContext` Context held as Compose state → `LocalResourcesContext` + custom `stringRes()`. See *Verified platform mechanisms* below.
- Keyboard extension: iOS = **separate process** (App Group `UserDefaults` channel); Android IME = **same process** (reuse the existing `onCreateInputView()` rebuild). Each needs its own update contract.
- **Gate**: a root-to-leaf reactive prototype proving live-switch for host + extension — pulled into its own **R1 spike, Android-first, BEFORE P1** (see *Execution rollout*).

---

## Best-practices alignment

| i18n best practice | This plan |
|---|---|
| Externalize strings; no hardcoded text | central `i18n/`; codegen keys only |
| Semantic keys | `settings.inputMode.title`, namespaced + scoped |
| Use native localization machinery where possible | codegen → `.xcstrings` / `values-*/strings.xml`; `setApplicationLocales()` |
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
| **P1** | Canonical `i18n/` schema + **native-resource codegen** + typed accessors + scope-aware key/completeness checks + **pseudo-locale** + **live-switch reactive prototype** (host + extension) | behavior-preserving | single source of truth; mirror dies; proves the hard part |
| **P2** | Locale **state + persistence + live-switch**, done as a complete vertical slice in **English** (incl. native plural, dynamic-type/long-string layout, a11y locale) | adds picker (English only) | first real switchable language, fully gated |
| **P3a** | Japanese (font verified: global Open Huninn keeps full kana coverage — Hiragana/Katakana/halfwidth — only kanji render in Taiwan rounded-gothic forms; kept for a consistent app aesthetic, no per-`ja` font swap) | adds language | — |
| **P3b** | TL authoring (Taigi prose; Core Principle #3 authoritative-source-only; never invent TL) | adds language | — |
| **P3c** | POJ derive-and-store from tailo (`derive_poj.py`, lockstep-gated) + human diff review | adds language | — |

Picker shows a language only after it passes completeness + layout + accessibility gates. P1 alone delivers value (single source of truth) independent of multi-language shipping.

### Delivery status (rounds)

P0 and P1 are delivered as small per-PR rounds (Codex pre-impl 2026-06-20 refuted a single large P1 PR; namespace-atomic split avoids a JSON-vs-`*Texts` dual-source-of-truth drift window):

| Round | Scope | Status |
|---|---|---|
| **R0** | Wording reconcile (`台語齒盤` / `建中整理、提供`) | Merged — PR #447 |
| **R1** | Android live-switch architecture spike (throwaway) | Gate pass — PR #448 closed (D2 hybrid + D7 confirmed on device) |
| **R2a-1** | Codegen pipeline (`tools/i18n/`, `make i18n`, Gradle freshness gate) + Android resolution infra (`DisplayLanguage` / `StringResolution` / `StringResolver` / `ProvideDisplayLanguage`) + debug probe fixture (`i18n/probe.json`, no real namespace data) | **PR #449 — Android gate pass, dogfood pending** |
| **R2a-2** | `i18n/common.json` + migrate all `CommonTexts.*` call sites + delete `CommonTexts.kt` (same PR) | Not started |
| **R2a-3** | `i18n/settings.json` + migrate all `SettingsTexts.*` call sites + delete `SettingsTexts.kt` (keep `confirmKeyLabel`) | Not started |
| **R2b** | iOS infra (`.xcstrings` + per-`.lproj`) | Blocked — needs USER pbxproj edits (`CFBundleLocalizations`, `knownRegions`, Resources phase) |

Deferred to P2 (recorded during R2a-1 review): emit the pseudo-locale map under a `src/debug/` source set (it is dead bytecode in release at full keyset); add a "superseded by `i18n/`" pointer on the legacy `*Texts` objects. P3c POJ-derive reuses `dictionary/common/taigi_bridge.py::convert_tl_to_poj` (do not reinvent).

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
- **String-table selection = per-bundle `.lproj` override**: resolve a per-language `Bundle(path: Bundle.main.path(forResource: <full-identifier>, ofType: "lproj"))` and read every string via `Text(key, bundle: chosenBundle)` / `NSLocalizedString(_, bundle:)`. Works for private-use tags (lookup is by raw identifier string, no OS-locale validation). This is the **same pattern KeyboardKit itself uses** (`references/keyboardkit9.9.0/.../Bundle+Locale.swift:23-27`). Preferred over the `AppleLanguages` UserDefaults + `Bundle` swizzle approach (no swizzling).
- **`.xcstrings` String Catalog** is the source resource (compiles to per-locale `.lproj` at build; runtime lookup identical to legacy). Holds arbitrary locale keys incl. private-use. azooKey confirms it works in an iOS IME (`Localizable.xcstrings`, dual-target Resources membership).
- **`CFBundleLocalizations`** (Info.plist) is **mandatory** to make `nan-Latn-TW-x-tailo` / `nan-Latn-TW-x-poj` resolvable; `knownRegions` must also be extended. **USER-only edit** (Core Principle #1).
- Name each `.lproj` by the **full identifier** (`nan-Latn-TW-x-tailo.lproj`) — KeyboardKit/Foundation only does identifier→languageCode fallback, will not synthesize the private-use part.
- **Live switch** = root `@Observable` language store → computed per-language `Bundle`; publishing a change re-renders the tree, no restart. Set `\.locale` additionally for number/date formatting.
- **Extension** (separate process from host) follows the host via **App Group shared `UserDefaults(suiteName:)`**; mirrors the same bundle resolver; sets `KeyboardContext.locale` for KeyboardKit's own keycap labels. Add the `.xcstrings` to BOTH targets' Resources phases (each target compiles its own copy — azooKey pattern; no App Group needed for the strings themselves).

### Android (Kotlin / Compose / FlorisBoard base; minSdk 28, AppCompat 1.7.1, Compose BOM 2026.01.01)

- ⚠ **`AppCompatDelegate.setApplicationLocales()` is the WRONG tool for TL/POJ**: `LocaleList` normalization strips `-x-` private-use subtags, and the system per-app-language picker only surfaces real OS locales. Verified limitation.
- **Resource qualifier dirs cannot encode `-x-` private-use** → cannot split TL vs POJ as two `values-b+nan+Latn+TW+x+…` dirs.
- **VERIFIED model (florisboard, the app's base)**: identity = app-level `DisplayLanguage` enum in **DataStore** (NOT `setApplicationLocales`) → build a localized Context via `createConfigurationContext(Configuration().apply { setLocale(...) })` held as Compose `mutableStateOf` → expose through a `LocalResourcesContext` CompositionLocal + a custom `stringRes()` reading `LocalResourcesContext.current.resources.getString(id)`. Gives **live switch with zero Activity/IME `recreate()`**. Source: `references/florisboard/.../Resources.kt:40-88`, `FlorisAppActivity.kt:99-104`, `OtherScreen.kt:85-148`.
- **3 real locales** (en/ja/zh-Hant) → native `values/`, `values-ja/`, `values-b+zh+Hant/` (rename legacy `values-zh-rTW/`; `b+` qualifiers are API 24+). **TL/POJ** → an **enum-selected generated string set read by the custom resolver** (not a `values-*` dir, since both map to `nan-Latn-TW` and qualifiers can't split them). This is exactly D2's hybrid.
- **IME runs in the SAME app process** (not a separate process like the iOS extension — verified `TaigiKeyboard : LifecycleInputMethodService`, no `android:process`). Reuse the existing `onConfigurationChanged` → `onCreateInputView()` / `setInputView()` rebuild hook (`android/.../ime/core/TaigiKeyboard.kt:316`) + collect the same DataStore Flow → recompose the input view. No service restart.
- **No pbxproj-equivalent blocker** — `build.gradle.kts`, `res/`, and the manifest are all Claude-editable.

### Cross-platform conclusion

D2 (hybrid) and D7 (reactive root state) **CONFIRMED + sharpened**: 3 real locales lean on native resources; TL/POJ resolve through a custom indirection on BOTH platforms (iOS per-bundle `.lproj`; Android `LocalResourcesContext`). **No reference IME implements a non-OS-locale display language** — florisboard/azooKey switch only real locales. So TL/POJ resolution + cross-process live-switch is the genuinely unprecedented part, validating the spike-first gate.

### Codegen (Python, mirrors `dictionary/` convention)

- Tool under `tools/i18n/`; new **`make i18n` target** (mirrors `make dict` — explicit committed output, NOT a per-compile Xcode/Gradle phase, so archive / non-symlink builds stay fresh). POJ-derive (D4) shells out to the Node `taigi-converter` (precedent: `dictionary/common/taigi_bridge.py` Python→Node bridge).
- Emits: iOS `Localizable.xcstrings` + `L10n.swift` typed accessors (into a synced group → Swift auto-includes, no pbxproj edit); Android `values-*/strings.xml` + a Kotlin accessor object + the TL/POJ generated string maps. `content/*.json` grows `tailo`/`ja`/`en` keys (already symlinked + multilang-shaped — lowest-risk, highest-volume path).

### USER project-config hand-offs (iOS only — Claude cannot edit pbxproj/Info.plist)

1. Add generated `Localizable.xcstrings` to **both** the host app and Keyboard extension Resources build phases (String Catalogs are resources, NOT covered by synced-group Swift auto-include).
2. Add `CFBundleLocalizations` (the 5 identifiers incl. the two private-use tags) to both `Info.plist`.
3. Extend `knownRegions`.
4. Possibly add `.lproj` folder references for the TL/POJ custom-orthography sets.

Android has **no** equivalent gate.

---

## Execution rollout (risk-first sequencing, USER-gated)

Reorders the Phase table for de-risking: the D7 live-switch prototype (the #1 risk) runs as a throwaway spike **before** the P1 infra investment; Android leads because it has no pbxproj gate and its IME (same-process) + host both prove the cross-surface live-switch with full Claude control. iOS follows once USER does the project-config hand-offs.

| Round | Content | Platform | PR boundary | Gate |
|---|---|---|---|---|
| **R0** (P0 finish) | 2 wording reconciles (`HomeTexts.kt:15` drop `Android ` → `台語齒盤`; iOS `HomeTexts.swift:288` drop `「」` → `建中整理、提供`); optionally commit the Tier-1 report | iOS + Android | branch + PR, <10 LOC | visual dogfood; no test; Codex sandwich skippable (trivial value swap) |
| **R1** live-switch spike | Throwaway: root `DisplayLanguage` DataStore state → host (Compose) + IME (same-process) recompose via `LocalResourcesContext` / `createConfigurationContext`, incl. a TL/POJ enum-selected set. 2 strings × 2 langs, hardcoded | **Android first** | scratch / draft PR (not a feature) | **architecture go/no-go** — proves the unprecedented TL/POJ + live-switch part; pass → R2, fail → rethink D2/D7 before any infra |
| **R1′** iOS spike | Same minimal prototype: `@Observable` store → per-bundle `.lproj` override + `Text(key, bundle:)`; extension reads App Group `UserDefaults` | iOS | scratch | ⛔ BLOCKED on USER pbxproj (`CFBundleLocalizations` + TL/POJ `.lproj`) |
| **R2** (P1 infra) | `i18n/` JSON schema (1-2 namespaces first: `common` + `settings`) + Python codegen + `make i18n` + native resources + typed accessors + scope-aware key check + pseudo-locale + freshness check; hand-mirror dies incrementally | both (iOS resources USER-gated) | split R2a (codegen + schema + Android) / R2b (iOS); ~300-500 LOC each | spike passed |
| **R3** (P2) | locale state + persistence + picker (English only) + live-switch wired for real + native plural + dynamic-type / long-string layout + a11y locale | both | vertical slice | completeness + layout + a11y |
| **R4a/b/c** (P3) | ja (font verify) / TL authoring (Core Principle #3 — never invent TL) / POJ (derive-and-store from tailo + diff review). `content/*.json` per-language authoring rides here | both | one language per round | per-language gate before entering picker |

Only change from the Phase table above: the D7 prototype is pulled out of P1 into its own R1 spike, run **before** P1 infra, Android-first.
