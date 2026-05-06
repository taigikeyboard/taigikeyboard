---
name: migration-residue
description: Audit Rust core ↔ iOS/Android platform code for residue, duplication, and redundancy left over from slice migrations. Produces a dated report at docs/reports/<YYYY-MM-DD>-migration-residue.md with P1/P2/P3 findings across seven dimensions (cross-language duplication, cross-crate duplication, over-public surface, stale comments, build-broken JVM tests, bridge surface parity, memory hygiene). Pure measurement — no auto-fix. Run before each Rust slice migration to seed the audit doc, and after each merge to verify cleanliness.
disable-model-invocation: true
---

# Migration Residue Audit

Scan the Taigi Keyboard codebase for migration debt left after a Rust slice swap. Goal: keep the migration architecture clean — no platform code mirrors a Rust algorithm, no Rust function is broader than its callers, no doc comment cites a deleted symbol.

## Arguments

- **No argument** = full audit across all seven dimensions.
- `--dimensions <a,b,c>` = run only listed dimensions (e.g. `--dimensions A,D` for cross-lang duplication + over-public Rust surface).
- `--codex` = after the local audit, pipe the report through `codex exec` for an independent second-opinion pass per `feedback_codex_only.md`.

## Output

`docs/reports/<YYYY-MM-DD>-migration-residue.md` (UTC). If same-date file exists, suffix `-rN`. Report template at the bottom of this file.

## When to run

- **Before** planning a new slice migration — produces the inventory of cleanup work to bundle in or punt out.
- **After** a slice PR merges — verifies the cleanup actually happened (regression catch).
- **Before** a release tag — final sweep so the version doesn't ship with stale doc comments / build-broken tests.

## Seven dimensions

### A — Cross-language algorithm duplication (P1 if production / P2 if test-only)

**Goal**: every algorithm lives in exactly one place. After a slice migrates to Rust, the platform mirror disappears.

**Detect**:

1. List Rust public API surface from `engine/`:
   ```bash
   grep -rn "^pub fn \|^pub use " engine --include="*.rs" | grep -v target
   ```

2. List platform helper functions in iOS Swift + Android Kotlin that share names with Rust:
   ```bash
   # iOS
   grep -rn "static func \|func " ios/Sources/TaigiKeyboard --include="*.swift" | grep -v Tests/
   # Android
   grep -rn "fun \|@JvmStatic" android/app/src/main --include="*.kt"
   ```

3. For each platform helper whose name matches a Rust function name (or differs only by camelCase ↔ snake_case), report it as a candidate duplicate. **Exception**: if the platform helper's body just calls `RustEngineBridge.<op>(...)` it's a thin wrapper, not a duplicate — keep.

4. Flag known mirror smells:
   - Per-codepoint scans: `for char in input.unicodeScalars { table[char] }` (Swift) / `for (c in input)` over a phonetic table (Kotlin)
   - Algorithm constants shared across language boundaries (e.g. tone-table maps copied verbatim)

**Report**: P1 if production code calls the platform helper; P2 if only tests do; P3 if it's just a doc reference.

### B — Cross-crate algorithm duplication inside `engine/` (P2)

**Goal**: every Rust algorithm lives in exactly one crate. No `engine/foo/src/x.rs` and `engine/bar/src/x.rs` with the same body.

**Detect**:

1. Find `pub fn` items across crates with similar names:
   ```bash
   grep -rn "^pub fn " engine --include="*.rs" | grep -v target | sort -u
   ```

2. For each candidate pair, compare bodies (read both, look for byte-identical or trivially-renamed implementations).

3. Known smells:
   - Multiple `nfd_preprocessed` / `taigi_unicode_*` / `*_to_*` helpers across crates
   - Two crates with `mod tables` containing the same constant tables

**Report**: P2 with proposed consolidation crate (usually phonetics, since it owns the lowest-level transformations).

### C — Over-public Rust surface (P2)

**Goal**: visibility matches actual reachability. `pub fn` only when external crates reach the symbol; `pub(crate)` for cross-module within crate; plain `fn` for same-file.

**Detect**:

For each `pub fn`, `pub struct`, `pub use`, `pub static`, `pub const` in `engine/<crate>/src/**.rs` (excluding tests, FFI extern, and crate-root re-exports):

1. Search every workspace caller:
   ```bash
   grep -rn "<symbol>" engine --include="*.rs" --include="*.toml"
   ```

