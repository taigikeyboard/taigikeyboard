# 教育部臺灣台語常用詞辭典

Data

- `01_raw_ods`: 原始 ODS
- `02_raw_csv`: ODS 拆出的 CSV
- `03_selected`: 篩選欄位、挑表、轉置
- `04_expanded`: 將含 `/` `,` `.` 的欄位展開成多筆資料
- `05_cleaned`: 移除註解、空白、特殊符號
- `06_merged`: 合併整理後的資料
- `07_variants`: 加入異用字與語音變體
- `08_frequency`: 加入詞頻資料
- `09_add_columns`: 增加 POJ、無聲調欄位
- `10_final`: 最終輸出

Scripts

- `02_extract_raw_csv.py`: ODS → CSV
- `03_select_columns.py`: 篩欄位、轉置
- `04_expand_fields.py`: 展開分隔欄位
- `05_clean_data.py`: 清理資料
- `06_merge_datasets.py`: 合併資料
- `07_add_variants.py`: 加入異用字與語音變體
- `08_add_frequency.py`: 加入詞頻資料
- `09_add_columns.py`: 增加 POJ、無聲調、音節數欄位
