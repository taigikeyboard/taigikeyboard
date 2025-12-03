#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import re
from abc import ABC, abstractmethod

class BaseProcessor(ABC):
    """基礎處理器類別，包含共用邏輯"""

    def __init__(self):
        # 統計計數器
        self.stats = {
            'multiple_hyphens_skipped': 0,
            'syllable_hanzi_mismatch_skipped': 0,
            'total_processed': 0,
            'total_valid': 0,
            'duplicates_removed': 0
        }

        self.initialize_mappings()
        self.initialize_patterns()

    def initialize_mappings(self):
        """初始化所有字元映射表"""
        self.poj_tone_mapping = {
            # a
            'á': 'a', 'à': 'a', 'â': 'a', 'ǎ': 'a', 'ā': 'a', 'a̍': 'a', 'ă': 'a',
            'Á': 'A', 'À': 'A', 'Â': 'A', 'Ǎ': 'A', 'Ā': 'A', 'A̍': 'A', 'Ă': 'A',
            # e
            'é': 'e', 'è': 'e', 'ê': 'e', 'ě': 'e', 'ē': 'e', 'e̍': 'e', 'ĕ': 'e',
            'É': 'E', 'È': 'E', 'Ê': 'E', 'Ě': 'E', 'Ē': 'E', 'E̍': 'E', 'Ĕ': 'E',
            # i
            'í': 'i', 'ì': 'i', 'î': 'i', 'ǐ': 'i', 'ī': 'i', 'i̍': 'i', 'ĭ': 'i',
            'Í': 'I', 'Ì': 'I', 'Î': 'I', 'Ǐ': 'I', 'Ī': 'I', 'I̍': 'I', 'Ĭ': 'I',
            # o
            'ó': 'o', 'ò': 'o', 'ô': 'o', 'ǒ': 'o', 'ō': 'o', 'o̍': 'o', 'ŏ': 'o',
            'Ó': 'O', 'Ò': 'O', 'Ô': 'O', 'Ǒ': 'O', 'Ō': 'O', 'O̍': 'O', 'Ŏ': 'O',
            # o͘ (o with dot above) - 只移除聲調，保留 o͘
            'ó͘': 'o͘', 'ò͘': 'o͘', 'ô͘': 'o͘', 'ǒ͘': 'o͘', 'ō͘': 'o͘', 'o̍͘': 'o͘', 'ŏ͘': 'o͘',
            'Ó͘': 'O͘', 'Ò͘': 'O͘', 'Ô͘': 'O͘', 'Ǒ͘': 'O͘', 'Ō͘': 'O͘', 'O̍͘': 'O͘', 'Ŏ͘': 'O͘',
            # u
            'ú': 'u', 'ù': 'u', 'û': 'u', 'ǔ': 'u', 'ū': 'u', 'u̍': 'u', 'ŭ': 'u',
            'Ú': 'U', 'Ù': 'U', 'Û': 'U', 'Ǔ': 'U', 'Ū': 'U', 'U̍': 'U', 'Ŭ': 'U',
            # n
            'ń': 'n', 'ǹ': 'n', 'n̂': 'n', 'ň': 'n', 'n̄': 'n', 'n̍': 'n', 'n̋': 'n',
            'Ń': 'N', 'Ǹ': 'N', 'N̂': 'N', 'Ň': 'N', 'N̄': 'N', 'N̍': 'N', 'N̋': 'N',
            # m
            'ḿ': 'm', 'm̀': 'm', 'm̂': 'm', 'm̌': 'm', 'm̄': 'm', 'm̍': 'm', 'm̋': 'm',
            'Ḿ': 'M', 'M̀': 'M', 'M̂': 'M', 'M̌': 'M', 'M̄': 'M', 'M̍': 'M', 'M̋': 'M',
            # ṳ (特殊字符：u with dot below)
            'ṳ́': 'u', 'ṳ̀': 'u', 'ṳ̂': 'u', 'ṳ̌': 'u', 'ṳ̄': 'u', 'ṳ̍': 'u', 'ṳ̋': 'u', 'ṳ': 'u',
            'Ṳ́': 'U', 'Ṳ̀': 'U', 'Ṳ̂': 'U', 'Ṳ̌': 'U', 'Ṳ̄': 'U', 'Ṳ̍': 'U', 'Ṳ̋': 'U', 'Ṳ': 'U',
        }

        # TL 模式保留特殊字元，不進行轉換
        self.tl_tone_mapping = dict(self.poj_tone_mapping)

        # 需要保留的特殊字元（不進行轉換）
        self.tl_preserved_chars = {'ⁿ', 'o͘', 'O͘'}

        # 從 TL 映射中移除需要保留的字元
        for char in self.tl_preserved_chars:
            if char in self.tl_tone_mapping:
                del self.tl_tone_mapping[char]

        # 允許的字符集合
        self.allowed_tl_chars = set('abcdefghijklmnopqrstuvwxyz-áàâǎāăéèêěēĕíìîǐīĭóòôǒōŏőúùûǔūŭűḿńǹ')
        self.allowed_tl_chars.update('ABCDEFGHIJKLMNOPQRSTUVWXYZ')
        self.allowed_tl_chars.update('ÁÀÂǍĀĂÉÈÊĚĒĔÍÌÎǏĪĬÓÒÔǑŌŎŐÚÙÛǓŪŬŰŰḾŃǸ')
        self.allowed_tl_chars.update('m̀m̂m̌m̄m̍m̋M̀M̂M̌M̄M̍M̋n̂ňn̄n̍n̋N̂Ňn̄n̍n̋')
        self.allowed_tl_chars.update('a̍e̍i̍o̍u̍A̍E̍I̍O̍U̍́͘ⁿ')

        self.allowed_poj_chars = set(self.allowed_tl_chars)
        self.allowed_poj_chars.update('ⁿⁿ͘o͘O͘ó͘ò͘ô͘ǒ͘ō͘o̍͘ŏ͘Ó͘Ò͘Ô͘Ǒ͘Ō͘O̍͘Ŏ̤͘ṳṲ')

    def initialize_patterns(self):
        """初始化正規表達式模式"""
        self.bracket_pattern = re.compile(r'[（(〈《「『【].*?[）)〉》」』】]')

    @abstractmethod
    def process_row(self, row):
        """處理單一資料列的抽象方法，子類別必須實作"""
        pass

    def split_variants(self, poj_text, tl_text, hanzi_text):
        """分割變體，處理斜線分隔的多個形式"""
        # 分割各欄位
        poj_variants = [v.strip() for v in poj_text.split('/')] if '/' in poj_text else [poj_text]
        tl_variants = [v.strip() for v in tl_text.split('/')] if '/' in tl_text else [tl_text]
        hanzi_variants = [v.strip() for v in hanzi_text.split('/')] if '/' in hanzi_text else [hanzi_text]

        # 確保數量一致
        max_variants = max(len(poj_variants), len(tl_variants), len(hanzi_variants))

        # 如果只有一個值，擴展到相同數量
        if len(poj_variants) == 1 and max_variants > 1:
            poj_variants = poj_variants * max_variants
        if len(tl_variants) == 1 and max_variants > 1:
            tl_variants = tl_variants * max_variants
        if len(hanzi_variants) == 1 and max_variants > 1:
            hanzi_variants = hanzi_variants * max_variants

        # 配對並返回
        return list(zip(poj_variants, tl_variants, hanzi_variants))

    def validate_character(self, text, allowed_chars):
        """驗證字符是否在允許的字符集合中（空白字符合法，但連續空白不合法）"""
        # 檢查連續空白
        if '  ' in text:
            return False, '  '

        # 驗證每個字符（空白視為合法）
        for char in text:
            if char != ' ' and char not in allowed_chars:
                return False, char
        return True, None

    def clean_text(self, text):
        """清理文字，移除所有括號內容"""
        text = self.bracket_pattern.sub('', text)
        return text.strip()

    def remove_prefix_hyphens(self, text):
        """移除前綴的 -- 符號"""
        if text.startswith('--'):
            return text[2:]
        return text

    def has_multiple_hyphens(self, text):
        """檢查是否有多個連字符（排除 --）"""
        hyphen_count = 0
        i = 0
        while i < len(text):
            if i < len(text) - 1 and text[i:i+2] == '--':
                i += 2
                continue
            if text[i] == '-':
                hyphen_count += 1
                if hyphen_count > 1:
                    return True
            i += 1
        return False

    def count_syllables(self, roman):
        """計算音節數量（支援空格和連字符作為音節分隔符）"""
        # 統一將空格替換為連字符
        roman = roman.replace(' ', '-')

        if '--' in roman:
            parts = roman.split('--')
            return sum(len(part.split('-')) for part in parts if part)
        else:
            return len(roman.split('-'))

    def is_syllable_count_valid(self, syllable_count):
        """檢查音節數量是否有效（<=3）"""
        return syllable_count <= 3

    def count_hanzi_characters(self, hanzi):
        """計算漢字字符數量"""
        count = 0
        for char in hanzi:
            # CJK 基本區 (U+4E00-U+9FFF)
            # CJK 擴展 A (U+3400-U+4DBF)
            # CJK 擴展 B (U+20000-U+2A6DF)
            # CJK 擴展 C (U+2A700-U+2B73F)
            # CJK 擴展 D (U+2B740-U+2B81F)
            # CJK 擴展 E (U+2B820-U+2CEAF)
            # CJK 擴展 F (U+2CEB0-U+2EBEF)
            # CJK 擴展 G (U+30000-U+3134F)
            # CJK 相容區 (U+F900-U+FAFF)
            if ('\u4e00' <= char <= '\u9fff' or  # 基本區
                '\u3400' <= char <= '\u4dbf' or  # 擴展 A
                '\U00020000' <= char <= '\U0002a6df' or  # 擴展 B
                '\U0002a700' <= char <= '\U0002b73f' or  # 擴展 C
                '\U0002b740' <= char <= '\U0002b81f' or  # 擴展 D
                '\U0002b820' <= char <= '\U0002ceaf' or  # 擴展 E
                '\U0002ceb0' <= char <= '\U0002ebef' or  # 擴展 F
                '\U00030000' <= char <= '\U0003134f' or  # 擴展 G
                '\uf900' <= char <= '\ufaff'):  # 相容區
                count += 1
        return count

    def remove_tones(self, text):
        """移除聲調"""
        if not text:
            return text

        # 先處理組合字符 (Unicode combining characters)
        import unicodedata
        # NFD 分解，然後過濾掉聲調組合標記
        text = unicodedata.normalize('NFD', text)

        # 需要保留的組合字符（非聲調符號）
        preserved_combining_chars = {
            '\u0358',  # ͘ COMBINING DOT ABOVE RIGHT (o͘ 的點)
        }

        # 只移除聲調相關的組合重音符號，保留特殊組合字符
        filtered_chars = []
        for c in text:
            if 0x0300 <= ord(c) <= 0x036F:  # 組合字符範圍
                if c in preserved_combining_chars:
                    filtered_chars.append(c)  # 保留特殊組合字符
                # else: 移除聲調組合字符
            else:
                filtered_chars.append(c)  # 保留非組合字符

        text = ''.join(filtered_chars)

        # 再處理預組合字符
        # 創建合併的映射表，按長度排序（長的優先處理）
        all_mappings = {}
        all_mappings.update(self.poj_tone_mapping)
        all_mappings.update(self.tl_tone_mapping)

        # 按鍵長度降序排列，確保長模式先匹配
        sorted_mappings = sorted(all_mappings.items(), key=lambda x: len(x[0]), reverse=True)

        for toned, untoned in sorted_mappings:
            text = text.replace(toned, untoned)
        return text

    def extract_first_letters(self, syllable_string):
        """
        提取音節首字母（最後音節保留完整）

        Args:
            syllable_string: 以 - 分隔的音節字串

        Returns:
            首字母組合 + 完整最後音節，若少於兩個音節則回傳 None

        範例:
            "abc-def" → "adef"
            "gua-cha-goa" → "gcgoa"
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

        # 前面音節取首字母，最後音節保留完整
        result_parts = [s[0] for s in syllables[:-1]]
        result_parts.append(syllables[-1])

        # 組合回傳
        return ''.join(result_parts)

    def convert_nn_to_nasal(self, text):
        """將 n-n 轉換為 ⁿ（處理大小寫）"""
        # 處理小寫 n-n
        text = re.sub(r'n-n', 'ⁿ', text)
        # 處理大寫 N-N
        text = re.sub(r'N-N', 'ⁿ', text)
        # 處理混合大小寫 N-n 或 n-N
        text = re.sub(r'[Nn]-[Nn]', 'ⁿ', text)
        return text

    def deduplicate(self, rows):
        """去除重複的資料"""
        seen = set()
        unique_rows = []

        for row in rows:
            # 使用 (tl, poj, hanzi, tl_no_tone, poj_no_tone) 作為唯一鍵
            # 包含 no_tone 欄位以支援 n-n 轉 ⁿ 的變體
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
                self.stats['duplicates_removed'] += 1

        return unique_rows