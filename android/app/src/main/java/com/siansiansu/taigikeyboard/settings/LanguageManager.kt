package com.siansiansu.taigikeyboard.settings

import android.content.Context
import androidx.lifecycle.LiveData
import androidx.lifecycle.MutableLiveData

class LanguageManager private constructor(context: Context) {
    private val _currentDisplayLanguage = MutableLiveData<DisplayLanguage>()
    val currentDisplayLanguage: LiveData<DisplayLanguage> = _currentDisplayLanguage

    init {
        // showHanjiMode 固定為 true，直接設定顯示語言為漢字
        _currentDisplayLanguage.value = DisplayLanguage.HANJI
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
