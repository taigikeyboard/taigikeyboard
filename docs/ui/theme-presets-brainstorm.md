# Theme Presets & Custom Themes — Brainstorm (v3.6.2)

> **Type**: Planning (brainstorm — evolving; USER will append ideas)
> **Keywords**: `theme`, `preset`, `palette`, `colorscheme`, `theme set`, `shelf`, `custom theme`, `user theme`, `image upload`
> **Status**: Brainstorm — NO code. Decided (USER 2026-06-05): **5th nav tab**, **theme-id resolve storage model**, **user-created named themes (the "+" flow)**. Remaining forks open.
> **Version scope**: v3.6.2 (USER-scoped 2026-06-05: 「這個列為 v3.6.2 的計劃」)
> **Related**: `docs/ui/theme.md` (current-state reference), `docs/roadmap.md` (deferred TODO "keyboard theme picker")

---

## Summary

- **Goal**: ship predefined **theme sets** (each = a named palette with a 小標題, bundling **light + dark** variants), let users **create / name / save multiple custom themes** (a KeyboardKit-style **"+"** flow that survives app updates), present everything in a KeyboardKit-Shelf-style picker on its **own 5th nav tab**, and evaluate **user-uploaded image** as a custom-theme source.
- **Current state is NOT greenfield** — both platforms already have a mirrored 6-role free-pick color system (`KeyboardColorSettings`). v3.6.2 shifts storage from "store 6 raw colors" to "**store a `selectedThemeId`, resolve 6 roles (and light/dark) at render time**", and adds a **persisted list of user-created themes**. The existing 6-picker UI becomes the theme **editor** reached via "+".
- **Layered design**: palette values from established editor/vim colorschemes (Catppuccin, Tokyo Night, Gruvbox, Solarized, Nord); the **picker UX** references KeyboardKit's `KeyboardTheme.Shelf`; we **deliberately do NOT use** KeyboardKit's theme *engine* (Pro-gated) or FlorisBoard's Snygg stylesheet engine + addon store.
- **USER decisions 2026-06-05**: (1) **Nav** — appearance/theme → its own top-level tab (4 → 5); (2) **Storage** — theme-id resolve model (auto-solves light/dark + active-identity forks); (3) **User themes** — multiple named, saved, update-durable, applyable themes via "+".
- **Remaining open forks**: preset coverage (6 roles vs full chrome, §6 A), theme-table source-of-truth (hand-mirror vs shared JSON, §6 C), **user-theme light/dark editing** (single vs pair, §6 E), **user-theme OS-backup policy** (exclude like other user-data vs allow iCloud restore, §6 F), user-theme count cap (§6 G), image-upload scope (§7), final roster + licensing (§5).

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
  colors: ThemeColors     // single SixRoleColors, or {light,dark} pair — fork §6 E
  createdAt / updatedAt
}

SixRoleColors = { background, keyText, normalKeyFill, specialKeyFill, candidateText, candidateBackground }

