package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.viewModels
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ui.setTaigiContent
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.AssociationDataScreen
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.AssociationDataViewModel
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.FrequencyDataScreen
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.FrequencyDataViewModel

// Viewer for user frequency and word association data
class FrequentWordsActivity : ComponentActivity() {
    companion object {
        private const val EXTRA_TYPE = "extra_type"
        const val TYPE_FREQUENCY = "frequency"
        const val TYPE_ASSOCIATION = "association"

        fun createIntent(
            context: Context,
            type: String,
        ): Intent =
            Intent(context, FrequentWordsActivity::class.java)
                .putExtra(EXTRA_TYPE, type)
    }

    private val frequencyViewModel: FrequencyDataViewModel by viewModels()
    private val associationViewModel: AssociationDataViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val type = intent.getStringExtra(EXTRA_TYPE) ?: TYPE_FREQUENCY
        val prefs = PrefHelper(this)

        setTaigiContent(prefs) {
            when (type) {
                TYPE_FREQUENCY -> {
                    FrequencyDataScreen(
                        viewModel = frequencyViewModel,
                        onNavigateBack = {
                            onBackPressedDispatcher.onBackPressed()
                        },
                    )
                }

                TYPE_ASSOCIATION -> {
                    AssociationDataScreen(
                        viewModel = associationViewModel,
                        onNavigateBack = {
                            onBackPressedDispatcher.onBackPressed()
                        },
                    )
                }
            }
        }
    }
}
