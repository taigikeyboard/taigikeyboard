#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import csv
import logging
import re
from pathlib import Path

def contains_invalid_symbols(text):
    """檢查文字是否含有無效符號（括號、驚嘆號、問號等，但保留連字符）"""
    # 定義要排除的符號模式：括號、驚嘆號、問號、斜線、@、#、$、%、&、*、+、=、|、<、>、[、]、{、}等
    # 保留連字符 - 和底線 _
    invalid_pattern = r'[()（）[\]【】{}｛｝<>《》!！?？/\\@#$%&*+=|~`^]'
    return bool(re.search(invalid_pattern, text))

def cleanup_dictionary():
    """清理辭典資料：移除指定列及含有無效符號的資料"""

    # 設定 logging 寫入檔案
    logging.basicConfig(
        level=logging.INFO,
        format='%(message)s',
        handlers=[
            logging.FileHandler('log/cleanup.log', mode='w', encoding='utf-8')
        ]
    )
    logger = logging.getLogger(__name__)

    # 在這裡加入要移除的完整列（不含表頭）
    rows_to_remove = [
        'tshì-pho,chhì-pho,草莓,tshi-pho,chhi-pho,2,itaigi',
    ]

    # 輸入輸出檔案
    input_file = Path('csv/merge.csv')
    output_file = Path('csv/clean.csv')

    rows_to_remove_set = set(rows_to_remove)
    kept_rows = []
    removed_by_list = []
    removed_by_symbols = []

    # 讀取並處理資料
    logger.info(f"正在讀取 {input_file}...")
    with open(input_file, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        fieldnames = reader.fieldnames

        for row in reader:
            # 建立不含 source 的列字串用於比對
            row_string = ','.join([
                row.get('tl', ''),
                row.get('poj', ''),
                row.get('hanzi', ''),
                row.get('tl_no_tone', ''),
                row.get('poj_no_tone', ''),
                row.get('syllable_count', ''),
                row.get('source', '')
            ])

            # 檢查是否在移除清單中
            if row_string in rows_to_remove_set:
                removed_by_list.append(row)
                logger.info(f"[清單移除] {row_string}")
                continue

            # 檢查所有欄位是否含有無效符號
            has_invalid = False
            for field in ['tl', 'poj', 'hanzi']:
                value = row.get(field, '')
                if contains_invalid_symbols(value):
                    has_invalid = True
                    removed_by_symbols.append(row)
                    logger.info(f"[符號移除] {row_string} (欄位: {field}, 值: {value})")
                    break

            if not has_invalid:
                kept_rows.append(row)

    # 寫入清理後的資料
    logger.info(f"\n正在寫入 {output_file}...")
    with open(output_file, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(kept_rows)

    # 輸出統計
    total_removed = len(removed_by_list) + len(removed_by_symbols)
    logger.info("\n" + "="*40)
    logger.info("清理統計")
    logger.info("="*40)
    logger.info(f"讀取資料:           {len(kept_rows) + total_removed:,} 筆")
    logger.info(f"清單移除:           {len(removed_by_list):,} 筆")
    logger.info(f"符號移除:           {len(removed_by_symbols):,} 筆")
    logger.info(f"總共移除:           {total_removed:,} 筆")
    logger.info(f"保留資料:           {len(kept_rows):,} 筆")
    logger.info("="*40)
    logger.info(f"\n辭典清理完成: {output_file}")

if __name__ == "__main__":
    cleanup_dictionary()
