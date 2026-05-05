package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.text.font.FontFamily
import com.siansiansu.taigikeyboard.content.ContentType
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ui.tabs.home.DetailScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.FontUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge
import androidx.compose.ui.text.font.Typeface as ComposeTypeface

// Generic detail page for feature explanations, FAQ, feedback, and version history
class DetailActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)
        val typeface = FontUtils.getTypefaceByType(prefs.fontType, this)
        val fontFamily = FontFamily(ComposeTypeface(typeface))

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
                    fontFamily = fontFamily,
                    onNavigationAction = { action ->
                        when (action) {
                            ACTION_SETUP_GUIDE -> {
                                startActivity(Intent(this, SetupGuideActivity::class.java))
                            }

                            ACTION_FEEDBACK -> {
                                startActivity(
                                    createIntent(
                                        this,
                                        titleKey = ContentType.KEY_CONTACT_US,
                                        contentType = ContentType.FEEDBACK,
                                        contentKeys = arrayOf(ContentType.KEY_FEEDBACK_DESCRIPTION, ContentType.KEY_FEEDBACK_EMAIL),
                                    ),
                                )
                            }
                        }
                    },
                    onExternalUrl = { url ->
                        startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
                    },
                    onNavigateBack = {
                        onBackPressedDispatcher.onBackPressed()
                    },
                )
            }
        }
    }

    companion object {
        const val EXTRA_TITLE_KEY = "extra_title_key"
        const val EXTRA_CONTENT_TYPE = "extra_content_type"
        const val EXTRA_CONTENT_KEYS = "extra_content_keys"

        private const val ACTION_SETUP_GUIDE = "setup_guide"
        private const val ACTION_FEEDBACK = "feedback"

        fun createIntent(
            context: Context,
            titleKey: String,
            contentType: String,
            contentKeys: Array<String>,
        ): Intent =
            Intent(context, DetailActivity::class.java).apply {
                putExtra(EXTRA_TITLE_KEY, titleKey)
                putExtra(EXTRA_CONTENT_TYPE, contentType)
                putExtra(EXTRA_CONTENT_KEYS, contentKeys)
            }
    }
}
