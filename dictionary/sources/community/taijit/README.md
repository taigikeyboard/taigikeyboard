# taijit — 台日大辭典

ChhoeTaigi's digitisation of 小川尚義《台日大辭典》 (Ogawa Naoyoshi,
1931-32). Largest community source (~64k rows, source bit 5). Primary
is public-domain; digitisation licensed under CC BY-SA 4.0 by
ChhoeTaigi (<https://github.com/ChhoeTaigi/ChhoeTaigiDatabase>).

Pipeline: `extract → expand → cleanup → frequency → poj → numtone →
notone → abbrev → source → variants`. Uses `no_hanzi: drop`. Historical
spellings occasionally mismatch modern TL after tone conversion; the
`cleanup` stage drops rows where `kesi-tui-be-tse` (roman↔hanji) cannot
reconcile.

See `../../../docs/SOURCES.md` for provenance and licence.
