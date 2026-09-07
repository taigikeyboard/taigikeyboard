# dictionary/

Taigi Keyboard's dictionary data pipeline. Builds the runtime artefacts
(`dictionary.bin` / `dictionary.fst` / `association.bin`) that both iOS
and Android consume. Each binary is generated directly from the canonical
`dictionary.csv` — no SQLite intermediates (removed in v3.5.6 part 2).

## Layout

```
dictionary/
├── run.sh                     # Pipeline entry point — regenerate per-source CSVs
├── build.sh                   # Build entry point — produces output/ + deploys to the repo-root dictionaries/
├── baseline.json              # Parity gate reference (compare_baseline.py verify)
├── requirements.txt           # Python deps
│
├── pipeline/                  # Shared pipeline driver (discover_configs + run_dict)
├── common/                    # Stage implementations + shared helpers (source_bits, romanization, …)
├── build/                     # Build stage scripts (merge_csv, create_fst, create_*_bin, …)
├── tools/                     # Dev utilities (compare_baseline, verify_csv, query_fst)
│
├── sources/                   # Per-source dictionary inputs (see sources/README.md)
│   ├── official/{kautian,taigitv,kungge,stti}/
│   ├── community/{itaigi,sitbut,taihoa,taijit}/
│   └── supplementary/khpoo/
│
├── supplementary/             # Auxiliary data merged in by build/merge_csv.py
│   ├── variants/              # 異用字 (is_variant bitmask)
│   ├── dev/                   # Developer additions (bit 10)
│   └── lkk/                   # LKK 漢羅合用建議用字 (bit 11)
│
├── shared/data/               # Shared reference files consumed by build + frequency stage
│   ├── char_freq_merged.txt   # Character-frequency table
│   ├── khiin_frequency.csv    # Khiin frequency supplement
│   ├── khiin_conversions.csv  # Khiin conversion table
│   └── 語音差異.csv            # Regional pronunciation reference
│
├── output/                    # Generated build artefacts
└── docs/                      # Pipeline + sources docs
```

## Quick start

```bash
# 1) Regenerate every source's canonical CSV (data/<key>.csv)
./run.sh

# 2) Build runtime artefacts into output/ + deploy to the repo-root dictionaries/
./build.sh

# 3) Sanity-check against the committed baseline
python3 tools/compare_baseline.py verify
```

`./run.sh` always runs the full 9-source pipeline. Single-source flags
(`--dict`, `--category`, `--list`) were removed in the consolidation
pass: the whole pipeline completes in seconds, none of the build steps
accept partial inputs, and the simpler CLI keeps `pipeline/run.py`
trivial. Use `python3 -m pipeline.run` directly from `dictionary/` if
you need to invoke it from Python.

## Sources (English key ↔ Chinese ↔ category)

| key        | chinese_name                   | category       | rows  | bit |
|------------|--------------------------------|----------------|-------|-----|
| kautian    | 教育部臺灣台語常用詞辭典       | official       | ~49k  | 0   |
| taigitv    | 公視台語新詞辭庫               | official       | ~2.1k | 1   |
| itaigi     | iTaigi 華台對照典 (ChhoeTaigi) | community      | ~15k  | 2   |
| sitbut     | 台灣植物名彙 (ChhoeTaigi)      | community      | ~1.3k | 3   |
| taihoa     | 台華線頂對照典 (ChhoeTaigi)    | community      | ~57k  | 4   |
| taijit     | 台日大辭典 (ChhoeTaigi)        | community      | ~64k  | 5   |
| kungge     | 台語工藝詞庫                   | official       | ~1k   | 6   |
| stti       | 學科術語辭典                   | official       | ~4.9k | 7   |
| khpoo      | 齒盤補充辭典                   | supplementary  | ~4.2k | 8   |
| khiin      | Khiin 頻率/轉換                | shared ref     | —     | 9   |
| dev        | 開發者補充辭典                 | supplementary  | —     | 10  |
| lkk        | LKK 漢羅合用建議用字           | supplementary  | —     | 11  |
| (variants) | 異用字                         | supplementary  | ~3k   | —   |

