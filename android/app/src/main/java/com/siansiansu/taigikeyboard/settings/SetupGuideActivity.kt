package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.view.View
import android.widget.Button
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding
import com.google.android.material.appbar.AppBarLayout
import com.google.android.material.appbar.MaterialToolbar
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab1Texts
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

/**
 * 啟用方法頁面
 * 共用於 Tab1 子頁面和全螢幕模式（首次啟動時）
 * 透過 isFullScreen 參數控制顯示模式
 */
class SetupGuideActivity : AppCompatActivity() {

    companion object {
        const val EXTRA_IS_FULL_SCREEN = "extra_is_full_screen"

        fun createIntent(context: Context, isFullScreen: Boolean = false): Intent {
            return Intent(context, SetupGuideActivity::class.java).apply {
                putExtra(EXTRA_IS_FULL_SCREEN, isFullScreen)
            }
        }
    }

    private lateinit var languageManager: LanguageManager
    private lateinit var prefs: PrefHelper
    private var isFullScreen: Boolean = false
    private var hasNavigatedToSettings: Boolean = false

    // 關閉時的 callback
    var onComplete: (() -> Unit)? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)
        languageManager = LanguageManager.getInstance(this)
        isFullScreen = intent.getBooleanExtra(EXTRA_IS_FULL_SCREEN, false)

        setContentView(R.layout.activity_setup_guide)

        setupEdgeToEdge()
        setupToolbar()
        setupViews()
    }

    private fun setupToolbar() {
        val appBar = findViewById<AppBarLayout>(R.id.app_bar)
        val toolbar = findViewById<MaterialToolbar>(R.id.toolbar)

        // 設定標題
        toolbar.title = languageManager.text(Tab1Texts.setupGuide)

        if (isFullScreen) {
            // 全螢幕模式：隱藏返回按鈕
            toolbar.navigationIcon = null
        } else {
            // 一般模式：顯示返回按鈕
            toolbar.setNavigationOnClickListener { finish() }
        }

        // 處理系統狀態列 insets
        ViewCompat.setOnApplyWindowInsetsListener(appBar) { view, windowInsets ->
            val insets = windowInsets.getInsets(WindowInsetsCompat.Type.statusBars())
            view.updatePadding(top = insets.top)
            windowInsets
        }
    }

    private fun setupViews() {
        val setupDescription = findViewById<TextView>(R.id.setup_description)
        val step1Title = findViewById<TextView>(R.id.step1_title)
        val step2Title = findViewById<TextView>(R.id.step2_title)
        val completedMessage = findViewById<TextView>(R.id.completed_message)
        val btnGoToSettings = findViewById<Button>(R.id.btn_go_to_settings)
        val privacyMessage = findViewById<TextView>(R.id.privacy_message)
        val brandWarningMessage = findViewById<TextView>(R.id.brand_warning_message)
        val btnClose = findViewById<Button>(R.id.btn_close)

        // 設定文字
        setupDescription.text = languageManager.text(Tab1Texts.setupGuideDescription)
        step1Title.text = languageManager.text(Tab1Texts.setupGuideStep1Settings)
        step2Title.text = languageManager.text(Tab1Texts.setupGuideStep2AddKeyboard)
        completedMessage.text = languageManager.text(Tab1Texts.setupGuideCompletedMessage)
        btnGoToSettings.text = languageManager.text(Tab1Texts.setupGuideGoToSettings)
        privacyMessage.text = languageManager.text(Tab1Texts.setupInfoMessage)
        brandWarningMessage.text = languageManager.text(Tab1Texts.setupBrandWarning)

        // 前往設定按鈕
        btnGoToSettings.setOnClickListener {
            hasNavigatedToSettings = true
            Intent(Settings.ACTION_INPUT_METHOD_SETTINGS).apply {
                startActivity(this)
            }
        }

        // 關閉按鈕（僅全螢幕模式顯示）
        if (isFullScreen) {
            btnClose.visibility = View.VISIBLE
            btnClose.text = languageManager.text(Tab1Texts.setupGuideCloseButton)
            btnClose.setOnClickListener {
                onComplete?.invoke()
                finish()
            }
        } else {
            btnClose.visibility = View.GONE
        }
    }

    override fun onResume() {
        super.onResume()

        // 從系統設定返回後，檢查鍵盤是否已啟用
        if (hasNavigatedToSettings) {
            hasNavigatedToSettings = false
            val isKeyboardEnabled = TaigiKeyboard.checkIfImeIsEnabled(this)
            if (isKeyboardEnabled) {
                // 鍵盤已啟用，關閉設定頁面回到主畫面
                finish()
            }
            // 若未啟用，留在當前頁面讓使用者繼續設定
        }
    }
}
