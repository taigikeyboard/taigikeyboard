package com.siansiansu.taigikeyboard.ime.theme

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import com.siansiansu.taigikeyboard.ime.core.isKeyboardNightMode
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme

/**
 * Material3 theme for every IME Compose root. Light / dark follows the IME context's
 * resources ([isKeyboardNightMode]), not `LocalConfiguration`: under a user theme the service
 * resources are forced light, while the window's config dispatch can still push the real night
 * uiMode into an existing ComposeView's `LocalConfiguration`.
 */
@Composable
fun KeyboardMaterialTheme(content: @Composable () -> Unit) {
    TaigiKeyboardTheme(darkTheme = isKeyboardNightMode(LocalContext.current), content = content)
}
