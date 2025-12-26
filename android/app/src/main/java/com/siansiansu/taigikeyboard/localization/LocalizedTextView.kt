package com.siansiansu.taigikeyboard.localization

import android.widget.TextView
import androidx.lifecycle.LifecycleOwner
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch

/**
 * TextView 設定本地化 LocalizedText
 * 會監聽語言變化並自動更新
 */
fun TextView.setLocalizedText(
    localizedText: LocalizedText,
    languageManager: LanguageManager,
    lifecycleOwner: LifecycleOwner,
    scope: CoroutineScope
) {
    // 初始設定
    text = languageManager.text(localizedText)

    // 監聽語言變化
    scope.launch {
        languageManager.currentLanguageFlow.collectLatest { language ->
            text = localizedText.text(language)
        }
    }
}

/**
 * 簡化版：不監聽語言變化，只設定一次
 */
fun TextView.setLocalizedText(
    localizedText: LocalizedText,
    languageManager: LanguageManager
) {
    text = languageManager.text(localizedText)
}
