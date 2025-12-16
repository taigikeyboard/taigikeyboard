#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
從 dictionary.csv 產生 Android InputNormalizerTest.kt

輸入：output/dictionary.csv
輸出：android/app/src/test/.../InputNormalizerTest.kt

測試邏輯：
- tl（帶調符）-> tl_num
- poj（帶調符）-> poj_num
"""

import os
import random
import pandas as pd

SAMPLE_SIZE = 300

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
BASE_DIR = os.path.dirname(SCRIPT_DIR)
INPUT_FILE = os.path.join(BASE_DIR, "output", "dictionary.csv")
OUTPUT_FILE = os.path.join(
    BASE_DIR,
    "..",
    "android",
    "app",
    "src",
    "test",
    "java",
    "com",
    "siansiansu",
    "taigikeyboard",
    "ime",
    "dictionary",
    "InputNormalizerTest.kt",
)


def escape_kotlin_string(s: str) -> str:
    """轉義 Kotlin 字串中的特殊字元"""
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
        tl = escape_kotlin_string(row["tl"])
        poj = escape_kotlin_string(row["poj"])
        tl_num = escape_kotlin_string(row["tl_num"])
        poj_num = escape_kotlin_string(row["poj_num"])

        # tl -> tl_num
        if tl and tl_num:
            tl_cases.append(f'            "{tl}" to "{tl_num}",')

        # poj -> poj_num
        if poj and poj_num:
            poj_cases.append(f'            "{poj}" to "{poj_num}",')

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

    kotlin_code = f'''package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * InputNormalizer 測試
 * 自動產生自 dictionary.csv（{total_cases} 筆測試案例）
 */
class InputNormalizerTest {{

    // tl -> tl_num
    private val tlCases = listOf(
{chr(10).join(tl_cases)}
    )

    // poj -> poj_num
    private val pojCases = listOf(
{chr(10).join(poj_cases)}
    )

    @Test
    fun testTlToTlNum() {{
        tlCases.forEach {{ (input, expected) ->
            assertEquals(
                "normalize(\\"$input\\", TL) should be \\"$expected\\"",
                expected,
                InputNormalizer.normalize(input, InputMode.TL)
            )
        }}
    }}

    @Test
    fun testPojToPojNum() {{
        pojCases.forEach {{ (input, expected) ->
            assertEquals(
                "normalize(\\"$input\\", POJ) should be \\"$expected\\"",
                expected,
                InputNormalizer.normalize(input, InputMode.POJ)
            )
        }}
    }}
}}
'''

    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        f.write(kotlin_code)

    print(f"\nSaved: {OUTPUT_FILE}")


if __name__ == "__main__":
    main()
