package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.util.Log
import androidx.lifecycle.LiveData
import androidx.lifecycle.MutableLiveData
import androidx.preference.PreferenceManager
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.launchIn
import kotlinx.coroutines.flow.onEach

class LanguageManager private constructor(context: Context) {
    private val prefs: PrefHelper = PrefHelper(context.applicationContext)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    private val _currentDisplayLanguage = MutableLiveData<DisplayLanguage>()
    val currentDisplayLanguage: LiveData<DisplayLanguage> = _currentDisplayLanguage

    init {
        // Automatically observe preference changes and update display language
        combine(
            prefs.observeInputMode(),
            prefs.observeShowHanjiMode()
        ) { inputMode, showHanji ->
            val newLanguage = if (showHanji) {
                DisplayLanguage.HANJI
            } else {
                if (inputMode == "poj") DisplayLanguage.POJ else DisplayLanguage.TL
            }

            newLanguage
        }.onEach { newLanguage ->
            _currentDisplayLanguage.value = newLanguage
        }.launchIn(scope)
    }

    fun getText(localizedText: LocalizedText): String {
        return localizedText.text(_currentDisplayLanguage.value ?: DisplayLanguage.HANJI)
    }

    companion object {
        @Volatile
        private var instance: LanguageManager? = null

        fun getInstance(context: Context): LanguageManager {
            return instance ?: synchronized(this) {
                instance ?: LanguageManager(context).also { instance = it }
            }
        }
    }
}
