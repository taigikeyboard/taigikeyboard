package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.widget.Toast
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.lifecycleScope
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab4Texts
import com.siansiansu.taigikeyboard.model.ContentType
import com.siansiansu.taigikeyboard.ui.tabs.MainSettingsScreen
import com.siansiansu.taigikeyboard.ui.tabs.TabItem
import com.siansiansu.taigikeyboard.ui.tabs.tab1.HomeScreen
import com.siansiansu.taigikeyboard.ui.tabs.tab2.LayoutScreen
import com.siansiansu.taigikeyboard.ui.tabs.tab3.DictionarySearchViewModel
import com.siansiansu.taigikeyboard.ui.tabs.tab3.DictionarySettingsScreen
import com.siansiansu.taigikeyboard.ui.tabs.tab4.InputSettingsScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.AppVersionUtils
import com.siansiansu.taigikeyboard.util.PackageManagerUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge
import kotlinx.coroutines.launch

class SettingsMainActivity : AppCompatActivity() {
    companion object {
        const val EXTRA_START_TAB = "extra_start_tab"

        // Tab indices
        private const val TAB_HOME = 0
        private const val TAB_LAYOUT = 1
        private const val TAB_DICTIONARY = 2
        private const val TAB_SETTINGS = 3
    }

    lateinit var prefs: PrefHelper

    private val searchViewModel: DictionarySearchViewModel by viewModels()
    private var resetCounter by mutableIntStateOf(0)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)

        // Check if keyboard is enabled; show setup guide if not
        if (!TaigiKeyboard.checkIfImeIsEnabled(this)) {
            startActivity(SetupGuideActivity.createIntent(this, isFullScreen = true))
        }

        val mode =
            when (prefs.settingsTheme) {
                "light" -> AppCompatDelegate.MODE_NIGHT_NO
                "dark" -> AppCompatDelegate.MODE_NIGHT_YES
                "auto" -> AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM
                else -> AppCompatDelegate.MODE_NIGHT_UNSPECIFIED
            }
        AppCompatDelegate.setDefaultNightMode(mode)

        setupEdgeToEdge()

        AppVersionUtils.updateVersionOnInstallAndLastUse(this, prefs)

        val languageManager = LanguageManager.getInstance(this)
        val initialTab = intent.getIntExtra(EXTRA_START_TAB, TAB_HOME)

        val versionName =
            try {
                packageManager.getPackageInfo(packageName, 0).versionName ?: "1.0"
            } catch (e: Exception) {
                "1.0"
            }

        setContent {
            TaigiKeyboardTheme {
                MainSettingsScreen(
                    tabs =
                        listOf(
                            TabItem(R.drawable.ic_home, getString(R.string.tab_home)),
                            TabItem(R.drawable.keyboard_24, getString(R.string.tab_layout)),
                            TabItem(R.drawable.dictionary_24, getString(R.string.tab_dictionary)),
                            TabItem(R.drawable.ic_settings, getString(R.string.tab_settings)),
                        ),
                    initialTab = initialTab,
                ) { selectedTab ->
                    when (selectedTab) {
                        TAB_HOME -> {
                            HomeScreen(
                                languageManager = languageManager,
                                versionName = versionName,
                                onSetupGuide = {
                                    startActivity(Intent(this, SetupGuideActivity::class.java))
                                },
                                onFeatureClick = { titleKey, contentType, contentKeys ->
                                    openDetailActivity(titleKey, contentType, contentKeys)
                                },
                                onUrlClick = ::openUrl,
                                onCopyright = {
                                    startActivity(Intent(this, CopyrightActivity::class.java))
                                },
                                onFeedback = {
                                    openDetailActivity(
                                        "contact_us",
                                        ContentType.FEEDBACK,
                                        arrayOf("feedback_email"),
                                    )
                                },
                                onVersionHistory = {
                                    openDetailActivity("version_history", ContentType.VERSION, emptyArray())
                                },
                                onFaqClick = { titleKey, contentKeys ->
                                    openDetailActivity(titleKey, ContentType.FAQ, contentKeys)
                                },
                            )
                        }

                        TAB_LAYOUT -> {
                            LayoutScreen(
                                languageManager = languageManager,
                                prefs = prefs,
                                onAppearanceSettings = {
                                    startActivity(
                                        Intent(this, AppearanceSettingsActivity::class.java),
                                    )
                                },
                            )
                        }

                        TAB_DICTIONARY -> {
                            DictionarySettingsScreen(
                                languageManager = languageManager,
                                prefs = prefs,
                                onCustomDictionary = {
                                    startActivity(CustomDictionaryActivity.createIntent(this))
                                },
                                onNavigateToFrequency = {
                                    startActivity(FrequentWordsActivity.createIntent(this, FrequentWordsActivity.TYPE_FREQUENCY))
                                },
                                onNavigateToAssociation = {
                                    startActivity(FrequentWordsActivity.createIntent(this, FrequentWordsActivity.TYPE_ASSOCIATION))
                                },
                                onBackupRestore = {
                                    startActivity(DataManagementActivity.createIntent(this))
                                },
                                searchViewModel = searchViewModel,
                            )
                        }

                        TAB_SETTINGS -> {
                            InputSettingsScreen(
                                languageManager = languageManager,
                                prefs = prefs,
                                onResetSettings = ::resetAllSettings,
                                resetCounter = resetCounter,
                            )
                        }
                    }
                }
            }
        }
    }

    private fun openDetailActivity(
        titleKey: String,
        contentType: String,
        contentKeys: Array<String>,
    ) {
        startActivity(
            DetailActivity.createIntent(this, titleKey, contentType, contentKeys),
        )
    }

    private fun openUrl(url: String) {
        try {
            startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
        } catch (_: Exception) {
            // Handle exception
        }
    }

    private fun resetAllSettings() {
        val languageManager = LanguageManager.getInstance(this)
        lifecycleScope.launch {
            try {
                prefs.resetToDefaults()
                UserFrequencyService.deleteDatabase()
                NextWordService.clearAllAssociations(this@SettingsMainActivity)
                resetCounter++
                Toast
                    .makeText(
                        this@SettingsMainActivity,
                        languageManager.text(Tab4Texts.resetSuccess),
                        Toast.LENGTH_SHORT,
                    ).show()
            } catch (_: Exception) {
                Toast
                    .makeText(
                        this@SettingsMainActivity,
                        languageManager.text(Tab4Texts.resetFailed),
                        Toast.LENGTH_SHORT,
                    ).show()
            }
        }
    }

    private fun updateLauncherIconStatus() {
        if (prefs.showAppIcon) {
            PackageManagerUtils.showAppIcon(this)
        } else {
            PackageManagerUtils.hideAppIcon(this)
        }
    }

    override fun onPause() {
        updateLauncherIconStatus()
        super.onPause()
    }
}
