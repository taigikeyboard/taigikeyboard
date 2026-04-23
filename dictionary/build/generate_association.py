#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
產生 NextWord 用的詞彙關聯資料（字典來源）

從 dictionary.csv 的多字詞產生兩種關聯：

1. 「相鄰字 Bigram」— 單字→單字：
   - 「早起床」→ (早→起), (起→床)

2. 「字→詞 Phrase」— 單字→多字詞（3 字以上的詞才產生）：
   - 「七娘媽生」→ (七→娘媽生), (娘→媽生)

輸入：output/dictionary.csv
輸出：
  - output/dictionary.db（加入 word_association 表）
  - output/word_association.csv（CSV 檔案，方便瀏覽）

註：
  - 此表只包含字典來源的關聯，使用者學習資料存於 user_association.db
  - 字數限制：2-5 字的多字詞
  - 包含詞庫來源欄位（kautian, taigitv, itaigi, sitbut, taihoa, taijit, kungge, stti, khpoo）
"""

import re
import sqlite3

import pandas as pd

from build.common import LOG_DIR, OUTPUT_DIR
from common.logging_utils import setup_logging, log_header
from common.source_bits import ASSOC_SOURCE_COLUMNS

INPUT_FILE = OUTPUT_DIR / "dictionary.csv"
OUTPUT_DB_FILE = OUTPUT_DIR / "dictionary.db"
OUTPUT_CSV_FILE = OUTPUT_DIR / "word_association.csv"
SCRIPT_NAME = "generate_association"

# 字數限制
MIN_WORD_LEN = 2
MAX_WORD_LEN = 5  # NextWord 用 ≤5 字，比前綴搜尋 (≤4 字) 更寬
MAX_NEXT_WORD_LEN = 3  # Phrase association 的 next_word 最多 3 字

# 詞庫來源欄位
SOURCE_COLUMNS = ASSOC_SOURCE_COLUMNS


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


def generate_bigrams(hanzi: str, tl: str, frequency: int, sources: dict) -> list:
    """
    從多字詞產生相鄰字 Bigram

    使用 TL 羅馬字的連字符決定切分邊界：
    - TL 有明確的音節邊界（用 - 或 -- 分隔）
    - 漢字逐字切分後，與 TL 音節數量比對
    - 數量一致才產生 bigram

    Args:
        hanzi: 漢字詞（如「早起床」）
        tl: TL 羅馬字（如「tsá-khí-chhn̂g」）
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

        bigrams.append({
            "prev_word": prev_char,
            "next_word": next_char,
            "next_tl": next_tl,
            "count": frequency,
            "sources": sources.copy()
        })

    return bigrams


def generate_phrase_associations(hanzi: str, tl: str, frequency: int, sources: dict) -> list:
    """
    從 3+ 字詞產生「字→詞」關聯

    For each position i where the remaining suffix has >= 2 chars,
    generate (char[i] → chars[i+1:]).

    Example: 七娘媽生 (tshit-niû-má-senn)
      - Position 0: 七 → 娘媽生 (next_tl = niû-má-senn)
      - Position 1: 娘 → 媽生   (next_tl = má-senn)
      (Position 2: 媽→生 is a single-char bigram, skip)

    Args:
        hanzi: 漢字詞（如「七娘媽生」）
        tl: TL 羅馬字（如「tshit-niû-má-senn」）
        frequency: 詞頻
        sources: 詞庫來源 dict

    Returns:
        Phrase association list
    """
    tl_parts = [p for p in re.split(r'-+', tl) if p] if tl and not pd.isna(tl) else []
    hanzi_chars = [c for c in hanzi if is_cjk(c)]

    if len(hanzi_chars) < 3:
        return []

    if len(tl_parts) > 0 and len(hanzi_chars) != len(tl_parts):
        return []

    phrases = []
    for i in range(len(hanzi_chars) - 2):  # remaining suffix must be >= 2 chars
        remaining = hanzi_chars[i + 1:]
        if len(remaining) > MAX_NEXT_WORD_LEN:
            continue
        prev_char = hanzi_chars[i]
        next_phrase = "".join(remaining)
        next_tl = "-".join(tl_parts[i + 1:]) if len(tl_parts) > i + 1 else ""

        phrases.append({
            "prev_word": prev_char,
            "next_word": next_phrase,
            "next_tl": next_tl,
            "count": frequency,
            "sources": sources.copy()
        })

    return phrases


