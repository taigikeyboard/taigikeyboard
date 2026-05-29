#!/bin/bash
#
# 辭典建置主腳本 — 每次執行 = 完整建置 + deploy
#
# 執行順序：
#   1. merge_csv                - 合併各詞庫 CSV (寫入 .build_stats.json drop 統計)
#   2. create_dictionary_bin    - 建立 dictionary.bin (binary mmap)，並寫入共享 build_ts
#   3. create_fst               - 建立 dictionary.fst 前綴索引 (tl:/poj:/tps:/hanzi:)
#   4. create_syllables_fst     - 建立 syllables.fst 音節庫 (tl:/poj:/tps: tagged-single-FST)
#   5. create_association_bin   - 建立 association.bin (binary mmap)，沿用 build_ts
#   6. verify_poj_integrity     - fail-fast POJ invariant gate (前 audit 12/13 stale_poj)
#   7. version_snapshot         - drop 摘要 + vs 上一版 diff；release 模式寫入版本快照
#   8. verify_known_keys        - fst fixture 驗證 (poj:chi2 等已知 key 命中數)
#   9. deploy                   - 複製到 Android/iOS 專案 (含 syllables.fst)
#
# Release 版本標籤：設 RELEASE_VERSION=vX.Y.Z（或傳第一個參數）→ step 7 寫入
#   snapshots/vX.Y.Z.tsv 並修剪到最新 3 版。未設定 → step 7 僅報告 diff，不寫快照。
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

# Release version label: env RELEASE_VERSION or first positional arg.
RELEASE_VERSION="${RELEASE_VERSION:-${1:-}}"

step() {
    echo ""
    echo -e "${GREEN}▶ $1${NC}"
    echo ""
}

step "Step 1/9: Merging dictionaries..."
python3 -m build.merge_csv

step "Step 2/9: Creating dictionary.bin..."
python3 -m build.create_dictionary_bin --verify

step "Step 3/9: Creating fst prefix index..."
python3 -m build.create_fst

step "Step 4/9: Creating syllables.fst (tl/poj/tps tagged-single-FST)..."
python3 -m build.create_syllables_fst

step "Step 5/9: Creating association.bin..."
python3 -m build.create_association_bin --verify

step "Step 6/9: Verifying POJ integrity (fatal gate)..."
python3 -m build.verify_poj_integrity

step "Step 7/9: Version snapshot + drop/diff summary..."
if [ -n "$RELEASE_VERSION" ]; then
    python3 -m build.version_snapshot --version "$RELEASE_VERSION"
else
    python3 -m build.version_snapshot
fi

step "Step 8/9: Verifying known fst keys..."
python3 -m tools.verify_known_keys

step "Step 9/9: Deploying to Android/iOS..."
bash "$BUILD_DIR/deploy.sh"

echo ""
echo -e "${GREEN}✓ Build and deploy complete!${NC}"
echo "  Output: $OUTPUT_DIR"
