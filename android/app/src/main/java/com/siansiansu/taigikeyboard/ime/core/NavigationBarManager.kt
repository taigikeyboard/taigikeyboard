// 中文: 導覽列圖示色管理 — 背景色已在 theme.xml 設透明,讓鍵盤底色直接延伸到 navbar 區。
// 中文: 此檔只負責切換圖示前景色(深淺色 mode 對應)。設計參考 FlorisBoard。

package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import android.content.res.Configuration
import android.util.Log
import android.view.Window
import androidx.core.view.WindowCompat
import com.siansiansu.taigikeyboard.BuildConfig

/**
 * 管理導覽列的前景色（圖示顏色）
 * 參考 FlorisBoard 的 SystemUi 實作
 *
 * 注意：導覽列背景色設定為透明（在 theme.xml 中），
 * 讓鍵盤背景自然延伸到導覽列區域
 */
class NavigationBarManager {
    /**
     * 判斷當前是否為深色模式
     */
    private fun isDarkMode(context: Context): Boolean {
        val nightModeFlags = context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK
        return nightModeFlags == Configuration.UI_MODE_NIGHT_YES
    }

    /**
     * 更新導覽列的圖示顏色
     * @param window IME 的 Window
     * @param context Context
     */
    fun updateNavigationBar(
        window: Window,
        context: Context,
    ) {
        val isDark = isDarkMode(context)

        if (BuildConfig.DEBUG) {
            Log.d("NavigationBarManager", "=== Updating Navigation Bar ===")
            Log.d("NavigationBarManager", "  Dark mode: $isDark")
            Log.d("NavigationBarManager", "  Will use light icons: ${!isDark}")
        }

        // 設定導覽列前景色（圖示顏色）
        // light mode: 深色圖示，dark mode: 淺色圖示
        WindowCompat
            .getInsetsController(window, window.decorView)
            .isAppearanceLightNavigationBars = !isDark

        if (BuildConfig.DEBUG) {
            Log.d("NavigationBarManager", "  isAppearanceLightNavigationBars set to: ${!isDark}")
        }
    }
}
