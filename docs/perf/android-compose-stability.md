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

## Step 2 observability gate — re-run procedure

The acceptance gate from § "Step 2 outcome" is **qualitative dogfood**
per `~/.claude/rules/code-review-rules.md §9`. It is not a one-shot
artifact — re-run after any branch that touches `ime/text/smartbar/`,
`ime/dictionary/TaigiWord.kt`, `ime/text/composing/ComposingManager.kt`,
or any `@Composable` reachable from `SmartbarContainer`. The procedure
below captures both the static pre-screen + the device dogfood.

### Baseline (post-step-2 ship)

Module totals from the `f14f4e0b` (B10 step 1) + `5077915e` (B10 step 2)
release-with-reports build held below — these are the comparison anchors
for every later re-run:

| Metric | Baseline value |
|---|---|
| `totalComposables` | 421 |
| `skippableComposables` | 312 |
| `restartableComposables` | 417 |
| `markedStableClasses` | 2 |
| `inferredStableClasses` | 127 |
| `inferredUnstableClasses` | 94 |
| `knownStableArguments` | 5399 |
| `knownUnstableArguments` | 91 |
| `featureFlags.StrongSkipping` | `true` |

Hot-path Composables at baseline (all `restartable skippable`, none with
unstable params after the `TaigiWord` config entry):

- `TaigiCandidateStrip(state: CandidateStripState)`
- `EnglishCandidateStrip(state: CandidateStripState)`
- `CandidateCell(word: TaigiWord, …)`
- `EnglishCandidateCell(…)`

### Step A — static pre-screen

Before running the build, grep the diff range for shapes that move
stability:

```bash
git diff --stat 5077915e..HEAD -- \
  android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/smartbar/ \
  android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/TaigiWord.kt \
  android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/composing/ \
  android/app/compose_compiler_config.conf
```

Flag (re-run mandatory) when the diff:

- adds / removes a `@Composable` in `ime/text/smartbar/` or any reachable
  caller of `SmartbarContainer`;
- adds a `data class` / `class` that is a parameter to an existing
  hot-path `@Composable`;
- mutates `TaigiWord` — new `var`, new mutable collection field, or any
  property whose producer doesn't guarantee structural immutability;
- changes `compose_compiler_config.conf`;
- swaps a `StateFlow<T>` into a Composable parameter slot (Composables
  should still take the collected `.value`, not the `StateFlow`).

Skip (re-run optional) when the diff is confined to internal
collaborators that are never `@Composable` params — e.g. B5's
`TextInputKeyHandler` / `KeyboardUiCoordinator` / `ImeKeyEventDispatcher`
are `internal class` IME plumbing, B9's `ComposingManager` exposes
`StateFlow<String>` / `StateFlow<Boolean>` / `StateFlow<Int>` collected
via `.value` (primitive — stable).

### Step B — release report build (USER)

USER runs (mid-round builds are USER's responsibility per project
CLAUDE.md § Build & Test — Claude does not invoke `./gradlew` outside
the post-PR parallel-verification exception):

```bash
cd android
./gradlew :app:assembleRelease -PenableComposeCompilerReports=true
```

Outputs land under `android/app/build/compose_compiler/`. Read in this
order:

1. `release/app-module.json` — module totals; compare line-by-line to
   the baseline table above.
2. `app-classes.txt` — grep for new `unstable class …` entries under
   `com.siansiansu.taigikeyboard.ime.text.smartbar.` or
   `com.siansiansu.taigikeyboard.ime.dictionary.`.
3. `app-composables.txt` — grep for hot-path Composable signatures and
   verify the `restartable skippable` flag is still set with no unstable
   params.

### Step C — per-keystroke dogfood (USER)

On a real Android device with the release build installed, exercise the
candidate strip through the §9 qualitative gate (`docs/architecture/dogfood-checklist.md`
S1–S3):

- **S1 POJ diacritics** — type a long POJ word with multiple tone marks
  (e.g. `kerngerngeh` → 砍砍下), scroll the candidate list, tap a
  middle-of-list candidate.
- **S2 TPS composition** — type a TPS sequence with nasal coda
  (e.g. `ㄉㄞㄨㄢㄉㄞㆣㄧ` → 臺灣台語), commit via candidate tap, then
  continuous-input a follow-up syllable.
- **S3 Hanji candidate scroll** — type a high-frequency Hanji prefix
  (e.g. `hak` → 學校 / 學生 / …), fast-scroll the candidate row at least
  one screen-width before tapping.

Watch for: visible recomposition flicker on the candidate strip, scroll
jank, candidate-cell text re-layout between keystrokes, or candidate-strip
freeze during continuous-input segmentation. None of these should be
worse than the baseline build.

### Decision rule

| Observation | Action |
|---|---|
| `app-module.json` regressions: `inferredStableClasses` ↓ AND a new hot-path Composable has an unstable param | Open a B10 step 3 round; apply the option A/B/C/D/E decision rule above to the new unstable class |
| `app-module.json` deltas confined to non-hot-path classes (e.g. settings-screen DTOs) | Note in the round summary; no new round |
| Dogfood S1/S2/S3 all behave like baseline | Gate clean — close round |
| Dogfood shows visible regression on a Composable that the report says is `skippable` | Likely a `mutableStateOf` / collection-mutation bug, not a stability bug. Trace the producer, not the annotation |

A no-regression observation closes the gate for this branch. The next
re-run fires when Step A's flag list trips again.

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
