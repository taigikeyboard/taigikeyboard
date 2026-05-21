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
