# Theme Presets & Custom Themes — Brainstorm (v3.6.2)

> **Type**: Planning (brainstorm — evolving; USER will append ideas)
> **Keywords**: `theme`, `preset`, `palette`, `colorscheme`, `theme set`, `shelf`, `custom theme`, `image upload`
> **Status**: Brainstorm — NO code. Two forks decided (USER 2026-06-05): **5th nav tab** + **theme-id resolve storage model**. Remaining forks open.
> **Version scope**: v3.6.2 (USER-scoped 2026-06-05: 「這個列為 v3.6.2 的計劃」)
> **Related**: `docs/ui/theme.md` (current-state reference), `docs/roadmap.md` (deferred TODO "keyboard theme picker")

---

## Summary

- **Goal**: ship predefined **theme sets** (each = a named palette with a 小標題, bundling **light + dark** variants), a KeyboardKit-Shelf-style picker on its **own 5th nav tab**, and evaluate **user-uploaded image** as a custom-theme source.
- **Current state is NOT greenfield** — both platforms already have a mirrored 6-role free-pick color system (`KeyboardColorSettings`). The v3.6.2 work shifts storage from "store 6 raw colors" to "**store a `selectedThemeId`, resolve 6 roles (and light/dark) at render time**"; the existing free-pick path becomes the **Custom** theme.
- **Layered design**: palette values from established editor/vim colorschemes (Catppuccin, Tokyo Night, Gruvbox, Solarized, Nord); the **picker UX** references KeyboardKit's `KeyboardTheme.Shelf`; we **deliberately do NOT use** KeyboardKit's theme *engine* (Pro-gated) or FlorisBoard's Snygg stylesheet engine + addon store.
- **Two USER decisions 2026-06-05**: (1) **Nav fork A** — appearance/theme becomes its own top-level tab (4 → 5 tabs); (2) **Storage fork** — adopt the theme-id resolve model (auto-solves the light/dark fork B and active-identity fork D).
- **Remaining open forks**: preset coverage (6 roles vs full chrome, §6 A), preset source-of-truth (hand-mirror vs shared JSON, §6 C), image-upload scope (§7), final roster + licensing (§5).

---

## 1. Goal & USER directives

USER 2026-06-05 (verbatim, kept as evidence):

> 「我想製作一些定義的主題色讓 user 選」「評估讓使用者上傳圖片當作自定義的可能性」「先撰寫文件,我想到什麼再補充上來」「這個列為 v3.6.2 的計劃」
> 「根據主流 vim 主題配色、包含 light/dark theme」「一個主題 set 有小標題」「主題選擇頁面我想參考 keyboardkit」「目前 app 有四個 tabs 導覽列,將主題外觀獨立成一個 tab,總共會有 5 個」

These satisfy CLAUDE.md Core Principle #5 (release scope user-gated) — v3.6.2 scope + the nav/storage decisions are the USER's explicit dated instructions, not inferred. Remaining phase sequencing stays user-gated.

Aligns with the existing `docs/roadmap.md` deferred TODO: **"keyboard theme picker"** (the single open roadmap item as of 2026-06-01).

---

## 2. Current state (grounded in code, 2026-06-05)

Both platforms share a **deliberately mirrored** architecture (`KeyboardColorSettings.kt:14` "Matches iOS KeyboardColorSettings structure"). Any schema change here is a cross-platform-invariant surface per `.claude/rules/cross-platform-alignment.md`.

### 2.1 Color role set — 6 user-controllable roles (identical both platforms)

| # | Role | iOS field | Android field |
|---|---|---|---|
| 1 | Keyboard background | `backgroundColor` | `backgroundColor` |
| 2 | Key text | `keyTextColor` | `keyTextColor` |
| 3 | Normal key fill | `normalKeyFillColor` | `normalKeyFillColor` |
| 4 | Special key fill | `specialKeyFillColor` | `specialKeyFillColor` |
| 5 | Candidate text | `candidateTextColor` | `candidateTextColor` |
| 6 | Candidate background | `candidateBackgroundColor` | `candidateBackgroundColor` |

