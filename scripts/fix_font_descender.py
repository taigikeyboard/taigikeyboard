#!/usr/bin/env python3
"""
字型 Descender 修復腳本
修復字型的 descender 截斷問題

使用方法：
python3 scripts/fix_font_descender.py <字型檔案路徑>

範例：
python3 scripts/fix_font_descender.py Iansui-Regular.ttf

作者：Claude
日期：2025-09-28
"""

from fontTools import ttLib
import os
import sys
import shutil
from datetime import datetime

def backup_original_font(font_path):
    """備份原始字型檔案"""
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    backup_path = f"{font_path}.backup_{timestamp}"
    shutil.copy2(font_path, backup_path)
    print(f"✅ 原字型已備份至: {backup_path}")
    return backup_path

def analyze_font_metrics(font_path):
    """分析字型 metrics"""
    print(f"\n📊 分析字型: {font_path}")
    print("=" * 50)

    font = ttLib.TTFont(font_path)

    # 讀取 hhea table
    hhea = font['hhea']
    print(f"hhea table:")
    print(f"  ascent:    {hhea.ascent}")
    print(f"  descent:   {hhea.descent}")
    print(f"  lineGap:   {hhea.lineGap}")

    # 讀取 OS/2 table
    os2 = font['OS/2']
    print(f"\nOS/2 table:")
    print(f"  sTypoAscender:  {os2.sTypoAscender}")
    print(f"  sTypoDescender: {os2.sTypoDescender}")
    print(f"  sTypoLineGap:   {os2.sTypoLineGap}")
    print(f"  usWinAscent:    {os2.usWinAscent}")
    print(f"  usWinDescent:   {os2.usWinDescent}")
    print(f"  fsSelection:    {bin(os2.fsSelection)} ({os2.fsSelection})")

    # 讀取 head table
    head = font['head']
    print(f"\nhead table:")
    print(f"  unitsPerEm:     {head.unitsPerEm}")

    font.close()
    return hhea.descent, os2.sTypoDescender

def fix_font_descender(font_path, output_path=None):
    """修復字型 descender 問題"""
    if output_path is None:
        output_path = font_path.replace('.ttf', '-fixed.ttf')

    print(f"\n🔧 開始修復字型...")

    # 載入字型
    font = ttLib.TTFont(font_path)

    # 取得 tables
    hhea = font['hhea']
    os2 = font['OS/2']

    # 記錄原始值
    original_hhea_descent = hhea.descent
    original_os2_descent = os2.sTypoDescender

    print(f"原始值:")
    print(f"  hhea.descent: {original_hhea_descent}")
    print(f"  OS/2.sTypoDescender: {original_os2_descent}")

    # 修改 descender 值
    # 參考 Noto Sans CJK TC 的成功案例：hhea.descent: -288
    new_hhea_descent = -250  # 從 -153 增加到 -250
    new_os2_descent = -200   # 與 hhea 稍微不同，創造不一致性

    hhea.descent = new_hhea_descent
    os2.sTypoDescender = new_os2_descent

    print(f"\n修改後:")
    print(f"  hhea.descent: {new_hhea_descent} (增加 {abs(new_hhea_descent - original_hhea_descent)} 單位)")
    print(f"  OS/2.sTypoDescender: {new_os2_descent} (增加 {abs(new_os2_descent - original_os2_descent)} 單位)")

    # 保存修改後的字型
    font.save(output_path)
    font.close()

    print(f"✅ 修復完成，新字型已保存至: {output_path}")
    return output_path

def verify_fix(fixed_font_path, original_descent):
    """驗證修復效果"""
    print(f"\n🔍 驗證修復效果...")
    analyze_font_metrics(fixed_font_path)

    # 計算改善程度
    font = ttLib.TTFont(fixed_font_path)
    hhea = font['hhea']
    os2 = font['OS/2']

    descender_space = abs(hhea.descent)
    typo_space = abs(os2.sTypoDescender)
    original_abs = abs(original_descent)

    print(f"\n📈 修復效果評估:")
    print(f"  Descender 可用空間: {descender_space} 單位")
    print(f"  與原始 {original_descent} 相比: 增加 {descender_space - original_abs} 單位 ({((descender_space - original_abs) / original_abs * 100):.1f}%)")
    print(f"  hhea 與 OS/2 差異: {abs(hhea.descent - os2.sTypoDescender)} 單位")

    font.close()

def update_project_font(fixed_font_path, original_font_path):
    """更新專案中的字型檔案"""
    if os.path.exists(original_font_path):
        # 備份專案中的原字型
        backup_path = backup_original_font(original_font_path)

        # 複製修復後的字型到專案
        shutil.copy2(fixed_font_path, original_font_path)
        print(f"✅ 專案字型已更新: {original_font_path}")
        print(f"📁 原字型備份: {backup_path}")

        return True
    else:
        print(f"❌ 找不到專案字型檔案: {original_font_path}")
        return False

def main():
    """主函數"""
    if len(sys.argv) < 2:
        print("使用方法: python3 scripts/fix_font_descender.py <字型檔案路徑>")
        print("範例: python3 scripts/fix_font_descender.py Iansui-Regular.ttf")
        sys.exit(1)

    font_path = sys.argv[1]

    print("🎯 字型 Descender 修復工具")
    print("=" * 60)

    # 檢查字型檔案是否存在
    if not os.path.exists(font_path):
        print(f"❌ 錯誤: 找不到字型檔案 {font_path}")
        sys.exit(1)

    # 1. 分析原始字型
    print("🔍 步驟 1: 分析原始字型")
    original_descent, original_typo = analyze_font_metrics(font_path)

    # 2. 備份原始字型
    print("\n💾 步驟 2: 備份原始字型")
    backup_path = backup_original_font(font_path)

    # 3. 修復字型
    print("\n🔧 步驟 3: 修復字型 descender")
    fixed_font_path = fix_font_descender(font_path)

    # 4. 驗證修復效果
    print("\n🔍 步驟 4: 驗證修復效果")
    verify_fix(fixed_font_path, original_descent)

    # 5. 詢問是否更新專案字型
    print(f"\n❓ 是否要用修復後的字型替換專案中的原字型？")
    print(f"   原字型會自動備份，可以隨時還原")

    response = input("輸入 'y' 確認，其他鍵取消: ").lower().strip()

    if response == 'y':
        print("\n📦 步驟 5: 更新專案字型")
        success = update_project_font(fixed_font_path, font_path)

        if success:
            print("\n🎉 字型修復完成！")
            print("\n📋 後續步驟:")
            print("1. 重新編譯鍵盤擴充功能")
            print("2. 測試 y、j、g 等字母的顯示效果")
            print("3. 如有問題，可用備份字型還原")
        else:
            print(f"\n💡 手動操作:")
            print(f"請將 {fixed_font_path} 重新命名為 {font_path}")
    else:
        print(f"\n💡 手動測試:")
        print(f"修復後的字型: {fixed_font_path}")
        print(f"可以手動替換 {font_path} 進行測試")

    print(f"\n📁 檔案清單:")
    print(f"  原字型備份: {backup_path}")
    print(f"  修復後字型: {fixed_font_path}")

if __name__ == "__main__":
    main()