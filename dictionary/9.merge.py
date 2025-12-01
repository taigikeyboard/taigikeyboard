#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import csv
import logging
from pathlib import Path

def merge_dictionaries():
    """合併 standard.csv 和 extra.csv，extra.csv 作為增補，產生 merge.csv"""

    # 設定 logging 寫入檔案
    logging.basicConfig(
        level=logging.INFO,
        format='%(message)s',
        handlers=[
            logging.FileHandler('log/merge.log', mode='w', encoding='utf-8')
        ]
    )
    logger = logging.getLogger(__name__)

    # 輸入檔案
    standard_file = Path('csv/standard.csv')
    extra_file = Path('csv/extra.csv')

    # 輸出檔案
    output_file = Path('csv/merge.csv')

    # 讀取 standard.csv（主要辭典）
    logger.info(f"正在讀取 {standard_file}...")
    standard_rows = []
    standard_keys = set()

    with open(standard_file, 'r', encoding='utf-8') as csvfile:
        reader = csv.DictReader(csvfile)
        for row in reader:
            standard_rows.append(row)
            # 使用 (poj, tl, hanzi) 作為比較鍵
            key = (
                row.get('poj', ''),
                row.get('tl', ''),
                row.get('hanzi', '')
            )
            standard_keys.add(key)

    logger.info(f"  讀取 {len(standard_rows)} 筆資料")

    # 讀取 extra.csv（額外辭典）
    logger.info(f"正在讀取 {extra_file}...")
    extra_rows = []

    with open(extra_file, 'r', encoding='utf-8') as csvfile:
        reader = csv.DictReader(csvfile)
        extra_rows = list(reader)

    logger.info(f"  讀取 {len(extra_rows)} 筆資料")

    # 比較並加入 standard.csv 沒有的詞
    logger.info("正在比較並增補新詞...")
    added_rows = []

    for row in extra_rows:
        key = (
            row.get('poj', ''),
            row.get('tl', ''),
            row.get('hanzi', '')
        )
        if key not in standard_keys:
            added_rows.append(row)
            standard_rows.append(row)
            standard_keys.add(key)

    logger.info(f"  從 extra.csv 增補 {len(added_rows)} 筆新詞")

    # 按音節數排序
    logger.info("正在按音節數排序...")
    standard_rows.sort(key=lambda row: int(row.get('syllable_count', 0)))

    # 寫入合併後的檔案
    logger.info(f"正在寫入合併檔案: {output_file}")
    with open(output_file, 'w', newline='', encoding='utf-8') as csvfile:
        fieldnames = ['tl', 'poj', 'hanzi', 'tl_no_tone', 'poj_no_tone', 'syllable_count', 'source']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(standard_rows)

    # 輸出統計
    logger.info("\n" + "="*40)
    logger.info("合併統計")
    logger.info("="*40)
    logger.info(f"標準辭典 (standard.csv):  {len(standard_rows) - len(added_rows):,} 筆")
    logger.info(f"額外辭典 (extra.csv):      {len(extra_rows):,} 筆")
    logger.info(f"  其中新增補的詞:          {len(added_rows):,} 筆")
    logger.info(f"  重複未加入:              {len(extra_rows) - len(added_rows):,} 筆")
    logger.info("-" * 40)
    logger.info(f"最終詞典筆數:              {len(standard_rows):,} 筆")
    logger.info("="*40)
    logger.info(f"\n詞典合併完成: {output_file}")

if __name__ == "__main__":
    merge_dictionaries()