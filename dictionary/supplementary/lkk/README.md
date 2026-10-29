# lkk — Project-maintained spelling supplement

`data/lkk.csv` is the manually curated source of truth for this supplement.
Its spelling/reading pairs are informed by the recommendations of
李江却台語文教基金會. It extends the original mixed-script supplement and is
not a complete or official release of the foundation's word list.

Reference: <https://tsbp.tgb.org.tw/p/bong_8.html>. The page embeds the LKK
spreadsheet titled as updated on 2024-05-10; the reference was checked on
2026-10-17. No copy of the source TSV or automatic importer is included.

## Maintaining the list

Edit `data/lkk.csv` directly, with one explicit `(hanzi, canonical-TL)` pair
per row. Review each spelling and reading against its reference before
adding it; expand optional forms and alternate readings into separate pairs
rather than storing shorthand or usage notes in the CSV.

- Include only spellings that contain both Hanji and Latin letters.
  Fully romanized and all-Hanji spellings do not belong in this supplement.
- Deduplicate by the complete spelling/reading pair. Different readings of
  the same Hanji remain distinct entries.
- Exclude exact pairs already present among the MoE dictionary's non-variant
  entries. Sharing only the Hanji is not grounds for exclusion; generated
  variant spellings do not count as MoE recommendations for this rule.
- Exclude forms whose spelling or reading starts with `--`. The engine
  inserts typed leading hyphens literally, so such candidates would duplicate
  the marker on selection. Keep internal neutral-tone markers in words such
  as the original `邊--a` and `tso̍h--日`.

The curated list currently contains **82 unique pairs**. Tests check
mixed-script spellings, canonical readings, pair uniqueness, and both
exclusions. Run them from `dictionary/`:

```sh
python3 -m pytest tests/test_lkk_supplement.py -q
```

The source pipeline leaves this curated CSV unchanged.
`build/merge_csv.py::_load_lkk_entries` reads it and generates the derived
romanization columns. It adds source bit 11 to an existing matching pair in
the other dictionaries or inserts a new pair, preserving other source tags.
After changing the curated list, rebuild the dictionary artifacts with
`make dict` from the repository root.

## Attribution and reuse status

See [the source inventory](../../docs/SOURCES.md) and
[data licences](../../LICENSE) for LKK attribution and unresolved reuse terms.
The source page, embedded spreadsheet, foundation homepage, and publication
masthead were checked on 2026-10-17; no explicit open-data licence or grant
to redistribute the table was identified. Project curation does not claim
independent authorship or establish a redistribution licence for
source-derived data.
