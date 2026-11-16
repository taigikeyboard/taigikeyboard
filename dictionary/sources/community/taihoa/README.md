# taihoa — 台華線頂對照典

Broadest community Taigi↔Mandarin crossreference (~57k rows, source bit
4), from the ChhoeTaigi project
(<https://github.com/ChhoeTaigi/ChhoeTaigiDatabase>). Second only to
kautian in footprint.

Pipeline: `extract → expand → cleanup → frequency → poj → numtone →
notone → abbrev → source → variants`. Uses `no_hanzi: drop`.

See `../../../docs/SOURCES.md` for provenance and licence.

Cleanup notes:
- **Han-Lo `--ê` normalization**: 15 rows written `<Hanzi>ê` with neutral-tone `<tl>--ê` (e.g. `大ê,tuā--ê`) normalize to `<Hanzi>--ê` (`大--ê`) in `cleanup` so Han-Lo neutral-tone roman suffixes match `矮--ê`, `邊--a`, and `尾--à`.
- **Truncated IDS `-` fragments (`drop_hyphen_in_pure_hanzi: true`)**: raw `HanLoTaibunKip` uses `-` as a missing-glyph component prefix (`飽-月貴`, `大-片`, `短-示`, `軟-手豪`); the 10 truncated rows that otherwise match syllable count are dropped in `cleanup` rather than stripped into wrong plain-Hanji words.
