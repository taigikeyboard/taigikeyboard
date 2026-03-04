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

// Teal/green brand color scheme (seed: #008A73, tone 50)
// Surface container hierarchy follows iOS grouped-list pattern:
//   screen bg = surfaceContainerLow (light gray)
//   card bg   = surface (white/lightest)
private val LightColorScheme = lightColorScheme(
    primary = Color(0xFF008A73),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFF7FF8DA),
    onPrimaryContainer = Color(0xFF00201A),
    secondary = Color(0xFF4B635C),
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFCDE8DF),
    onSecondaryContainer = Color(0xFF07201A),
    tertiary = Color(0xFF406277),
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFC3E7FF),
    onTertiaryContainer = Color(0xFF001E2E),
    error = Color(0xFFBA1A1A),
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFFFDAD6),
    onErrorContainer = Color(0xFF410002),
    background = Color(0xFFF5FBF7),
    onBackground = Color(0xFF171D1B),
    surface = Color(0xFFFBFFFD),
    onSurface = Color(0xFF171D1B),
    surfaceVariant = Color(0xFFDBE5DF),
    onSurfaceVariant = Color(0xFF3F4945),
    surfaceDim = Color(0xFFD5DBD8),
    surfaceBright = Color(0xFFF5FBF7),
    surfaceContainerLowest = Color(0xFFFFFFFF),
    surfaceContainerLow = Color(0xFFEFF5F1),
    surfaceContainer = Color(0xFFE9F0EC),
    surfaceContainerHigh = Color(0xFFE4EAE6),
    surfaceContainerHighest = Color(0xFFDEE4E0),
    outline = Color(0xFF6F7975),
    outlineVariant = Color(0xFFBFC9C4),
    scrim = Color(0xFF000000),
    inverseSurface = Color(0xFF2B322F),
    inverseOnSurface = Color(0xFFECF2EE),
    inversePrimary = Color(0xFF6DE0C2),
)

private val DarkColorScheme = darkColorScheme(
    primary = Color(0xFF6DE0C2),
    onPrimary = Color(0xFF00382E),
    primaryContainer = Color(0xFF005143),
    onPrimaryContainer = Color(0xFF7FF8DA),
    secondary = Color(0xFFB2CCC3),
    onSecondary = Color(0xFF1D352F),
    secondaryContainer = Color(0xFF344C45),
    onSecondaryContainer = Color(0xFFCDE8DF),
    tertiary = Color(0xFFA7CBE3),
    onTertiary = Color(0xFF0C3447),
    tertiaryContainer = Color(0xFF274B5F),
    onTertiaryContainer = Color(0xFFC3E7FF),
    error = Color(0xFFFFB4AB),
    onError = Color(0xFF690005),
    errorContainer = Color(0xFF93000A),
    onErrorContainer = Color(0xFFFFDAD6),
    background = Color(0xFF0E1513),
    onBackground = Color(0xFFDEE4E0),
    surface = Color(0xFF141A18),
    onSurface = Color(0xFFDEE4E0),
    surfaceVariant = Color(0xFF3F4945),
    onSurfaceVariant = Color(0xFFBFC9C4),
    surfaceDim = Color(0xFF0E1513),
    surfaceBright = Color(0xFF343B38),
    surfaceContainerLowest = Color(0xFF090F0D),
    surfaceContainerLow = Color(0xFF171D1B),
    surfaceContainer = Color(0xFF1B211F),
    surfaceContainerHigh = Color(0xFF252B29),
    surfaceContainerHighest = Color(0xFF303634),
    outline = Color(0xFF89938F),
    outlineVariant = Color(0xFF3F4945),
    scrim = Color(0xFF000000),
    inverseSurface = Color(0xFFDEE4E0),
    inverseOnSurface = Color(0xFF2B322F),
    inversePrimary = Color(0xFF008A73),
)

/**
 * Taigi Keyboard theme
 *
 * Uses teal/green brand colors as fallback, with Android 12+ dynamic color support.
 *
 * @param darkTheme Whether to use dark theme (follows system)
 * @param dynamicColor Whether to use Android 12+ dynamic color (default: true)
 * @param content Composable content
 */
@Composable
fun TaigiKeyboardTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = true,
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context)
            else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
