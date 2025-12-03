#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import csv
import logging
from pathlib import Path

def deduplicate(rows):
    """去除重複的資料（根據 tl, poj, hanzi, tl_no_tone, poj_no_tone）"""
    seen = set()
    unique_rows = []
    duplicate_rows = []

    for row in rows:
        # 使用 (tl, poj, hanzi, tl_no_tone, poj_no_tone) 作為唯一鍵
        key = (
            row.get('tl', ''),
            row.get('poj', ''),
            row.get('hanzi', ''),
            row.get('tl_no_tone', ''),
            row.get('poj_no_tone', '')
        )
        if key not in seen:
            seen.add(key)
            unique_rows.append(row)
        else:
            duplicate_rows.append(row)

    return unique_rows, duplicate_rows

def main():
    """主程式：合併教典和台語電視資料，去重後輸出為標準資料集"""
    # 設定 logging 寫入檔案
    logging.basicConfig(
        level=logging.INFO,
        format='%(message)s',
        handlers=[
            logging.FileHandler('log/standard.log', mode='w', encoding='utf-8')
        ]
    )
    logger = logging.getLogger(__name__)

    # 設定檔案路徑
    kautian_file = Path('csv/kautian.csv')
    taigitv_file = Path('csv/taigitv.csv')
    output_file = Path('csv/standard.csv')

    all_rows = []

    # 讀取 kautian.csv
    kautian_count = 0
    with open(kautian_file, 'r', encoding='utf-8') as csvfile:
        reader = csv.DictReader(csvfile)
        kautian_rows = list(reader)
        kautian_count = len(kautian_rows)
        all_rows.extend(kautian_rows)

    # 讀取 taigitv.csv
    taigitv_count = 0
    with open(taigitv_file, 'r', encoding='utf-8') as csvfile:
        reader = csv.DictReader(csvfile)
        taigitv_rows = list(reader)
        taigitv_count = len(taigitv_rows)
        all_rows.extend(taigitv_rows)

    # 去重
    unique_rows, duplicate_rows = deduplicate(all_rows)

    # 寫入 CSV
    with open(output_file, 'w', newline='', encoding='utf-8') as csvfile:
        fieldnames = ['tl', 'poj', 'hanzi', 'tl_no_tone', 'poj_no_tone', 'syllable_count', 'source', 'is_variant']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames, extrasaction='ignore')
        writer.writeheader()
        writer.writerows(unique_rows)

    # 輸出統計
    logger.info(f"處理完成")
    logger.info(f"教典資料: {kautian_count} 筆")
    logger.info(f"台語電視資料: {taigitv_count} 筆")
    logger.info(f"合併前總數: {len(all_rows)} 筆")
    logger.info(f"重複記錄: {len(duplicate_rows)} 筆")
    logger.info(f"最終輸出: {len(unique_rows)} 筆（去重後）")

    # 輸出重複記錄詳細資訊
    if duplicate_rows:
        logger.info(f"\n=== 重複的記錄 (共 {len(duplicate_rows)} 筆) ===")
        for row in duplicate_rows:
            logger.info(f"TL: {row.get('tl', '')}, POJ: {row.get('poj', '')}, 漢字: {row.get('hanzi', '')}, 來源: {row.get('source', '')}")

if __name__ == "__main__":
    main()
