# Taigi Keyboard Project Guidelines

## Engineering Standards
- **Source Control**: NEVER stage or commit changes unless explicitly requested.
- **Project Integrity**: NEVER modify `.xcodeproj`, `.xcworkspace`, `.pbxproj`, `build.gradle`, or `.gradle.kts` files. These must be updated manually by the user.
- **Cross-platform Alignment**: When porting logic (iOS/Android), align on **intended behavior** (user experience) first, then verify each platform's API independently. Document differences in comments.
- **Phonetic Conversion**: Must read `knowledge/taigi-phonetics-reference.md` and consult `taigi-converter/` (submodule) as the canonical reference before changes.

## Development Lifecycle
1. **Research**: Use `docs/keywords.md` to map terms to code and `docs/file-structure.md` for platform mapping.
2. **Strategy**: Propose solution options and explain scope of impact before implementation.
3. **Execution**: Think in English, write code comments/responses in English. Use Taiwanese Mandarin only for proper nouns.

## Build & Test Commands
- **iOS**: `xcodebuild test` (requires manual target addition for new files).
- **Android**: `cd android && ./gradlew test`.
- **Converter**: `cd taigi-converter && node --test tests/`.