2. Classify:
   - Used outside the defining crate (other workspace crates / CLI / integration tests / FFI bridges) → keep `pub`
   - Used only within the defining crate → recommend `pub(crate)`
   - Used only within the defining file (or `mod tests`) → recommend plain `fn`

3. Special case: items re-exported at `crate::lib.rs` via `pub use` MUST stay `pub` (cross-crate public surface).

**Skip rules**:

- `engine/swift-ffi/`: `pub extern "C"` / `#[swift_bridge::bridge]` items are FFI exports — don't recommend tightening.
- `engine/android-jni/`: `#[no_mangle] pub extern "system" fn Java_...` are JNI exports — don't recommend tightening.
- Items inside `mod tests` blocks (already test-private).
- Workspace-public items used by `engine/cli/`.

**Report**: P2 with file:line + recommended visibility level.

### D — Stale doc-comment references (P3 mostly, P2 if misleading reader)

**Goal**: every `path/to/file.rs:line`, function name, or memory file path in a doc comment is still valid.

**Detect**:

1. Extract all path-style references from doc comments + module docs across:
   - `engine/**.rs` (`//!` and `///` lines)
   - `ios/Sources/TaigiKeyboard/**.swift` (`///` and `// MARK:` blocks)
   - `android/app/src/main/**.kt` (`/** */` Kdoc)
   - `docs/engine/*.md` (markdown bodies)
   - `rules/*.md`
   - `~/.claude/.../memory/MEMORY.md` and `feedback_*.md`

2. For each referenced path/symbol:
   - File path mentioned exists?
   - Function name still defined at the cited line range?
   - Memory file in `MEMORY.md` index actually exists in the memory dir?

3. Known smells:
   - `phonetics::api::process_request` (deleted v3.5.3)
   - `engine/ranking/src/nfd.rs` (deleted v3.5.3-followup)
   - `parser.rs` (renamed to `syllable.rs` in v3.5.3)
   - `feedback_jvm_test_jni_compat.md` (deleted v3.5.3-followup)
   - `nfd_preprocessed` (renamed to `taigi_unicode_base_form` / `trie_key_unicode_form` in v3.5.3)
   - `D9.1` / `D9.2` / `D9.4` history wording in headers (cosmetic project-history baggage)

**Report**: P3 by default; P2 if a reader following the citation would be misled into editing the wrong place.

### E — Build-broken JVM unit tests (P1)

**Goal**: every `android/app/src/test/**.kt` either compiles cleanly OR is deleted. No silent build-broken tests.

**Detect**:

The check resolves each import against the **fully qualified name** that `src/main` actually declares — not just the trailing symbol — because a name-only grep treats `ime.dictionary.ToneConverterModels.InputMode` as resolved as soon as ANY `InputMode` enum exists anywhere in the tree, even when `ToneConverterModels` itself is deleted.

1. Build a map of every top-level FQN currently declared in `android/app/src/main/`:
   ```bash
   find android/app/src/main/java -name "*.kt" -type f | while read f; do
     pkg=$(grep -m1 "^package " "$f" | awk '{print $2}')
     grep -E "^(class|object|interface|enum class|sealed class|data class|typealias|fun|val|var) " "$f" \
       | sed -E "s|^[a-z ]+ ([A-Za-z_][A-Za-z0-9_]*).*|$pkg.\1|"
   done | sort -u > /tmp/main_fqns.txt
   ```

2. List **project-package** imports only across `android/app/src/test/**.kt` — exclude framework / stdlib / 3rd-party imports (`org.junit.*`, `kotlinx.*`, `kotlin.*`, `androidx.*`, `com.google.*`, etc.) since they live outside the Android `src/main/` symbol space:
   ```bash
   grep -rhn "^import com\.siansiansu\.taigikeyboard\." android/app/src/test --include="*.kt" | sort -u
   ```

3. For each project-package import, walk suffix prefixes of the FQN against `main_fqns.txt`. The walk handles both top-level and nested-member imports:
   - `import com.siansiansu.taigikeyboard.pkg.Type` → match `pkg.Type` directly.
   - `import com.siansiansu.taigikeyboard.pkg.OuterType.Member` → truncate `.Member` → match `pkg.OuterType` (the outer FQN must exist).
   ```bash
   grep -rh "^import com\.siansiansu\.taigikeyboard\." android/app/src/test --include="*.kt" \
     | sort -u | while read line; do
       fqn="${line#import }"
       prefix="$fqn"
       matched=false
       while [ "$prefix" != "com.siansiansu.taigikeyboard" ]; do
         if grep -qxF "$prefix" /tmp/main_fqns.txt; then
           matched=true
           break
         fi
         prefix="${prefix%.*}"
       done
       [ "$matched" = false ] && echo "BROKEN: $line"
     done
   ```

