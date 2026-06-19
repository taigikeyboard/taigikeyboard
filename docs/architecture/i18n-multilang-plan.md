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
      "ja": "入力モード",
      "en": "Input Mode"
      // poj omitted → derived from tailo at build (D4)
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

Rationale: JSON (ARB-shaped) is the i18n best-practice choice — machine-safe, TMS-ready (Crowdin/Weblate ingest ARB/XLIFF; JSON↔ARB↔XLIFF convert cleanly), native placeholder/plural metadata, matches repo precedents (`content/*.json`, `taigi-emojis/dist/emoji.json`). Named placeholders `{imported}` (not positional `%d`); plurals via key-suffix entries (`x.count.one` / `x.count.other`) → codegen to native plurals; `poj` blank = derive, string = override, `"skip"` = no derive.

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

- Schema: default `poj = derive(tl)`, plus `pojOverride` and `derivePoj: false`, plus protected-span / placeholder handling.
- **Generated-diff human review** — "build succeeded" ≠ "language correct".

### 5. Codegen + scope-aware key checks  *(Codex: CONFIRM codegen, REFUTE hard iOS==Android equality)*

- Codegen typed accessors (`L10n.foo` / `Strings.foo`) — ~200+ strings is not over-engineering.
- **Scope-aware schema** (NOT a hard 3-way equal key set, which would force fake strings or block platform UX): `shared-host` / `ios-host` / `android-host` / `shared-extension` / `ios-extension` / `android-extension`. Build check = "every *referenced* key exists and matches its scope," not "both platforms reference identical sets." Preserves legitimate platform-only keys (`resetFailed`, `noEmailApp`).

### 6. No ICU runtime; native plural via codegen typed functions  *(Codex: REFUTE "only 2 %d", CONFIRM no ICU dep)*

- There are **≥5** format constants today, and adding English likely needs plurals (`1 item` / `2 items`).
- No cross-platform ICU dependency. Codegen emits **typed functions** (`importResult(imported:duplicates:)`), backed by Apple String Catalog plural/substitution + Android `plurals`/resources. Forbid call sites passing raw `%d`.

### 7. Reactive locale state (NOT singleton/wrapper)  *(Codex: REFUTE singleton, CONFIRM reactive state)* — **biggest gap**

A static `L10n.foo` getter does NOT tell SwiftUI/Compose to refresh → switching language could leave a screen mixing old/new text until a view rebuild / Activity restart / extension restart.

- iOS: app-root **observable locale state** injected via SwiftUI environment; the resolver itself stateless. Do NOT re-add per-view `@StateObject` wrappers (the Stage-7 boilerplate).
- Android: prefer `setApplicationLocales()`; for TL/POJ custom resource context, drive via root state / `CompositionLocal`.
- Keyboard extension is a **separate process/lifecycle** — needs its own defined update contract.
- **Gate**: build a root-to-leaf reactive prototype proving live-switch for BOTH host app and extension **before P1 closes**.

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
| **P3a** | Japanese (incl. font/glyph-shaping verification — global Open Huninn may render Taiwan glyph forms; likely system font for `ja`) | adds language | — |
| **P3b** | TL authoring (Taigi prose; Core Principle #3 authoritative-source-only; never invent TL) | adds language | — |
| **P3c** | POJ derive + override + human diff review | adds language | — |

Picker shows a language only after it passes completeness + layout + accessibility gates. P1 alone delivers value (single source of truth) independent of multi-language shipping.

---

## Design gaps to resolve (from Codex)

- **Automatic mode** = tri-state with persistence; not a one-time OS-locale seed.
- **Keyboard-extension scope** — `*Texts` is used by extension overlays; in scope, crosses process boundary.
- **FAQ/features content** (`content/*.json`) — host-app UI text, currently `hanji`-only; must enter a phase.
- **Fallback policy** — production completeness should fail build; `→ 漢字` is anti-crash only, else mixed-language UI.
- **Accessibility / TTS** — each language needs a speech locale; verify VoiceOver/TalkBack support for `nan-*` on device.
- **Japanese font** — verify kana coverage + glyph forms; likely system font, not forced Open Huninn.
- **Dynamic Type / long strings** — English is longer; truncation + screenshot tests on tab titles, settings rows, dialogs, overlays.
- **Number/date/plural formatting** — locale policy, not just template translation.
- **App metadata** — InfoPlist, Android app name, permission/setup copy, App Store / Play listing = separate localization scope.
- **Reset/migration semantics** — does "reset settings" clear the display-language override? Define.
- **OS negotiation** — primary locale vs first-supported in the preferred-language list. Define.

---

## Risks

- **Live-switch invalidation has no architectural definition** — the #1 risk. Static `L10n.foo` won't trigger SwiftUI/Compose refresh; extension is a separate process. Mitigation = the P1 reactive prototype gate (Decision 7).
- **TL authoring is a real linguistic task** — never invent TL (Core Principle #3 + taigi-emojis lesson); the long pole, not the engineering.
- **POJ auto-derive can corrupt non-Taigi tokens** — default-derive + override + diff review (Decision 4).
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
