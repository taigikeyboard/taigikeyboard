# CLAUDE.md

Guidelines for Claude Code in this repo (Python emoji-data pipeline for the Taigi keyboard).

Cross-project process rules (workflow, planning, review, naming, docs, Python) auto-load from
`~/.claude/rules/` (user-global, from the `configurations` dotfiles repo). Project rules live in
`.claude/rules/`; personal defaults in `~/.claude/CLAUDE.md`.

## Project Overview

**taigi-emojis** — the single emoji source of truth for the Taigi keyboard (iOS + Android).
Replaces iOS's unmaintained `ISEmojiView` and Android's inline `root.txt`. A Python generator
merges pinned Unicode + CLDR data with a hand overlay of 台語/華語 search keywords and emits one
`dist/emoji.json` both platforms consume directly from this repository.

## Structure

- `src/overrides.tsv` — the only hand-edited file (add/curate emoji + Taigi keywords)
- `scripts/generate.py` — generator (stdlib-only); `tests/` — golden specs + drift guard
- `data/` — pinned upstream snapshots (+ `SOURCES.md`)
- `dist/emoji.json` — the generated artifact both apps read directly (committed)

## Core Principles

1. **One source, both platforms** — emoji authored once here; no platform-side hardcoded lists.
2. **emoji-test.txt is membership authority** — never hand-code base lists. Grouping edge cases
   get a `MIXED_TONE_PATTERNS` entry + a golden test.
3. **Taigi keywords = authoritative-source-only** — never invent TL/POJ. Verify against the parent
   repository's `knowledge/taigi-phonetics-reference.md` + `taigi-converter`; use 漢字 when unsure.
4. **Deterministic build** — `dist/emoji.json` byte-stable; drift-guard test fails on stale output.
   Always `make build` after editing data/generator.
5. **Release scope / version-pin bumps = user-gated** — never raise `MAX_EMOJI_VERSION`, tag, or
   declare "ready to release" without the user's explicit word.

## Mandatory Rules

| Before… | Read |
|---|---|
| editing `src/overrides.tsv` or `data/` | `.claude/rules/emoji-data-authoring.md` |
| editing `scripts/generate.py` or tests | `.claude/rules/generator.md` |
| changing the json schema / platform consumption | `.claude/rules/output-contract.md` (always-on) |

## Build & Test

| Task | Command |
|---|---|
| Regenerate `dist/emoji.json` | `make build` |
| Run tests | `make test` |
| Lint + format check | `make lint` |
| Re-pull upstream (version bump only) | `make fetch` |

## App integration (in the parent repository)

This subtree ships **data only** — `dist/emoji.json`. Each app reads it directly and decodes into its
own native model; there are no shared Swift/Kotlin modules. Schema: `.claude/rules/output-contract.md`.

- **iOS**: bundles `dist/emoji.json` as a keyboard-extension resource; a slim `Codable` decodes it
  and feeds the vendored ISEmojiView via `customEmojis`.
- **Android**: `assets.srcDir` mounts `dist/`; Moshi decodes `emoji.json` into the existing
  `EmojiKeyData`/`EmojiSet`/`EmojiCategory` (replaced `root.txt`).
- **Both**: an app-side glyph filter drops emoji the OS font can't render — test the whole grapheme
  cluster (iOS CoreText, Android `PaintCompat.hasGlyph`), not per-scalar.

## Communication

- Reply in 台灣華語; docs + code comments in English (CJK OK for verbatim quotes + 台語/漢字/TL/POJ terms).
- Concise, bullet-point, key points only.
