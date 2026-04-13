package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.text.font.FontFamily
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.ui.tabs.tab1.DetailScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.FontUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

/**
 * Generic detail page Activity
 * Displays feature explanations, FAQ, feedback, version history, etc.
 */
class DetailActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)
        val languageManager = LanguageManager.getInstance(this)
        val typeface = FontUtils.getTypefaceByType(prefs.fontType, this)
        val fontFamily = FontFamily(androidx.compose.ui.text.font.Typeface(typeface))

        val titleKey = intent.getStringExtra(EXTRA_TITLE_KEY) ?: ""
        val contentType = intent.getStringExtra(EXTRA_CONTENT_TYPE) ?: ""
        val contentKeys = intent.getStringArrayExtra(EXTRA_CONTENT_KEYS) ?: emptyArray()

        setupEdgeToEdge()

        setContent {
            TaigiKeyboardTheme {
                DetailScreen(
                    titleKey = titleKey,
                    contentType = contentType,
                    contentKeys = contentKeys,
                    languageManager = languageManager,
                    fontFamily = fontFamily,
                    onNavigationAction = { action ->
                        when (action) {
                            "setup_guide" -> {
                                startActivity(Intent(this, SetupGuideActivity::class.java))
                            }
                            "feedback" -> {
                                startActivity(createIntent(
                                    this,
                                    titleKey = "contact_us",
                                    contentType = "feedback",
                                    contentKeys = arrayOf("feedback_description", "feedback_email")
                                ))
                            }
                        }
                    },
                    onExternalUrl = { url ->
                        startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
                    },
                    onNavigateBack = {
                        onBackPressedDispatcher.onBackPressed()
                    }
                )
            }
        }
    }

    companion object {
        const val EXTRA_TITLE_KEY = "extra_title_key"
        const val EXTRA_CONTENT_TYPE = "extra_content_type"
        const val EXTRA_CONTENT_KEYS = "extra_content_keys"

        fun createIntent(
            context: Context,
            titleKey: String,
            contentType: String,
            contentKeys: Array<String>
        ): Intent {
            return Intent(context, DetailActivity::class.java).apply {
                putExtra(EXTRA_TITLE_KEY, titleKey)
                putExtra(EXTRA_CONTENT_TYPE, contentType)
                putExtra(EXTRA_CONTENT_KEYS, contentKeys)
            }
        }
    }
}
