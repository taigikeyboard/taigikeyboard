package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.content.SharedPreferences
import android.net.Uri
import android.os.Bundle
import android.widget.Toast
import androidx.activity.compose.setContent
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.lifecycleScope
import androidx.preference.PreferenceManager
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.SubtypeManager
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.localization.Tab4Texts
import com.siansiansu.taigikeyboard.ui.settings.DictionarySettingsScreen
import com.siansiansu.taigikeyboard.ui.settings.HomeScreen
import com.siansiansu.taigikeyboard.ui.settings.InputSettingsScreen
import com.siansiansu.taigikeyboard.ui.settings.LayoutScreen
import com.siansiansu.taigikeyboard.ui.settings.MainSettingsScreen
import com.siansiansu.taigikeyboard.ui.settings.TabItem
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.AppVersionUtils
import com.siansiansu.taigikeyboard.util.PackageManagerUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge
import kotlinx.coroutines.launch

class SettingsMainActivity : AppCompatActivity(),
    SharedPreferences.OnSharedPreferenceChangeListener {

    companion object {
        const val EXTRA_START_TAB = "extra_start_tab"

        // Tab indices
        private const val TAB_HOME = 0
        private const val TAB_LAYOUT = 1
        private const val TAB_DICTIONARY = 2
        private const val TAB_SETTINGS = 3
    }

    lateinit var prefs: PrefHelper
    lateinit var subtypeManager: SubtypeManager

    private var resetCounter by mutableIntStateOf(0)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)
        prefs.initDefaultPreferences()

        // Check if keyboard is enabled; show setup guide if not
        if (!TaigiKeyboard.checkIfImeIsEnabled(this)) {
            startActivity(SetupGuideActivity.createIntent(this, isFullScreen = true))
        }

        subtypeManager = SubtypeManager(this, prefs)

        val mode = when (prefs.settingsTheme) {
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

        val versionName = try {
            packageManager.getPackageInfo(packageName, 0).versionName ?: "1.0"
        } catch (e: Exception) {
            "1.0"
        }

        setContent {
            TaigiKeyboardTheme {
                MainSettingsScreen(
                    tabs = listOf(
                        TabItem(R.drawable.ic_home, getString(R.string.tab_home)),
                        TabItem(R.drawable.keyboard_24, getString(R.string.tab_layout)),
                        TabItem(R.drawable.dictionary_24, getString(R.string.tab_dictionary)),
                        TabItem(R.drawable.ic_settings, getString(R.string.tab_settings))
                    ),
                    initialTab = initialTab
                ) { selectedTab ->
                    when (selectedTab) {
                        TAB_HOME -> HomeScreen(
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
                                    "contact_us", "feedback",
                                    arrayOf("feedback_description", "feedback_email")
                                )
                            },
                            onVersionHistory = {
                                openDetailActivity("version_history", "version", emptyArray())
                            },
                            onFaqClick = { titleKey, contentKeys ->
                                openDetailActivity(titleKey, "faq", contentKeys)
                            }
                        )

                        TAB_LAYOUT -> LayoutScreen(
                            languageManager = languageManager,
                            prefs = prefs,
                            onAppearanceSettings = {
                                startActivity(
                                    Intent(this, AppearanceSettingsActivity::class.java)
                                )
                            }
                        )

                        TAB_DICTIONARY -> DictionarySettingsScreen(
                            languageManager = languageManager,
                            prefs = prefs,
                            onClearCache = ::clearUserFrequencyDatabase,
                            onCustomDictionary = {
                                startActivity(CustomDictionaryActivity.createIntent(this))
                            }
                        )

                        TAB_SETTINGS -> InputSettingsScreen(
                            languageManager = languageManager,
                            prefs = prefs,
                            onResetSettings = ::resetAllSettings,
                            onNavigateToDebug = {
                                startActivity(Intent(this, DebugActivity::class.java))
                            },
                            isDebugBuild = BuildConfig.DEBUG,
                            resetCounter = resetCounter
                        )
                    }
                }
            }
        }
    }

    private fun openDetailActivity(
        titleKey: String,
        contentType: String,
        contentKeys: Array<String>
    ) {
        startActivity(
            DetailActivity.createIntent(this, titleKey, contentType, contentKeys)
        )
    }

    private fun openUrl(url: String) {
        try {
            startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
        } catch (_: Exception) {
            // Handle exception
        }
    }

    private fun clearUserFrequencyDatabase() {
        val languageManager = LanguageManager.getInstance(this)
        lifecycleScope.launch {
            try {
                UserFrequencyService.deleteDatabase()
                NextWordService.clearAllAssociations(this@SettingsMainActivity)
                Toast.makeText(
                    this@SettingsMainActivity,
                    languageManager.text(Tab3Texts.clearCacheSuccess),
                    Toast.LENGTH_SHORT
                ).show()
            } catch (_: Exception) {
                // Handle exception silently
            }
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
                Toast.makeText(
                    this@SettingsMainActivity,
                    languageManager.text(Tab4Texts.resetSuccess),
                    Toast.LENGTH_SHORT
                ).show()
            } catch (_: Exception) {
                // Handle exception silently
            }
        }
    }

    override fun onSharedPreferenceChanged(sp: SharedPreferences?, key: String?) {
        if (key == "advanced__settings_theme") {
            recreate()
        }
    }

    private fun updateLauncherIconStatus() {
        if (prefs.showAppIcon) {
            PackageManagerUtils.showAppIcon(this)
        } else {
            PackageManagerUtils.hideAppIcon(this)
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        // Note: tab switching via intent is handled by Compose's rememberSaveable
    }

    override fun onResume() {
        PreferenceManager.getDefaultSharedPreferences(this)
            .registerOnSharedPreferenceChangeListener(this)
        super.onResume()
    }

    override fun onPause() {
        PreferenceManager.getDefaultSharedPreferences(this)
            .unregisterOnSharedPreferenceChangeListener(this)
        updateLauncherIconStatus()
        super.onPause()
    }

    override fun onDestroy() {
        PreferenceManager.getDefaultSharedPreferences(this)
            .unregisterOnSharedPreferenceChangeListener(this)
        updateLauncherIconStatus()
        super.onDestroy()
    }

}
