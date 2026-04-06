package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.ui.settings.CustomDictionaryScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

class CustomDictionaryActivity : ComponentActivity() {

    companion object {
        fun createIntent(context: Context): Intent {
            return Intent(context, CustomDictionaryActivity::class.java)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        CustomDictionaryService.init(this)
        val languageManager = LanguageManager.getInstance(this)
        val prefHelper = PrefHelper(this)
        prefHelper.warmUp()

        setupEdgeToEdge()

        setContent {
            TaigiKeyboardTheme {
                CustomDictionaryScreen(
                    languageManager = languageManager,
                    prefs = prefHelper,
                    onNavigateBack = {
                        onBackPressedDispatcher.onBackPressed()
                    }
                )
            }
        }
    }
}
