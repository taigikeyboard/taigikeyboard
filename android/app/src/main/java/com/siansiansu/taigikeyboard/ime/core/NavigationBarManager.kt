package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import android.view.Window
import androidx.core.view.WindowCompat
import com.siansiansu.taigikeyboard.ime.core.logging.debug

private const val TAG = "NavigationBarManager"

/**
 * Manages the navigation bar's foreground (icon) color, modeled on FlorisBoard's SystemUi.
 * The navbar background is transparent (set in `theme.xml`) so the keyboard background extends
 * into the navigation bar area; only the icon tint is switched here.
 */
class NavigationBarManager {
    private fun isDarkMode(context: Context): Boolean = isKeyboardNightMode(context)

    fun updateNavigationBar(
        window: Window,
        context: Context,
    ) {
        val isDark = isDarkMode(context)
        val logger = CompositionRoot.shared(context).logger

        logger.debug(TAG) { "=== Updating Navigation Bar ===" }
        logger.debug(TAG) { "  Dark mode: $isDark" }
        logger.debug(TAG) { "  Will use light icons: ${!isDark}" }

        // Light mode gets dark icons, dark mode gets light icons.
        WindowCompat
            .getInsetsController(window, window.decorView)
            .isAppearanceLightNavigationBars = !isDark

        logger.debug(TAG) { "  isAppearanceLightNavigationBars set to: ${!isDark}" }
    }
}
