
package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.content.SharedPreferences
import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.fragment.app.Fragment
import androidx.preference.PreferenceFragmentCompat
import androidx.preference.PreferenceManager
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.SubtypeManager
import com.siansiansu.taigikeyboard.util.AppVersionUtils
import com.siansiansu.taigikeyboard.util.PackageManagerUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

private const val PREF_RES_ID = "PREF_RES_ID"

class SettingsMainActivity : AppCompatActivity(),
    SharedPreferences.OnSharedPreferenceChangeListener {

    lateinit var prefs: PrefHelper
    lateinit var subtypeManager: SubtypeManager

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)
        prefs.initDefaultPreferences()

        // 檢查是否需要顯示 onboarding
        if (!prefs.hasSeenOnboarding) {
            val intent = Intent(this, com.siansiansu.taigikeyboard.onboarding.OnboardingActivity::class.java)
            startActivity(intent)
            finish()
            return
        }

        subtypeManager = SubtypeManager(this, prefs)

        val mode = when (prefs.settingsTheme) {
            "light" -> AppCompatDelegate.MODE_NIGHT_NO
            "dark" -> AppCompatDelegate.MODE_NIGHT_YES
            "auto" -> AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM
            else -> AppCompatDelegate.MODE_NIGHT_UNSPECIFIED
        }
        AppCompatDelegate.setDefaultNightMode(mode)

        setContentView(R.layout.settings_activity)

        // 設定 Edge-to-Edge 顯示模式
        setupEdgeToEdge()

        AppVersionUtils.updateVersionOnInstallAndLastUse(this, prefs)

        // Load ContentFragment as the main screen
        if (savedInstanceState == null) {
            supportFragmentManager.beginTransaction()
                .replace(R.id.fragment_container, ContentFragment())
                .commit()
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

    abstract class SettingsFragment : Fragment() {
        // 使用 lazy 延遲初始化，避免在 Activity 屬性初始化前存取
        protected val settingsMainActivity: SettingsMainActivity by lazy {
            requireActivity() as SettingsMainActivity
        }

        protected val prefs: PrefHelper by lazy {
            settingsMainActivity.prefs
        }

        protected val subtypeManager: SubtypeManager by lazy {
            settingsMainActivity.subtypeManager
        }
    }

    class PrefFragment : PreferenceFragmentCompat() {
        companion object {
            fun createFromResource(prefResId: Int): PrefFragment {
                val args = Bundle()
                args.putInt(PREF_RES_ID, prefResId)
                val fragment = PrefFragment()
                fragment.arguments = args
                return fragment
            }
        }

        override fun onCreatePreferences(savedInstanceState: Bundle?, rootKey: String?) {
            setPreferencesFromResource(arguments?.getInt(PREF_RES_ID) ?: 0, rootKey)
        }
    }
}
