package com.siansiansu.taigikeyboard.localization

import android.content.Context
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * 語言管理器
 * 管理當前顯示語言，並提供語言切換功能
 */
class LanguageManager private constructor(context: Context) {
    private val _currentLanguageFlow = MutableStateFlow(DisplayLanguage.HANJI)
    val currentLanguageFlow: StateFlow<DisplayLanguage> = _currentLanguageFlow.asStateFlow()

    val currentLanguage: DisplayLanguage
        get() = _currentLanguageFlow.value

    fun text(localizedText: LocalizedText): String {
        return localizedText.text(currentLanguage)
    }

    fun setLanguage(language: DisplayLanguage) {
        _currentLanguageFlow.value = language
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
