# Documentation Sync Audit (2026-03-11)

## Accurate (14/17 files)
composing, autocomplete, tone, sort, segmentation, trie, tps, flow, nextword, keywords, layout, case, device, app-ui

## Needs Fix

### `docs/file-structure.md`
- Missing `Diagnostics/` directory in iOS listing
- References nonexistent `ThemeTokens.swift` (only exists in archived `references/taigikeyboard/`)

### `docs/ui/theme.md`
- Describes ThemeTokens mechanism but iOS actually uses SwiftUI native colors + Styling providers
- Should be rewritten to match actual implementation

## Correctly Marked Incomplete
- `docs/ui/flick.md` — design spec, not yet implemented

## Missing Documentation

| Feature | Key Files | Priority |
|---------|-----------|----------|
| Custom Dictionary | `CustomDictionaryService`, `CustomDictionaryRepository`, views | Medium |
| Diagnostic Service | `DiagnosticService.swift` (iOS & Android) | Low |
| Styling/Appearance | `Styling/Providers/*`, `ButtonFontProvider`, etc. | Low |
