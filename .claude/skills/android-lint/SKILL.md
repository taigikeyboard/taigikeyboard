---
name: android-lint
description: Run Android Lint, report issues, then use plan mode to plan and apply safe fixes. No runtime behavior change.
---

# Android Lint

Run lint, report, plan fixes, apply.

## Steps

### 1. Run Lint & Report

```bash
cd android && ./gradlew lintDebug 2>&1
```

Read the text report, extract category counts, and show summary table to user:

```
grep -oE '\[[A-Za-z]+\]' <report-path> | sort | uniq -c | sort -rn
```

### 2. Enter Plan Mode

Use `EnterPlanMode` to plan fixes. Categorize each issue as:

- **Suppress** — false positives or inherited issues (MissingTranslation, UnusedResources, GradleDependency, OldTargetApi, AndroidGradlePluginVersion) → add to `lint { disable += }` in build.gradle.kts
- **Fix** — real but safe issues (NewApi, MissingClass, InvalidManifestAttribute, ObsoleteSdkInt) → XML attribute or Kotlin version guard changes
- **Defer** — invasive changes (UseKtx, HardcodedText, StaticFieldLeak, etc.) → list but skip

Write plan, get user approval via `ExitPlanMode`.

### 3. Apply & Verify

Apply approved fixes, then:

```bash
cd android && ./gradlew assembleDebug 2>&1  # must pass
cd android && ./gradlew lintDebug 2>&1       # check reduction
```

Show before/after comparison.

## Rules

- Never modify build config (compileSdk, minSdk, dependencies) — only the `lint { }` block
- Never delete resource files — suppress via lint config
- Never change business logic, SQL, or UI behavior
- Never suppress NewApi or MissingClass — fix them with code
- `tools:targetApi` is required when the XML attribute itself needs higher API — don't blindly remove
- Verify `assembleDebug` passes after every change
