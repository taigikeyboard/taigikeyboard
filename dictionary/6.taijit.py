#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import csv
import logging
import unicodedata
from pathlib import Path
from base_processor import BaseProcessor

class TaijitProcessor(BaseProcessor):
    def __init__(self):
        """初始化台日大辭典處理器，設定統計計數器與跳過項目清單"""
        super().__init__()
        self.stats['syllable_count_exceeded'] = 0
        self.skipped_items = []
        self.special_encoding_items = []  # 記錄包含特殊編碼的項目

    def normalize_to_kautian_encoding(self, text):
        """將文字正規化為與 kautian.csv 一致的編碼形式"""
        if not text:
            return text, []

        special_chars_found = []

        # 先替換 headless i 為普通 i（保留原始聲調）
        if 'ɪ' in text:
            special_chars_found.append('ɪ (U+026A, headless i)')
            text = text.replace('ɪ', 'i')
        if 'ı' in text:
            special_chars_found.append('ı (U+0131, dotless i)')
            text = text.replace('ı', 'i')
        if 'Ɪ' in text:
            special_chars_found.append('Ɪ (U+0196, uppercase headless I)')
            text = text.replace('Ɪ', 'I')
        if 'İ' in text:
            special_chars_found.append('İ (U+0130, uppercase dotted I)')
            text = text.replace('İ', 'I')

        # 進行 NFD 分解，然後手動處理特定情況
        normalized = unicodedata.normalize('NFD', text)

        # 對於第8聲調，保持組合字元形式 (基本字母 + U+030D)
        # 對於其他聲調，轉換為預組合形式
        result = ''
        i = 0
        while i < len(normalized):
            char = normalized[i]

            # 檢查是否有組合字元跟隨
            if i + 1 < len(normalized):
                combining_char = normalized[i + 1]
                combining_category = unicodedata.category(combining_char)

                if combining_category == 'Mn':  # 組合字元
                    if combining_char == '\u030d':  # COMBINING VERTICAL LINE ABOVE (第8聲調)
                        # 保持組合字元形式
                        result += char + combining_char
                        i += 2
                        continue
                    else:
                        # 其他聲調：嘗試轉換為預組合形式
                        combined = char + combining_char
                        composed = unicodedata.normalize('NFC', combined)
                        if len(composed) == 1:  # 成功組合為單一字元
                            result += composed
                            i += 2
                            continue

            # 沒有組合字元或無法組合，直接添加字元
            result += char
            i += 1

        # 最後進行 NFC 正規化，確保所有可組合的字元都被正確組合
        # 但第8聲調（U+030D）已在前面特別處理，會保持組合字元形式
        result = unicodedata.normalize('NFC', result)

        return result, special_chars_found

    def process_row(self, row):
        """處理單筆 CSV 資料，將台羅和白話字配對並生成記錄"""
        results = []

        # 從原始 CSV 欄位讀取資料
        # PojUnicode: 白話字（POJ）拼音
        # KipUnicode: 台羅（TL）拼音
        # HanLoTaibunKip: 漢羅台文（漢字）
        poj = str(row.get('PojUnicode', '')).strip()
        tl = str(row.get('KipUnicode', '')).strip()
        hanji = str(row.get('HanLoTaibunKip', '')).strip()

        if not tl or not poj:
            return results

        # 分割變體（先按 / 再按 ,）
        tl_variants = []
        poj_variants = []
        hanji_variants = []

        # 分割 TL
        for tl_part in tl.split('/'):
            tl_variants.extend([v.strip() for v in tl_part.split(',') if v.strip()])

        # 分割 POJ
        for poj_part in poj.split('/'):
            poj_variants.extend([v.strip() for v in poj_part.split(',') if v.strip()])

        # 分割漢字
        for hanji_part in hanji.split('/'):
            hanji_variants.extend([v.strip() for v in hanji_part.split(',') if v.strip()])

        # 確保數量一致
        max_variants = max(len(tl_variants), len(poj_variants), len(hanji_variants))

        # 如果只有一個值，擴展到相同數量
        if len(tl_variants) == 1 and max_variants > 1:
            tl_variants = tl_variants * max_variants
        if len(poj_variants) == 1 and max_variants > 1:
            poj_variants = poj_variants * max_variants
        if len(hanji_variants) == 1 and max_variants > 1:
            hanji_variants = hanji_variants * max_variants

        # 處理每個變體組合
        for i in range(max_variants):
            tl_variant = tl_variants[i] if i < len(tl_variants) else (tl_variants[0] if tl_variants else '')
            poj_variant = poj_variants[i] if i < len(poj_variants) else (poj_variants[0] if poj_variants else '')
            hanji_variant = hanji_variants[i] if i < len(hanji_variants) else (hanji_variants[0] if hanji_variants else '')

            # 清理變體
            tl_variant = self.clean_text(tl_variant)
            poj_variant = self.clean_text(poj_variant)
            hanji_variant = self.clean_text(hanji_variant)

            # 正規化編碼以匹配 kautian.csv 格式
            tl_variant, tl_special_chars = self.normalize_to_kautian_encoding(tl_variant)
            poj_variant, poj_special_chars = self.normalize_to_kautian_encoding(poj_variant)

            # 記錄包含特殊編碼的項目
            if tl_special_chars or poj_special_chars:
                self.special_encoding_items.append({
                    'tl': tl_variant,
                    'poj': poj_variant,
                    'hanzi': hanji_variant,
                    'tl_special': tl_special_chars,
                    'poj_special': poj_special_chars
                })

            # 移除不可見字符
            tl_variant = tl_variant.replace('\u200b', '')  # zero-width space
            poj_variant = poj_variant.replace('\u200b', '')  # zero-width space

            tl_variant = self.remove_prefix_hyphens(tl_variant)
            poj_variant = self.remove_prefix_hyphens(poj_variant)

            # 先生成無聲調版本（在替換空格之前）
            tl_unicode = self.remove_tones(tl_variant)
            poj_unicode = self.remove_tones(poj_variant)

            # 將空格和特殊連字符替換為普通連字符（對有聲調和無聲調版本都要處理）
            tl_variant = tl_variant.replace(' ', '-').replace('　', '-').replace('‑', '-')
            poj_variant = poj_variant.replace(' ', '-').replace('　', '-').replace('‑', '-')
            tl_unicode = tl_unicode.replace(' ', '-').replace('　', '-').replace('‑', '-')
            poj_unicode = poj_unicode.replace(' ', '-').replace('　', '-').replace('‑', '-')

            # 跳過空白變體
            if not tl_variant:
                continue

            # 如果沒有漢字，使用羅馬字
            if not hanji_variant:
                hanji_variant = tl_variant

            # 檢查漢字欄位是否包含空格
            if ' ' in hanji_variant:
                self.stats['hanzi_with_space'] = self.stats.get('hanzi_with_space', 0) + 1
                self.skipped_items.append({
                    'tl': tl_variant,
                    'hanzi': hanji_variant,
                    'reason': f'漢字欄位包含空格'
                })
                continue

            # 計算音節數
            tl_syllables = self.count_syllables(tl_variant)
            hanzi_count = self.count_hanzi_characters(hanji_variant)

            # 檢查音節數量限制（<=3）
            if not self.is_syllable_count_valid(tl_syllables):
                self.stats['syllable_count_exceeded'] += 1
                self.skipped_items.append({
                    'tl': tl_variant,
                    'hanzi': hanji_variant,
                    'reason': f'音節數超過3 ({tl_syllables}音節)'
                })
                continue

            # 檢查音節數與漢字數量是否一致
            if hanzi_count > 0 and tl_syllables != hanzi_count:
                self.stats['syllable_hanzi_mismatch_skipped'] += 1
                self.skipped_items.append({
                    'tl': tl_variant,
                    'hanzi': hanji_variant,
                    'reason': f'音節數與漢字數不符 (TL:{tl_syllables}音節, 漢字:{hanzi_count}字)'
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
                    'hanzi': hanji_variant,
                    'reason': ' '.join(reason)
                })
                continue

            # 建立結果（羅馬字轉為小寫）
            result = {
                'tl': tl_variant.lower(),
                'poj': poj_variant.lower(),
                'hanzi': hanji_variant,
                'tl_no_tone': tl_unicode.lower(),
                'poj_no_tone': poj_unicode.lower(),
                'syllable_count': tl_syllables,
                'source': 'taijit'
            }

            results.append(result)
            self.stats['total_valid'] += 1

        return results

