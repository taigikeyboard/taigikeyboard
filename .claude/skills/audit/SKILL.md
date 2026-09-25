---
name: audit
description: Whole-repo maintainability audit — dead engine ops / bridges / fields, logic duplicated across platform twins (iOS↔Android, macOS↔iOS, Windows↔Linux), redundant fallbacks, layering violations, over-design, doc drift. Produces a ranked, file:line-cited report with estimated LOC and a draft round order. Use when asked to "audit", find dead code, assess architecture / maintenance cost, or before a large feature. Read-only — never edits, branches, builds or opens a round. Args: scope `engine` | `mobile` | `desktop` | `all` (default all). Diff-level quality is /simplify; bugs are /code-review; refactor behaviour-freeze is the refactor-reviewer agent.
disable-model-invocation: false
---

# Maintainability audit

Answer one question: **what in the tree costs maintenance without paying for it, and which rounds would remove it?**

Output is a report; the USER decides which rounds open (`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope). Never write "deferred", "post-vX" or "known limitation".

Worked example of the finished product: `docs/reports/2026-09-24-mobile-smart-suggestions-brainstorm.md` §3.

## 1. Scope → surfaces

| Scope | Surfaces |
|---|---|
| `engine` | `engine/*/src`, `engine/protos/proto/*.proto`, crate graph (`.claude/rules/rust-best-practices.md` §1a) |
| `mobile` | `ios/Sources/TaigiKeyboard`, `android/app/src/main/java/.../taigikeyboard`, their bridges (`RustEngineBridge+*.swift`, `engine/*Bridge.kt`) |
| `desktop` | `macos/Sources`, `desktop/crates`, `windows/`, `linux/` |
| `all` | all of the above + `docs/architecture/*` for drift |

Always exclude generated code from every count and every "caller" match: `*.pb.swift`, `android/.../engine/proto/*.java`, `Strings/Generated/`, `target/`, `build/`.

## 2. Recipes — one read-only agent per recipe, launched in one message

Give each agent the scope, its recipe and the § 3 verification rules; ask for `file:line` rows, not prose. Recipes that do not apply to the scope are skipped.

**R-dead · dead surface.** For every `oneof method` entry in `engine/protos/proto/*.proto`: engine dispatch arm → platform bridge function → **production** callers (outside tests, outside the bridge file, outside generated code). No production caller on any platform = dead. Same walk for proto fields (set by nobody or read by nobody), bridge functions, public Swift/Kotlin types, settings keys still plumbed after their UI was removed, unused parameters.

**R-twin · platform twins.** Diff the same responsibility across twins: iOS↔Android by file name (`NextWordService.swift`↔`.kt`, `ComposingManager`, `*Bridge`), macOS stores ↔ iOS stores, Windows↔Linux over `desktop/crates`. Flag (a) logic both sides implement that could live once in the engine, (b) twins that already drifted. For every drift, record **whether a user can observe it** (§ 3.2).

**R-fallback · redundant paths.** `~/.claude/rules/planning.md` § No redundant fallback: a second path that re-implements the primary (e.g. a platform mirror of engine bit math), "A fails → B" in data flow, legacy code kept for a caller that no longer exists.

**R-shape · layering + over-design.** Crate edges against `rust-best-practices.md` §1a; iOS layer imports against `.claude/rules/ios-architecture.md` §2; concrete over-design only: a protocol / trait / wrapper with one implementation and no test seam, a parameter every caller passes the same value, a config flag never flipped, two FFI hops where one would do. Size alone is not a finding — a large file is flagged only when it holds more than one concern (`rust-migration-policy.md` §1.4).

**R-drift · docs vs code.** `docs/architecture/system-overview.md`, `behavioral-invariants.md` platform sites, `docs/engine/migration-inventory.csv`, rule files: paths, symbols and counts that no longer match the tree.

## 3. Verification — main session, before anything reaches the report

1. **Callers are searched in every spelling.** Swift: `Type(`, `.init(`, `Type.init(`, `= .init(` at a typed declaration; Kotlin: `Type(`, extension-function calls without the receiver; Rust: `path::fn`, `use` re-exports; plus string op names in bridges (`op: "…"`). A missed `.init(` once broke an iOS build (R3 PR-b).
2. **Drift is only a finding when observable.** Trace each "iOS does X, Android does Y" to an input a user can type. In the R3 audit 4 of 5 drifts were unreachable (dictionary has no such word, engine never emits the value, the branch is dead); only the Ext-B `𣍐` key split was real (#195). Unreachable drift = dead-code cleanup, not a parity round.
3. **High-severity rows are verified by hand** (grep / Read in the main session) and marked **verified**; the rest carry the agent's `file:line` and "re-grep before quoting".
4. **LOC estimates exclude generated code** and are marked `~`.
5. Follow `.claude/rules/cross-platform-alignment.md` §3b: an inline `// matches iOS` / `CROSS-PLATFORM INVARIANT` comment means intended parity — divergence there is a bug candidate, reported as such.

## 4. Report

Write `docs/reports/<YYYY-MM-DD>-audit-<scope>.md` (English, dated snapshot — frozen afterwards):

1. **Summary** — one paragraph: the biggest cost and the recommended first round.
2. **Findings table** — `| Sev | Finding | Where (file:line) | Observable? | Est. LOC |`, High → Low. Sev: **High** = duplicated across platforms or dead surface > ~200 LOC; **Med** = single-platform dead code, drift, over-design; **Low** = naming, comments, doc drift.
3. **Justified-looking layers** — structure that looks heavy but earns its place (so the next audit does not re-flag it).
4. **Draft round order** — unscheduled, each PR 200–500 LOC (`~/.claude/rules/planning.md`), with the round type (refactor / parity / bugfix) and its pre-gate. Observable drift goes to its own `parity:` PR (`cross-platform-alignment.md` §1b).
5. **Open USER decisions** — ranked, each with a recommendation.

Then reply in chat with the summary, the top 3 rows, and one next action. Update the matching project memory file with the report path.

## 5. Never

Edit source, create a branch, run a build, open a PR, or start a round from this skill. Merge findings into an existing report only when the USER asks.
