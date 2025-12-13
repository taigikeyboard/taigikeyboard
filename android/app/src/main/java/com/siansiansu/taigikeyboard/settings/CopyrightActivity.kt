package com.siansiansu.taigikeyboard.settings

import android.os.Bundle
import android.view.MenuItem
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.Toolbar
import androidx.viewpager2.widget.ViewPager2
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.databinding.ActivityCopyrightBinding
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.model.CopyrightDataSource
import com.siansiansu.taigikeyboard.util.FontUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

class CopyrightActivity : AppCompatActivity() {
    private lateinit var binding: ActivityCopyrightBinding
    private lateinit var pagerAdapter: CopyrightPagerAdapter
    private lateinit var languageManager: LanguageManager
    private lateinit var prefs: PrefHelper
    private val copyrightPages = CopyrightDataSource.copyrightPages

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityCopyrightBinding.inflate(layoutInflater)
        setContentView(binding.root)

        prefs = PrefHelper(this)
        languageManager = LanguageManager.getInstance(this)

        // 設定 Edge-to-Edge 顯示模式
        setupEdgeToEdge()

        setupToolbar()
        setupViewPager()
        setupNavigation()
        observeLanguageChanges()
        applyCustomFont()
    }

    private fun setupToolbar() {
        val toolbar = findViewById<Toolbar>(R.id.toolbar)
        setSupportActionBar(toolbar)
        supportActionBar?.apply {
            title = languageManager.getText(AppTexts.copyrightTitle)
            setDisplayHomeAsUpEnabled(true)
        }

        // Set title text color to match home screen app title
        toolbar.setTitleTextColor(
            androidx.core.content.ContextCompat.getColor(this, R.color.modern_text_primary)
        )
    }

    private fun setupViewPager() {
        pagerAdapter = CopyrightPagerAdapter(this, copyrightPages, languageManager, prefs)
        binding.viewPager.adapter = pagerAdapter

        binding.viewPager.registerOnPageChangeCallback(object : ViewPager2.OnPageChangeCallback() {
            override fun onPageSelected(position: Int) {
                super.onPageSelected(position)
                updateUI(position)
            }
        })

        // Initialize UI for first page
        updateUI(0)
    }

    private fun setupNavigation() {
        binding.prevButton.setOnClickListener {
            val currentItem = binding.viewPager.currentItem
            if (currentItem > 0) {
                binding.viewPager.currentItem = currentItem - 1
            }
        }

        binding.nextButton.setOnClickListener {
            val currentItem = binding.viewPager.currentItem
            if (currentItem < copyrightPages.size - 1) {
                binding.viewPager.currentItem = currentItem + 1
            } else {
                finish()
            }
        }
    }

    private fun updateUI(position: Int) {
        val isLastPage = position == copyrightPages.size - 1

        // Update page indicator
        binding.pageIndicator.text = "${position + 1} / ${copyrightPages.size}"

        // Update progress bar
        val progress = ((position + 1) * 100) / copyrightPages.size
        binding.progressBar.progress = progress

        // Update prev button state
        binding.prevButton.isEnabled = position > 0
        binding.prevButton.alpha = if (position > 0) 1.0f else 0.5f
        binding.prevButton.text = languageManager.getText(AppTexts.guidePreviousPage)

        // Update next button text and state
        if (isLastPage) {
            binding.nextButton.text = languageManager.getText(AppTexts.done)
            binding.nextButton.setIconResource(android.R.drawable.ic_menu_close_clear_cancel)
        } else {
            binding.nextButton.text = languageManager.getText(AppTexts.guideNextPage)
            binding.nextButton.setIconResource(android.R.drawable.ic_media_next)
        }
    }

    private fun observeLanguageChanges() {
        languageManager.currentDisplayLanguage.observe(this) {
            // Update toolbar title
            supportActionBar?.title = languageManager.getText(AppTexts.copyrightTitle)

            // Update button texts
            updateUI(binding.viewPager.currentItem)

            // Notify adapter to refresh all pages
            pagerAdapter.notifyDataSetChanged()
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

    /**
     * 套用自訂字體到所有 UI 元件
     */
    private fun applyCustomFont() {
        val typeface = FontUtils.getTypefaceByType(prefs.fontType, this)

        // Page indicator and navigation buttons
        binding.pageIndicator.typeface = typeface
        binding.prevButton.typeface = typeface
        binding.nextButton.typeface = typeface
    }
}
