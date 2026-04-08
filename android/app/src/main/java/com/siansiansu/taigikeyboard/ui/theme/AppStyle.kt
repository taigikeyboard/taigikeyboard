package com.siansiansu.taigikeyboard.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SwitchColors
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * Centralized styling constants for the main app UI.
 *
 * All font sizes, colors, and dimensions used across screens and components
 * are defined here. Change a value here → applies everywhere.
 *
 * See rules/ui-style-guide.md for the cross-platform spec.
 */
object AppStyle {
    // ── Font Sizes ──────────────────────────────────────────────
    val pageTitleFontSize = 34.sp
    val sectionHeaderFontSize = 19.sp
    val bodyFontSize = 18.sp // row labels, body text, dialog text, search empty state
    val captionFontSize = 14.sp // trailing values, dates, TL annotations, tags, badges

    // ── Spacing ─────────────────────────────────────────────────
    val screenHorizontalPadding = 20.dp
    val contentVerticalPadding = 12.dp
    val sectionSpacing = 24.dp
    val sectionHeaderBottomPadding = 6.dp

    // ── LargeTopAppBar ──────────────────────────────────────────
    val largeTopAppBarExpandedHeight = 112.dp

    // ── Colors (not in Material 3) ──────────────────────────────
    // Aligned with iOS system colors (light / dark variants).

    /** Accent orange for warning and feature icons. iOS equivalent: .orange */
    @Composable
    fun warningOrange(): Color = if (isSystemInDarkTheme()) Color(0xFFFF9F0A) else Color(0xFFFF9500)

    /** Switch colors aligned with iOS toggle (green on, neutral gray off). */
    @Composable
    fun switchColors(): SwitchColors =
        SwitchDefaults.colors(
            checkedTrackColor = if (isSystemInDarkTheme()) Color(0xFF30D158) else Color(0xFF34C759),
            checkedThumbColor = Color.White,
            uncheckedTrackColor = if (isSystemInDarkTheme()) Color(0xFF48484A) else Color(0xFFC7C7CC),
            uncheckedThumbColor = Color.White,
            uncheckedBorderColor = Color.Transparent,
        )
}

/** Shared section header used across all tabs and settings screens. */
@Composable
fun SectionHeader(
    text: String,
    modifier: Modifier = Modifier
        .padding(start = 16.dp, bottom = AppStyle.sectionHeaderBottomPadding),
) {
    Text(
        text = text,
        modifier = modifier,
        fontSize = AppStyle.sectionHeaderFontSize,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}
