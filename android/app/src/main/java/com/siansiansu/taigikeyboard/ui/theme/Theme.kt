package com.siansiansu.taigikeyboard.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext

// 使用 Material 3 預設配色
private val LightColorScheme = lightColorScheme()
private val DarkColorScheme = darkColorScheme()

/**
 * 台語鍵盤主題
 *
 * 使用 Material 3 系統預設配色，支援 Android 12+ 動態配色
 *
 * @param darkTheme 是否使用深色主題（跟隨系統）
 * @param dynamicColor 是否使用 Android 12+ 動態配色（預設開啟）
 * @param content Composable 內容
 */
@Composable
fun TaigiKeyboardTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = true,
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        // Android 12+ 動態配色
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context)
            else dynamicLightColorScheme(context)
        }
        // 使用 Material 3 預設配色
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
