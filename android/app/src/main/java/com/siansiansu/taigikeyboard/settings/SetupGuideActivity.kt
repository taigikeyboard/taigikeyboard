package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ui.tabs.tab1.SetupGuideScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

// Setup guide for enabling the keyboard — shared between Tab1 sub-page and first-launch full-screen mode
class SetupGuideActivity : ComponentActivity() {
    companion object {
        const val EXTRA_IS_FULL_SCREEN = "extra_is_full_screen"

        fun createIntent(
            context: Context,
            isFullScreen: Boolean = false,
        ): Intent =
            Intent(context, SetupGuideActivity::class.java).apply {
                putExtra(EXTRA_IS_FULL_SCREEN, isFullScreen)
            }
    }

    private var hasNavigatedToSettings: Boolean = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val isFullScreen = intent.getBooleanExtra(EXTRA_IS_FULL_SCREEN, false)

        setupEdgeToEdge()

        setContent {
            TaigiKeyboardTheme {
                SetupGuideScreen(
                    isFullScreen = isFullScreen,
                    onGoToSettings = {
                        hasNavigatedToSettings = true
                        startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS))
                    },
                    onClose = {
                        finish()
                    },
                    onNavigateBack = {
                        onBackPressedDispatcher.onBackPressed()
                    },
                )
            }
        }
    }

    override fun onResume() {
        super.onResume()
        if (hasNavigatedToSettings) {
            hasNavigatedToSettings = false
            if (TaigiKeyboard.checkIfImeIsEnabled(this)) {
                finish()
            }
        }
    }
}
