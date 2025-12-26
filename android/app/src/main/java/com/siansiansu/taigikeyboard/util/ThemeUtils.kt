package com.siansiansu.taigikeyboard.util

import android.app.Activity
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper

/**
 * Theme 工具類別
 *
 * 根據使用者的字體設定套用對應的 Theme
 * 必須在 setContentView() 之前呼叫
 */
object ThemeUtils {

    /**
     * 根據字體設定套用對應的 Theme
     *
     * @param activity Activity 實例
     * @param prefs PrefHelper 實例（可選，若不提供則自動建立）
     */
    fun applyFontTheme(activity: Activity, prefs: PrefHelper? = null) {
        val prefHelper = prefs ?: PrefHelper(activity)
        val themeResId = when (prefHelper.fontType) {
            "system" -> R.style.SettingsTheme_FontSystem
            "openHuninn" -> R.style.SettingsTheme_FontOpenHuninn
            "iansui" -> R.style.SettingsTheme_FontIansui
            else -> R.style.SettingsTheme_FontOpenHuninn // 預設為粉圓
        }
        activity.setTheme(themeResId)
    }
}
