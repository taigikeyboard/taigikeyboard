# i18n codegen

Canonical app-UI string sources live in `i18n/*.json` (one file per namespace, ARB-shaped). This
tool generates the platform artifacts; **do not hand-edit the generated files.**

## Commands

| Command | Effect |
|---|---|
| `make i18n` | Regenerate all artifacts from `i18n/*.json` (commit the result). |
| `make i18n-check` | Fail if committed output is stale (no worktree mutation). |

The Android Gradle `checkI18nGenerated` task runs the same checker on `preBuild`, so Android Studio /
archive / `assemble` builds cannot link stale output.

## Generated artifacts (Android, R2a-1)

- `android/app/src/main/res/values/strings_i18n.xml` — Hanji default (the native-resource path).
- `android/app/src/main/java/com/siansiansu/taigikeyboard/i18n/generated/` — `StringKey`,
  `GeneratedTaigiStrings` (TL/POJ map), `GeneratedPseudoStrings` (debug layout probe), `L10n`
  (typed Compose accessors for plain keys), `StringResolverFormats` (typed non-Compose
  `StringResolver.<key>(args)` accessors for format keys). Excluded from spotless via `**/generated/**`.

## Source schema

```jsonc
{
  "namespace": "settings",
  "keys": {
    "inputMode": {
      "comment": "translator context",
      "scope": { "platforms": ["android", "ios"], "surfaces": ["host", "extension"] },
      "placeholders": { "count": "int" },   // optional; named {count}, never %d
      "values": {
        "hanji": "輸入模式",                 // required base language
        "ja": "入力モード",                   // authored per language phase
        "en": "Input Mode"
        // tailo/poj omitted -> derived/fallback in a later phase
      }
    }
  }
}
```

Rules enforced by the generator (the lint config disables `MissingTranslation`, so completeness is
the generator's job):

- Duplicate JSON keys are rejected (not silently last-wins).
- `scope.platforms` ⊆ {ios, android}, `scope.surfaces` ⊆ {host, extension}, both non-empty.
- Every key must define the base language (`hanji`).
- **Production completeness**: every key must author all user-selectable production languages
  (`hanji`, `ja`, `en` today — mirrors the platform `DisplayLanguage.productionLanguages` roster), so a
  picker option never renders a silent Hanji fallback. `tailo`/`poj` stay optional until they ship and
  join the roster. Enforced by `make i18n` / `make i18n-check` / Gradle `checkI18nGenerated` (not by the
  generic `build_outputs`, which tests drive with partial fixtures).
- Every authored language must carry the same `{placeholder}` set as the base (order may differ —
  substitution is by name, not position, so a reordered translation is allowed).

### Format keys (placeholders)

- A key whose base text contains `{placeholder}` spans MUST declare `placeholders` (name → type).
  The only supported type today is `int`; the name must be a lowerCamelCase non-keyword identifier.
- The canonical positional order (`%1$d`, `%2$d`, …) comes from first appearance in the base text;
  every language substitutes by name, so a translation may reorder the placeholders.
- Format keys do NOT get an `L10n` Compose getter (that would expose the raw `%1$d` template). They
  get a typed `StringResolver.<accessor>(name: Int, …)` function in `StringResolverFormats.kt`, backed
  by the hand-written `StringResolver.formatString` helper — call sites never touch a raw `%d`.

POJ derive-from-TL (via `taigi-converter`) is deliberately not wired until TL strings exist (P3c).
