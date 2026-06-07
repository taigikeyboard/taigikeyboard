# Theme Presets & Custom Themes — Brainstorm (v3.6.2)

> **Type**: Planning (brainstorm — evolving; USER will append ideas)
> **Keywords**: `theme`, `preset`, `palette`, `colorscheme`, `theme set`, `shelf`, `custom theme`, `user theme`
> **Status**: Brainstorm — NO code. Decided (USER 2026-06-05; editor + tab-position revised 2026-06-07): **dedicated nav tab placed 2nd** (5 tabs total), **theme-id resolve storage model**, **user-created named themes (the "+" flow)**, **user themes local-only** (excluded from OS auto-backup; NOT in `.taigi`), **full 6-role editor + a key-shadow intensity slider** (editor NOT simplified). Remaining forks (§10) **deferred to implementation-time review** — each carries a recommended lean but is NOT committed.
> **Version scope**: v3.6.2 (USER-scoped 2026-06-05: 「這個列為 v3.6.2 的計劃」)
> **Related**: `docs/ui/theme.md` (current-state reference), `docs/roadmap.md` (deferred TODO "keyboard theme picker")
>
> **DROPPED (USER 2026-06-07, 暫時不需要 — off the plan, re-open only on explicit revive)**:
> 1. **Image upload as a custom-theme source** (all of §7 — I-1 palette-extract + I-2 background-image). Removed from plan.
> 2. **Cross-device theme backup / `.taigi` theme export-import**. User themes are **local-only**: the shipped `isExcludedFromBackup` exclusion stays (themes are not in OS auto-backup), and themes are **NOT** added to `.taigi` serialization. Reinstall / new device does **not** carry custom themes — acceptable per USER.

---

## Summary

- **Goal**: ship predefined **theme sets** (each = a named palette with a 小標題, bundling **light + dark** variants), let users **create / name / save multiple custom themes** (a KeyboardKit-style **"+"** flow that survives app updates), present everything in a KeyboardKit-Shelf-style picker on its **own 5th nav tab**, and evaluate **user-uploaded image** as a custom-theme source.
- **Current state is NOT greenfield** — both platforms already have a mirrored 6-role free-pick color system (`KeyboardColorSettings`). v3.6.2 shifts storage from "store 6 raw colors" to "**store a `selectedThemeId`, resolve 6 roles (and light/dark) at render time**", and adds a **persisted list of user-created themes**. The existing 6-picker UI becomes the theme **editor** reached via "+".
- **Layered design**: palette values from established editor/vim colorschemes (Catppuccin, Tokyo Night, Gruvbox, Solarized, Nord); the **picker UX** references KeyboardKit's `KeyboardTheme.Shelf`; we **deliberately do NOT use** KeyboardKit's theme *engine* (Pro-gated) or FlorisBoard's Snygg stylesheet engine + addon store.
- **USER decisions 2026-06-05**: (1) **Nav** — appearance/theme → its own top-level tab (4 → 5; **placed 2nd**, after 頭頁 — USER 2026-06-07); (2) **Storage** — theme-id resolve model (auto-solves light/dark + active-identity forks); (3) **User themes** — multiple named, saved, update-durable, applyable themes via "+"; (4) **Backup** — user themes **local-only**: excluded from OS auto-backup (fork F-Exclude, shipped), and **NOT** in `.taigi` (cross-device theme backup dropped — USER 2026-06-07; see top banner).
- **Editor (USER 2026-06-05, revised 2026-06-07)**: the custom-theme editor keeps the **full 6-role free-pick** (NOT simplified — the existing 6-picker UI is reused as-is; the earlier 3-color + text merge is dropped) **+ a key-shadow intensity slider** (an independent render property, NOT tied to any fill merge). Built-ins keep the authored 6-role tables + `keyShadow = 0` (flat); the user editor adds the shadow control on top of the 6 roles (§4b.2/§4b.5).
- **Working default (USER may override)**: user-theme light/dark editing = **single value (E1)** for MVP (§6 E).
- **Remaining open forks**: preset coverage (6 roles vs full chrome, §6 A), theme-table source-of-truth (hand-mirror vs shared JSON, §6 C), final roster + licensing (§5). (Image-upload scope §7 and cross-device theme backup — both DROPPED, see top banner.)

---

## 1. Goal & USER directives

USER 2026-06-05 (verbatim, kept as evidence):

> 「我想製作一些定義的主題色讓 user 選」「評估讓使用者上傳圖片當作自定義的可能性」「先撰寫文件,我想到什麼再補充上來」「這個列為 v3.6.2 的計劃」
> 「根據主流 vim 主題配色、包含 light/dark theme」「一個主題 set 有小標題」「主題選擇頁面我想參考 keyboardkit」「目前 app 有四個 tabs 導覽列,將主題外觀獨立成一個 tab,總共會有 5 個」
> 「我希望使用者可以儲存自定義的 theme,像是 keyboardkit 那樣,有一個 + 號,可以替自己自定義的主題取名、儲存,不會因為更新不見,可以自定義多個主題套用」

These satisfy CLAUDE.md Core Principle #5 (release scope user-gated) — v3.6.2 scope + the nav/storage/user-theme decisions are the USER's explicit dated instructions, not inferred. Remaining phase sequencing stays user-gated.

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
- **No separate light/dark storage** — one value per role, both modes. (§4 storage shift addresses this.)
- **App-container persistence survives app updates** on both platforms (UserDefaults / DataStore are not wiped on update). This is the durability the USER's "不會因為更新不見" relies on (see §4b.3).

