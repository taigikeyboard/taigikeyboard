#!/bin/bash
#
# 辭典建置主腳本 — 每次執行 = 完整建置 + deploy
#
# 執行順序：
#   1. merge_csv                - 合併各詞庫 CSV
#   2. create_app_db            - 建立 App 使用的 SQLite 資料庫
#   3. generate_association     - 產生 NextWord 詞彙關聯（加入 dictionary.db）
#   4. create_trie_db           - 建立 fst 建置用的 SQLite 資料庫
#   5. create_fst               - 建立 fst 前綴索引
#   6. create_dictionary_bin    - 建立 dictionary.bin (binary mmap)
#   7. create_association_bin   - 建立 association.bin (binary mmap)
#   8. audit                    - 產生審計報告
#   9. deploy                   - 複製到 Android/iOS 專案
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

step "Step 1/9: Merging dictionaries..."
python3 -m build.merge_csv

step "Step 2/9: Creating App SQLite database..."
bash "$BUILD_DIR/create_app_db.sh"

step "Step 3/9: Generating NextWord associations..."
python3 -m build.generate_association

step "Step 4/9: Creating fst-build SQLite database..."
bash "$BUILD_DIR/create_trie_db.sh"

step "Step 5/9: Creating fst prefix index..."
python3 -m build.create_fst

step "Step 6/9: Creating dictionary.bin..."
python3 -m build.create_dictionary_bin --verify

step "Step 7/9: Creating association.bin..."
python3 -m build.create_association_bin --verify

step "Step 8/9: Running audit report..."
python3 -m build.audit

step "Step 9/9: Deploying to Android/iOS..."
bash "$BUILD_DIR/deploy.sh"

echo ""
echo -e "${GREEN}✓ Build and deploy complete!${NC}"
echo "  Output: $OUTPUT_DIR"
