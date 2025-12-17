#!/usr/bin/env python3
"""
針對 異用字_羅馬字.csv 新增 TL 欄位。
如果漢字在 詞目.csv 中有 1:1 對應，則填入 TL 值，否則留空。
"""

import csv
import sys
from collections import defaultdict
from pathlib import Path

# 加入 common 路徑以引用 cleanup 模組
script_dir = Path(__file__).parent
sys.path.insert(0, str(script_dir.parent / "common"))

from cleanup import clean_brackets


def main():
    # 檔案路徑
    tsubo_path = script_dir.parent / "1_教育部臺灣台語常用詞辭典" / "data" / "02_extracted" / "詞目.csv"
    input_path = script_dir / "data" / "1_異用字.csv"
    output_path = script_dir / "data" / "2_異用字_羅馬字.csv"

    # 讀取 詞目.csv，建立 hanzi -> [tl] 的對應表
    # 注意：同一個詞的腔調變體（如 khioh-hīn/khioh-hūn）視為一筆資料
    hanzi_to_tl = defaultdict(set)

    print(f"讀取 {tsubo_path}...")
    with open(tsubo_path, "r", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            # 清理漢字欄位的括號標註（如【替】【白】【文】）
            hanzi = clean_brackets(row["漢字"])
            tl = clean_brackets(row["羅馬字"])
            if hanzi and tl:
                # 保留完整羅馬字（含 / 分隔的腔調變體）
                hanzi_to_tl[hanzi].add(tl)

    print(f"共讀取 {len(hanzi_to_tl)} 個不重複漢字")

    # 讀取 異用字_羅馬字.csv
    print(f"讀取 {input_path}...")
    rows = []
    with open(input_path, "r", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        fieldnames = reader.fieldnames
        for row in reader:
            rows.append(row)

    print(f"共讀取 {len(rows)} 筆資料")

    # 新增 TL 欄位
    if "TL" not in fieldnames:
        fieldnames = list(fieldnames) + ["TL"]

    matched_count = 0
    for row in rows:
        hanzi = row["漢字"]
        tl_set = hanzi_to_tl.get(hanzi, set())

        # 只有 1:1 對應時才填入
        if len(tl_set) == 1:
            row["TL"] = next(iter(tl_set))
            matched_count += 1
        else:
            row["TL"] = ""

    print(f"1:1 對應成功: {matched_count} 筆")

    # 寫回檔案
    print(f"寫入 {output_path}...")
    with open(output_path, "w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    print("完成!")


if __name__ == "__main__":
    main()
