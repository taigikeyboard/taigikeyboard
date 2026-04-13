package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.text.font.FontFamily
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.model.CopyrightDataSource
import com.siansiansu.taigikeyboard.ui.tabs.tab1.CopyrightScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.FontUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

class CopyrightActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)
        val languageManager = LanguageManager.getInstance(this)
        val typeface = FontUtils.getTypefaceByType(prefs.fontType, this)
        val fontFamily = FontFamily(androidx.compose.ui.text.font.Typeface(typeface))

        setupEdgeToEdge()

        setContent {
            TaigiKeyboardTheme {
                CopyrightScreen(
                    copyrightPages = CopyrightDataSource.copyrightPages,
                    languageManager = languageManager,
                    fontFamily = fontFamily,
                    onButtonClick = { url ->
                        startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
                    },
                    onNavigateBack = {
                        onBackPressedDispatcher.onBackPressed()
                    }
                )
            }
        }
    }
}
