#!/bin/bash
# 依序處理所有辭典的 run_all.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

for dir in \
    "1_教育部臺灣台語常用詞辭典" \
    "2_台語新詞辭庫" \
    "3_iTaigi華台對照典" \
    "4_台灣植物名彙" \
    "5_台華線頂對照典" \
    "6_台日大辭典" \
    "7_台語工藝詞庫"
do
    echo "=== $dir ==="
    (cd "$dir" && ./run_all.sh)
done

echo "=== 全部完成 ==="
