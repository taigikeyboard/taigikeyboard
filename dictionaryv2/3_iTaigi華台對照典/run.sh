#!/bin/bash
# 執行所有資料處理腳本

set -e  # 遇到錯誤立即停止

cd "$(dirname "$0")"

echo "=== iTaigi 華台對照典 ==="
echo ""

scripts=(
    "01_select.py"
    "02_expand.py"
    "03_cleanup.py"
    "04_add_frequency.py"
    "05_add_poj.py"
    "06_add_numtone.py"
    "07_add_notone.py"
    "08_add_abbrev.py"
    "09_add_source.py"
    "10_add_variants.py"
)

for script in "${scripts[@]}"; do
    if [ -f "$script" ]; then
        echo ">>> Running $script"
        python3 "$script"
        echo ""
    fi
done

echo "=== Done ==="
