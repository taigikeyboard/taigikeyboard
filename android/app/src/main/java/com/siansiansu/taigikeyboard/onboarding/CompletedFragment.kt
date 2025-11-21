package com.siansiansu.taigikeyboard.onboarding

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.TextView
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.settings.AppTexts
import com.siansiansu.taigikeyboard.settings.LanguageManager
import com.siansiansu.taigikeyboard.settings.setLocalizedText
import kotlinx.coroutines.launch

/**
 * Onboarding 完成頁面
 */
class CompletedFragment : Fragment() {

    private lateinit var languageManager: LanguageManager

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View? {
        return inflater.inflate(R.layout.fragment_completed, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        languageManager = LanguageManager.getInstance(requireContext())

        // 設定標題、訊息和按鈕文字
        val titleTextView = view.findViewById<TextView>(R.id.onboarding_completed_title)
        val messageTextView = view.findViewById<TextView>(R.id.onboarding_completed_message)
        val btnGetStarted = view.findViewById<Button>(R.id.btn_get_started)

        titleTextView.setLocalizedText(AppTexts.onboardingCompletedTitle, languageManager, viewLifecycleOwner)
        messageTextView.setLocalizedText(AppTexts.onboardingCompletedMessage, languageManager, viewLifecycleOwner)
        btnGetStarted.setLocalizedText(AppTexts.onboardingGetStarted, languageManager, viewLifecycleOwner)

        btnGetStarted.setOnClickListener {
            // 使用 lifecycleScope 確保 suspend function 正確執行
            lifecycleScope.launch {
                (activity as? OnboardingActivity)?.completeOnboarding()
            }
        }
    }
}
