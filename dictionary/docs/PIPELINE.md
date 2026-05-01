# Pipeline

`./run.sh` drives a shared, config-controlled pipeline that turns each
source's raw input into a canonical `data/<key>.csv` under
`sources/<cat>/<key>/`. Stage lists are declared per source in
`config.yaml`; the same Python implementations run for every dictionary.

Stage implementations live in `common/stages/*.py`. The entry point is
`pipeline/run.py`; shared state per source is `PipelineContext`
(`pipeline/context.py`).

## Stages (12)

Stages operate on either a single `ctx._df` (DataFrame) or
`ctx._sheets` (dict of sheet-name → DataFrame; kautian-only). The
`merge` stage is the one that collapses multi-sheet state into a
single DataFrame.

| Stage       | Input     | Output    | What it does                                                         |
|-------------|-----------|-----------|----------------------------------------------------------------------|
| `extract`   | disk      | `_df` or `_sheets` | Load `input_path` (.ods / .json / .csv / .csv_noheader). Kautian + stti multi-sheet. |
| `select`    | `_sheets` | `_sheets` | Kautian only. Rename columns, melt dialect tables, pivot kautian's 18 sheets into canonical columns. |
| `expand`    | `_sheets` or `_df` | same     | Split rows whose fields carry `/`, `,`, `.` or Chinese-digit variants into multiple rows. |
| `merge`     | `_sheets` | `_df`     | Kautian: concatenate sheets in sorted order, drop `異用字`, dedupe on (hanzi, tl). |
| `cleanup`   | `_df`     | `_df`     | Drop proverbs, roman-char hanzi, long entries, hanlo mismatches. `minimal: true` skips most checks (khpoo). |
| `frequency` | `_df`     | `_df`     | Join `shared/data/char_freq_merged.txt` per-character frequencies; legacy sort `[freq desc, hanzi asc, tl asc]`. |
| `poj`       | `_df`     | `_df`     | Add `poj` column via taigi-converter TL→POJ. |
| `numtone`   | `_df`     | `_df`     | Add `tl_num` + `poj_num` (numeric-tone ASCII-only) via `romanization.to_numeric_tone`. |
| `notone`    | `_df`     | `_df`     | Add `tl_notone` + `poj_notone` by stripping tone digits from `tl_num` / `poj_num`. |
| `abbrev`    | `_df`     | `_df`     | Add `tl_abbrev` + `poj_abbrev` (first letter of each syllable). |
| `source`    | `_df`     | `_df`     | Add per-source boolean column (`kautian=True`, etc.). `no_hanzi: drop` filters NaN/empty hanzi for ChhoeTaigi. |
| `variants`  | `_df`     | `_df`     | Mark `is_variant=True` via substring+syllable match against `supplementary/variants/data/variants.csv`. Kautian uses a generate-variants pre-pass instead. |

## Per-source stage order

Declared in each `config.yaml[stages]`:

| Source  | Stages                                                                                          |
|---------|-------------------------------------------------------------------------------------------------|
| kautian | `extract`, `select`, `expand`, `merge`, `cleanup`, `frequency`, `poj`, `numtone`, `notone`, `abbrev`, `source`, `variants` |
| taigitv | `extract`, `cleanup`, `frequency`, `poj`, `numtone`, `notone`, `abbrev`, `source`, `variants`   |
| itaigi  | `extract`, `expand`, `cleanup`, `frequency`, `poj`, `numtone`, `notone`, `abbrev`, `source`, `variants` |
| sitbut  | same as itaigi                                                                                  |
| taihoa  | same as itaigi                                                                                  |
| taijit  | same as itaigi                                                                                  |
| kungge  | same as taigitv                                                                                 |
| stti    | same as taigitv                                                                                 |
| khpoo   | same as taigitv (`cleanup.minimal: true`)                                                       |

Kautian is the only source that exercises `select`/`expand`/`merge` in
multi-sheet mode; the ChhoeTaigi sources run `expand` in single-DF mode.
taigitv/kungge/stti/khpoo inputs are already pre-merged, so their
pipelines skip the sheet-based stages.

## Data dependency graph

```
shared/data/
├── char_freq_merged.txt ──┐
├── khiin_*.csv ───────────┼──► build/merge_csv.py ──┐
└── 語音差異.csv (ref only) │                         │
                           │                         │
supplementary/             │                         │
├── variants/ ─► variants stage (per source)         │
├── dev/ ──────────────────┼──► build/merge_csv.py ──┤
└── lkk/ ──────────────────┘                         │
                                                      ▼
sources/<cat>/<key>/                         output/dictionary.csv
  config.yaml ──────► pipeline/run.py ──► data/<key>.csv ──► build/* ──►
    input_path                                                          ├─► dictionary.db
                                                                        ├─► trie.db ─► dictionary.fst
                                                                        ├─► dictionary.bin
                                                                        └─► association.bin
```

`./build.sh` executes the stages right of the pipe in fixed order
(see `build.sh`). Each build step's naming follows the script filename:
`merge_csv` → `create_app_db` → `generate_association` →
`create_trie_db` → `create_fst` → `create_dictionary_bin` →
`create_association_bin` → `audit` → `deploy`.

## Parity gate

`tools/compare_baseline.py verify` checks both byte-level and SQL-semantic
equality against `baseline.json` (captured at Step 0). Any non-trivial
pipeline or build change must keep this gate green. `build_ts` (8-byte
field) in `dictionary.bin` / `association.bin` is masked during hashing.