def main():
    logger = setup_logging(SCRIPT_NAME, log_dir=LOG_DIR)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, OUTPUT_DB_FILE)

    if not INPUT_FILE.exists():
        logger.error(f"Input file not found: {INPUT_FILE}")
        return

    if not OUTPUT_DB_FILE.exists():
        logger.error(f"dictionary.db not found: {OUTPUT_DB_FILE}")
        logger.error("Please run create_app_db.sh first")
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
    # key: (prev_word, next_word, next_tl) — 不同讀音各自獨立
    # value: {count, sources}
    associations = {}

    for _, row in df_multi.iterrows():
        hanzi = row["hanzi"]
        tl = row.get("tl", "") or ""
        frequency = row.get("frequency", 1) or 1

        if pd.isna(hanzi) or len(hanzi) < MIN_WORD_LEN:
            continue

        # 取得詞庫來源
        sources = {}
        for col in SOURCE_COLUMNS:
            val = row.get(col, 0)
            sources[col] = 1 if (val == 1 or val == "1" or val is True) else 0

        # 產生相鄰字 Bigram + 字→詞 Phrase
        all_assocs = generate_bigrams(hanzi, tl, frequency, sources) + \
            generate_phrase_associations(hanzi, tl, frequency, sources)

        for assoc in all_assocs:
            key = (assoc["prev_word"], assoc["next_word"], assoc["next_tl"])

            if key in associations:
                # 累加頻率，合併來源（OR 邏輯）
                existing = associations[key]
                merged_sources = {}
                for col in SOURCE_COLUMNS:
                    merged_sources[col] = max(existing["sources"].get(col, 0), assoc["sources"].get(col, 0))

                associations[key] = {
                    "count": existing["count"] + assoc["count"],
                    "sources": merged_sources
                }
            else:
                associations[key] = {
                    "count": assoc["count"],
                    "sources": assoc["sources"]
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
            count INTEGER DEFAULT 1,
            {source_columns_sql},
            UNIQUE(prev_word, next_word, next_tl)
        )
    """)

    cursor.execute("CREATE INDEX idx_prev_word ON word_association(prev_word)")

    # 插入資料
    source_placeholders = ", ".join(["?"] * len(SOURCE_COLUMNS))
    source_columns_names = ", ".join(SOURCE_COLUMNS)
    insert_sql = f"""
        INSERT INTO word_association (prev_word, next_word, next_tl, count, {source_columns_names})
        VALUES (?, ?, ?, ?, {source_placeholders})
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
                key[2],  # next_tl
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
        SELECT prev_word, next_word, next_tl, count, {source_columns_names}
        FROM word_association
        ORDER BY count DESC
        LIMIT 20
    """)
    for row in cursor.fetchall():
        # 取得有哪些詞庫來源
        sources_list = [SOURCE_COLUMNS[i] for i in range(len(SOURCE_COLUMNS)) if row[4 + i] == 1]
        sources_str = ",".join(sources_list) if sources_list else "none"
        logger.info(f"  {row[0]} → {row[1]} ({row[2]}): {row[3]} [{sources_str}]")

    conn.close()

    # 產生 CSV 檔案（方便瀏覽）
    logger.info(f"\n[Generating CSV file]")
    csv_data = []
    for key, val in associations.items():
        row = {
            "prev_word": key[0],
            "next_word": key[1],
            "next_tl": key[2],
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
    logger.info(f"  Model: Adjacent character Bigram + Char-to-Phrase")
    logger.info(f"  Word length: {MIN_WORD_LEN}-{MAX_WORD_LEN} chars")
    logger.info(f"  Unique associations: {count}")
    logger.info(f"  Total frequency: {total_count}")
    logger.info(f"  Output DB: {OUTPUT_DB_FILE} (word_association table)")
    logger.info(f"  Output CSV: {OUTPUT_CSV_FILE}")


if __name__ == "__main__":
    main()