- iOS: `ios/Sources/TaigiKeyboard/Settings/KeyboardColorSettings.swift:42-58` (each `CodableColor?`, `nil` = uncustomized → KeyboardKit dynamic fallback).
- Android: `android/.../ime/core/KeyboardColorSettings.kt:16-23` (each nullable ARGB `Int`, `null` = theme default).

**The gap**: the full keyboard chrome is larger than 6 roles and is **not** user-overridable today (Android ~30 XML attrs `attrs.xml:4-40`; iOS KeyboardKit adaptive internals). A theme can only recolor the 6 roles — pressed/popup/enter/emoji/smartbar chrome stays platform-default (see §6 fork A).

### 2.2 Storage today — single JSON blob, one key, per-platform

- iOS: `SharedSettings.swift:99` `colorSettingsKey = .codable("colorSettings", default: .default)` → App Group UserDefaults. `CodableColor` = RGBA `Double` ×4.
- Android: `PrefHelper.colorSettings: String` (`PrefHelper.kt:500`, default `"{}"`) → DataStore key `COLOR_SETTINGS`; JSON, each role nullable packed-ARGB `Int`.
- **No separate light/dark storage** — one value per role, both modes. (This is what the §4 storage shift fixes.)

### 2.3 Customization UI today — per-element free pick, NO preset concept

- iOS: 6 `ColorPicker` rows (`AppearanceSettingsView.swift:186-209`). Android: 6 `ColorSettingRow` → `ColorPickerDialog` (`ColorPickerDialog.kt:73-75`). Neither has any named-preset entity. Live preview via `KeyboardPreviewPanel`.

### 2.4 Navigation today — 4 tabs, appearance nested under Layout

- iOS: `TabType.swift:7-12` — `.home / .layout / .dictionary / .settings`; built in `ContentView.swift:16-60`. Appearance lives under the **Layout** tab (「齒盤佈局」= layout + font + color), file `App/Tabs/Layout/AppearanceSettingsView.swift`.
- Android: `ui/tabs/{home,layout,dictionary,settings}/` + `MainSettingsScreen.kt` (NavigationBar). Appearance under **layout**, file `ui/tabs/layout/AppearanceSettingsScreen.kt`.

### 2.5 Resolver path (one-line, both platforms)

- iOS: `UserDefaults["colorSettings"]` → `TaigiKeyboardView.@State colorSettings` → `CandidateTheme.resolved` (candidate text via `.candidateTheme` env) + inline KeyboardKit `.background`/`.keyboardButtonStyle`/`candidateStyle` closures. Live-refresh on `UserDefaults.didChangeNotification`.
- Android: `prefs.colorSettings` JSON → `KeyboardColorSettings.fromJson()` → `KeyboardAppearanceResolver` (keys) + `SmartbarManager.colorSettings()` (candidates) → `userOverride ?: getColorFromAttr(themeAttr)`.

**Key consequence**: the renderer already consumes a `KeyboardColorSettings` (6 roles). The §4 model only changes *where those 6 values come from* (a resolved theme vs raw storage) — the render sinks are untouched.

---

## 3. Design decision — palette source vs apply architecture (layers)

The original fork ("FlorisBoard addon vs vim colorscheme") conflates separate layers. They are NOT either/or.

| Layer | Question | Reference borrowed |
|---|---|---|
| **A. Palette source** | which colors, which roles | **vim/editor colorschemes** (Catppuccin/Tokyo Night/Gruvbox/Solarized/Nord) |
| **B. Picker UX** | how the user browses + picks a theme | **KeyboardKit `KeyboardTheme.Shelf`** (UI design only) |
| **C. Apply architecture** | how colors reach each element | our existing 6-role resolver; theme = a resolved `KeyboardColorSettings` |

