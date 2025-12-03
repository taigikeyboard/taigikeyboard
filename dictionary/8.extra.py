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
    """主程式：合併 itaigi, sitbut, taigitv, taihoa, taijit 資料，去重後輸出為額外資料集"""
    # 設定 logging 寫入檔案
    logging.basicConfig(
        level=logging.INFO,
        format='%(message)s',
        handlers=[
            logging.FileHandler('log/extra.log', mode='w', encoding='utf-8')
        ]
    )
    logger = logging.getLogger(__name__)

    # 設定檔案路徑
    input_files = {
        'itaigi': Path('csv/itaigi.csv'),
        'sitbut': Path('csv/sitbut.csv'),
        'taigitv': Path('csv/taigitv.csv'),
        'taihoa': Path('csv/taihoa.csv'),
        'taijit': Path('csv/taijit.csv')
    }
    output_file = Path('csv/extra.csv')

    all_rows = []
    file_counts = {}

    # 讀取所有輸入檔案
    for name, file_path in input_files.items():
        with open(file_path, 'r', encoding='utf-8') as csvfile:
            reader = csv.DictReader(csvfile)
            rows = list(reader)
            file_counts[name] = len(rows)
            all_rows.extend(rows)
            logger.info(f"{name} 資料: {len(rows)} 筆")

    # 去重
    unique_rows, duplicate_rows = deduplicate(all_rows)

    # 寫入 CSV
    with open(output_file, 'w', newline='', encoding='utf-8') as csvfile:
        fieldnames = ['tl', 'poj', 'hanzi', 'tl_no_tone', 'poj_no_tone', 'syllable_count', 'source']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(unique_rows)

    # 輸出統計
    logger.info(f"\n處理完成")
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
