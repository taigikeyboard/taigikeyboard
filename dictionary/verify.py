#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
驗證台語詞典 CSV 檔案

檢查項目：
- 字符有效性（TL/POJ 允許字符）
- 空格問題
- 標點符號問題
- 重複記錄
"""

import csv
import re
import sys
import logging
from collections import defaultdict, Counter

class DictionaryVerifier:
    def __init__(self):
        # 定義允許的字符集
        self.allowed_tl_chars = set('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-'
                                   'áàâǎāa̍a̋éèêěēe̍e̋íìîǐīi̍i̋óòôǒōo̍őúùûǔūu̍űṳṳ́ṳ̀ṳ̂ṳ̌ṳ̄ṳ̍ṳ̋'
                                   'ńǹn̂ňn̄n̍n̋ḿm̀m̂m̌m̄m̍m̋ⁿ'
                                   'ÁÀÂǍĀA̍A̋ÉÈÊĚĒE̍E̋ÍÌÎǏĪI̍I̋ÓÒÔǑŌO̍ŐÚÙÛǓŪU̍ŰṲṲ́Ṳ̀Ṳ̂Ṳ̌Ṳ̄Ṳ̍Ṳ̋'
                                   'ŃǸN̂ŇN̄N̍N̋ḾM̀M̂M̌M̄M̍M̋')

        self.allowed_poj_chars = set('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-'
                                    'áàâǎāa̍ăéèêěēe̍ĕíìîǐīi̍ĭóòôǒōo̍ŏúùûǔūu̍ŭṳṳ́ṳ̀ṳ̂ṳ̌ṳ̄ṳ̍ṳ̋'
                                    'ńǹn̂ňn̄n̍n̋ḿm̀m̂m̌m̄m̍m̋ó͘ò͘ô͘ǒ͘ō͘o̍͘ŏ͘ⁿ'
                                    'ÁÀÂǍĀA̍ĂÉÈÊĚĒE̍ĔÍÌÎǏĪI̍ĬÓÒÔǑŌO̍ŎÚÙÛǓŪU̍ŬṲṲ́Ṳ̀Ṳ̂Ṳ̌Ṳ̄Ṳ̍Ṳ̋'
                                    'ŃǸN̂ŇN̄N̍N̋ḾM̀M̂M̌M̄M̍M̋Ó͘Ò͘Ô͘Ǒ͘Ō͘O̍͘Ŏ͘')

        # 定義需要檢查的問題符號
        self.problematic_chars = {
            '【': '中文方括號',
            '】': '中文方括號',
            '（': '中文圓括號',
            '）': '中文圓括號',
            '。': '中文句號',
            '，': '中文逗號',
            '；': '中文分號',
            '：': '中文冒號',
            '？': '中文問號',
            '！': '中文驚嘆號',
            '「': '中文引號',
            '」': '中文引號',
            '『': '中文引號',
            '』': '中文引號',
            '/': '斜線（應該已被分割）',
            ' ': '空格',
            '\t': 'Tab字符',
            '\n': '換行符',
            '\r': '回車符',
        }

        # ASCII 特殊符號
        ascii_specials = '()[]{}.,;:?!"\''
        for char in ascii_specials:
            self.problematic_chars[char] = f'ASCII特殊符號 ({char})'

    def check_character_validity(self, text, field_name, allowed_chars):
        """檢查字符是否在允許的字符集中"""
        issues = []
        for i, char in enumerate(text):
            if char not in allowed_chars:
                char_name = self.problematic_chars.get(char, f'未知字符 (U+{ord(char):04X})')
                issues.append({
                    'type': '非法字符',
                    'field': field_name,
                    'position': i,
                    'character': char,
                    'description': char_name
                })
        return issues

    def check_spacing_issues(self, text, field_name):
        """檢查空格和縮進問題"""
        issues = []

        # 檢查開頭或結尾的空白
        if text.startswith(' ') or text.startswith('\t'):
            issues.append({
                'type': '開頭空白',
                'field': field_name,
                'text': repr(text[:10])
            })

        if text.endswith(' ') or text.endswith('\t'):
            issues.append({
                'type': '結尾空白',
                'field': field_name,
                'text': repr(text[-10:])
            })

        # 檢查連續空格
        if '  ' in text:  # 兩個或以上空格
            issues.append({
                'type': '連續空格',
                'field': field_name,
                'text': text
            })

        return issues

    def check_punctuation_issues(self, text, field_name):
        """檢查標點符號問題"""
        issues = []

        for char, description in self.problematic_chars.items():
            # hanzi 欄位允許空格（漢羅混寫，如「gá-suh 爐」「厚 tshì-sìr」）
            if field_name == 'hanzi' and char == ' ':
                continue

            if char in text:
                positions = [i for i, c in enumerate(text) if c == char]
                issues.append({
                    'type': '問題符號',
                    'field': field_name,
                    'character': char,
                    'description': description,
                    'positions': positions,
                    'text': text
                })

        return issues

    def check_empty_or_missing(self, row, row_num):
        """檢查空白或缺失的欄位"""
        issues = []
        # tl_no_tone, poj_no_tone 可為空（某些情況下正常）
        required_fields = ['tl', 'poj']

        for field in required_fields:
            value = row.get(field, '').strip()
            if not value:
                issues.append({
                    'type': '空白欄位',
                    'field': field,
                    'row': row_num
                })

        return issues

    def check_consistency(self, row, row_num):
        """檢查資料一致性"""
        issues = []

        tl = row.get('tl', '').strip()
        poj = row.get('poj', '').strip()
        tl_no_tone = row.get('tl_no_tone', '').strip()
        poj_no_tone = row.get('poj_no_tone', '').strip()
        hanji = row.get('hanji', '').strip()

        # 檢查漢字與台羅音節數是否一致
        if hanji and tl:
            # 計算台羅音節數（以連字號分隔）
            tl_syllables = len([s for s in re.split(r'[-\s]+', tl) if s])

            # 計算漢字字數（過濾非中文字符）
            chinese_chars = re.findall(r'[\u4e00-\u9fff]', hanji)
            hanji_count = len(chinese_chars)

            if hanji_count > 0 and tl_syllables != hanji_count:
                issues.append({
                    'type': '漢字音節數不符',
                    'field': 'hanji_syllable_mismatch',
                    'hanji': hanji,
                    'tl': tl,
                    'hanji_count': hanji_count,
                    'tl_syllables': tl_syllables,
                    'row': row_num
                })

        return issues

    def verify_csv(self, csv_path):
        """驗證 CSV 檔案"""
        logger = logging.getLogger(__name__)
        logger.info(f"[INFO] 驗證檔案: {csv_path}")

        try:
            with open(csv_path, 'r', encoding='utf-8') as f:
                reader = csv.DictReader(f)

                total_rows = 0
                all_issues = []
                field_stats = defaultdict(Counter)
                seen_records = {}  # 用於檢查重複

                for row_num, row in enumerate(reader, start=2):  # 從第2行開始（第1行是標題）
                    total_rows += 1

                    # 檢查重複
                    key = (row.get('tl', '').strip(),
                           row.get('poj', '').strip(),
                           row.get('hanzi', '').strip())
                    if key in seen_records:
                        all_issues.append({
                            'type': '重複記錄',
                            'row': row_num,
                            'first_occurrence': seen_records[key],
                            'tl': key[0],
                            'poj': key[1],
                            'hanzi': key[2]
                        })
                    else:
                        seen_records[key] = row_num

                    # 基本檢查
                    issues = self.check_empty_or_missing(row, row_num)
                    all_issues.extend(issues)

                    # 一致性檢查
                    issues = self.check_consistency(row, row_num)
                    all_issues.extend(issues)

                    # 逐欄位檢查
                    for field, value in row.items():
                        if not value:
                            continue

                        value = value.strip()

                        # 字符有效性檢查
                        if field in ['tl', 'tl_no_tone']:
                            issues = self.check_character_validity(value, field, self.allowed_tl_chars)
                            all_issues.extend([{**issue, 'row': row_num} for issue in issues])
                        elif field in ['poj', 'poj_no_tone']:
                            issues = self.check_character_validity(value, field, self.allowed_poj_chars)
                            all_issues.extend([{**issue, 'row': row_num} for issue in issues])

                        # 空格問題檢查
                        issues = self.check_spacing_issues(value, field)
                        all_issues.extend([{**issue, 'row': row_num} for issue in issues])

                        # 標點符號檢查
                        issues = self.check_punctuation_issues(value, field)
                        all_issues.extend([{**issue, 'row': row_num} for issue in issues])

                        # 統計欄位字符使用情況
                        for char in value:
                            field_stats[field][char] += 1

                # 生成報告
                self.generate_report(total_rows, all_issues, field_stats, logger)

        except FileNotFoundError:
            logger.error(f"[ERROR] 檔案不存在: {csv_path}")
            return False
        except Exception as e:
            logger.error(f"[ERROR] 讀取檔案失敗: {e}")
            return False

        return len(all_issues) == 0

    def generate_report(self, total_rows, issues, field_stats, logger):
        """生成驗證報告"""
        logger.info(f"\n=== 驗證報告 ===")
        logger.info(f"總記錄數: {total_rows}")
        logger.info(f"發現問題數: {len(issues)}")

        if not issues:
            logger.info("✅ 所有檢查通過！")
            return

        # 按問題類型分組
        issues_by_type = defaultdict(list)
        for issue in issues:
            issues_by_type[issue['type']].append(issue)

        logger.info(f"\n=== 問題統計 ===")
        for issue_type, type_issues in issues_by_type.items():
            logger.info(f"{issue_type}: {len(type_issues)} 個")

        logger.info(f"\n=== 詳細問題 ===")
        for issue_type, type_issues in sorted(issues_by_type.items()):
            logger.info(f"\n--- {issue_type} ({len(type_issues)} 個) ---")

            # 顯示所有問題
            for i, issue in enumerate(type_issues):
                self.print_issue_detail(issue, logger)

        # 字符使用統計
        logger.info(f"\n=== 特殊字符使用統計 ===")
        for field, char_counts in field_stats.items():
            special_chars = {char: count for char, count in char_counts.items()
                           if char in self.problematic_chars}
            if special_chars:
                logger.info(f"{field}: {special_chars}")

    def print_issue_detail(self, issue, logger):
        """列印問題詳情"""
        row_info = f"第 {issue['row']} 行" if 'row' in issue else ""

        if issue['type'] == '非法字符':
            logger.info(f"  {row_info} {issue['field']} 欄位位置 {issue['position']}: "
                  f"'{issue['character']}' ({issue['description']})")

        elif issue['type'] == '問題符號':
            logger.info(f"  {row_info} {issue['field']} 欄位: "
                  f"含有 '{issue['character']}' ({issue['description']}) "
                  f"位置: {issue['positions']} 內容: {issue['text'][:50]}")

        elif issue['type'] in ['開頭空白', '結尾空白', '連續空格']:
            logger.info(f"  {row_info} {issue['field']} 欄位: {issue['type']} - {issue['text']}")

        elif issue['type'] == '空白欄位':
            logger.info(f"  {row_info} {issue['field']} 欄位為空")

        elif issue['type'] == '音節數不符':
            logger.info(f"  {row_info} 音節數不符: 預期 {issue['expected']}，實際 {issue['actual']} "
                  f"(tl: {issue['tl']})")

        elif issue['type'] == '漢字音節數不符':
            logger.info(f"  {row_info} 漢字音節數不符: 漢字 '{issue['hanji']}' ({issue['hanji_count']} 字) "
                  f"vs 台羅 '{issue['tl']}' ({issue['tl_syllables']} 音節)")

        elif issue['type'] == '重複記錄':
            logger.info(f"  {row_info} 重複記錄 (第一次出現在第 {issue['first_occurrence']} 行): "
                  f"TL='{issue['tl']}' POJ='{issue['poj']}' 漢字='{issue['hanzi']}'")

        else:
            logger.info(f"  {row_info} {issue}")

def main():
    import argparse

    parser = argparse.ArgumentParser(description='驗證台語詞典 CSV 檔案')
    parser.add_argument('file', nargs='?', default='dictionary.csv',
                       help='要驗證的 CSV 檔案（預設：dictionary.csv）')

    args = parser.parse_args()

    # 輸出到螢幕
    logging.basicConfig(
        level=logging.INFO,
        format='%(message)s',
    )

    verifier = DictionaryVerifier()
    success = verifier.verify_csv(args.file)

    if success:
        print(f"\n✅ 檔案驗證成功！")
        sys.exit(0)
    else:
        print(f"\n❌ 發現問題，請檢查上述報告")
        sys.exit(1)

if __name__ == "__main__":
    main()