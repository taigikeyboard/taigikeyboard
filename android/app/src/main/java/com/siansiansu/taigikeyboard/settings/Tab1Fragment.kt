package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab1Texts

/**
 * Tab1: 頭頁
 * 內容：啟用方法、新功能、處理中問題、預計新功能、資源連結、FAQ
 */
class Tab1Fragment : SettingsMainActivity.BaseSettingsFragment() {

    private lateinit var languageManager: LanguageManager

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View? {
        return inflater.inflate(R.layout.fragment_tab1, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        languageManager = LanguageManager.getInstance(requireContext())

        setupVersionInfo(view)
        setupLocalizedTexts(view)
        setupClickListeners(view)
    }

    private fun setupLocalizedTexts(view: View) {
        // 頁面標題
        view.findViewById<TextView>(R.id.header_title).text =
            languageManager.text(Tab1Texts.appHeaderTitle)

        // 區塊標題
        view.findViewById<TextView>(R.id.section_setup_keyboard).text =
            languageManager.text(Tab1Texts.setupKeyboard)
        view.findViewById<TextView>(R.id.section_new_features).text =
            languageManager.text(Tab1Texts.newFeatures)
        view.findViewById<TextView>(R.id.section_known_issues).text =
            languageManager.text(Tab1Texts.knownIssues)
        view.findViewById<TextView>(R.id.section_upcoming_features).text =
            languageManager.text(Tab1Texts.upcomingFeatures)
        view.findViewById<TextView>(R.id.section_faq).text =
            languageManager.text(Tab1Texts.faq)

        // 啟用方法
        view.findViewById<TextView>(R.id.text_setup_guide).text =
            languageManager.text(Tab1Texts.setupGuide)

        // 資源連結
        view.findViewById<TextView>(R.id.text_user_guide).text =
            languageManager.text(Tab1Texts.userGuide)
        view.findViewById<TextView>(R.id.text_privacy_policy).text =
            languageManager.text(Tab1Texts.privacyPolicy)
        view.findViewById<TextView>(R.id.text_rate_us).text =
            languageManager.text(Tab1Texts.rateUs)

        // 版權聲明
        view.findViewById<TextView>(R.id.text_copyright).text =
            languageManager.text(Tab1Texts.copyrightNotice)
    }

    private fun setupVersionInfo(view: View) {
        val versionTextView = view.findViewById<TextView>(R.id.text_version)
        try {
            val packageInfo = requireContext().packageManager.getPackageInfo(
                requireContext().packageName, 0
            )
            versionTextView.text = packageInfo.versionName
        } catch (e: Exception) {
            versionTextView.text = "1.0"
        }
    }

    private fun setupClickListeners(view: View) {
        // 啟用鍵盤 - 導航到 SetupGuide 頁面
        view.findViewById<View>(R.id.row_setup_guide).setOnClickListener {
            startActivity(Intent(requireContext(), SetupGuideActivity::class.java))
        }

        // 新功能
        view.findViewById<View>(R.id.row_feature_1).setOnClickListener {
            openDetailActivity(
                titleKey = "feature_next_word",
                contentType = "feature",
                contentKeys = arrayOf("feature_next_word_detail")
            )
        }
        view.findViewById<View>(R.id.row_feature_2).setOnClickListener {
            openDetailActivity(
                titleKey = "feature_variant",
                contentType = "feature",
                contentKeys = arrayOf("feature_variant_detail")
            )
        }
        view.findViewById<View>(R.id.row_feature_3).setOnClickListener {
            openDetailActivity(
                titleKey = "feature_custom_font",
                contentType = "feature",
                contentKeys = arrayOf("feature_custom_font_detail")
            )
        }
        view.findViewById<View>(R.id.row_feature_4).setOnClickListener {
            openDetailActivity(
                titleKey = "feature_user_dict",
                contentType = "feature",
                contentKeys = arrayOf("feature_user_dict_detail")
            )
        }
        view.findViewById<View>(R.id.row_feature_5).setOnClickListener {
            openDetailActivity(
                titleKey = "feature_case_switch",
                contentType = "feature",
                contentKeys = arrayOf("feature_case_switch_detail")
            )
        }

        // 處理中的問題
        view.findViewById<View>(R.id.row_issue_1).setOnClickListener {
            openDetailActivity(
                titleKey = "issue_1",
                contentType = "issue",
                contentKeys = arrayOf("issue_1_detail")
            )
        }
        view.findViewById<View>(R.id.row_issue_2).setOnClickListener {
            openDetailActivity(
                titleKey = "issue_2",
                contentType = "issue",
                contentKeys = arrayOf("issue_2_detail")
            )
        }
        view.findViewById<View>(R.id.row_issue_3).setOnClickListener {
            openDetailActivity(
                titleKey = "issue_3",
                contentType = "issue",
                contentKeys = arrayOf("issue_3_detail")
            )
        }

        // 預計新功能
        view.findViewById<View>(R.id.row_upcoming_1).setOnClickListener {
            openDetailActivity(
                titleKey = "upcoming_1",
                contentType = "upcoming",
                contentKeys = arrayOf("upcoming_1_detail")
            )
        }
        view.findViewById<View>(R.id.row_upcoming_2).setOnClickListener {
            openDetailActivity(
                titleKey = "upcoming_2",
                contentType = "upcoming",
                contentKeys = arrayOf("upcoming_2_detail")
            )
        }
        view.findViewById<View>(R.id.row_upcoming_3).setOnClickListener {
            openDetailActivity(
                titleKey = "upcoming_3",
                contentType = "upcoming",
                contentKeys = arrayOf("upcoming_3_detail")
            )
        }

        // 資源連結
        view.findViewById<View>(R.id.row_website).setOnClickListener {
            openUrl(getString(R.string.url_website))
        }
        view.findViewById<View>(R.id.row_privacy).setOnClickListener {
            openUrl(getString(R.string.url_privacy))
        }
        view.findViewById<View>(R.id.row_rate).setOnClickListener {
            openUrl(getString(R.string.url_rate))
        }
        view.findViewById<View>(R.id.row_copyright).setOnClickListener {
            startActivity(Intent(requireContext(), CopyrightActivity::class.java))
        }
        view.findViewById<View>(R.id.row_feedback).setOnClickListener {
            openDetailActivity(
                titleKey = "contact_us",
                contentType = "feedback",
                contentKeys = arrayOf("feedback_description", "feedback_email")
            )
        }
        view.findViewById<View>(R.id.row_version_history).setOnClickListener {
            openDetailActivity(
                titleKey = "version_history",
                contentType = "version",
                contentKeys = arrayOf("version_3_3_8", "version_3_3_8_changes")
            )
        }

        // FAQ
        view.findViewById<View>(R.id.row_faq_1).setOnClickListener {
            openDetailActivity(
                titleKey = "faq_1_question",
                contentType = "faq",
                contentKeys = arrayOf("faq_1_answer")
            )
        }
        view.findViewById<View>(R.id.row_faq_2).setOnClickListener {
            openDetailActivity(
                titleKey = "faq_2_question",
                contentType = "faq",
                contentKeys = arrayOf("faq_2_answer")
            )
        }
        view.findViewById<View>(R.id.row_faq_3).setOnClickListener {
            openDetailActivity(
                titleKey = "faq_3_question",
                contentType = "faq",
                contentKeys = arrayOf("faq_3_answer")
            )
        }
    }

    private fun openDetailActivity(
        titleKey: String,
        contentType: String,
        contentKeys: Array<String>
    ) {
        val intent = DetailActivity.createIntent(
            context = requireContext(),
            titleKey = titleKey,
            contentType = contentType,
            contentKeys = contentKeys
        )
        startActivity(intent)
    }

    private fun openUrl(url: String) {
        try {
            val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url))
            startActivity(intent)
        } catch (e: Exception) {
            // Handle exception
        }
    }

    companion object {
        fun newInstance() = Tab1Fragment()
    }
}
