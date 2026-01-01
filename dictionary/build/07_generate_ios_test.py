#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
從 dictionary.csv 產生 iOS InputNormalizerTests.swift

輸入：output/dictionary.csv
輸出：ios/TaigiKeyboardTests/InputNormalizerTests.swift

測試邏輯：
- tl（帶調符）-> tl_num
- poj（帶調符）-> poj_num
"""

import os
import random
import unicodedata
import pandas as pd

SAMPLE_SIZE = 300

# 聲調組合標記（用於判斷是否有調符）
TONE_MARKS = {
    '\u0301',  # ́ COMBINING ACUTE ACCENT (2)
    '\u0300',  # ̀ COMBINING GRAVE ACCENT (3)
    '\u0302',  # ̂ COMBINING CIRCUMFLEX ACCENT (5)
    '\u030C',  # ̌ COMBINING CARON (6)
    '\u0304',  # ̄ COMBINING MACRON (7)
    '\u030D',  # ̍ COMBINING VERTICAL LINE ABOVE (8)
    '\u0306',  # ̆ COMBINING BREVE (POJ 9)
    '\u030B',  # ̋ COMBINING DOUBLE ACUTE ACCENT (TL 9)
}


def has_tone_marks(s: str) -> bool:
    """檢查字串是否包含聲調標記"""
    if pd.isna(s):
        return False
    # NFD 分解後檢查是否有聲調組合標記
    nfd = unicodedata.normalize('NFD', str(s))
    return any(c in TONE_MARKS for c in nfd)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
BASE_DIR = os.path.dirname(SCRIPT_DIR)
INPUT_FILE = os.path.join(BASE_DIR, "output", "dictionary.csv")
OUTPUT_DIR = os.path.join(BASE_DIR, "..", "ios", "TaigiKeyboardTests")
OUTPUT_FILE = os.path.join(OUTPUT_DIR, "InputNormalizerTests.swift")


def escape_swift_string(s: str) -> str:
    """轉義 Swift 字串中的特殊字元"""
    if pd.isna(s):
        return ""
    return str(s).replace("\\", "\\\\").replace('"', '\\"')


def main():
    print(f"Input:  {INPUT_FILE}")
    print(f"Output: {OUTPUT_FILE}")

    df = pd.read_csv(INPUT_FILE)
    print(f"Loaded {len(df)} records")

    # 過濾有效資料
    df = df[df["tl"].notna() & df["tl_num"].notna()]
    df = df[df["tl"].str.strip() != ""]
    df = df[df["tl_num"].str.strip() != ""]
    print(f"Valid records: {len(df)}")

    # 產生測試程式碼（2 組）
    tl_cases = []   # tl（帶調符）-> tl_num
    poj_cases = []  # poj（帶調符）-> poj_num

    for _, row in df.iterrows():
        tl = escape_swift_string(row["tl"])
        poj = escape_swift_string(row["poj"])
        tl_num = escape_swift_string(row["tl_num"])
        poj_num = escape_swift_string(row["poj_num"])

        # tl -> tl_num（只測試有調符的輸入）
        if tl and tl_num and has_tone_marks(row["tl"]):
            tl_cases.append(f'            ("{tl}", "{tl_num}"),')

        # poj -> poj_num（只測試有調符的輸入）
        if poj and poj_num and has_tone_marks(row["poj"]):
            poj_cases.append(f'            ("{poj}", "{poj_num}"),')

    # 去重
    tl_cases = list(dict.fromkeys(tl_cases))
    poj_cases = list(dict.fromkeys(poj_cases))

    print(f"tl_cases: {len(tl_cases)}")
    print(f"poj_cases: {len(poj_cases)}")

    # 隨機取樣
    if len(tl_cases) > SAMPLE_SIZE:
        tl_cases = random.sample(tl_cases, SAMPLE_SIZE)
    if len(poj_cases) > SAMPLE_SIZE:
        poj_cases = random.sample(poj_cases, SAMPLE_SIZE)

    print(f"After sampling: tl={len(tl_cases)}, poj={len(poj_cases)}")

    total_cases = len(tl_cases) + len(poj_cases)

    swift_code = f'''import XCTest
@testable import TaigiKeyboard

/// InputNormalizer 測試
/// 自動產生自 dictionary.csv（{total_cases} 筆測試案例）
final class InputNormalizerTests: XCTestCase {{

    // MARK: - Test Data

    /// tl（帶調符）-> tl_num
    private let tlCases: [(input: String, expected: String)] = [
{chr(10).join(tl_cases)}
    ]

    /// poj（帶調符）-> poj_num
    private let pojCases: [(input: String, expected: String)] = [
{chr(10).join(poj_cases)}
    ]

    // MARK: - Tests

    func testTlToTlNum() {{
        for (input, expected) in tlCases {{
            let result = InputNormalizer.normalize(input, mode: .tl)
            XCTAssertEqual(
                result,
                expected,
                "normalize(\\"\\(input)\\", .tl) should be \\"\\(expected)\\", got \\"\\(result)\\""
            )
        }}
    }}

    func testPojToPojNum() {{
        for (input, expected) in pojCases {{
            let result = InputNormalizer.normalize(input, mode: .poj)
            XCTAssertEqual(
                result,
                expected,
                "normalize(\\"\\(input)\\", .poj) should be \\"\\(expected)\\", got \\"\\(result)\\""
            )
        }}
    }}
}}
'''

    # 確保輸出目錄存在
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        f.write(swift_code)

    print(f"\nSaved: {OUTPUT_FILE}")


if __name__ == "__main__":
    main()
