---
name: fetch-emoji
description: Fetch and regenerate emoji data for a target Unicode Emoji version. Use when bumping the emoji version, refreshing CLDR keyword data, or when the user wants newer emoji (e.g. "update to Emoji 17", "pull the latest emoji"). Pulls canonical emoji-test.txt + matching CLDR annotations for the chosen version, regenerates dist/emoji.json, runs tests.
---

# fetch-emoji

Refresh the emoji dataset to a chosen Unicode Emoji version. The canonical data source is
the machine-readable Unicode + CLDR files — `emojipedia.org` and
`unicode.org/emoji/charts/full-emoji-list.html` are **human references only** (browse them
to decide which version / verify what a version added), never the data pipe.

## Steps

1. **Decide the target version.** Ask the user, or browse the references:
   - https://unicode.org/Public/emoji/ — the available `emoji-test.txt` versions (the source).
   - https://emojipedia.org/ or https://unicode.org/emoji/charts/full-emoji-list.html — human
     reference to see what each version contains.

2. **Find the matching CLDR release tag.** CLDR ships keyword annotations per release; the
   release that matches an Emoji version:

   | Emoji version | CLDR tag |
   |---|---|
   | 15.1 | `release-44` |
   | 16.0 | `release-46` |
   | 17.0 | `release-48` (verify on https://github.com/unicode-org/cldr/tags) |

   If unsure, check `https://github.com/unicode-org/cldr/tags` for the newest release whose
   notes cite the target Emoji version.

3. **Update the three version pins together:**
   - `Makefile`: `UNICODE_EMOJI_VERSION` + `CLDR_TAG`.
   - `scripts/generate.py`: `MAX_EMOJI_VERSION` (e.g. `"E17.0"`) + `CLDR_VERSION` (e.g. `"48"`).
   - `data/SOURCES.md`: the version table + pin rationale.

4. **Fetch + regenerate + verify:**
   ```bash
   make fetch    # re-pulls data/ from the new pins
   make build    # regenerate dist/emoji.json
   make test     # golden specs + drift guard
   ```

5. **Review `git diff`** of `data/` + `dist/emoji.json`. New emoji appear as added lines.
   Sanity-check a few against the reference pages. The per-emoji `version` field will carry
   the new version (e.g. `E17.0`); old devices filter those out at load via the platform
   glyph check (see `.claude/rules/output-contract.md`).

6. **If a golden test breaks**, the grouping of a new emoji likely needs a
   `MIXED_TONE_PATTERNS` entry — fix in `scripts/generate.py`, add a golden test. Do NOT
   weaken `filter_and_validate`.

## Notes

- Raising the version pin is a deliberate change — release scope/timing is user-gated. Do
  not bump without the user's word.
- Keep the generator stdlib-only; CLDR is plain XML parsed via `xml.etree`.
