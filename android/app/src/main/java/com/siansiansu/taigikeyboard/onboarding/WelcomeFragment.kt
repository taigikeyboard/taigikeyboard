package com.siansiansu.taigikeyboard.onboarding

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.TextView
import androidx.fragment.app.Fragment
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.settings.AppTexts
import com.siansiansu.taigikeyboard.settings.LanguageManager
import com.siansiansu.taigikeyboard.settings.setLocalizedText
import com.siansiansu.taigikeyboard.util.FontUtils

/**
 * Onboarding 歡迎頁面
 */
class WelcomeFragment : Fragment() {

    private lateinit var languageManager: LanguageManager
    private lateinit var prefs: PrefHelper

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View? {
        return inflater.inflate(R.layout.fragment_welcome, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        prefs = PrefHelper(requireContext())
        languageManager = LanguageManager.getInstance(requireContext())

        // 設定標題和訊息文字
        val titleTextView = view.findViewById<TextView>(R.id.onboarding_welcome_title)
        val messageTextView = view.findViewById<TextView>(R.id.onboarding_welcome_message)
        val startButton = view.findViewById<Button>(R.id.btn_start_setup)

        titleTextView.setLocalizedText(AppTexts.onboardingWelcomeTitle, languageManager, viewLifecycleOwner)
        messageTextView.setLocalizedText(AppTexts.onboardingWelcomeMessage, languageManager, viewLifecycleOwner)
        startButton.setLocalizedText(AppTexts.onboardingStartSetup, languageManager, viewLifecycleOwner)

        // 套用自訂字體
        val typeface = FontUtils.getTypefaceByType(prefs.fontType, requireContext())
        titleTextView.typeface = typeface
        messageTextView.typeface = typeface
        startButton.typeface = typeface

        startButton.setOnClickListener {
            (activity as? OnboardingActivity)?.navigateToSetup()
        }
    }
}
