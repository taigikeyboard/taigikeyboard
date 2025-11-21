
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
        binding.root.findViewById<TextView>(R.id.resource_sponsorship_title)?.text =
            languageManager.getText(AppTexts.sponsorship)
        binding.root.findViewById<TextView>(R.id.resource_contact_title)?.text =
            languageManager.getText(AppTexts.contactUs)
        binding.root.findViewById<TextView>(R.id.resource_rate_title)?.text =
            languageManager.getText(AppTexts.rateUs)
        binding.root.findViewById<TextView>(R.id.resource_share_title)?.text =
            languageManager.getText(AppTexts.shareToFriends)
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

        // Website Introduction
        binding.root.findViewById<View>(R.id.nav_item_website_intro)?.setOnClickListener {
            Intent(context, WebsiteIntroActivity::class.java).apply {
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
        // Sponsorship
        binding.root.findViewById<View>(R.id.resource_item_sponsorship)?.setOnClickListener {
            Intent(context, com.siansiansu.taigikeyboard.sponsorship.SponsorshipActivity::class.java).apply {
                startActivity(this)
            }
        }

        // Contact / Feedback
        binding.root.findViewById<View>(R.id.resource_item_contact)?.setOnClickListener {
            Intent(context, ContactActivity::class.java).apply {
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

        // Share
        binding.root.findViewById<View>(R.id.resource_item_share)?.setOnClickListener {
            val shareIntent = Intent(Intent.ACTION_SEND).apply {
                type = "text/plain"
                putExtra(Intent.EXTRA_SUBJECT, getString(R.string.home__share__title))
                putExtra(Intent.EXTRA_TEXT, "${getString(R.string.home__share__title)}\n${getString(R.string.home__share__url)}")
            }
            startActivity(Intent.createChooser(shareIntent, getString(R.string.home__resource__share)))
        }
    }
}
