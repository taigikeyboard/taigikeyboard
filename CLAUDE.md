# CLAUDE.md

Guidelines for **Claude Code (claude.ai/code)** when working with this codebase.

## Project Overview

**Taigi Keyboard** - Cross-platform Taiwanese input method
- iOS: Built with Swift and KeyboardKit
- Android: Kotlin implementation based on FlorisBoard
- Supports Taiwanese romanization (POJ/TL) and Hanji (漢字) input
- Tone variation and autocomplete features

## Project Structure

```
taigikeyboard/
├── android/        # Android version (Kotlin + FlorisBoard)
├── ios/            # iOS version (Swift + KeyboardKit)
├── docs/           # Technical specifications
│   ├── engine/     # IME core logic (composing, tone, trie, autocomplete, sort, nextword, flow, tps)
│   ├── ui/         # Presentation & layout (layout, flick, case, device, app-ui, theme)
│   ├── references/ # External IME research (azookey, khiin, moe-taigi, rime)
│   ├── keywords.md # Keyword glossary — start here
│   └── README.md   # Full index
└── CLAUDE.md       # This file
```

## Context Management

- Before starting multi-file refactoring or cross-platform changes, assess remaining context
- **Do not start a large task if**:
  - The system has already auto-compressed earlier messages
  - The conversation has 10+ file reads or multiple agent outputs accumulated
- **When context is low**: summarize progress, delegate file-heavy work to subagents, break remaining work into smaller steps that can each complete within available context
- Cross-platform changes (iOS + Android) consume significant context — consider handling one platform per pass

## Core Development Principles

1. **No unsolicited implementation** - Confirm with user before any feature or file changes
2. **No arbitrary removal** - Confirm with user before removing any functionality
3. **Follow YAGNI** - Only implement what's currently needed, keep it simple
4. **No project config modification** - `.xcodeproj`, `.xcworkspace`, `.pbxproj`, `build.gradle`, and other project/build configuration files must be modified manually by the user. AI must never edit these files
5. **Cross-platform alignment** - When porting logic between iOS and Android, align on **intended behavior** (what the user should see), not on **API calls**. iOS and Android have different platform semantics (e.g. marked text vs composing text). Always: (1) define the expected behavior first, (2) verify each platform's API achieves that behavior independently, (3) document in code comments when the same behavior requires different implementation per platform

## Communication Guidelines

- Think in English. Write code comments and responses in English
- Use Taiwanese Mandarin only when necessary for proper nouns or clarification
- Keep answers concise and direct
- Analyze problems first, provide solution options for user to choose
- Explain scope of impact before making changes

---

# iOS Project Guidelines

## KeyboardKit Development Rules

- **Must consult KeyboardKit documentation before implementation**
- **KeyboardKit 10+ is closed-source** - cannot view source code directly
- Local docs: `./references/KeyboardKit-Documentation/`
- Online docs: https://keyboardkit.github.io/KeyboardKitDocs/

## Memory Management

1. **Separate SwiftUI View from Controller** - Views must not directly hold Controller references
2. **setupKeyboardView safe mode** - Ignore controller parameter, use `self.state` and `self.services`
3. **Service class Delegates** - Must use `weak` reference
4. **Any memory-related changes must explicitly document risks**

## Test Conventions

- Tests must be simple, effective, and non-redundant — no duplicate coverage across files
- All conversion-related tests (TPS, TL, POJ, tone marks) use `./references/taigi-converter` as canonical reference implementation
- When tests fail, verify against reference behavior before changing production code
- Assertion messages must be descriptive enough to copy-paste for debugging
- Framework: XCTest; pattern: parametric arrays `[(input, expected)]` with loops + `XCTAssertEqual`
- Naming: `test{Component}_{scenario}`

---

# Claude Code Task Guidelines

## Role
- You are a senior Mobile (iOS/Android) engineer working on a Custom Keyboard project

## Research Directive
- When studying unfamiliar code or planning changes, read all related files thoroughly — understand data flow, edge cases, and dependencies before proposing solutions
- Do not skim; trace the full call chain from entry point to final output
- When asked to research, produce a written summary documenting findings

## Task Requirements
- Restate the problem to confirm mutual understanding
- Draft a fix plan without implementing
- Keep responses concise and focused, no emotional expressions
- Do not compile or test; fixes must not affect existing functionality
- Think in English. Write code comments and responses in English. Use Taiwanese Mandarin only when necessary for proper nouns or clarification

## Project Assistance
- Read `./docs/` folder to quickly understand implementation details, architecture, and keyword definitions
  - `docs/keywords.md` — keyword glossary: term definitions mapped to classes/methods
  - `docs/file-structure.md` — iOS/Android file mapping and directory structure
  - `docs/engine/flow.md` — end-to-end input flow (ActionHandler dispatch)
  - `docs/engine/` — core logic specs (composing, tone, trie, autocomplete, sort, nextword, tps)
  - `docs/ui/` — presentation specs (layout, flick, case, device, app-ui, theme)
  - `docs/references/` — external IME research (azookey, khiin, moe-taigi, rime)
- `./references/` contains cloned external repos (azooKey, KeyboardKit-Documentation, etc.) — search here first when referencing external projects, no need for web search
- iOS: New files require manual target addition by user

## Design Principles

### Android / Kotlin
- Follow Kotlin, Android, Jetpack official best practices
- Reference: [Creating Input Method](https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method)
- Comply with Android Keyboard Design Guidelines
- Follow KeyboardKit best practices

### iOS / Swift
- Follow SwiftUI / UIKit official best practices

### General
- Follow GitHub open-source conventions
- Do not arbitrarily remove features or create files without user confirmation
- No build testing required
