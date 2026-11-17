# AGENTS.md

Guidelines for the `emoji/` subtree (Python emoji-data pipeline for TaigiKeyboard).

Working instructions for coding agents — and for humans — in this subtree. Rules live in `docs/`; repository-wide rules in the parent `AGENTS.md`.

## Project Overview

**emoji/** — the single emoji source of truth for TaigiKeyboard (iOS + Android).
A Python generator
merges pinned Unicode + CLDR data with a hand overlay of 台語/華語 search keywords and emits one
`dist/emoji.json` both platforms consume directly from this repository.

Layout, curation workflow and version bumps: [`README.md`](README.md).

## Core Principles

1. **One source, both platforms** — emoji authored once here; no platform-side hardcoded lists.
2. **emoji-test.txt is membership authority** — never hand-code base lists. Grouping edge cases
   get a `MIXED_TONE_PATTERNS` entry + a golden test.
3. **Taigi keywords = authoritative-source-only** — never invent TL/POJ. Verify against the parent
   repository's `docs/phonetics/taigi-phonetics-reference.md` + `taigi-converter`; use 漢字 when unsure.
4. **Deterministic build** — `dist/emoji.json` byte-stable; drift-guard test fails on stale output.
   Always `make -C emoji build` after editing data/generator.
5. **Release scope / version-pin bumps = user-gated** — never raise `MAX_EMOJI_VERSION`, tag, or
   declare "ready to release" without the user's explicit word.

## Read before…

| Before… | Read |
|---|---|
| editing `src/overrides.tsv` or `data/` | `docs/emoji-data-authoring.md` |
| editing `scripts/generate.py` or tests | `docs/generator.md` |
| changing the json schema / platform consumption | `docs/output-contract.md` |


## Build & Test

`make -C emoji build` (regenerate `dist/emoji.json`) · `test` · `lint` (lint + format check) · `fetch` (re-pull upstream, version bump only).

## App integration

How each app consumes `dist/emoji.json`: [`docs/ui/emoji.md`](../docs/ui/emoji.md) § Data; schema: `docs/output-contract.md`.

## Communication

Docs and code comments follow the root `AGENTS.md` § Conventions.
