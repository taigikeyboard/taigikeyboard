# Emoji Output Contract (cross-platform)

Always-on. Governs `dist/emoji.json` — the single artifact both iOS and Android consume.
Change the schema only with a matching update to both platform loaders + the tests.

## Schema

```jsonc
{
  "meta": { "emojiVersion": "E17.0", "cldrVersion": "48", "count": 1889,
            "generator": "taigi-emojis/scripts/generate.py" },
  "categories": [
    {
      "id": "smileys_emotion",          // stable id; see GROUP_TO_CATEGORY in generate.py
      "title": "Smileys & Emotion",     // display title (English; platforms may localize)
      "order": 0,                        // category sort order
      "emoji": [
        {
          "base": "😀",                  // the emoji string to insert (fully-qualified)
          "cp": [128512],                // codepoints of base (for validation / tests)
          "name": "grinning face",       // English name from emoji-test.txt
          "subgroup": "face-smiling",    // Unicode subgroup (debug / future ordering)
          "version": "E1.0",             // emoji version, or "custom" for overrides add
          "variations": ["😀🏻", ...],    // skin-tone / mixed-tone forms, [] if none
          "keywords": ["grin","笑",...], // merged en+zh+tl, lowercased, deduped (search)
          "keywordsByLocale": {          // source-of-truth split for future ranking
            "en": [...], "zh_Hant": [...], "taigi": [...]
          }
        }
      ]
    }
  ]
}
```

## Invariants both platforms rely on

- **Categories**: exactly the 9 ids in `GROUP_TO_CATEGORY`, in `order`. Unicode "Component"
  group is excluded. No empty category ships.
- **Identity**: `base` is the insert string. A base never carries a skin-tone modifier —
  tone forms live only in `variations`. No `base` appears in two categories.
- **Keywords**: `keywords` is the flat search index (substring/prefix match). It is
  lowercased + deduped + order-stable. `keywordsByLocale` is the un-merged source if a
  platform later wants per-locale ranking. Search reads `keywords`; do not re-merge.
- **Recents** are platform-side state (iOS App Group `UserDefaults`, Android prefs). Not
  in the json.

## Platform integration notes

- **Glyph filter at load**: an OS font may not render every shipped emoji (the version pin
  is conservative but not zero-gap). Each platform filters unsupported emoji at load.
  **Filter the whole grapheme cluster, not per-scalar** — an unsupported ZWJ sequence can
  falsely pass a per-codepoint check.
  - Android: `PaintCompat.hasGlyph(paint, base)` on the full base string (already the
    pattern in `EmojiLayoutData.kt`).
  - iOS: check the full string renders (e.g. `CTFontGetGlyphsForCharacters` over the
    cluster / a `String`-renders helper), not per `UnicodeScalar`.
- **Distribution**: this pipeline is tracked directly in the app repository. Apps read
  `taigi-emojis/dist/emoji.json`; iOS bundles it into the keyboard extension, Android into
  `assets/`.
- **Cross-platform parity**: the json is the shared contract. iOS native SwiftUI view and
  Android Compose view must derive categories, ordering, variations, and search from this
  file — no platform-side hardcoded emoji lists.
