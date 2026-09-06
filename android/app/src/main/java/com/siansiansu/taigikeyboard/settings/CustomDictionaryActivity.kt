package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.viewModels
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ui.setTaigiContent
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.CustomDictionaryScreen
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.CustomDictionaryViewModel

// Custom dictionary management
class CustomDictionaryActivity : ComponentActivity() {
    companion object {
        fun createIntent(context: Context): Intent = Intent(context, CustomDictionaryActivity::class.java)
    }

    private val viewModel: CustomDictionaryViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)

        setTaigiContent(prefs) {
            CustomDictionaryScreen(
                viewModel = viewModel,
                onNavigateBack = {
                    onBackPressedDispatcher.onBackPressed()
                },
            )
        }
    }
}
