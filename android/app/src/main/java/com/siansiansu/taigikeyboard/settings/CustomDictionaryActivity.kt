package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import com.siansiansu.taigikeyboard.ui.setupEdgeToEdge
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.CustomDictionaryScreen
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.CustomDictionaryViewModel
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme

// Custom dictionary management
class CustomDictionaryActivity : ComponentActivity() {
    companion object {
        fun createIntent(context: Context): Intent = Intent(context, CustomDictionaryActivity::class.java)
    }

    private val viewModel: CustomDictionaryViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setupEdgeToEdge()

        setContent {
            TaigiKeyboardTheme {
                CustomDictionaryScreen(
                    viewModel = viewModel,
                    onNavigateBack = {
                        onBackPressedDispatcher.onBackPressed()
                    },
                )
            }
        }
    }
}
