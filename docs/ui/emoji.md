# Emoji Keyboard

> **Type**: Feature
> **Keywords**: `emoji`, `ISEmojiView`, `skin-tone`, `taigi-emojis`, `media-input`
> **Related**: ../architecture/behavioral-invariants.md (§13 atomic external insert), ../architecture/data-artifacts-portability.md, system-overview.md

---

## Summary

- Full emoji palette on both platforms, backed by the `taigi-emojis` data submodule (`dist/emoji.json`, Unicode Emoji 17.0 / CLDR 48, 1889 emoji, 9 categories).
- **Intentional UI divergence**: iOS uses the vendored third-party `ISEmojiView` (UIKit); Android uses a custom Jetpack Compose palette. This is recorded per `.claude/rules/cross-platform-alignment.md` §3, not a parity bug.
- Both insert emoji through the engine's atomic preedit-commit path (`ComposingManager.commitPreeditThenInsertExternal`) — the one strict parity point.

---

## Files

### iOS

| File | Responsibility |
|------|----------------|
| `Emojis/EmojiService.swift` | Wraps vendored `ISEmojiView`; configures `KeyboardSettings`, bridges `EmojiViewDelegate` → `EmojiServiceDelegate`; exposes `emojiKeyboardView` via `UIViewRepresentable` |
| `Emojis/TaigiEmojiData.swift` | Loads bundled `emoji.json`, maps taigi-emojis categories → ISEmojiView `[EmojiCategory]`, CoreText glyph-filters. Single source, no fallback |
| `KeyboardExtension/KeyboardViewController+EmojiDelegate.swift` | `EmojiServiceDelegate`: emoji insert (atomic), switch-to-alphabetic, dismiss, backspace |
| `Layout/KeyDef.swift` / `LayoutConverter.swift` | `.emoji` key def → KeyboardKit `.keyboardType(.emojis)` |
| `Vendor/ISEmojiView/` | Vendored third-party emoji UI (category bar, grid, skin-tone pop-preview, recents) |

### Android

| File | Responsibility |
|------|----------------|
| `ime/media/emoji/EmojiLayoutData.kt` | Moshi-decodes `emoji.json` → model; `PaintCompat.hasGlyph` gate. Single source of truth |
| `ime/media/emoji/EmojiPaletteView.kt` | Compose UI: category tab row + `HorizontalPager` + `LazyVerticalGrid` (7 cols); long-press `EmojiVariationsPopup` |
| `ime/media/emoji/EmojiKeyboardView.kt` | `FrameLayout` host embedding `ComposeView`; async data load on IO; routes taps to `MediaInputManager` |
| `ime/media/emoji/EmojiPreferences.kt` | DataStore (`emoji_preferences`) — persisted preferred skin tone |
| `ime/keyboard/EmojiSkinTone.kt` | 6 Fitzpatrick tones (incl. DEFAULT) + codepoint mapping |
| `ime/media/MediaInputManager.kt` | Media-mode host (ViewFlipper, switch-to-text + backspace buttons, `sendEmojiKeyPress`) |

---

## Show + insert flow

**iOS** — tapping the `.emoji` key maps (via `LayoutConverter`) to KeyboardKit's `.keyboardType(.emojis)`; KeyboardKit swaps in `TaigiKeyboardView`'s `emojiKeyboard:` builder, which renders the `ISEmojiView` at `layout.totalHeight`. Selecting an emoji → `EmojiServiceDelegate.emojiDidSelect` → `ComposingManager.commitPreeditThenInsertExternal(emoji)`. The ABC button returns to `.alphabetic`.

**Android** — the emoji-toggle key carries `KeyCode.SWITCH_TO_MEDIA_CONTEXT (-213)`; `TextInputKeyHandler` calls `setActiveInput(R.id.media_input)`, flipping to the media ViewFlipper. `EmojiKeyboardView` async-loads `emoji.json` on `Dispatchers.IO`, sets Compose content observing the skin-tone Flow. Tap → `MediaInputManager.sendEmojiKeyPress` → `ComposingManager.commitPreeditThenInsertExternal(emoji, ic)`. The switch-to-text button emits `SWITCH_TO_TEXT_CONTEXT`.