// Selection
selectedThemeId: String   // "default" | built-in id | UserTheme UUID
```

- **`default`** = today's nil-fallback (platform / KeyboardKit adaptive). Users who never customize get this.
- **Render-time resolve**: built-in → pick `light`/`dark` by system `colorScheme` (fallback to the other if nil, e.g. Nord dark-only). `default` → platform adaptive. user theme → its colors (light/dark per §6 E).

### 4.2 Persistence

- **New key** `selectedThemeId: String` (iOS `SettingsKey<String>` in `SharedSettings`; Android DataStore key). Default `default`.
- **User themes list** — durable store, see §4b.3.
- **Existing `colorSettings` blob** — repurposed as the *editor scratch / current-edit buffer*; saved edits become `UserTheme`s.

### 4.3 Migration (lightweight, non-destructive)

- Existing users with a customized `colorSettings` blob → seed **one** `UserTheme` named e.g. "我的主題" from it, set `selectedThemeId` to that UUID → their current look is preserved as a saved theme.
- Users with the default `{}` blob → `selectedThemeId = default`, no user theme created.
- Pure additive keys; no destructive rebuild. Mirrors the non-destructive migration discipline across v3.6.1 (`.claude/rules/taigi-incidents.md` S11–S15).

### 4.4 Why this resolves two forks

- **Fork B (light/dark)**: solved — the pair lives in the static table (built-ins) / per-user-theme, not in a single flat blob. System appearance switches the variant at render time.
- **Fork D (active-preset identity)**: solved — `selectedThemeId` *is* the persisted identity; the shelf highlights it directly.

### 4.5 Cross-platform parity invariant (mandatory)

- The static built-in table (ids, names, every hex) **and** the user-theme serialization schema (+ `.taigi` format) must be **identical** on iOS and Android. Built-in table now up to 12 colors/theme (6 light + 6 dark) → **raises Fork C option P2 (shared JSON)** over hand-mirroring. See §6 fork C.

---

## 4b. User-created themes — the "+" flow (DECIDED 2026-06-05)

USER: 「有一個 + 號,可以替自己自定義的主題取名、儲存,不會因為更新不見,可以自定義多個主題套用」.

### 4b.1 CRUD surface

- The Shelf shows: **Default** + built-in themes + **the user's saved themes** + a trailing **"+"** item.
- **"+"** → opens the **editor** (reuses today's 6 `ColorPicker` rows + live `KeyboardPreviewPanel`) with a **name field** → **Save** → appends a `UserTheme`, selects it.
- A saved user theme's detail / long-press → **Rename / Duplicate / Delete**.
- **Duplicate-a-built-in to edit**: selecting a built-in → "Duplicate" → creates an editable `UserTheme` pre-filled from that built-in's resolved colors (KeyboardKit-like "start from a base theme"). Built-ins themselves stay read-only.

### 4b.2 Editor = the existing 6-picker UI

No new color-editing UI — `AppearanceSettingsView` (iOS) / `ColorPickerDialog` + `ColorSettingRow` (Android) become the theme editor body, plus a name field and Save/Cancel. Live preview is already wired.

### 4b.3 Durability — two layers (answer to "不會因為更新不見")

| Scenario | Auto-preserved? | Mechanism |
|---|---|---|
| **App update** (same device) | ✅ automatic | UserDefaults / DataStore / files / SQLite live in the app container; updates do not wipe them (§2.2). Core requirement met. |
| **Reinstall / new device** | ⚠ policy-dependent | per v3.6.1 R7, user-data DBs are **excluded from OS auto-backup**; manual `.taigi` is the cross-device path. User themes are user content → must **join the `.taigi` export**. OS-backup inclusion is fork §6 F. |

**Storage location (recommendation)**: a small bounded list (≤ a few dozen × 12 colors + name = tiny). Recommend a **JSON-encoded list in settings storage** (iOS App Group UserDefaults `SettingsKey<[UserTheme]>`; Android DataStore) over a new SQLite table (YAGNI — no query/relational need). It **must** be added to `.taigi` serialization for reinstall/new-device survival (parity surface).

### 4b.4 Constraints

- **Count cap** (defensive, fork §6 G): suggest ~50 user themes; mirror the iOS/Android cap-parity pattern (S14 `CustomDictionaryCapacityPolicy`).
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

- **Nav fork → A**: appearance/theme = its own 5th top-level tab (§8.1).
- **Storage fork → theme-id resolve** (§4) — closes Fork B (light/dark) + Fork D (active identity).
- **User themes → yes**: multiple named, saved, update-durable, applyable, via "+" (§4b).

### Fork A (open) — preset coverage: 6 roles only, or widen the override surface?

- **A1 (MVP)**: themes recolor only the 6 roles; pressed/popup/enter/emoji/smartbar chrome stays platform-default → may clash with strongly-tinted themes.
- **A2 (full theme)**: make chrome overridable (Android route `getColorFromAttr`/`KeyboardChromeColors.from` through a palette layer; iOS extend KeyboardKit style closures). Larger.
- Recommendation: **start A1**, dogfood, widen to A2 only if themes look broken.

### Fork C (open) — theme table source of truth: hand-mirror (P1) or shared JSON (P2)?

- §4 enlarges the built-in table; recommendation leans **P2 (shared JSON parsed by both)**; USER to confirm.

### Fork E (open) — user-theme light/dark editing: single value, or editable pair?

- **E1 (MVP)**: a user theme stores one `SixRoleColors`, applied in both modes (matches today's free-pick).
- **E2**: editor toggle to define light + dark separately (parity with built-ins' auto-switch).
- Recommendation: **E1 for MVP** (simpler editor), E2 a follow-up. USER to confirm.

### Fork F (open) — user-theme OS-backup policy

- v3.6.1 R7 excluded the 3 user-data DBs from OS auto-backup **for privacy** (typed-text-derived data). **User themes are colors + names — NOT privacy-sensitive.**
- **F-Exclude**: treat like other user-data — exclude from OS backup, `.taigi` only (policy uniform; lose themes on reinstall without export).
- **F-Allow**: allow OS backup (iCloud / Google auto-restores themes since non-sensitive) **and** include in `.taigi` (friendlier reinstall; diverges from R7's "all user-data excluded" posture).
- No default assumed — USER decision (release/privacy posture, Core Principle #5).

### Fork G (open) — user-theme count cap

- Suggest ~50; mirror the cap-parity pattern (S14). USER to confirm the number / whether a cap is wanted.

---

## 7. Image upload as custom theme — feasibility evaluation

Two interpretations with very different cost/risk:

### I-1 — extract a palette FROM the image (image → 6 roles → creates a user theme)

- Pipeline: pick image (host app) → quantization (Android `androidx.palette`; iOS Core Image / vImage) → dominant colors → heuristic map to 6 roles (contrast clamp) → **pre-fill the "+" editor**, user names + saves it as a `UserTheme`.
- **Fits the model perfectly** — output is a normal user theme; zero renderer change; no image stored at keyboard runtime; the 64 MB extension cap is avoided (host-app-side, only 6 colors persist).
- Risk: low-contrast auto-maps → enforce min contrast ratio + live preview + user tweak via the editor.
- **Feasibility: HIGH.** Recommended image path.

### I-2 — image AS the keyboard background (background-image layer)

- Needs a new "background image" layer behind translucent keys + the bitmap loaded **inside the keyboard extension at render time**.
- **iOS blocker**: ~64 MB hard memory cap (`.claude/rules/taigi-incidents.md`). A full-res photo decoded in the extension can blow the cap → keyboard killed. Needs app-side downscale + decoded-size budgeting + translucent-key restyling. Fragile.
- **Feasibility: LOW-MEDIUM, iOS-gated.** Recommend **defer** to its own user-gated slice with an explicit memory budget.

### Image recommendation

- MVP: **I-1 only** (palette extraction → a user theme). Defer **I-2**.

---

## 8. Information architecture & phases

### 8.1 Navigation — 5th tab (DECIDED: fork A, 2026-06-05)

Appearance/theme promoted to its own top-level tab. Result: **5 tabs** (at the iOS / Android Material-3 NavigationBar practical maximum — acceptable, no room for a 6th).

- **iOS**: add `case theme` to `TabType` (`TabType.swift`), add the 5th `tabItem` in `ContentView.swift`, icon `paintpalette.fill` (or `paintbrush.fill`), localized title via a new `ThemeTexts`. Move `AppearanceSettingsView` out from under `LayoutTab` into a new `ThemeTab`.
- **Android**: add a `theme` destination to the NavigationBar in `MainSettingsScreen.kt` + a `ui/tabs/theme/` package, mirroring `ui/tabs/layout/`. Move `AppearanceSettingsScreen` content into it.
- **Layout tab retains** keyboard layout + font.
- **Cross-platform parity**: tab order, icon semantics, title mirror (capture in `ui-style-guide.md` at impl). Check `.switchToSettingsTab`-style deep links for the new index.
- **Trade-off recorded**: spends a scarce nav slot on a set-and-forget feature, running against `ui-style-guide.md` §Feature Grouping by Usage Frequency. USER accepted for discoverability / showcase value (appearance is a keyboard-app selling point; SwiftKey/Gboard surface themes prominently).

### 8.2 Draft phase table (NOT committed sequencing — user-gated)

Sizing targets 200–500 LOC/PR per `~/.claude/rules/planning.md`.

| Phase | Scope | Depends on |
|---|---|---|
| P0 (admin) | This doc + roadmap entry + memory file | — |
| P1 | 5th tab scaffolding both platforms (move appearance out of Layout, no behavior change) | — |
| P2 | Theme-id storage model + resolver (`selectedThemeId`, `default`, migration) | — |
| P3 | Built-in static theme table (resolve Fork C) + curated hex (resolve Fork A coverage) | P2 |
| P4 | Shelf picker UI both platforms + live preview + light/dark resolve | P1, P2, P3 |
| P5 | User-theme CRUD: "+" create / name / save / rename / delete / duplicate + persistence + `.taigi` (resolve Forks E/F/G) | P2, P4 |
| P6 (optional) | Image → palette extraction (I-1) pre-filling the editor | P5 |
| (deferred) | A2 full-chrome override / I-2 background image / E2 light-dark editing / keyword-list extra themes | user-gated later |

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
| Palette extraction from image | AndroidX `androidx.palette` (canonical) | §7 I-1 |

**Project rules cited**: `cross-platform-alignment.md` (built-in table + user-theme schema + `.taigi` format + tab order are parity surfaces, §4.5/§4b.3/§8.1); CLAUDE.md Core Principle #2/#5/#6; `taigi-incidents.md` (iOS 64 MB cap §7; non-destructive migration §4.3; cap-parity S14 §4b.4; R7 backup posture §6 F); `ui-style-guide.md` §Feature Grouping (the §8.1 trade-off); `code-review-rules.md §9` (qualitative dogfood gate).

**刻意不採用 (deliberately not adopted)**:
- **KeyboardKit Pro theme engine** — Pro-gated + closed-source; reuse only its Shelf UI *design* (§3 Layer B).
- **FlorisBoard Snygg stylesheet engine / addon store** — YAGNI (§3 Layer C).
- **A new SQLite table for user themes** — YAGNI; a JSON list in settings suffices for a small bounded set (§4b.3).
- **I-2 runtime background-image in the iOS extension** — deferred on the 64 MB cap (§7).
- **Per-role day/dark flat storage (fork B2)** — unnecessary; the theme-id model keeps pairs in the table / per-theme (§4.4).

---

## 10. Open questions / TODO (USER to append)

- [x] Nav structure — **A: 5th tab** (2026-06-05)
- [x] Storage model — **theme-id resolve** (2026-06-05)
- [x] User-created themes — **yes, "+" CRUD, update-durable, multiple** (2026-06-05)
- [ ] Fork A — 6-role MVP vs full-chrome override?
- [ ] Fork C — hand-mirrored built-in table vs shared JSON (model favors shared JSON)?
- [ ] Fork E — user-theme light/dark: single value (MVP) vs editable pair?
- [ ] Fork F — user-theme OS-backup: exclude (like other user-data) vs allow iCloud restore (non-sensitive)?
- [ ] Fork G — user-theme count cap (suggest ~50) — number / needed?
- [ ] Nord light variant — ship dark-only, or author a "Nord Light"?
- [ ] Keyword-list extras (Tron / Nothing / Windows Phone / Oblivion) — which to include; license-check.
- [ ] Image upload — I-1 (palette extract → user theme) in v3.6.2, or defer?
- [ ] Tab icon choice (iOS `paintpalette.fill` vs `paintbrush.fill`; Android equivalent) + localized tab title.
- [ ] (USER additions below)

---

## Appendix — code-grounding citations (read 2026-06-05, READ-ONLY)

**iOS**: `Settings/KeyboardColorSettings.swift:10-58` · `Autocomplete/Models/CandidateTheme.swift:46-73` · `App/Tabs/Layout/AppearanceSettingsView.swift:44-243` · `AppearanceSettingsViewModel.swift:19-162` · `SharedSettings.swift:99,502-505,572` · `App/Tabs/TabType.swift:7-34` · `App/ContentView.swift:16-65`.

**Android**: `ime/core/KeyboardColorSettings.kt:14-54` · `ime/theme/ThemeAttributeColors.kt:9-16` · `ime/text/KeyboardAppearanceResolver.kt:47,56-63` · `ime/text/smartbar/KeyboardChromeColors.kt:28-47` · `ui/tabs/layout/AppearanceSettingsScreen.kt:80-392` · `ui/tabs/layout/ColorPickerDialog.kt:73-280` · `ui/tabs/MainSettingsScreen.kt` (NavigationBar) · `PrefHelper.kt:84,500`.

**KeyboardKit reference** (9.9.0, doc-lookup 2026-06-05): `references/keyboardkit9.9.0/Sources/KeyboardKit/KeyboardKit.docc/Features/Themes-Article.md:14-255` (Shelf/ShelfItem, ThemeContext, predefined themes incl. `.tron`, code-level custom themes); `_Pro/ProPlaceholders.swift` (engine is Pro-gated).

**FlorisBoard reference**: `references/florisboard/.../org.florisboard.themes/{extension.json, stylesheets/floris_day.json, floris_night.json}`.
