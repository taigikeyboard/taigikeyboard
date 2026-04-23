# sources/

Per-source dictionary inputs discovered by the pipeline. Each subdirectory
contains one `config.yaml` + `data/01_raw/<original>` + a canonical
`data/<key>.csv` produced by `./run.sh`.

## Sources

| key     | chinese_name                   | category      | source bit |
|---------|--------------------------------|---------------|------------|
| kautian | 教育部臺灣台語常用詞辭典       | official      | 0          |
| taigitv | 公視台語新詞辭庫               | official      | 1          |
| itaigi  | iTaigi 華台對照典 (ChhoeTaigi) | community     | 2          |
| sitbut  | 台灣植物名彙 (ChhoeTaigi)      | community     | 3          |
| taihoa  | 台華線頂對照典 (ChhoeTaigi)    | community     | 4          |
| taijit  | 台日大辭典 (ChhoeTaigi)        | community     | 5          |
| kungge  | 台語工藝詞庫                   | official      | 6          |
| stti    | 學科術語辭典                   | official      | 7          |
| khpoo   | 齒盤補充辭典                   | supplementary | 8          |

See `../docs/SOURCES.md` for original URLs, licences, and capture dates,
and each subdirectory's `README.md` for a short description of the
source and its pipeline quirks.

**Category semantics:**
- `official` — government-released (MoE) or vetted publisher output
- `community` — community-maintained releases (ChhoeTaigi project)
- `supplementary` — targeted coverage gap-fillers; smaller footprint