def main():
    """主程式：讀取台日大辭典 CSV 檔案，處理資料並輸出為 CSV 格式"""
    # 設定 logging 寫入檔案
    logging.basicConfig(
        level=logging.INFO,
        format='%(message)s',
        handlers=[
            logging.FileHandler('log/taijit.log', mode='w', encoding='utf-8')
        ]
    )
    logger = logging.getLogger(__name__)

    processor = TaijitProcessor()
    input_file = Path('raw/ChhoeTaigi_TaijitToaSutian.csv')
    output_file = Path('csv/taijit.csv')

    all_rows = []

    # 讀取 CSV
    with open(input_file, 'r', encoding='utf-8') as csvfile:
        reader = csv.DictReader(csvfile)

        for row in reader:
            processor.stats['total_processed'] += 1
            results = processor.process_row(row)
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

    # 輸出特殊編碼統計
    if processor.special_encoding_items:
        logger.info(f"\n=== 包含特殊編碼的記錄 (共 {len(processor.special_encoding_items)} 筆) ===")
        for item in processor.special_encoding_items:
            special_info = []
            if item['tl_special']:
                special_info.append(f"TL: {', '.join(item['tl_special'])}")
            if item['poj_special']:
                special_info.append(f"POJ: {', '.join(item['poj_special'])}")
            logger.info(f"TL: {item['tl']}, POJ: {item['poj']}, 漢字: {item['hanzi']}")
            logger.info(f"  特殊字符: {' | '.join(special_info)}")

    # 輸出被跳過的項目（按原因分類）
    if processor.skipped_items:
        # 按原因分組
        skipped_by_reason = {}
        for item in processor.skipped_items:
            reason = item['reason'].split(' (')[0]  # 取得原因的主要部分
            if reason not in skipped_by_reason:
                skipped_by_reason[reason] = []
            skipped_by_reason[reason].append(item)

        logger.info(f"=== 跳過的詞條 (共 {len(processor.skipped_items)} 筆) ===")

        # 按原因顯示
        for reason in sorted(skipped_by_reason.keys()):
            items = skipped_by_reason[reason]
            logger.info(f"\n--- {reason} ({len(items)} 筆) ---")
            for item in items:
                logger.info(f"{item['tl']} ({item['hanzi']}): {item['reason']}")

if __name__ == "__main__":
    main()
