# Taigi Keyboard - Technical Specifications

> **Type**: Index
> **Keywords**: `spec`, `index`, `specification`

---

## Summary

- Cross-platform reference for iOS/Android implementation alignment
- Quick reference for Claude Code, bullet-point style, concise

---

## Document Index

### `engine/` — IME Core Logic

| File | Description | Status |
|------|-------------|--------|
| `engine/composing.md` | Composing state management (`rawInput`, `composingText`) | Active |
| `engine/autocomplete.md` | Candidate search and suggestion pipeline | Active |
| `engine/tone.md` | Tone conversion and restoration (`ToneConverter`) | Active |
| `engine/sort.md` | Candidate sorting with user frequency scoring | Active |
| `engine/trie.md` | Trie index + binary mmap readers (`MARISA`, `BinaryReader`, `Bitmask`) | Active |
| `engine/flow.md` | iOS end-to-end data flow and action handling | Active |
| `engine/nextword.md` | Next word prediction (binary association + user learning) | Active |
| `engine/segmentation.md` | Syllable segmentation (DAG+DP, onset atomicity) | Active |
| `engine/tps.md` | TPS Taiwanese Phonetic Symbols (方音符號) | Active |
| `engine/custom-dictionary.md` | User-defined dictionary (CRUD, CSV import/export) | Active |
| `engine/diagnostics.md` | Device info collection for bug reporting | Active |

### `ui/` — Presentation & Layout (6)

| File | Description | Status |
|------|-------------|--------|
| `ui/layout.md` | Keyboard layout definitions and conversion | Active |
| `ui/flick.md` | Flick tone keyboard layout and input | Active |
| `ui/case.md` | Case handling (Shift, CapsLock, transformation) | Active |
| `ui/device.md` | Device adaptation for iPhone and iPad | Active |
| `ui/app-ui.md` | Main App UI structure (tabs, settings) | Active |
| `ui/theme.md` | Theme & styling (colors, fonts, user customization) | Active |

### `references/` — External IME Research (5)

| File | Description | Status |
|------|-------------|--------|
| `references/azookey-reference.md` | azooKey research (SwiftUI, Flick, CustardKit) | Reference |
| `references/khiin-reference.md` | khiin-rs research (DPSegment, Bigram, Trie) | Reference |
| `references/moe-taigi-reference.md` | MOE Taigi IME analysis (Segmentation, Nail) | Reference |
| `references/moe-taigi-asr-reference.md` | MOE Taigi IME ASR implementation analysis | Reference |
| `references/rime-reference.md` | librime research (Pipeline, DAG, SpellingAlgebra) | Reference |

### `reports/` — Audit & Analysis Reports

| File | Description | Status |
|------|-------------|--------|
| `reports/architecture-review.md` | Full architecture review: SRP, layers, folders, naming, data flow | Active |
| `reports/refactor-backlog.md` | Refactoring task backlog (iOS + Android) | Active |
| `reports/codebase-health.md` | Code quality metrics and health check | Reference |
| `reports/docs-audit.md` | Documentation accuracy verification | Reference |
| `reports/khiin-lattice-research.md` | Word lattice architecture research | Reference |
| `reports/segmentation-tie-bug.md` | CVC+V segmentation tie-breaking analysis | Reference |
| `reports/2026-03-11-audit-report.md` | Combined audit report (docs, health, research) | Reference |

### Root — Guides & Planning (4)

| File | Description | Status |
|------|-------------|--------|
| `keywords.md` | Standardized keyword mapping for core logic and UI | Reference |
| `file-structure.md` | File index, directory structure, naming conventions | Reference |


---

## Naming Conventions

| Category | Pattern | Example |
|----------|---------|---------|
| Engine spec | `engine/{module}.md` | `engine/composing.md` |
| UI spec | `ui/{module}.md` | `ui/layout.md` |
| External reference | `references/{project}-reference.md` | `references/khiin-reference.md` |
| Guide / Planning | `{descriptive-name}.md` (root) | `roadmap.md` |

---

## Format Guidelines

### Document Structure

```markdown
# [Title]

> **Type**: [Feature|Reference|Planning|Index]
> **Keywords**: `keyword1`, `keyword2`
> **Related**: file1.md, file2.md

---

## Summary
- 1-3 sentence core purpose

## Core Concepts
- Bullet point highlights

## Platform Comparison
| Item | iOS | Android |

## Related Files
| File | Description |

## Notes
- Key decisions or pitfalls
```

### Writing Principles

- Bullet points preferred, avoid long paragraphs
- Use English for keywords
- Code snippets should be key fragments only (< 10 lines)
- Keep it concise to reduce token consumption

---

## Feature Module Codes

| Code | Description | iOS Entry | Android Entry |
|------|-------------|-----------|---------------|
| `Composing` | Composing management | `ComposingManager.swift` | `ComposingManager.kt` |
| `Autocomplete` | Autocomplete | `AutocompleteService.swift` | `TaigiAutocompleteService.kt` |
| `Lexicon` | Dictionary query | `LexiconService.swift` | `LexiconService.kt` |
| `BinaryReader` | Binary mmap dict/assoc | `DictionaryBinaryReader.swift` | `DictionaryBinaryReader.kt` |
| `Trie` | Trie indexing | `TrieService.swift` | `TrieService.kt` |
| ~~`Segmentation`~~ | ~~Syllable segmentation~~ (removed v3.4.6) | — | — |
| `Tone` | Tone processing | `ToneConverter.swift` | `ToneConverter.kt` |
| `UserFrequency` | User frequency | `UserFrequencyService.swift` | `UserFrequencyService.kt` |
| `NextWord` | Next word prediction | `NextWordService.swift` | `NextWordService.kt` |
| `Layout` | Keyboard layout | `CustomLayoutService.swift` | `LayoutManager.kt` |
| `Theme` | Theme & styling | `Styling/Providers/` | `themes.xml` + `PrefHelper.kt` |
| `CustomDictionary` | User-defined dictionary | `CustomDictionaryRepository.swift` | `CustomDictionaryService.kt` |
| `Diagnostics` | Device info for bug reports | `DiagnosticService.swift` | `DiagnosticService.kt` |
