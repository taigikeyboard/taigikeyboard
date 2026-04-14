package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ui.tabs.tab3.AssociationDataScreen
import com.siansiansu.taigikeyboard.ui.tabs.tab3.FrequencyDataScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

class FrequentWordsActivity : ComponentActivity() {
    companion object {
        private const val EXTRA_TYPE = "type"
        const val TYPE_FREQUENCY = "frequency"
        const val TYPE_ASSOCIATION = "association"

        fun createIntent(
            context: Context,
            type: String,
        ): Intent =
            Intent(context, FrequentWordsActivity::class.java)
                .putExtra(EXTRA_TYPE, type)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)
        val type = intent.getStringExtra(EXTRA_TYPE) ?: TYPE_FREQUENCY

        setupEdgeToEdge()

        setContent {
            TaigiKeyboardTheme {
                when (type) {
                    TYPE_FREQUENCY -> {
                        FrequencyDataScreen(
                            prefs = prefs,
                            onNavigateBack = {
                                onBackPressedDispatcher.onBackPressed()
                            },
                        )
                    }

                    TYPE_ASSOCIATION -> {
                        AssociationDataScreen(
                            prefs = prefs,
                            onNavigateBack = {
                                onBackPressedDispatcher.onBackPressed()
                            },
                        )
                    }
                }
            }
        }
    }
}