### 2.3 Customization UI today — per-element free pick, NO preset concept

- iOS: 6 `ColorPicker` rows (`AppearanceSettingsView.swift:186-209`). Android: 6 `ColorSettingRow` → `ColorPickerDialog` (`ColorPickerDialog.kt:73-75`). Neither has any named-preset entity. Live preview via `KeyboardPreviewPanel`. **This 6-picker surface is reused as the theme editor (§4b).**

### 2.4 Navigation today — 4 tabs, appearance nested under Layout

- iOS: `TabType.swift:7-12` — `.home / .layout / .dictionary / .settings`; built in `ContentView.swift:16-60`. Appearance lives under the **Layout** tab (「齒盤佈局」= layout + font + color), file `App/Tabs/Layout/AppearanceSettingsView.swift`.
- Android: `ui/tabs/{home,layout,dictionary,settings}/` + `MainSettingsScreen.kt` (NavigationBar). Appearance under **layout**, file `ui/tabs/layout/AppearanceSettingsScreen.kt`.

### 2.5 Resolver path (one-line, both platforms)

- iOS: `UserDefaults["colorSettings"]` → `TaigiKeyboardView.@State colorSettings` → `CandidateTheme.resolved` (candidate text via `.candidateTheme` env) + inline KeyboardKit `.background`/`.keyboardButtonStyle`/`candidateStyle` closures. Live-refresh on `UserDefaults.didChangeNotification`.
- Android: `prefs.colorSettings` JSON → `KeyboardColorSettings.fromJson()` → `KeyboardAppearanceResolver` (keys) + `SmartbarManager.colorSettings()` (candidates) → `userOverride ?: getColorFromAttr(themeAttr)`.

**Key consequence**: the renderer already consumes a `KeyboardColorSettings` (6 roles). The §4 model only changes *where those 6 values come from* (a resolved theme — built-in or user-created — vs raw storage). The render sinks are untouched.

---

## 3. Design decision — palette source vs apply architecture (layers)

The original fork ("FlorisBoard addon vs vim colorscheme") conflates separate layers. They are NOT either/or.

| Layer | Question | Reference borrowed |
|---|---|---|
| **A. Palette source** | which colors, which roles | **vim/editor colorschemes** (Catppuccin/Tokyo Night/Gruvbox/Solarized/Nord) |
| **B. Picker UX** | how the user browses + picks + creates a theme | **KeyboardKit `KeyboardTheme.Shelf`** + a "+" create item (UI design only) |
| **C. Apply architecture** | how colors reach each element | our existing 6-role resolver; theme = a resolved `KeyboardColorSettings` |

**Critical fact**: Catppuccin / Tokyo Night / Gruvbox / Tron etc. are originally editor/vim colorschemes. FlorisBoard addons merely repackage them. So "vim colorscheme" and "those FlorisBoard addons" are the **same color-source pool**, differing only in delivery.

### Layer A — palette: adopt the vim-colorscheme model

Pre-curated, harmonious, semantic-role native, brand-recognized, permissively licensed (most MIT). Maps cleanly onto our 6 roles. See §5.

### Layer B — picker UX: reference KeyboardKit `KeyboardTheme.Shelf` + a "+" item

Grounded in KK 9.9.0 docs (`references/keyboardkit9.9.0/.../Themes-Article.md:179-194`, doc-lookup 2026-06-05):

- `KeyboardTheme.Shelf` = "a vertical list of **horizontally scrolling shelves**"; `KeyboardTheme.ShelfItem` renders "how a `Keyboard.Button` will look" — i.e. **live keyboard-button preview swatches**.
- Maps 1:1 to the USER's model: a theme **set** = one shelf with a **title (小標題)**; variants (light/dark) = horizontally-scrolling `ShelfItem`s, each a live preview.
- KK's own themes use the same set→variation shape (`KeyboardTheme.standard` + `.blue`/`.green`; `.tron` + `.fcon`/`.virus`, `Themes-Article.md:71,160`).
- A trailing **"+" item** opens the editor to create a user theme (§4b).

⚠ **Licensing + honesty caveats (doc-lookup gate)**:
- KeyboardKit's theme **engine** is **Pro-gated** — open-source 9.9.0 has only `_Pro/ProPlaceholders.swift`. We **cannot use the KK theme engine**; we **reference the Shelf UI design** and feed our own table + user themes into the existing 6-role resolver.
- KK open-source docs show only **code-level** custom themes (`Themes-Article.md:210-255`, `extension KeyboardTheme { static var greenPrimary }`). The **runtime "+" save-named-theme UI is OUR addition** (a Gboard / SwiftKey-class pattern), not an open-source KK feature. Do not cite KK as the source for the runtime CRUD.
- iOS picker = a SwiftUI re-implementation of the Shelf layout; Android = a Compose `LazyColumn` of horizontally-scrolling `LazyRow` shelves.

### Layer C — apply architecture: existing 6-role resolver, NOT a new engine

