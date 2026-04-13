package com.siansiansu.taigikeyboard.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R

// ── Typography ──────────────────────────────────────────────────────────────
// HuninnFontFamily applied to all slots. Four slots customized to align with iOS:
//   headlineLarge = 34sp (page title),  titleMedium = 18sp (section header),
//   bodyLarge     = 17sp (body text),   labelLarge  = 14sp (caption, M3 default)
private val HuninnFontFamily = FontFamily(Font(R.font.jf_openhuninn_2_1))

private val DefaultTypography = Typography()

private val AppTypography =
    Typography(
        displayLarge = DefaultTypography.displayLarge.copy(fontFamily = HuninnFontFamily),
        displayMedium = DefaultTypography.displayMedium.copy(fontFamily = HuninnFontFamily),
        displaySmall = DefaultTypography.displaySmall.copy(fontFamily = HuninnFontFamily),
        headlineLarge = DefaultTypography.headlineLarge.copy(fontFamily = HuninnFontFamily, fontSize = 34.sp),
        headlineMedium = DefaultTypography.headlineMedium.copy(fontFamily = HuninnFontFamily),
        headlineSmall = DefaultTypography.headlineSmall.copy(fontFamily = HuninnFontFamily),
        titleLarge = DefaultTypography.titleLarge.copy(fontFamily = HuninnFontFamily),
        titleMedium = DefaultTypography.titleMedium.copy(fontFamily = HuninnFontFamily, fontSize = 18.sp),
        titleSmall = DefaultTypography.titleSmall.copy(fontFamily = HuninnFontFamily),
        bodyLarge = DefaultTypography.bodyLarge.copy(fontFamily = HuninnFontFamily, fontSize = 17.sp),
        bodyMedium = DefaultTypography.bodyMedium.copy(fontFamily = HuninnFontFamily),
        bodySmall = DefaultTypography.bodySmall.copy(fontFamily = HuninnFontFamily),
        labelLarge = DefaultTypography.labelLarge.copy(fontFamily = HuninnFontFamily),
        labelMedium = DefaultTypography.labelMedium.copy(fontFamily = HuninnFontFamily),
        labelSmall = DefaultTypography.labelSmall.copy(fontFamily = HuninnFontFamily),
    )

// ── Colors ──────────────────────────────────────────────────────────────────
// Blue primary aligned with iOS .accentColor (#007AFF light / #0A84FF dark)
// Surface container hierarchy follows iOS grouped-list pattern:
//   screen bg = surfaceContainer (gray, 2 levels above surface)
//   card bg   = surface (white/lightest)
private val LightColorScheme =
    lightColorScheme(
        primary = Color(0xFF007AFF),
        onPrimary = Color(0xFFFFFFFF),
        primaryContainer = Color(0xFFD6E4FF),
        onPrimaryContainer = Color(0xFF001B3E),
        secondary = Color(0xFF4B635C),
        onSecondary = Color(0xFFFFFFFF),
        secondaryContainer = Color(0xFFCDE8DF),
        onSecondaryContainer = Color(0xFF07201A),
        tertiary = Color(0xFF406277),
        onTertiary = Color(0xFFFFFFFF),
        tertiaryContainer = Color(0xFFC3E7FF),
        onTertiaryContainer = Color(0xFF001E2E),
        error = Color(0xFFFF3B30),
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
        surfaceContainer = Color(0xFFF2F2F7), // iOS systemGroupedBackground
        surfaceContainerHigh = Color(0xFFE4EAE6),
        surfaceContainerHighest = Color(0xFFDEE4E0),
        outline = Color(0xFF6F7975),
        outlineVariant = Color(0xFFBFC9C4),
        scrim = Color(0xFF000000),
        inverseSurface = Color(0xFF2B322F),
        inverseOnSurface = Color(0xFFECF2EE),
        inversePrimary = Color(0xFF0A84FF),
    )

private val DarkColorScheme =
    darkColorScheme(
        primary = Color(0xFF0A84FF),
        onPrimary = Color(0xFF003062),
        primaryContainer = Color(0xFF00468A),
        onPrimaryContainer = Color(0xFFD6E4FF),
        secondary = Color(0xFFB2CCC3),
        onSecondary = Color(0xFF1D352F),
        secondaryContainer = Color(0xFF344C45),
        onSecondaryContainer = Color(0xFFCDE8DF),
        tertiary = Color(0xFFA7CBE3),
        onTertiary = Color(0xFF0C3447),
        tertiaryContainer = Color(0xFF274B5F),
        onTertiaryContainer = Color(0xFFC3E7FF),
        error = Color(0xFFFF453A),
        onError = Color(0xFFFFFFFF),
        errorContainer = Color(0xFF93000A),
        onErrorContainer = Color(0xFFFFDAD6),
        // Surface & neutral colors — neutral gray, aligned with iOS dark grouped-list
        background = Color(0xFF000000),
        onBackground = Color(0xFFE2E2E6),
        surface = Color(0xFF1C1C1E), // card bg (iOS secondarySystemGroupedBackground)
        onSurface = Color(0xFFE2E2E6),
        surfaceVariant = Color(0xFF3A3A3C),
        onSurfaceVariant = Color(0xFFC0C0C4),
        surfaceDim = Color(0xFF000000),
        surfaceBright = Color(0xFF3A3A3C),
        surfaceContainerLowest = Color(0xFF000000),
        surfaceContainerLow = Color(0xFF0C0C0E),
        surfaceContainer = Color(0xFF000000), // screen bg (iOS systemGroupedBackground)
        surfaceContainerHigh = Color(0xFF252528),
        surfaceContainerHighest = Color(0xFF2C2C2E),
        outline = Color(0xFF8E8E93),
        outlineVariant = Color(0xFF3A3A3C),
        scrim = Color(0xFF000000),
        inverseSurface = Color(0xFFE2E2E6),
        inverseOnSurface = Color(0xFF2C2C2E),
        inversePrimary = Color(0xFF007AFF),
    )

/**
 * Taigi Keyboard theme
 *
 * Uses fixed brand colors aligned with iOS (no dynamic color).
 *
 * @param darkTheme Whether to use dark theme (follows system)
 * @param content Composable content
 */
@Composable
fun TaigiKeyboardTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val colorScheme = if (darkTheme) DarkColorScheme else LightColorScheme

    MaterialTheme(
        colorScheme = colorScheme,
        typography = AppTypography,
        content = content,
    )
}
