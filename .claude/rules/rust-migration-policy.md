---
paths:
  - "engine/**"
  - "docs/engine/migration-inventory.csv"
  - "ios/Sources/TaigiKeyboard/Engine/**"
  - "android/app/src/main/java/com/siansiansu/taigikeyboard/engine/**"
---

# Rust Migration Policy

Mandatory rules before any platform impl → Rust engine swap, new Rust slice (in or after Phase IV-B which closed 2026-05-05), `.proto` addition, or platform-mirror delete. Companion to `.claude/rules/rust-best-practices.md` (how the Rust code itself should look).

## 1. Four design goals per slice

Every Rust slice must satisfy ALL four:

1. **High cohesion, low coupling.** Each crate owns ONE engine concern with no cross-imports beyond `protos` + `dispatch`. Cross-module deps are minimal and explicit.
2. **Rust idioms.** `unsafe_code = "forbid"`, `thiserror` typed errors, `prost` for proto, `Mutex<T>` for shared state, `once_cell` for static init, `#[cfg(test)]` fixtures, no panics in non-test paths (use `Result`).
3. **Single responsibility naming.** File name = role (`trie.rs` not `util.rs`); function name = action verb (`lookup_prefix` not `process`); variable name = content (`row_ids` not `result`). One file / function / variable = one thing.
4. **Idiomatic file organization.** Defer to Rust convention: tests live inline via `#[cfg(test)] mod tests { }`. Files split by sub-concern (cohesion-driven), NOT by line count. **No hard LOC cap** (revised 2026-05-07). Use length as a smell signal — "does this file actually own one concern?" — not a blocker. A 650-LOC file with one concern + cohesive tests reads better than 2 files with `#[path]` indirection.

Examples already aligned: `engine/nextword/{lib,api,dispatch,handle,decide,filter,scorer,booster}.rs` — 8 files for 1 crate, each owning one concern. `engine/composing/{lib,api,dispatch,handle,transition,derived,continuous,shadow}.rs` — same pattern, 8 files. Platform-side bridge files split by slice too (`RustEngineBridge+<Slice>.swift`).

## 2. No slice toggles

Each slice ships as a **direct swap** with no fallback code path. Old Swift/Kotlin impl is DELETED in the same PR. No `useXxxRust: Bool` toggle, no parallel implementations.

- Solo maintainer with direct release control; revert-PR + cut hotfix is the rollback mechanism.
- Dogfood gate is the production gate (S1/S2/S3 + no dismiss + leak-free per `~/.claude/rules/code-review-rules.md` §9; concrete Taigi acceptance sequences in `.claude/rules/taigi-incidents.md` § Qualitative perf gate).
- Toggle adds permanent cost: dual-path maintenance, doubled test matrix, binary growth, rotting dead code.
- Reference IMEs (McBopomofo, khiin-rs) don't toggle engine implementations.
- If a slice "feels like it needs a toggle", that signals the slice is too large — split it.

## 3. Bridge swap preserves the surrounding pipeline

When swapping `OldHelper.x(...)` → `RustEngineBridge.x(...)`, the bridge op is rarely a 1:1 replacement. The original call site often has:

- Adjacent preprocessing (e.g. `TaigiUnicode.nfdPreprocessed` before `stripTone`).
- Mode/flag mapping that must forward ALL enum cases (e.g. ENGLISH must NOT collapse to TL).
- Joiners / separators (iOS `joined(separator: " ")`, Android `joinToString(" ")` — NOT `""`).
- Per-token decision branches (e.g. `or_maps_to_er` is a per-vowel-token override inside `to_zhuyin`, NOT a whole-string `.replace("er","or")`).

When the swap drops or collapses these, the bridge call appears to "work" but produces subtly wrong output on edge inputs (POJ `o͘`, multi-syllable, dialect toggles, English mode).

Procedure for each swap:

1. Diff BEFORE vs AFTER with 5-10 lines of context, not just the call line.
2. Check NFD / case / mode / joiner / special-case logic preservation.
3. Cross-check iOS vs Android side-by-side — divergence is a strong tell that one swap dropped something.
4. Add regression-prone modules (`stripTone+NFD`, exhaustive mode `when`/`switch`, per-token vowel overrides, multi-syllable joiner) to the Codex pre-impl prompt explicitly.

Incident: D9.4 PR #186 had 4 post-merge regressions, all the same root cause — focus on "route X through bridge" lost track of "what wraps X". Symbol grep found 0 hits but dropped preprocessing wasn't a symbol reference.

## 4. Proto generation: triple-touch on new .proto

When adding a new `.proto` file under `engine/protos/proto/`, update ALL THREE:

1. `engine/protos/build.rs` (Rust side via `prost`).
2. `engine/scripts/gen-platform-protos.sh` — both `--swift_out` and `--java_out=lite` blocks.
3. Run the script and commit the regenerated `.pb.swift` (iOS) + `.java` (Android) files in the same commit or the immediate next commit. Platform bindings are **checked into git, NOT build-time generated**.

Without this, bridge code references generated types that don't exist; the build silently breaks until next ad-hoc regen. Incident: PR #205 case-transform slice — Codex post-impl caught it as a BLOCK; the fix added `case.proto` and re-emitted ~140 `.java` files (most no-op trailing whitespace; semantic diff was envelope + new case files only).

## 5. Path G — delete platform mirrors when slice migrates

When a slice ships to Rust, **DELETE** the Kotlin/Swift mirror sources + their JVM unit tests in the same slice. Do NOT keep them as JVM-test compat shims.

- Solo maintainer; real-device dogfood is the production gate; Rust workspace has comprehensive algorithm tests.
- Reference IMEs (`khiin-rs`, `McBopomofo`) ship thin bridges with no platform-side math test coverage.
- iOS XCTest can stay because `xcframework` links statically into the test binary; iOS-side FFI-boundary parity tests (`RustEngineBridgeRankingTests`, etc.) keep their value.

Procedure when slice swaps `PlatformHelper.x()` → `RustEngineBridge.x()`:

1. Delete `PlatformHelper.{swift,kt}` whole file (or migrated functions if the file has out-of-slice helpers).
2. Delete the corresponding `*Test.kt` JVM tests entirely. Verify parity tests in `engine/<crate>/tests/*.rs` cover the deleted JVM cases before deletion.
3. Fold non-migrated platform callers (cold-start fallbacks, URL builders) into the Rust round-trip too.
4. If a B-class JVM test has been silently build-broken since a prior slice deleted referenced symbols, **delete it** as artifact cleanup. Check `git grep` for refs to deleted symbols (e.g. `TaigiPhonetics`, `ToneRestoration`, `TPSConverter`) inside `src/test/`.

Out of scope (still platform-owned): UI layer / KeyboardKit / FlorisBoard adapter, IME lifecycle, settings storage backends, logging sinks, URL semantics in URL builders.

## 6. User-data SQLite stays platform-native

Do NOT propose moving user-writable SQLite to Rust shared core:

- `user_association.db` (NextWord user-learned bigrams)
- `user_frequency.db` (per-word selection frequency)
- `custom_dictionary.db` (user-added entries)

User-write data volume is small + non-urgent; platform-native SQLite integrates with Android backup APIs, iOS App Group / iCloud KeyValueStore, encryption hooks, debugger tooling. Cross-platform schema parity is already maintained at the service layer. Revisit ONLY if a concrete cross-platform schema-evolution need appears (e.g. shared backup format requiring identical serialization).

This rule overrides the default "every SQLite call site is a shared-core candidate" instinct.
