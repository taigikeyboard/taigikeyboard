Pre-merge readiness check for the current branch.

1. Show branch status: `git diff main..HEAD --stat`
2. Run Android tests: `cd android && ./gradlew test`
3. Run taigi-converter tests: `cd taigi-converter && node --test tests/`
4. Run `/cross-platform-audit --changes` to check alignment on changed files
5. Run `/log-audit --changes` to verify logging hygiene on changed files

Summarize as a merge readiness checklist:
- [ ] Tests passing (Android / converter)
- [ ] Cross-platform aligned
- [ ] Logging conventions followed
- [ ] Any manual review items