**Critical fact**: Catppuccin / Tokyo Night / Gruvbox / Tron etc. are originally editor/vim colorschemes. FlorisBoard addons merely repackage them. So "vim colorscheme" and "those FlorisBoard addons" are the **same color-source pool**, differing only in delivery.

### Layer A — palette: adopt the vim-colorscheme model

Pre-curated, harmonious, semantic-role native, brand-recognized, permissively licensed (most MIT). Maps cleanly onto our 6 roles. See §5.

### Layer B — picker UX: reference KeyboardKit `KeyboardTheme.Shelf`

Grounded in KK 9.9.0 docs (`references/keyboardkit9.9.0/.../Themes-Article.md:179-194`, doc-lookup 2026-06-05):

- `KeyboardTheme.Shelf` = "a vertical list of **horizontally scrolling shelves**"; `KeyboardTheme.ShelfItem` renders "how a `Keyboard.Button` will look" — i.e. **live keyboard-button preview swatches**.
- This maps 1:1 to the USER's model: a theme **set** = one shelf with a **title (小標題)**; the set's **variants** (light/dark) = horizontally-scrolling `ShelfItem`s, each a live preview.
- KK's own themes use the same set→variation shape (`KeyboardTheme.standard` + `.blue`/`.green` variations; `.tron` + `.fcon`/`.virus`, `Themes-Article.md:71,160`).

⚠ **Licensing caveat (doc-lookup gate)**: KeyboardKit's theme **engine** is **Pro-gated** — the open-source 9.9.0 has only `_Pro/ProPlaceholders.swift`. We **cannot use the KK theme engine**; we **reference the Shelf UI design** and feed our own static theme table into the existing 6-role resolver. iOS picker = a SwiftUI re-implementation of the Shelf layout; Android = a Compose `LazyColumn` of horizontally-scrolling `LazyRow` shelves (mirrors the Shelf shape).

### Layer C — apply architecture: existing 6-role resolver, NOT a new engine

- **Deliberately NOT adopted** (YAGNI): FlorisBoard Snygg stylesheet engine + arbitrary per-element selectors + addon store + publish pipeline; **and** KeyboardKit Pro's theme engine (license + closed-source). Both = a whole theming engine we don't need.
- **Adopted**: a theme = a fully-resolved `KeyboardColorSettings` (6 roles), chosen from a static table. No new render path.

---

## 4. Storage & resolve model — theme-id resolve (DECIDED 2026-06-05)

**Decision**: shift from "store 6 raw colors" to "**store a `selectedThemeId`; resolve the 6 roles — and the light/dark variant — at render time**." The existing free-pick 6-color path becomes a reserved theme id `custom`.

### 4.1 Static theme table (the new model layer)

```
ThemeSet {
  id: String              // "catppuccin", "gruvbox", "tokyoNight", "solarized", "nord", "florisDefault", "custom"
  displayName: String     // 小標題 shown on the shelf, e.g. "Catppuccin"
  light: SixRoleColors?   // nil if the set has no light variant
  dark:  SixRoleColors?   // nil if the set has no dark variant
}
SixRoleColors = { background, keyText, normalKeyFill, specialKeyFill, candidateText, candidateBackground }
```

- **Render-time resolve**: pick `light` vs `dark` by the system `colorScheme`; if the chosen variant is `nil`, fall back to the other (e.g. Nord dark-only → dark used in both modes).
- **`custom`**: `light == dark ==` the user's existing 6-color free-pick values. This preserves today's behavior as one selectable theme.

### 4.2 Persistence