- **Deliberately NOT adopted** (YAGNI): FlorisBoard Snygg stylesheet engine + per-element selectors + addon store; **and** KeyboardKit Pro's theme engine (license + closed-source).
- **Adopted**: a theme (built-in OR user-created) = a fully-resolved `KeyboardColorSettings` (6 roles), chosen from a static table or the user-theme list. No new render path.

---

## 4. Storage & resolve model — theme-id resolve (DECIDED 2026-06-05)

**Decision**: store a `selectedThemeId`; resolve the 6 roles — and the light/dark variant — at render time. The id space spans `default`, built-in palette ids, and **user-created theme UUIDs** (§4b).

### 4.1 Theme types

```
// Built-in (static, read-only)
BuiltInTheme {
  id: String              // "catppuccin", "gruvbox", "tokyoNight", "solarized", "nord", "florisDefault"
  displayName: String     // 小標題 on the shelf
  light: SixRoleColors?   // nil if no light variant
  dark:  SixRoleColors?   // nil if no dark variant
}

// User-created (persisted, CRUD) — see §4b
UserTheme {
  id: UUID                // stable, survives updates
  name: String            // user-given; shown as 小標題
  colors: SixRoleColors   // internal still 6 roles (the editor exposes fewer — §4b.2)
  keyShadow: ShadowSpec   // NEW render property (§4b.5); default none (flat)
  createdAt / updatedAt
}

SixRoleColors = { background, keyText, normalKeyFill, specialKeyFill, candidateText, candidateBackground }
ShadowSpec    = { intensity: 0.0…1.0 }   // 0 = flat. DECIDED: intensity-only, color derived (no color picker). §6 H

// Editor (USER 2026-06-07: NOT simplified) edits all 6 roles directly + the shadow slider.

// Selection
selectedThemeId: String   // "default" | built-in id | UserTheme UUID
```

- **`default`** = today's nil-fallback (platform / KeyboardKit adaptive). Users who never customize get this.
- **Built-in themes keep the full authored 6-role `SixRoleColors`** (richer depth; §5). The **user-theme editor** writes all 6 roles directly (plus the shadow control, §4b.2), so the renderer path is identical for built-in and user themes.
- **Render-time resolve**: built-in → pick `light`/`dark` by system `colorScheme` (fallback to the other if nil, e.g. Nord dark-only). `default` → platform adaptive. user theme → its `SixRoleColors` + `keyShadow` (light/dark per §6 E).

### 4.2 Persistence

- **New key** `selectedThemeId: String` (iOS `SettingsKey<String>` in `SharedSettings`; Android DataStore key). Default `default`.
- **User themes list** — durable store, see §4b.3.
- **Existing `colorSettings` blob** — repurposed as the *editor scratch / current-edit buffer*; saved edits become `UserTheme`s.

### 4.3 Migration (lightweight, non-destructive)

- Existing users with a customized `colorSettings` blob → seed **one** `UserTheme` named e.g. "我的主題" from it (all 6 roles preserved → **lossless**; the editor exposes all 6 directly), set `selectedThemeId` to that UUID → their current look is preserved.
- Users with the default `{}` blob → `selectedThemeId = default`, no user theme created.
- Pure additive keys; no destructive rebuild. Mirrors the non-destructive migration discipline across v3.6.1 (`.claude/rules/taigi-incidents.md` S11–S15).

### 4.4 Why this resolves two forks

- **Fork B (light/dark)**: solved — the pair lives in the static table (built-ins) / per-user-theme, not in a single flat blob. System appearance switches the variant at render time.
- **Fork D (active-preset identity)**: solved — `selectedThemeId` *is* the persisted identity; the shelf highlights it directly.

### 4.5 Cross-platform parity invariant (mandatory)

- The static built-in table (ids, names, every hex) **and** the user-theme serialization schema must be **identical** on iOS and Android. (No `.taigi` theme format — cross-device theme backup dropped 2026-06-07.) Built-in table now up to 12 colors/theme (6 light + 6 dark) → **raises Fork C option P2 (shared JSON)** over hand-mirroring. See §6 fork C.

---

## 4b. User-created themes — the "+" flow (DECIDED 2026-06-05)

USER: 「有一個 + 號,可以替自己自定義的主題取名、儲存,不會因為更新不見,可以自定義多個主題套用」.

### 4b.1 CRUD surface

- The Shelf shows: **Default** + built-in themes + **the user's saved themes** + a trailing **"+"** item.
- **"+"** → opens the **editor** (§4b.2: 6 color rows + a shadow slider + live `KeyboardPreviewPanel`) with a **name field** → **Save** → appends a `UserTheme`, selects it.
- A saved user theme's detail / long-press → **Rename / Duplicate / Delete**.
- **Duplicate-a-built-in to edit**: selecting a built-in → "Duplicate" → creates an editable `UserTheme` pre-filled from that built-in's resolved colors (KeyboardKit-like "start from a base theme"). Built-ins themselves stay read-only.

### 4b.2 Editor — full 6-role free-pick + a shadow slider (USER 2026-06-05, revised 2026-06-07)

USER 2026-06-07: 「編輯器不需要簡化」. The editor keeps the **full 6-role free-pick** — the existing 6 color pickers reused as-is — plus an independent key-shadow slider:

