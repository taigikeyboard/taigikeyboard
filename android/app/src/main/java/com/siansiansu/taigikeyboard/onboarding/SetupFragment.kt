package com.siansiansu.taigikeyboard.onboarding

import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.TextView
import androidx.fragment.app.Fragment
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.settings.AppTexts
import com.siansiansu.taigikeyboard.settings.LanguageManager
import com.siansiansu.taigikeyboard.settings.setLocalizedText

/**
 * Onboarding 設定頁面
 * 顯示設定步驟和狀態檢查
 */
class SetupFragment : Fragment() {

    private lateinit var languageManager: LanguageManager
    private lateinit var btnGoToSettings: Button
    private lateinit var privacyMessageTextView: TextView

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View? {
        return inflater.inflate(R.layout.fragment_setup, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        languageManager = LanguageManager.getInstance(requireContext())

        // 初始化 views
        val titleTextView = view.findViewById<TextView>(R.id.onboarding_setup_title)
        privacyMessageTextView = view.findViewById(R.id.privacy_message)
        btnGoToSettings = view.findViewById(R.id.btn_go_to_settings)

        // 設定文字
        titleTextView.setLocalizedText(AppTexts.onboardingAddKeyboardTitle, languageManager, viewLifecycleOwner)
        privacyMessageTextView.setLocalizedText(AppTexts.setupInfoMessage, languageManager, viewLifecycleOwner)
        btnGoToSettings.setLocalizedText(AppTexts.onboardingGoToSettings, languageManager, viewLifecycleOwner)

        btnGoToSettings.setOnClickListener {
            Intent(Settings.ACTION_INPUT_METHOD_SETTINGS).apply {
                startActivity(this)
            }
        }

        updateStatus()
    }

    override fun onResume() {
        super.onResume()
        updateStatus()
    }

    /**
     * 檢查鍵盤狀態並自動導航
     */
    private fun updateStatus() {
        val context = requireContext()
        val isEnabled = TaigiKeyboard.checkIfImeIsEnabled(context)

        // 如果鍵盤已啟用，自動跳轉到完成頁面
        if (isEnabled) {
            (activity as? OnboardingActivity)?.navigateToCompleted()
        }
    }
}
