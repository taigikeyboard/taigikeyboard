package com.siansiansu.taigikeyboard.ui.components

import androidx.compose.material.icons.Icons
import androidx.compose.ui.graphics.vector.ImageVector

/**
 * Shared icon definitions for settings screens.
 *
 * Used by both InputSettingsScreen (Settings tab) and SettingsOverlayContent (keyboard overlay)
 * to keep icons in sync across the app and keyboard.
 */
object SettingsIcons {
    val candidateDisplayMode: ImageVector get() = Icons.Outlined.ShortText
    val outputBothScripts: ImageVector get() = Icons.Outlined.Translate
    val literalRomanCandidate: ImageVector get() = Icons.Outlined.Abc
    val autoCapitalization: ImageVector get() = Icons.Outlined.FormatSize
    val autoSpace: ImageVector get() = Icons.Outlined.SpaceBar
    val toolbar: ImageVector get() = Icons.Outlined.ViewStream
    val globe: ImageVector get() = Icons.Outlined.Language
    val sound: ImageVector get() = Icons.AutoMirrored.Outlined.VolumeUp
    val vibration: ImageVector get() = Icons.Outlined.Vibration
}
