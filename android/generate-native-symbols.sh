#!/bin/bash

# 生成 native-debug-symbols.zip 腳本
# 用於手動打包原生庫的 debug symbols

set -e

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$PROJECT_DIR/app/build"
OUTPUT_FILE="$PROJECT_DIR/native-debug-symbols.zip"
TEMP_DIR="$PROJECT_DIR/temp-native-symbols"

echo "========================================="
echo "生成 Native Debug Symbols"
echo "========================================="

# 檢查是否已經執行過 build
if [ ! -d "$BUILD_DIR/intermediates/merged_native_libs/release" ]; then
    echo "❌ 錯誤：找不到 release build 產物"
    echo "請先執行：./gradlew bundleRelease"
    exit 1
fi

# 清理舊的暫存目錄
rm -rf "$TEMP_DIR"
mkdir -p "$TEMP_DIR"

echo ""
echo "📦 收集原生庫檔案..."

# 尋找原生庫來源（優先順序：unstripped > merged > stripped）
NATIVE_LIBS_DIR=""

# 1. 嘗試找 unstripped 版本（如果有自己的 native 代碼）
if [ -d "$BUILD_DIR/intermediates/cmake/release/obj" ]; then
    NATIVE_LIBS_DIR="$BUILD_DIR/intermediates/cmake/release/obj"
    echo "✓ 找到 unstripped 版本（CMake 編譯）"
elif [ -d "$BUILD_DIR/intermediates/ndkBuild/release/obj" ]; then
    NATIVE_LIBS_DIR="$BUILD_DIR/intermediates/ndkBuild/release/obj"
    echo "✓ 找到 unstripped 版本（ndk-build 編譯）"
# 2. 使用 merged 版本（包含所有依賴）
elif [ -d "$BUILD_DIR/intermediates/merged_native_libs/release/mergeReleaseNativeLibs/out/lib" ]; then
    NATIVE_LIBS_DIR="$BUILD_DIR/intermediates/merged_native_libs/release/mergeReleaseNativeLibs/out/lib"
    echo "⚠️  使用已合併的原生庫（第三方依賴，已 stripped）"
# 3. 使用 stripped 版本
elif [ -d "$BUILD_DIR/intermediates/stripped_native_libs/release/stripReleaseDebugSymbols/out/lib" ]; then
    NATIVE_LIBS_DIR="$BUILD_DIR/intermediates/stripped_native_libs/release/stripReleaseDebugSymbols/out/lib"
    echo "⚠️  使用 stripped 版本"
else
    echo "❌ 錯誤：找不到原生庫檔案"
    exit 1
fi

# 複製所有架構的 .so 檔案
SO_COUNT=0
for arch in arm64-v8a armeabi-v7a x86 x86_64; do
    if [ -d "$NATIVE_LIBS_DIR/$arch" ]; then
        mkdir -p "$TEMP_DIR/$arch"
        if ls "$NATIVE_LIBS_DIR/$arch"/*.so 1> /dev/null 2>&1; then
            cp "$NATIVE_LIBS_DIR/$arch"/*.so "$TEMP_DIR/$arch/"
            SO_FILES=$(ls "$NATIVE_LIBS_DIR/$arch"/*.so | wc -l | tr -d ' ')
            echo "  - $arch: $SO_FILES 個檔案"
            SO_COUNT=$((SO_COUNT + SO_FILES))
        fi
    fi
done

if [ $SO_COUNT -eq 0 ]; then
    echo "❌ 錯誤：沒有找到任何 .so 檔案"
    rm -rf "$TEMP_DIR"
    exit 1
fi

echo ""
echo "📝 檢查檔案狀態..."

# 檢查檔案是否 stripped
SAMPLE_SO=$(find "$TEMP_DIR" -name "*.so" | head -1)
if file "$SAMPLE_SO" | grep -q "stripped"; then
    echo "⚠️  警告：檔案已被 stripped（debug symbols 已移除）"
    echo "   這對第三方庫是正常的，Google Play 已有其符號資訊"
else
    echo "✓ 檔案包含 debug symbols"
fi

echo ""
echo "📦 建立 ZIP 封裝..."

# 刪除舊的 zip
rm -f "$OUTPUT_FILE"

# 建立 zip（使用相對路徑）
cd "$TEMP_DIR"
zip -r "$OUTPUT_FILE" . -q

cd "$PROJECT_DIR"

# 清理暫存目錄
rm -rf "$TEMP_DIR"

# 顯示結果
FILE_SIZE=$(du -h "$OUTPUT_FILE" | cut -f1)
echo ""
echo "========================================="
echo "✅ 完成！"
echo "========================================="
echo "檔案位置: $OUTPUT_FILE"
echo "檔案大小: $FILE_SIZE"
echo "包含檔案: $SO_COUNT 個 .so 檔案"
echo ""
echo "📌 注意事項："
echo "1. 如果檔案是 stripped 版本，表示來自第三方依賴"
echo "2. Google Play 已有 AndroidX 等常見庫的符號資訊"
echo "3. 只有當你有自己的 C/C++/JNI 代碼時才真正需要此檔案"
echo ""
echo "上傳方式："
echo "1. 登入 Google Play Console"
echo "2. 選擇應用程式 > 發布 > App Bundle 探索工具"
echo "3. 選擇對應版本，上傳 native-debug-symbols.zip"
echo "========================================="
