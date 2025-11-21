package com.siansiansu.taigikeyboard.util

import android.content.Context
import android.graphics.Typeface
import androidx.core.content.res.ResourcesCompat
import com.siansiansu.taigikeyboard.R

/**
 * 字體工具類別
 *
 * 提供自訂字體載入與 CJK Extension 字元檢測功能
 * 對應 iOS 版本 KeyboardModels.Fonts
 */
object FontUtils {

    /**
     * 自訂字體名稱：jf-openhuninn-2.1
     */
    private const val CUSTOM_FONT_NAME = "jf-openhuninn-2.1"

    /**
     * CJK Extension 系列 Unicode 範圍
     * 對應 iOS 版本的 cjkExtensionRanges
     */
    private val CJK_EXTENSION_RANGES = listOf(
        0x3400..0x4DBF,      // CJK Extension A
        0x20000..0x2A6DF,    // CJK Extension B
        0x2A700..0x2B73F,    // CJK Extension C
        0x2B740..0x2B81F,    // CJK Extension D
        0x2B820..0x2CEAF,    // CJK Extension E
        0x2CEB0..0x2EBEF,    // CJK Extension F
        0x30000..0x3134F     // CJK Extension G
    )

    /**
     * 快取的自訂字體 Typeface
     */
    private var customTypeface: Typeface? = null

    /**
     * 檢查文字是否包含 CJK Extension 字元
     * 對應 iOS 版本的 containsCJKExtension(_ text: String) -> Bool
     *
     * 注意：必須正確處理 UTF-16 surrogate pairs
     * CJK Extension B-G (0x20000-0x3134F) 需要 surrogate pairs 來表示
     *
     * @param text 要檢查的文字
     * @return true 如果包含 CJK Extension 字元
     */
    fun containsCJKExtension(text: String): Boolean {
        var i = 0
        while (i < text.length) {
            val codePoint = text.codePointAt(i)
            if (CJK_EXTENSION_RANGES.any { codePoint in it }) {
                return true
            }
            // 處理 surrogate pairs：高位 surrogate 需要跳過兩個 char
            i += Character.charCount(codePoint)
        }
        return false
    }

    /**
     * 取得自訂字體 Typeface
     *
     * @param context Android Context
     * @return 自訂字體 Typeface，如果載入失敗則回傳預設字體
     */
    fun getCustomTypeface(context: Context): Typeface {
        if (customTypeface == null) {
            customTypeface = try {
                ResourcesCompat.getFont(context, R.font.jf_openhuninn)
            } catch (e: Exception) {
                // 如果載入失敗，使用預設字體
                Typeface.DEFAULT
            }
        }
        return customTypeface ?: Typeface.DEFAULT
    }

    /**
     * 根據設定和文字內容取得適當的字體
     * 對應 iOS 版本的 buttonKeyboardFont(for action:) 邏輯
     *
     * toggleCustomFont 關閉時：
     * - 僅對 CJK Extension 字元使用自訂字體
     * - 其他字元使用系統預設字體
     *
     * toggleCustomFont 開啟時：
     * - 所有字元都使用自訂字體
     *
     * @param text 要顯示的文字
     * @param customFontEnabled 是否啟用自訂字體（來自 toggleCustomFont 設定）
     * @param context Android Context
     * @return 適當的 Typeface
     */
    fun getKeyFont(text: String, customFontEnabled: Boolean, context: Context): Typeface {
        return when {
            // 自訂字體開啟：所有文字都使用自訂字體
            customFontEnabled -> getCustomTypeface(context)

            // 自訂字體關閉：只對 CJK Extension 字元使用自訂字體
            containsCJKExtension(text) -> getCustomTypeface(context)

            // 其他情況使用系統預設字體
            else -> Typeface.DEFAULT
        }
    }

    /**
     * 清除快取的字體（用於測試或記憶體管理）
     */
    fun clearCache() {
        customTypeface = null
    }
}
