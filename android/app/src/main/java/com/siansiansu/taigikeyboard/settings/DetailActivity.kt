package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.graphics.Typeface
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.cardview.widget.CardView
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding
import androidx.lifecycle.lifecycleScope
import com.google.android.material.appbar.MaterialToolbar
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.LocalizedText
import com.siansiansu.taigikeyboard.localization.Tab1Texts
import com.siansiansu.taigikeyboard.util.FontUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch

/**
 * 通用詳情頁面 Activity
 * 用於顯示功能說明、問題說明、FAQ 等詳細內容
 */
class DetailActivity : AppCompatActivity() {

    private lateinit var prefs: PrefHelper
    private lateinit var languageManager: LanguageManager

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)
        languageManager = LanguageManager.getInstance(this)

        setContentView(R.layout.activity_detail)

        setupEdgeToEdge()
        setupToolbar()
        setupContent()
        observeLanguageChanges()
    }

    private fun setupToolbar() {
        val toolbar = findViewById<MaterialToolbar>(R.id.toolbar)
        toolbar.setNavigationOnClickListener { finish() }

        // 處理系統狀態列 insets
        ViewCompat.setOnApplyWindowInsetsListener(findViewById(R.id.app_bar)) { view, windowInsets ->
            val insets = windowInsets.getInsets(WindowInsetsCompat.Type.statusBars())
            view.updatePadding(top = insets.top)
            windowInsets
        }

        // 設定標題
        val titleKey = intent.getStringExtra(EXTRA_TITLE_KEY)
        if (titleKey != null) {
            updateToolbarTitle(toolbar, titleKey)
        } else {
            toolbar.title = intent.getStringExtra(EXTRA_TITLE) ?: ""
        }
    }

    private fun updateToolbarTitle(toolbar: MaterialToolbar, titleKey: String) {
        val localizedText = getLocalizedTextByKey(titleKey)
        if (localizedText != null) {
            toolbar.title = languageManager.text(localizedText)
        }
    }

    private fun setupContent() {
        val container = findViewById<LinearLayout>(R.id.content_container)
        container.removeAllViews()

        val contentType = intent.getStringExtra(EXTRA_CONTENT_TYPE) ?: return
        val contentKeys = intent.getStringArrayExtra(EXTRA_CONTENT_KEYS) ?: return

        val typeface = FontUtils.getTypefaceByType(prefs.fontType, this)

        // 特殊處理 feedback 類型
        if (contentType == "feedback") {
            setupFeedbackContent(container, typeface)
            return
        }

        // 特殊處理 version 類型（版本紀錄）
        if (contentType == "version") {
            setupVersionHistoryContent(container, typeface)
            return
        }

        var isFirstCard = true

        contentKeys.forEach { key ->
            // 嘗試取得多段落內容
            val paragraphs = getLocalizedTextListByKey(key)
            if (paragraphs != null) {
                paragraphs.forEachIndexed { index, localizedText ->
                    if (!isFirstCard) {
                        addSpacer(container)
                    }

                    // 連紲建議詞第 1 段：文字 + 圖片在同一卡片
                    if (key == "feature_next_word_detail" && index == 0) {
                        val card = createParagraphCardWithImages(
                            languageManager.text(localizedText),
                            typeface,
                            getImagesForKey(key)
                        )
                        container.addView(card)
                    } else {
                        val card = createParagraphCard(languageManager.text(localizedText), typeface)
                        container.addView(card)
                    }
                    isFirstCard = false

                    // 異用字開關：在第 1 段後插入教育部辭典外部連結
                    if (key == "feature_variant_detail" && index == 0) {
                        addSpacer(container)
                        val linkRow = createExternalLinkRow(
                            languageManager.text(Tab1Texts.featureVariantDictLink),
                            typeface,
                            R.drawable.ic_open_in_new,
                            "https://sutian.moe.edu.tw/zh-hant/siongkuantsuguan/"
                        )
                        container.addView(linkRow)
                        isFirstCard = false
                    }

                    // FAQ 1：在第 1 段後插入導覽到「啟用方法」的項目
                    if (key == "faq_1_answer" && index == 0) {
                        addSpacer(container)
                        val navRow = createNavigationRow(
                            languageManager.text(Tab1Texts.goToSetupGuide),
                            typeface,
                            R.drawable.keyboard_24
                        ) {
                            startActivity(Intent(this@DetailActivity, SetupGuideActivity::class.java))
                        }
                        container.addView(navRow)
                        isFirstCard = false
                    }

                    // FAQ 2：在第 1 段後插入導覽到「問題回報」的項目
                    if (key == "faq_2_answer" && index == 0) {
                        addSpacer(container)
                        val navRow = createNavigationRow(
                            languageManager.text(Tab1Texts.goToFeedback),
                            typeface,
                            R.drawable.ic_email
                        ) {
                            val intent = DetailActivity.createIntent(
                                this@DetailActivity,
                                titleKey = "contact_us",
                                contentType = "feedback",
                                contentKeys = arrayOf("feedback_description", "feedback_email")
                            )
                            startActivity(intent)
                        }
                        container.addView(navRow)
                        isFirstCard = false
                    }

                    // FAQ 3：在第 1 段後插入截圖
                    if (key == "faq_3_answer" && index == 0) {
                        addSpacer(container)
                        val imageCard = createImageCard(R.drawable.faq_tone_handling)
                        container.addView(imageCard)
                        isFirstCard = false
                    }

                    // 拍字記持詞庫：在第 2 段後插入截圖
                    if (key == "feature_user_dict_detail" && index == 1) {
                        addSpacer(container)
                        val imageCard = createImageCard(R.drawable.feature_userdict)
                        container.addView(imageCard)
                        isFirstCard = false
                    }

                    // 大小寫切換：每段後插入對應截圖
                    if (key == "feature_case_switch_detail") {
                        when (index) {
                            0 -> {
                                // shift 圖片輪播
                                addSpacer(container)
                                val slideshow = ImageSlideshowView(this@DetailActivity).apply {
                                    layoutParams = LinearLayout.LayoutParams(
                                        LinearLayout.LayoutParams.MATCH_PARENT,
                                        LinearLayout.LayoutParams.WRAP_CONTENT
                                    )
                                    setImages(listOf(
                                        R.drawable.case_shift_1,
                                        R.drawable.case_shift_2
                                    ), intervalMs = 1500L)
                                }
                                val slideshowCard = CardView(this@DetailActivity).apply {
                                    layoutParams = LinearLayout.LayoutParams(
                                        LinearLayout.LayoutParams.MATCH_PARENT,
                                        LinearLayout.LayoutParams.WRAP_CONTENT
                                    )
                                    radius = resources.getDimension(R.dimen.card_corner_radius)
                                    cardElevation = 0f
                                    setCardBackgroundColor(getColor(R.color.modern_surface_secondary))
                                    addView(slideshow)
                                }
                                container.addView(slideshowCard)
                                isFirstCard = false
                            }
                            1 -> {
                                // lowercase 圖片
                                addSpacer(container)
                                val imageCard = createImageCard(R.drawable.case_lowercase)
                                container.addView(imageCard)
                                isFirstCard = false
                            }
                            2 -> {
                                // capslock 圖片
                                addSpacer(container)
                                val imageCard = createImageCard(R.drawable.case_capslock)
                                container.addView(imageCard)
                                isFirstCard = false
                            }
                            // index 3 無圖片（警告文字）
                        }
                    }
                }
            } else {
                // 嘗試取得單一文字
                val localizedText = getLocalizedTextByKey(key)
                if (localizedText != null) {
                    if (!isFirstCard) {
                        addSpacer(container)
                    }
                    val card = createParagraphCard(languageManager.text(localizedText), typeface)
                    container.addView(card)
                    isFirstCard = false
                }
            }
        }
    }

    private fun addSpacer(container: LinearLayout) {
        val spacer = View(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                resources.getDimensionPixelSize(R.dimen.card_margin)
            )
        }
        container.addView(spacer)
    }

    private fun createParagraphCard(text: String, typeface: Typeface): CardView {
        val card = CardView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            radius = resources.getDimension(R.dimen.card_corner_radius)
            cardElevation = 0f
            setCardBackgroundColor(getColor(R.color.modern_surface_secondary))
        }

        val textView = TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                val padding = resources.getDimensionPixelSize(R.dimen.card_margin)
                setMargins(padding, padding, padding, padding)
            }
            this.text = text
            this.typeface = typeface
            textSize = 17f
            setTextColor(getColor(R.color.modern_text_primary))
            setLineSpacing(0f, 1.4f)
        }

        card.addView(textView)
        return card
    }

    private fun createParagraphCardWithImages(
        text: String,
        typeface: Typeface,
        imageResIds: List<Int>
    ): CardView {
        val card = CardView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            radius = resources.getDimension(R.dimen.card_corner_radius)
            cardElevation = 0f
            setCardBackgroundColor(getColor(R.color.modern_surface_secondary))
        }

        val container = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            orientation = LinearLayout.VERTICAL
            val padding = resources.getDimensionPixelSize(R.dimen.card_margin)
            setPadding(padding, padding, padding, padding)
        }

        val textView = TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            this.text = text
            this.typeface = typeface
            textSize = 17f
            setTextColor(getColor(R.color.modern_text_primary))
            setLineSpacing(0f, 1.4f)
        }

        container.addView(textView)

        if (imageResIds.isNotEmpty()) {
            val slideshow = ImageSlideshowView(this).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = resources.getDimensionPixelSize(R.dimen.card_margin)
                }
                setImages(imageResIds, intervalMs = 1500L)
            }
            container.addView(slideshow)
        }

        card.addView(container)
        return card
    }

    private fun getImagesForKey(key: String): List<Int> {
        return when (key) {
            "feature_next_word_detail" -> listOf(
                R.drawable.nextword_1,
                R.drawable.nextword_2,
                R.drawable.nextword_3
            )
            else -> emptyList()
        }
    }

    private fun createImageCard(imageResId: Int): CardView {
        val card = CardView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            radius = resources.getDimension(R.dimen.card_corner_radius)
            cardElevation = 0f
            setCardBackgroundColor(getColor(R.color.modern_surface_secondary))
        }

        val imageView = ImageView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            adjustViewBounds = true
            scaleType = ImageView.ScaleType.FIT_CENTER
            setImageResource(imageResId)
        }

        card.addView(imageView)
        return card
    }

    private fun createNavigationRow(
        text: String,
        typeface: Typeface,
        iconResId: Int = R.drawable.dictionary_24,
        onClick: () -> Unit
    ): CardView {
        val card = CardView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            radius = resources.getDimension(R.dimen.card_corner_radius)
            cardElevation = 0f
            setCardBackgroundColor(getColor(R.color.modern_surface_secondary))
            isClickable = true
            isFocusable = true
            // 設定 ripple 效果
            val typedValue = android.util.TypedValue()
            theme.resolveAttribute(android.R.attr.selectableItemBackground, typedValue, true)
            foreground = resources.getDrawable(typedValue.resourceId, theme)
            setOnClickListener { onClick() }
        }

        val container = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            val paddingHorizontal = resources.getDimensionPixelSize(R.dimen.card_margin)
            val paddingVertical = (14 * resources.displayMetrics.density).toInt()
            setPadding(paddingHorizontal, paddingVertical, paddingHorizontal, paddingVertical)
        }

        // 圖示
        val icon = ImageView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                (24 * resources.displayMetrics.density).toInt(),
                (24 * resources.displayMetrics.density).toInt()
            ).apply {
                marginEnd = (12 * resources.displayMetrics.density).toInt()
            }
            setImageResource(iconResId)
            setColorFilter(getColor(R.color.modern_accent))
        }

        // 文字
        val textView = TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.WRAP_CONTENT,
                1f
            )
            this.text = text
            this.typeface = typeface
            textSize = 17f
            setTextColor(getColor(R.color.modern_text_primary))
        }

        // 箭頭
        val chevron = ImageView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                (12 * resources.displayMetrics.density).toInt(),
                (12 * resources.displayMetrics.density).toInt()
            )
            setImageResource(R.drawable.ic_chevron_right)
            setColorFilter(getColor(R.color.modern_text_secondary))
        }

        container.addView(icon)
        container.addView(textView)
        container.addView(chevron)
        card.addView(container)

        return card
    }

    private fun createExternalLinkRow(
        text: String,
        typeface: Typeface,
        iconResId: Int,
        url: String
    ): CardView {
        val card = CardView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            radius = resources.getDimension(R.dimen.card_corner_radius)
            cardElevation = 0f
            setCardBackgroundColor(getColor(R.color.modern_surface_secondary))
            isClickable = true
            isFocusable = true
            val typedValue = android.util.TypedValue()
            theme.resolveAttribute(android.R.attr.selectableItemBackground, typedValue, true)
            foreground = resources.getDrawable(typedValue.resourceId, theme)
            setOnClickListener {
                val intent = Intent(Intent.ACTION_VIEW, android.net.Uri.parse(url))
                startActivity(intent)
            }
        }

        val container = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            val paddingHorizontal = resources.getDimensionPixelSize(R.dimen.card_margin)
            val paddingVertical = (14 * resources.displayMetrics.density).toInt()
            setPadding(paddingHorizontal, paddingVertical, paddingHorizontal, paddingVertical)
        }

        val icon = ImageView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                (24 * resources.displayMetrics.density).toInt(),
                (24 * resources.displayMetrics.density).toInt()
            ).apply {
                marginEnd = (12 * resources.displayMetrics.density).toInt()
            }
            setImageResource(iconResId)
            setColorFilter(getColor(R.color.modern_accent))
        }

        val textView = TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.WRAP_CONTENT,
                1f
            )
            this.text = text
            this.typeface = typeface
            textSize = 17f
            setTextColor(getColor(R.color.modern_text_primary))
        }

        // 外部連結圖示
        val externalIcon = ImageView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                (12 * resources.displayMetrics.density).toInt(),
                (12 * resources.displayMetrics.density).toInt()
            )
            setImageResource(R.drawable.ic_open_in_new)
            setColorFilter(getColor(R.color.modern_text_secondary))
        }

        container.addView(icon)
        container.addView(textView)
        container.addView(externalIcon)
        card.addView(container)

        return card
    }

    private fun navigateToDictionarySettings() {
        val intent = Intent(this, SettingsMainActivity::class.java).apply {
            putExtra(SettingsMainActivity.EXTRA_START_TAB, R.id.nav_dictionary)
            flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        startActivity(intent)
        finish()
    }

    private fun setupFeedbackContent(container: LinearLayout, typeface: Typeface) {
        val googleFormUrl = "https://docs.google.com/forms/d/e/1FAIpQLSd7PEppQ9MdAptvoY-PaaXDlbbL9Gq9Y4lFjgU9sLz4ENiPoA/viewform?usp=header"
        val supportUrl = "https://portaly.cc/siansiansu/support"

        // 1. 說明文字
        val descCard = createParagraphCard(
            languageManager.text(Tab1Texts.feedbackDescription),
            typeface
        )
        container.addView(descCard)

        // 2. Google 表單連結
        addSpacer(container)
        val googleFormRow = createExternalLinkRow(
            languageManager.text(Tab1Texts.goToGoogleForm),
            typeface,
            R.drawable.ic_article,
            googleFormUrl
        )
        container.addView(googleFormRow)

        // 3. Email 聯絡
        addSpacer(container)
        val emailCard = createParagraphCard(
            languageManager.text(Tab1Texts.emailContact),
            typeface
        )
        container.addView(emailCard)

        // 4. 贊助連結
        addSpacer(container)
        val supportRow = createExternalLinkRow(
            languageManager.text(Tab1Texts.supportUs),
            typeface,
            R.drawable.ic_star,
            supportUrl
        )
        container.addView(supportRow)
    }

    /**
     * 設定版本紀錄內容（與 iOS VersionHistoryDetailView 一致）
     */
    private fun setupVersionHistoryContent(container: LinearLayout, typeface: Typeface) {
        var isFirstCard = true

        Tab1Texts.versionHistoryEntries.forEach { entry ->
            if (!isFirstCard) {
                addSpacer(container)
            }

            val card = createVersionEntryCard(entry, typeface)
            container.addView(card)
            isFirstCard = false
        }
    }

    /**
     * 建立版本項目卡片
     */
    private fun createVersionEntryCard(
        entry: Tab1Texts.VersionEntry,
        typeface: Typeface
    ): CardView {
        val card = CardView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            radius = resources.getDimension(R.dimen.card_corner_radius)
            cardElevation = 0f
            setCardBackgroundColor(getColor(R.color.modern_surface_secondary))
        }

        val contentContainer = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            orientation = LinearLayout.VERTICAL
            val padding = resources.getDimensionPixelSize(R.dimen.card_margin)
            setPadding(padding, padding, padding, padding)
        }

        // 版本號與日期列
        val headerRow = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }

        // 版本號
        val versionText = TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.WRAP_CONTENT,
                1f
            )
            text = "v${entry.version}"
            this.typeface = Typeface.create(typeface, Typeface.BOLD)
            textSize = 18f
            setTextColor(getColor(R.color.modern_accent))
        }

        // 日期
        val dateText = TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            text = entry.date
            this.typeface = typeface
            textSize = 14f
            setTextColor(getColor(R.color.modern_text_secondary))
        }

        headerRow.addView(versionText)
        headerRow.addView(dateText)
        contentContainer.addView(headerRow)

        // 變更內容列表
        val changesContainer = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = (12 * resources.displayMetrics.density).toInt()
            }
            orientation = LinearLayout.VERTICAL
        }

        entry.changes.forEach { change ->
            val changeRow = LinearLayout(this).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    bottomMargin = (8 * resources.displayMetrics.density).toInt()
                }
                orientation = LinearLayout.HORIZONTAL
            }

            // 項目符號
            val bullet = TextView(this).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    marginEnd = (8 * resources.displayMetrics.density).toInt()
                }
                text = "•"
                textSize = 16f
                setTextColor(getColor(R.color.modern_text_secondary))
            }

            // 變更文字
            val changeText = TextView(this).apply {
                layoutParams = LinearLayout.LayoutParams(
                    0,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    1f
                )
                text = languageManager.text(change)
                this.typeface = typeface
                textSize = 16f
                setTextColor(getColor(R.color.modern_text_primary))
            }

            changeRow.addView(bullet)
            changeRow.addView(changeText)
            changesContainer.addView(changeRow)
        }

        contentContainer.addView(changesContainer)
        card.addView(contentContainer)

        return card
    }

    private fun observeLanguageChanges() {
        lifecycleScope.launch {
            languageManager.currentLanguageFlow.collectLatest {
                // 重新載入內容
                val toolbar = findViewById<MaterialToolbar>(R.id.toolbar)
                val titleKey = intent.getStringExtra(EXTRA_TITLE_KEY)
                if (titleKey != null) {
                    updateToolbarTitle(toolbar, titleKey)
                }
                setupContent()
            }
        }
    }

    private fun getLocalizedTextByKey(key: String): LocalizedText? {
        return when (key) {
            // 功能說明
            "feature_next_word" -> Tab1Texts.featureNextWord
            "feature_variant" -> Tab1Texts.featureVariant
            "feature_custom_font" -> Tab1Texts.featureCustomFont
            "feature_user_dict" -> Tab1Texts.featureUserDict
            "feature_case_switch" -> Tab1Texts.featureCaseSwitch

            // 處理中問題
            "issue_1" -> Tab1Texts.issue1
            "issue_2" -> Tab1Texts.issue2
            "issue_3" -> Tab1Texts.issue3

            // 預計新功能
            "upcoming_1" -> Tab1Texts.upcoming1
            "upcoming_2" -> Tab1Texts.upcoming2
            "upcoming_3" -> Tab1Texts.upcoming3

            // FAQ
            "faq_1_question" -> Tab1Texts.faq1Question
            "faq_2_question" -> Tab1Texts.faq2Question
            "faq_3_question" -> Tab1Texts.faq3Question

            // 問題回報
            "contact_us" -> Tab1Texts.contactUs
            "feedback_description" -> Tab1Texts.feedbackDescription
            "feedback_email" -> Tab1Texts.emailContact

            // 版本紀錄
            "version_history" -> Tab1Texts.versionHistory

            else -> null
        }
    }

    /**
     * 取得多段落內容
     */
    private fun getLocalizedTextListByKey(key: String): List<LocalizedText>? {
        return when (key) {
            // 功能說明段落
            "feature_next_word_detail" -> Tab1Texts.featureNextWordParagraphs
            "feature_variant_detail" -> Tab1Texts.featureVariantParagraphs
            "feature_custom_font_detail" -> Tab1Texts.featureCustomFontParagraphs
            "feature_user_dict_detail" -> Tab1Texts.featureUserDictParagraphs
            "feature_case_switch_detail" -> Tab1Texts.featureCaseSwitchParagraphs

            // 處理中問題段落
            "issue_1_detail" -> Tab1Texts.issue1Paragraphs
            "issue_2_detail" -> Tab1Texts.issue2Paragraphs
            "issue_3_detail" -> Tab1Texts.issue3Paragraphs

            // 預計新功能段落
            "upcoming_1_detail" -> Tab1Texts.upcoming1Paragraphs
            "upcoming_2_detail" -> Tab1Texts.upcoming2Paragraphs
            "upcoming_3_detail" -> Tab1Texts.upcoming3Paragraphs

            // FAQ 答案段落
            "faq_1_answer" -> Tab1Texts.faq1Paragraphs
            "faq_2_answer" -> Tab1Texts.faq2Paragraphs
            "faq_3_answer" -> Tab1Texts.faq3Paragraphs

            // 版本紀錄（取第一個版本的變更）
            "version_3_3_8_changes" -> Tab1Texts.versionHistoryEntries.firstOrNull()?.changes

            else -> null
        }
    }

    companion object {
        const val EXTRA_TITLE = "extra_title"
        const val EXTRA_TITLE_KEY = "extra_title_key"
        const val EXTRA_CONTENT_TYPE = "extra_content_type"
        const val EXTRA_CONTENT_KEYS = "extra_content_keys"

        fun createIntent(
            context: Context,
            titleKey: String,
            contentType: String,
            contentKeys: Array<String>
        ): Intent {
            return Intent(context, DetailActivity::class.java).apply {
                putExtra(EXTRA_TITLE_KEY, titleKey)
                putExtra(EXTRA_CONTENT_TYPE, contentType)
                putExtra(EXTRA_CONTENT_KEYS, contentKeys)
            }
        }
    }
}
