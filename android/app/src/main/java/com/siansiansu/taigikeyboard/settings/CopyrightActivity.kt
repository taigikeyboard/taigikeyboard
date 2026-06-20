package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.remember
import androidx.compose.ui.text.font.FontFamily
import androidx.core.net.toUri
import com.siansiansu.taigikeyboard.content.CopyrightDataSource
import com.siansiansu.taigikeyboard.i18n.LocalStringResolver
import com.siansiansu.taigikeyboard.i18n.ProvideDisplayLanguage
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.typeface.TypefaceLoader
import com.siansiansu.taigikeyboard.ui.setupEdgeToEdge
import com.siansiansu.taigikeyboard.ui.tabs.home.CopyrightScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import androidx.compose.ui.text.font.Typeface as ComposeTypeface

// Copyright and license information display
class CopyrightActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)
        val typeface = TypefaceLoader.getTypefaceByType(prefs.fontType, this)
        val fontFamily = FontFamily(ComposeTypeface(typeface))

        setupEdgeToEdge()

        setContent {
            ProvideDisplayLanguage(prefs) {
                val resolver = LocalStringResolver.current
                TaigiKeyboardTheme {
                    CopyrightScreen(
                        copyrightPages = remember(resolver) { CopyrightDataSource.copyrightPages(resolver) },
                        fontFamily = fontFamily,
                        onButtonClick = { url ->
                            startActivity(Intent(Intent.ACTION_VIEW, url.toUri()))
                        },
                        onNavigateBack = {
                            onBackPressedDispatcher.onBackPressed()
                        },
                    )
                }
            }
        }
    }
}
