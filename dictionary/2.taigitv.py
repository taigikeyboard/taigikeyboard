#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import csv
import json
import logging
from pathlib import Path
from base_processor import BaseProcessor
from kesi import Ku

class TaigitvProcessor(BaseProcessor):
    def __init__(self):
        """初始化台語電視處理器，設定統計計數器與跳過項目清單"""
        super().__init__()
        self.stats['syllable_count_exceeded'] = 0
        self.skipped_items = []

    def convert_to_poj(self, roman: str) -> str:
        """將台羅拼音轉換為白話字（POJ）"""
        ku = Ku(roman)
        return ku.POJ().hanlo

    def process_row(self, row):
        """處理單筆 JSON 資料，將台羅轉換為白話字並生成記錄"""
        results = []

        # 從 JSON 取得 TL 和漢字
        pn_list = row.get('pn', [])
        if not pn_list:
            return results

        hanji = str(row.get('title', '')).strip()

        # 迴圈處理每個 TL
        for tl_roman in pn_list:
            tl_roman = str(tl_roman).strip()

            if not tl_roman:
                continue

            # 清理文字
            tl_variant = self.clean_text(tl_roman)
            tl_variant = self.remove_prefix_hyphens(tl_variant)

            # 跳過空白
            if not tl_variant:
                continue

            # 轉換 TL 為 POJ
            poj_variant = self.convert_to_poj(tl_variant)

            # 計算音節數
            tl_syllables = self.count_syllables(tl_variant)

            # 清理漢字
            cleaned_hanji = self.clean_text(hanji)

            # 如果沒有漢字，使用羅馬字作為漢字
            if not cleaned_hanji:
                cleaned_hanji = tl_variant

            # 檢查音節數量限制（<=3）
            if not self.is_syllable_count_valid(tl_syllables):
                self.stats['syllable_count_exceeded'] += 1
                self.skipped_items.append({
                    'tl': tl_variant,
                    'hanzi': cleaned_hanji,
                    'reason': f'音節數超過3 ({tl_syllables}音節)'
                })
                continue

            # 驗證字符
            tl_valid, invalid_tl = self.validate_character(tl_variant, self.allowed_tl_chars)
            poj_valid, invalid_poj = self.validate_character(poj_variant, self.allowed_poj_chars)

            if not tl_valid or not poj_valid:
                reason = []
                if not tl_valid:
                    reason.append(f'TL含非法字符:{invalid_tl}')
                if not poj_valid:
                    reason.append(f'POJ含非法字符:{invalid_poj}')
                self.skipped_items.append({
                    'tl': tl_variant,
                    'hanzi': cleaned_hanji,
                    'reason': ' '.join(reason)
                })
                continue

            # 生成無聲調版本
            tl_unicode = self.remove_tones(tl_variant)
            poj_unicode = self.remove_tones(poj_variant)

            # 將空格和特殊連字符替換為普通連字符（對有聲調和無聲調版本都要處理）
            tl_variant = tl_variant.replace(' ', '-').replace('　', '-').replace('‑', '-')
            poj_variant = poj_variant.replace(' ', '-').replace('　', '-').replace('‑', '-')
            tl_unicode = tl_unicode.replace(' ', '-').replace('　', '-').replace('‑', '-')
            poj_unicode = poj_unicode.replace(' ', '-').replace('　', '-').replace('‑', '-')

            # 建立結果（羅馬字轉為小寫）
            result = {
                'tl': tl_variant.lower(),
                'poj': poj_variant.lower(),
                'hanzi': cleaned_hanji,
                'tl_no_tone': tl_unicode.lower(),
                'poj_no_tone': poj_unicode.lower(),
                'syllable_count': tl_syllables,
                'source': 'taigitv'
            }

            results.append(result)
            self.stats['total_valid'] += 1

        return results

def main():
    """主程式：讀取台語電視 JSON 檔案，處理資料並輸出為 CSV 格式"""
    # 設定 logging 寫入檔案
    logging.basicConfig(
        level=logging.INFO,
        format='%(message)s',
        handlers=[
            logging.FileHandler('log/taigitv.log', mode='w', encoding='utf-8')
        ]
    )
    logger = logging.getLogger(__name__)

    processor = TaigitvProcessor()
    input_file = Path('raw/scrape-20250928T154651Z.json')
    output_file = Path('csv/taigitv.csv')

    all_rows = []

    # 讀取 JSON 檔案
    with open(input_file, 'r', encoding='utf-8') as f:
        data = json.load(f)

    # 處理每筆記錄
    for item in data:
        processor.stats['total_processed'] += 1
        results = processor.process_row(item)
        all_rows.extend(results)

    # 去重
    unique_rows = processor.deduplicate(all_rows)

    # 寫入 CSV
    with open(output_file, 'w', newline='', encoding='utf-8') as csvfile:
        fieldnames = ['tl', 'poj', 'hanzi', 'tl_no_tone', 'poj_no_tone', 'syllable_count', 'source']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(unique_rows)

    # 輸出統計
    logger.info(f"處理完成")
    logger.info(f"總共處理: {processor.stats['total_processed']} 筆")
    logger.info(f"有效記錄: {processor.stats['total_valid']} 筆")
    logger.info(f"最終輸出: {len(unique_rows)} 筆（去重後）")

    # 輸出被跳過的項目（按原因分類）
    if processor.skipped_items:
        # 按原因分組
        skipped_by_reason = {}
        for item in processor.skipped_items:
            reason = item['reason'].split(' (')[0]  # 取得原因的主要部分
            if reason not in skipped_by_reason:
                skipped_by_reason[reason] = []
            skipped_by_reason[reason].append(item)

        logger.info(f"\n=== 跳過的詞條 (共 {len(processor.skipped_items)} 筆) ===")

        # 按原因顯示
        for reason in sorted(skipped_by_reason.keys()):
            items = skipped_by_reason[reason]
            logger.info(f"\n--- {reason} ({len(items)} 筆) ---")
            for item in items:
                logger.info(f"{item['tl']} ({item['hanzi']}): {item['reason']}")

if __name__ == "__main__":
    main()