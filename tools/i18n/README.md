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
        "tailo": "su-ji̍p bôo-sik",          // authored (Tâi-lô)
        "poj": "su-ji̍p bô͘-sek",            // derived from tailo by `make i18n-derive-poj`
        "ja": "入力モード",                   // authored per language phase
        "en": "Input Mode"
        // a key omitting tailo/poj falls back to Hanji (the pair must be present together)
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

### POJ (Pe̍h-ōe-jī)

POJ is a deterministic transliteration of TL (ts→ch, tsh→chh, oo→o͘, nn→ⁿ, ua→oa, ...) with no semantics
of its own, so it is **derived-and-stored**, not hand-authored. `tools/i18n/derive_poj.py` (`make
i18n-derive-poj`) runs every `tailo` value through the canonical `taigi-converter` bridge
(`convert_tl_to_poj_strict`) and writes the result back as the `poj` value in `i18n/*.json`; `make i18n`
then emits it like any other language. Re-run the derive after any `tailo` correction — `--check` exits
non-zero if a committed `poj` is stale. The codegen itself reads `poj` from the JSON like every other
language (no Node at `make i18n` / `make i18n-check` time). tailo and poj are authored in lockstep:
`validate_generated_map_completeness` rejects a key that has one but not the other. (The earlier plan-D4
`pojOverride` / `derivePoj:false` / protected-span schema proved unnecessary — an audit of all real
strings showed the converter preserves every brand / acronym / `{placeholder}` token verbatim.)
