---
paths:
  - "scripts/**"
  - "tests/**"
  - "Makefile"
---

# Generator Conventions

Read before editing `scripts/generate.py` or `tests/`.

## Pipeline (order matters)

1. `parse_emoji_test` — `emoji-test.txt` is authoritative for membership, order,
   groups/subgroups, names, version. Only `fully-qualified` entries. Skin-tone-bearing
   entries are folded into their base's `variations`, never surfaced as standalone.
2. `attach_variations` — fold tone forms via `_reduce_to_base` (plain modifier removal +
   `MIXED_TONE_PATTERNS` for two-person sequences that collapse to one neutral base).
3. `apply_cldr` — en + zh-Hant keywords (main + derived annotation files).
4. `apply_overrides` — Taigi overlay (patch/add/exclude).
5. `filter_and_validate` — version cap, drop `LEGACY_FAMILY_EMOJIS`, assert membership +
   no duplicate base.
6. `build_json` — emit the `output-contract.md` schema.

## Rules

- **Stdlib only.** No third-party deps in the generator (CLDR is plain XML via
  `xml.etree`). Keep it that way — a maintainer should run it with bare `python3`.
- **emoji-test.txt is the membership authority.** Do not reintroduce hand-coded base lists.
  When a grouping edge case breaks, add a `MIXED_TONE_PATTERNS` entry + a golden test, do
  not special-case in the main loop.
- **Validation is load-bearing.** `filter_and_validate` asserts every non-custom emoji
  exists upstream and no base is duplicated across categories. Keep these asserts.
- **Determinism.** Output must be byte-stable across runs (`test_generation_is_deterministic`
  + the drift guard pin it). No clock reads, no set iteration leaking into output order.
- **Tests are the spec.** Every grouping edge case (skin tone, mixed-tone, hair, direction,
  keycap, tag flag, FE0F, legacy-family drop) has a golden test. Add one when you touch the
  pipeline. Trace expected values from the real data, do not reverse-rationalize.
- Lint/format with `ruff` (`make lint`). `make build` regenerates; `make test` runs pytest.
