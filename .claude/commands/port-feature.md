Port a feature between iOS and Android platforms.

Arguments: $ARGUMENTS (e.g., "ios Tone" or "android Composing")

Steps:
1. Read `docs/architecture/file-structure.md` to find the iOS ↔ Android file pairs for the specified module
2. Read the source platform's implementation files thoroughly
3. If the module involves phonetics, also read `knowledge/taigi-phonetics-reference.md`
4. Draft a porting plan showing:
   - Source files to reference
   - Target files to create/modify
   - Platform-specific adaptations needed (different APIs, different paradigms)
   - Key behavioral alignment points (what the user should see must match)
5. Wait for user approval before implementing

Remember: align on **intended behavior**, not on API calls. iOS and Android have different platform semantics.
