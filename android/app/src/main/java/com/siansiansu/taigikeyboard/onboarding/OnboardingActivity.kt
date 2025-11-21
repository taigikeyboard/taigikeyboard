package com.siansiansu.taigikeyboard.onboarding

import android.content.Intent
import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.fragment.app.Fragment
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.settings.SettingsMainActivity
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

/**
 * Onboarding 引導頁面
 * 首次安裝時顯示，引導使用者啟用鍵盤和設定權限
 */
class OnboardingActivity : AppCompatActivity() {

    private lateinit var prefs: PrefHelper

    override fun onCreate(savedInstanceState: Bundle?) {
        prefs = PrefHelper(this)
        prefs.initDefaultPreferences()

        // 套用主題設定
        val mode = when (prefs.settingsTheme) {
            "light" -> AppCompatDelegate.MODE_NIGHT_NO
            "dark" -> AppCompatDelegate.MODE_NIGHT_YES
            "auto" -> AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM
            else -> AppCompatDelegate.MODE_NIGHT_UNSPECIFIED
        }
        AppCompatDelegate.setDefaultNightMode(mode)

        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_onboarding)

        // 設定 Edge-to-Edge 顯示模式
        setupEdgeToEdge()

        // 檢查要顯示哪個頁面
        val showCompleted = intent.getBooleanExtra("show_completed", false)
        val showSetup = intent.getBooleanExtra("show_setup", false)

        // 第一次載入顯示對應頁面
        if (savedInstanceState == null) {
            val fragment = when {
                showCompleted -> CompletedFragment()
                showSetup -> SetupFragment()
                else -> WelcomeFragment()
            }
            supportFragmentManager.beginTransaction()
                .replace(R.id.onboarding_container, fragment)
                .commit()
        }
    }

    /**
     * 導航到設定頁面
     */
    fun navigateToSetup() {
        supportFragmentManager.beginTransaction()
            .replace(R.id.onboarding_container, SetupFragment())
            .addToBackStack(null)
            .commit()
    }

    /**
     * 導航到完成頁面
     */
    fun navigateToCompleted() {
        supportFragmentManager.beginTransaction()
            .replace(R.id.onboarding_container, CompletedFragment())
            .commit()
    }

    /**
     * 完成 onboarding，進入主頁面
     * 使用 suspend function 確保 hasSeenOnboarding 寫入完成後再導航
     */
    suspend fun completeOnboarding() {
        prefs.setHasSeenOnboarding(true)

        val intent = Intent(this, SettingsMainActivity::class.java)
        intent.flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
        startActivity(intent)
        finish()
    }
}