The atomic-insert routing is `INVARIANT_composing_external_insert_commits_preedit_atomically` (behavioral-invariants §13) — both platforms honor it; an in-progress preedit is committed before the emoji, never interleaved.

---

## Cross-platform divergence

| Aspect | iOS | Android |
|---|---|---|
| Emoji UI | Vendored **ISEmojiView** (UIKit) in `UIViewRepresentable` | Custom **Jetpack Compose** (`EmojiPaletteView`) |
| Mode switch | KeyboardKit native `.keyboardType(.emojis)` | `SWITCH_TO_MEDIA_CONTEXT` keycode → ViewFlipper |
| Recently-used | ISEmojiView built-in, persisted to `UserDefaults` (count 30) | **None** — no recents category or persistence |
| Skin-tone persistence | None app-side (ISEmojiView pop-preview only) | DataStore `emoji_preferences` (`preferred_skin_tone`), applied as default base |
| Categories shown | 8 (+ recents) — merges `smileys_emotion`+`people_body` | 9 — those two are separate tabs |
| Keyword search | Keywords not decoded (memory-lean) | Keywords decoded but **no search UI consumes them** |
| Glyph filter | CoreText `CTFontGetGlyphsForCharacters` (AppleColorEmoji) | `PaintCompat.hasGlyph` |
| Atomic insert | `commitPreeditThenInsertExternal` | `commitPreeditThenInsertExternal` — **parity** |

---

## Data — `taigi-emojis/dist/emoji.json`

Data-only submodule (no shared Swift/Kotlin module); each platform reads `dist/emoji.json` into its own native model and glyph-filters the whole grapheme cluster at load. Schema is frozen by the submodule's `output-contract.md` + a drift-guard test; version-pin bumps are user-gated.

- Top level: `{ meta, categories[] }`. `meta` = `{ emojiVersion: "E17.0", cldrVersion: "48", count: 1889, generator }`.
- 9 categories in order: `smileys_emotion, people_body, animals_nature, food_drink, travel_places, activities, objects, symbols, flags`.
- Each emoji: `{ base, cp, name, subgroup, version, variations[], keywords[], keywordsByLocale }`. iOS decodes only `id`/`base`/`variations`; Android decodes `id`/`base`/`name`/`variations`/`keywords`.
- iOS bundles the file as a keyboard-extension resource (missing file → `assertionFailure`, no fallback). Android mounts `../taigi-emojis/dist` as an assets srcDir (`app/build.gradle.kts`), so `emoji.json` lands at the assets root; a parse failure logs and returns an empty map.

**Skin tones (Android `EmojiSkinTone`)**: DEFAULT (0x0), LIGHT (1F3FB), MEDIUM_LIGHT (1F3FC), MEDIUM (1F3FD), MEDIUM_DARK (1F3FE), DARK (1F3FF) — Fitzpatrick modifiers.

> Note: the Gradle/README comment says the submodule is "pinned v0.1.0"; the actual recorded commit is a few commits past the `v0.1.0` tag. The pin is the recorded gitlink SHA, not the tag string — check `git submodule status` for the real pin.

---

## Constants

- iOS recents count: 30 (`EmojiService.swift`); ISEmojiView cap `MaxCountOfRecentsEmojis = 50`.
- Android grid: 7 columns, 240.dp grid height, 35.sp emoji font, variations popup 6/row.
- Android emoji-toggle keycode: `SWITCH_TO_MEDIA_CONTEXT = -213`.
- Submodule: `taigi-emojis` (branch `main`), consumed as `dist/emoji.json`.

---

## See also

- `taigi-emojis/README.md` + `taigi-emojis/CLAUDE.md` + `.claude/rules/output-contract.md` (submodule-owned consumption contract + JSON schema).
- `behavioral-invariants.md` §13 (atomic external insert), popup-hide invariants.
- `ui/callouts.md` (the long-press popup mechanism the emoji skin-tone variations reuse on Android).