| Editor control | Internal field |
|---|---|
| **背景 Background** | `background` |
| **候選列背景 Candidate background** | `candidateBackground` |
| **一般按鍵 Normal key** | `normalKeyFill` |
| **特殊按鍵 Special key** | `specialKeyFill` |
| **按鍵文字 Key text** | `keyText` |
| **候選文字 Candidate text** | `candidateText` |
| **按鍵陰影 Key shadow** | `keyShadow.intensity` slider, 0 = flat (intensity-only, no color — fork H1) |

- **Editor NOT simplified** (USER 2026-06-07): the earlier 3-color merge (Background = bg+candidate, Key = normal+special, Text = key+candidate) is **dropped**; all 6 roles are edited individually, matching the built-in tables 1:1.
- **Shadow kept as an independent feature** (USER 2026-06-07): the key-shadow slider is NOT tied to any fill merge — it is a standalone elevation control on top of the 6 roles (§4b.5). Default 0 (flat).
- Reuses the existing `AppearanceSettingsView` / `ColorSettingRow` 6-picker layout **verbatim** + a name field + Save/Cancel + the shadow slider. Live preview already wired.
- **Internal model = 6 roles + shadow** (§4.1); renderer unchanged; built-in themes (authored at full 6-role, shadow 0) coexist.

### 4b.5 Key shadow — new render property (design-stance departure)

- **Project is currently flat / no-shadow** (`docs/ui/theme.md` "Flat design: no shadows, uses borders"). Adding an adjustable key shadow is a departure — **sanctioned by USER for custom themes only**. `default` + all **built-in** themes keep `keyShadow.intensity = 0` (flat look preserved).
- **iOS feasibility (grounded, doc-lookup 2026-06-05)**: KeyboardKit `Keyboard.ButtonStyle.shadow: ShadowStyle?` (`ButtonShadowStyle` = color + size, `references/keyboardkit9.9.0/.../Keyboard+ButtonStyle.swift:139,197` + `Keyboard+ButtonShadow.swift`). Driven via the `keyboardButtonStyle { }` closure we already use (`TaigiKeyboardView.swift:318-350`).
- **Android feasibility**: keys are custom-drawn (`KeyContent.kt`) → shadow via `Paint.setShadowLayer` / a shadow layer in the draw pass. Confirm at impl.
- **New cross-platform surface**: `keyShadow` is a parity field (schema + intensity→render mapping mirror iOS/Android; not in `.taigi` — theme backup dropped). Pin an invariant when implemented.

### 4b.3 Durability — two layers (answer to "不會因為更新不見")

