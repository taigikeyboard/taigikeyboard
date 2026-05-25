#!/bin/bash
#
# 辭典建置主腳本 — 每次執行 = 完整建置 + deploy
#
# 執行順序（v3.5.9 D / C-0：加入 syllables.fst + TPS 三索引）：
#   1. merge_csv                - 合併各詞庫 CSV
#   2. create_dictionary_bin    - 建立 dictionary.bin (binary mmap)，並寫入共享 build_ts
#   3. create_fst               - 建立 dictionary.fst 前綴索引 (tl:/poj:/tps:/hanzi:)
#   4. create_syllables_fst     - 建立 syllables.fst 音節庫 (tl:/poj:/tps: tagged-single-FST)
#   5. create_association_bin   - 建立 association.bin (binary mmap)，沿用 build_ts
#   6. audit                    - 審計報告 + fail-fast invariant (12_stale_poj)
#   7. verify_known_keys        - fst fixture 驗證 (poj:chi2 等已知 key 命中數)
#   8. deploy                   - 複製到 Android/iOS 專案 (含 syllables.fst)
#
# output/ 由使用者手動清除；本腳本不提供 clean / deploy-only 子命令。

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"
OUTPUT_DIR="$SCRIPT_DIR/output"

# Build scripts import from `build.common` + `common.*`; run them as modules
# from dictionary/ so the package imports resolve without sys.path hacks.
export PYTHONPATH="$SCRIPT_DIR"
cd "$SCRIPT_DIR"

GREEN='\033[0;32m'
NC='\033[0m'

step() {
    echo ""
    echo -e "${GREEN}▶ $1${NC}"
    echo ""
}

step "Step 1/8: Merging dictionaries..."
python3 -m build.merge_csv

step "Step 2/8: Creating dictionary.bin..."
python3 -m build.create_dictionary_bin --verify

step "Step 3/8: Creating fst prefix index..."
python3 -m build.create_fst

step "Step 4/8: Creating syllables.fst (tl/poj/tps tagged-single-FST)..."
python3 -m build.create_syllables_fst

step "Step 5/8: Creating association.bin..."
python3 -m build.create_association_bin --verify

step "Step 6/8: Running audit report..."
python3 -m build.audit

step "Step 7/8: Verifying known fst keys..."
python3 -m tools.verify_known_keys

step "Step 8/8: Deploying to Android/iOS..."
bash "$BUILD_DIR/deploy.sh"

echo ""
echo -e "${GREEN}✓ Build and deploy complete!${NC}"
echo "  Output: $OUTPUT_DIR"
