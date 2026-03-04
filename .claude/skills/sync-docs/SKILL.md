---
name: sync-docs
description: Sync ./docs/ specs with current code reality. Detects stale references, missing entries, and outdated descriptions by comparing docs against actual source files. Use when docs may be out of date after code changes, or periodically to ensure doc accuracy.
disable-model-invocation: true
---

# Sync Docs

Audit `./docs/` documentation against the actual codebase. Detect stale references, missing entries, outdated descriptions, and terminology drift. Produce a report with proposed updates, then apply after user approval.

## Arguments

The user may provide:
- A **doc file name** (e.g., `tone.md`, `flow.md`, `file-structure.md`) to audit a single doc
- A **module name** (e.g., `Tone`, `Composing`, `Autocomplete`) to audit the doc spec for that module
- `--keywords` to only audit `docs/keywords.md`
- `--file-structure` to only audit `docs/file-structure.md`
- `--changes` to audit docs affected by files changed on the current branch vs main
- No argument = audit all docs

## Steps

### 1. Determine scope

**If a doc file name is provided:**
- Audit that doc only, plus identify its related source files.

**If a module name is provided:**
- Map to the corresponding doc under `docs/engine/` or `docs/ui/`.
- Also check `docs/keywords.md` and `docs/file-structure.md` for entries related to that module.

**If `--changes` is provided:**
- Run `git diff main..HEAD --name-only` to get changed source files.
- Map changed files to their related docs using `docs/file-structure.md`.
- Also always include `docs/keywords.md` and `docs/file-structure.md` in scope.

**If no argument:**
- Audit all files under `docs/` (excluding `docs/references/` — those are external project docs).

### 2. Read docs and source files

For each doc in scope:

1. Read the doc file completely.
2. Identify all source files the doc references (by file path, class name, function name, or module name).
3. Read those source files in parallel.
4. Also read `docs/keywords.md` for terminology cross-reference.

### 3. Check each doc

#### 3a. Stale References

Check every code reference in the doc against the actual codebase:

- **Dead symbols** — Function, class, property, or enum case mentioned in the doc but no longer exists in source code
- **Renamed symbols** — Symbol exists under a different name (search for similar names if the original is missing)
- **Moved files** — File path mentioned in the doc but the file has moved to a different location
- **Deleted files** — File path referenced but the file no longer exists
- **Wrong signatures** — Function signature in the doc doesn't match the actual signature (different parameters, return type)

For each stale reference, search the codebase to find the current equivalent (if any).

#### 3b. Missing Documentation

Compare what's in the doc against what's in the code:

- **Undocumented public functions** — Public/internal functions in the source files that aren't mentioned in the spec
- **Undocumented classes/structs** — New types added to the module but not reflected in the doc
- **Undocumented parameters** — Function parameters not described in the doc's API section
- **Missing flow steps** — For `flow.md`: check if new processing steps have been added to the input pipeline that aren't documented
- **Missing keywords** — For `keywords.md`: check if new domain terms appear in code comments or function names but aren't in the glossary

#### 3c. Outdated Descriptions

Check whether prose descriptions still match code behavior:

- **Algorithm changes** — The doc describes an algorithm but the code implements it differently
- **Constant/threshold drift** — The doc states a value (e.g., "max 10 candidates") but the code uses a different value
- **Flow changes** — Steps described in the doc no longer match the actual execution order
- **Removed features** — The doc describes a feature that has been removed from the code
- **New behavior** — The code has new branches, modes, or behaviors not reflected in the doc

#### 3d. File Structure Audit (for `docs/file-structure.md`)

Compare the file structure doc against the actual filesystem:

- **Missing files** — Source files that exist but aren't listed in the doc
- **Ghost files** — Files listed in the doc that no longer exist
- **Wrong paths** — Files listed with incorrect paths
- **Missing pairs** — iOS file exists but no Android counterpart listed (or vice versa)
- **New directories** — New directories created in the source tree but not reflected in the structure

Scan actual directories:
- `ios/Sources/TaigiKeyboard/` (`.swift`)
- `android/app/src/main/java/com/siansiansu/taigikeyboard/` (`.kt`)

#### 3e. Keywords Audit (for `docs/keywords.md`)

- **Stale terms** — Terms defined in keywords.md that no longer appear in the codebase
- **Missing terms** — Domain-specific terms used in code (comments, function names, type names) that aren't in keywords.md
- **Wrong definitions** — Term definitions that no longer match how the term is used in code
- **Wrong class/method mapping** — The keyword entry maps to a class or method that has been renamed or removed

Scan source files for domain-specific terms by checking:
- Class and struct names
- Function names containing domain terms (e.g., `tone`, `trie`, `compose`, `segment`)
- Comments containing Taiwanese/linguistic terminology
- Enum case names

#### 3f. Cross-Doc Consistency

Check that docs don't contradict each other:

- Same function described differently in `flow.md` vs its module spec
- `keywords.md` definition conflicts with module spec description
- `file-structure.md` file pairs don't match what module specs reference

#### 3g. Language and Style