| Scenario | Auto-preserved? | Mechanism |
|---|---|---|
| **App update** (same device) | ✅ automatic | UserDefaults / DataStore / files / SQLite live in the app container; updates do not wipe them (§2.2). Core requirement met. |
| **Reinstall / new device** | ❌ not carried (DROPPED 2026-06-07) | **DECIDED (USER 2026-06-05, fork F-Exclude; shipped PR-A #404)**: user themes are **excluded from OS auto-backup**, uniform with the 3 user-data DBs (R7 posture). **Cross-device `.taigi` theme export was DROPPED (USER 2026-06-07)** — themes are **local-only**; reinstall / new device does NOT carry them. ⚠ UX footgun (same as S16): the backup/export UI copy must never promise OS / iCloud restore for themes. |

**Storage location** — constrained by the F-Exclude decision:
- A small bounded list (≤ 5 × 12 colors + name = tiny) → a **JSON-encoded list**, NOT a new SQLite table (YAGNI — no query/relational need).
- ⚠ **OS-backup exclusion drives the location** (code-grounded vs R7): iOS marks individual **files** excluded via `isExcludedFromBackup` (R7 did this on the SQLite DBs, `SQLiteConnectionManager.connect()`). **App Group UserDefaults plist is backed up wholesale and cannot be selectively excluded** — so on iOS the theme JSON must live in a **standalone file in the App Group container** marked `isExcludedFromBackup`, **not** a `SettingsKey`/UserDefaults key. Android: any file under app storage is already excluded by the existing `allowBackup="false"` (R7) — DataStore / a JSON file both fine.
- ~~Added to `.taigi` serialization~~ **DROPPED (USER 2026-06-07)**: themes stay local-only, not in `.taigi`.

### 4b.4 Constraints

- **Count cap**: **5** user themes (DECIDED USER 2026-06-07); mirror the iOS/Android cap-parity pattern (S14 `CustomDictionaryCapacityPolicy`). At-cap "+" → disabled / a "max 5" notice.
- **Name**: non-empty; duplicates allowed (UUID is the identity, name is a label) or de-duplicated — minor, decide at impl.
- **Light/dark editing** (fork §6 E): MVP single color set (both modes) vs editor toggle to define light + dark separately.

---

## 5. Theme sets (starting curation) — built-in vim colorschemes with light/dark

Built-in, read-only sets. Each = a 小標題 + light and/or dark variant. Hex mapped to the 6 roles with this rule: **`background` darkest → `specialKeyFill` mid → `normalKeyFill` lightest** (letters most raised, matches platform default where special keys are darker/muted). Light variants: letter keys near-white, special keys gray, bg the palette's light base. `accent` is reserved (not a role today; used only if §6 fork A widens chrome, or for the §19 first-candidate keycap hint).

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

> Nord has no official light theme. Ship dark-only (variant fallback handles both modes) or author a "Nord Light" (snow-storm tones). Flagged for USER.

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

### Plus: `Default` (reserved id) + user themes

`Default` = platform adaptive (today's nil-fallback). User-created themes (§4b) appear alongside built-ins in the shelf.

### Keyword-list extras (deferred — license + design check)

`Tron` / `Nothing` / `Windows Phone` / `Oblivion` from the USER's original list: not single canonical palettes (Tron/Nothing are aesthetics needing original interpretation) and need per-source license checks. Deferred until USER confirms which to include.

> All hex values above are derived from canonical palettes mapped to 6 roles — **final values gated on device dogfood** for contrast/legibility (`code-review-rules.md §9`).

---

## 6. Open design forks

### Decided 2026-06-05

- **Nav fork → A**: appearance/theme = its own top-level tab, **placed 2nd** (after 頭頁; USER 2026-06-07) (§8.1).
- **Storage fork → theme-id resolve** (§4) — closes Fork B (light/dark) + Fork D (active identity).
- **User themes → yes**: multiple named, saved, update-durable, applyable, via "+" (§4b).
- **Fork F → F-Exclude** (shipped PR-A #404): user themes excluded from OS auto-backup. **Cross-device `.taigi` theme export DROPPED (USER 2026-06-07)** — themes local-only (§4b.3, top banner).
- **Fork E → E1 working default** (USER may override): single value, both modes.
- **Editor → full 6-role free-pick + shadow slider** (§4b.2, revised 2026-06-07): NOT simplified (3-color + text merge dropped); all 6 roles edited individually + an independent key-shadow intensity slider (fork H1). Built-ins keep authored 6-role + shadow 0.

### Fork A (open) — preset coverage: 6 roles only, or widen the override surface?

- **A1 (MVP)**: themes recolor only the 6 roles; pressed/popup/enter/emoji/smartbar chrome stays platform-default → may clash with strongly-tinted themes.
- **A2 (full theme)**: make chrome overridable (Android route `getColorFromAttr`/`KeyboardChromeColors.from` through a palette layer; iOS extend KeyboardKit style closures). Larger.
- Recommendation: **start A1**, dogfood, widen to A2 only if themes look broken.

### Fork C (open) — theme table source of truth: hand-mirror (P1) or shared JSON (P2)?

- §4 enlarges the built-in table; recommendation leans **P2 (shared JSON parsed by both)**; USER to confirm.

### Fork E — user-theme light/dark editing (E1 working default, USER may override)

- **E1 (MVP, adopted default)**: a user theme stores one `SixRoleColors`, applied in both modes (matches today's free-pick). Simpler editor.
- **E2 (deferred)**: editor toggle to define light + dark separately (parity with built-ins' auto-switch). Follow-up.

### Fork F — user-theme OS-backup policy → **F-Exclude (DECIDED, USER 2026-06-05)**

- v3.6.1 R7 excluded the 3 user-data DBs from OS auto-backup. Although themes are colors + names (not privacy-sensitive), USER chose **uniform policy**: user themes are **excluded from OS auto-backup**. **Cross-device `.taigi` theme export DROPPED (USER 2026-06-07)** → themes are local-only (no cross-device path).
- **Implementation** (§4b.3, shipped PR-A #404): iOS stores the theme JSON in a standalone App Group file marked `isExcludedFromBackup` (UserDefaults plist can't be selectively excluded); Android is already covered by `allowBackup="false"`. ~~Themes join `.taigi` export~~ DROPPED. UI copy must not promise OS/iCloud restore (S16 footgun).

### Fork G → **5 (DECIDED, USER 2026-06-07)**

- User-theme count cap = **5**; mirror the cap-parity pattern (S14). At-cap "+" disabled / "max 5" notice. Android mirrors the same cap when it lands.

### Fork H — key-shadow controls → **H1 (DECIDED, USER 2026-06-05)**

- **H1 (adopted)**: a single **intensity** slider (0 = flat); shadow color derived (dark / from text color). No color picker.
- ~~H2 (shadow color picker)~~ — not adopted (YAGNI).

### Editor scope → **NOT simplified (DECIDED, USER 2026-06-07)**

- USER 「編輯器不需要簡化」: the editor keeps the **full 6-role free-pick**; the earlier 3-color merge + text-merge (`keyText`+`candidateText` into one) are **dropped** — all 6 roles are edited individually. The key-shadow slider stays as an **independent** feature (fork H1).

---

## 7. Image upload as custom theme — DROPPED (USER 2026-06-07, 暫時不需要)

**Entire section removed from the plan** — neither image path is being built. Re-open only on explicit USER revive. Historical feasibility notes (I-1 palette-extract = HIGH feasibility, host-app-side; I-2 background-image = LOW/iOS-64MB-gated) preserved in git history.

---

## 8. Information architecture & phases

### 8.1 Navigation — dedicated tab, placed 2nd (DECIDED: fork A 2026-06-05; position 2026-06-07)

Appearance/theme promoted to its own top-level tab. Result: **5 tabs** (at the iOS / Android Material-3 NavigationBar practical maximum — acceptable, no room for a 6th). **Tab order (USER 2026-06-07): 頭頁 → 主題 → 齒佈 → 詞庫 → 設定** — theme sits at **position 2 (index 1)**, immediately after Home.

- **iOS**: insert `case theme = 1` into `TabType` (`TabType.swift`) and **renumber** `layout = 2 / dictionary = 3 / settings = 4` (the enum's `Int` rawValue *is* the tab index — `TabType.swift` comment "Int rawValue 同時是 tab 索引"). Add the `tabItem` at slot 1 in `ContentView.swift`, icon `paintpalette.fill` (or `paintbrush.fill`), localized title via a new `ThemeTexts`. Move `AppearanceSettingsView` out from under `LayoutTab` into a new `ThemeTab`.
- **⚠ Index-shift hazard**: renumbering rawValues breaks any **persisted "last-selected tab index"** and any **deep-link constant** (`.switchToSettingsTab`-style, Home quick-links) that hard-codes the old `layout=1 / dictionary=2 / settings=3`. Grep every `TabType(rawValue:)` / raw `selectedTab = N` / deep-link before landing P1; bump each. A persisted index from a prior install now points one tab to the left → migrate or reset to `.home`.
- **Android**: add a `theme` destination at NavigationBar slot 1 in `MainSettingsScreen.kt` + a `ui/tabs/theme/` package, mirroring `ui/tabs/layout/`. Move `AppearanceSettingsScreen` content into it. Update any nav-route ordinal / saved-state route key the same way.
- **Layout tab retains** keyboard layout + font.
- **Cross-platform parity**: tab order (頭頁→主題→齒佈→詞庫→設定), icon semantics, title mirror (capture in `ui-style-guide.md` at impl).
- **Trade-off recorded**: spends a scarce nav slot — now in **prime position 2** — on a set-and-forget feature, running against `ui-style-guide.md` §Feature Grouping by Usage Frequency. USER accepted for discoverability / showcase value (appearance is a keyboard-app selling point; SwiftKey/Gboard surface themes prominently); the 2nd-position placement maximizes that showcase intent.

### 8.2 Phase table — **iOS-first implementation (USER 2026-06-07: 「先對 iOS 做一版來測試」)**

**Platform order**: build the full iOS slice first (testable on device), then mirror to Android via `/port-feature`. iOS is the color-model source-of-truth (`KeyboardColorSettings.kt:14` "Matches iOS"). ⚠ Android key-shadow draw-pass (`Paint.setShadowLayer`) is the one unverified feasibility — spike early.

Sizing targets 200–500 LOC/PR per `~/.claude/rules/planning.md`.

| Phase | Scope | Depends on |
|---|---|---|
| P0 (admin) | This doc + roadmap entry + memory file | — |
| P1 | Dedicated theme tab scaffolding both platforms — insert at **index 1** (renumber layout/dictionary/settings + fix deep links), move appearance out of Layout, no behavior change | — |
| P2 | Theme-id storage model + resolver (`selectedThemeId`, `default`, migration) | — |
| P3 | Built-in static theme table (resolve Fork C) + curated hex (resolve Fork A coverage) | P2 |
| P4 | Shelf picker UI both platforms + live preview + light/dark resolve | P1, P2, P3 |
| P5 | User-theme CRUD: "+" create / name / save / rename / delete / duplicate + persistence (local-only, no `.taigi`) (resolve Forks E/F/G) | P2, P4 |
| ~~P6~~ | ~~Image → palette extraction~~ **DROPPED 2026-06-07** | — |
| (deferred) | A2 full-chrome override / E2 light-dark editing / keyword-list extra themes | user-gated later |

---

## 8b. iOS implementation plan (grounded in code + Codex pre-impl 2026-06-07)

iOS-first (USER 2026-06-07). Codex pre-impl review (gpt-5.5, ANALYSIS-ONLY) corrected the seam + phasing. **PR-2 split into 2a/2b.**

### Grounded render path (actual code)
- `SharedSettings.colorSettings` (App Group UserDefaults, `.codable`, default `.default`=all-nil) → `KeyboardColorSettings` (6 OPTIONAL `CodableColor?`; nil = KK adaptive fallback; light+dark share one RGBA).
- Consumers (ALL must read the resolved value — Codex Q1): `TaigiKeyboardView` `RenderProviders.keyTextColor` (`:74`), root `.background()` (`:141-144`), liquid-glass gate `backgroundColor==nil` (`:106`), `.keyboardButtonStyle{}` normal/special fill (`:318-344`), `candidateStyle` (`:358-364`), `CandidateTheme.resolved` (`Autocomplete/Models/CandidateTheme.swift:48-65`). Live-read re-reads every render (`:152-154`).
- Editor today: `App/Tabs/Layout/AppearanceSettingsView.swift` + `…ViewModel.swift` (6 ColorPicker rows over `settings.colorSettings`), preview via `App/Tabs/Layout/KeyboardPreviewPanel.swift` (already syncs `colorScheme` via `onChange`, `:44`).
- Nav: `App/Tabs/TabType.swift` enum; `ContentView.swift` body order = tab order; `.tag(TabType.x)` enum identity; deep-link `.switchToSettingsTab` sets `selectedTab=.settings` (enum). `selectedTab` `@State`, NOT persisted. No `TabType.allCases[rawValue]` / rawValue-index usage (grepped) → renumber safe.

### Type design (Codex Q5/Q1/Q8)
- `Settings/KeyboardThemeModels.swift` (Platform layer): `ThemeId` ("default" | builtin id | UUID), `BuiltInTheme {id, displayName, light: KeyboardColorSettings?, dark: KeyboardColorSettings?}` (concrete-color pairs), `UserTheme {id:UUID, name, colors: KeyboardColorSettings (OPTIONAL per-field — preserves "2/6 customized" migration), keyShadow:{intensity:Double}, timestamps}`, `ResolvedKeyboardTheme {colors: KeyboardColorSettings, keyShadowIntensity: Double}`.
- `Settings/ThemeResolver.swift`: `(selectedThemeId, colorScheme) → ResolvedKeyboardTheme`. **`default` special-cases to `KeyboardColorSettings.default` (all nil) + intensity 0** — preserves Liquid Glass (Codex Q3). Invalid/missing id → fallback `default` (+ repair). Built-in → pick light/dark by colorScheme; Nord dark-only → fallback the other.
- `Settings/UserThemeStore.swift`: standalone App Group JSON file, `isExcludedFromBackup` (F-Exclude; plist can't selectively exclude). CRUD + cap 5. **Bumps a `themeRevision` UserDefaults key on every mutation** so the extension refreshes when the active theme is edited (selectedThemeId unchanged — Codex Q4). Write order: write file → then set selectedThemeId; delete active → write list → then fallback.
- Place theme core in `Settings/` (Platform), NOT `App/` — `KeyboardExtension/` can't depend on App layer (Codex Q8). UI in `App/Tabs/Theme/`.

### Render seam (Codex Q1/Q2)
- `TaigiKeyboardView` body top computes ONE `resolved = ThemeResolver.resolved(selectedThemeId, colorScheme)` (colorScheme via `@Environment` / `keyboardContext.colorScheme`), feeds `resolved.colors` to every consumer above + `resolved.keyShadowIntensity` to the button-shadow path. Resolve in body (not only on UserDefaults notification) so a system light/dark switch re-renders. Mirror in `KeyboardPreviewPanel`.
- Shadow render: KeyboardKit `Keyboard.ButtonStyle.shadow`/`ButtonShadowStyle` inside the existing `keyboardButtonStyle{}` closure; built-ins/default intensity 0.

### Phasing (revised — each its own PR/branch)
| PR | Scope | Testable |
|---|---|---|
| **iOS PR-1** | Theme tab scaffold at **position 2** (TabType `theme=1` + renumber, `ThemeTexts`, `ContentView` insert, new `App/Tabs/Theme/ThemeTab` hosting moved `AppearanceSettingsView`). No behavior change. | Tab appears 2nd, appearance editor works there |
| **iOS PR-2a** | Theme **core**: models + `UserThemeStore` (file r/w + `isExcludedFromBackup` + `themeRevision`) + `selectedThemeId` + `ThemeResolver` (default→nil special-case, invalid→fallback) + **migration** (seed one UserTheme from non-default `colorSettings`) + **renderer switch** (all consumers read resolved) + fallback/migration tests. NO shelf. | Existing custom colors survive (migration); default users unchanged (Liquid Glass intact) |
| **iOS PR-2b** | Built-in static table (6 sets hex, §5) + Shelf picker UI (`App/Tabs/Theme/**`) + live mini-keyboard previews + apply-built-in (light/dark resolve). | **Pick a built-in → keyboard recolors** (real test version) |
| **iOS PR-3** | User CRUD ("+"/name/save/rename/delete/duplicate) + full 6-role editor + shadow slider + cap **5** (local-only, no `.taigi` — DROPPED). Active edit bumps `themeRevision`. | Create/save/apply user themes (local-only) |
| → Android | Mirror via `/port-feature` after iOS dogfood. ⚠ spike `Paint.setShadowLayer` early. | — |

**Hard ordering (Codex Q7)**: migration + renderer-switch MUST land together in PR-2a (else existing custom-color users revert to default). UserThemeStore can't be deferred past PR-2a (migration seeds a UserTheme).

---

## 9. Best-practices alignment (per `~/.claude/rules/planning.md`)

| 主流做法 | 來源 | 本 plan 對應 |
|---|---|---|
| Theme picker as scrolling shelves of live previews | KeyboardKit `Themes-Article.md:179-194` (`KeyboardTheme.Shelf` / `ShelfItem`) | §3 Layer B, §8.1, P4 |
| Theme = set + style variations | KeyboardKit `Themes-Article.md:71,160` (`.standard`+`.blue`; `.tron`+`.virus`) | §4 types, §5 sets |
| Custom theme based on a base theme | KeyboardKit `Themes-Article.md:227-242` (start from `.minimal`, tweak) | §4b.1 duplicate-to-edit |
| Persist a selected-theme identity | KeyboardKit `KeyboardThemeContext` (Themes-Article.md:28-32) | §4.2 `selectedThemeId` |
| Runtime user-created named themes (CRUD + "+") | Gboard / SwiftKey custom-theme pattern (NOT KK open-source) | §4b |
| Semantic color tokens (`@defines`) | FlorisBoard `stylesheets/floris_day.json:3-31` | §4.1 `SixRoleColors` |
| Curated colorscheme palettes | Catppuccin / Tokyo Night / Gruvbox / Solarized / Nord (mostly MIT) | §5 |

**Project rules cited**: `cross-platform-alignment.md` (built-in table + user-theme schema + tab order are parity surfaces, §4.5/§4b.3/§8.1); CLAUDE.md Core Principle #2/#5/#6; `taigi-incidents.md` (non-destructive migration §4.3; cap-parity S14 §4b.4; R7 backup posture §6 F); `ui-style-guide.md` §Feature Grouping (the §8.1 trade-off); `code-review-rules.md §9` (qualitative dogfood gate).

**刻意不採用 (deliberately not adopted)**:
- **KeyboardKit Pro theme engine** — Pro-gated + closed-source; reuse only its Shelf UI *design* (§3 Layer B).
- **FlorisBoard Snygg stylesheet engine / addon store** — YAGNI (§3 Layer C).
- **A new SQLite table for user themes** — YAGNI; a JSON list in settings suffices for a small bounded set (§4b.3).
- **Image upload as a theme source (I-1 + I-2)** — DROPPED (USER 2026-06-07, §7); not built.
- **Cross-device theme backup / `.taigi` theme export** — DROPPED (USER 2026-06-07); themes local-only (§4b.3).
- **Per-role day/dark flat storage (fork B2)** — unnecessary; the theme-id model keeps pairs in the table / per-theme (§4.4).

---

## 10. Open questions / TODO (USER to append)

- [x] Nav structure — **A: dedicated tab, placed 2nd** (頭頁→主題→齒佈→詞庫→設定) (2026-06-05; position 2026-06-07)
- [x] Storage model — **theme-id resolve** (2026-06-05)
- [x] User-created themes — **yes, "+" CRUD, update-durable, multiple** (2026-06-05)
- [x] Fork F — user-theme OS-backup → **F-Exclude** (excluded from OS backup, uniform with user-data DBs) (2026-06-05; shipped PR-A #404). Cross-device `.taigi` theme export DROPPED 2026-06-07.
- [x] Fork E — user-theme light/dark → **E1 single value** working default (E2 deferred; USER may override)
- [x] Editor scope → **full 6-role free-pick, NOT simplified** (3-color + text merge dropped) (revised 2026-06-07)
- [x] Editor text → **keep keyText / candidateText separate** (merge reverted) (2026-06-07)
- [x] Fork H — key-shadow → **intensity-only**, color derived; kept as an **independent** feature (2026-06-05; reaffirmed 2026-06-07)
**Deferred to implementation-time review (USER 2026-06-05: 「先記錄,之後真正要實作的時候再 review」)** — the items below carry a *recommended working default* but are NOT decided. Re-confirm with USER when the relevant phase starts; do not treat as committed (Core Principle #5).

- [ ] Fork A — coverage. **Lean: A1** (recolor the 6 roles only; chrome stays platform-default). Revisit if dogfood shows clashing. Decide at P3.
- [ ] Fork C — built-in table source of truth. **Lean: P2** (shared JSON parsed by both; drift-proof). Decide at P3.
- [x] Fork G — user-theme count cap → **5** (DECIDED USER 2026-06-07; mirror S14 cap-parity).
- [ ] Nord light variant. **Lean: ship dark-only** (no official Nord light; don't fabricate). Decide at P3.
- [ ] Keyword-list extras (Tron / Nothing / Windows Phone / Oblivion). **Lean: not included** for v3.6.2 (not single canonical palettes; non-MIT licensing). Add individually only if USER wants a specific one (then license + palette work). Decide at P3.
- [x] Image upload (palette extract / background image). **DROPPED (USER 2026-06-07, 暫時不需要)** — off the plan (§7).
- [x] Cross-device theme backup (`.taigi` theme export-import). **DROPPED (USER 2026-06-07)** — themes local-only (§4b.3).
- [ ] Tab icon + title. **Lean: `paintpalette.fill` + 「主題」**. Decide at P1.
- [ ] (USER additions below)

---

## Appendix — code-grounding citations (read 2026-06-05, READ-ONLY)

**iOS**: `Settings/KeyboardColorSettings.swift:10-58` · `Autocomplete/Models/CandidateTheme.swift:46-73` · `App/Tabs/Layout/AppearanceSettingsView.swift:44-243` · `AppearanceSettingsViewModel.swift:19-162` · `SharedSettings.swift:99,502-505,572` · `App/Tabs/TabType.swift:7-34` · `App/ContentView.swift:16-65`.

**Android**: `ime/core/KeyboardColorSettings.kt:14-54` · `ime/theme/ThemeAttributeColors.kt:9-16` · `ime/text/KeyboardAppearanceResolver.kt:47,56-63` · `ime/text/smartbar/KeyboardChromeColors.kt:28-47` · `ui/tabs/layout/AppearanceSettingsScreen.kt:80-392` · `ui/tabs/layout/ColorPickerDialog.kt:73-280` · `ui/tabs/MainSettingsScreen.kt` (NavigationBar) · `PrefHelper.kt:84,500`.

**KeyboardKit reference** (9.9.0, doc-lookup 2026-06-05): `references/keyboardkit9.9.0/Sources/KeyboardKit/KeyboardKit.docc/Features/Themes-Article.md:14-255` (Shelf/ShelfItem, ThemeContext, predefined themes incl. `.tron`, code-level custom themes); `_Pro/ProPlaceholders.swift` (engine is Pro-gated).

**FlorisBoard reference**: `references/florisboard/.../org.florisboard.themes/{extension.json, stylesheets/floris_day.json, floris_night.json}`.
