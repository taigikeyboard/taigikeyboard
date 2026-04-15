package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import com.siansiansu.taigikeyboard.ui.tabs.tab3.CustomDictionaryScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

// Custom dictionary management
class CustomDictionaryActivity : ComponentActivity() {
    companion object {
        fun createIntent(context: Context): Intent = Intent(context, CustomDictionaryActivity::class.java)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        CustomDictionaryService.init(this)
        val prefs = PrefHelper(this)
        prefs.warmUp()

        setupEdgeToEdge()

        setContent {
            TaigiKeyboardTheme {
                CustomDictionaryScreen(
                    prefs = prefs,
                    onNavigateBack = {
                        onBackPressedDispatcher.onBackPressed()
                    },
                )
            }
        }
    }
}
