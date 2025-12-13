# 台語新詞辭庫

Data

- `01_raw`: 原始 JSON
- `02_extracted`: 從 JSON 提取的 CSV
- `03_variants`: 加入異用字與語音變體
- `04_cleanup`: 清理資料（剔除俚語、正規化、去重複）
- `05_frequency`: 加入詞頻資料
- `06_add_columns`: 加入 POJ、無聲調、音節數欄位
- `07_abbrev`: 產生縮寫變體
- `08_final`: 最終輸出

Scripts

- `01_extract.py`: JSON → CSV
- `02_add_variants.py`: 加入異用字與語音變體
- `03_cleanup.py`: 清理資料
- `04_add_frequency.py`: 加入詞頻
- `05_add_columns.py`: 加入 POJ、無聲調、音節數
- `06_abbrev.py`: 產生縮寫變體
- `07_final.py`: 加入 taigitv 欄位
