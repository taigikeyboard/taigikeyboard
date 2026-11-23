---
name: audit
description: Whole-repo maintainability audit — dead engine ops / bridges / fields, logic duplicated across platform twins (iOS↔Android, macOS↔iOS, Windows↔Linux), redundant fallbacks, layering violations, over-design, doc drift, stale source comments. Produces a ranked, file:line-cited report with estimated LOC and a draft round order. Use when asked to "audit", find dead code, assess architecture / maintenance cost, or before a large feature. Read-only on code — writes only its dated report under `docs/reports/`; never edits source, branches, builds or opens a round. Args: scope `engine` | `mobile` | `desktop` | `all` (default all). Diff-level quality is /simplify; bugs are /code-review; refactor behaviour-freeze is the refactor-reviewer agent.
disable-model-invocation: false
---

# Maintainability audit

Answer one question: **what in the tree costs maintenance without paying for it, and which rounds would remove it?**

Output is a report; the maintainer decides which rounds open. Never write "deferred", "post-vX" or "known limitation".

Earlier `docs/reports/*-audit-*.md` reports illustrate the § 4 shape; their findings, length and round order belong to that run.

## 1. Scope → surfaces

| Scope | Surfaces |
|---|---|
| `engine` | `engine/*/src`, `engine/protos/proto/*.proto`, crate graph (`docs/contributing/rust-best-practices.md` §1a) |
| `mobile` | `ios/Sources/TaigiKeyboard`, `android/app/src/main/java/.../taigikeyboard`, their bridges (`RustEngineBridge+*.swift`, `engine/*Bridge.kt`) |
| `desktop` | `macos/Sources`, `macos/crates`, `desktop/crates`, `windows/`, `linux/` |
| `all` | all of the above + `docs/architecture/*` for drift |

Exclude generated code from every count and every "caller" match: `*.pb.swift`, `android/.../engine/proto/*.java`, `Strings/Generated/`, `android/.../i18n/generated/`, `desktop/crates/taigi-desktop-core/src/strings/generated.rs`, `target/`, `build/`.

## 2. Recipes — one read-only agent per recipe, launched in one message

Give each agent the scope, its recipe and the § 3 verification rules; ask for `file:line` rows, not prose. Recipes that do not apply to the scope are skipped.

**R-dead · dead surface.** For every `oneof method` entry in `engine/protos/proto/*.proto`: engine dispatch arm → platform bridge function → **production** callers (outside tests, outside the bridge file, outside generated code). No production caller on any platform = dead. Same walk for proto fields (set by nobody or read by nobody), bridge functions, public Swift/Kotlin types, settings keys still plumbed after their UI was removed, unused parameters.

**R-twin · platform twins.** Diff the same responsibility across twins: iOS↔Android by file name (`ComposingManager`, `NextWordController`, `*Bridge`), the `UserDataClient` twins (iOS / macOS / Android) over `engine/userdata`, the three desktops over `desktop/crates`, and the areas macOS keeps in Swift beside desktop-core (candidate-window geometry, settings storage, global shortcuts, update flow) ↔ their Windows / Linux twins. Flag (a) logic both sides implement that could live once in the engine, (b) twins that already drifted. For every drift, record **whether a user can observe it** (§ 3.2).

**R-fallback · redundant paths.** Per `AGENTS.md` § Design principles: a second path that re-implements the primary (e.g. a platform mirror of engine bit math), "A fails → B" in data flow, legacy code kept for a caller that no longer exists.

**R-shape · layering + over-design.** Crate edges against `rust-best-practices.md` §1a; iOS layer imports against `docs/contributing/ios-architecture.md` §2; concrete over-design only: a protocol / trait / wrapper with one implementation and no test seam, a parameter every caller passes the same value, a config flag never flipped, two FFI hops where one would do. Size alone is not a finding — a large file is flagged only when it holds more than one concern (`rust-migration-policy.md` §1.4).

**R-drift · docs vs code.** `docs/architecture/system-overview.md`, `behavioral-invariants.md` platform sites, `docs/engine/migration-inventory.csv`, rule files: paths, symbols and counts that no longer match the tree.

**R-comment · stale source comments.** A comment an agent trusts over the code steers later fixes and tests wrong, so a contradicted comment is worse than none. Scan comments in the scope's source that make a checkable claim: a path or symbol (`see foo.rs`, `` `fn_name` ``, `FooManager.bar`), an issue (`TODO(#N)`, `#N` — resolve pre-2026-09-07 numbers per `docs/architecture/pr-number-migration.md`), a value or count (`max 1.15`, `three platforms`), a counterpart (`Mirrors iOS`, `matches Android`, `same as X`). Flag (a) **stale** — the target is gone, renamed or the issue closed, or the claim contradicts the code it sits on → delete or correct; (b) **restating** — says what the next line already says → delete; (c) commented-out code and section banners → delete. A comment that explains a non-obvious *why* (platform limit, past incident) stays. Counterpart comments are checked here only for the counterpart's existence — whether the two sides still agree is § 3.5.

## 3. Verification — main session, before anything reaches the report

1. **Callers are searched in every spelling.** Swift: `Type(`, `.init(`, `Type.init(`, `= .init(` at a typed declaration; Kotlin: `Type(`, extension-function calls without the receiver; Rust: `path::fn`, `use` re-exports; plus string op names in bridges (`op: "…"`). A missed `.init(` caller turns a "dead" deletion into a broken build.
2. **Drift is only a finding when observable.** Trace each "iOS does X, Android does Y" to an input a user can type. Most alleged drifts turn out unreachable (the dictionary has no such word, the engine never emits the value, the branch is dead). Unreachable drift = dead-code cleanup, not a parity round.
3. **High-severity rows are verified by hand** (grep / Read in the main session) and marked **verified**; the rest carry the agent's `file:line` and "re-grep before quoting".
4. **LOC estimates exclude generated code** and are marked `~`.
5. **A stale-comment row cites both lines** — the comment's `file:line` and the code or missing target that contradicts it. A comment whose claim cannot be checked is not a finding.
6. Follow `docs/contributing/cross-platform-alignment.md` §3b: an inline `Mirrors iOS` / `matches Android` / `CROSS-PLATFORM INVARIANT` comment means intended parity — divergence there is a bug candidate, reported as such.

## 4. Report

Write `docs/reports/<YYYY-MM-DD>-audit-<scope>.md` (English, dated snapshot — frozen afterwards):

1. **Summary** — one paragraph: the biggest cost and the recommended first round.
2. **Findings table** — `| Sev | Finding | Where (file:line) | Observable? | Est. LOC |`, High → Low. Sev: **High** = duplicated across platforms or dead surface > ~200 LOC; **Med** = single-platform dead code, drift, over-design, a stale comment that contradicts the code; **Low** = naming, restating comments, commented-out code, doc drift.
3. **Justified-looking layers** — structure that looks heavy but earns its place (so the next audit does not re-flag it).
4. **Draft round order** — unscheduled, each PR 200–500 LOC, with the round type (refactor / parity / bugfix) and its pre-gate. Observable drift goes to its own `parity:` PR (`cross-platform-alignment.md` §1b). R-comment rows batch into one comment-only `chore:` PR per platform area (no behavior change, Low tier).
5. **Open USER decisions** — ranked, each with a recommendation.

Then reply in chat with the summary, the top 3 rows, the report path, and one next action.

## 5. Scope of this skill

The report is the only file it writes; a fix is a separate round. Merge findings into an existing report only when the USER asks.
