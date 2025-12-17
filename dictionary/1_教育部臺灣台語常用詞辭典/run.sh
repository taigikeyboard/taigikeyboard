#!/bin/bash
# 執行所有資料處理腳本

set -e  # 遇到錯誤立即停止

cd "$(dirname "$0")"

echo "=== 教育部臺灣台語常用詞辭典 ==="
echo ""

scripts=(
    "01_extract.py"
    "02_select.py"
    "03_expand.py"
    "04_merge.py"
    "05_cleanup.py"
    "06_add_frequency.py"
    "07_add_poj.py"
    "08_add_numtone.py"
    "09_add_notone.py"
    "10_add_abbrev.py"
    "11_add_source.py"
    "12_add_variants.py"
)

for script in "${scripts[@]}"; do
    if [ -f "$script" ]; then
        echo ">>> Running $script"
        python3 "$script"
        echo ""
    fi
done

echo "=== Done ==="