4. Independent check: any test that does `import com.siansiansu.taigikeyboard.engine.RustEngineBridge` cannot run on the host JVM regardless of import resolution — `System.loadLibrary("rust_taigi")` triggers `UnsatisfiedLinkError`. Flag these as build-broken-at-runtime.

5. Common causes for orphaned imports:
   - Slice deleted the platform helper but not its test (e.g. `TaigiPhonetics`, `ToneRestoration`, `TPSConverter`, `ToneConverterModels` deleted post-D9.4 but tests still import nested members like `ToneConverterModels.InputMode` — the unrelated top-level `InputMode` enum at `ime/core/settings/` masks the breakage in a name-only grep).
   - Test bundled into a slice that hasn't shipped yet — flag with the slice name and check the open PR.

**Why match against full FQN**: name-only matching gives false-clean when the deleted outer container shares a member name with an unrelated surviving symbol. The walk above resolves the FULL import target before declaring success, mirroring how `kotlinc` actually resolves imports.

**Report**: P1 — these tests must either be fixed or deleted. They contribute zero coverage while looking like they do.

### F — Bridge surface parity (P2)

**Goal**: every Rust op exposed to platforms has a matching wrapper on BOTH iOS `RustEngineBridge.swift` AND Android `RustEngineBridge.kt`.

**Detect**:

1. Enumerate **dispatchable method entries** only — restricted to the `oneof method { ... }` blocks of `PhoneticsRequest` (in `phonetics.proto`) and `LexiconRequest` (in `lexicon.proto`). A naive scan over every proto field would match envelope / message fields like `CommandType type = 2;` and report bogus parity gaps.
   ```bash
   # phonetics.proto — extract entries inside `PhoneticsRequest { oneof method { ... } }`
   awk '/^message PhoneticsRequest/,/^}/' engine/protos/proto/phonetics.proto \
     | awk '/oneof method/,/^  }/' \
     | grep -E "^\s+[A-Z][A-Za-z]+ [a-z_]+ = [0-9]+;"
   # lexicon.proto — same shape
   awk '/^message LexiconRequest/,/^}/' engine/protos/proto/lexicon.proto \
     | awk '/oneof method/,/^  }/' \
     | grep -E "^\s+[A-Z][A-Za-z]+ [a-z_]+ = [0-9]+;"
   ```

2. Cross-check against the Rust dispatch tables — every method enumerated above must have a matching arm in `phonetics::dispatch::handle` (`engine/phonetics/src/dispatch.rs::Method::*`) or `engine/dispatch::run` (`lexicon_request::Method::*`). Use Rust as the canonical source of truth — anything missing on the Rust side is a different audit (proto-vs-dispatch drift, P1).

3. For each dispatchable op (e.g. `NormalizeTone`, `NfdPreprocessForLookup`, `ProcessCandidates`):
   - iOS: search `ios/Sources/TaigiKeyboard/Engine/RustEngineBridge.swift` for a `public static func` whose body builds the op's payload type.
   - Android: search `android/app/src/main/.../engine/RustEngineBridge.kt` for a `fun` whose body builds the matching payload.

4. Report ops with both a proto entry and a Rust dispatch arm but no platform wrapper on iOS or Android.

**Edge cases**:

- Test-only ops (e.g. `panic_for_test` outside the method oneof) are exempt by virtue of the scoped enumeration in step 1.
- Ops added in the current branch but not yet exposed are flagged P2 with "in flight" caveat — verify against the active feature branch.

**Report**: P2 with the missing platform's bridge file path.

### G — Memory + doc hygiene (P3)

**Goal**: `MEMORY.md` index entries point to existing files; obsolete `feedback_*.md` are pruned; release map labels match reality.

**Detect**:

1. Derive the per-repo memory directory portably — Claude Code stores per-repo memory under `~/.claude/projects/<dash-encoded-repo-root>/memory/`. Compute the path from the current git toplevel rather than hard-coding any single user's home:
   ```bash
   repo_root="$(git rev-parse --show-toplevel)"
   mem_dir="$HOME/.claude/projects/$(echo "$repo_root" | sed 's|/|-|g')/memory"
   if [ ! -d "$mem_dir" ]; then
     echo "memory dir not found for repo $repo_root — skipping Dimension G"
     # graceful skip; Dimension G is a no-op when memory is unavailable (CI, fresh clone)
   fi
   ```

