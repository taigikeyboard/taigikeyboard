package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.LayoutInflater
import android.view.MenuItem
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.Toolbar
import androidx.cardview.widget.CardView
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab1Texts
import com.siansiansu.taigikeyboard.model.CopyrightDataSource
import com.siansiansu.taigikeyboard.model.CopyrightPage
import com.siansiansu.taigikeyboard.util.FontUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch

class CopyrightActivity : AppCompatActivity() {
    private lateinit var languageManager: LanguageManager
    private lateinit var prefs: PrefHelper
    private val copyrightPages = CopyrightDataSource.copyrightPages

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)
        languageManager = LanguageManager.getInstance(this)

        setContentView(R.layout.activity_copyright)

        // 設定 Edge-to-Edge 顯示模式
        setupEdgeToEdge()

        setupToolbar()
        setupCopyrightCards()
        observeLanguageChanges()
    }

    private fun setupToolbar() {
        val toolbar = findViewById<Toolbar>(R.id.toolbar)
        setSupportActionBar(toolbar)
        supportActionBar?.apply {
            title = languageManager.text(Tab1Texts.copyrightNotice)
            setDisplayHomeAsUpEnabled(true)
        }

        toolbar.setTitleTextColor(
            ContextCompat.getColor(this, R.color.modern_text_primary)
        )
    }

    private fun setupCopyrightCards() {
        val container = findViewById<LinearLayout>(R.id.copyright_container)
        container.removeAllViews()

        val typeface = FontUtils.getTypefaceByType(prefs.fontType, this)

        copyrightPages.forEach { page ->
            val cardView = createCopyrightCard(page, typeface)
            container.addView(cardView)
        }
    }

    private fun createCopyrightCard(
        page: CopyrightPage,
        typeface: android.graphics.Typeface
    ): View {
        val cardView = LayoutInflater.from(this)
            .inflate(R.layout.copyright_card_item, null)

        val card = cardView.findViewById<CardView>(R.id.card)
        val title = cardView.findViewById<TextView>(R.id.title)
        val description = cardView.findViewById<TextView>(R.id.description)
        val license = cardView.findViewById<TextView>(R.id.license)
        val buttonsContainer = cardView.findViewById<LinearLayout>(R.id.buttons_container)

        // 套用字體
        title.typeface = typeface
        description.typeface = typeface
        license.typeface = typeface

        // 設定內容
        title.text = languageManager.text(page.title)
        description.text = languageManager.text(page.description)
        license.text = languageManager.text(page.license)

        // 設定按鈕
        buttonsContainer.removeAllViews()
        page.buttons.forEachIndexed { index, button ->
            val buttonView = LayoutInflater.from(this)
                .inflate(R.layout.copyright_action_button, buttonsContainer, false)

            val buttonText = buttonView.findViewById<TextView>(R.id.button_text)
            val buttonContainer = buttonView.findViewById<LinearLayout>(R.id.action_button_container)

            buttonText.text = languageManager.text(button.text)
            buttonText.typeface = typeface

            buttonContainer.setOnClickListener {
                val intent = Intent(Intent.ACTION_VIEW, Uri.parse(button.url))
                startActivity(intent)
            }

            buttonsContainer.addView(buttonView)

            // 添加分隔線
            if (index < page.buttons.size - 1) {
                val divider = View(this).apply {
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        1
                    )
                    setBackgroundColor(
                        ContextCompat.getColor(
                            this@CopyrightActivity,
                            R.color.modern_shadow_light
                        )
                    )
                }
                buttonsContainer.addView(divider)
            }
        }

        // 設定卡片邊距
        val layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            bottomMargin = resources.getDimensionPixelSize(R.dimen.card_margin)
        }
        cardView.layoutParams = layoutParams

        return cardView
    }

    private fun observeLanguageChanges() {
        lifecycleScope.launch {
            languageManager.currentLanguageFlow.collectLatest {
                supportActionBar?.title = languageManager.text(Tab1Texts.copyrightNotice)
                setupCopyrightCards()
            }
        }
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        return when (item.itemId) {
            android.R.id.home -> {
                onBackPressedDispatcher.onBackPressed()
                true
            }
            else -> super.onOptionsItemSelected(item)
        }
    }
}
