#!/bin/bash
#
# 辭典建置主腳本
#
# 執行順序：
#   1. merge_csv       - 合併各詞庫 CSV
#   2. create_app_db   - 建立 App 使用的 SQLite 資料庫
#   3. create_trie_db  - 建立 Trie 建置用的 SQLite 資料庫
#   4. create_trie     - 建立 MARISA-trie
#   5. deploy          - 複製到 Android 專案
#
# 用法：
#   ./build.sh          # 完整建置（不含 deploy）
#   ./build.sh all      # 完整建置 + deploy
#   ./build.sh deploy   # 只執行 deploy
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
    print_step "Step 1/5: Merging dictionaries..."
    python3 "$BUILD_DIR/01_merge_csv.py"
}

# Step 2: Create App DB
do_create_app_db() {
    print_step "Step 2/5: Creating App SQLite database..."
    bash "$BUILD_DIR/02_create_app_db.sh"
}

# Step 3: Create Trie DB
do_create_trie_db() {
    print_step "Step 3/5: Creating Trie SQLite database..."
    bash "$BUILD_DIR/03_create_trie_db.sh"
}

# Step 4: Create Trie
do_create_trie() {
    print_step "Step 4/5: Creating MARISA-trie..."
    python3 "$BUILD_DIR/04_create_trie.py"
}

# Step 5: Deploy
do_deploy() {
    print_step "Step 5/5: Deploying to Android..."
    bash "$BUILD_DIR/05_deploy.sh"
}

# 完整建置（不含 deploy）
do_build() {
    do_merge_csv
    do_create_app_db
    do_create_trie_db
    do_create_trie
}

# 顯示用法
show_usage() {
    echo "用法: ./build.sh [command]"
    echo ""
    echo "Commands:"
    echo "  (none)    完整建置（不含 deploy）"
    echo "  all       完整建置 + deploy"
    echo "  deploy    只執行 deploy"
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
