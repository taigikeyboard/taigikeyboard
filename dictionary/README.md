# 台語詞目處理工具

本工具可將教育部《臺灣台語常用詞辭典》的詞目資料（ODS 格式）轉換為 SQLite 資料庫，適用於輸入法開發、查詢系統建置等用途。

---

## 功能說明

- 解析原始 ODS 詞目資料（kautian.ods）
- 清理資料並輸出為結構化 CSV
- 自動產生台羅、白話字
- 產出無聲調欄位（base_form）
- 建立可查詢的 SQLite 資料庫

---

## 使用步驟

### 1. 下載原始資料

請至教育部「臺灣台語常用詞辭典」網站下載詞目檔：

[https://sutian.moe.edu.tw/zh-hant/siongkuantsuguan/](https://sutian.moe.edu.tw/zh-hant/siongkuantsuguan/)

下載後將 `kautian.ods` 放置於本專案目錄下。

### 2. 安裝必要套件

請使用 Python 3.8 以上版本，並安裝所需套件：

```bash
pip install pandas odfpy
```

或是

```bash
pip install -r requirements.txt
```

### 3. 清理並轉換資料

執行以下指令，將 ODS 檔轉換為結構化的 CSV：

```bash
python clean.py
```

輸出結果為 `kautian.csv`，內容包含教育部台語羅馬字與 POJ。

### 4. 建立 SQLite 資料庫

```bash
./create_db.sh
```

此腳本會建立 `dictionary.db`。


## 輸出格式範例（CSV）

| roman            | hanzi    | base_form        | category | variant_type | created_at |
|------------------|----------|------------------|----------|--------------|------------|
| chhit-niû-má-seⁿ | 七娘媽生 | chhit-niu-ma-seⁿ | word     | poj          | 2025-08-01 |
| chhit-niû-má-siⁿ | 七娘媽生 | chhit-niu-ma-siⁿ | word     | poj          | 2025-08-01 |
| jîn-ke           | 人家     | jin-ke           | word     | poj          | 2025-08-01 |
| lîn-ke           | 人家     | lin-ke           | word     | poj          | 2025-08-01 |
| jîn-ke-chhù-á    | 人家厝仔 | jin-ke-chhu-a    | word     | poj          | 2025-08-01 |
| lîn-ke-chhù-á    | 人家厝仔 | lin-ke-chhu-a    | word     | poj          | 2025-08-01 |
| pat-ka-chiòng    | 八家將   | pat-ka-chiong    | word     | poj          | 2025-08-01 |
