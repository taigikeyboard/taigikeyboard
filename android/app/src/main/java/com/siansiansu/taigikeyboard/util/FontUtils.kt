package com.siansiansu.taigikeyboard.util

import android.content.Context
import android.graphics.Typeface
import androidx.core.content.res.ResourcesCompat
import com.siansiansu.taigikeyboard.R

/**
 * 字體工具類別
 *
 * 提供自訂字體載入功能
 * 對應 iOS 版本 KeyboardModels.Fonts
 */
object FontUtils {

    /**
     * 自訂字體名稱：jf-openhuninn-2.1
     */
    private const val CUSTOM_FONT_NAME = "jf-openhuninn-2.1"

    /**
     * 快取的自訂字體 Typeface
     */
    private var customTypeface: Typeface? = null

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
     * 根據設定取得適當的字體
     * 對應 iOS 版本的 buttonKeyboardFont(for action:) 邏輯
     *
     * @param customFontEnabled 是否啟用自訂字體（來自 toggleCustomFont 設定）
     * @param context Android Context
     * @return 適當的 Typeface
     */
    fun getKeyFont(customFontEnabled: Boolean, context: Context): Typeface {
        return if (customFontEnabled) {
            getCustomTypeface(context)
        } else {
            Typeface.DEFAULT
        }
    }

    /**
     * 清除快取的字體（用於測試或記憶體管理）
     */
    fun clearCache() {
        customTypeface = null
    }
}
