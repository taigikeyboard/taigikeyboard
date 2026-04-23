# itaigi — iTaigi 華台對照典

Community-curated Mandarin→Taigi crossreference (~15k rows, source bit 2),
distributed via the ChhoeTaigi project (<https://github.com/ChhoeTaigi/ChhoeTaigiDatabase>).
Upstream of ChhoeTaigi is <https://itaigi.tw>.

Pipeline: `extract → expand → cleanup → frequency → poj → numtone →
notone → abbrev → source → variants`. `expand` splits rows with `/` or
comma-separated variant readings. `source` stage uses
`no_hanzi: drop` policy (ChhoeTaigi releases sometimes have empty hanzi
for romanization-only entries; they're excluded from the main index).

See `../../../docs/SOURCES.md` for provenance and licence.
