
package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.Settings
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import android.widget.Toast
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.databinding.FragmentContentBinding
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.util.AppVersionUtils

class ContentFragment : SettingsMainActivity.SettingsFragment() {
    private lateinit var binding: FragmentContentBinding
    private lateinit var languageManager: LanguageManager

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View {
        binding = FragmentContentBinding.inflate(inflater, container, false)
        languageManager = LanguageManager.getInstance(requireContext())

        setupNavigationItems()
        setupResourceItems()
        observeLanguageChanges()
        setupFooterVersion()

        // Ensure initial display is correct
        updateAllTexts()

        return binding.root
    }

    private fun observeLanguageChanges() {
        languageManager.currentDisplayLanguage.observe(viewLifecycleOwner) {
            updateAllTexts()
        }
    }

    private fun updateAllTexts() {
        // Update navigation items
        binding.root.findViewById<TextView>(R.id.nav_setup_title)?.text =
            languageManager.getText(AppTexts.setupGuide)
        binding.root.findViewById<TextView>(R.id.nav_settings_title)?.text =
            languageManager.getText(AppTexts.keyboardSettings)
        binding.root.findViewById<TextView>(R.id.nav_website_intro_title)?.text =
            languageManager.getText(AppTexts.websiteIntro)
        binding.root.findViewById<TextView>(R.id.nav_copyright_title)?.text =
            languageManager.getText(AppTexts.copyrightNotice)

        // Update resource items
        binding.root.findViewById<TextView>(R.id.resource_contact_title)?.text =
            languageManager.getText(AppTexts.contactUs)
        binding.root.findViewById<TextView>(R.id.resource_rate_title)?.text =
            languageManager.getText(AppTexts.rateUs)
    }

    private fun setupNavigationItems() {
        // Setup Guide
        binding.root.findViewById<View>(R.id.nav_item_setup)?.setOnClickListener {
            // 檢查鍵盤是否已啟用
            val isEnabled = TaigiKeyboard.checkIfImeIsEnabled(requireContext())

            val intent = Intent(context, com.siansiansu.taigikeyboard.onboarding.OnboardingActivity::class.java)
            if (isEnabled) {
                // 已啟用，直接顯示完成頁面
                intent.putExtra("show_completed", true)
            } else {
                // 未啟用，顯示設定頁面
                intent.putExtra("show_setup", true)
            }
            startActivity(intent)
        }

        // Keyboard Settings
        binding.root.findViewById<View>(R.id.nav_item_settings)?.setOnClickListener {
            Intent(context, KeyboardSettingsActivity::class.java).apply {
                startActivity(this)
            }
        }

        // Website Introduction - 使用系統預設瀏覽器開啟
        binding.root.findViewById<View>(R.id.nav_item_website_intro)?.setOnClickListener {
            val websiteUrl = "https://www.taigikeyboard.tw/"
            Intent(Intent.ACTION_VIEW, Uri.parse(websiteUrl)).apply {
                startActivity(this)
            }
        }

        // Copyright
        binding.root.findViewById<View>(R.id.nav_item_copyright)?.setOnClickListener {
            Intent(context, CopyrightActivity::class.java).apply {
                startActivity(this)
            }
        }
    }

    private fun setupResourceItems() {
        // Contact / Feedback - 使用系統預設瀏覽器開啟 Google Forms
        binding.root.findViewById<View>(R.id.resource_item_contact)?.setOnClickListener {
            val contactFormUrl = "https://docs.google.com/forms/d/e/1FAIpQLSd7PEppQ9MdAptvoY-PaaXDlbbL9Gq9Y4lFjgU9sLz4ENiPoA/viewform?usp=header"
            Intent(Intent.ACTION_VIEW, Uri.parse(contactFormUrl)).apply {
                startActivity(this)
            }
        }

        // Rate
        binding.root.findViewById<View>(R.id.resource_item_rate)?.setOnClickListener {
            val packageName = requireContext().packageName
            try {
                // Try to open Google Play app
                Intent(Intent.ACTION_VIEW, Uri.parse("market://details?id=$packageName")).apply {
                    startActivity(this)
                }
            } catch (e: Exception) {
                // Fallback to browser
                Intent(
                    Intent.ACTION_VIEW,
                    Uri.parse("https://play.google.com/store/apps/details?id=$packageName")
                ).apply {
                    startActivity(this)
                }
            }
        }
    }

    private fun setupFooterVersion() {
        // 設定版號顯示
        binding.root.findViewById<TextView>(R.id.footer_version)?.apply {
            val version = AppVersionUtils.getRawVersionName(requireContext())
            text = "v$version"
        }
    }
}
