package com.siansiansu.taigikeyboard.settings

import android.widget.TextView
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.Observer

/**
 * TextView 設定本地化 LocalizedText
 * 會監聽語言變化並自動更新
 */
fun TextView.setLocalizedText(
    localizedText: LocalizedText,
    languageManager: LanguageManager,
    lifecycleOwner: LifecycleOwner
) {
    // 初始設定
    text = languageManager.getText(localizedText)

    // 監聽語言變化
    languageManager.currentDisplayLanguage.observe(lifecycleOwner, Observer {
        text = localizedText.text(it)
    })
}
