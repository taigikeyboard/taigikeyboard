# Theme Presets & Custom Themes — Brainstorm (v3.6.2)

> **Type**: Planning (brainstorm — evolving; USER will append ideas)
> **Keywords**: `theme`, `preset`, `palette`, `colorscheme`, `custom theme`, `image upload`
> **Status**: Brainstorm — NO code, NO committed phase sequencing yet
> **Version scope**: v3.6.2 (USER-scoped 2026-06-05: 「這個列為 v3.6.2 的計劃」)
> **Related**: `docs/ui/theme.md` (current-state reference), `docs/roadmap.md` (deferred TODO "keyboard theme picker")

---

## Summary

- **Goal**: ship a set of **predefined theme presets** (named palettes the user picks in one tap), and evaluate **user-uploaded image** as a custom-theme source.
- **Current state is NOT greenfield** — both platforms already have a mirrored 6-role free-pick color system (`KeyboardColorSettings`); a preset is just "a fully-populated `KeyboardColorSettings` applied at once."
- **Layered design answer**: borrow **palette values** from established editor/vim colorschemes (Catppuccin, Tokyo Night, Nord, Gruvbox), borrow only the **semantic-token concept** from FlorisBoard's stylesheet — NOT its full addon-store + per-element stylesheet engine (over-engineering for a solo-maintainer cross-platform app).
- **Two hard constraints** surfaced by code-grounding: (1) only **6 roles** are user-overridable today — full chrome (pressed states, popups, enter key, emoji, smartbar) is platform-default; (2) **no light/dark split** in stored colors — a Catppuccin Latte-vs-Mocha pairing needs a schema change that is a cross-platform-invariant surface.
- **Image upload**: two distinct interpretations (palette-extraction vs background-image layer); the iOS keyboard-extension **64 MB hard memory cap** is the dominant feasibility constraint.

---

## 1. Goal & USER directive

USER 2026-06-05 (verbatim, kept as evidence):

> 「這是我未來的規劃,我想製作一些定義的主題色讓 user 選」
> 「評估讓使用者上傳圖片當作自定義的可能性」
> 「先撰寫文件,我想到什麼再補充上來」「這個列為 v3.6.2 的計劃」

This satisfies CLAUDE.md Core Principle #5 (release scope user-gated) — the v3.6.2 scope is the USER's explicit dated instruction, not an inferred scope. Phase sequencing below is **draft** and remains user-gated.

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

**The gap**: the full keyboard chrome is much larger than 6 roles and is **not** user-overridable today:
- Android Track A — ~30 XML theme attrs (`attrs.xml:4-40`): `key_bgColorPressed/Active`, `key_function_bgColor`, `key_enter_bgColor/fgColor`, `key_popup_bgColor/fgColor`, `key_popup_extended_*`, `emoji_key_*`, `smartbar_*`, `smartbar_accentColor`. Only 3 of these feed the Compose overlay chrome via `KeyboardChromeColors.from()` (`KeyboardChromeColors.kt:28-33`) — and that path is **not** fed by user colors.
- iOS — uncustomized roles delegate to KeyboardKit adaptive `Color(.assetName)` + system `Color(.label)`/`Color(.secondaryLabel)` (`CandidateTheme.swift:70-71`); pressed/popup/accent are KeyboardKit-internal.

→ **A preset can only recolor the 6 roles.** Pressed states, popups, enter key, emoji, smartbar chrome stay platform-default and may visually mismatch a strongly-tinted preset unless the override surface is widened (see §6 fork A).

### 2.2 Storage — single JSON blob, one key, per-platform

- iOS: `SharedSettings.swift:99` `colorSettingsKey = .codable("colorSettings", default: .default)` → App Group UserDefaults. `CodableColor` = RGBA `Double` ×4 (`KeyboardColorSettings.swift:16-37`).
- Android: `PrefHelper.colorSettings: String` (`PrefHelper.kt:500`, default `"{}"`) → DataStore key `COLOR_SETTINGS`; JSON object, each role a nullable packed-ARGB `Int`.
- **No separate light/dark storage** on either platform — one value per role, applied in both modes (overrides the adaptive light/dark switch for that role). iOS: `CodableColor` "light and dark share the same value by design" (`KeyboardColorSettings.swift:11-15`). Android: same single-Int model.

