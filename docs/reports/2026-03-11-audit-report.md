# Taigi Keyboard Audit Report — 2026-03-11

> **Note (2026-04)**: Snapshot report. dictionary.db eliminated on both platforms (binary mmap migration complete). Many action items resolved — see individual reports for current status.

Three audits performed: Documentation sync, codebase health, and architecture research.

---

## 1. Documentation Sync Audit

### Accurate (14/17 files)
composing, autocomplete, tone, sort, segmentation, trie, tps, flow, nextword, keywords, layout, case, device, app-ui

### Needs Fix
| File | Issue |
|------|-------|
| `docs/file-structure.md` | Missing `Diagnostics/` directory; references nonexistent `ThemeTokens.swift` |
| `docs/ui/theme.md` | Describes ThemeTokens mechanism but iOS uses SwiftUI native colors + Styling providers |

### Correctly Marked Incomplete
- `docs/ui/flick.md` — design spec, not yet implemented

### Missing Documentation
| Feature | Key Files |
|---------|-----------|
| Custom Dictionary | `CustomDictionaryService.swift`, `CustomDictionaryRepository.swift`, views |
| Diagnostic Service | `DiagnosticService.swift` (iOS & Android) |
| Styling/Appearance | `Styling/Providers/*`, `ButtonFontProvider`, `ButtonTextProvider`, `ButtonImageProvider` |

---

## 2. Codebase Health Check

**Codebase size**: iOS 97 Swift files (14,363 LOC) | Android 100+ Kotlin files (20,431+ LOC)

### Clean Areas
- Unused imports: **0** (iOS & Android)
- TODO/FIXME/HACK: **0**
- Dead code: None significant
- Core engine test coverage: Good (10 test files covering segmenter, tone, phonetics, TPS, integration)

### Android — Critical File Sizes

| File | Lines | Concern |
|------|-------|---------|
| `SmartbarManager.kt` | 1364 | Toolbar appearance + actions + theming mixed |
| `AppearanceSettingsScreen.kt` | 1239 | Single Composable with color picker, preview, gradients |
| `TextInputManager.kt` | 1231 | All input dispatch, composing, tone, suggestions |

**Recommendation**: Split each into 2-3 single-responsibility components.

### iOS — Notable Files (acceptable)

| File | Lines | Note |
|------|-------|------|
| `NextWordService.swift` | 669 | Bigram + decay + SQLite |
| `ExpandedCandidateOverlay.swift` | 539 | 12-level SwiftUI nesting |
| `UserFrequencyRepository.swift` | 515 | SQL transaction handling |

### Test Coverage Gaps
- **Untested (acceptable)**: All UI/SwiftUI code (50+ view files)
- **Could benefit from tests**: ActionHandler dispatch (5 files, 900+ LOC), database operations, CustomDictionary CRUD

### Code Smells
- iOS: `SharedSettings.shared` singleton used in 15+ files (limits testability)
- iOS: 3 repository files with repeated SQLite boilerplate
- Android: 20+ branch when/switch blocks in TextInputManager and SmartbarManager

---

## 3. Khiin/RIME Word Lattice Architecture Research

### Khiin's Approach
- **Word-level DP** — unifies segmentation + word selection in one step
- Pre-loads all words + frequencies into `HashMap<String, f64>` for O(1) lookup
- Cost function: `ln(1/freq) / word_len^0.2 * syl_count^0.2` (minimize)
- Uses `<=` tie-breaking (later-evaluated path wins; moot with frequency data)

### RIME's Approach
- Word lattice (DAG) + Viterbi + bigram language model
- Score: `Σ log P(word_i | word_{i-1})` — captures word sequence patterns
- More accurate but requires corpus-trained bigram data

### Current Taigi Keyboard Gap
> **Note (2026-04)**: Segmenter removed in v3.4.6. Tie bug resolved via `WordPrefixChecker`. Frequency now in `dictionary.bin` (binary mmap).

- ~~Segmenter is syllable-level, no word frequency access~~ (resolved)
- ~~Frequency data locked in SQLite, not accessible to segmenter~~ (now in binary mmap)
- ~~Two-stage separation (segmenter → autocomplete) causes CVC+V tie bugs~~ (resolved)

### Recommended Migration Path

| Stage | Content | Risk | When |
|-------|---------|------|------|
| **1** | Expose word frequency to segmenter via closure for tie-breaking | Low | Next version |
| **2** | Word-level DP with cost map (Khiin-style) | Medium | If ties persist or sentence input needed |
| **3** | Bigram + Viterbi (RIME-style) | High | Long-term, requires corpus |

### Key Insight
Mobile keyboard processes 1-3 words at a time with candidate bar for correction. **Stage 1 is sufficient** for current needs. Stage 2+ only justified if sentence-level continuous input becomes a feature request.

---

## Action Items Summary

### High Priority
- [ ] Android: Break down 3 files >1200 lines into smaller components

### Medium Priority (resolved 2026-04)
- [x] Fix `docs/file-structure.md` — updated with binary mmap files
- [x] Rewrite `docs/ui/theme.md` — matches actual implementation
- [x] Add doc for Custom Dictionary feature — `engine/custom-dictionary.md`
- [x] Add doc for Diagnostic Service — `engine/diagnostics.md`

### Low Priority
- [ ] Add integration tests for ActionHandler dispatch flow
