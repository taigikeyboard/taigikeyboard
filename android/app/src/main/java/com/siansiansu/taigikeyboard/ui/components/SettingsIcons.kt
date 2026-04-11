package com.siansiansu.taigikeyboard.ui.components

import androidx.compose.material.icons.Icons
import com.siansiansu.taigikeyboard.ui.components.FormatSize
import com.siansiansu.taigikeyboard.ui.components.Language
import com.siansiansu.taigikeyboard.ui.components.SpaceBar
import com.siansiansu.taigikeyboard.ui.components.Translate
import com.siansiansu.taigikeyboard.ui.components.Vibration
import com.siansiansu.taigikeyboard.ui.components.ViewStream
import com.siansiansu.taigikeyboard.ui.components.VolumeUp
import androidx.compose.ui.graphics.vector.ImageVector

/**
 * Shared icon definitions for settings screens.
 *
 * Used by both InputSettingsScreen (Tab4) and SettingsOverlayContent (keyboard overlay)
 * to keep icons in sync across the app and keyboard.
 */
object SettingsIcons {
    val outputBothScripts: ImageVector get() = Icons.Outlined.Translate
    val autoCapitalization: ImageVector get() = Icons.Outlined.FormatSize
    val autoSpace: ImageVector get() = Icons.Outlined.SpaceBar
    val toolbar: ImageVector get() = Icons.Outlined.ViewStream
    val globe: ImageVector get() = Icons.Outlined.Language
    val sound: ImageVector get() = Icons.AutoMirrored.Outlined.VolumeUp
    val vibration: ImageVector get() = Icons.Outlined.Vibration
}
