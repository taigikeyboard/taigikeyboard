
package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.content.SharedPreferences
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updateLayoutParams
import androidx.fragment.app.Fragment
import androidx.preference.PreferenceFragmentCompat
import androidx.preference.PreferenceManager
import com.google.android.material.bottomnavigation.BottomNavigationView
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.SubtypeManager
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.util.AppVersionUtils
import com.siansiansu.taigikeyboard.util.PackageManagerUtils
import com.siansiansu.taigikeyboard.util.ThemeUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

private const val PREF_RES_ID = "PREF_RES_ID"

class SettingsMainActivity : AppCompatActivity(),
    SharedPreferences.OnSharedPreferenceChangeListener {

    companion object {
        const val EXTRA_START_TAB = "extra_start_tab"
    }

    lateinit var prefs: PrefHelper
    lateinit var subtypeManager: SubtypeManager

    private lateinit var bottomNavigation: BottomNavigationView

    // 緩存 Fragment 實例
    private val tab1Fragment by lazy { Tab1Fragment.newInstance() }
    private val tab2Fragment by lazy { Tab2Fragment.newInstance() }
    private val tab3Fragment by lazy { Tab3Fragment.newInstance() }
    private val tab4Fragment by lazy { Tab4Fragment.newInstance() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)
        prefs.initDefaultPreferences()

        // 套用字體 Theme（必須在 setContentView 之前）
        ThemeUtils.applyFontTheme(this, prefs)

        // 檢查鍵盤是否已啟用，若未啟用則顯示設定引導（與 iOS 一致）
        val isKeyboardEnabled = TaigiKeyboard.checkIfImeIsEnabled(this)
        if (!isKeyboardEnabled) {
            val intent = SetupGuideActivity.createIntent(this, isFullScreen = true)
            startActivity(intent)
            // 不 finish()，讓用戶完成設定後返回
        }

        subtypeManager = SubtypeManager(this, prefs)

        val mode = when (prefs.settingsTheme) {
            "light" -> AppCompatDelegate.MODE_NIGHT_NO
            "dark" -> AppCompatDelegate.MODE_NIGHT_YES
            "auto" -> AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM
            else -> AppCompatDelegate.MODE_NIGHT_UNSPECIFIED
        }
        AppCompatDelegate.setDefaultNightMode(mode)

        setContentView(R.layout.activity_main_tabs)

        // 設定 Edge-to-Edge 顯示模式
        setupEdgeToEdge()

        AppVersionUtils.updateVersionOnInstallAndLastUse(this, prefs)

        // 設定底部導航
        setupBottomNavigation()

        // 載入預設 Fragment（頭頁）
        if (savedInstanceState == null) {
            val startTab = intent.getIntExtra(EXTRA_START_TAB, R.id.nav_home)
            if (startTab == R.id.nav_home) {
                loadFragment(tab1Fragment)
            } else {
                handleStartTab(intent)
            }
        }
    }

    private fun setupBottomNavigation() {
        bottomNavigation = findViewById(R.id.bottom_navigation)
        val fragmentContainer = findViewById<FrameLayout>(R.id.fragment_container)

        // 處理系統導航列的 insets，避免 BottomNavigationView 被遮住
        ViewCompat.setOnApplyWindowInsetsListener(bottomNavigation) { view, windowInsets ->
            val insets = windowInsets.getInsets(WindowInsetsCompat.Type.systemBars())

            // 使用 margin 而非 padding，避免影響內部 label 顯示
            view.updateLayoutParams<ViewGroup.MarginLayoutParams> {
                bottomMargin = insets.bottom
            }

            // 同時更新 fragment_container 的 bottom margin
            val bottomNavHeight = resources.getDimensionPixelSize(R.dimen.bottom_nav_height)
            fragmentContainer.updateLayoutParams<ViewGroup.MarginLayoutParams> {
                bottomMargin = bottomNavHeight + insets.bottom
            }

            windowInsets
        }

        bottomNavigation.setOnItemSelectedListener { item ->
            when (item.itemId) {
                R.id.nav_home -> {
                    loadFragment(tab1Fragment)
                    true
                }
                R.id.nav_layout -> {
                    loadFragment(tab2Fragment)
                    true
                }
                R.id.nav_dictionary -> {
                    loadFragment(tab3Fragment)
                    true
                }
                R.id.nav_settings -> {
                    loadFragment(tab4Fragment)
                    true
                }
                else -> false
            }
        }
    }

    private fun loadFragment(fragment: Fragment) {
        supportFragmentManager.beginTransaction()
            .replace(R.id.fragment_container, fragment)
            .commit()
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
        handleStartTab(intent)
    }

    private fun handleStartTab(intent: Intent) {
        val startTab = intent.getIntExtra(EXTRA_START_TAB, -1)
        if (startTab != -1) {
            when (startTab) {
                R.id.nav_dictionary -> {
                    loadFragment(tab3Fragment)
                    bottomNavigation.selectedItemId = R.id.nav_dictionary
                }
                R.id.nav_home -> {
                    loadFragment(tab1Fragment)
                    bottomNavigation.selectedItemId = R.id.nav_home
                }
                R.id.nav_layout -> {
                    loadFragment(tab2Fragment)
                    bottomNavigation.selectedItemId = R.id.nav_layout
                }
                R.id.nav_settings -> {
                    loadFragment(tab4Fragment)
                    bottomNavigation.selectedItemId = R.id.nav_settings
                }
            }
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

    abstract class BaseSettingsFragment : Fragment() {
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
