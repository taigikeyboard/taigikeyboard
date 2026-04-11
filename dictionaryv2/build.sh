#!/bin/bash
#
# 辭典建置主腳本
#
# 執行順序：
#   1. merge_csv            - 合併各詞庫 CSV
#   2. create_app_db        - 建立 App 使用的 SQLite 資料庫
#   3. generate_association - 產生 NextWord 詞彙關聯（加入 dictionary.db）
#   4. create_trie_db       - 建立 Trie 建置用的 SQLite 資料庫
#   5. create_trie          - 建立 MARISA-trie
#   6. audit                - 產生審計報告
#   7. split_packages       - 分拆為各辭典獨立 package
#   8. deploy               - 複製到 Android/iOS 專案
#
# 用法：
#   ./build.sh          # 完整建置（不含 deploy）
#   ./build.sh all      # 完整建置 + deploy
#   ./build.sh deploy   # 只執行 deploy
#   ./build.sh publish  # 發布 packages 到 GitHub Release
#   ./build.sh clean    # 清除 output/

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"
OUTPUT_DIR="$SCRIPT_DIR/output"

# 顏色
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

print_step() {
    echo ""
    echo -e "${GREEN}▶ $1${NC}"
    echo ""
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# 清除 output/
do_clean() {
    print_step "Cleaning output directory..."
    rm -rf "$OUTPUT_DIR"/*
    print_success "Cleaned: $OUTPUT_DIR"
}

# Step 1: Merge CSV
do_merge_csv() {
    print_step "Step 1/11: Merging dictionaries..."
    python3 "$BUILD_DIR/01_merge_csv.py"
}

# Step 2: Create App DB
do_create_app_db() {
    print_step "Step 2/11: Creating App SQLite database..."
    bash "$BUILD_DIR/02_create_app_db.sh"
}

# Step 3: Generate Association
do_generate_association() {
    print_step "Step 3/11: Generating NextWord associations..."
    python3 "$BUILD_DIR/05_generate_association.py"
}

# Step 4: Create Trie DB
do_create_trie_db() {
    print_step "Step 4/11: Creating Trie SQLite database..."
    bash "$BUILD_DIR/03_create_trie_db.sh"
}

# Step 5: Create Trie
do_create_trie() {
    print_step "Step 5/11: Creating MARISA-trie..."
    python3 "$BUILD_DIR/04_create_trie.py"
}

# Step 6: Create dictionary.bin
do_create_dictionary_bin() {
    print_step "Step 6/11: Creating dictionary.bin..."
    python3 "$BUILD_DIR/10_create_dictionary_bin.py" --verify
}

# Step 7: Create association.bin
do_create_association_bin() {
    print_step "Step 7/11: Creating association.bin..."
    python3 "$BUILD_DIR/11_create_association_bin.py" --verify
}

# Step 8: Create hanzi.trie
do_create_hanzi_trie() {
    print_step "Step 8/11: Creating hanzi.trie..."
    python3 "$BUILD_DIR/12_create_hanzi_trie.py" --verify
}

# Step 9: Audit
do_audit() {
    print_step "Step 9/11: Running audit report..."
    python3 "$BUILD_DIR/07_audit.py"
}

# Step 10: Split Packages
do_split_packages() {
    print_step "Step 10/11: Splitting into per-dictionary packages..."
    python3 "$BUILD_DIR/08_split_packages.py"
}

# Step 11: Deploy
do_deploy() {
    print_step "Step 11/11: Deploying to Android/iOS..."
    bash "$BUILD_DIR/06_deploy.sh"
}

# Publish packages to GitHub Release
do_publish() {
    bash "$BUILD_DIR/09_publish_release.sh"
}

# 完整建置（不含 deploy）
do_build() {
    do_merge_csv
    do_create_app_db
    do_generate_association
    do_create_trie_db
    do_create_trie
    do_create_dictionary_bin
    do_create_association_bin
    do_create_hanzi_trie
    do_audit
    do_split_packages
}

# 顯示用法
show_usage() {
    echo "用法: ./build.sh [command]"
    echo ""
    echo "Commands:"
    echo "  (none)    完整建置（不含 deploy）"
    echo "  all       完整建置 + deploy"
    echo "  deploy    只執行 deploy"
    echo "  publish   發布 packages 到 GitHub Release"
    echo "  clean     清除 output/"
    echo ""
}

# Main
case "${1:-}" in
    "")
        do_build
        echo ""
        print_success "Build complete!"
        echo "  Output: $OUTPUT_DIR"
        echo ""
        echo "  Run './build.sh deploy' to copy to Android."
        ;;
    "all")
        do_build
        do_deploy
        echo ""
        print_success "Build and deploy complete!"
        ;;
    "deploy")
        do_deploy
        ;;
    "publish")
        do_publish
        ;;
    "clean")
        do_clean
        ;;
    "help"|"-h"|"--help")
        show_usage
        ;;
    *)
        print_error "Unknown command: $1"
        show_usage
        exit 1
        ;;
esac