### 2.3 Customization UI today — per-element free pick, NO preset concept

- iOS: 6 `ColorPicker` rows (`AppearanceSettingsView.swift:186-209`), per-row reset + global reset (`:128-135`).
- Android: 6 `ColorSettingRow` → `ColorPickerDialog` with Grid / Spectrum / Sliders / hex tabs (`ColorPickerDialog.kt:73-75`). The "Grid" swatches are individual colors, **not** named palettes.
- **Neither platform has any named-preset / palette entity.** The only multi-color operation today is "reset all to nil/null". Live preview via `KeyboardPreviewPanel` (iOS `AppearanceSettingsView.swift:139`, Android `AppearanceSettingsScreen.kt:340`).

### 2.4 Resolver path (one-line, both platforms)

- iOS: `UserDefaults["colorSettings"]` → `TaigiKeyboardView.@State colorSettings` → split into `CandidateTheme.resolved` (candidate text, via `.candidateTheme` env) + inline KeyboardKit `.background`/`.keyboardButtonStyle`/`candidateStyle` closures. Live-refresh on `UserDefaults.didChangeNotification`.
- Android: `prefs.colorSettings` JSON → `KeyboardColorSettings.fromJson()` → `KeyboardAppearanceResolver` (keys) + `SmartbarManager.colorSettings()` (candidates) → applied as `userOverride ?: getColorFromAttr(themeAttr)`.

**Key consequence**: because all user color state already funnels through one `(role → color)` blob, a preset needs **zero renderer changes** — it only writes the blob.

---

## 3. Design decision — palette source vs apply architecture (two layers)

The original fork ("FlorisBoard addon theme vs vim colorscheme") conflates two different layers. They are NOT either/or.

| Layer | Question | Reference to borrow from |
|---|---|---|
| **A. Palette source** | which colors, which semantic roles | **vim/editor colorschemes** |
| **B. Apply architecture** | how colors reach each keyboard element | FlorisBoard token *concept* only — NOT its engine |

**Critical fact**: Catppuccin / Tokyo Night / Nord / Gruvbox / Oblivion are **originally editor/vim colorschemes**. The FlorisBoard addons the USER listed merely repackage them into keyboard stylesheets. So "vim colorscheme" and "those FlorisBoard addons" are the **same color-source pool**, differing only in delivery format.

### Layer A — palette: adopt vim-colorscheme model

- **Pre-curated & harmonious** — designer-tuned contrast, validated in both light/dark; safer than ad-hoc hand-mixing.
- **Semantic-role native** — colorschemes are already `bg / fg / accent / …` role maps; maps cleanly onto our 6 roles (e.g. Catppuccin `base/mantle/crust` → backgrounds, `text` → foreground, `mauve/blue` → accent).
- **Brand recognition = marketing** — "Catppuccin" / "Tokyo Night" carry community goodwill; better than "Theme 1".
- **Permissive licensing** — Catppuccin / Tokyo Night / Nord / Gruvbox are **MIT**; porting palette values + attribution is fine. Oblivion (GNOME) / Windows Phone need per-source license check before adoption.

### Layer B — architecture: FlorisBoard concept only, not the engine

FlorisBoard's Snygg stylesheet engine = CSS-vars + per-element selectors + downloadable addon store + publish flow (`org.florisboard.themes/extension.json`, `stylesheets/*.json` with `@defines` + per-selector rules).

- **Deliberately NOT adopted** (YAGNI, over-engineering for a solo-maintainer cross-platform app): the Snygg parser, arbitrary per-element selectors, the addon store, the publish/upload pipeline. Replicating that = a whole theming engine on two platforms with zero current need.
- **Adopted**: the `@defines` **semantic-token idea** — we already have it as the 6-role `KeyboardColorSettings`. A preset = a fixed role→hex table. No new engine.

---

## 4. Proposed preset model (MVP)

A preset is **"a fully-populated `KeyboardColorSettings` applied in one tap."** Injection points (both platforms, mirrored):

### 4.1 Model layer (best fit — the only place that knows all 6 roles)

