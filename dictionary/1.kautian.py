#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import csv
import logging
from pathlib import Path
from base_processor import BaseProcessor
from kesi import Ku

class KautianProcessor(BaseProcessor):
    def __init__(self):
        """初始化教典處理器，設定統計計數器與跳過項目清單"""
        super().__init__()
        self.stats['syllable_count_exceeded'] = 0
        self.skipped_items = []

    def convert_to_poj(self, roman: str) -> str:
        """將台羅拼音轉換為白話字（POJ）"""
        ku = Ku(roman)
        return ku.POJ().hanlo

    def extract_variant_mappings(self, df):
        """從異用字表中提取漢字與異用字的映射關係"""
        variant_mappings = {}
        for _, row in df.iterrows():
            original = str(row["漢字"]).strip()
            variant = str(row["異用字"]).strip()

            if original and variant and original != variant:
                if original not in variant_mappings:
                    variant_mappings[original] = []
                if variant not in variant_mappings[original]:
                    variant_mappings[original].append(variant)

        return variant_mappings

    def generate_variant_records(self, original_records, variant_mappings):
        """根據異用字映射表，為原始記錄生成異用字變體記錄"""
        variant_records = []

        for record in original_records:
            hanzi = record.get('hanzi', '')
            if not hanzi:
                continue

            modified_hanzi_list = self.generate_variant_text(hanzi, variant_mappings)

            for modified_hanzi in modified_hanzi_list:
                if modified_hanzi != hanzi:
                    variant_record = record.copy()
                    variant_record['hanzi'] = modified_hanzi
                    variant_record['is_variant'] = True
                    variant_records.append(variant_record)

        return variant_records

    def generate_variant_text(self, text, variant_mappings):
        """將文字中包含的原始詞替換為異用字，生成所有可能的變體組合"""
        results = [text]

        for original, variants in variant_mappings.items():
            if original in text:
                new_results = []
                for result in results:
                    for variant in variants:
                        new_results.append(result.replace(original, variant))
                results.extend(new_results)

        return list(set(results))

    def extract_pronunciation_mappings(self, df):
        """從語音差異表中提取漢字與各地發音變體的映射關係

        Returns:
            dict[str, dict]: {漢字: {原始TL: [變體TL列表]}}
        """
        pronunciation_mappings = {}
        region_columns = [
            '鹿港偏泉腔', '三峽偏泉腔', '臺北偏泉腔', '宜蘭偏漳腔',
            '臺南混合腔', '高雄混合腔', '金門偏泉腔', '馬公偏泉腔',
            '新竹偏泉腔', '臺中偏漳腔'
        ]

        for _, row in df.iterrows():
            hanzi = str(row.get('漢字', '')).strip()
            if not hanzi or hanzi == 'nan':
                continue

            all_variants = set()
            for col in region_columns:
                value = str(row.get(col, '')).strip()
                if value and value != 'nan':
                    for v in value.split(','):
                        v = v.strip()
                        if v:
                            all_variants.add(v)

            if all_variants:
                pronunciation_mappings[hanzi] = list(all_variants)

        return pronunciation_mappings

    def generate_pronunciation_variant_records(self, original_records, pronunciation_mappings):
        """根據語音差異映射表，為原始記錄生成發音變體記錄"""
        variant_records = []

        for record in original_records:
            hanzi = record.get('hanzi', '')
            tl = record.get('tl', '')

            if not hanzi or not tl:
                continue

            hanzi_chars = list(hanzi)
            tl_syllables = tl.split('-')

            if len(hanzi_chars) != len(tl_syllables):
                continue

            variant_tl_sets = [set() for _ in tl_syllables]

            for i, char in enumerate(hanzi_chars):
                if char in pronunciation_mappings:
                    for variant in pronunciation_mappings[char]:
                        variant_tl_sets[i].add(variant)

            if not any(variant_tl_sets):
                continue

            from itertools import product

            options_per_position = []
            for i, syllable in enumerate(tl_syllables):
                if variant_tl_sets[i]:
                    options_per_position.append(list(variant_tl_sets[i]))
                else:
                    options_per_position.append([syllable])

            for combo in product(*options_per_position):
                new_tl = '-'.join(combo)
                if new_tl != tl:
                    new_poj = self.convert_to_poj(new_tl)
                    tl_no_tone = self.remove_tones(new_tl)
                    poj_no_tone = self.remove_tones(new_poj)

                    variant_record = record.copy()
                    variant_record['tl'] = new_tl.lower()
                    variant_record['poj'] = new_poj.lower()
                    variant_record['tl_no_tone'] = tl_no_tone.lower()
                    variant_record['poj_no_tone'] = poj_no_tone.lower()
                    variant_record['is_variant'] = True
                    variant_records.append(variant_record)

        return variant_records

    def process_row(self, row):
        """處理單筆資料列，將台羅轉換為白話字並生成多種變體記錄"""
        results = []

        tl_roman = str(row.get('羅馬字', '')).strip()
        hanji = str(row.get('漢字', '')).strip()

        if tl_roman.lower() == 'nan':
            tl_roman = ''
        if hanji.lower() == 'nan':
            hanji = ''

        if not tl_roman and not hanji:
            return results

        hanji = self.clean_text(hanji)

        tl_variants = [v.strip() for v in tl_roman.split('/') if v.strip()]

        # 處理每個變體
        for tl_variant in tl_variants:
            # 清理文字
            tl_variant = self.clean_text(tl_variant)
            tl_variant = self.remove_prefix_hyphens(tl_variant)

            # 跳過空白變體（只有 tl_variant 是必要的）
            if not tl_variant:
                continue

            # 轉換 TL 為 POJ
            poj_variant = self.convert_to_poj(tl_variant)

            # 計算音節數
            tl_syllables = self.count_syllables(tl_variant)

            # 如果沒有漢字，使用羅馬字作為漢字
            if not hanji:
                hanji = tl_variant

            # 檢查音節數量限制（<=3）
            if not self.is_syllable_count_valid(tl_syllables):
                self.stats['syllable_count_exceeded'] += 1
                self.skipped_items.append({
                    'tl': tl_variant,
                    'hanzi': hanji,
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
                    'hanzi': hanji,
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
                'hanzi': hanji,
                'tl_no_tone': tl_unicode.lower(),
                'poj_no_tone': poj_unicode.lower(),
                'syllable_count': tl_syllables,
                'source': 'kautian',
                'is_variant': False
            }

            results.append(result)
            self.stats['total_valid'] += 1

        return results

def main():
    """主程式：讀取教典 ODS 檔案，處理資料並輸出為 CSV 格式"""
    # 設定 logging 寫入檔案
    logging.basicConfig(
        level=logging.INFO,
        format='%(message)s',
        handlers=[
            logging.FileHandler('log/kautian.log', mode='w', encoding='utf-8')
        ]
    )
    logger = logging.getLogger(__name__)

    processor = KautianProcessor()
    input_file = Path('raw/kautian.ods')
    output_file = Path('csv/kautian.csv')

    sheets_to_read = ["詞目", "又唸作", "合音唸作", "俗唸作", "詞彙比較", "名", "姓", "異用字", "語音差異"]
    all_rows = []
    variant_mappings = {}
    pronunciation_mappings = {}

    import pandas as pd

    with pd.ExcelFile(input_file, engine='odf') as excel_file:
        if "異用字" in excel_file.sheet_names:
            variant_df = pd.read_excel(excel_file, sheet_name="異用字")
            variant_mappings = processor.extract_variant_mappings(variant_df)

        if "語音差異" in excel_file.sheet_names:
            pronunciation_df = pd.read_excel(excel_file, sheet_name="語音差異")
            pronunciation_mappings = processor.extract_pronunciation_mappings(pronunciation_df)

        for sheet_name in sheets_to_read:
            if sheet_name in ["異用字", "語音差異"]:
                continue

            if sheet_name not in excel_file.sheet_names:
                continue

            df = pd.read_excel(excel_file, sheet_name=sheet_name)

            sheet_records = []
            for _, row in df.iterrows():
                processor.stats['total_processed'] += 1
                results = processor.process_row(row.to_dict())
                sheet_records.extend(results)


            all_rows.extend(sheet_records)

            if variant_mappings:
                variant_records = processor.generate_variant_records(sheet_records, variant_mappings)
                all_rows.extend(variant_records)

            if pronunciation_mappings:
                pronunciation_records = processor.generate_pronunciation_variant_records(sheet_records, pronunciation_mappings)
                all_rows.extend(pronunciation_records)

    # 去重
    unique_rows = processor.deduplicate(all_rows)

    # 寫入 CSV
    with open(output_file, 'w', newline='', encoding='utf-8') as csvfile:
        fieldnames = ['tl', 'poj', 'hanzi', 'tl_no_tone', 'poj_no_tone', 'syllable_count', 'source', 'is_variant']
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

        logger.info(f"=== 跳過的詞條 (共 {len(processor.skipped_items)} 筆) ===")

        # 按原因顯示
        for reason in sorted(skipped_by_reason.keys()):
            items = skipped_by_reason[reason]
            logger.info(f"\n--- {reason} ({len(items)} 筆) ---")
            for item in items:
                logger.info(f"{item['tl']} ({item['hanzi']}): {item['reason']}")

if __name__ == "__main__":
    main()