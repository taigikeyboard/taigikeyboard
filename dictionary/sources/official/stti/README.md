# stti — 學科術語辭典

Academic-discipline terminology (~4.9k rows, source bit 7), CSV input.
Co-published by MoE / 國家教育研究院.

The `extract` stage invokes a bespoke `expand_variant_readings` routine
(see `common/stages/extract.py:124-170`) that was ported verbatim from
the legacy `01_extract.py` — stti rows frequently list multiple valid
readings separated by `/` or Chinese-comma, and each reading must yield
a distinct row before `cleanup`.

Flow: `extract → cleanup → frequency → poj → numtone → notone → abbrev
→ source → variants`.

See `../../../docs/SOURCES.md` for provenance and licence.
