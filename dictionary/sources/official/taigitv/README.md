# taigitv — 公視台語新詞辭庫

Coverage-of-neologisms supplement (~2.1k rows, source bit 1), scraped
from the 公視台語台 site as JSON. The filename
`data/01_raw/scrape-20260319T144906Z.json` records the capture moment.

Pipeline skips the sheet-based stages (input is already per-headword
JSON). Flow: `extract → cleanup → frequency → poj → numtone → notone →
abbrev → source → variants`.

See `../../../docs/SOURCES.md` for provenance and licence.
