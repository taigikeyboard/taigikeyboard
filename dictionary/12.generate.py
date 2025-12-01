#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import csv
import logging
import re
from pathlib import Path

def convert_nn_to_nasal(text):
    """將 n-n 轉換為 ⁿ（處理大小寫）"""
    # 處理小寫 n-n
    text = re.sub(r'n-n', 'ⁿ', text)
    # 處理大寫 N-N
    text = re.sub(r'N-N', 'ⁿ', text)
    # 處理混合大小寫 N-n 或 n-N
    text = re.sub(r'[Nn]-[Nn]', 'ⁿ', text)
    return text

def extract_first_letters(syllable_string):
    """
    提取音節首字母（所有音節都只取首字母）

    Args:
        syllable_string: 以 - 分隔的音節字串

    Returns:
        所有音節的首字母組合，若少於兩個音節則回傳 None

    範例:
        "abc-def" → "ad"
        "gua-cha-goa" → "gcg"
    """
    if not syllable_string:
        return None

    # 以 - 分割音節
    syllables = syllable_string.split('-')

    # 過濾空音節
    syllables = [s for s in syllables if s]

    # 檢查音節數量 >= 2
    if len(syllables) < 2:
        return None

    # 所有音節都取首字母
    result_parts = [s[0] for s in syllables]

    # 組合回傳
    return ''.join(result_parts)

def generate_variants():
    """產生變體：將 n-n 轉 ⁿ 和提取首字母的變體"""

    # 設定 logging 寫入檔案
    logging.basicConfig(
        level=logging.INFO,
        format='%(message)s',
        handlers=[
            logging.FileHandler('log/generate.log', mode='w', encoding='utf-8')
        ]
    )
    logger = logging.getLogger(__name__)

    # 輸入輸出檔案
    input_file = Path('csv/clean.csv')
    output_file = Path('csv/dictionary.csv')

    # 讀取資料
    logger.info(f"正在讀取 {input_file}...")
    original_rows = []

    with open(input_file, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        fieldnames = reader.fieldnames
        original_rows = list(reader)

    logger.info(f"  讀取 {len(original_rows)} 筆資料")

    # 儲存所有資料（原始 + 變體）
    all_rows = []

    # 用於去重的集合
    seen_keys = set()

    # 統計
    nn_variants_added = 0
    first_letter_variants_added = 0
    duplicates_skipped = 0

    # 處理每一筆資料
    for row in original_rows:
        tl = row.get('tl', '')
        poj = row.get('poj', '')
        hanzi = row.get('hanzi', '')
        tl_no_tone = row.get('tl_no_tone', '')
        poj_no_tone = row.get('poj_no_tone', '')
        syllable_count = row.get('syllable_count', '')
        source = row.get('source', '')

        # 建立唯一鍵用於去重
        def add_row_if_unique(tl_val, poj_val, hanzi_val, tl_nt_val, poj_nt_val, syllable_val, source_val):
            key = (tl_val, poj_val, hanzi_val, tl_nt_val, poj_nt_val)
            if key not in seen_keys:
                seen_keys.add(key)
                all_rows.append({
                    'tl': tl_val,
                    'poj': poj_val,
                    'hanzi': hanzi_val,
                    'tl_no_tone': tl_nt_val,
                    'poj_no_tone': poj_nt_val,
                    'syllable_count': syllable_val,
                    'source': source_val
                })
                return True
            return False

        # 1. 加入原始資料
        add_row_if_unique(tl, poj, hanzi, tl_no_tone, poj_no_tone, syllable_count, source)

        # 2. 檢查 POJ 是否包含 n-n 形式，產生鼻化音變體
        if re.search(r'[Nn]-[Nn]', poj):
            # 生成額外的鼻化音版本
            poj_no_tone_nasal = convert_nn_to_nasal(poj_no_tone)

            # 注意：tl 和 poj 保持原樣，只有 poj_no_tone 轉換
            if add_row_if_unique(tl, poj, hanzi, tl_no_tone, poj_no_tone_nasal, syllable_count, source):
                nn_variants_added += 1
                logger.info(f"[n-n→ⁿ] poj_no_tone: {poj_no_tone} → {poj_no_tone_nasal}")

        # 3. 提取首字母（針對兩個以上音節）
        poj_first_letters = extract_first_letters(poj_no_tone)
        tl_first_letters = extract_first_letters(tl_no_tone)

        if poj_first_letters or tl_first_letters:
            # 使用首字母版本，沒有則用原本的
            tl_nt_variant = tl_first_letters if tl_first_letters else tl_no_tone
            poj_nt_variant = poj_first_letters if poj_first_letters else poj_no_tone

            if add_row_if_unique(tl, poj, hanzi, tl_nt_variant, poj_nt_variant, syllable_count, source):
                first_letter_variants_added += 1
                logger.info(f"[首字母] tl_no_tone: {tl_no_tone} → {tl_nt_variant}, poj_no_tone: {poj_no_tone} → {poj_nt_variant}")

    # 寫入檔案
    logger.info(f"\n正在寫入 {output_file}...")
    with open(output_file, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(all_rows)

    # 輸出統計
    logger.info("\n" + "="*40)
    logger.info("變體產生統計")
    logger.info("="*40)
    logger.info(f"原始資料:           {len(original_rows):,} 筆")
    logger.info(f"n-n→ⁿ 變體:          {nn_variants_added:,} 筆")
    logger.info(f"首字母變體:         {first_letter_variants_added:,} 筆")
    logger.info(f"重複略過:           {duplicates_skipped:,} 筆")
    logger.info("-" * 40)
    logger.info(f"最終資料:           {len(all_rows):,} 筆")
    logger.info("="*40)
    logger.info(f"\n辭典產生完成: {output_file}")

if __name__ == "__main__":
    generate_variants()
