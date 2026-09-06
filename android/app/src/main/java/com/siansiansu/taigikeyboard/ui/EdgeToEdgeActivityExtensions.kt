package com.siansiansu.taigikeyboard.ui

import android.content.res.Configuration
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.runtime.Composable
import androidx.core.view.WindowCompat
import com.siansiansu.taigikeyboard.i18n.ProvideDisplayLanguage
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme

// Edge-to-edge display setup and Compose-root scaffold extensions for Activity classes
fun AppCompatActivity.setupEdgeToEdge() {
    enableEdgeToEdge()
    WindowCompat.getInsetsController(window, window.decorView).apply {
        isAppearanceLightStatusBars = !isDarkMode()
    }
}

fun ComponentActivity.setupEdgeToEdge() {
    enableEdgeToEdge()
}

// Compose root shared by the stand-alone Settings Activities: edge-to-edge, then
// [content] under the live display-language resolver and the app theme. Call it
// last in onCreate — after intent extras and any early `finish()` — so the
// setupEdgeToEdge → setContent order is unchanged.
fun ComponentActivity.setTaigiContent(
    prefs: PrefHelper,
    content: @Composable () -> Unit,
) {
    setupEdgeToEdge()
    setContent {
        ProvideDisplayLanguage(prefs) {
            TaigiKeyboardTheme(content = content)
        }
    }
}

private fun AppCompatActivity.isDarkMode(): Boolean {
    val nightModeFlags = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK
    return nightModeFlags == Configuration.UI_MODE_NIGHT_YES
}
