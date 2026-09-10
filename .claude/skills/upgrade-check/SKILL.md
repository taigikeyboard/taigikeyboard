---
name: upgrade-check
description: Audit whether users can upgrade cleanly between two app versions. Diffs the persistence + upgrade-mechanics surface (DB schema versions, .taigi backup format, settings keys, AndroidManifest permissions, removed assets, version monotonicity) and classifies every finding BLOCKING / BEHAVIOR-CHANGE / SAFE. Read-only — never builds, edits, or commits. Args: <base-ref> [<target-ref>] (target defaults to HEAD).
disable-model-invocation: false
---

# Upgrade Compatibility Check

Answer one question: **can a user on `<base-ref>` upgrade to `<target-ref>` without a crash, a forced data wipe, or a store-rejected install?**

Run from `main`. User supplies `<base-ref> [<target-ref>]`.
Example: `/upgrade-check v3.6.2 v3.6.3` · `/upgrade-check v3.6.2` (target = HEAD).

**Read-only.** This skill greps two refs and reports. It NEVER builds, edits files, or commits. Safe to run anytime, on a dirty tree.

## What this checks (and what it does NOT)

This is a **persistence + upgrade-mechanics** audit. In scope: anything that travels across an app update on the user's device — SQLite user-data, `.taigi` backups, settings storage, manifest permissions, bundled-asset references, store version gates.

**Out of scope** (use dogfood / tests / `/code-review` instead): functional regressions, input-correctness, ranking quality, UI glitches. A clean upgrade-check does NOT mean the new version behaves correctly — only that the old user's data and install survive the jump.

## Classification

| Tier | Meaning | Examples |
|---|---|---|
| **BLOCKING** | Upgrade crashes, wipes user data, or the store/OS refuses the install | settings key removed but still read; schema bumped with no forward migration; `.taigi` accept-floor raised so old backups reject; versionCode not monotonic; new **dangerous** permission silently breaking a flow |
| **BEHAVIOR-CHANGE** | Upgrade succeeds, but a user sees something different — must be in the changelog | settings default flipped (default-reliant users change); a bundled item removed from recents; intentional UX default change |
| **SAFE** | No user-visible upgrade effect | new key with default; new normal permission auto-granted; schema unchanged (no migration runs); bundled artifact rebuilt with identical content |

A BLOCKING finding fails the check. BEHAVIOR-CHANGE findings must each map to a changelog line. SAFE findings are listed for the receipt.

## Procedure

Read each file at both refs with `git show <ref>:<path>`; for code that may move, `git diff <base>..<target> -- <path>`. Trace, don't assume — quote `file:line` + old→new value for every finding.

### 1. SQLite user-data schema versions

These three DBs persist on-device and migrate forward via `PRAGMA user_version`. The iOS and Android constants are paired (must stay aligned).

| DB | Android constant | iOS constant |
|---|---|---|
| 自訂詞 custom dict | `ime/dictionary/CustomDictionaryService.kt` `DATABASE_VERSION` | `Lexicon/Database/CustomDictionarySchema.swift` `schemaVersion` |
| 詞關聯 association | `ime/dictionary/NextWordService.kt` `DATABASE_VERSION` | `NextWord/Repository/NextWordSchema.swift` `schemaVersion` |
| 詞頻 frequency | `ime/text/composing/UserFrequencyService.kt` `DATABASE_VERSION` | `Lexicon/Database/UserFrequencySchema.swift` `pairKeySchemaVersion` |

For each: compare the constant at `<base>` vs `<target>`.

- **Unchanged** → no migration runs on upgrade (the `current >= target → return` guard short-circuits). SAFE.
- **Bumped** → a migration MUST exist and be:
  - **forward-only + idempotent** — guarded by `currentVersion >= VERSION return` (Android) / `currentVersion < schemaVersion` gate (iOS), ALTER/backfill unconditional inside the guard so a half-migrated DB re-runs clean.
  - **non-destructive** — verify it does not `DROP`/recreate a table that holds user rows without backfill (a `v<N` full-rebuild branch is acceptable ONLY for pre-feature versions that had no user data of that shape — confirm against the migration body).
  - **iOS↔Android aligned** — a bump on one platform with no matching bump on the other is a divergence (`NextWordSchema.swift` carries a `mirrors Android's DATABASE_VERSION` comment). Flag it.
  - Bumped with no migration body, or a destructive one → **BLOCKING**.

### 2. `.taigi` backup format

`android/.../ime/dictionary/BackupService.kt` — `put("version", N)` (write version) + `require(version >= M)` (accept floor). iOS counterpart in the App-layer backup/restore code.

- Write version bumped but accept-floor unchanged → new app still reads old backups. SAFE for the upgrade path (the cross-device concern — old app reading a *new* backup — is a separate downgrade case; note it but it is not an upgrade blocker).
- **Accept-floor raised** (`>= M` with M increased) → a user's existing/exported backup at an older version now **rejects on import** → BLOCKING for restore.
- New fields added to the JSON with safe `opt*` reads → SAFE.

### 3. Settings keys (storage)

iOS `Settings/SharedSettings.swift` (`SettingsKey` definitions + `resetToDefaults()`); Android `ime/core/PrefHelper.kt` (`PreferenceKeys` + `preference(...)` delegates). `git diff` both.

