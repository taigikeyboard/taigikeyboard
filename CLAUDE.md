# CLAUDE.md

Guidelines for **Claude Code (claude.ai/code)** when working with this codebase.

## Project Overview

**Taigi Keyboard** — Cross-platform Taiwanese input method (iOS: Swift + KeyboardKit, Android: Kotlin + FlorisBoard). Supports POJ/TL romanization, Hanji (漢字), tone variation, and autocomplete.

## Project Structure

```
taigikeyboard/
├── android/           # Android (Kotlin + FlorisBoard)
├── ios/               # iOS (Swift + KeyboardKit)
├── docs/              # Specs: engine/, ui/, references/, keywords.md, README.md
├── knowledge/         # Taiwanese phonetics reference (TL/POJ/TPS)
├── taigi-converter/   # Canonical TL↔POJ↔TPS converter (git submodule)
├── rules/             # Mandatory rules (must read before relevant changes)
│   ├── ui-style-guide.md   # Cross-platform UI styling spec
│   ├── security-rules.md   # Logging, SQL, network, data storage rules
│   └── ios-guidelines.md   # SourceKit, KeyboardKit, memory, naming, tests
├── dictionary/        # Dictionary data files
├── scripts/           # Build and utility scripts
└── references/        # External repos (gitignored — don't place tracked content here)
```

## Core Development Principles

1. **No unsolicited implementation** — Confirm with user before any feature or file changes
2. **No arbitrary removal** — Confirm with user before removing any functionality
3. **Follow YAGNI** — Only implement what's currently needed, keep it simple
4. **No project config modification** — `.xcodeproj`, `.pbxproj`, `build.gradle` etc. must be modified manually by the user. AI must never edit these files
5. **Cross-platform alignment** — Align on **intended behavior**, not API calls. Define expected behavior first, verify each platform independently, document when same behavior requires different implementation
6. **Phonetic conversion** — When working on TL/POJ/TPS conversion, **must** read `knowledge/taigi-phonetics-reference.md` and consult `taigi-converter/` before changes
7. **UI Style Guide** — Before modifying app UI, **must** read `rules/ui-style-guide.md`
8. **Security Rules** — Before adding logging, SQL, network, or storage code, **must** read `rules/security-rules.md`
9. **iOS Guidelines** — Before modifying iOS code, **must** read `rules/ios-guidelines.md`

## Build & Test

| Platform | Build | Test |
|----------|-------|------|
| iOS | Open `ios/` in Xcode, build keyboard extension | `xcodebuild test` or Xcode |
| Android | `cd android && ./gradlew assembleDebug` | `cd android && ./gradlew test` |
| taigi-converter | — | `cd taigi-converter && node --test tests/` |

## Communication

- Think and write in English. Use Taiwanese Mandarin only for proper nouns or clarification
- Analyze problems first, provide solution options for user to choose
- Explain scope of impact before making changes

## Key References

- `docs/README.md` — Full documentation index (engine, UI, keywords, references)
- `references/` — Cloned external repos (azooKey, KeyboardKit-Documentation) — search here first
- `knowledge/taigi-phonetics-reference.md` — TL/POJ/TPS cross-reference
- iOS: New files require manual Xcode target addition by user
- Android: Follow [Creating Input Method](https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method) guidelines
