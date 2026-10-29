# Sources

Per-source provenance for the 9 configured dictionaries + 3 supplementary
sources + shared Khiin reference data. Populate the TBD cells with
current-at-release values when cutting a dictionary refresh.

## Main dictionaries

### kautian — 教育部臺灣台語常用詞辭典
- **Publisher**: 中華民國教育部 (MoE Taiwan)
- **Origin URL**: <https://sutian.moe.edu.tw/>
- **Format**: ODS (OpenDocument Spreadsheet, 19 sheets in the committed capture)
- **Captured**: `data/raw/kautian.ods` on 2026-10-29 from
  <https://sutian.moe.edu.tw/media/senn/ods/kautian.ods>.
  Upstream Last-Modified: 2026-10-28 18:04:13 UTC; SHA-256:
  `a050babbac724fbf21d675f9d20222281b87320d6648701d402b109fae4899d3`.
- **Licence**: **CC BY-ND 3.0 TW** (創用CC 姓名標示-禁止改作 3.0 臺灣) — verified 2026-09-05 against the publisher's copyright notice, <https://sutian.moe.edu.tw/zh-hant/piantsip/pankhuan-singbing/>. Covers text and audio content (文字、音檔內容). **Commercial use permitted; no NonCommercial term.** This file previously recorded CC BY-NC 4.0, which was wrong. Only Hanji + romanization are taken — see `dictionary/LICENSE` for why CC BY-ND permits that.
- **Notes**: Multi-sheet; pipeline `select`/`expand`/`merge` stages
  handle the pivoting. Largest single source (~49k rows).
- **Update watch**: `.github/workflows/kautian-watch.yml` downloads
  <https://sutian.moe.edu.tw/media/senn/ods/kautian.ods> daily at 00:00 UTC
  (08:00 Taiwan time), with HTTPS certificate verification. The downstream
  `taigikeyboard/kautian-mirror` checks this repository's main at 09:00 Taiwan
  time, leaving one hour for the upstream rebuild and verification. Data PRs
  require manual merge; the site picks up merged data on a later scheduled run.
  GitHub scheduled runs can be delayed; the two schedules are not a strict
  completion dependency. The standard-library tool `tools/kautian_watch.py`
  compares all sheets' data rows
  with the committed capture, ignoring packaging, styles and row ordering.
  Duplicate rows retain their counts. The issue reports before/after totals,
  additions, removals and modifications per sheet, with named-field Git-style
  diff blocks and a linked full Markdown/JSON diff (Actions artifact, retained
  90 days; sign-in required).
  Only explicit source IDs unique in both complete sheets pair modifications
  for 詞目, 義項 and 例句; other changes remain additions/removals. These are raw
  source-record counts, not counts of rebuilt keyboard dictionary entries.
  Sheet additions/removals and header changes are reported explicitly.
  Existing bot-created issues, including #534, receive a generated section;
  human text and comments remain intact. Unchanged sheets and comparison
  metadata are collapsed. The bounded summary keeps complete record diffs;
  the full report includes every change. Identical comparisons with the same
  renderer version do not rewrite the issue. No-change runs retain the latest
  diff. Data PRs include a closing reference to the matching issue, so manually
  merging the PR into main automatically closes the issue and preserves its report.
  Data changes trigger `make dict`, including binary round-trip, POJ integrity
  and known-key verification. The workflow commits only the Kautian capture,
  provenance, source CSV, derived variants, merged dictionary, walker statistics,
  word keys and four shipped artifacts, then opens a PR. Callable Python and
  Engine workflows validate its exact commit; the maintainer merges the PR manually.
  These explicit gates avoid requiring manual approval of workflows triggered
  by a `GITHUB_TOKEN`-created PR. Actions must be allowed to create PRs in the
  repository's workflow permissions; no additional token is required.
  Rebuild failures stop PR creation. Verification failures remain visible for
  review before manual merge. An existing open automation PR skips duplicate
  refresh creation while awaiting the maintainer; daily issue reports still update.
  The pending PR keeps its original capture; upstream changes arriving while it
  is open are detected again after that PR merges.
  Dictionary merging preserves source and row order within official-source
  priority, so unrelated additions cannot change the selected TL/POJ spelling.
  Corpus membership changes require manual regeneration of both corpus TSVs
  from all source corpora with `build.corpus_bigrams`; partial-source generation
  would lose counts. Automation preserves that failure instead of dropping data.
  A manual **Kautian watch** run regenerates the full report if an artifact
  expires; its artifacts remain available even when the issue is unchanged.
  To inspect locally without publishing:
  `python3 tools/kautian_watch.py compare dictionary/sources/official/kautian/data/raw/kautian.ods /path/to/upstream.ods --baseline-commit "$(git rev-parse HEAD)" --output /tmp/kautian-diff`.

