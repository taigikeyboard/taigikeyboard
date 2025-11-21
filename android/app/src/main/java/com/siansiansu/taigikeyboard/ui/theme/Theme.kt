package com.siansiansu.taigikeyboard.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

// Light Mode 配色
private val LightColorScheme = lightColorScheme(
    primary = Color(0xFF007AFF),           // modern_accent
    onPrimary = Color.White,
    primaryContainer = Color(0xFFE6F2F2F7),  // modern_surface_secondary
    onPrimaryContainer = Color(0xFF0F0F12),  // modern_text_primary

    secondary = Color(0xFF8C3FF2),         // modern_accent_secondary
    onSecondary = Color.White,
    secondaryContainer = Color(0xFFF2FAFAFC), // modern_surface_primary
    onSecondaryContainer = Color(0xFF6D6D73), // modern_text_secondary

    tertiary = Color(0xFF34C759),          // modern_icon_green
    onTertiary = Color.White,

    error = Color(0xFFB00020),
    onError = Color.White,

    background = Color(0xFFFAFAFA),        // windowBackground
    onBackground = Color(0xFF0F0F12),

    surface = Color(0xFFF2FAFAFC),         // modern_surface_primary
    onSurface = Color(0xFF0F0F12),
    surfaceVariant = Color(0xFFE6F2F2F7),
    onSurfaceVariant = Color(0xFF6D6D73),

    outline = Color(0xFFDDDDDD),
    outlineVariant = Color(0xFFEEEEEE)
)

// Dark Mode 配色
private val DarkColorScheme = darkColorScheme(
    primary = Color(0xFF669DF1),           // modern_accent dark
    onPrimary = Color.Black,
    primaryContainer = Color(0xE61F1F23),  // modern_surface_secondary dark
    onPrimaryContainer = Color(0xFFEBEBF5), // modern_text_primary dark

    secondary = Color(0xFFAD78FA),         // modern_accent_secondary dark
    onSecondary = Color.Black,
    secondaryContainer = Color(0xF2141417), // modern_surface_primary dark
    onSecondaryContainer = Color(0xFFAEAEB2), // modern_text_secondary dark

    tertiary = Color(0xFF30D158),          // modern_icon_green dark
    onTertiary = Color.Black,

    error = Color(0xFFFF453A),             // modern_icon_red dark
    onError = Color.Black,

    background = Color(0xFF303030),        // windowBackground dark
    onBackground = Color(0xFFEBEBF5),

    surface = Color(0xF2141417),           // modern_surface_primary dark
    onSurface = Color(0xFFEBEBF5),
    surfaceVariant = Color(0xE61F1F23),
    onSurfaceVariant = Color(0xFFAEAEB2),

    outline = Color(0xFF444444),
    outlineVariant = Color(0xFF333333)
)

/**
 * 台羅鍵盤主題
 *
 * @param darkTheme 是否使用深色主題
 * @param dynamicColor 是否使用 Android 12+ 動態配色（預設關閉）
 * @param content Composable 內容
 */
@Composable
fun TaigiKeyboardTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = false,
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        // Android 12+ 動態配色
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context)
            else dynamicLightColorScheme(context)
        }
        // 使用專案配色
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
