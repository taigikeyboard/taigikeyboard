# kungge — 台語工藝詞庫

Specialised craft / industry terminology (~1k rows, source bit 6),
released by the MoE and scraped as JSON. The filename
`data/01_raw/scrape-20251210.json` records the capture date (2025-12-10).

Pipeline is the same shape as `taigitv`: `extract → cleanup → frequency
→ poj → numtone → notone → abbrev → source → variants`. Small row count
so tier weighting matters — see `common/source_bits.py` `SOURCE_TIERS`.

See `../../../docs/SOURCES.md` for provenance and licence.