- **New key** `selectedThemeId: String` (iOS `SettingsKey<String>` in `SharedSettings`; Android DataStore key). Default `custom` (so existing users keep their current look).
- **Keep** the existing `colorSettings` blob — it now backs *only* the `custom` theme (the free-pick pickers write here).
- **Precedence**: `selectedThemeId != custom` → resolve from the static table; `== custom` → use the `colorSettings` 6 overrides (today's path).

### 4.3 Migration (lightweight, non-destructive)

- Existing users have a `colorSettings` blob and no `selectedThemeId`. On read, absent `selectedThemeId` defaults to `custom` → their current colors render unchanged. No data move, no schema rebuild of the color blob.
- This is a pure additive key; mirrors the non-destructive migration discipline used across v3.6.1 (see `.claude/rules/taigi-incidents.md` S11–S15).

### 4.4 Why this resolves two forks

- **Fork B (light/dark)**: solved without per-role day/dark storage — the pair lives in the static table, not user storage. System appearance switches the variant at render time.
- **Fork D (active-preset identity)**: solved — `selectedThemeId` *is* the persisted identity; the shelf highlights it directly.

### 4.5 Cross-platform parity invariant (mandatory)

- The static theme table (ids, display names, every hex) must be **identical** on iOS and Android — now a larger surface (per theme up to 12 colors: 6 light + 6 dark). This **raises the value of Fork C option P2** (a shared JSON table parsed by both) over P1 (hand-mirrored tables). See §6 fork C.

---

## 5. Theme sets (starting curation) — vim colorschemes with light/dark

Each set = a 小標題 + light and/or dark variant. Hex mapped to the 6 roles with this rule: **`background` darkest → `specialKeyFill` mid → `normalKeyFill` lightest** (letters most raised, matches platform default where special keys are darker/muted). Light variants: letter keys near-white, special keys gray, bg the palette's light base. `accent` is reserved (not one of the 6 roles today; used only if §6 fork A widens chrome, or for the §19 first-candidate keycap hint).

### Catppuccin (MIT) — Latte (light) + Mocha (dark)

| role | Latte (light) | Mocha (dark) |
|---|---|---|
| background | `#eff1f5` | `#1e1e2e` |
| candidateBackground | `#e6e9ef` | `#181825` |
| specialKeyFill | `#ccd0da` | `#313244` |
| normalKeyFill | `#ffffff` | `#45475a` |
| keyText / candidateText | `#4c4f69` | `#cdd6f4` |
| *accent (reserved)* | `#8839ef` | `#cba6f7` |

### Gruvbox (MIT) — light + dark (warm/retro, fits our design philosophy)

| role | light | dark |
|---|---|---|
| background | `#fbf1c7` | `#282828` |
| candidateBackground | `#f2e5bc` | `#1d2021` |
| specialKeyFill | `#ebdbb2` | `#3c3836` |
| normalKeyFill | `#ffffff` | `#504945` |
| keyText / candidateText | `#3c3836` | `#ebdbb2` |
| *accent (reserved)* | `#b57614` | `#fabd2f` |

### Tokyo Night (MIT) — Day (light) + Night (dark)

| role | Day (light) | Night (dark) |
|---|---|---|
| background | `#e1e2e7` | `#1a1b26` |
| candidateBackground | `#d5d6db` | `#16161e` |
| specialKeyFill | `#c4c8da` | `#292e42` |
| normalKeyFill | `#ffffff` | `#414868` |
| keyText / candidateText | `#343b58` | `#c0caf5` |
| *accent (reserved)* | `#2e7de9` | `#7aa2f7` |

### Solarized (BSD/MIT-style, Ethan Schoonover) — Light + Dark (THE canonical light/dark pair)

| role | Light | Dark |
|---|---|---|
| background | `#eee8d5` (base2) | `#073642` (base02) |
| candidateBackground | `#fdf6e3` (base3) | `#002b36` (base03) |
| specialKeyFill | `#e3dcc4` | `#0a3a45` |
| normalKeyFill | `#fdf6e3` (base3) | `#0d4a57` |
| keyText / candidateText | `#657b83` (base00) | `#93a1a1` (base1) |
| *accent (reserved)* | `#268bd2` (blue) | `#268bd2` (blue) |

> Solarized fills are interpolated within the base tones for keyboard legibility; dogfood-tune.

### Nord (MIT) — dark-only canonical (light variant optional, flagged)

| role | dark |
|---|---|
| background | `#2e3440` (nord0) |
| candidateBackground | `#2e3440` |
| specialKeyFill | `#3b4252` (nord1) |
| normalKeyFill | `#434c5e` (nord2) |
| keyText / candidateText | `#eceff4` (nord6) |
| *accent (reserved)* | `#88c0d0` (nord8) |

> Nord has no official light theme. Options: ship dark-only (variant fallback handles both modes) or author a "Nord Light" using snow-storm tones (`#eceff4` bg / `#2e3440` text). Flagged for USER.

### Floris Default (apache-2.0) — Day + Night (keep existing baseline)

| role | Day (light) | Night (dark) |
|---|---|---|
| background | `#e0e0e0` | `#212121` |
| candidateBackground | `#f5f5f5` | `#212121` |
| specialKeyFill | `#d0d0d0` | `#313131` |
| normalKeyFill | `#ffffff` | `#424242` |
| keyText / candidateText | `#121212` | `#dcdcdc` |
| *accent (reserved)* | `#4caf50` | `#4caf50` |

Source: bundled florisboard `org.florisboard.themes/stylesheets/{floris_day,floris_night}.json`.

### Plus: `Custom` (reserved id)

The existing 6-role free-pick. Always present; selecting it activates the per-element ColorPickers.

### Keyword-list extras (deferred — license + design check)

`Tron` / `Nothing` / `Windows Phone` / `Oblivion` from the USER's original list: not single canonical palettes (Tron/Nothing are aesthetics needing original interpretation) and need per-source license checks. Deferred until USER confirms which to include.

> All hex values above are derived from canonical palettes mapped to 6 roles — **final values gated on device dogfood** for contrast/legibility (`code-review-rules.md §9`).

---

## 6. Open design forks

### Decided 2026-06-05

- **Nav fork → A**: appearance/theme becomes its **own 5th top-level tab** (see §8.1).
- **Storage fork → theme-id resolve** (§4). This **closes Fork B (light/dark)** and **Fork D (active identity)**.

### Fork A (open) — preset coverage: 6 roles only, or widen the override surface?

- **A1 (MVP)**: themes recolor only the 6 existing roles; pressed/popup/enter/emoji/smartbar chrome stays platform-default → may clash with strongly-tinted themes.
- **A2 (full theme)**: make chrome overridable — Android route `getColorFromAttr`/`KeyboardChromeColors.from` through a palette layer; iOS extend the KeyboardKit style closures. Larger, touches XML-attr seam + KeyboardKit internals.
- Recommendation: **start A1**, dogfood, widen to A2 only if themes look broken (direction-first per Core Principle #6, but decide with evidence).

### Fork C (open) — theme table source of truth: hand-mirror (P1) or shared JSON (P2)?

- P1 = consistent with current `KeyboardColorSettings` mirroring; P2 = drift-proof shared JSON parsed by both, closer to FlorisBoard's stylesheet-as-data.
- **The §4 model enlarges the table (up to 12 colors/theme), raising P2's value.** Recommendation leans P2; USER to confirm.

---

## 7. Image upload as custom theme — feasibility evaluation

Two distinct interpretations with very different cost/risk:

### I-1 — extract a palette FROM the image (image → the 6 roles → writes the `custom` theme)

- Pipeline: pick image (host app) → color quantization (Android `androidx.palette`; iOS Core Image / vImage quantizer) → dominant colors → heuristic map to 6 roles (with a contrast clamp) → write the `colorSettings` blob + set `selectedThemeId = custom`.
- **Fits the model perfectly** — output is the existing 6-role `custom` theme; zero renderer change; no image stored at keyboard runtime; the 64 MB extension cap is avoided (processing is host-app-side, only 6 colors cross into shared settings).
- Risk: auto-mapped palettes can be low-contrast → enforce a min contrast ratio + live preview + let the user tweak via the existing 6 pickers.
- **Feasibility: HIGH.** Recommended image path — degrades to "a fancy Custom-theme generator."

### I-2 — image AS the keyboard background (background-image layer)

- Needs a new "background image" concept (a layer behind translucent keys) + the bitmap loaded **inside the keyboard extension at render time**.
- **iOS blocker**: keyboard extension ~64 MB hard memory cap (`.claude/rules/taigi-incidents.md`). A full-res photo decoded in the extension can blow the cap → keyboard killed. Requires app-side downscale/re-encode to a strict max dimension + decoded-size budgeting against engine + dict. Fragile. Also needs translucent-key restyling for legibility.
- **Feasibility: LOW-MEDIUM, iOS-gated.** Recommend **defer** to its own user-gated slice with an explicit memory budget.

### Image recommendation

- MVP: **I-1 only** (palette extraction → `custom` theme). Defer **I-2** (background image).

---

## 8. Information architecture & phases

### 8.1 Navigation — 5th tab (DECIDED: fork A, 2026-06-05)

Appearance/theme is promoted to its own top-level tab. Result: **5 tabs** (at the iOS / Android Material-3 NavigationBar practical maximum — acceptable, no room for a 6th).

- **iOS**: add `case theme` to `TabType` (`TabType.swift`), add the 5th `tabItem` in `ContentView.swift`, icon `paintpalette.fill` (or `paintbrush.fill`), localized title via a new `ThemeTexts`. Move `AppearanceSettingsView` out from under `LayoutTab` into a new `ThemeTab`.
- **Android**: add a `theme` destination to the NavigationBar in `MainSettingsScreen.kt` + a `ui/tabs/theme/` package, mirroring `ui/tabs/layout/`. Move `AppearanceSettingsScreen` content into it.
- **Layout tab retains** keyboard layout + font (it stays a valid tab; just no longer overloaded with color).
- **Cross-platform parity**: tab order, icon semantics, and title must mirror (a UI parity surface; capture in `ui-style-guide.md` when implemented). The existing `.switchToSettingsTab`-style deep links must be checked for the new index.
- **Trade-off recorded** (for posterity): this spends a scarce nav slot on a set-and-forget feature, which runs against `ui-style-guide.md` §Feature Grouping by Usage Frequency. USER accepted the trade for discoverability / showcase value (appearance is a keyboard-app selling point; SwiftKey/Gboard surface themes prominently).

### 8.2 Draft phase table (NOT committed sequencing — user-gated)

Sizing targets 200–500 LOC/PR per `~/.claude/rules/planning.md`.

| Phase | Scope | Depends on |
|---|---|---|
| P0 (admin) | This doc + roadmap entry + memory file | — |
| P1 | 5th tab scaffolding both platforms (move appearance out of Layout, no behavior change) | — |
| P2 | Theme-id storage model + resolver (`selectedThemeId`, `custom` precedence, migration) | — |
| P3 | Static theme table (resolve Fork C) + curated hex (resolve Fork A coverage) | P2 |
| P4 | Shelf-style picker UI both platforms + live preview + light/dark resolve | P1, P2, P3 |
| P5 (optional) | Image → palette extraction (I-1) writing `custom` | P2 |
| (deferred) | A2 full-chrome override / I-2 background image / keyword-list extra themes | user-gated later |

---

## 9. Best-practices alignment (per `~/.claude/rules/planning.md`)

| 主流做法 | 來源 | 本 plan 對應 |
|---|---|---|
| Theme picker as scrolling shelves of live previews | KeyboardKit `Themes-Article.md:179-194` (`KeyboardTheme.Shelf` / `ShelfItem`) | §3 Layer B, §8.1, P4 |
| Theme = set + style variations | KeyboardKit `Themes-Article.md:71,160` (`.standard`+`.blue`; `.tron`+`.virus`) | §4 ThemeSet, §5 sets |
| Persist a selected-theme identity | KeyboardKit `KeyboardThemeContext` (Themes-Article.md:28-32) | §4.2 `selectedThemeId` |
| Semantic color tokens (`@defines`) | FlorisBoard `stylesheets/floris_day.json:3-31` | §4.1 `SixRoleColors` |
| Curated colorscheme palettes | Catppuccin / Tokyo Night / Gruvbox / Solarized / Nord (mostly MIT) | §5 |
| Palette extraction from image | AndroidX `androidx.palette` (canonical) | §7 I-1 |

**Project rules cited**: `cross-platform-alignment.md` (theme table + tab order are parity surfaces, §4.5/§8.1); CLAUDE.md Core Principle #2/#5/#6; `taigi-incidents.md` (iOS 64 MB cap §7; non-destructive migration §4.3); `ui-style-guide.md` §Feature Grouping (the §8.1 trade-off); `code-review-rules.md §9` (qualitative dogfood gate for theme legibility).

**刻意不採用 (deliberately not adopted)**:
- **KeyboardKit Pro theme engine** — Pro-gated + closed-source; we reuse only its Shelf UI *design* and feed our own table into the existing 6-role resolver (§3 Layer B).
- **FlorisBoard Snygg stylesheet engine / addon store** — YAGNI, over-engineering on two platforms (§3 Layer C).
- **I-2 runtime background-image in the iOS extension** — deferred on the 64 MB cap (§7).
- **Per-role day/dark storage (fork B2)** — unnecessary; the theme-id model keeps the pair in the static table, not user storage (§4.4).

---

## 10. Open questions / TODO (USER to append)

- [x] Nav structure — **A: 5th tab** (2026-06-05)
- [x] Storage model — **theme-id resolve** (2026-06-05); closes light/dark + active-identity forks
- [ ] Fork A — 6-role MVP vs full-chrome override?
- [ ] Fork C — hand-mirrored theme table vs shared JSON source of truth (model now favors shared JSON)?
- [ ] Nord light variant — ship dark-only, or author a "Nord Light"?
- [ ] Keyword-list extras (Tron / Nothing / Windows Phone / Oblivion) — which to include; license-check.
- [ ] Image upload — I-1 (palette extract) in v3.6.2, or defer all image work?
- [ ] Tab icon choice (iOS `paintpalette.fill` vs `paintbrush.fill`; Android equivalent) + localized tab title.
- [ ] (USER additions below)

---

## Appendix — code-grounding citations (read 2026-06-05, READ-ONLY)

**iOS**: `Settings/KeyboardColorSettings.swift:10-58` · `Autocomplete/Models/CandidateTheme.swift:46-73` · `App/Tabs/Layout/AppearanceSettingsView.swift:44-243` · `AppearanceSettingsViewModel.swift:19-162` · `SharedSettings.swift:99,502-505,572` · `App/Tabs/TabType.swift:7-34` · `App/ContentView.swift:16-65`.

**Android**: `ime/core/KeyboardColorSettings.kt:14-54` · `ime/theme/ThemeAttributeColors.kt:9-16` · `ime/text/KeyboardAppearanceResolver.kt:47,56-63` · `ime/text/smartbar/KeyboardChromeColors.kt:28-47` · `ui/tabs/layout/AppearanceSettingsScreen.kt:80-392` · `ui/tabs/layout/ColorPickerDialog.kt:73-280` · `ui/tabs/MainSettingsScreen.kt` (NavigationBar) · `PrefHelper.kt:84,500`.

**KeyboardKit reference** (9.9.0, doc-lookup 2026-06-05): `references/keyboardkit9.9.0/Sources/KeyboardKit/KeyboardKit.docc/Features/Themes-Article.md:14-194` (Shelf/ShelfItem, ThemeContext, predefined themes incl. `.tron`); `_Pro/ProPlaceholders.swift` (engine is Pro-gated).

**FlorisBoard reference**: `references/florisboard/.../org.florisboard.themes/{extension.json, stylesheets/floris_day.json, floris_night.json}`.
