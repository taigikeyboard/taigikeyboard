# khpoo — 齒盤補充辭典

Keyboard-specific supplementary set (~4.2k rows, source bit 8). Input
is a headerless CSV of already-curated entries (see
`config.yaml[input_format]: csv_noheader`).

Pipeline: `extract → cleanup → frequency → poj → numtone → notone →
abbrev → source → variants`. `cleanup.minimal: true` — because entries
are pre-processed, only `normalize_roman` runs (no proverb/length/hanlo
checks); full cleanup would strip entries the editor intentionally
accepted.

See `../../../docs/SOURCES.md` for provenance and licence.