2. Cross-check `MEMORY.md` index entries against the resolved `mem_dir`:
   ```bash
   grep -E "\[.*\]\(.*\.md\)" "$mem_dir/MEMORY.md"
   ```

3. For each cited file path (`feedback_*.md`, `project_*.md`), verify the file exists in the memory dir.

4. Read `$mem_dir/project_rust_migration_cadence.md` release map; cross-check version labels against `git log --oneline -20` for tagged releases.

5. Flag obsolete strategies — memories whose framing was reversed by a later slice (e.g. `feedback_jvm_test_jni_compat.md` was reversed by path G in v3.5.3-followup; should already be deleted, but verify).

**Report**: P3 with the memory dir paths to clean up. If `mem_dir` does not exist (CI, fresh clone, different developer), Dimension G silently skips — the seven-dimension verdict still computes from A-F.

## Report template

Output file shape — adapt section counts to actual findings:

```markdown
# Migration Residue Audit — <YYYY-MM-DD>

> Branch: `<branch>` (HEAD `<sha>`)
> Run by: `/migration-residue` skill
> Last slice: `<vX.Y.Z>` (<short label>)

## Verdict

- **<N> P1 findings** (must fix before next slice)
- **<N> P2 findings** (recommended cleanup)
- **<N> P3 findings** (hygiene, defer if pressed)

Overall: PASS / NEEDS-CLEANUP / BLOCKER

## P1 — Critical

### A — Cross-language duplication

(file:line citations + recommended fix per finding)

### E — Build-broken JVM tests

(...)

## P2 — Recommended

### B — Cross-crate duplication

(...)

### C — Over-public Rust surface

(...)

### F — Bridge surface parity

(...)

## P3 — Hygiene

### D — Stale doc comments

(...)

### G — Memory + doc hygiene

(...)

## Summary table

| Dim | P1 | P2 | P3 | Notes |
|---|---|---|---|---|
| A — Cross-lang duplication | | | | |
| B — Cross-crate duplication | | | | |
| C — Over-public surface | | | | |
| D — Stale comments | | | | |
| E — Build-broken JVM tests | | | | |
| F — Bridge surface parity | | | | |
| G — Memory hygiene | | | | |

## Suggested next slice scope

(if P1+P2 findings cluster around a single slice candidate, propose it as the next refactor PR; otherwise leave blank)
```

## How to invoke

```
/migration-residue                       # full audit
/migration-residue --dimensions A,C,E    # subset
/migration-residue --codex               # +Codex second-opinion pass
```

When `--codex` is set, after writing the local report, hand it to Codex with this framing:

```
You are doing a second-opinion pass on the migration-residue audit at <report-path>. The local audit ran the seven dimensions described in .claude/skills/migration-residue/SKILL.md. Verify each P1 finding with file:line evidence. Refute or confirm. Flag any dimension the local audit missed. Format reply as P1 / P2 / P3 with concrete fix suggestions per finding.
```

Apply Codex's corrections back to the report inline, marking with "(Codex revised)" so the human reader knows which findings survived the second pass.

## Anti-goals

- **Do not auto-fix.** Findings are diagnostic; fixes go in a separate slice per `feedback_round_hygiene.md`.
- **Do not propose new release versions.** Suggested next-slice scope is descriptive, not prescriptive (per `feedback_no_future_planning.md`).
- **Do not re-flag retained-by-design platform helpers**:
  - iOS `CandidateProcessor` `isHanzi` / `capitalize` / `startsWithRomanLetter` — platform-only, not Rust mirrors.
  - URL builders — URL semantics are platform-owned per v3.5.3-followup `ExternalLookupURLBuilder` docstring.
  - JVM-test compat retention is reversed since v3.5.3-followup; if a memory still cites `feedback_jvm_test_jni_compat.md`, flag it as G/P3.
- **Do not run platform builds.** Static-analysis only. Per `feedback_manual_build_test.md`, `make build` / `gradlew` / `xcodebuild` are user actions.

## Cross-refs

- `docs/engine/v3.5.3-cleanup-audit.md` — engine workspace cleanup precedent.
- `docs/engine/v3.5.3-followup-jvm-duplication-audit.md` — path-G JVM-duplication precedent.
- `rules/rust-best-practices.md` §3a — domain↔proto boundary rule that pins the canonical façade shape.
- `feedback_path_g_delete_mirrors.md` — "delete platform mirrors when slice migrates" pattern.
- `feedback_codex_review_sandwich.md` — pre-impl + post-impl review gates.