- iOS: add `KeyboardColorPreset` enum (name + factory → fully-populated `KeyboardColorSettings`) next to `KeyboardColorSettings.swift`.
- Android: add preset constants/factory returning a fully-populated `KeyboardColorSettings` beside its companion (`KeyboardColorSettings.kt:35`).
- Each case maps 6 hex values → the role fields. **No renderer change** — the existing resolver consumes any `KeyboardColorSettings` verbatim.

### 4.2 Persistence

- Reuse the existing `colorSettings` key on both platforms (no new color storage). Applying a preset = write `preset.makeSettings()` through the existing change callback (iOS `applyColorChange` sibling; Android `onColorSettingsChanged`).
- **Open**: to highlight the *active* preset in the UI, add a small new key (`colorPresetName: String`), since today only the resolved colors are stored, not the preset identity. Alternative: reverse-match the blob against known presets (fragile once the user hand-edits one color). Lean toward storing the name.

### 4.3 UI

- Add a preset picker section/subpage parallel to the existing **font picker** (iOS `AppearanceFontPickerView` `AppearanceSettingsView.swift:215-243`; Android `FontPickerContent` `AppearanceSettingsScreen.kt:80-89`).
- Each preset row shows a swatch-set preview (the 6 colors). Selecting applies + (optionally) marks active.
- The live `KeyboardPreviewPanel` reflects the change for free (renders the real keyboard reading the same live store).

### 4.4 ViewModel sync (the one non-trivial piece)

- iOS `AppearanceSettingsViewModel`: add `applyPreset(_:)` that sets `settings.colorSettings` **and** re-seeds the 6 `@Published` color props + `savedColors`, so the per-element pickers reflect the preset (`AppearanceSettingsViewModel.swift:114-162` is the mirror to follow).
- Android: selecting a preset writes the blob; the per-role rows re-read from the same state — verify the `AppearanceSettingsScreen` state hoist reflects an external full-blob write (likely already does via DataStore flow; confirm at impl time).

### 4.5 Cross-platform parity invariant (mandatory)

- Preset list, names, and hex values must be **identical** on iOS and Android. Single source of truth options:
  - **(P1)** Two hand-mirrored tables (current pattern for `KeyboardColorSettings`) — simplest, but drift risk; needs a parity test/checklist.
  - **(P2)** A shared JSON preset table in-repo (e.g. `docs/ui/theme-presets.json` or an asset bundled to both), each platform parses. Closer to FlorisBoard's stylesheet-as-data; eliminates drift; slightly more plumbing.
- **Open fork** — see §6 fork C.

---

## 5. Preset shortlist (starting curation)

| Preset | Source license | Light/dark | Notes |
|---|---|---|---|
| Catppuccin Latte | MIT | light | pairs with Mocha |
| Catppuccin Mocha | MIT | dark | |
| Tokyo Night | MIT | dark | |
| Nord | MIT | dark-leaning | |
| Gruvbox (light + dark) | MIT | both | warm/retro — fits our retro design philosophy (`theme.md` §Design Philosophy) |
| Floris Day / Night | apache-2.0 | both | keep existing as baseline |

Oblivion / Windows Phone / Nothing / Tron from the USER's original keyword list: **license + source check required** before inclusion (not all MIT). Tron/Nothing-style are aesthetics, not a single canonical palette — would be an original interpretation, not a port.

> Final per-role hex tables are TBD — to be filled once §6 forks are resolved. Each preset must define all 6 roles; if §6 fork B (light/dark split) is adopted, each role carries a day+dark pair.

---

## 6. Open design forks (need USER decision before plan finalizes)

### Fork A — preset coverage: 6 roles only, or widen the override surface?

