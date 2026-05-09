// 中文: Settings App 主 Activity — 安裝 EdgeToEdge、套主題、setContent 掛載 MainSettingsScreen。
// 中文: 也是 dictionary / diagnostic / settings-reset 等 ViewModel 的 owner。

package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.os.Bundle
import android.widget.Toast
import androidx.core.net.toUri
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.content.ContentType
import com.siansiansu.taigikeyboard.ime.core.AppVersionTracker
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.localization.SettingsTexts
import com.siansiansu.taigikeyboard.ui.setupEdgeToEdge
import com.siansiansu.taigikeyboard.ui.tabs.MainSettingsScreen
import com.siansiansu.taigikeyboard.ui.tabs.TabItem
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.DictionarySearchViewModel
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.DictionarySettingsScreen
import com.siansiansu.taigikeyboard.ui.tabs.home.HomeScreen
import com.siansiansu.taigikeyboard.ui.tabs.layout.LayoutScreen
import com.siansiansu.taigikeyboard.ui.tabs.settings.DiagnosticViewModel
import com.siansiansu.taigikeyboard.ui.tabs.settings.InputSettingsScreen
import com.siansiansu.taigikeyboard.ui.tabs.settings.SettingsResetViewModel
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme

// Main settings host — tabbed UI for home, layout, dictionary, and input settings
class SettingsMainActivity : AppCompatActivity() {
    companion object {
        const val EXTRA_START_TAB = "extra_start_tab"

        private const val TAB_HOME = 0
        private const val TAB_LAYOUT = 1
        private const val TAB_DICTIONARY = 2
        private const val TAB_SETTINGS = 3

        private const val THEME_LIGHT = "light"
        private const val THEME_DARK = "dark"
        private const val THEME_AUTO = "auto"

        private const val FALLBACK_VERSION = "1.0"
    }

    lateinit var prefs: PrefHelper

    private val searchViewModel: DictionarySearchViewModel by viewModels()
    private val diagnosticViewModel: DiagnosticViewModel by viewModels()
    private val resetViewModel: SettingsResetViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)

        // If IME is not enabled, launch setup guide (activity continues to render main UI)
        if (!TaigiKeyboard.checkIfImeIsEnabled(this)) {
            startActivity(SetupGuideActivity.createIntent(this, isFullScreen = true))
        }

        val mode =
            when (prefs.settingsTheme) {
                THEME_LIGHT -> AppCompatDelegate.MODE_NIGHT_NO
                THEME_DARK -> AppCompatDelegate.MODE_NIGHT_YES
                THEME_AUTO -> AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM
                else -> AppCompatDelegate.MODE_NIGHT_UNSPECIFIED
            }
        AppCompatDelegate.setDefaultNightMode(mode)

        setupEdgeToEdge()

        AppVersionTracker.updateVersionOnInstallAndLastUse(this, prefs)

        val initialTab = intent.getIntExtra(EXTRA_START_TAB, TAB_HOME)

        val versionName =
            try {
                packageManager.getPackageInfo(packageName, 0).versionName ?: FALLBACK_VERSION
            } catch (_: Exception) {
                FALLBACK_VERSION
            }

        setContent {
            val resetCounter by resetViewModel.resetCounter.collectAsStateWithLifecycle()
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
                                onAboutDeveloper = {
                                    openDetailActivity(
                                        ContentType.KEY_ABOUT_DEVELOPER,
                                        ContentType.ABOUT_DEVELOPER,
                                        emptyArray(),
                                    )
                                },
                                onVersionHistory = {
                                    openDetailActivity(ContentType.KEY_VERSION_HISTORY, ContentType.VERSION, emptyArray())
                                },
                                onFaqClick = { titleKey, contentKeys ->
                                    openDetailActivity(titleKey, ContentType.FAQ, contentKeys)
                                },
                            )
                        }

                        TAB_LAYOUT -> {
                            LayoutScreen(
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
                                resetCounter = resetCounter,
                            )
                        }

                        TAB_SETTINGS -> {
                            InputSettingsScreen(
                                prefs = prefs,
                                diagnosticViewModel = diagnosticViewModel,
                                onResetSettings = {
                                    resetViewModel.resetAllSettings(prefs) { success ->
                                        val message = if (success) SettingsTexts.resetSuccess else SettingsTexts.resetFailed
                                        Toast
                                            .makeText(this, message, Toast.LENGTH_SHORT)
                                            .show()
                                    }
                                },
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
            startActivity(Intent(Intent.ACTION_VIEW, url.toUri()))
        } catch (_: Exception) {
            // No browser available to handle the URL
        }
    }

    private fun updateLauncherIconStatus() {
        if (prefs.showAppIcon) {
            LauncherIconController.showAppIcon(this)
        } else {
            LauncherIconController.hideAppIcon(this)
        }
    }

    override fun onPause() {
        updateLauncherIconStatus()
        super.onPause()
    }
}
