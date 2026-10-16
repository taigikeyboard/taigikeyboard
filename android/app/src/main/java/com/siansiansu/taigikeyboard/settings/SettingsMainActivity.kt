package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.os.Bundle
import android.widget.Toast
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.core.net.toUri
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.content.ContentType
import com.siansiansu.taigikeyboard.i18n.LocalStringResolver
import com.siansiansu.taigikeyboard.i18n.ProvideDisplayLanguage
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.ime.core.AppVersionTracker
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import com.siansiansu.taigikeyboard.ui.setupEdgeToEdge
import com.siansiansu.taigikeyboard.ui.tabs.MainSettingsScreen
import com.siansiansu.taigikeyboard.ui.tabs.TabItem
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.DictionarySettingsScreen
import com.siansiansu.taigikeyboard.ui.tabs.home.HomeScreen
import com.siansiansu.taigikeyboard.ui.tabs.layout.LayoutScreen
import com.siansiansu.taigikeyboard.ui.tabs.settings.DiagnosticViewModel
import com.siansiansu.taigikeyboard.ui.tabs.settings.InputSettingsScreen
import com.siansiansu.taigikeyboard.ui.tabs.settings.SettingsResetViewModel
import com.siansiansu.taigikeyboard.ui.tabs.theme.ThemePickerScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme

// Main settings host — tabbed UI for home, theme, layout, dictionary, and input settings
class SettingsMainActivity : AppCompatActivity() {
    companion object {
        const val EXTRA_START_TAB = "extra_start_tab"

        private const val TAB_HOME = 0
        private const val TAB_THEME = 1
        private const val TAB_LAYOUT = 2
        private const val TAB_DICTIONARY = 3
        private const val TAB_SETTINGS = 4

        private const val FALLBACK_VERSION = "1.0"
    }

    lateinit var prefs: PrefHelper

    private val diagnosticViewModel: DiagnosticViewModel by viewModels()
    private val resetViewModel: SettingsResetViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)

        // If IME is not enabled, launch setup guide (activity continues to render main UI). Only on a
        // fresh start: a recreated activity (rotation, theme change) already has the guide on top.
        if (savedInstanceState == null && !TaigiKeyboard.checkIfImeIsEnabled(this)) {
            startActivity(SetupGuideActivity.createIntent(this, isFullScreen = true))
        }

        AppCompatDelegate.setDefaultNightMode(AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM)

        setupEdgeToEdge()

        AppVersionTracker.updateVersionOnInstallAndLastUse(this, prefs)

        // Clamp to the valid tab range — the Activity is exported, so a stale or
        // out-of-range EXTRA_START_TAB from an external launcher must never land on a
        // blank tab. With a valid initialTab, the bottom-bar only ever selects valid
        // indices, so the when(selectedTab) dispatch below stays total.
        val initialTab = intent.getIntExtra(EXTRA_START_TAB, TAB_HOME).coerceIn(TAB_HOME, TAB_SETTINGS)

        val versionName =
            try {
                packageManager.getPackageInfo(packageName, 0).versionName ?: FALLBACK_VERSION
            } catch (_: Exception) {
                FALLBACK_VERSION
            }

        setContent {
            val resetCounter by resetViewModel.resetCounter.collectAsStateWithLifecycle()
            // i18n live-switch root: when the host Settings language picker changes prefs.displayLanguageTag,
            // re-provide this subtree and recompose every stringRes consumer — no Activity recreate.
            ProvideDisplayLanguage(prefs) {
                TaigiKeyboardTheme {
                    MainSettingsScreen(
                        tabs =
                            listOf(
                                TabItem(R.drawable.ic_home, StringKey.NAV_TAB_HOME),
                                TabItem(R.drawable.ic_palette, StringKey.NAV_TAB_THEME),
                                TabItem(R.drawable.keyboard_24, StringKey.NAV_TAB_LAYOUT),
                                TabItem(R.drawable.dictionary_24, StringKey.NAV_TAB_DICTIONARY),
                                TabItem(R.drawable.ic_settings, StringKey.COMMON_SETTINGS),
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
                                    onFaqClick = { titleKey, contentKeys ->
                                        openDetailActivity(titleKey, ContentType.FAQ, contentKeys)
                                    },
                                )
                            }

                            TAB_THEME -> {
                                ThemePickerScreen(prefs = prefs)
                            }

                            TAB_LAYOUT -> {
                                LayoutScreen(prefs = prefs)
                            }

                            TAB_DICTIONARY -> {
                                DictionarySettingsScreen(
                                    onManageDictionaries = {
                                        startActivity(DictionarySourcesActivity.createIntent(this))
                                    },
                                    onCustomDictionary = {
                                        startActivity(CustomDictionaryActivity.createIntent(this))
                                    },
                                    onLearningRecords = {
                                        startActivity(LearningRecordsMenuActivity.createIntent(this))
                                    },
                                    onBackupRestore = {
                                        startActivity(DataManagementActivity.createIntent(this))
                                    },
                                )
                            }

                            TAB_SETTINGS -> {
                                // Failure toast fires from a ViewModel completion callback — capture the
                                // live resolver here, same seam as the R2a-2 dictionary screens.
                                val stringResolver by rememberUpdatedState(LocalStringResolver.current)
                                InputSettingsScreen(
                                    prefs = prefs,
                                    diagnosticViewModel = diagnosticViewModel,
                                    onResetSettings = {
                                        // Only a failure is said; a reset that landed shows in the
                                        // rows themselves (USER 2026-10-16).
                                        resetViewModel.resetAllSettings(prefs) { success ->
                                            if (!success) {
                                                Toast
                                                    .makeText(
                                                        this,
                                                        stringResolver.resolve(StringKey.SETTINGS_RESET_FAILED),
                                                        Toast.LENGTH_SHORT,
                                                    ).show()
                                            }
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
}