- **A1 (MVP, minimal)**: presets recolor only the 6 existing roles. Pressed/popup/enter/emoji/smartbar chrome stays platform-default → may clash with a strongly-tinted preset.
- **A2 (full theme)**: make the chrome overridable too — Android route `getColorFromAttr`/`KeyboardChromeColors.from` through a user-palette layer (`ThemeAttributeColors.kt:9`, `KeyboardChromeColors.kt:28`); iOS extend the KeyboardKit style closures. Larger, touches the XML-attr seam + KeyboardKit internals on both platforms.
- Recommendation: **start A1**, evaluate visual mismatch via dogfood, widen to A2 only if presets look broken. (Direction-first per Core Principle #6 — but A2's cost is real; decide with dogfood evidence.)

### Fork B — light/dark: single value, or per-mode pair?

- Today: one value per role, both modes (`CodableColor` / single Int).
- Catppuccin Latte-vs-Mocha and Gruvbox light/dark **cannot** be expressed without extending the schema to a day+dark pair per role.
- **B1**: keep single-value — ship each variant as a *separate* preset entry (Latte and Mocha are two list items). Simple, no schema change, but the user must re-pick when they switch system appearance.
- **B2**: extend `KeyboardColorSettings`/`CodableColor` to per-mode pairs — auto-follows system light/dark. **Cross-platform-invariant schema change** (both platforms, migration of existing stored blobs). Bigger.
- Recommendation: **B1 for v3.6.2 MVP** (no schema churn); record B2 as a follow-up the USER may scope later.

### Fork C — preset source of truth: hand-mirrored tables (P1) or shared JSON (P2)?

- P1 = consistent with current `KeyboardColorSettings` mirroring; P2 = drift-proof, more FlorisBoard-like. See §4.5.

### Fork D — active-preset identity: store name, or reverse-match?

- Lean store-name (§4.2). USER to confirm whether "currently selected: X" UI state is wanted in MVP.

---

## 7. Image upload as custom theme — feasibility evaluation

USER asked to evaluate user-uploaded image as a custom-theme source. **Two distinct interpretations** — they have very different cost/risk:

### Interpretation I-1 — extract a palette FROM the image (image → 6 role colors)

- Pipeline: user picks image → color quantization (k-means / median-cut) → dominant colors → heuristic map to the 6 roles (darkest→bg, highest-contrast→text, accent→saturated) → write `KeyboardColorSettings`.
- **Fits the existing architecture** — output is still just the 6-role blob; no renderer change, no image stored at keyboard runtime.
- Cost: a one-shot extraction in the **host app** (not the extension), so the 64 MB extension cap is avoided — the image is processed app-side, only 6 colors cross into shared settings.
- Per platform: iOS `UIImage` + Core Image / vImage or a small quantizer; Android `Palette` API (AndroidX `androidx.palette`) does exactly this out of the box.
- Risk: auto-mapped palettes can produce low-contrast / unreadable results → must clamp contrast (WCAG-ish min ratio) + always show live preview + let the user tweak afterward (the 6 pickers already exist).
- **Feasibility: HIGH.** This is the recommended image path — it degrades to "a fancy preset generator" and touches nothing risky.

### Interpretation I-2 — use the image AS the keyboard background (background-image layer)

- This is what FlorisBoard/Gboard "photo theme" does: the bitmap renders behind translucent keys.
- **Does NOT fit the current 6-role model** — needs a new "background image" concept (a 7th layer behind keys), translucent key fills, and the image bytes loaded **inside the keyboard extension at render time**.
- **iOS blocker**: keyboard extension has a **~64 MB hard memory cap** (project incident: `.claude/rules/taigi-incidents.md` S-perf, iOS 64 MB). A full-res user photo decoded in the extension can blow the cap → keyboard killed. Would require: downscale + re-encode app-side to a strict max dimension, cache a small bitmap in the App Group, and budget its decoded size against 64 MB alongside the engine + dict. Fragile.
- Android: less strict but still an IME-process memory concern; also needs translucent-key restyling to stay legible.
- Storage: processed image in App Group container (iOS) / `filesDir` (Android) — both already used for user data; mark as user-content.
- **Feasibility: LOW-MEDIUM, iOS-gated.** High effort, real crash risk on iOS, needs key-translucency rework. Recommend **defer** unless USER specifically wants photo backgrounds (then scope it as its own slice with a hard memory budget).

### Image-upload recommendation

- **MVP**: support **I-1 (palette extraction)** only — high value, low risk, reuses everything. Android `Palette` + an iOS quantizer, host-app-side, output = the 6-role blob.
- **Defer I-2 (background image)** to a separate user-gated slice with an explicit iOS 64 MB memory budget — do not bundle into the preset MVP.

---

## 8. Draft scope (v3.6.2) — NOT yet committed sequencing

USER-scoped to v3.6.2 (2026-06-05). Phase order below is **draft**; final sequencing + what-ships-in-v3.6.2 stays user-gated (Core Principle #5). Sizing targets 200–500 LOC/PR per `~/.claude/rules/planning.md`.

| Phase | Scope | Depends on |
|---|---|---|
| P0 (admin) | This doc + roadmap entry + memory file | — |
| P1 | Preset model + persistence + parity source-of-truth (resolve Fork C) | Fork B (B1), Fork D |
| P2 | Preset picker UI both platforms + live preview wiring | P1 |
| P3 | Curated preset hex tables (resolve Fork A coverage) + dogfood | P1, P2 |
| P4 (optional) | Image → palette extraction (I-1) | P1 |
| (deferred) | A2 full-chrome override / B2 light-dark pairs / I-2 background image | user-gated later |

---

## 9. Best-practices alignment (per `~/.claude/rules/planning.md`)

| 主流做法 | 來源 | 本 plan 對應 |
|---|---|---|
| Semantic color tokens (`@defines`) | FlorisBoard `org.florisboard.themes/extension.json` + `stylesheets/floris_day.json:3-31` | §3 Layer B — adopt token concept; 6-role `KeyboardColorSettings` is our `@defines` |
| Preset = named palette as data | FlorisBoard `extension.json` `themes[]` array (id/label/isNight) | §4 preset enum/table; §6 Fork B mirrors `isNight` |
| Curated colorscheme palettes | Catppuccin / Tokyo Night / Nord / Gruvbox (MIT) | §5 shortlist |
| Palette extraction from image | AndroidX `androidx.palette` (canonical) | §7 I-1 |

**Project rules cited**:
- `.claude/rules/cross-platform-alignment.md` — preset list/values are a parity surface (§2, §4.5).
- CLAUDE.md Core Principle #2 (align on intended behavior), #5 (release scope user-gated), #6 (direction-first).
- `.claude/rules/taigi-incidents.md` — iOS 64 MB extension cap gates §7 I-2.
- `code-review-rules.md §9` — qualitative dogfood gate for preset visual correctness (no quantitative perf numbers).

**刻意不採用 (deliberately not adopted)**:
- FlorisBoard Snygg stylesheet engine / arbitrary per-element selectors — YAGNI, over-engineering on two platforms (§3 Layer B).
- FlorisBoard Addons Store (download/publish pipeline) — out of scope; presets ship bundled in-app.
- I-2 background-image at runtime in the iOS extension — deferred on memory-cap risk (§7).
- B2 per-mode light/dark schema change — deferred; B1 ships variants as separate presets (§6 Fork B).

---

## 10. Open questions / TODO (USER to append)

- [ ] Fork A — 6-role MVP vs full-chrome override?
- [ ] Fork B — single-value (ship variants) vs per-mode light/dark pair?
- [ ] Fork C — hand-mirrored preset tables vs shared JSON source of truth?
- [ ] Fork D — store active-preset name vs reverse-match?
- [ ] Final preset roster — which of the keyword list (Tron / Catppuccin / Tokyo Night / Nothing / Windows Phone / Oblivion) to include; license-check the non-MIT ones.
- [ ] Image upload — I-1 (palette extract) in v3.6.2, or defer all image work?
- [ ] (USER additions below)

---

## Appendix — code-grounding citations (read 2026-06-05, READ-ONLY)

**iOS**: `Settings/KeyboardColorSettings.swift:10-58` · `Autocomplete/Models/CandidateTheme.swift:46-73` · `App/Tabs/Layout/AppearanceSettingsView.swift:44-243` · `AppearanceSettingsViewModel.swift:19-162` · `SharedSettings.swift:99,502-505,572`.

**Android**: `ime/core/KeyboardColorSettings.kt:14-54` · `ime/theme/ThemeAttributeColors.kt:9-16` · `ime/text/KeyboardAppearanceResolver.kt:47,56-63` · `ime/text/smartbar/KeyboardChromeColors.kt:28-47` · `ui/tabs/layout/AppearanceSettingsScreen.kt:80-392` · `ui/tabs/layout/ColorPickerDialog.kt:73-280` · `PrefHelper.kt:84,500` · `res/values[-night]/themes.xml` · `attrs.xml:4-40`.

**FlorisBoard reference**: `references/florisboard/app/src/main/assets/ime/theme/org.florisboard.themes/{extension.json, stylesheets/floris_day.json}`.
