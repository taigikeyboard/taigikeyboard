package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import androidx.recyclerview.widget.RecyclerView
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.model.CopyrightPage

class CopyrightPagerAdapter(
    private val context: Context,
    private val pages: List<CopyrightPage>,
    private val languageManager: LanguageManager
) : RecyclerView.Adapter<CopyrightPagerAdapter.PageViewHolder>() {

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): PageViewHolder {
        val view = LayoutInflater.from(context)
            .inflate(R.layout.copyright_page_item, parent, false)
        return PageViewHolder(view)
    }

    override fun onBindViewHolder(holder: PageViewHolder, position: Int) {
        holder.bind(pages[position])
    }

    override fun getItemCount(): Int = pages.size

    inner class PageViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        // 移除 icon 和 license_icon，改為復古純文字風格
        private val title: TextView = itemView.findViewById(R.id.title)
        private val entryCount: TextView = itemView.findViewById(R.id.entry_count)
        private val description: TextView = itemView.findViewById(R.id.description)
        private val licenseDescription: TextView = itemView.findViewById(R.id.license_description)
        private val buttonsContainer: LinearLayout = itemView.findViewById(R.id.buttons_container)
        private val acknowledgment: TextView = itemView.findViewById(R.id.acknowledgment)

        fun bind(page: CopyrightPage) {
            // 取得主題綠色（用於按鈕 icon）
            val themeGreenColor = ContextCompat.getColor(context, R.color.modern_accent)

            // Set title
            title.text = languageManager.getText(page.title)

            // Set description
            description.text = languageManager.getText(page.description)

            // Set license description
            licenseDescription.text = languageManager.getText(page.license)

            // Clear previous buttons
            buttonsContainer.removeAllViews()

            // Add action buttons
            page.buttons.forEachIndexed { index, button ->
                val buttonView = LayoutInflater.from(context)
                    .inflate(R.layout.copyright_action_button, buttonsContainer, false)

                val buttonIcon = buttonView.findViewById<ImageView>(R.id.button_icon)
                val buttonText = buttonView.findViewById<TextView>(R.id.button_text)
                val buttonContainer = buttonView.findViewById<LinearLayout>(R.id.action_button_container)

                buttonIcon.setImageResource(button.iconResId)
                buttonIcon.setColorFilter(themeGreenColor)
                buttonText.text = languageManager.getText(button.text)

                buttonContainer.setOnClickListener {
                    val intent = Intent(Intent.ACTION_VIEW, Uri.parse(button.url))
                    context.startActivity(intent)
                }

                buttonsContainer.addView(buttonView)

                // Add divider between buttons
                if (index < page.buttons.size - 1) {
                    val dividerMargin = (52 * context.resources.displayMetrics.density).toInt()
                    val divider = View(context).apply {
                        layoutParams = LinearLayout.LayoutParams(
                            LinearLayout.LayoutParams.MATCH_PARENT,
                            1
                        ).apply {
                            marginStart = dividerMargin
                        }
                        setBackgroundColor(
                            ContextCompat.getColor(
                                context,
                                R.color.modern_text_secondary
                            )
                        )
                        alpha = 0.1f
                    }
                    buttonsContainer.addView(divider)
                }
            }
        }
    }
}
