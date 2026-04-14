package com.siansiansu.taigikeyboard.settings

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ui.tabs.tab2.AppearanceSettingsScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

class AppearanceSettingsActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)
        prefs.warmUp()

        setupEdgeToEdge()

        setContent {
            TaigiKeyboardTheme {
                AppearanceSettingsScreen(
                    prefs = prefs,
                    onNavigateBack = {
                        onBackPressedDispatcher.onBackPressed()
                    },
                    onFontChanged = {
                        recreate()
                    },
                )
            }
        }
    }
}
