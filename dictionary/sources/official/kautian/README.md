# kautian — 教育部臺灣台語常用詞辭典

The largest and most authoritative source in the dictionary (~49k rows,
source bit 0). Published by the Ministry of Education (Taiwan); consumed
as ODS with 18 sheets covering headwords, 異體字, 俗唸作 pronunciations,
dialect variants, and 異用字 cross-references.

Pipeline runs the full 12-stage flow — only source that uses
`select` / `expand` / `merge` in multi-sheet mode. `select` pivots the
ODS sheets into a unified schema; `expand` splits multi-value cells;
`merge` concatenates sheets in sorted order and drops the 異用字 sheet
(the variants table lives in `supplementary/variants/` after a separate
derivation step). The extract's 異用字 sheet is also what seeds
`supplementary/variants/data/variants.csv`.

See `../../../docs/SOURCES.md` for provenance and licence.