### taigitv — 公視台語新詞辭庫
- **Publisher**: 公共電視文化事業基金會 / 公視台語台 (PTS Taigi)
- **Origin URL**: TBD
- **Format**: JSON (scraped; filename encodes capture timestamp)
- **Captured**: `data/raw/scrape-20260319T144906Z.json` → 2026-03-19 14:49 UTC
- **Licence**: **CC BY 4.0** — verified 2026-09-05 against the publisher (公視台語台 台語新詞辭庫, <https://www.taigitv.org.tw/taigi-words>).
- **Notes**: Supplement dictionary for neologisms; ~2.1k rows.

### itaigi — iTaigi 華台對照典
- **Publisher**: ChhoeTaigi 找台語 project
- **Origin URL**: <https://github.com/ChhoeTaigi/ChhoeTaigiDatabase>
- **Format**: CSV (ChhoeTaigi canonical export)
- **Captured**: `data/raw/ChhoeTaigi_iTaigiHoataiTuichiautian.csv` — capture date TBD
- **Licence**: **CC0** (per the ChhoeTaigi database README; verified 2026-09-05)
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
- **Licence**: primary source public domain; **digitisation CC BY-NC-SA 3.0 TW** (per the ChhoeTaigi database README; verified 2026-09-05) — **non-commercial**
- **Notes**: Largest community source (~64k rows); historic Taigi-Japanese dictionary.

### kungge — 臺灣台語工藝詞庫
- **Publisher**: 國立臺灣工藝研究發展中心 (NTCRI)
- **Origin URL**: <https://kanggesu.ntcri.gov.tw/NTCRI_TaigiWebSite>
- **Format**: JSON (scraped; filename encodes capture date)
- **Captured**: `data/raw/scrape-20251210.json` → 2025-12-10
- **Licence**: **no licence statement covers the vocabulary.** The site's only licence page is the image licence, 圖像授權 (<https://kanggesu.ntcri.gov.tw/NTCRI_TaigiWebSite/ImageLicense>, read 2026-09-07): 姓名標示-非商業性-禁止改作 (CC BY-NC-ND, no version stated), scoped to the illustrations. This project takes only Hanji + TL romanization and drops `memo` / `audioPath` / images, so the position rests on Copyright Act (著作權法) §10-1 (facts are not expression), not on a grant — reasoning in `dictionary/LICENSE`. The earlier "CC BY-NC 4.0" record came from the in-app credits screen and was wrong.
- **Notes**: Specialised craft/industry terminology; ~1.2k raw entries → 994 rows. 11 craft categories.

### stti — 學科術語辭典
- **Publisher**: 中華民國教育部 / 國家教育研究院 (National Academy for Educational Research)
- **Origin URL**: TBD
- **Format**: CSV
- **Captured**: `data/raw/stti.csv` — capture date TBD
- **Licence**: **開放政府資料授權條款 1.0** (Open Government Data Licence, Taiwan) per the in-app credits screen — permits commercial use and derivatives. Publisher statement not re-confirmed.
- **Notes**: Academic-discipline terminology; ~4.9k rows. Extract stage
  runs a bespoke `expand_variant_readings` (see `common/stages/extract.py`).

### khpoo — 齒盤補充辭典
- **Publisher**: TBD (supplementary set maintained for keyboard coverage)
- **Origin URL**: TBD
- **Format**: CSV (no header)
- **Captured**: `data/raw/khpoo.csv` — capture date TBD
- **Licence**: **Unverified** — origin and rights holder not established.
- **Notes**: Pre-processed data — pipeline uses `cleanup.minimal: true`
  (normalize_roman only). ~4.2k rows.

## Supplementary inputs

### variants — 異用字 (supplementary/variants/)
- **Publisher**: 教育部臺灣台語常用詞辭典 (derived from kautian's 異用字 sheet)
- **Origin URL**: embedded in kautian release
- **Format**: CSV (`data/variants.csv`)
- **Captured**: derived at pipeline `select` stage of kautian; the snapshot at
  `supplementary/variants/data/` was frozen from an earlier MoE release (TBD).
- **Licence**: inherits from kautian (CC BY-ND 3.0 TW)
- **Notes**: ~3k variant rows. Applied by the `variants` stage of each
  source to set `is_variant` bit on matching (hanzi, tl) rows.

### dev — 開發者補充辭典 (supplementary/dev/)
- **Publisher**: TaigiKeyboard maintainers
- **Origin URL**: internal (repo-native)
- **Format**: CSV (`data/dev.csv`)
- **Captured**: repo-native; maintained by hand
- **Licence**: same as repo
- **Notes**: Targeted coverage gaps uncovered during dogfooding.

### lkk — LKK-derived spelling supplement (supplementary/lkk/)
- **Maintainer**: This project maintains the selected spelling/reading pairs.
- **Reference publisher**: 李江却台語文教基金會
- **Origin URL**: <https://tsbp.tgb.org.tw/p/bong_8.html> (publisher's word-list page)
- **Published spreadsheet**: [LKK word list, updated 2024-05-10](https://docs.google.com/spreadsheets/d/e/2PACX-1vR6sABIf13wvn95hKApMWmEYYD-vDL62mVAYBE1jycBRTkiJQush3-HCkkaPMSsv2cOcPZ0blNODFpx/pubhtml?widget=true&headers=false), embedded on the origin page; source verified 2026-10-17.
- **Format**: Manually curated CSV (`data/lkk.csv`), the source of truth for
  this supplement; see `supplementary/lkk/README.md`. No source TSV snapshot
  or automatic importer is included.
- **Reference checked**: 2026-10-17, spreadsheet titled as updated on 2024-05-10.
  The original 58-pair subset was added on 2026-03-21 in commit
  `31d41453d9cb` (old PR #103), with no recorded selection rule.
- **Licence**: **Unverified** — no open-data licence or explicit redistribution
  grant was identified on the source page, embedded spreadsheet, foundation
  homepage, or publication masthead when checked on 2026-10-17. Citation and
  curation are not evidence of a redistribution licence.
- **Notes**: A project-maintained supplement informed by LKK recommendations,
  not a complete or official LKK dataset. Each row stores an explicit
  `(hanzi, canonical-TL)` pair whose spelling contains both Hanji and Latin
  letters. Fully romanized and all-Hanji spellings are excluded. Current total:
  82 unique pairs. Tests enforce mixed-script spellings, pair uniqueness and
  exact MoE-pair and leading-`--` exclusions. The source pipeline does not
  regenerate this CSV. The aggregate build adds source bit 11 to matching
  entries in other dictionaries or inserts new pairs.

## Shared reference data (shared/data/)

- `char_freq_merged.txt` — character-frequency corpus driving the
  `frequency` stage. Provenance: merged user corpus (TBD).
- `khiin_frequency.csv` — frequency values supplementing Khiin IME coverage.
  Provenance: Khiin project (<https://github.com/khiin-pjh/khiin>), date TBD.
- `romanized_readings.tsv` — reading (`tl_num`) → hanji for the words that
  Han-Lo sources write in romanization (`kap`, `tī`, `hō͘`, …), so they do not
  break the bigram chain. 47 rows, hand-curated 2026-09-28: each hanji is the
  form the 教典 example sentences use for that reading (checked against
  `docs/phonetics/taigi-phonetics-reference.md` and `output/dictionary.csv`; where
  the dictionary lists several accepted spellings — 𪜶/怹, 共/給, 閣/擱, 欲/卜,
  咧/塊, 毋/呣, 袂/𣍐, 這/即, 你/汝, 蹛/滯 — the 教典 examples use only the
  first). `ê` is deliberately absent (的 or 个 by context). Loaded by
  `build/corpus_bigrams.py`, which refuses a row that is not a dictionary word.
- `word_bigrams.tsv` / `word_unigrams.tsv` — cross-word `(hanji, TL)` pair and
  word counts from the aligned Taiwanese corpora (`corpus/taigi-corpus`
  sources icorpus_hanji, moe_kautian 例句, sinpak_900leku, kok4hau7,
  taigi_bible_nt, kipsupin_2009, nmtl_dadwt, khinhoan_pojbh, plus the
  `mapped` articles of `corpus/taigi-typing`, whose text is never committed).
  Raw counts in one column per source (`count` = their sum); sentence-end punctuation
  breaks the pair chain like any other break; pairs
  seen fewer than 2 times are dropped. Regenerate with
  `PYTHONPATH=. python3 -m build.corpus_bigrams` after
  `git submodule update --init --checkout corpus/taigi-corpus`. Not a
  `build.sh` step; `create_association_bin` reads the bigrams and
  `create_dictionary_bin` the unigrams. Method and numbers:
  `docs/reports/2026-09-28-bigram-corpus-spike.md`.
- `khiin_conversions.csv` — tone-conversion pairs from the Khiin project
  used in `build/merge_csv.py` to backfill rows absent from the nine main
  sources. Date TBD.

## Re-capture notes

When refreshing a source:

1. Replace `data/raw/<file>` with the new capture. Include the date in
   the filename if the publisher provides one (`scrape-YYYYMMDD*.json`).
2. Update `Captured:` above with the new date.
3. Re-run `./run.sh` (regenerates every source's `data/<key>.csv`) then `./build.sh` and
   `tools/compare_baseline.py verify`. Content changes will typically
   fail the gate — decide whether to re-capture the baseline (rare,
   explicit user decision) or roll the refresh into a larger update.
