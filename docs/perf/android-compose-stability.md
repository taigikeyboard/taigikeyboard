# Android Compose Stability — Report Generation & Decision Rule

B10 step 1 sets up the Compose compiler stability report + a stability
configuration file. **No** `@Stable` / `@Immutable` annotations are added in
step 1, and no Composable / data-class code is changed. The annotation /
collection-swap work is **step 2** — gated on the report's findings and on
per-keystroke dogfood.

## Why this exists

The candidate strip (`SmartbarCandidateStrip` + friends) is re-composed on
every keystroke. Today `@Stable` / `@Immutable` is used in **2** places
across **86** `@Composable` declarations, and the obvious models cannot be
naively marked `@Immutable`:

- `CandidateStripState.kt:26` — `CandidateMode.Taigi` holds
  `List<TaigiWord>`.
- `TaigiWord.kt:33` — holds `Map<String, String> additionalInfo`.

Naively annotating these as `@Immutable` would be aspirational, not
structural — the Compose compiler still infers `List` / `Map` as unstable.
The actionable move is: **generate the compiler's stability report, read
which classes are flagged unstable + why, then either declare them stable
through the stability config file (when the immutability contract is truly
held), switch to genuinely immutable collections, or split state**.

## Generate a report

Reports are opt-in (default builds are unchanged). Use a **release** build —
debug builds include dev-only annotations that can mislead the report:

```bash
cd android
./gradlew :app:assembleRelease -PenableComposeCompilerReports=true
```

Outputs land under `android/app/build/compose_compiler/` (the plugin may
nest a per-compilation subdirectory for metrics). Files include:

| File | Use |
|---|---|
| `app_release-classes.txt` | Per-class stability inference + the structural reason — read this to drive step-2 decisions |
| `app_release-composables.txt` | Per-`@Composable` signature with `restartable` / `skippable` / `readonly` flags |
| `app_release-composables.csv` | CSV metrics per `@Composable` (counts of parameters, stable/unstable params, etc.); read this for bulk analysis, not as a `.txt` replacement |
| `app_release-module.json` | Module-level totals — stable vs unstable class counts, restartable / skippable composable counts |

Default build (no property):

```bash
./gradlew :app:assembleDebug      # byte-identical APK to pre-B10
./gradlew :app:assembleRelease    # byte-identical APK to pre-B10
```

Reports are **not** generated. The compose-compiler plugin still receives
the (comment-only) stability config path, so Kotlin compile-task cache keys
shift once relative to pre-B10 — there is no runtime / packaged-artifact
delta. The directory `app/build/compose_compiler/` is **not** created when
the property is absent.

## Reading the classes report

Each line in `*-classes.txt` looks like:

```
unstable class CandidateMode {
  stable val mode: ...
  unstable val items: List<TaigiWord>
  <runtime stability> = Unstable
}
```

| Token | Meaning |
|---|---|
| `stable class X` | All fields are stable; Compose can skip re-composition when `X` is structurally equal |
| `unstable class X` | At least one field is unstable; Compose treats every call site as "changed" |
| `runtime class X` | Stability depends on a generic parameter actually substituted at call site |

The composables report flags each `@Composable` as `skippable` /
`restartable` / `readonly`. **Hot-path Composables** that are
`restartable, not skippable` are the highest-value step-2 targets.

## Stability configuration file

`android/app/compose_compiler_config.conf` is wired into the build
unconditionally. Format:

- One fully-qualified class name per line.
- `//` for comments; blank lines ignored. Comments must start at the
  beginning of the line — trailing inline `//` after a pattern is **not**
  stripped and will be parsed as part of the pattern.
- **No** `#` comments — the JetBrains `StabilityConfigParser` reads `#`
  lines as type patterns and fails the build.
- `*` matches a single package segment; `**` matches any depth; generic
  parameters can be matched with `<*,_>` syntax (`*` = stability-affecting,
  `_` = stability-blind).
- Missing file = build failure (the parser reads the path directly). The
  empty starter file shipped by B10 step 1 satisfies the existence check.

Examples (not added yet — step 2 decides):

```config
// Treat candidate metadata map as immutable by construction.
com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
// Treat all per-keystroke UI state as stable.
com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateStripState
com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateMode
com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateDisplayParams
```

## Step-2 decision rule

After the report runs, classify each unstable hot-path class against the
options below (Codex pre-impl B10 verdict — keep ALL options in scope):

| Option | When to pick |
|---|---|
| **A. Stability config entry** | The structural immutability contract is genuinely held by every producer (e.g. the `Map`/`List` is never mutated after construction). Cheapest fix. |
| **B. Immutable collection** | The producer cannot guarantee structural immutability today, but a swap to `kotlinx.collections.immutable.ImmutableMap` / `ImmutableList` is bounded. |
| **C. Split state** | The class mixes hot per-keystroke fields with cold rarely-changing fields. Hoist the cold half. |
| **D. UI-specific DTO** | The unstable class is shared-core / domain-shaped (e.g. `TaigiWord` is consumed by autocomplete + click handler + smartbar). Introduce a narrower UI-facing DTO instead of mutating the domain type. |
| **E. Do nothing** | The report shows the call site is already `skippable` / strong-skipping is doing its job — leave it alone. |

**Do not** blanket-declare `kotlin.collections.*` stable at the project
level. That treats every `List<*>` / `Map<*, *>` everywhere as stable; the
hot path benefits, but unrelated code paths inherit a contract their
producers do not honor.

## Behaviour-neutral guarantee for step 1

- `assembleDebug` / `assembleRelease` without the property emit no extra
  artifacts and produce a byte-identical APK to pre-B10.