- **Non-English prose** — Flag doc sections written in Chinese or other non-English languages (code examples and domain terms are OK)
- **Inconsistent terminology** — Same concept referred to by different names across docs (use `keywords.md` as the canonical source)
- **Broken markdown** — Malformed tables, broken links, unclosed code blocks

### 4. Produce the report

Output a structured report:

```markdown
## Sync Docs Report

**Scope**: [doc file | module name | keywords | file-structure | changed files | full]
**Date**: [today]
**Docs audited**: [count]
**Source files cross-referenced**: [count]

### Summary

| Category                | Count |
|-------------------------|-------|
| Stale references        | N     |
| Missing documentation   | N     |
| Outdated descriptions   | N     |
| File structure issues   | N     |
| Keywords issues         | N     |
| Cross-doc conflicts     | N     |
| Language/style issues   | N     |
| **Total**               | N     |

### Findings

#### [docs/engine/tone.md]

| #  | Category       | Location     | Issue                                              | Proposed Fix                                        |
|----|----------------|--------------|----------------------------------------------------|----------------------------------------------------|
| 1  | Stale ref      | Line 45      | `func oldToneConvert()` no longer exists           | Update to `func convertTone()`                     |
| 2  | Missing doc    | —            | `func handleToneSandhi()` not documented           | Add section describing tone sandhi handling         |
| 3  | Outdated desc  | Line 78      | Says "max 5 candidates" but code uses 10           | Update to "max 10 candidates"                      |
| 4  | Wrong signature| Line 30      | `func convert(_ input: String)` → actual has 2 params | Update signature to match code                  |

#### [docs/keywords.md]

| #  | Category       | Term           | Issue                                    | Proposed Fix                    |
|----|----------------|----------------|------------------------------------------|---------------------------------|
| 1  | Missing term   | segmentation   | Used in SyllableSegmenter but not in glossary | Add definition                |
| 2  | Wrong mapping  | trieKey        | Maps to `TrieNode.key` but renamed to `TrieNode.searchKey` | Update mapping   |

#### [docs/file-structure.md]

| #  | Category       | Path                           | Issue                      | Proposed Fix           |
|----|----------------|--------------------------------|----------------------------|------------------------|
| 1  | Ghost file     | `ios/.../OldManager.swift`     | File deleted               | Remove from doc        |
| 2  | Missing file   | `ios/.../NewFeature.swift`     | File exists but not listed | Add to doc             |

#### [Next doc...]
...
```

### 5. Ask for user approval

Present the report and ask the user:
1. Which findings to apply (all? specific categories? specific items?)
2. Review proposed fixes for accuracy — especially for outdated descriptions, where the user may know the intended behavior better
3. For missing documentation, ask whether to add brief entries or detailed sections

Do NOT modify any files until the user confirms.

### 6. Apply changes

After user approval:

1. Apply approved changes doc by doc.
2. For each doc:
   - Update stale references to current names/paths
   - Add missing documentation entries in the appropriate section, following the doc's existing format
   - Correct outdated descriptions to match current code
   - Fix file structure entries (add missing, remove ghosts, correct paths)
   - Update keywords entries (add missing, correct mappings, remove stale)
   - Fix cross-doc inconsistencies (align to the most accurate source)
   - Fix language/style issues

3. **Format rules:**
   - Match the existing formatting style of each doc (heading levels, table format, list style)
   - New entries should follow the same structure as existing entries
   - Keep docs concise — don't add lengthy explanations where a one-liner suffices
   - Use terms from `docs/keywords.md` consistently
   - Write in English; preserve domain terms (方音符號, 漢字, POJ, TL, TPS) as-is

### 7. Summary

After applying changes, output a summary:

```markdown
## Changes Applied

| Doc File                  | Updated | Added | Removed | Lines Changed |
|---------------------------|---------|-------|---------|---------------|
| docs/engine/tone.md       | 3       | 1     | 0       | 12            |
| docs/keywords.md          | 1       | 2     | 1       | 8             |
| docs/file-structure.md    | 0       | 3     | 2       | 10            |
| **Total**                 | 4       | 6     | 3       | 30            |
```

### 8. Suggest /clean-comments

If doc updates revealed new terminology or renamed concepts, suggest running `/clean-comments` to update code comments accordingly. List the specific modules that might be affected.

## Important

- **Never modify source code files** — this skill only modifies files under `./docs/`
- **Never delete a doc file** — only update contents. If a doc is entirely obsolete, flag it in the report and let the user decide
- **Never rewrite a doc from scratch** — make targeted edits to fix specific issues. Preserve the author's structure and style
- **Preserve domain terms** — 方音符號, 漢字, POJ, TL, TPS should remain as-is in docs
- **`docs/references/` is read-only** — never modify files under `docs/references/`, they document external projects
- **When adding missing documentation, keep it proportional** — a missing utility function gets a one-liner, a missing core algorithm gets a proper section
- **When unsure if a description is outdated or intentionally aspirational** (describing planned behavior), flag it and ask the user
- **Cross-doc fixes should align to the most authoritative source** — module specs are more authoritative than `flow.md` for module-specific details; `keywords.md` is authoritative for terminology
- Follow the existing doc formatting conventions throughout
