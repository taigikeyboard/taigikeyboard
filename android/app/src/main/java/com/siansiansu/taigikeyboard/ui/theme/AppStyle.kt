package com.siansiansu.taigikeyboard.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

/**
 * Centralized styling constants for the main app UI.
 *
 * Font sizes are defined in Typography (Theme.kt) and accessed via
 * MaterialTheme.typography.*. This object holds spacing, dimensions,
 * and colors not covered by Material 3.
 *
 * See docs/contributing/ui-style-guide.md for the cross-platform spec.
 */
object AppStyle {
    // ── Icon Sizes ──────────────────────────────────────────────
    val trailingChevronSize = 24.dp
    val smallIconSize = 16.dp // detail icons, external link, info/help
    val selectionIconSize = 20.dp // checkmark, reset/refresh

    // ── Spacing ─────────────────────────────────────────────────
    val sectionSpacing = 24.dp
    val sectionHeaderBottomPadding = 6.dp
    val scrollContentBottomPadding = 40.dp

    // ── LargeTopAppBar ──────────────────────────────────────────
    val largeTopAppBarExpandedHeight = 112.dp

    // ── Colors (not in Material 3) ──────────────────────────────
    // Aligned with iOS system colors (light / dark variants).

    /** Accent orange for warning and feature icons. iOS equivalent: .orange */
    @Composable
    fun warningOrange(): Color = if (isSystemInDarkTheme()) Color(0xFFFF9F0A) else Color(0xFFFF9500)

    /** Dashed outlines. iOS equivalent: secondaryLabel */
    @Composable
    fun secondaryLabel(isDark: Boolean = isSystemInDarkTheme()): Color = (if (isDark) Color(0xFFEBEBF5) else Color(0xFF3C3C43)).copy(alpha = 0.6f)

    /** Gallery create-card panel. iOS equivalent: systemGray4 */
    @Composable
    fun createCardFill(): Color = if (isSystemInDarkTheme()) Color(0xFF3A3A3C) else Color(0xFFD1D1D6)

    /** Neutral bordered-button background, also its disabled state. iOS equivalent: secondarySystemFill */
    @Composable
    fun neutralFill(): Color = Color(0xFF787880).copy(alpha = if (isSystemInDarkTheme()) 0.32f else 0.16f)

    /** Destructive bordered-button background. iOS equivalent: `.bordered` under `.tint(.red)` */
    @Composable
    fun destructiveFill(): Color = MaterialTheme.colorScheme.error.copy(alpha = if (isSystemInDarkTheme()) 0.20f else 0.18f)

    /** Disabled bordered-button content. iOS equivalent: tertiaryLabel */
    @Composable
    fun disabledContent(): Color = (if (isSystemInDarkTheme()) Color(0xFFEBEBF5) else Color(0xFF3C3C43)).copy(alpha = 0.3f)
}

/** Shared section header used across all tabs and settings screens. */
@Composable
fun SectionHeader(
    text: String,
    modifier: Modifier = Modifier,
) {
    Text(
        text = text,
        modifier = modifier.padding(start = 16.dp, bottom = AppStyle.sectionHeaderBottomPadding),
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        style = MaterialTheme.typography.titleMedium,
    )
}