`(variants)` is applied as the `is_variant` bit (12) on matching rows rather
than contributing its own source bit. See `common/source_bits.py` for the
authoritative bit assignment; `docs/SOURCES.md` for per-source licences +
original URLs.

## File-purpose cheatsheet

| Path                             | Role                                                         |
|----------------------------------|--------------------------------------------------------------|
| `run.sh`                         | Thin wrapper → `python3 -m pipeline.run`                     |
| `pipeline/run.py`                | Discover `sources/*/*/config.yaml`, run staged pipeline      |
| `pipeline/context.py`            | `PipelineContext` threaded through every stage               |
| `common/stages/*.py`             | Individual stage implementations (12 stages total)           |
| `common/source_bits.py`          | Authoritative SOURCE_BITS / SOURCE_TIERS / column orders     |
| `build/merge_csv.py`             | Merge 9 per-source CSVs + khiin/dev/lkk supplements          |
| `build/dictionary_records.py`    | Shared loader — CSV → filtered records with rowids 1..N      |
| `build/associations.py`          | Shared bigram + char-to-phrase generator from dictionary.csv |
| `build/create_fst.py`            | fst prefix index from CSV (shells to engine/build-helpers/fst-builder) |
| `build/create_{dictionary,association}_bin.py` | Binary mmap formats consumed by mobile apps |
| `build/verify_poj_integrity.py`  | Fatal POJ-integrity gate — halts build if `poj`/derived ≠ `convert_tl_to_poj(tl)` (+ KeSi report-only) |
| `build/version_snapshot.py`      | Build-drop summary + `(hanzi, tl)` diff vs the previous release tag's `dictionary.csv` (read via `git show`; no snapshot file stored) |
| `tools/compare_baseline.py`      | Parity gate — SHA256 + CSV-derived semantic diff vs baseline.json |
| `tools/verify_csv.py`            | CSV character-validity + duplicate sanity checker            |
| `tools/query_fst.py`             | Query the compiled fst prefix index (dev debug)              |

## Build-drop summary & version diff

`build.sh` step 7 (`version_snapshot`) prints a build-drop summary (raw →
dedup → supplements → final) plus a `(hanzi, tl)`-entry diff (added / removed)
against the **previous release tag's** `dictionary/output/dictionary.csv`. That
CSV is already git-tracked and committed at every release tag, so the previous
release is read straight from git (`git show <tag>:…`) — no separate snapshot
file is stored. Full added+removed lists land in `output/version_diff.txt`
(ephemeral, gitignored).

Diff-base resolution is semver-aware (**3-segment `vX.Y.Z` tags only**; a
4-segment tag like `v3.4.8.1` is ignored):

```bash
RELEASE_VERSION=v3.6.0 ./build.sh   # diff base = newest tag strictly < v3.6.0
./build.sh                          # diff base = newest release tag overall
```

Passing the version excludes the target tag itself, so re-running after it is
tagged still diffs against the real predecessor. The report never halts the
build (any git/parse failure degrades to "nothing to diff") — release scope is
the maintainer's call.

To inspect a previous release's full dictionary directly:
`git show v3.5.9:dictionary/output/dictionary.csv`.

## Notes

- **`aiongtaigi-dictionary.csv` (removed Step 8)** — the legacy Aiong-Taigi
  source dump was replaced by the ChhoeTaigi community sources. Its former
  consumer `scripts/generate-syllable-test-data.mjs` (also removed) produced
  `android/app/src/test/resources/syllable-test-data.csv`, removed 2026-09-05
  once no test read it any more.
- **Binary format reference**: `../docs/engine/binary-format.md` documents
  `dictionary.bin` / `association.bin` on-disk layout.
- **Cross-platform invariant**: `SOURCE_BITS` + `SOURCE_TIERS` + tier
  denominator in `common/source_bits.py` must mirror the iOS/Android
  `DictionaryBinaryReader.{swift,kt}` and `CandidateProcessor.{swift,kt}`.
  `.claude/rules/cross-platform-alignment.md` §3a governs drift.
