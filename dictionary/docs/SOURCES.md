# Sources

Per-source provenance for the 9 configured dictionaries + 3 supplementary
sources + shared Khiin reference data. Populate the TBD cells with
current-at-release values when cutting a dictionary refresh.

## Main dictionaries

### kautian — 教育部臺灣台語常用詞辭典
- **Publisher**: 中華民國教育部 (MoE Taiwan)
- **Origin URL**: <https://sutian.moe.edu.tw/>
- **Format**: ODS (OpenDocument Spreadsheet, 18 sheets)
- **Captured**: `data/raw/kautian.ods` (file committed 2026-04-23; actual
  MoE release date TBD)
- **Licence**: 創用 CC 姓名標示-非商業性 4.0 國際 / CC BY-NC 4.0 (per sutian.moe.edu.tw)
- **Notes**: Multi-sheet; pipeline `select`/`expand`/`merge` stages
  handle the pivoting. Largest single source (~49k rows).

### taigitv — 公視台語新詞辭庫
- **Publisher**: 公共電視文化事業基金會 / 公視台語台 (PTS Taigi)
- **Origin URL**: TBD
- **Format**: JSON (scraped; filename encodes capture timestamp)
- **Captured**: `data/raw/scrape-20260319T144906Z.json` → 2026-03-19 14:49 UTC
- **Licence**: TBD (public-service broadcast content; consult 公視 usage terms)
- **Notes**: Supplement dictionary for neologisms; ~2.1k rows.

### itaigi — iTaigi 華台對照典
- **Publisher**: ChhoeTaigi 找台語 project
- **Origin URL**: <https://github.com/ChhoeTaigi/ChhoeTaigiDatabase>
- **Format**: CSV (ChhoeTaigi canonical export)
- **Captured**: `data/raw/ChhoeTaigi_iTaigiHoataiTuichiautian.csv` — capture date TBD
- **Licence**: CC BY-SA 4.0 (per ChhoeTaigi)
- **Notes**: Original source iTaigi (<https://itaigi.tw>); ~15k rows.

### sitbut — 台灣植物名彙
- **Publisher**: ChhoeTaigi
- **Origin URL**: <https://github.com/ChhoeTaigi/ChhoeTaigiDatabase>
- **Format**: CSV
- **Captured**: `data/raw/ChhoeTaigi_TaioanSitbutMialui.csv` — capture date TBD
- **Licence**: CC BY-SA 4.0 (per ChhoeTaigi)
- **Notes**: Plant-name vocabulary; ~1.3k rows.

### taihoa — 台華線頂對照典
- **Publisher**: ChhoeTaigi
- **Origin URL**: <https://github.com/ChhoeTaigi/ChhoeTaigiDatabase>
- **Format**: CSV
- **Captured**: `data/raw/ChhoeTaigi_TaihoaSoanntengTuichiautian.csv` — capture date TBD
- **Licence**: CC BY-SA 4.0 (per ChhoeTaigi)
- **Notes**: Broadest community Taigi↔Mandarin crossreference; ~57k rows.

### taijit — 台日大辭典
- **Publisher**: ChhoeTaigi (digitisation of 小川尚義《台日大辭典》 1931–32)
- **Origin URL**: <https://github.com/ChhoeTaigi/ChhoeTaigiDatabase>
- **Format**: CSV
- **Captured**: `data/raw/ChhoeTaigi_TaijitToaSutian.csv` — capture date TBD
- **Licence**: public-domain primary source, digitisation terms CC BY-SA 4.0 (per ChhoeTaigi)
- **Notes**: Largest community source (~64k rows); historic Taigi-Japanese dictionary.

### kungge — 台語工藝詞庫
- **Publisher**: 中華民國教育部 (MoE Taiwan)
- **Origin URL**: TBD
- **Format**: JSON (scraped; filename encodes capture date)
- **Captured**: `data/raw/scrape-20251210.json` → 2025-12-10
- **Licence**: TBD (MoE-derived)
- **Notes**: Specialised craft/industry terminology; ~1k rows.

### stti — 學科術語辭典
- **Publisher**: 中華民國教育部 / 國家教育研究院 (National Academy for Educational Research)
- **Origin URL**: TBD
- **Format**: CSV
- **Captured**: `data/raw/stti.csv` — capture date TBD
- **Licence**: TBD (government-released; likely 開放政府資料授權條款 1.0 / Open Government Data Licence)
- **Notes**: Academic-discipline terminology; ~4.9k rows. Extract stage
  runs a bespoke `expand_variant_readings` (see `common/stages/extract.py`).

### khpoo — 齒盤補充辭典
- **Publisher**: TBD (supplementary set maintained for keyboard coverage)
- **Origin URL**: TBD
- **Format**: CSV (no header)
- **Captured**: `data/raw/khpoo.csv` — capture date TBD
- **Licence**: TBD
- **Notes**: Pre-processed data — pipeline uses `cleanup.minimal: true`
  (normalize_roman only). ~4.2k rows.

## Supplementary inputs

### variants — 異用字 (supplementary/variants/)
- **Publisher**: 教育部臺灣台語常用詞辭典 (derived from kautian's 異用字 sheet)
- **Origin URL**: embedded in kautian release
- **Format**: CSV (`data/variants.csv`)
- **Captured**: derived at pipeline `select` stage of kautian; the snapshot at
  `supplementary/variants/data/` was frozen from an earlier MoE release (TBD).
- **Licence**: inherits from kautian (CC BY-NC 4.0)
- **Notes**: ~3k variant rows. Applied by the `variants` stage of each
  source to set `is_variant` bit on matching (hanzi, tl) rows.

### dev — 開發者補充辭典 (supplementary/dev/)
- **Publisher**: Taigi Keyboard maintainers
- **Origin URL**: internal (repo-native)
- **Format**: CSV (`data/dev.csv`)
- **Captured**: repo-native; maintained by hand
- **Licence**: same as repo
- **Notes**: Targeted coverage gaps uncovered during dogfooding.

### lkk — LKK 漢羅合用建議用字 (supplementary/lkk/)
- **Publisher**: TBD (community-authored list of recommended 漢羅混寫 spellings)
- **Origin URL**: TBD
- **Format**: CSV (`data/lkk.csv`)
- **Captured**: TBD
- **Licence**: TBD
- **Notes**: Style/convention guide merged into the main dictionary as
  supplementary hints (bit 11).

## Shared reference data (shared/data/)

- `char_freq_merged.txt` — character-frequency corpus driving the
  `frequency` stage. Provenance: merged user corpus (TBD).
- `khiin_frequency.csv` — frequency values supplementing Khiin IME coverage.
  Provenance: Khiin project (<https://github.com/khiin-pjh/khiin>),
  date TBD.
- `khiin_conversions.csv` — tone-conversion pairs from the Khiin project
  used in `build/merge_csv.py` to backfill rows absent from the nine main
  sources. Date TBD.
- `語音差異.csv` — regional pronunciation variance reference (not currently
  consumed by the build; retained for future stages).

## Re-capture notes

When refreshing a source:

1. Replace `data/raw/<file>` with the new capture. Include the date in
   the filename if the publisher provides one (`scrape-YYYYMMDD*.json`).
2. Update `Captured:` above with the new date.
3. Re-run `./run.sh` (regenerates every source's `data/<key>.csv`; the
   CLI no longer supports per-source filtering) then `./build.sh` and
   `tools/compare_baseline.py verify`. Content changes will typically
   fail the gate — decide whether to re-capture the baseline (rare,
   explicit user decision) or roll the refresh into a larger update.
