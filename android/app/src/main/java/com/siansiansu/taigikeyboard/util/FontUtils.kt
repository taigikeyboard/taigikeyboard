package com.siansiansu.taigikeyboard.util

import android.content.Context
import android.graphics.Typeface
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.core.content.res.ResourcesCompat
import com.siansiansu.taigikeyboard.R

/**
 * 字體工具類別
 *
 * 提供自訂字體載入功能
 * 對應 iOS 版本 KeyboardModels.Fonts
 *
 * 注意：ResourcesCompat.getFont() 內建快取機制，
 * 因此這裡不需要額外實作快取
 */
object FontUtils {

    /**
     * 字型類型
     */
    enum class FontType(val value: String) {
        SYSTEM("system"),
        OPEN_HUNINN("openHuninn"),
        IANSUI("iansui")
    }

    /**
     * 取得 粉圓字體 Typeface
     */
    fun getOpenHuninnTypeface(context: Context): Typeface {
        return try {
            ResourcesCompat.getFont(context, R.font.jf_openhuninn) ?: Typeface.DEFAULT
        } catch (e: Exception) {
            Typeface.DEFAULT
        }
    }

    /**
     * 取得芫荽字體 Typeface
     */
    fun getIansuiTypeface(context: Context): Typeface {
        return try {
            ResourcesCompat.getFont(context, R.font.iansui_regular) ?: Typeface.DEFAULT
        } catch (e: Exception) {
            Typeface.DEFAULT
        }
    }

    /**
     * 根據字型類型取得適當的字體
     *
     * @param fontType 字型類型 (system, openHuninn, iansui)
     * @param context Android Context
     * @return 適當的 Typeface
     */
    fun getTypefaceByType(fontType: String, context: Context): Typeface {
        return when (fontType) {
            FontType.SYSTEM.value -> Typeface.DEFAULT
            FontType.OPEN_HUNINN.value -> getOpenHuninnTypeface(context)
            FontType.IANSUI.value -> getIansuiTypeface(context)
            else -> getOpenHuninnTypeface(context) // 預設為 粉圓
        }
    }

    /**
     * 遞迴套用字體到 View 及其所有子 View
     */
    fun applyTypefaceRecursively(view: View, typeface: Typeface) {
        if (view is TextView) {
            view.typeface = typeface
        }
        if (view is ViewGroup) {
            for (i in 0 until view.childCount) {
                applyTypefaceRecursively(view.getChildAt(i), typeface)
            }
        }
    }

    /**
     * 根據字型設定套用字體到 View
     */
    fun applyFontToView(view: View, fontType: String, context: Context) {
        val typeface = getTypefaceByType(fontType, context)
        applyTypefaceRecursively(view, typeface)
    }
}
