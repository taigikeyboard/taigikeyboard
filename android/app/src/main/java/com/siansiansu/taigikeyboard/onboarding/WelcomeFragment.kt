package com.siansiansu.taigikeyboard.onboarding

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.TextView
import androidx.fragment.app.Fragment
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.settings.AppTexts
import com.siansiansu.taigikeyboard.settings.LanguageManager
import com.siansiansu.taigikeyboard.settings.setLocalizedText

/**
 * Onboarding 歡迎頁面
 */
class WelcomeFragment : Fragment() {

    private lateinit var languageManager: LanguageManager

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View? {
        return inflater.inflate(R.layout.fragment_welcome, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        languageManager = LanguageManager.getInstance(requireContext())

        // 設定標題和訊息文字
        val titleTextView = view.findViewById<TextView>(R.id.onboarding_welcome_title)
        val messageTextView = view.findViewById<TextView>(R.id.onboarding_welcome_message)
        val startButton = view.findViewById<Button>(R.id.btn_start_setup)

        titleTextView.setLocalizedText(AppTexts.onboardingWelcomeTitle, languageManager, viewLifecycleOwner)
        messageTextView.setLocalizedText(AppTexts.onboardingWelcomeMessage, languageManager, viewLifecycleOwner)
        startButton.setLocalizedText(AppTexts.onboardingStartSetup, languageManager, viewLifecycleOwner)

        startButton.setOnClickListener {
            (activity as? OnboardingActivity)?.navigateToSetup()
        }
    }
}
