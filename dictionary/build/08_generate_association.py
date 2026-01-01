#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
產生 NextWord 用的詞彙關聯資料（字典來源）

從 dictionary.csv 的多字詞產生「相鄰字 Bigram」關聯：
- 「早餐」→ 早 → 餐
- 「早起床」→ 早 → 起、起 → 床

輸入：output/dictionary.csv
輸出：
  - output/dictionary.db（加入 word_association 表）
  - output/word_association.csv（CSV 檔案，方便瀏覽）

註：
  - 此表只包含字典來源的關聯，使用者學習資料存於 user_association.db
  - 字數限制：2-3 字的多字詞（與 dictionary 一致）
  - 包含詞庫來源欄位（kautian, taigitv, itaigi, sitbut, taihoa, taijit, kungge）
"""

import os
import sys
import re
import sqlite3
import pandas as pd
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
INPUT_FILE = os.path.join(BASE_DIR, "output", "dictionary.csv")
OUTPUT_DB_FILE = os.path.join(BASE_DIR, "output", "dictionary.db")
OUTPUT_CSV_FILE = os.path.join(BASE_DIR, "output", "word_association.csv")
SCRIPT_NAME = "08_generate_association"

# 字數限制
MIN_WORD_LEN = 2
MAX_WORD_LEN = 5  # NextWord 用 ≤5 字，比前綴搜尋 (≤4 字) 更寬

# 詞庫來源欄位
SOURCE_COLUMNS = ["kautian", "taigitv", "itaigi", "sitbut", "taihoa", "taijit", "kungge"]


def is_cjk(char: str) -> bool:
    """
    判斷字元是否為 CJK 漢字

    CJK Unicode 範圍：
    - CJK Unified Ideographs: U+4E00 - U+9FFF
    - CJK Unified Ideographs Extension A: U+3400 - U+4DBF
    - CJK Unified Ideographs Extension B-F: U+20000 - U+2CEAF
    - CJK Compatibility Ideographs: U+F900 - U+FAFF
    """
    if len(char) != 1:
        return False
    code = ord(char)
    return (
        0x4E00 <= code <= 0x9FFF or      # CJK Unified Ideographs
        0x3400 <= code <= 0x4DBF or      # Extension A
        0x20000 <= code <= 0x2CEAF or    # Extension B-F
        0xF900 <= code <= 0xFAFF         # Compatibility
    )


def is_valid_part(part: str) -> bool:
    """
    判斷切分後的部分是否有效

    規則：
    - 空字串：無效
    - 單一字元：必須是 CJK 漢字
    - 多字元：有效（可能是羅馬字如 "tshit"）
    """
    if not part:
        return False
    if len(part) == 1:
        return is_cjk(part)
    return True


def generate_bigrams(hanzi: str, tl: str, poj: str, frequency: int, sources: dict) -> list:
    """
    從多字詞產生相鄰字 Bigram

    使用 TL 羅馬字的連字符決定切分邊界：
    - TL 有明確的音節邊界（用 - 或 -- 分隔）
    - 漢字逐字切分後，與 TL 音節數量比對
    - 數量一致才產生 bigram

    Args:
        hanzi: 漢字詞（如「早起床」）
        tl: TL 羅馬字（如「tsá-khí-chhn̂g」）
        poj: POJ 羅馬字
        frequency: 詞頻
        sources: 詞庫來源 dict（如 {"kautian": 1, "itaigi": 0, ...}）

    Returns:
        Bigram 列表

    範例:
        hanzi="早起床", tl="tsá-khí-chhn̂g"
        → tl_parts = ["tsá", "khí", "chhn̂g"] (3 音節)
        → hanzi_chars = ["早", "起", "床"] (3 字)
        → bigrams: (早, 起), (起, 床)
    """
    # 用 TL 的連字符切分音節（支援 - 和 --）
    tl_parts = [p for p in re.split(r'-+', tl) if p] if tl and not pd.isna(tl) else []
    poj_parts = [p for p in re.split(r'-+', poj) if p] if poj and not pd.isna(poj) else []

    # 漢字逐字切分
    hanzi_chars = list(hanzi) if hanzi else []

    # 過濾無效字元（必須是 CJK 漢字）
    hanzi_chars = [c for c in hanzi_chars if is_cjk(c)]

    # 檢查數量是否一致（漢字數 = TL 音節數）
    # 不一致表示資料品質問題，跳過
    if len(hanzi_chars) < 2:
        return []

    if len(tl_parts) > 0 and len(hanzi_chars) != len(tl_parts):
        # 數量不一致，可能是髒資料，跳過
        return []

    bigrams = []
    for i in range(len(hanzi_chars) - 1):
        prev_char = hanzi_chars[i]
        next_char = hanzi_chars[i + 1]

        # 取得下一個字的羅馬字（索引 i+1）
        next_tl = tl_parts[i + 1] if i + 1 < len(tl_parts) else ""
        next_poj = poj_parts[i + 1] if i + 1 < len(poj_parts) else ""

        bigrams.append({
            "prev_word": prev_char,
            "next_word": next_char,
            "next_tl": next_tl,
            "next_poj": next_poj,
            "delimiter": "-",  # 來自同一詞，用連字符
            "count": frequency,
            "sources": sources.copy()
        })

    return bigrams


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, OUTPUT_DB_FILE)

    if not os.path.exists(INPUT_FILE):
        logger.error(f"Input file not found: {INPUT_FILE}")
        return

    if not os.path.exists(OUTPUT_DB_FILE):
        logger.error(f"dictionary.db not found: {OUTPUT_DB_FILE}")
        logger.error("Please run 02_create_app_db.sh first")
        return

    # 讀取字典
    df = pd.read_csv(INPUT_FILE)
    logger.info(f"Loaded {len(df)} records from dictionary.csv")

    # 篩選多字詞（字數限制：MIN_WORD_LEN ~ MAX_WORD_LEN）
    df_multi = df[df["hanzi"].apply(
        lambda x: isinstance(x, str) and MIN_WORD_LEN <= len(x) <= MAX_WORD_LEN
    )].copy()
    logger.info(f"Multi-char words ({MIN_WORD_LEN}-{MAX_WORD_LEN} chars): {len(df_multi)}")

    # 提取「相鄰字 Bigram」關聯
    # key: (prev_word, next_word)
    # value: {count, next_tl, next_poj, delimiter, sources}
    associations = {}

    for _, row in df_multi.iterrows():
        hanzi = row["hanzi"]
        tl = row.get("tl", "") or ""
        poj = row.get("poj", "") or ""
        frequency = row.get("frequency", 1) or 1

        if pd.isna(hanzi) or len(hanzi) < MIN_WORD_LEN:
            continue

        # 取得詞庫來源
        sources = {}
        for col in SOURCE_COLUMNS:
            val = row.get(col, 0)
            sources[col] = 1 if (val == 1 or val == "1" or val is True) else 0

        # 產生相鄰字 Bigram
        for bigram in generate_bigrams(hanzi, tl, poj, frequency, sources):
            key = (bigram["prev_word"], bigram["next_word"])

            if key in associations:
                # 累加頻率，保留有值的羅馬字，合併來源（OR 邏輯）
                existing = associations[key]
                merged_sources = {}
                for col in SOURCE_COLUMNS:
                    merged_sources[col] = max(existing["sources"].get(col, 0), bigram["sources"].get(col, 0))

                associations[key] = {
                    "count": existing["count"] + bigram["count"],
                    "next_tl": bigram["next_tl"] or existing["next_tl"],
                    "next_poj": bigram["next_poj"] or existing["next_poj"],
                    "delimiter": bigram["delimiter"],
                    "sources": merged_sources
                }
            else:
                associations[key] = {
                    "count": bigram["count"],
                    "next_tl": bigram["next_tl"],
                    "next_poj": bigram["next_poj"],
                    "delimiter": bigram["delimiter"],
                    "sources": bigram["sources"]
                }

    logger.info(f"Total unique associations: {len(associations)}")

    # 連接現有的 dictionary.db，重建 word_association 表
    conn = sqlite3.connect(OUTPUT_DB_FILE)
    cursor = conn.cursor()

    # 刪除舊表（如果存在）
    cursor.execute("DROP TABLE IF EXISTS word_association")
    cursor.execute("DROP INDEX IF EXISTS idx_prev_word")

    # 建立表格（相鄰字 Bigram + 詞庫來源）
    source_columns_sql = ",\n            ".join([f"{col} INTEGER DEFAULT 0" for col in SOURCE_COLUMNS])
    cursor.execute(f"""
        CREATE TABLE word_association (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            prev_word TEXT NOT NULL,
            next_word TEXT NOT NULL,
            next_tl TEXT,
            next_poj TEXT,
            delimiter TEXT DEFAULT '-',
            count INTEGER DEFAULT 1,
            {source_columns_sql},
            UNIQUE(prev_word, next_word)
        )
    """)

    cursor.execute("CREATE INDEX idx_prev_word ON word_association(prev_word)")

    # 插入資料
    source_placeholders = ", ".join(["?"] * len(SOURCE_COLUMNS))
    source_columns_names = ", ".join(SOURCE_COLUMNS)
    insert_sql = f"""
        INSERT INTO word_association (prev_word, next_word, next_tl, next_poj, delimiter, count, {source_columns_names})
        VALUES (?, ?, ?, ?, ?, ?, {source_placeholders})
    """

    batch_size = 10000
    association_list = list(associations.items())
    total = len(association_list)

    for i in range(0, total, batch_size):
        batch = association_list[i:i + batch_size]
        data = []
        for key, val in batch:
            row = [
                key[0],  # prev_word
                key[1],  # next_word
                val["next_tl"],
                val["next_poj"],
                val["delimiter"],
                val["count"]
            ]
            # 加入詞庫來源欄位
            for col in SOURCE_COLUMNS:
                row.append(val["sources"].get(col, 0))
            data.append(tuple(row))
        cursor.executemany(insert_sql, data)
        conn.commit()
        logger.info(f"  Inserted {min(i + batch_size, total)}/{total} associations")

    # 統計
    cursor.execute("SELECT COUNT(*) FROM word_association")
    count = cursor.fetchone()[0]

    cursor.execute("SELECT SUM(count) FROM word_association")
    total_count = cursor.fetchone()[0]

    # 顯示範例
    logger.info(f"\n[Sample associations (top 20 by count)]:")
    cursor.execute(f"""
        SELECT prev_word, next_word, next_tl, next_poj, delimiter, count, {source_columns_names}
        FROM word_association
        ORDER BY count DESC
        LIMIT 20
    """)
    for row in cursor.fetchall():
        # 取得有哪些詞庫來源
        sources_list = [SOURCE_COLUMNS[i] for i in range(len(SOURCE_COLUMNS)) if row[6 + i] == 1]
        sources_str = ",".join(sources_list) if sources_list else "none"
        logger.info(f"  {row[0]} → {row[1]} ({row[2]}/{row[3]}, '{row[4]}'): {row[5]} [{sources_str}]")

    conn.close()

    # 產生 CSV 檔案（方便瀏覽）
    logger.info(f"\n[Generating CSV file]")
    csv_data = []
    for key, val in associations.items():
        row = {
            "prev_word": key[0],
            "next_word": key[1],
            "next_tl": val["next_tl"],
            "next_poj": val["next_poj"],
            "delimiter": val["delimiter"],
            "count": val["count"]
        }
        # 加入詞庫來源欄位
        for col in SOURCE_COLUMNS:
            row[col] = val["sources"].get(col, 0)
        csv_data.append(row)
    df_csv = pd.DataFrame(csv_data)
    df_csv = df_csv.sort_values(by=["count", "prev_word"], ascending=[False, True])
    df_csv.to_csv(OUTPUT_CSV_FILE, index=False, encoding="utf-8")
    logger.info(f"  CSV file: {OUTPUT_CSV_FILE}")

    logger.info(f"\n[Summary]")
    logger.info(f"  Model: Adjacent character Bigram")
    logger.info(f"  Word length: {MIN_WORD_LEN}-{MAX_WORD_LEN} chars")
    logger.info(f"  Unique associations: {count}")
    logger.info(f"  Total frequency: {total_count}")
    logger.info(f"  Output DB: {OUTPUT_DB_FILE} (word_association table)")
    logger.info(f"  Output CSV: {OUTPUT_CSV_FILE}")


if __name__ == "__main__":
    main()