- **Key removed** while a reader still references it → BLOCKING (crash / lost setting). Grep the codebase for the removed key name at `<target>` to confirm no live reader.
- **Default value flipped** on an existing key → BEHAVIOR-CHANGE. Only default-reliant users (never wrote the key) are affected; users who explicitly set it keep their stored value (the default applies only when the key is absent). The `resetToDefaults()` change does NOT run on upgrade — it fires only on an explicit user reset. Must be in the changelog.
- **Semantics changed** (same key, new meaning / new encoding) → BEHAVIOR-CHANGE, possibly BLOCKING if the stored value is now misread.
- **New key + default** → SAFE.

### 4. AndroidManifest

`android/app/src/main/AndroidManifest.xml` — `git diff`.

- **New normal permission** (e.g. `VIBRATE`, `INTERNET`) → auto-granted at install, silent on upgrade. SAFE (but list it — INTERNET specifically warrants a privacy/security glance given this keyboard's no-network posture).
- **New dangerous permission** (`READ_CONTACTS`, `RECORD_AUDIO`, …) → upgrade shows a prompt / the feature is dead until granted → BLOCKING or BEHAVIOR-CHANGE depending on whether a core flow depends on it.
- **`android:exported` flipped to true**, or an exported component losing its `android:permission` (`BIND_INPUT_METHOD`) → security regression → BLOCKING. Cross-check `.claude/rules/security-rules.md` § Exported Components.
- **`allowBackup`** flipped → backup-posture change (`INVARIANT_USER_DATA_EXCLUDED_FROM_OS_BACKUP` §29) → BEHAVIOR-CHANGE.

### 5. Removed / renamed bundled assets

`git diff <base>..<target> --diff-filter=D --name-only -- 'android/app/src/main/assets/**' 'ios/Sources/**/Assets.xcassets/**' 'ios/Resources/**' 'macos/Resources/**' 'windows/resources/**' 'dictionaries/**'`.

For each deleted asset, ask: **does any persisted user state reference it by an unstable handle?**

- Recents / favorites stored by **stable value** (emoji codepoint string, canonical TL) → a removed item just stops appearing; no crash. SAFE / minor BEHAVIOR-CHANGE.
- Persisted state stored by **index / row-id / resource-name into the old asset set** → removing an item shifts indices → wrong data or crash → BLOCKING. (Emoji recents on both platforms key by emoji string → SAFE. Verify before clearing.)
- A drawable referenced via `getIdentifier()` and stripped by resource-shrink → check `keep.xml` (see `project_android_release_resource_shrink.md`).

### 6. Bundled binary format versions

`dictionary.bin` header version, `syllables.fst` / `dictionary.fst` layout, `association.bin` layout. These ship inside the APK/IPA — replaced wholesale on install, so the on-device pairing is always (new engine + new bin). 

- A format-version constant bump where the **shipped reader** parses the new format → SAFE (they travel together).
- The only risk is a reader that also opens a **user-writable** copy of one of these (e.g. a migrated-into-place DB) at an old format → trace that path if a format constant changed.

### 7. Version monotonicity (store gate)

- **Android** `app/build.gradle.kts` `versionCode` — must strictly increase or the store / installer refuses the update. This repo computes `versionCode = (System.currentTimeMillis()/60_000)` → monotonic by construction; just confirm the expression is unchanged. A hardcoded or decreased versionCode → BLOCKING.
- **iOS** `MARKETING_VERSION` in `project.pbxproj` — must increase for App Store; `make version-mobile x.y.z` writes it on iOS + Android together (the mobile train). macOS + Windows are the separately numbered **desktop train** (`make version-desktop x.y.z`; `CFBundleVersion` = `MAJOR*10000 + MINOR*100 + PATCH` must increase per shipped pkg). A desktop audit takes `desktop-<version>` (the tag a desktop release creates in this repo since 2026-09-09) or, for anything older, a main-repo commit — the earlier `macos-v*` / `windows-v*` tags live on the website repo and are not in this history. `CURRENT_PROJECT_VERSION` stays 1 by design — App Store Connect numbers a marketing version's uploads itself — so a build number that did NOT move is not a finding. **pbxproj is user-owned** (`.claude/rules/ios-guidelines.md`) — do NOT edit it; read both targets (app + keyboard-extension), and when a bump is needed name the `make version-mobile` / `make version-desktop` command as a user action item.

## Output

Emit a markdown report:

```
# Upgrade check: <base> → <target>

**Verdict: <CLEAN | CLEAN WITH BEHAVIOR CHANGES | BLOCKED>**

## Findings
| # | Area | Tier | Evidence (file:line, old→new) | Action |
|---|------|------|-------------------------------|--------|
...

## Behavior changes — confirm each is in changelog/<target>.md (mobile) / changelog/desktop-<target>.md (desktop)
- ...

## Blocking — must fix before release
- ...   (omit section if none)
```

Rules:
- Every row cites `file:line` + the old→new value. No claim without evidence.
- A SAFE result for a whole area still gets one row ("schema: all 6 unchanged → no migration runs").
- BEHAVIOR-CHANGE findings are cross-checked against `changelog/<target>.md`; flag any that are missing from the changelog.
- Never assign release scope (in/out of vX) — that is user-gated (`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope). Report compat facts only.
- This skill does not fix anything. If a BLOCKING finding needs a code change, that is a separate user-gated bugfix round (Core Principle #4).

## Notes

- Refs can be tags (`v3.6.2`), branches, or SHAs. `git show <ref>:<path>` reads a file at a ref without checkout.
- If `<base-ref>` is not an ancestor of `<target-ref>`, say so — a non-linear comparison may miss reverts.
- The file paths above are the canonical surface as of v3.6.3. If a future refactor moves a schema/settings file, update this list in the same PR that moves it.