- `assembleRelease -PenableComposeCompilerReports=true` only adds files
  under `app/build/compose_compiler/` (build dir, not packaged in the APK).
- The empty stability config file has no compiled effect — the parser
  ignores `//` comments and blank lines.

No `@Composable` code, candidate-strip code, or runtime path is touched in
step 1. Step 2 will be a separate slice, on its own branch, with the
per-keystroke dogfood gate from `~/.claude/rules/code-review-rules.md §9`.

## Step 2 outcome

A release-with-reports build was run against `f14f4e0b` (B10 step 1 ship) +
the candidate-strip code as it stood at that commit. Module totals:

| Metric | Value |
|---|---|
| `totalComposables` | 421 |
| `skippableComposables` | 312 (74%) |
| `markedStableClasses` | 2 (pre-existing emoji) |
| `inferredStableClasses` | 125 |
| `inferredUnstableClasses` | 96 |
| `knownUnstableArguments` | 92 / 5530 (1.7%) |
| `featureFlags.StrongSkipping` | `true` |

All four candidate-strip Composables are already `restartable skippable`:

| Composable | Status | Unstable params |
|---|---|---|
| `TaigiCandidateStrip(state)` | skippable | none — `state` is stable |
| `EnglishCandidateStrip(state)` | skippable | none — `state` is stable |
| `CandidateCell(word, …)` | skippable | `word: TaigiWord` |
| `EnglishCandidateCell(…)` | skippable | none |

Suspect classes resolved:

| Class | Pre-run guess | Report verdict |
|---|---|---|
| `CandidateStripState` | suspect | already `stable` |
| `CandidateMode` (sealed parent) | suspect | already `stable` |
| `CandidateMode.Empty` | n/a | `stable` |
| `CandidateMode.Taigi` | suspect | `unstable` (sole reason: `items: List<TaigiWord>`) |
| `CandidateMode.English` | suspect | `unstable` (sole reason: `items: List<TaigiWord>`) |
| `CandidateDisplayParams` | suspect | already `stable` |
| `TaigiWord` | suspect | `unstable` (sole reason: `additionalInfo: Map<String, String>`) |

### Per-class decision

**`TaigiWord` — Option A (stability config entry).**

The only hot-path Composable taking an unstable param is `CandidateCell(word:
TaigiWord)`. With strong-skipping on, unstable params are compared with
instance equality (`===`) — practically never skipped, since the candidate
list is rebuilt fresh on every push (`SuggestionCaseTransformer.transform` →
`mapIndexed { … TaigiWord(…) }`). Declaring `TaigiWord` stable promotes the
comparison to `Object.equals()`, so structurally equal cells (common on
`toggleTranslateSwapped` re-pushing the same suggestions, or on continuous
re-derivation that produces the same row content) actually skip.

Producer audit (`TaigiAutocompleteService.kt:116` /
`NextWordHandler.kt:436` / `CandidateUpdateCoordinator.kt:242` /
`LexiconBridge.kt:703`) confirms `additionalInfo` is always built via
`mapOf(…)` or default `emptyMap()` and never mutated after construction;
the rest of the fields are `val` primitives or `String`. Option B
(`kotlinx.collections.immutable.ImmutableMap`) was rejected as
disproportionate: it would add a transitive dependency and force four
producer sites + every future caller to use `persistentMapOf`, all to
enforce a contract the current code already keeps. The stability-config
entry carries a CONTRACT comment in both `compose_compiler_config.conf`
and the `TaigiWord` KDoc so a future contributor adding a `var` or mutable
collection will see the obligation before shipping.

**`CandidateMode.Taigi` / `CandidateMode.English` — Option E (do nothing).**

Neither subtype is taken directly by any `@Composable`. The hot consumers
are `TaigiCandidateStrip(state: CandidateStripState)` and
`EnglishCandidateStrip(state: CandidateStripState)`, both of which take the
sealed-parent-typed `state.mode` indirectly — and `CandidateStripState` is
already `stable`. `updateSeq` already forces re-evaluation on every push
(by design — drives `LaunchedEffect` scroll-to-zero). Stabilizing the
nested unstable subtypes would change report cosmetics without changing
the skip behavior at any actual call site.

**Other suspects already stable.** `CandidateStripState`,
`CandidateDisplayParams`, `CandidateMode` (sealed parent), and
`CandidateMode.Empty` need no entry.

### Behaviour-neutral guarantee for step 2

The diff is a stability-config entry + KDoc + this doc section. No code
path is rewritten; no producer is touched. Compose's per-cell strong-skip
comparison flips from `===` to `equals()` for `CandidateCell(word)`,
unlocking structural skips when the same `TaigiWord` value is re-pushed.
The acceptance gate is a release-build per-keystroke dogfood on the
candidate strip per `~/.claude/rules/code-review-rules.md §9` (qualitative,
no P50/P95). A no-regression dogfood = ship.

## References

- Android — Compose Compiler Gradle plugin setup:
  <https://developer.android.com/develop/ui/compose/compiler>
- Android — Diagnose Compose stability:
  <https://developer.android.com/develop/ui/compose/performance/stability/diagnose>
- Android — Fix Compose stability:
  <https://developer.android.com/develop/ui/compose/performance/stability/fix>
- Kotlin — Compose Compiler Gradle plugin DSL:
  <https://kotlinlang.org/api/kotlin-gradle-plugin/compose-compiler-gradle-plugin/>
- Now in Android — `compose_compiler_config.conf` precedent:
  <https://github.com/android/nowinandroid>
- Jetpack Compose Samples — PR #1606 added the same shape:
  <https://github.com/android/compose-samples/pull/1606>
- Chris Banes — Tivi metrics writeup (opt-in property gate prior art):
  <https://chris.banes.dev/composable-metrics/>
