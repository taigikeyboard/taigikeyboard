#!/bin/bash
# Run all data processing scripts

set -e  # Stop on error

cd "$(dirname "$0")"

echo "=== 齒盤補充辭典 ==="
echo ""

scripts=(
    "01_extract.py"
    "02_cleanup.py"
    "03_add_frequency.py"
    "04_add_poj.py"
    "05_add_numtone.py"
    "06_add_notone.py"
    "07_add_abbrev.py"
    "08_add_source.py"
    "09_add_variants.py"
)

for script in "${scripts[@]}"; do
    if [ -f "$script" ]; then
        echo ">>> Running $script"
        python3 "$script"
        echo ""
    fi
done

echo "=== Done ==="
