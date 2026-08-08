# taigi-emojis

Single emoji source of truth for the [Taigi keyboard](https://github.com/taigikeyboard)
(iOS + Android). A Python generator merges pinned Unicode + CLDR data with a hand overlay
of Taiwanese (台語) / 華語 search keywords and emits one `dist/emoji.json` both apps consume.

Currently pinned to **Unicode Emoji 17.0** (CLDR 48) — 1889 emoji.

## Layout

- `src/overrides.tsv` — the only hand-edited file (add/curate emoji + Taigi keywords)
- `scripts/generate.py` — the generator (stdlib-only, runs under bare `python3`)
- `data/` — pinned upstream snapshots (emoji-test.txt + CLDR xml); see `data/SOURCES.md`
- `dist/emoji.json` — the generated artifact both apps read directly (committed)
- `tests/` — golden specs + drift guard

## Add or curate emoji

Edit **`src/overrides.tsv`** (the only hand-edited file), then rebuild:

```bash
make build   # regenerate dist/emoji.json
make test    # golden specs + drift guard
```

`overrides.tsv` columns (tab-separated):

```
emoji  action  category  order  taigi_keywords  zh_Hant_keywords  en_keywords  name  notes
```

- `patch` — add keywords to an existing emoji (e.g. add 台語 search terms).
- `add` — add a brand-new emoji not in upstream.
- `exclude` — drop an upstream emoji.

Keywords are `|`-separated. For Taigi terms, use 漢字 and only verified 羅馬字 — never invent
romanization (see `.claude/rules/emoji-data-authoring.md`).

## Update to a new Unicode version

Version-pin bumps are deliberate. Bump `UNICODE_EMOJI_VERSION` + `CLDR_TAG` in the `Makefile`
and `MAX_EMOJI_VERSION` + `CLDR_VERSION` in `scripts/generate.py` together, then:

```bash
make fetch   # re-pull pinned upstream for the new pins
make build
make test
```

See `data/SOURCES.md` for origins, versions, and the version-pin rationale.

## Output

`dist/emoji.json` — categories → emoji (base, codepoints, name, skin-tone variations,
merged + per-locale keywords). Schema in `.claude/rules/output-contract.md`.

Both apps read `dist/emoji.json` directly and decode it into their own native model — no shared
platform modules, no platform-side hardcoded emoji lists. This pipeline is tracked directly in the
Taigi Keyboard repository; each app glyph-filters emoji its OS font cannot render at load (test the
whole grapheme cluster, not per-scalar).

The original standalone [taigi-emojis repository](https://github.com/taigikeyboard/taigi-emojis)
remains available for history and reference. It is not automatically synchronized with this in-repo
copy; new Taigi Keyboard emoji changes are authored here.
