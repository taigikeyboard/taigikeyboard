package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.AttributeSet
import android.widget.FrameLayout
import android.widget.ImageView

/**
 * 圖片輪播元件
 * 自動切換顯示多張圖片
 */
class ImageSlideshowView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : FrameLayout(context, attrs, defStyleAttr) {

    private val imageView: ImageView
    private val handler = Handler(Looper.getMainLooper())
    private var imageResIds: List<Int> = emptyList()
    private var currentIndex = 0
    private var intervalMs: Long = 1500L
    private var isRunning = false

    private val slideRunnable = object : Runnable {
        override fun run() {
            if (imageResIds.isNotEmpty()) {
                currentIndex = (currentIndex + 1) % imageResIds.size
                imageView.setImageResource(imageResIds[currentIndex])
                handler.postDelayed(this, intervalMs)
            }
        }
    }

    init {
        imageView = ImageView(context).apply {
            layoutParams = LayoutParams(
                LayoutParams.MATCH_PARENT,
                LayoutParams.WRAP_CONTENT
            )
            scaleType = ImageView.ScaleType.FIT_CENTER
            adjustViewBounds = true
        }
        addView(imageView)
    }

    /**
     * 設定要輪播的圖片資源
     */
    fun setImages(resIds: List<Int>, intervalMs: Long = 1500L) {
        this.imageResIds = resIds
        this.intervalMs = intervalMs
        this.currentIndex = 0

        if (resIds.isNotEmpty()) {
            imageView.setImageResource(resIds[0])
        }
    }

    fun startSlideshow() {
        if (!isRunning && imageResIds.size > 1) {
            isRunning = true
            handler.postDelayed(slideRunnable, intervalMs)
        }
    }

    fun stopSlideshow() {
        isRunning = false
        handler.removeCallbacks(slideRunnable)
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        startSlideshow()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        stopSlideshow()
    }
}
